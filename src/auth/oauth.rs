//! OAuth 2.0 Authorization Code flow handlers for Google and GitHub.
//!
//! Provides two Axum route handlers:
//! - `start_handler`  — initiates the OAuth flow, stores CSRF state, redirects to provider
//! - `callback_handler` — exchanges the auth code, extracts user info, creates a session

use axum::extract::{Path, Query, State};
use axum_extra::extract::cookie::CookieJar;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use chrono::Utc;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{linking, session};
use crate::db;

// ── Type alias for fully-configured OAuth clients ───────────────────────────
//
// After calling `.set_auth_uri()` and `.set_token_uri()`, the typestate
// parameters change from `EndpointNotSet` to `EndpointSet` for those
// endpoints, while the others remain `EndpointNotSet`.

type ConfiguredClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

// ── Application state ──────────────────────────────────────────────────────

/// Shared state for the OAuth handlers.
///
/// Contains the database pool, OAuth client configuration, and a shared
/// HTTP client for outbound requests (token exchange + user info).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Google OAuth client, fully configured with auth/token/redirect URLs.
    pub google_client: ConfiguredClient,
    /// GitHub OAuth client, fully configured with auth/token/redirect URLs.
    pub github_client: ConfiguredClient,
    /// Shared HTTP client used for token exchange and user-info endpoints.
    /// Reusing a single client preserves the internal connection pool.
    pub http_client: reqwest::Client,
}

