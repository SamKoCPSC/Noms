//! Logout handler: clears the session cookie and redirects to home.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::auth::session;

/// Handle a logout request.
///
/// Clears the session cookie by setting it with `max-age=0` and redirects
/// the client to the home page (`/`).
pub async fn handle_logout() -> Response {
    let clear_cookie = session::clear_session_cookie();
    let cookie_header = clear_cookie.encoded().to_string();

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        cookie_header.parse().expect("valid cookie header"),
    );
    headers.insert(axum::http::header::LOCATION, "/".parse().unwrap());

    (StatusCode::FOUND, headers, ()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    /// Build a minimal router exposing the logout handler for testing.
    fn make_router() -> axum::Router {
        axum::Router::new().route("/auth/logout", axum::routing::post(handle_logout))
    }

    #[tokio::test]
    async fn logout_returns_302_with_redirect() {
        let app = make_router();
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
        let app = make_router();
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
}
