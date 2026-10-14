//! Route protection middleware.
//!
//! Verifies the session cookie and either:
//! - Injects the authenticated user into request extensions, or
//! - Redirects unauthenticated users away from protected routes to `/login`.

use std::collections::HashSet;
use std::sync::LazyLock;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::http::Response;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::extract::cookie::CookieJar;
use sqlx::PgPool;

use crate::auth::context::{AuthUser, AuthUserProfile, UserProfile};
use crate::auth::session;
use crate::db;

/// Check if a path matches a protected route pattern.
///
/// Handles both exact matches and parameterized routes like `/recipes/:uuid`.
fn is_protected_path(path: &str) -> bool {
    matches!(
        path,
        "/home"
            | "/dashboard"
            | "/search"
            | "/recipes/new"
            | "/collections"
            | "/collections/new"
            | "/settings/profile"
            | "/settings/accounts"
    ) || is_recipe_route(path)
        || is_collection_route(path)
}

fn is_collection_route(path: &str) -> bool {
    if !path.starts_with("/collections/") {
        return false;
    }
    let segments = path["/collections/".len()..].split('/').collect::<Vec<_>>();
    matches!(segments.as_slice(), [id, "edit"] if is_valid_uuid_string(id))
}

/// Check for exact owner-only recipe routes while leaving published detail public.
fn is_recipe_route(path: &str) -> bool {
    if !path.starts_with("/recipes/") {
        return false;
    }
    let rest = &path["/recipes/".len()..];
    if rest.is_empty() || rest.starts_with('/') {
        return false;
    }
    let segments = rest.split('/').collect::<Vec<_>>();
    if segments.first() == Some(&"drafts") {
        return matches!(segments.as_slice(), ["drafts", id, "edit"] if is_valid_uuid_string(id));
    }
    match segments.as_slice() {
        [id, "edit"] => is_valid_uuid_string(id),
        _ => false,
    }
}

/// Check if a string matches the UUID format (8-4-4-4-12 hex chars with dashes).
fn is_valid_uuid_string(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    if parts[0].len() != 8
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 4
        || parts[4].len() != 12
    {
        return false;
    }
    s.replace('-', "").chars().all(|c| c.is_ascii_hexdigit())
}

/// Routes that redirect authenticated users away.
static REDIRECT_IF_AUTHED_PATHS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["/login"].into_iter().collect());

/// Axum middleware handler for route protection.
///
/// Reads the session cookie, verifies the JWT, and either:
/// - Injects `AuthUser` and `AuthUserProfile` into request extensions for downstream handlers
/// - Returns a 302 redirect to `/login` for unauthenticated users on protected paths
/// - Returns a 302 redirect to `/home` for authenticated users on `/login`
pub async fn handle_auth(
    State(pool): State<PgPool>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = req.uri().path().to_string();
    let original_target = req
        .uri()
        .path_and_query()
        .map_or_else(|| path.clone(), |target| target.as_str().to_string());

    // Extract session cookie from headers
    let jar = CookieJar::from_headers(req.headers());

    // Check for valid session
    let configured_cookie_name = session::cookie_name();
    let session_token = jar.get(configured_cookie_name.as_ref());
    let verified_user_id = if let Some(cookie) = session_token {
        session::verify_session(&pool, cookie.value()).await.ok()
    } else {
        None
    };

    let is_authenticated = verified_user_id.is_some();

    // Check if path is protected
    let is_protected = is_protected_path(&path);
    let is_redirect_if_authed = REDIRECT_IF_AUTHED_PATHS.contains(path.as_str());

    // Redirect authenticated users away from login
    if is_authenticated && is_redirect_if_authed {
        return redirect_to("/home");
    }

    // Redirect unauthenticated users from protected paths to login
    if !is_authenticated && is_protected {
        let encoded = percent_encoding::utf8_percent_encode(
            &original_target,
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        let location = format!("/login?redirect_uri={encoded}");
        return redirect_to(&location);
    }

    // Inject user into extensions if authenticated
    if let Some(user_id) = verified_user_id {
        req.extensions_mut().insert(AuthUser { user_id });
        if let Ok(Some(user)) = db::get_user_by_id(&pool, user_id).await {
            let profile = UserProfile {
                id: user.id,
                username: user.username,
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                bio: user.bio,
            };
            req.extensions_mut().insert(AuthUserProfile { profile });
        }
    }

    // Continue to the next handler
    let mut response: Response<Body> = next.run(req).await;

    // Rolling session refresh: if token is old but still valid, issue a new one
    if let Some(_user_id) = verified_user_id {
        if let Some(cookie) = session_token {
            if session::should_refresh(&pool, cookie.value())
                .await
                .unwrap_or(false)
            {
                if let Ok(new_token) = session::refresh_session(&pool, cookie.value()).await {
                    let new_cookie = session::build_session_cookie(&new_token);
                    response.headers_mut().insert(
                        axum::http::header::SET_COOKIE,
                        new_cookie
                            .to_string()
                            .parse()
                            .expect("cookie string is valid HeaderValue"),
                    );
                }
            }
        }
    }

    response
}

fn redirect_to(location: &str) -> Response<Body> {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::LOCATION,
        location
            .parse()
            .expect("redirect location must be a valid HeaderValue"),
    );
    (StatusCode::FOUND, headers, Body::empty()).into_response()
}