// ── Query parameter types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StartQuery {
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub state: String,
    pub code: String,
}

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum OAuthError {
    InvalidProvider(String),
    InvalidRedirectUri(String),
    StateNotFound,
    StateExpired,
    ProviderMismatch,
    TokenExchange(String),
    UserInfoExtraction(String),
    DbError(String),
    SessionError(String),
    LinkError(String),
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProvider(p) => write!(f, "Invalid provider: {p}"),
            Self::InvalidRedirectUri(u) => write!(f, "Invalid redirect_uri: {u}"),
            Self::StateNotFound => write!(f, "CSRF state not found"),
            Self::StateExpired => write!(f, "CSRF state expired"),
            Self::ProviderMismatch => write!(f, "Provider mismatch"),
            Self::TokenExchange(e) => write!(f, "Token exchange failed: {e}"),
            Self::UserInfoExtraction(e) => write!(f, "User info extraction failed: {e}"),
            Self::DbError(e) => write!(f, "Database error: {e}"),
            Self::SessionError(e) => write!(f, "Session error: {e}"),
            Self::LinkError(e) => write!(f, "Link error: {e}"),
        }
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            Self::InvalidProvider(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::InvalidRedirectUri(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::StateNotFound => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::StateExpired => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::ProviderMismatch => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::TokenExchange(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::UserInfoExtraction(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::SessionError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            Self::LinkError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, message).into_response()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Validate that the provider is one of the supported values.
fn validate_provider(provider: &str) -> Result<linking::Provider, OAuthError> {
    match provider {
        "google" => Ok(linking::Provider::Google),
        "github" => Ok(linking::Provider::GitHub),
        other => Err(OAuthError::InvalidProvider(other.to_string())),
    }
}

/// Validate that the redirect_uri is a same-origin relative path.
///
/// Must start with `/` and must not contain `://` (i.e., no absolute URLs)
/// and must not start with `//` (i.e., no protocol-relative URLs).
fn validate_redirect_uri(uri: &str) -> Result<(), OAuthError> {
    if !uri.starts_with('/') || uri.starts_with("//") || uri.contains("://") {
        return Err(OAuthError::InvalidRedirectUri(uri.to_string()));
    }
    Ok(())
}

/// Build OAuth clients for Google and GitHub using environment variables.
///
/// Falls back to mock server URLs (localhost:8082) when env vars are not set.
pub fn build_oauth_clients(base_url: &str) -> (ConfiguredClient, ConfiguredClient) {
    let google = BasicClient::new(ClientId::new(env_or(
        "GOOGLE_CLIENT_ID",
        "mock-google-client-id",
    )))
    .set_client_secret(ClientSecret::new(env_or(
        "GOOGLE_CLIENT_SECRET",
        "mock-google-client-secret",
    )))
    .set_auth_uri(
        AuthUrl::new(env_or("GOOGLE_AUTH_URL", "http://localhost:8082/authorize"))
            .expect("invalid Google auth URL"),
    )
    .set_token_uri(
        TokenUrl::new(env_or("GOOGLE_TOKEN_URL", "http://localhost:8082/token"))
            .expect("invalid Google token URL"),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/auth/google/callback", base_url))
            .expect("invalid Google redirect URL"),
    );

    let github = BasicClient::new(ClientId::new(env_or(
        "GITHUB_CLIENT_ID",
        "mock-github-client-id",
    )))
    .set_client_secret(ClientSecret::new(env_or(
        "GITHUB_CLIENT_SECRET",
        "mock-github-client-secret",
    )))
    .set_auth_uri(
        AuthUrl::new(env_or("GITHUB_AUTH_URL", "http://localhost:8082/authorize"))
            .expect("invalid GitHub auth URL"),
    )
    .set_token_uri(
        TokenUrl::new(env_or("GITHUB_TOKEN_URL", "http://localhost:8082/token"))
            .expect("invalid GitHub token URL"),
    )
    .set_redirect_uri(
        RedirectUrl::new(format!("{}/auth/github/callback", base_url))
            .expect("invalid GitHub redirect URL"),
    );

    (google, github)
}

/// Read an environment variable or return a default value.
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// CSRF state lifetime in seconds (10 minutes).
const CSRF_STATE_TTL_SECS: i64 = 600;

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /auth/:provider/start — initiate an OAuth flow.
///
/// Validates the provider and redirect_uri, generates a CSRF state UUID,
/// persists it in the database, then redirects the user-agent to the
/// provider's authorization endpoint.
pub async fn start_handler(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(params): Query<StartQuery>,
) -> Result<impl IntoResponse, OAuthError> {
    let prov = validate_provider(&provider)?;
    validate_redirect_uri(&params.redirect_uri)?;

    // Generate a CSRF state UUID and persist it in the DB.
    let csrf_state = Uuid::new_v4().to_string();

    db::insert_auth_state(
        &state.pool,
        &csrf_state,
        prov.as_str(),
        &params.redirect_uri,
    )
    .await
    .map_err(|e| OAuthError::DbError(e.to_string()))?;

    // Select the appropriate OAuth client.
    let client = match prov {
        linking::Provider::Google => &state.google_client,
        linking::Provider::GitHub => &state.github_client,
        _ => return Err(OAuthError::InvalidProvider(provider)),
    };

    // Build the authorization URL with our state parameter.
    let mut req = client.authorize_url(|| CsrfToken::new(csrf_state.clone()));

    req = req
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()));

    let (auth_url, _csrf_token) = req.url();

    Ok(Redirect::temporary(auth_url.as_ref()))
}

/// GET /auth/:provider/callback — process the OAuth callback.
///
/// Validates the CSRF state, exchanges the authorization code for tokens,
/// extracts user info, links or creates a local user, establishes a session,
/// and redirects the user-agent to the stored redirect_uri with a session cookie.
pub async fn callback_handler(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(params): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<impl IntoResponse, OAuthError> {
    let prov = validate_provider(&provider)?;

    // Retrieve the stored CSRF state.
    let auth_state = db::get_auth_state(&state.pool, &params.state)
        .await
        .map_err(|e| OAuthError::DbError(e.to_string()))?
        .ok_or(OAuthError::StateNotFound)?;

    // Check expiry (10 min TTL).
    let elapsed = Utc::now()
        .signed_duration_since(auth_state.created_at)
        .num_seconds();
    if elapsed > CSRF_STATE_TTL_SECS {
        let _ = db::delete_auth_state(&state.pool, &params.state).await;
        return Err(OAuthError::StateExpired);
    }

    // Verify provider matches.
    if auth_state.provider != prov.as_str() {
        let _ = db::delete_auth_state(&state.pool, &params.state).await;
        return Err(OAuthError::ProviderMismatch);
    }

    // Consume the state so it cannot be reused.
    // If delete returns Ok(false), the state was already consumed — treat as not found.
    let deleted = db::delete_auth_state(&state.pool, &params.state)
        .await
        .map_err(|e| OAuthError::DbError(e.to_string()))?;
    if !deleted {
        return Err(OAuthError::StateNotFound);
    }

    // Check if there's an existing authenticated session
    let existing_user_id = jar
        .get(session::COOKIE_NAME)
        .and_then(|cookie| session::verify_session(cookie.value()).ok());

    // Select the appropriate OAuth client.
    let client = match prov {
        linking::Provider::Google => &state.google_client,
        linking::Provider::GitHub => &state.github_client,
        _ => return Err(OAuthError::InvalidProvider(provider)),
    };

    // Exchange the authorization code for tokens using the shared HTTP client.
    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code.clone()))
        .request_async(&state.http_client)
        .await
        .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

    // Extract user info from the token response.
    let user_info = match prov {
        linking::Provider::Google => {
            extract_google_user_info(&token_response, &state.http_client).await?
        }
        linking::Provider::GitHub => {
            extract_github_user_info(&token_response, &state.http_client).await?
        }
        _ => unreachable!(),
    };

    // Link the OAuth identity to a user (or create a new one).
    let link_result = linking::link_or_create(&state.pool, user_info, existing_user_id)
        .await
        .map_err(|e| OAuthError::LinkError(e.to_string()))?;

    // Create a session JWT.
    let jwt = session::create_session(link_result.user_id)
        .map_err(|e| OAuthError::SessionError(e.to_string()))?;

    // Build the session cookie.
    let cookie = session::build_session_cookie(&jwt);

    // Redirect to the stored redirect_uri with a session cookie.
    Ok((
        StatusCode::SEE_OTHER,
        [
            ("location", auth_state.redirect_uri),
            ("set-cookie", cookie.to_string()),
        ],
    ))
}

