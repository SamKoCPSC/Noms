//! User profile API endpoint.
//!
//! Returns the current user's profile as JSON. Used by the client to
//! fetch the user profile after hydration.

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use sqlx::PgPool;

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
    _req: Request<Body>,
) -> Result<Json<crate::auth::context::AuthContext>, (StatusCode, String)> {
    // Check for valid session
    let session_token = jar.get(session::COOKIE_NAME);
    let verified_user_id =
        session_token.and_then(|cookie| session::verify_session(cookie.value()).ok());

    match verified_user_id {
        Some(user_id) => {
            // Fetch user from database
            let user = crate::db::get_user_by_id(&state.pool, user_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

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
        None => {
            // No valid session - return unauthenticated context
            Ok(Json(crate::auth::context::AuthContext::default()))
        }
    }
}
