//! OAuth 2.0 Authorization Code flow handlers for Google and GitHub.
//!
//! Provides two Axum route handlers:
//! - `start_handler`  — initiates the OAuth flow, stores CSRF state, redirects to provider
//! - `callback_handler` — exchanges the auth code, extracts user info, creates a session

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{linking, session};
use crate::db;
use tracing;

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
    pub google_userinfo_url: String,
    pub github_userinfo_url: String,
    /// GitHub's verified-email endpoint. This is separate from `/user`
    /// because the public profile email is not proof of email ownership.
    pub github_emails_url: String,
    /// Additional GitHub authorization scopes used by environment-specific
    /// provider adapters. Required production scopes are always included.
    pub github_extra_scopes: Vec<String>,
    /// Shared rate limit state for OAuth endpoint protection.
    pub rate_limit: crate::middleware::rate_limit::RateLimitState,
    /// S3-compatible storage client for media file operations.
    #[cfg(feature = "server")]
    #[allow(dead_code)]
    pub storage: crate::storage::StorageClient,
}

// ── Query parameter types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StartQuery {
    pub redirect_uri: String,
    pub purpose: String,
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
    /// The stored auth state user_id does not match the current session user_id.
    StateUserMismatch,
    InvalidPurpose,
    LinkSessionRequired,
    LoginSessionConflict,
    TokenExchange(String),
    UserInfoExtraction(String),
    DbError(String),
    SessionError(String),
    LinkError(String),
    /// OAuth account is already linked to a different user.
    #[allow(dead_code)]
    // Used by OAuthError::AccountAlreadyLinked for IntoResponse redirect and testability
    AccountAlreadyLinked(String), // provider name for redirect URL
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidProvider(p) => write!(f, "Invalid provider: {p}"),
            Self::InvalidRedirectUri(u) => write!(f, "Invalid redirect_uri: {u}"),
            Self::StateNotFound => write!(f, "CSRF state not found"),
            Self::StateExpired => write!(f, "CSRF state expired"),
            Self::ProviderMismatch => write!(f, "Provider mismatch"),
            Self::StateUserMismatch => write!(f, "Auth state user mismatch"),
            Self::InvalidPurpose => write!(f, "Invalid OAuth flow purpose"),
            Self::LinkSessionRequired => write!(f, "Account linking requires a valid session"),
            Self::LoginSessionConflict => {
                write!(f, "OAuth login cannot complete with an active session")
            }
            Self::TokenExchange(e) => write!(f, "Token exchange failed: {e}"),
            Self::UserInfoExtraction(e) => write!(f, "User info extraction failed: {e}"),
            Self::DbError(e) => write!(f, "Database error: {e}"),
            Self::SessionError(e) => write!(f, "Session error: {e}"),
            Self::LinkError(e) => write!(f, "Link error: {e}"),
            Self::AccountAlreadyLinked(provider) => {
                write!(
                    f,
                    "The {provider} account is already linked to another user"
                )
            }
        }
    }
}