// ── User info extraction ────────────────────────────────────────────────────

/// Extract user info from Google by decoding the ID token from the token response.
///
/// The Google token response includes an `id_token` field which is a JWT.
/// We base64url-decode the payload segment to extract user claims.
/// No cryptographic verification is performed — this is acceptable for
/// the mock server in development.
async fn extract_google_user_info(
    token_response: &oauth2::basic::BasicTokenResponse,
    http_client: &reqwest::Client,
) -> Result<linking::OauthUserInfo, OAuthError> {
    // Try to get the ID token from the response extras.
    // The oauth2 v5 crate's BasicTokenResponse uses EmptyExtraTokenFields,
    // which doesn't include an id_token field. Instead, we use Google's
    // userinfo endpoint with the access token.
    let access_token = token_response.access_token().secret();
    let userinfo_url = env_or(
        "GOOGLE_USERINFO_URL",
        "https://www.googleapis.com/oauth2/v3/userinfo",
    );

    // Use the shared HTTP client for the userinfo endpoint.
    let resp: serde_json::Value = http_client
        .get(&userinfo_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            OAuthError::UserInfoExtraction(format!("Google userinfo request failed: {e}"))
        })?
        .json()
        .await
        .map_err(|e| OAuthError::UserInfoExtraction(format!("Google userinfo parse error: {e}")))?;

    Ok(linking::OauthUserInfo {
        provider: linking::Provider::Google,
        provider_uid: resp["sub"].as_str().unwrap_or("").to_string(),
        email: resp["email"].as_str().map(|s| s.to_string()),
        display_name: resp["name"].as_str().unwrap_or("").to_string(),
        avatar_url: resp["picture"].as_str().map(|s| s.to_string()),
    })
}

