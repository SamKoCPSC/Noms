//! Logout handler: revokes the session in the database, clears the session
//! cookie, and redirects to home (or a validated `redirect_uri`).

use axum::extract::Query;
use axum::http::{HeaderMap, Method, StatusCode};
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

/// Query parameters for logout requests.
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
/// Only POST is state-changing, and an exact configured-origin header is
/// required before the session is revoked or its cookie is cleared.
pub async fn handle_logout(
    state: axum::extract::State<AppState>,
    method: Method,
    headers: HeaderMap,
    jar: CookieJar,
    Query(params): Query<LogoutQuery>,
) -> Response {
    if method != Method::POST {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    if let Err(category) = validate_logout_origin(&headers, &state.base_url_origin) {
        let total = state
            .auth_security_metrics
            .record_logout_origin_rejection(category);
        tracing::warn!(
            rejection_category = category,
            rejection_total = total,
            "Logout rejected because request origin was not exact"
        );
        return StatusCode::FORBIDDEN.into_response();
    }

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

fn validate_logout_origin(
    headers: &HeaderMap,
    expected_origin: &url::Origin,
) -> Result<(), &'static str> {
    let mut values = headers.get_all(axum::http::header::ORIGIN).iter();
    let Some(value) = values.next() else {
        return Err("missing");
    };
    if values.next().is_some() {
        return Err("duplicate");
    }
    let raw = value.to_str().map_err(|_| "malformed")?;
    if raw == "null" {
        return Err("null");
    }

    // Origin's grammar contains only a scheme and authority. Parsing as a URL
    // handles canonical host/default-port semantics after excluding paths,
    // credentials, query strings, and fragments at the syntax boundary.
    let (scheme, authority) = raw.split_once("://").ok_or("malformed")?;
    if !matches!(scheme, "http" | "https")
        || authority.is_empty()
        || authority.parse::<axum::http::uri::Authority>().is_err()
    {
        return Err("malformed");
    }
    let parsed = url::Url::parse(raw).map_err(|_| "malformed")?;
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.host().is_none()
    {
        return Err("malformed");
    }

    if &parsed.origin() == expected_origin {
        Ok(())
    } else {
        Err("mismatch")
    }
}

/// Build a minimal router exposing the logout handler for testing.
/// Requires a `PgPool` as state (wrapped in `AppState`).
#[cfg(test)]
fn make_router(pool: PgPool) -> axum::Router {
    let (_google, _github) = crate::auth::oauth::build_oauth_clients("http://localhost:3000");
    let state = AppState {
        pool,
        base_url_origin: url::Url::parse("http://localhost:3000").unwrap().origin(),
        auth_security_metrics: crate::auth::oauth::AuthSecurityMetrics::default(),
        google_client: _google,
        github_client: _github,
        http_client: reqwest::Client::new(),
        google_userinfo_url: "http://localhost:8082/google/userinfo".to_string(),
        github_userinfo_url: "http://localhost:8082/github/userinfo".to_string(),
        github_emails_url: "http://localhost:8082/github/emails".to_string(),
        github_extra_scopes: Vec::new(),
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
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_no_params_redirects_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_valid_redirect_uri() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout?redirect_uri=/dashboard")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_redirect_uri_with_query_string() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout?redirect_uri=/dashboard%3Ftab%3Drecipes")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_invalid_redirect_external_url_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout?redirect_uri=https://evil.com/phish")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_invalid_redirect_protocol_relative_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout?redirect_uri=//evil.com/phish")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_normalization_confusion_defaults_to_home() {
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
                        .method("POST")
                        .uri(format!("/auth/logout?redirect_uri={redirect}"))
                        .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_invalid_redirect_no_leading_slash_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout?redirect_uri=dashboard")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_empty_redirect_uri_defaults_to_home() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let app = make_router(pool);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout?redirect_uri=")
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
    async fn logout_post_overlong_redirect_uri_defaults_to_home() {
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
                    .method("POST")
                    .uri(format!("/auth/logout?redirect_uri={}", encoded))
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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
                    .header(axum::http::header::ORIGIN, "http://localhost:3000")
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

    #[tokio::test]
    async fn logout_get_does_not_revoke_or_clear_session() {
        use crate::auth::session as sess;

        sess::set_test_secret("test-secret-32-bytes-long-enough!!");
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = crate::db::insert_user(
            &pool,
            &format!("logout_get_{}", test_utils::uid()),
            "Logout GET User",
            &format!("logout-get-{}@example.com", test_utils::uid()),
            None,
        )
        .await
        .unwrap();
        let token = sess::create_session(&pool, user.id).await.unwrap();

        let response = make_router(pool.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout")
                    .header("Cookie", format!("noms_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(!response
            .headers()
            .contains_key(axum::http::header::SET_COOKIE));
        assert_eq!(sess::verify_session(&pool, &token).await.unwrap(), user.id);
        sess::clear_test_secret();
    }

    #[tokio::test]
    async fn logout_rejects_every_non_exact_origin_without_changing_session() {
        use crate::auth::session as sess;

        sess::set_test_secret("test-secret-32-bytes-long-enough!!");
        let (_db, pool) = test_utils::setup_test_db().await;
        let cases: &[(&str, &[&str])] = &[
            ("missing", &[]),
            (
                "duplicate",
                &["http://localhost:3000", "http://localhost:3000"],
            ),
            ("null", &["null"]),
            ("malformed", &["not an origin"]),
            ("path", &["http://localhost:3000/logout"]),
            ("query", &["http://localhost:3000?logout=1"]),
            ("fragment", &["http://localhost:3000#logout"]),
            ("credentials", &["http://user@localhost:3000"]),
            ("scheme mismatch", &["https://localhost:3000"]),
            ("host mismatch", &["http://evil.example:3000"]),
            ("port mismatch", &["http://localhost:3001"]),
        ];

        for (label, origins) in cases {
            let user = crate::db::insert_user(
                &pool,
                &format!("logout_origin_{}", test_utils::uid()),
                "Logout Origin User",
                &format!("logout-origin-{}@example.com", test_utils::uid()),
                None,
            )
            .await
            .unwrap();
            let token = sess::create_session(&pool, user.id).await.unwrap();
            let mut request = Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .header("Cookie", format!("noms_session={token}"));
            for origin in *origins {
                request = request.header(axum::http::header::ORIGIN, *origin);
            }

            let response = make_router(pool.clone())
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{label}");
            assert!(!response
                .headers()
                .contains_key(axum::http::header::SET_COOKIE));
            assert_eq!(
                sess::verify_session(&pool, &token).await.unwrap(),
                user.id,
                "{label} must not revoke the session"
            );
        }
        sess::clear_test_secret();
    }
}