#[cfg(test)]
mod tests {
    use super::is_protected_path;

    async fn test_user(pool: &sqlx::PgPool, label: &str) -> crate::db::User {
        let unique = crate::test_utils::uid();
        crate::db::insert_user(
            pool,
            &format!("{label}_{unique}"),
            label,
            &format!("{label}{unique}@example.com"),
            None,
        )
        .await
        .unwrap()
    }

    fn identity_router(pool: sqlx::PgPool) -> axum::Router {
        axum::Router::new()
            .route(
                "/dashboard",
                axum::routing::get(
                    |axum::Extension(user): axum::Extension<crate::auth::context::AuthUser>| async move {
                        user.user_id.to_string()
                    },
                ),
            )
            .layer(axum::middleware::from_fn_with_state(pool, super::handle_auth))
    }

    fn session_request(token: &str) -> axum::http::Request<axum::body::Body> {
        axum::http::Request::builder()
            .uri("/dashboard")
            .header(
                "cookie",
                format!("{}={token}", crate::auth::session::cookie_name()),
            )
            .body(axum::body::Body::empty())
            .unwrap()
    }

    async fn make_session_refreshable(pool: &sqlx::PgPool, user_id: uuid::Uuid) {
        sqlx::query(
            "UPDATE sessions SET expires_at = NOW() + INTERVAL '5 minutes' WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn session_revoked_during_downstream_work_emits_no_refresh_cookie() {
        use crate::auth::session;
        use tower::ServiceExt;

        session::set_test_secret("test-secret-32-bytes-long-enough!!");
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let owner = test_user(&pool, "pending_refresh").await;
        let replacement = test_user(&pool, "replacement_refresh").await;
        let token = session::create_session(&pool, owner.id).await.unwrap();
        make_session_refreshable(&pool, owner.id).await;
        assert!(session::should_refresh(&pool, &token).await.unwrap());

        let admitted = std::sync::Arc::new(tokio::sync::Notify::new());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        let gate = (admitted.clone(), release.clone());
        let router = axum::Router::new()
            .route(
                "/dashboard",
                axum::routing::get(
                    move |axum::Extension(user): axum::Extension<
                        crate::auth::context::AuthUser,
                    >| {
                        let (admitted, release) = gate.clone();
                        async move {
                            admitted.notify_one();
                            release.notified().await;
                            user.user_id.to_string()
                        }
                    },
                ),
            )
            .layer(axum::middleware::from_fn_with_state(
                pool.clone(),
                super::handle_auth,
            ));
        let pending = router.oneshot(session_request(&token));
        let revoke = async {
            admitted.notified().await;
            session::revoke_session(&pool, &token).await.unwrap();
            let replacement_token = session::create_session(&pool, replacement.id)
                .await
                .unwrap();
            release.notify_one();
            replacement_token
        };
        let (response, replacement_token) =
            tokio::time::timeout(std::time::Duration::from_secs(15), async {
                tokio::join!(pending, revoke)
            })
            .await
            .expect("pending request should complete");
        let response = response.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(!response.headers().contains_key("set-cookie"));
        let body = axum::body::to_bytes(response.into_body(), 128)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), owner.id.to_string().as_bytes());
        assert!(session::verify_session(&pool, &token).await.is_err());
        assert_eq!(
            session::verify_session(&pool, &replacement_token)
                .await
                .unwrap(),
            replacement.id
        );
        session::clear_test_secret();
    }

    #[tokio::test]
    async fn delayed_refresh_cookie_can_replace_active_identity_but_cannot_revive_revoked_session()
    {
        use crate::auth::session;
        use tower::ServiceExt;

        session::set_test_secret("test-secret-32-bytes-long-enough!!");
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let original = test_user(&pool, "delivered_refresh").await;
        let replacement = test_user(&pool, "delivered_replacement").await;
        let token = session::create_session(&pool, original.id).await.unwrap();
        make_session_refreshable(&pool, original.id).await;
        let response = identity_router(pool.clone())
            .oneshot(session_request(&token))
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let refresh_cookie = axum_extra::extract::cookie::Cookie::parse(
            response.headers()["set-cookie"]
                .to_str()
                .unwrap()
                .to_string(),
        )
        .unwrap()
        .into_owned();
        assert_eq!(refresh_cookie.name(), session::cookie_name().as_ref());
        assert_eq!(
            session::verify_session(&pool, refresh_cookie.value())
                .await
                .unwrap(),
            original.id
        );
        let replacement_token = session::create_session(&pool, replacement.id)
            .await
            .unwrap();

        // Replay delivery of an already generated response after the newer
        // identity's cookie. No browser scheduler or authentication epoch is modeled.
        let replacement_cookie = session::build_session_cookie(&replacement_token);
        let jar = axum_extra::extract::cookie::CookieJar::new()
            .add(replacement_cookie.clone())
            .add(refresh_cookie.clone());
        let response = identity_router(pool.clone())
            .oneshot(session_request(
                jar.get(session::cookie_name().as_ref()).unwrap().value(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 128)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), original.id.to_string().as_bytes());

        session::revoke_session(&pool, &token).await.unwrap();
        let jar = axum_extra::extract::cookie::CookieJar::new()
            .add(replacement_cookie)
            .add(refresh_cookie.clone());
        let response = identity_router(pool.clone())
            .oneshot(session_request(
                jar.get(session::cookie_name().as_ref()).unwrap().value(),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::FOUND);
        assert_eq!(
            response.headers()["location"],
            "/login?redirect_uri=%2Fdashboard"
        );
        assert!(!response.headers().contains_key("set-cookie"));
        assert!(session::verify_session(&pool, refresh_cookie.value())
            .await
            .is_err());
        assert!(session::refresh_session(&pool, refresh_cookie.value())
            .await
            .is_err());
        assert_eq!(
            session::verify_session(&pool, &replacement_token)
                .await
                .unwrap(),
            replacement.id
        );
        let session_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1")
                .bind(original.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            session_count, 1,
            "rolling refresh updates the original row, not a new session"
        );
        session::clear_test_secret();
    }

    #[test]
    fn community_home_requires_sign_in_but_the_landing_page_remains_public() {
        assert!(is_protected_path("/home"));
        assert!(!is_protected_path("/"));
        assert!(is_protected_path("/dashboard"));
    }

    const COLLECTION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn search_requires_authentication() {
        assert!(is_protected_path("/search"));
    }

    #[test]
    fn collection_owner_routes_require_authentication_but_detail_is_shareable() {
        assert!(is_protected_path("/collections"));
        assert!(is_protected_path("/collections/new"));
        assert!(!is_protected_path(&format!("/collections/{COLLECTION_ID}")));
        assert!(is_protected_path(&format!(
            "/collections/{COLLECTION_ID}/edit"
        )));
    }

    #[test]
    fn malformed_collection_routes_are_not_treated_as_crud_routes() {
        assert!(!is_protected_path("/collections/not-a-uuid"));
        assert!(!is_protected_path(&format!(
            "/collections/{COLLECTION_ID}/extra"
        )));
        assert!(!is_protected_path("/collections/42"));
    }

    #[test]
    fn published_recipe_detail_is_shareable_but_owner_recipe_routes_stay_protected() {
        assert!(!is_protected_path(&format!("/recipes/{COLLECTION_ID}")));
        assert!(is_protected_path(&format!("/recipes/{COLLECTION_ID}/edit")));
        assert!(!is_protected_path(&format!(
            "/recipes/{COLLECTION_ID}/versions/1"
        )));
        assert!(is_protected_path(&format!(
            "/recipes/drafts/{COLLECTION_ID}/edit"
        )));
    }
}