/// Extract user info from GitHub by calling the /user API endpoint.
///
/// Uses the access token from the OAuth token response.
async fn extract_github_user_info(
    token_response: &oauth2::basic::BasicTokenResponse,
    http_client: &reqwest::Client,
) -> Result<linking::OauthUserInfo, OAuthError> {
    let access_token = token_response.access_token().secret();

    let github_api_url = env_or("GITHUB_API_URL", "https://api.github.com");

    let resp: serde_json::Value = http_client
        .get(format!("{}/user", github_api_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "noms-app")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| OAuthError::UserInfoExtraction(format!("GitHub API request failed: {e}")))?
        .json()
        .await
        .map_err(|e| OAuthError::UserInfoExtraction(format!("GitHub API parse error: {e}")))?;

    Ok(linking::OauthUserInfo {
        provider: linking::Provider::GitHub,
        provider_uid: resp["id"].to_string(),
        email: resp["email"].as_str().map(|s| s.to_string()),
        display_name: resp["login"].as_str().unwrap_or("").to_string(),
        avatar_url: resp["avatar_url"].as_str().map(|s| s.to_string()),
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_provider_google() {
        assert_eq!(
            validate_provider("google").unwrap(),
            linking::Provider::Google
        );
    }

    #[test]
    fn test_validate_provider_github() {
        assert_eq!(
            validate_provider("github").unwrap(),
            linking::Provider::GitHub
        );
    }

    #[test]
    fn test_validate_provider_invalid() {
        assert!(matches!(
            validate_provider("facebook"),
            Err(OAuthError::InvalidProvider(_))
        ));
    }

    #[test]
    fn test_validate_redirect_uri_valid() {
        assert!(validate_redirect_uri("/dashboard").is_ok());
        assert!(validate_redirect_uri("/dashboard?tab=recipes").is_ok());
    }

    #[test]
    fn test_validate_redirect_uri_invalid_absolute() {
        assert!(validate_redirect_uri("https://evil.com").is_err());
    }

    #[test]
    fn test_validate_redirect_uri_invalid_no_slash() {
        assert!(validate_redirect_uri("dashboard").is_err());
    }

    #[test]
    fn test_validate_redirect_uri_invalid_protocol_relative() {
        assert!(validate_redirect_uri("//evil.com").is_err());
    }

    #[test]
    fn test_build_oauth_clients() {
        // Just verify it doesn't panic with defaults.
        let (google, _github) = build_oauth_clients("http://localhost:3000");
        // Verify the client was created and can generate an auth URL.
        let url = google.authorize_url(CsrfToken::new_random).url();
        assert!(!url.0.to_string().is_empty());
    }

    /// Integration tests requiring a database (use `pgtemp`).
    mod db_tests {
        use super::*;
        use crate::test_utils;

        #[tokio::test]
        async fn test_insert_auth_state_with_provider() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());

            db::insert_auth_state(&pool, &state_id, "google", "/dashboard")
                .await
                .unwrap();

            let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
            assert_eq!(state.provider, "google");
            assert_eq!(state.redirect_uri, "/dashboard");
        }

        #[tokio::test]
        async fn test_provider_mismatch_detection() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());

            db::insert_auth_state(&pool, &state_id, "google", "/dashboard")
                .await
                .unwrap();

            let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
            // State stored with "google" provider
            assert_eq!(state.provider, "google");
            // If callback comes for "github", it won't match
            assert_ne!(state.provider, "github");
        }

        #[tokio::test]
        async fn test_state_expiry_check() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());

            db::insert_auth_state(&pool, &state_id, "google", "/dashboard")
                .await
                .unwrap();

            let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
            // Freshly created state should not be expired
            let elapsed = Utc::now()
                .signed_duration_since(state.created_at)
                .num_seconds();
            assert!(elapsed < CSRF_STATE_TTL_SECS);
        }

        /// Verify that a valid session cookie causes link_or_create to link
        /// the new OAuth provider to the existing user, rather than creating
        /// a new user. This is the core fix for Issue #1.
        #[tokio::test]
        async fn test_callback_links_to_existing_session() {
            use axum_extra::extract::cookie::CookieJar;

            let (_db, pool) = test_utils::setup_test_db().await;
            let u = test_utils::uid();

            // Set up a test session secret via env var so we can create/verify tokens
            std::env::set_var("SESSION_SECRET", "test-secret-32-bytes-long-enough!!");

            // Create a user with a Google account
            let user = db::insert_user(
                &pool,
                &format!("sessionlink_{u}"),
                "Session Link User",
                &format!("sessionlink{u}@example.com"),
                None,
            )
            .await
            .unwrap();

            db::insert_oauth_account(
                &pool,
                user.id,
                "google",
                &format!("google-sessionlink-{u}"),
                Some(&format!("sessionlink{u}@example.com")),
                None,
            )
            .await
            .unwrap();

            // Create a valid session JWT for this user
            let jwt = session::create_session(user.id).unwrap();

            // Build a CookieJar with the session cookie
            let cookie = session::build_session_cookie(&jwt);
            let jar = CookieJar::new().add(cookie);

            // Extract the user ID from the session cookie (simulating callback_handler logic)
            let existing_user_id = jar
                .get(session::COOKIE_NAME)
                .and_then(|cookie| session::verify_session(cookie.value()).ok());

            // Verify the session was read correctly
            assert_eq!(existing_user_id, Some(user.id));

            // Now call link_or_create with the extracted user ID (simulating GitHub callback)
            let result = linking::link_or_create(
                &pool,
                linking::OauthUserInfo {
                    provider: linking::Provider::GitHub,
                    provider_uid: format!("github-sessionlink-{u}"),
                    email: Some(format!("sessionlink{u}@example.com")),
                    display_name: "Session Link User".to_string(),
                    avatar_url: None,
                },
                existing_user_id,
            )
            .await
            .unwrap();

            // Should link to the existing user
            assert_eq!(result.user_id, user.id);
            assert!(!result.is_new_user);

            // Verify the GitHub account was created and linked to the existing user
            let github_account =
                db::get_oauth_account_by_provider(&pool, "github", &format!("github-sessionlink-{u}"))
                    .await
                    .unwrap()
                    .unwrap();
            assert_eq!(github_account.user_id, user.id);

            // Verify no new user was created (only one user exists)
            let user_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&pool).await.unwrap();
            assert_eq!(user_count, 1);
        }
    }
}
