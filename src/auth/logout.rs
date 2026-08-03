//! Logout handler: revokes the session in the database, clears the session
//! cookie, and redirects to home (or a validated `redirect_uri`).

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;

#[cfg(test)]
use axum::routing::get;
#[cfg(test)]
use sqlx::PgPool;

use crate::auth::oauth::AppState;
use crate::auth::session;
use tracing;

/// Query parameters for GET logout requests.
#[derive(Debug, Deserialize)]
pub(crate) struct LogoutQuery {
    #[serde(default)]
    redirect_uri: Option<String>,
}

fn validate_redirect_uri(uri: &str) -> Option<String> {
    crate::auth::redirect::normalize_same_origin_redirect(uri)
}

/// Handle a logout request.
///
/// For GET requests: validates an optional `redirect_uri` query parameter.
/// If valid, redirects to that URI. If missing or invalid, defaults to `/`.
/// For POST requests: always redirects to `/` (unchanged behavior).
///
/// Revokes the session in the database (sets `revoked = TRUE`), then clears
/// the session cookie by setting it with `max-age=0`.
pub async fn handle_logout(
    state: axum::extract::State<AppState>,
    jar: CookieJar,
    Query(params): Query<LogoutQuery>,
) -> Response {
    // Revoke the session in the database if a valid token is present
    let configured_cookie_name = session::cookie_name();
    if let Some(cookie) = jar.get(configured_cookie_name.as_ref()) {
        if let Err(e) = session::revoke_session(&state.pool, cookie.value()).await {
            tracing::warn!(error = %e, "Failed to revoke session during logout");
        }
    }

    let clear_cookie = session::clear_session_cookie();
    let cookie_header = clear_cookie.encoded().to_string();

    // Determine redirect target: validate redirect_uri if provided, default to "/"
    let redirect_target = match &params.redirect_uri {
        Some(uri) => validate_redirect_uri(uri).unwrap_or_else(|| "/".to_string()),
        None => "/".to_string(),
    };

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        cookie_header.parse().expect("valid cookie header"),
    );
    headers.insert(
        axum::http::header::LOCATION,
        redirect_target.parse().expect("valid redirect location"),
    );

    (StatusCode::FOUND, headers, ()).into_response()
}

/// Build a minimal router exposing the logout handler for testing.
/// Requires a `PgPool` as state (wrapped in `AppState`).
#[cfg(test)]
fn make_router(pool: PgPool) -> axum::Router {
    let (_google, _github) = crate::auth::oauth::build_oauth_clients("http://localhost:3000");
    let state = AppState {
        pool,
        google_client: _google,
        github_client: _github,
        http_client: reqwest::Client::new(),
        google_userinfo_url: "http://localhost:8082/google/userinfo".to_string(),
        github_userinfo_url: "http://localhost:8082/github/userinfo".to_string(),
        rate_limit: crate::middleware::rate_limit::RateLimitState::default(),
        #[cfg(feature = "server")]
        storage: crate::storage::StorageClient::for_test(),
    };
    axum::Router::new()
        .route("/auth/logout", get(handle_logout).post(handle_logout))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn logout_post_returns_302_with_redirect() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_sets_clear_cookie_header() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let set_cookie = response
            .headers()
            .get(axum::http::header::SET_COOKIE)
            .expect("Set-Cookie header should be present");

        let cookie_str = set_cookie.to_str().unwrap();
        // Cookie name should be present
        assert!(cookie_str.contains("noms_session"));
        // Max-Age should be 0 (deleting the cookie)
        assert!(cookie_str.contains("Max-Age=0"));
    }

    #[tokio::test]
    async fn logout_get_no_params_redirects_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
        // Also verify Set-Cookie is present
        assert!(response
            .headers()
            .contains_key(axum::http::header::SET_COOKIE));
    }

    // ── New tests for redirect_uri validation ──────────────────────────────

    #[tokio::test]
    async fn logout_get_valid_redirect_uri() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/dashboard"
        );
        assert!(response
            .headers()
            .contains_key(axum::http::header::SET_COOKIE));
    }

    #[tokio::test]
    async fn logout_get_redirect_uri_with_query_string() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=/dashboard%3Ftab%3Drecipes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/dashboard?tab=recipes"
        );
    }

    #[tokio::test]
    async fn logout_get_invalid_redirect_external_url_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=https://evil.com/phish")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        // Should default to "/" because external URL is rejected
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_invalid_redirect_protocol_relative_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=//evil.com/phish")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_normalization_confusion_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        for redirect in [
            "%2F%5Cevil.example",
            "%2F%255cevil.example",
            "%2F%252fevil.example",
            "%2F.%2F%2Fevil.example",
            "%2F%252e%252e%2F%2Fevil.example",
            "%2F%25zz",
            "%2F%25FF",
        ] {
            let response = make_router(pool.clone())
                .oneshot(
                    axum::http::Request::builder()
                        .uri(format!("/auth/logout?redirect_uri={redirect}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::FOUND);
            assert_eq!(
                response.headers().get(axum::http::header::LOCATION),
                Some(&axum::http::HeaderValue::from_static("/")),
                "normalization-confusing redirect should fall back: {redirect}"
            );
        }
    }

    #[tokio::test]
    async fn logout_get_invalid_redirect_no_leading_slash_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_empty_redirect_uri_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_overlong_redirect_uri_defaults_to_home() {
        let long_uri = format!(
            "/{}",
            "a".repeat(crate::auth::redirect::REDIRECT_URI_MAX_LEN)
        );
        let encoded =
            percent_encoding::utf8_percent_encode(&long_uri, percent_encoding::NON_ALPHANUMERIC)
                .to_string();
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/auth/logout?redirect_uri={}", encoded))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap(),
            "/"
        );
    }

    // ── DB-backed revoke tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn logout_revokes_session_in_db() {
        use crate::auth::session as sess;

        // Set thread-local secret for session module
        sess::set_test_secret("test-secret-32-bytes-long-enough!!");

        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();
        let user = crate::db::insert_user(
            &pool,
            &format!("logout_{u}"),
            "Logout User",
            &format!("logout{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Create a session
        let token = sess::create_session(&pool, user.id).await.unwrap();

        // Verify session is active
        let verified = sess::verify_session(&pool, &token).await.unwrap();
        assert_eq!(verified, user.id);

        // Build a router and call logout with the session cookie
        let app = make_router(pool.clone());
        let _response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header("Cookie", format!("noms_session={}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Session should now be revoked
        let result = sess::verify_session(&pool, &token).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), sess::SessionError::SessionInvalid);
    }
}
