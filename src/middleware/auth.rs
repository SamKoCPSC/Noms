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
        "/dashboard"
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
/// - Returns a 302 redirect to `/dashboard` for authenticated users on `/login`
pub async fn handle_auth(
    State(pool): State<PgPool>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = req.uri().path().to_string();

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
        return redirect_to("/dashboard");
    }

    // Redirect unauthenticated users from protected paths to login
    if !is_authenticated && is_protected {
        let encoded =
            percent_encoding::utf8_percent_encode(&path, percent_encoding::NON_ALPHANUMERIC)
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

    const COLLECTION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

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