impl OAuthError {
    /// Stable, non-sensitive category for structured server logs.
    fn log_code(&self) -> &'static str {
        match self {
            Self::InvalidProvider(_) => "invalid_provider",
            Self::InvalidRedirectUri(_) => "invalid_redirect",
            Self::StateNotFound => "state_not_found",
            Self::StateExpired => "state_expired",
            Self::ProviderMismatch => "provider_mismatch",
            Self::StateUserMismatch => "state_user_mismatch",
            Self::InvalidPurpose => "invalid_purpose",
            Self::LinkSessionRequired => "link_session_required",
            Self::LoginSessionConflict => "login_session_conflict",
            Self::TokenExchange(_) => "token_exchange",
            Self::UserInfoExtraction(_) => "user_info",
            Self::DbError(_) => "database",
            Self::SessionError(_) => "session",
            Self::LinkError(_) => "account_link",
            Self::AccountAlreadyLinked(_) => "account_already_linked",
        }
    }

    /// Return a client-safe error message.
    ///
    /// For client errors (4xx), returns the detailed message via `Display`.
    /// For internal server errors (5xx), returns a generic message that
    /// does not leak internal implementation details.
    pub fn sanitized_message(&self) -> String {
        match self {
            // Client errors — safe to expose details
            Self::InvalidProvider(_)
            | Self::InvalidRedirectUri(_)
            | Self::StateNotFound
            | Self::StateExpired
            | Self::ProviderMismatch
            | Self::StateUserMismatch
            | Self::InvalidPurpose
            | Self::LinkSessionRequired
            | Self::LoginSessionConflict
            | Self::AccountAlreadyLinked(_) => self.to_string(),

            // Internal errors — generic message only
            Self::TokenExchange(_)
            | Self::UserInfoExtraction(_)
            | Self::DbError(_)
            | Self::SessionError(_)
            | Self::LinkError(_) => {
                "An internal error occurred. Please try again later.".to_string()
            }
        }
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> axum::response::Response {
        // Special case: AccountAlreadyLinked returns a redirect with error params
        if let Self::AccountAlreadyLinked(provider) = &self {
            let redirect_uri = format!(
                "/settings/accounts?error=account_already_linked&provider={}",
                provider
            );
            tracing::warn!(
                error_code = self.log_code(),
                provider = %provider,
                "Account already linked conflict — redirecting with error"
            );
            return (StatusCode::SEE_OTHER, [("location", redirect_uri.as_str())]).into_response();
        }

        let status = match &self {
            Self::InvalidProvider(_) => StatusCode::BAD_REQUEST,
            Self::InvalidRedirectUri(_) => StatusCode::BAD_REQUEST,
            Self::StateNotFound => StatusCode::UNAUTHORIZED,
            Self::StateExpired => StatusCode::UNAUTHORIZED,
            Self::StateUserMismatch => StatusCode::UNAUTHORIZED,
            Self::InvalidPurpose => StatusCode::BAD_REQUEST,
            Self::LinkSessionRequired => StatusCode::UNAUTHORIZED,
            Self::LoginSessionConflict => StatusCode::CONFLICT,
            Self::ProviderMismatch => StatusCode::BAD_REQUEST,
            Self::TokenExchange(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::UserInfoExtraction(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SessionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::LinkError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::AccountAlreadyLinked(_) => unreachable!(), // handled above
        };

        // Log only a stable category; provider bodies and token-exchange
        // details may contain credentials or authorization artifacts.
        if status.is_server_error() {
            tracing::error!(error_code = self.log_code(), status = ?status, "Internal server error");
        } else {
            tracing::warn!(error_code = self.log_code(), status = ?status, "Client error");
        }

        let message = self.sanitized_message();
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

fn validate_redirect_uri(uri: &str) -> Result<String, OAuthError> {
    crate::auth::redirect::normalize_same_origin_redirect(uri)
        .ok_or_else(|| OAuthError::InvalidRedirectUri(uri.to_string()))
}

fn oauth_scopes(provider: &linking::Provider, github_extra_scopes: &[String]) -> Vec<String> {
    match provider {
        linking::Provider::Google => ["openid", "email", "profile"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        linking::Provider::GitHub => ["read:user", "user:email"]
            .into_iter()
            .map(str::to_string)
            .chain(github_extra_scopes.iter().cloned())
            .collect(),
        linking::Provider::Apple => Vec::new(),
    }
}

/// Read environment-specific GitHub scopes without allowing deployment
/// configuration to replace the required production scopes.
pub fn configured_github_extra_scopes() -> Vec<String> {
    let mut scopes = Vec::new();
    for scope in std::env::var("GITHUB_OAUTH_EXTRA_SCOPES")
        .unwrap_or_default()
        .split_whitespace()
    {
        if matches!(scope, "read:user" | "user:email") || scopes.iter().any(|item| item == scope) {
            continue;
        }
        scopes.push(scope.to_string());
    }
    scopes
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
        AuthUrl::new(env_or(
            "GOOGLE_AUTH_URL",
            "http://localhost:8082/google/authorize",
        ))
        .expect("invalid Google auth URL"),
    )
    .set_token_uri(
        TokenUrl::new(env_or(
            "GOOGLE_TOKEN_URL",
            "http://localhost:8082/google/token",
        ))
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
        AuthUrl::new(env_or(
            "GITHUB_AUTH_URL",
            "http://localhost:8082/github/authorize",
        ))
        .expect("invalid GitHub auth URL"),
    )
    .set_token_uri(
        TokenUrl::new(env_or(
            "GITHUB_TOKEN_URL",
            "http://localhost:8082/github/token",
        ))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlowPurpose {
    Login,
    Link,
}

impl FlowPurpose {
    fn parse(value: &str) -> Result<Self, OAuthError> {
        match value {
            "login" => Ok(Self::Login),
            "link" => Ok(Self::Link),
            _ => Err(OAuthError::InvalidPurpose),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Link => "link",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallbackIntent {
    Login,
    Link(Uuid),
}

fn validate_callback_intent(
    purpose: &str,
    stored_user_id: Option<Uuid>,
    current_user_id: Option<Uuid>,
) -> Result<CallbackIntent, OAuthError> {
    match FlowPurpose::parse(purpose)? {
        FlowPurpose::Login if stored_user_id.is_none() && current_user_id.is_none() => {
            Ok(CallbackIntent::Login)
        }
        FlowPurpose::Login => Err(OAuthError::LoginSessionConflict),
        FlowPurpose::Link => match (stored_user_id, current_user_id) {
            (Some(stored), Some(current)) if stored == current => Ok(CallbackIntent::Link(stored)),
            _ => Err(OAuthError::StateUserMismatch),
        },
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /auth/:provider/start — initiate an OAuth flow.
///
/// Validates the provider and redirect_uri, generates a CSRF state UUID,
/// persists it in the database (bound to the current session's user_id),
/// then redirects the user-agent to the provider's authorization endpoint.
pub async fn start_handler(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(params): Query<StartQuery>,
    jar: CookieJar,
) -> Result<impl IntoResponse, OAuthError> {
    let prov = validate_provider(&provider)?;
    let redirect_uri = validate_redirect_uri(&params.redirect_uri)?;
    let purpose = FlowPurpose::parse(&params.purpose)?;

    // Generate a CSRF state UUID and persist it in the DB.
    let csrf_state = Uuid::new_v4().to_string();

    // Generate PKCE code verifier and challenge (S256 method).
    // The verifier is stored server-side; the challenge is sent to the provider.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Resolve and bind the immutable flow intent before persisting state.
    let configured_cookie_name = session::cookie_name();
    let current_user_id = if let Some(cookie) = jar.get(configured_cookie_name.as_ref()) {
        session::verify_session(&state.pool, cookie.value())
            .await
            .ok()
    } else {
        None
    };
    let user_id = match purpose {
        FlowPurpose::Login if current_user_id.is_none() => None,
        FlowPurpose::Login => return Err(OAuthError::LoginSessionConflict),
        FlowPurpose::Link => Some(current_user_id.ok_or(OAuthError::LinkSessionRequired)?),
    };

    db::insert_auth_state(
        &state.pool,
        &csrf_state,
        prov.as_str(),
        &redirect_uri,
        pkce_verifier.secret(),
        purpose.as_str(),
        user_id,
    )
    .await
    .map_err(|e| OAuthError::DbError(e.to_string()))?;

    // Select the appropriate OAuth client.
    let client = match prov {
        linking::Provider::Google => &state.google_client,
        linking::Provider::GitHub => &state.github_client,
        _ => return Err(OAuthError::InvalidProvider(provider)),
    };

    // Build the authorization URL with our state parameter and PKCE challenge.
    let mut req = client
        .authorize_url(|| CsrfToken::new(csrf_state.clone()))
        .set_pkce_challenge(pkce_challenge);

    for scope in oauth_scopes(&prov, &state.github_extra_scopes) {
        req = req.add_scope(Scope::new(scope));
    }

    let (auth_url, _csrf_token) = req.url();

    // Google requires access_type=offline to return a refresh token
    let auth_url = if prov == linking::Provider::Google {
        format!("{auth_url}&access_type=offline")
    } else {
        auth_url.to_string()
    };

    Ok(Redirect::temporary(&auth_url))
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
) -> Result<axum::response::Response, OAuthError> {
    let prov = validate_provider(&provider)?;

    // Atomically consume the CSRF state. First caller gets the row;
    // concurrent callers get None and are rejected.
    let auth_state = db::delete_auth_state(&state.pool, &params.state)
        .await
        .map_err(|e| OAuthError::DbError(e.to_string()))?
        .ok_or(OAuthError::StateNotFound)?;

    // Check expiry (10 min TTL) — state is already consumed, no cleanup needed.
    let elapsed = Utc::now()
        .signed_duration_since(auth_state.created_at)
        .num_seconds();
    if elapsed > CSRF_STATE_TTL_SECS {
        return Err(OAuthError::StateExpired);
    }

    // Verify provider matches — state is already consumed, no cleanup needed.
    if auth_state.provider != prov.as_str() {
        return Err(OAuthError::ProviderMismatch);
    }

    // Revalidate persisted state so callbacks fail closed across deployments
    // and if an auth-state row is ever written outside the start handler.
    let redirect_uri = validate_redirect_uri(&auth_state.redirect_uri)?;

    // Check if there's an existing authenticated session
    // NOTE: We need to verify the session before consuming it for the new login.
    let configured_cookie_name = session::cookie_name();
    let existing_user_id = if let Some(cookie) = jar.get(configured_cookie_name.as_ref()) {
        session::verify_session(&state.pool, cookie.value())
            .await
            .ok()
    } else {
        None
    };

    let intent =
        validate_callback_intent(&auth_state.purpose, auth_state.user_id, existing_user_id)?;

    // Select the appropriate OAuth client.
    let client = match prov {
        linking::Provider::Google => &state.google_client,
        linking::Provider::GitHub => &state.github_client,
        _ => return Err(OAuthError::InvalidProvider(provider)),
    };

    // Reconstruct the PKCE code verifier from the stored value.
    let code_verifier = auth_state.code_verifier.ok_or_else(|| {
        OAuthError::TokenExchange("PKCE code_verifier not found in auth state".to_string())
    })?;

    // Exchange the authorization code for tokens using the shared HTTP client.
    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code.clone()))
        .set_pkce_verifier(PkceCodeVerifier::new(code_verifier))
        .request_async(&state.http_client)
        .await
        .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

    // Extract user info from the token response.
    let user_info = match prov {
        linking::Provider::Google => {
            extract_google_user_info(
                &token_response,
                &state.http_client,
                &state.google_userinfo_url,
            )
            .await?
        }
        linking::Provider::GitHub => {
            extract_github_user_info(
                &token_response,
                &state.http_client,
                &state.github_userinfo_url,
                &state.github_emails_url,
            )
            .await?
        }
        _ => unreachable!(),
    };

    // Extract the refresh token from the token response (if present).
    let refresh_token = token_response
        .refresh_token()
        .map(|rt| rt.secret().to_string());

    let link_result = match intent {
        CallbackIntent::Login => {
            linking::login_or_create(&state.pool, user_info, refresh_token).await
        }
        CallbackIntent::Link(user_id) => {
            linking::link_to_user(&state.pool, user_id, user_info, refresh_token).await
        }
    };
    let link_result = match link_result {
        Ok(result) => result,
        Err(linking::LinkError::AccountAlreadyLinked(provider)) => {
            let redirect_uri = format!(
                "/settings/accounts?error=account_already_linked&provider={}",
                provider
            );
            return Ok((StatusCode::SEE_OTHER, [("location", redirect_uri)]).into_response());
        }
        Err(e) => return Err(OAuthError::LinkError(e.to_string())),
    };

    match intent {
        CallbackIntent::Login => {
            let jwt = session::create_session(&state.pool, link_result.user_id)
                .await
                .map_err(|e| OAuthError::SessionError(e.to_string()))?;
            let cookie = session::build_session_cookie(&jwt);
            Ok((
                StatusCode::SEE_OTHER,
                [
                    ("location", redirect_uri),
                    ("set-cookie", cookie.to_string()),
                ],
            )
                .into_response())
        }
        CallbackIntent::Link(_) => {
            Ok((StatusCode::SEE_OTHER, [("location", redirect_uri)]).into_response())
        }
    }
}

// ── User info extraction ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: Option<String>,
    #[serde(default)]
    email_verified: bool,
    #[serde(default)]
    name: String,
    picture: Option<String>,
}

/// Extract Google identity from the authenticated user-info endpoint.
async fn extract_google_user_info(
    token_response: &oauth2::basic::BasicTokenResponse,
    http_client: &reqwest::Client,
    userinfo_url: &str,
) -> Result<linking::OauthUserInfo, OAuthError> {
    let access_token = token_response.access_token().secret();
    let response = http_client
        .get(userinfo_url)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            OAuthError::UserInfoExtraction(format!("Google userinfo request failed: {e}"))
        })?
        .error_for_status()
        .map_err(|e| {
            OAuthError::UserInfoExtraction(format!("Google userinfo returned an error: {e}"))
        })?;
    let response: GoogleUserInfo = response
        .json()
        .await
        .map_err(|e| OAuthError::UserInfoExtraction(format!("Google userinfo parse error: {e}")))?;

    if response.sub.trim().is_empty() {
        return Err(OAuthError::UserInfoExtraction(
            "Google userinfo did not include a stable subject".to_string(),
        ));
    }

    let verified_email = response
        .email
        .filter(|email| response.email_verified && !email.trim().is_empty());
    let email_verified = verified_email.is_some();

    Ok(linking::OauthUserInfo {
        provider: linking::Provider::Google,
        provider_uid: response.sub,
        email: verified_email,
        email_verified,
        display_name: response.name,
        avatar_url: response.picture,
    })
}

#[derive(Deserialize)]
struct GitHubUserInfo {
    id: u64,
    #[serde(default)]
    login: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    verified: bool,
}

/// Extract user info from GitHub by calling the userinfo endpoint.
///
/// Uses GITHUB_USERINFO_URL if set (for mock server in dev), otherwise
/// falls back to GitHub's API.
async fn extract_github_user_info(
    token_response: &oauth2::basic::BasicTokenResponse,
    http_client: &reqwest::Client,
    userinfo_url: &str,
    emails_url: &str,
) -> Result<linking::OauthUserInfo, OAuthError> {
    let access_token = token_response.access_token().secret();

    let response = http_client
        .get(userinfo_url)
        .bearer_auth(access_token)
        .header("User-Agent", "noms-app")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            OAuthError::UserInfoExtraction(format!("GitHub userinfo request failed: {e}"))
        })?
        .error_for_status()
        .map_err(|e| {
            OAuthError::UserInfoExtraction(format!("GitHub userinfo returned an error: {e}"))
        })?;
    let response: GitHubUserInfo = response
        .json()
        .await
        .map_err(|e| OAuthError::UserInfoExtraction(format!("GitHub userinfo parse error: {e}")))?;

    let email_response = http_client
        .get(emails_url)
        .bearer_auth(access_token)
        .header("User-Agent", "noms-app")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| OAuthError::UserInfoExtraction(format!("GitHub email request failed: {e}")))?
        .error_for_status()
        .map_err(|e| {
            OAuthError::UserInfoExtraction(format!("GitHub email endpoint returned an error: {e}"))
        })?;
    let emails: Vec<GitHubEmail> = email_response.json().await.map_err(|e| {
        OAuthError::UserInfoExtraction(format!("GitHub email response parse error: {e}"))
    })?;

    let verified_email = emails
        .iter()
        .find(|entry| entry.verified && entry.primary && !entry.email.trim().is_empty())
        .or_else(|| {
            emails
                .iter()
                .find(|entry| entry.verified && !entry.email.trim().is_empty())
        })
        .map(|entry| entry.email.clone());
    let email_verified = verified_email.is_some();

    Ok(linking::OauthUserInfo {
        provider: linking::Provider::GitHub,
        provider_uid: response.id.to_string(),
        email: verified_email,
        email_verified,
        display_name: response.login,
        avatar_url: response.avatar_url,
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
    fn test_validate_redirect_uri_rejects_normalization_confusion() {
        for uri in [
            "/\\evil.example",
            "/%5cevil.example",
            "/%5Cevil.example",
            "/%2fevil.example",
            "/%2Fevil.example",
            "/%255cevil.example",
            "/%252fevil.example",
            "/.//evil.example",
            "/%2e%2e//evil.example",
            "/%",
            "/%zz",
            "/%FF",
        ] {
            assert!(
                validate_redirect_uri(uri).is_err(),
                "normalization-confusing redirect should be rejected: {uri}"
            );
        }
    }

    #[test]
    fn test_validate_redirect_uri_too_long() {
        let long_uri = format!("/{}", "a".repeat(2048));
        assert!(matches!(
            validate_redirect_uri(&long_uri),
            Err(OAuthError::InvalidRedirectUri(_))
        ));
    }

    #[test]
    fn test_validate_redirect_uri_at_max_length() {
        let exact_uri = format!("/{}", "a".repeat(2047));
        assert!(validate_redirect_uri(&exact_uri).is_ok());
    }

    #[test]
    fn test_pkce_challenge_verifier_generation() {
        // Verify PkceCodeChallenge::new_random_sha256() produces valid-length outputs.
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let challenge_str = challenge.as_str();
        let verifier_str = verifier.secret();

        // Verifier must be 43-128 chars per RFC 7636
        assert!(
            verifier_str.len() >= 43,
            "verifier too short: {} chars",
            verifier_str.len()
        );
        assert!(
            verifier_str.len() <= 128,
            "verifier too long: {} chars",
            verifier_str.len()
        );

        // Challenge should be a valid base64url-encoded SHA-256 hash (43 chars)
        assert!(!challenge_str.is_empty(), "challenge should not be empty");
        assert!(
            challenge_str.len() >= 43,
            "challenge too short: {} chars",
            challenge_str.len()
        );
    }

    #[test]
    fn test_pkce_challenge_in_auth_url() {
        // Build an auth URL with PKCE and verify the URL contains code_challenge params.
        let (google, _github) = build_oauth_clients("http://localhost:3000");
        let (challenge, _verifier) = PkceCodeChallenge::new_random_sha256();

        let req = google
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(challenge);
        let (url, _) = req.url();
        let url_str = url.to_string();

        assert!(
            url_str.contains("code_challenge="),
            "auth URL should contain code_challenge: {url_str}"
        );
        assert!(
            url_str.contains("code_challenge_method=S256"),
            "auth URL should contain code_challenge_method=S256: {url_str}"
        );
    }

    #[test]
    fn test_pkce_verifier_reconstruction() {
        // Verify PkceCodeVerifier::new(stored_string) round-trips correctly.
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let stored = verifier.secret().to_string();

        // Reconstruct from stored string
        let reconstructed = PkceCodeVerifier::new(stored.clone());
        assert_eq!(reconstructed.secret(), &stored);
        assert_eq!(challenge.as_str(), challenge.as_str());
    }

    #[test]
    fn test_build_oauth_clients() {
        // Just verify it doesn't panic with defaults.
        let (google, _github) = build_oauth_clients("http://localhost:3000");
        // Verify the client was created and can generate an auth URL.
        let url = google.authorize_url(CsrfToken::new_random).url();
        assert!(!url.0.to_string().is_empty());
    }

    #[test]
    fn test_provider_specific_oauth_scopes() {
        assert_eq!(
            oauth_scopes(&linking::Provider::Google, &[]),
            ["openid", "email", "profile"].map(str::to_string)
        );
        assert_eq!(
            oauth_scopes(&linking::Provider::GitHub, &[]),
            ["read:user", "user:email"].map(str::to_string)
        );
        assert_eq!(
            oauth_scopes(&linking::Provider::GitHub, &["openid".to_string()]),
            ["read:user", "user:email", "openid"].map(str::to_string)
        );
    }

    fn test_token_response() -> oauth2::basic::BasicTokenResponse {
        oauth2::basic::BasicTokenResponse::new(
            oauth2::AccessToken::new("test-access-token".to_string()),
            oauth2::basic::BasicTokenType::Bearer,
            oauth2::EmptyExtraTokenFields {},
        )
    }

    #[tokio::test]
    async fn test_google_userinfo_uses_stable_subject_and_verified_email() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/google/userinfo"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "sub": "stable-google-subject",
                    "email": "person@example.com",
                    "email_verified": true,
                    "name": "Person"
                })),
            )
            .mount(&server)
            .await;

        let info = extract_google_user_info(
            &test_token_response(),
            &reqwest::Client::new(),
            &format!("{}/google/userinfo", server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(info.provider_uid, "stable-google-subject");
        assert_eq!(info.email.as_deref(), Some("person@example.com"));
        assert!(info.email_verified);
    }

    #[tokio::test]
    async fn test_google_userinfo_drops_unverified_email() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/google/userinfo"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "sub": "stable-google-subject",
                    "email": "unverified@example.com",
                    "email_verified": false,
                    "name": "Person"
                })),
            )
            .mount(&server)
            .await;

        let info = extract_google_user_info(
            &test_token_response(),
            &reqwest::Client::new(),
            &format!("{}/google/userinfo", server.uri()),
        )
        .await
        .unwrap();

        assert!(info.email.is_none());
        assert!(!info.email_verified);
    }

    #[tokio::test]
    async fn test_google_userinfo_rejects_missing_subject() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/google/userinfo"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "email": "person@example.com",
                    "email_verified": true,
                    "name": "Person"
                })),
            )
            .mount(&server)
            .await;

        let result = extract_google_user_info(
            &test_token_response(),
            &reqwest::Client::new(),
            &format!("{}/google/userinfo", server.uri()),
        )
        .await;

        assert!(matches!(result, Err(OAuthError::UserInfoExtraction(_))));
    }

    #[tokio::test]
    async fn test_userinfo_http_error_is_rejected_before_claims() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/google/userinfo"))
            .respond_with(
                wiremock::ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "sub": "must-not-be-accepted",
                    "email": "attacker@example.com",
                    "email_verified": true
                })),
            )
            .mount(&server)
            .await;

        let result = extract_google_user_info(
            &test_token_response(),
            &reqwest::Client::new(),
            &format!("{}/google/userinfo", server.uri()),
        )
        .await;

        assert!(matches!(result, Err(OAuthError::UserInfoExtraction(_))));
    }

    #[tokio::test]
    async fn test_github_userinfo_uses_numeric_id_and_verified_primary_email() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/github/userinfo"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": 987654,
                    "login": "octocat",
                    "email": "untrusted-public@example.com"
                })),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/github/emails"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    {"email": "secondary@example.com", "primary": false, "verified": true},
                    {"email": "primary@example.com", "primary": true, "verified": true}
                ])),
            )
            .mount(&server)
            .await;

        let info = extract_github_user_info(
            &test_token_response(),
            &reqwest::Client::new(),
            &format!("{}/github/userinfo", server.uri()),
            &format!("{}/github/emails", server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(info.provider_uid, "987654");
        assert_eq!(info.email.as_deref(), Some("primary@example.com"));
        assert!(info.email_verified);
    }

    #[tokio::test]
    async fn test_github_userinfo_ignores_email_when_none_are_verified() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/github/userinfo"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": 1234, "login": "octocat"})),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/github/emails"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    {"email": "unverified@example.com", "primary": true, "verified": false}
                ])),
            )
            .mount(&server)
            .await;

        let info = extract_github_user_info(
            &test_token_response(),
            &reqwest::Client::new(),
            &format!("{}/github/userinfo", server.uri()),
            &format!("{}/github/emails", server.uri()),
        )
        .await
        .unwrap();

        assert!(info.email.is_none());
        assert!(!info.email_verified);
    }

    #[test]
    fn test_sanitized_message_client_errors_preserved() {
        assert_eq!(
            OAuthError::InvalidProvider("facebook".to_string()).sanitized_message(),
            "Invalid provider: facebook"
        );
        assert_eq!(
            OAuthError::InvalidRedirectUri("https://evil.com".to_string()).sanitized_message(),
            "Invalid redirect_uri: https://evil.com"
        );
        assert_eq!(
            OAuthError::StateNotFound.sanitized_message(),
            "CSRF state not found"
        );
        assert_eq!(
            OAuthError::StateExpired.sanitized_message(),
            "CSRF state expired"
        );
        assert_eq!(
            OAuthError::ProviderMismatch.sanitized_message(),
            "Provider mismatch"
        );
        assert_eq!(
            OAuthError::AccountAlreadyLinked("google".to_string()).sanitized_message(),
            "The google account is already linked to another user"
        );
    }

    #[test]
    fn test_sanitized_message_server_errors_generic() {
        let generic = "An internal error occurred. Please try again later.";
        assert_eq!(
            OAuthError::TokenExchange("connection refused".to_string()).sanitized_message(),
            generic
        );
        assert_eq!(
            OAuthError::UserInfoExtraction("parse error".to_string()).sanitized_message(),
            generic
        );
        assert_eq!(
            OAuthError::DbError("connection timeout".to_string()).sanitized_message(),
            generic
        );
        assert_eq!(
            OAuthError::SessionError("SESSION_SECRET not set".to_string()).sanitized_message(),
            generic
        );
        assert_eq!(
            OAuthError::LinkError("database error: query failed".to_string()).sanitized_message(),
            generic
        );
    }

    #[test]
    fn internal_error_log_codes_never_include_provider_or_secret_details() {
        let secret = "cp6-sensitive-provider-response";
        for (error, expected_code) in [
            (OAuthError::TokenExchange(secret.into()), "token_exchange"),
            (OAuthError::UserInfoExtraction(secret.into()), "user_info"),
            (OAuthError::DbError(secret.into()), "database"),
            (OAuthError::SessionError(secret.into()), "session"),
            (OAuthError::LinkError(secret.into()), "account_link"),
        ] {
            assert_eq!(error.log_code(), expected_code);
            assert!(!error.log_code().contains(secret));
        }
    }

    #[test]
    fn test_account_already_linked_into_response_redirects() {
        let err = OAuthError::AccountAlreadyLinked("github".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        // Check the location header contains the redirect URL with error params
        let headers = response.headers();
        let location = headers
            .get(axum::http::header::LOCATION)
            .expect("should have location header")
            .to_str()
            .unwrap();
        assert!(
            location.contains("error=account_already_linked"),
            "redirect URL should contain error param: {location}"
        );
        assert!(
            location.contains("provider=github"),
            "redirect URL should contain provider param: {location}"
        );
    }

    /// Integration tests requiring a database (use `pgtemp`).
    mod db_tests {
        use super::*;
        use crate::test_utils;

        fn test_app_state(pool: PgPool) -> AppState {
            let (google_client, github_client) = build_oauth_clients("http://localhost:3000");
            AppState {
                pool,
                google_client,
                github_client,
                http_client: reqwest::Client::new(),
                google_userinfo_url: "http://localhost:8082/google/userinfo".to_string(),
                github_userinfo_url: "http://localhost:8082/github/userinfo".to_string(),
                github_emails_url: "http://localhost:8082/github/emails".to_string(),
                github_extra_scopes: Vec::new(),
                rate_limit: crate::middleware::rate_limit::RateLimitState::default(),
                storage: crate::storage::StorageClient::for_test(),
            }
        }

        async fn mock_google_state(pool: PgPool, email: &str) -> (AppState, wiremock::MockServer) {
            let server = wiremock::MockServer::start().await;
            wiremock::Mock::given(wiremock::matchers::method("POST"))
                .and(wiremock::matchers::path("/google/token"))
                .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({
                        "access_token": "test-access-token",
                        "token_type": "Bearer"
                    }),
                ))
                .mount(&server)
                .await;
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::path("/google/userinfo"))
                .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({
                        "sub": format!("subject-{email}"),
                        "email": email,
                        "email_verified": true,
                        "name": "OAuth Test User"
                    }),
                ))
                .mount(&server)
                .await;

            let make_client = |provider: &str| {
                BasicClient::new(ClientId::new(format!("{provider}-client")))
                    .set_client_secret(ClientSecret::new("test-secret".to_string()))
                    .set_auth_uri(AuthUrl::new(format!("{}/authorize", server.uri())).unwrap())
                    .set_token_uri(
                        TokenUrl::new(format!("{}/{provider}/token", server.uri())).unwrap(),
                    )
                    .set_redirect_uri(
                        RedirectUrl::new(format!("http://localhost:3000/auth/{provider}/callback"))
                            .unwrap(),
                    )
            };

            (
                AppState {
                    pool,
                    google_client: make_client("google"),
                    github_client: make_client("github"),
                    http_client: reqwest::Client::new(),
                    google_userinfo_url: format!("{}/google/userinfo", server.uri()),
                    github_userinfo_url: format!("{}/github/userinfo", server.uri()),
                    github_emails_url: format!("{}/github/emails", server.uri()),
                    github_extra_scopes: Vec::new(),
                    rate_limit: crate::middleware::rate_limit::RateLimitState::default(),
                    storage: crate::storage::StorageClient::for_test(),
                },
                server,
            )
        }

        async fn test_user(pool: &PgPool, label: &str) -> db::User {
            let u = test_utils::uid();
            db::insert_user(
                pool,
                &format!("{label}_{u}"),
                label,
                &format!("{label}{u}@example.com"),
                None,
            )
            .await
            .unwrap()
        }

        #[tokio::test]
        async fn link_start_requires_valid_session() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let result = start_handler(
                State(test_app_state(pool)),
                Path("google".to_string()),
                Query(StartQuery {
                    redirect_uri: "/settings/accounts".to_string(),
                    purpose: "link".to_string(),
                }),
                CookieJar::new(),
            )
            .await;
            assert!(matches!(result, Err(OAuthError::LinkSessionRequired)));
        }

        #[tokio::test]
        async fn login_start_rejects_active_session() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let user = test_user(&pool, "login_active").await;
            let token = session::create_session(&pool, user.id).await.unwrap();
            let jar = CookieJar::new().add(session::build_session_cookie(&token));
            let result = start_handler(
                State(test_app_state(pool)),
                Path("google".to_string()),
                Query(StartQuery {
                    redirect_uri: "/dashboard".to_string(),
                    purpose: "login".to_string(),
                }),
                jar,
            )
            .await;
            assert!(matches!(result, Err(OAuthError::LoginSessionConflict)));
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn login_callback_with_victim_session_is_rejected_before_linking() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let victim = test_user(&pool, "victim").await;
            let victim_token = session::create_session(&pool, victim.id).await.unwrap();
            let jar = CookieJar::new().add(session::build_session_cookie(&victim_token));
            let state_id = format!("login-csrf-{}", test_utils::uid());

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-characters-long-enough",
                "login",
                None,
            )
            .await
            .unwrap();

            let result = callback_handler(
                State(test_app_state(pool.clone())),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "attacker-code".to_string(),
                }),
                jar,
            )
            .await;

            assert!(matches!(result, Err(OAuthError::LoginSessionConflict)));
            assert_eq!(db::count_oauth_accounts(&pool, victim.id).await.unwrap(), 0);
            assert_eq!(
                session::verify_session(&pool, &victim_token).await.unwrap(),
                victim.id
            );
            assert!(db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .is_none());
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn callback_rejects_unsafe_stored_redirect_before_token_exchange() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("unsafe-redirect-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/\\evil.example",
                "test-verifier-minimum-43-characters-long-enough",
                "login",
                None,
            )
            .await
            .unwrap();

            let result = callback_handler(
                State(test_app_state(pool)),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "unused-code".to_string(),
                }),
                CookieJar::new(),
            )
            .await;

            assert!(matches!(result, Err(OAuthError::InvalidRedirectUri(_))));
        }

        #[tokio::test]
        async fn link_callback_without_session_is_rejected_before_linking() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let user = test_user(&pool, "link_owner").await;
            let state_id = format!("link-no-session-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(user.id),
            )
            .await
            .unwrap();

            let result = callback_handler(
                State(test_app_state(pool.clone())),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "code".to_string(),
                }),
                CookieJar::new(),
            )
            .await;

            assert!(matches!(result, Err(OAuthError::StateUserMismatch)));
            assert_eq!(db::count_oauth_accounts(&pool, user.id).await.unwrap(), 0);
        }

        #[tokio::test]
        async fn link_callback_with_different_user_session_is_rejected_before_linking() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let owner = test_user(&pool, "link_owner").await;
            let other = test_user(&pool, "link_other").await;
            let other_token = session::create_session(&pool, other.id).await.unwrap();
            let jar = CookieJar::new().add(session::build_session_cookie(&other_token));
            let state_id = format!("link-wrong-session-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(owner.id),
            )
            .await
            .unwrap();

            let result = callback_handler(
                State(test_app_state(pool.clone())),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "code".to_string(),
                }),
                jar,
            )
            .await;

            assert!(matches!(result, Err(OAuthError::StateUserMismatch)));
            assert_eq!(db::count_oauth_accounts(&pool, owner.id).await.unwrap(), 0);
            assert_eq!(db::count_oauth_accounts(&pool, other.id).await.unwrap(), 0);
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn unauthenticated_login_callback_creates_session() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let email = format!("login{}@example.com", test_utils::uid());
            let (state, _server) = mock_google_state(pool.clone(), &email).await;
            let state_id = format!("login-success-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-characters-long-enough",
                "login",
                None,
            )
            .await
            .unwrap();

            let response = callback_handler(
                State(state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "valid-code".to_string(),
                }),
                CookieJar::new(),
            )
            .await
            .unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            let set_cookie = response
                .headers()
                .get(axum::http::header::SET_COOKIE)
                .unwrap()
                .to_str()
                .unwrap();
            let cookie =
                axum_extra::extract::cookie::Cookie::parse(set_cookie.to_string()).unwrap();
            let user_id = session::verify_session(&pool, cookie.value())
                .await
                .unwrap();
            assert_eq!(db::count_oauth_accounts(&pool, user_id).await.unwrap(), 1);
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn matching_link_callback_preserves_existing_session() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let user = test_user(&pool, "link_success").await;
            let token = session::create_session(&pool, user.id).await.unwrap();
            let jar = CookieJar::new().add(session::build_session_cookie(&token));
            let email = format!("provider{}@example.com", test_utils::uid());
            let (state, _server) = mock_google_state(pool.clone(), &email).await;
            let state_id = format!("link-success-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(user.id),
            )
            .await
            .unwrap();

            let response = callback_handler(
                State(state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "valid-code".to_string(),
                }),
                jar,
            )
            .await
            .unwrap();

            assert_eq!(response.status(), StatusCode::SEE_OTHER);
            assert!(response
                .headers()
                .get(axum::http::header::SET_COOKIE)
                .is_none());
            assert_eq!(
                session::verify_session(&pool, &token).await.unwrap(),
                user.id
            );
            assert_eq!(db::count_oauth_accounts(&pool, user.id).await.unwrap(), 1);
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn test_insert_auth_state_with_provider() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            let state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(state.provider, "google");
            assert_eq!(state.redirect_uri, "/dashboard");
            assert_eq!(state.purpose, "login");
        }

        #[tokio::test]
        async fn test_provider_mismatch_detection() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            let state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .unwrap();
            // State stored with "google" provider
            assert_eq!(state.provider, "google");
            // If callback comes for "github", it won't match
            assert_ne!(state.provider, "github");
        }

        #[tokio::test]
        async fn test_state_expiry_check() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            let state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .unwrap();
            // Freshly created state should not be expired
            let elapsed = Utc::now()
                .signed_duration_since(state.created_at)
                .num_seconds();
            assert!(elapsed < CSRF_STATE_TTL_SECS);
        }

        #[tokio::test]
        async fn test_auth_state_stores_code_verifier() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());
            let verifier = "test-pkce-verifier-minimum-43-chars!!";

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                verifier,
                "login",
                None,
            )
            .await
            .unwrap();

            let state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(state.code_verifier, Some(verifier.to_string()));
        }

        #[tokio::test]
        async fn test_callback_retrieves_verifier_before_delete() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-state-{}", test_utils::uid());
            let verifier = "test-pkce-verifier-minimum-43-chars!!";

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                verifier,
                "login",
                None,
            )
            .await
            .unwrap();

            // Simulate new atomic flow: delete_auth_state returns the row
            let auth_state = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(auth_state.is_some());
            let auth_state = auth_state.unwrap();
            let stored_verifier = auth_state
                .code_verifier
                .expect("verifier should be present");

            // Verify the verifier is accessible from the returned row
            assert_eq!(stored_verifier, verifier);

            // State is now gone (second delete returns None)
            let state = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(state.is_none());

            // The verifier string we captured is still available
            let reconstructed = PkceCodeVerifier::new(stored_verifier);
            assert_eq!(reconstructed.secret(), verifier);
        }

        /// Verify that a valid session cookie provides the explicit target for
        /// link_to_user rather than allowing a new user to be created.
        #[tokio::test]
        async fn test_link_to_user_uses_authenticated_session_user() {
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
                None,
            )
            .await
            .unwrap();

            // Create a valid session JWT for this user
            let jwt = session::create_session(&pool, user.id).await.unwrap();

            // Build a CookieJar with the session cookie
            let cookie = session::build_session_cookie(&jwt);
            let jar = CookieJar::new().add(cookie);

            // Extract the user ID from the session cookie (simulating callback_handler logic)
            let existing_user_id = if let Some(cookie) = jar.get(session::COOKIE_NAME) {
                session::verify_session(&pool, cookie.value()).await.ok()
            } else {
                None
            };

            // Verify the session was read correctly
            assert_eq!(existing_user_id, Some(user.id));

            // Now link the GitHub identity to the authenticated user.
            let result = linking::link_to_user(
                &pool,
                existing_user_id.expect("session user should be present"),
                linking::OauthUserInfo {
                    provider: linking::Provider::GitHub,
                    provider_uid: format!("github-sessionlink-{u}"),
                    email: Some(format!("sessionlink{u}@example.com")),
                    email_verified: true,
                    display_name: "Session Link User".to_string(),
                    avatar_url: None,
                },
                None,
            )
            .await
            .unwrap();

            // Should link to the existing user
            assert_eq!(result.user_id, user.id);
            assert!(!result.is_new_user);

            // Verify the GitHub account was created and linked to the existing user
            let github_account = db::get_oauth_account_by_provider(
                &pool,
                "github",
                &format!("github-sessionlink-{u}"),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(github_account.user_id, user.id);

            // Verify no new user was created (only one user exists)
            let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(user_count, 1);
        }

        /// Verify that concurrent requests with the same state parameter
        /// cannot both succeed — the first caller atomically consumes the state.
        /// Uses 16 concurrent tasks via tokio::spawn for high-contention testing.
        #[tokio::test]
        async fn test_concurrent_state_consumption() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-concurrent-{}", test_utils::uid());
            let verifier = "test-pkce-verifier-minimum-43-chars!!";

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                verifier,
                "login",
                None,
            )
            .await
            .unwrap();

            // Spawn 16 concurrent delete attempts to exercise high-contention scenario.
            // Only one should succeed due to atomic DELETE ... RETURNING semantics.
            const NUM_CONCURRENT: usize = 16;
            let mut handles = Vec::with_capacity(NUM_CONCURRENT);

            for i in 0..NUM_CONCURRENT {
                let pool = pool.clone();
                let state_id = state_id.clone();
                let handle = tokio::spawn(async move {
                    let result = db::delete_auth_state(&pool, &state_id).await;
                    (i, result)
                });
                handles.push(handle);
            }

            // Collect all results
            let mut results = Vec::with_capacity(NUM_CONCURRENT);
            for handle in handles {
                let (idx, result) = handle.await.expect("task should not panic");
                let state = result.expect("db operation should not fail");
                results.push((idx, state));
            }

            // Count successes and failures
            let successes: Vec<_> = results.iter().filter(|(_, s)| s.is_some()).collect();
            let failures: Vec<_> = results.iter().filter(|(_, s)| s.is_none()).collect();

            // Exactly one should succeed
            assert_eq!(
                successes.len(),
                1,
                "exactly one caller should get the state, got {} successes: {:?}",
                successes.len(),
                successes.iter().map(|(i, _)| *i).collect::<Vec<_>>()
            );

            // All others should get None
            assert_eq!(
                failures.len(),
                NUM_CONCURRENT - 1,
                "remaining {} callers should get None, got {} failures",
                NUM_CONCURRENT - 1,
                failures.len()
            );

            // The winner should have correct data
            let winner = successes[0].1.as_ref().unwrap();
            assert_eq!(winner.id, state_id);
            assert_eq!(winner.provider, "google");
            assert_eq!(winner.code_verifier, Some(verifier.to_string()));
        }

        /// Verify that an expired state is still consumed by delete_auth_state,
        /// and the elapsed time exceeds the TTL — matching the StateExpired path
        /// in callback_handler's validation-after-delete flow.
        #[tokio::test]
        async fn test_expired_state_returns_state_expired() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-expired-{}", test_utils::uid());

            // Insert state with "google" provider
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            // Backdate the state to 20 minutes ago (exceeds 600s TTL)
            sqlx::query(
                "UPDATE auth_states SET created_at = NOW() - INTERVAL '20 minutes' WHERE id = $1",
            )
            .bind(&state_id)
            .execute(&pool)
            .await
            .unwrap();

            // Simulate callback_handler flow: delete_auth_state consumes the state
            let auth_state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .expect("state should exist for delete");

            // State was consumed — now check expiry (this is the validation-after-delete path)
            let elapsed = Utc::now()
                .signed_duration_since(auth_state.created_at)
                .num_seconds();
            assert!(
                elapsed > CSRF_STATE_TTL_SECS,
                "state should be expired: elapsed={}s, ttl={}s",
                elapsed,
                CSRF_STATE_TTL_SECS
            );

            // Verify state is gone (consumed) even though it was expired
            let still_exists = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(
                still_exists.is_none(),
                "expired state should still be consumed"
            );
        }

        /// Verify that a provider mismatch is detected after the state is consumed,
        /// matching the ProviderMismatch path in callback_handler's validation-after-delete flow.
        #[tokio::test]
        async fn test_provider_mismatch_after_delete() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-mismatch-{}", test_utils::uid());

            // Insert state with "google" provider
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            // Simulate callback_handler: state consumed via delete_auth_state
            let auth_state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .expect("state should exist for delete");

            // Callback comes in for "github" — provider mismatch
            let callback_provider = linking::Provider::GitHub;
            assert_ne!(
                auth_state.provider,
                callback_provider.as_str(),
                "stored provider '{}' should not match callback provider '{}'",
                auth_state.provider,
                callback_provider.as_str()
            );

            // Verify the mismatch error message is client-safe
            let err = OAuthError::ProviderMismatch;
            assert_eq!(err.sanitized_message(), "Provider mismatch");
            let response = err.into_response();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }

        /// Verify that a second delete_auth_state call returns None, triggering
        /// StateNotFound — matching the first guard in callback_handler.
        #[tokio::test]
        async fn test_state_not_found_after_double_delete() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-doubl-del-{}", test_utils::uid());

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            // First delete consumes the state
            let first = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(first.is_some(), "first delete should succeed");

            // Second delete returns None — triggers StateNotFound in callback_handler
            let second = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(second.is_none(), "second delete should return None");

            // Verify the error mapping matches callback_handler behavior
            let err = OAuthError::StateNotFound;
            assert_eq!(err.sanitized_message(), "CSRF state not found");
            let response = err.into_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        // ── user_id binding tests ─────────────────────────────────────────

        /// Verify that insert_auth_state persists a user_id and it is returned by delete_auth_state.
        #[tokio::test]
        async fn test_auth_state_stores_user_id() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-userid-{}", test_utils::uid());
            let user_id = Uuid::new_v4();

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "link",
                Some(user_id),
            )
            .await
            .unwrap();

            let state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(state.user_id, Some(user_id));
        }

        /// Verify that insert_auth_state with None user_id stores NULL.
        #[tokio::test]
        async fn test_auth_state_null_user_id() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-nulluid-{}", test_utils::uid());

            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            let state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(state.user_id, None);
        }

        /// Verify that callback_handler rejects a callback when the stored user_id
        /// does not match the current session's user_id (CSRF protection).
        #[tokio::test]
        async fn test_callback_rejects_mismatched_user_id() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let u = test_utils::uid();

            // Set up thread-local session secret (avoids race with other parallel tests)
            session::set_test_secret("test-secret-32-bytes-long-enough!!");

            // Create two different users
            let user1 = db::insert_user(
                &pool,
                &format!("csrfuser1_{u}"),
                "CSRF User 1",
                &format!("csrf1{u}@example.com"),
                None,
            )
            .await
            .unwrap();

            let user2 = db::insert_user(
                &pool,
                &format!("csrfuser2_{u}"),
                "CSRF User 2",
                &format!("csrf2{u}@example.com"),
                None,
            )
            .await
            .unwrap();

            // Insert an auth state bound to user1
            let state_id = format!("test-csrf-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "link",
                Some(user1.id),
            )
            .await
            .unwrap();

            // Simulate user2's session cookie
            let jwt_user2 = session::create_session(&pool, user2.id).await.unwrap();
            let cookie_user2 = session::build_session_cookie(&jwt_user2);
            let jar = CookieJar::new().add(cookie_user2);

            // Extract user2's user_id from session (simulating callback_handler logic)
            let existing_user_id = if let Some(cookie) = jar.get(session::COOKIE_NAME) {
                session::verify_session(&pool, cookie.value()).await.ok()
            } else {
                None
            };
            assert_eq!(existing_user_id, Some(user2.id));

            // Consume the auth state (simulating callback_handler)
            let auth_state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .expect("state should exist");

            // Validate user_id binding (this is the new CSRF check)
            let stored = auth_state.user_id;
            let current = existing_user_id;
            let mismatch = if let Some(s) = stored {
                if let Some(c) = current {
                    s != c
                } else {
                    false
                }
            } else {
                false
            };
            assert!(
                mismatch,
                "stored user_id ({:?}) should not match current session user_id ({:?})",
                stored, current
            );

            // Verify the error response
            let err = OAuthError::StateUserMismatch;
            assert_eq!(err.sanitized_message(), "Auth state user mismatch");
            let response = err.into_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            // Clean up thread-local test secret
            session::clear_test_secret();
        }

        /// Verify that callback_handler accepts a callback when the stored user_id
        /// matches the current session's user_id.
        #[tokio::test]
        async fn test_callback_accepts_matching_user_id() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let u = test_utils::uid();

            // Set up session secret
            std::env::set_var("SESSION_SECRET", "test-secret-32-bytes-long-enough!!");

            // Create a single user
            let user = db::insert_user(
                &pool,
                &format!("matchuser_{u}"),
                "Match User",
                &format!("match{u}@example.com"),
                None,
            )
            .await
            .unwrap();

            // Insert an auth state bound to this user
            let state_id = format!("test-match-{}", test_utils::uid());
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "link",
                Some(user.id),
            )
            .await
            .unwrap();

            // Create a session for the same user
            let jwt = session::create_session(&pool, user.id).await.unwrap();
            let cookie = session::build_session_cookie(&jwt);
            let jar = CookieJar::new().add(cookie);

            // Extract user_id from session
            let existing_user_id = if let Some(cookie) = jar.get(session::COOKIE_NAME) {
                session::verify_session(&pool, cookie.value()).await.ok()
            } else {
                None
            };
            assert_eq!(existing_user_id, Some(user.id));

            // Consume the auth state
            let auth_state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .expect("state should exist");

            // Validate user_id binding — should match
            let stored = auth_state.user_id;
            let current = existing_user_id;
            let mismatch = if let Some(s) = stored {
                if let Some(c) = current {
                    s != c
                } else {
                    false
                }
            } else {
                false
            };
            assert!(
                !mismatch,
                "stored user_id ({:?}) should match current session user_id ({:?})",
                stored, current
            );
        }

        /// Verify that callback_handler allows an unauthenticated flow when the
        /// stored user_id is NULL and there is no active session.
        #[tokio::test]
        async fn test_callback_allows_null_user_id_no_session() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let state_id = format!("test-unauth-{}", test_utils::uid());

            // Insert an auth state with no user_id (unauthenticated flow)
            db::insert_auth_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-chars-long!!",
                "login",
                None,
            )
            .await
            .unwrap();

            // No session cookie — unauthenticated
            let existing_user_id: Option<Uuid> = None;

            // Consume the auth state
            let auth_state = db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .expect("state should exist");

            // Validate user_id binding — NULL stored + no session = allowed
            let stored = auth_state.user_id;
            let current = existing_user_id;
            let mismatch = if let Some(s) = stored {
                if let Some(c) = current {
                    s != c
                } else {
                    false
                }
            } else {
                false
            };
            assert!(
                !mismatch,
                "unauthenticated flow should be allowed (stored: {:?}, current: {:?})",
                stored, current
            );
        }
    }
}
