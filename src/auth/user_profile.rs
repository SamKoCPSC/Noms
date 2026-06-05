//! User profile API endpoint.
//!
//! Returns the current user's profile as JSON. Used by the client to
//! fetch the user profile after hydration.

use axum::body::Body;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::Method;
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use sqlx::PgPool;
use tracing;

use crate::auth::session;

/// Application state for the user profile endpoint.
#[derive(Clone)]
pub struct UserProfileState {
    pub pool: PgPool,
}

/// Handle GET /api/user_profile.
///
/// Reads the session cookie, verifies the user, and returns the user
/// profile as JSON. Returns unauthenticated context if no valid session.
pub async fn handle_user_profile(
    State(state): State<UserProfileState>,
    jar: CookieJar,
    req: Request<Body>,
) -> axum::response::Response {
    // Enforce GET-only method
    if req.method() != Method::GET {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ALLOW,
            "GET".parse().expect("valid Allow header value"),
        );
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            headers,
            "Method Not Allowed",
        )
            .into_response();
    }

    // Check for valid session
    let session_token = jar.get(session::COOKIE_NAME);
    let verified_user_id = if let Some(cookie) = session_token {
        session::verify_session(&state.pool, cookie.value()).await.ok()
    } else {
        None
    };

    match verified_user_id {
        Some(user_id) => {
            match crate::db::get_user_by_id(&state.pool, user_id).await {
                Ok(Some(user)) => {
                    // Convert User to UserProfile
                    let profile = crate::auth::context::UserProfile {
                        id: user.id,
                        username: user.username,
                        display_name: user.display_name,
                        email: user.email,
                        avatar_url: user.avatar_url,
                        bio: user.bio,
                    };

                    let ctx = crate::auth::context::AuthContext {
                        current_user_id: Some(user_id),
                        current_user: Some(profile),
                        is_authenticated: true,
                    };

                    Ok(Json(ctx))
                }
                Ok(None) => Err((StatusCode::NOT_FOUND, "User not found".to_string())),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to fetch user profile");
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "An internal error occurred. Please try again later.".to_string(),
                    ))
                }
            }
        }
        None => {
            // No valid session - return unauthenticated context
            Ok(Json(crate::auth::context::AuthContext::default()))
        }
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body as AxumBody;
    use axum::http::Request;
    use axum::Router;
    use tower::ServiceExt;

    /// Build a minimal router exposing the user_profile handler for testing.
    /// Registers all methods so that requests reach the handler (bypassing
    /// the router-level method filter), allowing us to test the handler's
    /// own method enforcement.
    async fn make_router() -> Router {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        Router::new()
            .route(
                "/api/user_profile",
                axum::routing::get(handle_user_profile)
                    .post(handle_user_profile)
                    .put(handle_user_profile)
                    .delete(handle_user_profile),
            )
            .with_state(UserProfileState { pool })
    }

    #[tokio::test]
    async fn user_profile_get_returns_200_unauthenticated() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Without a valid session cookie, should return 200 with unauthenticated context
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn user_profile_post_returns_405() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        // Verify Allow header is present and contains GET
        let allow_header = response
            .headers()
            .get(axum::http::header::ALLOW)
            .expect("Allow header should be present on 405 response");
        assert_eq!(allow_header, "GET");
    }

    #[tokio::test]
    async fn user_profile_put_returns_405() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let allow_header = response
            .headers()
            .get(axum::http::header::ALLOW)
            .expect("Allow header should be present on 405 response");
        assert_eq!(allow_header, "GET");
    }

    #[tokio::test]
    async fn user_profile_delete_returns_405() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let allow_header = response
            .headers()
            .get(axum::http::header::ALLOW)
            .expect("Allow header should be present on 405 response");
        assert_eq!(allow_header, "GET");
    }
}
