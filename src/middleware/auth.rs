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

/// Protected routes that require authentication.
///
/// Kept in sync with the `Route` enum in `main.rs` by convention.
static PROTECTED_PATHS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "/dashboard",
        "/recipes/new",
        "/collections",
        "/settings/profile",
        "/settings/accounts",
    ]
    .into_iter()
    .collect()
});

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
    let session_token = jar.get(session::COOKIE_NAME);
    let verified_user_id =
        session_token.and_then(|cookie| session::verify_session(cookie.value()).ok());

    let is_authenticated = verified_user_id.is_some();

    // Check if path is protected
    let is_protected = PROTECTED_PATHS.contains(path.as_str());
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
    if let Some(user_id) = verified_user_id {
        if let Some(cookie) = session_token {
            if session::should_refresh(cookie.value()).unwrap_or(false) {
                if let Ok(new_token) = session::create_session(user_id) {
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
