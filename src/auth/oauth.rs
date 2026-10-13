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
use cookie::{Cookie, CookieBuilder, SameSite};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use time::Duration as TimeDuration;
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
    /// Parsed configured application origin used for exact request-origin and
    /// cookie transport decisions.
    pub base_url_origin: url::Origin,
    /// Aggregate, non-sensitive authentication rejection counters.
    pub auth_security_metrics: AuthSecurityMetrics,
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

#[derive(Default)]
struct AuthSecurityMetricsInner {
    browser_binding_missing: AtomicU64,
    browser_binding_malformed: AtomicU64,
    browser_binding_mismatch: AtomicU64,
    logout_origin_missing: AtomicU64,
    logout_origin_duplicate: AtomicU64,
    logout_origin_null: AtomicU64,
    logout_origin_malformed: AtomicU64,
    logout_origin_mismatch: AtomicU64,
}

/// Cloneable aggregate metrics shared by all authentication handlers.
#[derive(Clone, Default)]
pub struct AuthSecurityMetrics(Arc<AuthSecurityMetricsInner>);

impl AuthSecurityMetrics {
    fn record_browser_binding_rejection(&self, category: &'static str) -> u64 {
        let counter = match category {
            "missing" => &self.0.browser_binding_missing,
            "malformed" => &self.0.browser_binding_malformed,
            "mismatch" => &self.0.browser_binding_mismatch,
            _ => unreachable!("closed browser-binding rejection category"),
        };
        counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub(crate) fn record_logout_origin_rejection(&self, category: &'static str) -> u64 {
        let counter = match category {
            "missing" => &self.0.logout_origin_missing,
            "duplicate" => &self.0.logout_origin_duplicate,
            "null" => &self.0.logout_origin_null,
            "malformed" => &self.0.logout_origin_malformed,
            "mismatch" => &self.0.logout_origin_mismatch,
            _ => unreachable!("closed logout-origin rejection category"),
        };
        counter.fetch_add(1, Ordering::Relaxed) + 1
    }
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
    EmailAlreadyInUse(String),
    BrowserBindingMissing,
    BrowserBindingMalformed,
    BrowserBindingMismatch,
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
            Self::EmailAlreadyInUse(_) => write!(
                f,
                "An account with the same email address already exists using a different sign-in provider. Sign in to your existing account using that provider, then go to Settings → Accounts to link this provider manually."
            ),
            Self::BrowserBindingMissing => write!(f, "OAuth browser binding is missing"),
            Self::BrowserBindingMalformed => write!(f, "OAuth browser binding is malformed"),
            Self::BrowserBindingMismatch => write!(f, "OAuth browser binding does not match"),
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
            Self::EmailAlreadyInUse(_) => "email_already_in_use",
            Self::BrowserBindingMissing => "browser_binding_missing",
            Self::BrowserBindingMalformed => "browser_binding_malformed",
            Self::BrowserBindingMismatch => "browser_binding_mismatch",
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
            | Self::EmailAlreadyInUse(_)
            | Self::BrowserBindingMissing
            | Self::BrowserBindingMalformed
            | Self::BrowserBindingMismatch
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
        if let Self::EmailAlreadyInUse(redirect_uri) = &self {
            let encoded = percent_encoding::utf8_percent_encode(
                redirect_uri,
                percent_encoding::NON_ALPHANUMERIC,
            );
            let location = format!("/login?error=email_already_in_use&redirect_uri={encoded}");
            tracing::warn!(
                error_code = self.log_code(),
                "OAuth email conflict — returning to sign-in"
            );
            return (StatusCode::SEE_OTHER, [("location", location)]).into_response();
        }

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
            Self::EmailAlreadyInUse(_) => StatusCode::CONFLICT,
            Self::BrowserBindingMissing
            | Self::BrowserBindingMalformed
            | Self::BrowserBindingMismatch => StatusCode::UNAUTHORIZED,
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

const BROWSER_BINDING_COOKIE_PREFIX: &str = "noms_oauth_tx_";
const BROWSER_BINDING_SECRET_HEX_LEN: usize = 64;

fn browser_binding_cookie_name(state_id: &str) -> String {
    let digest = Sha256::digest(state_id.as_bytes());
    let mut suffix = String::with_capacity(32);
    for byte in &digest[..16] {
        write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
    }
    format!("{BROWSER_BINDING_COOKIE_PREFIX}{suffix}")
}

fn browser_binding_cookie_path(provider: &str) -> String {
    match provider {
        "google" => "/auth/google/callback".to_string(),
        "github" => "/auth/github/callback".to_string(),
        // No transaction cookie is ever issued for another provider. Keep the
        // error-path clearing header syntactically safe and narrowly scoped.
        _ => "/auth/".to_string(),
    }
}

fn browser_binding_secret() -> String {
    // Two independent UUIDv4 values provide over 240 bits of randomness while
    // retaining a strict, cookie-safe representation that is cheap to validate.
    format!(
        "{:032x}{:032x}",
        Uuid::new_v4().as_u128(),
        Uuid::new_v4().as_u128()
    )
}

fn browser_binding_digest(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

fn is_valid_browser_binding_secret(secret: &str) -> bool {
    secret.len() == BROWSER_BINDING_SECRET_HEX_LEN
        && secret.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn binding_cookie_secure(state: &AppState) -> bool {
    // Production configuration already requires HTTPS. Local mode permits
    // other HTTP hostnames as well as loopback, and those must remain usable.
    !matches!(
        &state.base_url_origin,
        url::Origin::Tuple(scheme, _, _) if scheme == "http"
    )
}

fn build_browser_binding_cookie(
    state: &AppState,
    provider: &str,
    state_id: &str,
    value: String,
    max_age: TimeDuration,
) -> Cookie<'static> {
    CookieBuilder::new(browser_binding_cookie_name(state_id), value)
        .http_only(true)
        .secure(binding_cookie_secure(state))
        .same_site(SameSite::Lax)
        .path(browser_binding_cookie_path(provider))
        .max_age(max_age)
        .build()
}

fn clear_browser_binding_cookie(
    state: &AppState,
    provider: &str,
    state_id: &str,
) -> Cookie<'static> {
    build_browser_binding_cookie(state, provider, state_id, String::new(), TimeDuration::ZERO)
}

fn append_set_cookie(response: &mut axum::response::Response, cookie: Cookie<'static>) {
    response.headers_mut().append(
        axum::http::header::SET_COOKIE,
        cookie
            .encoded()
            .to_string()
            .parse()
            .expect("OAuth binding cookie should form a valid header"),
    );
}

fn binding_rejection(state: &AppState, error: OAuthError, category: &'static str) -> OAuthError {
    let total = state
        .auth_security_metrics
        .record_browser_binding_rejection(category);
    tracing::warn!(
        error_code = error.log_code(),
        rejection_total = total,
        "OAuth callback rejected by browser transaction binding"
    );
    error
}

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

    // Generate a CSRF state UUID and a separate browser-held transaction key.
    // Only the key digest is persisted.
    let csrf_state = Uuid::new_v4().to_string();
    let binding_secret = browser_binding_secret();
    let binding_digest = browser_binding_digest(&binding_secret);

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

    db::insert_bound_auth_state(
        &state.pool,
        &csrf_state,
        prov.as_str(),
        &redirect_uri,
        pkce_verifier.secret(),
        purpose.as_str(),
        user_id,
        &binding_digest,
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

    let binding_cookie = build_browser_binding_cookie(
        &state,
        prov.as_str(),
        &csrf_state,
        binding_secret,
        TimeDuration::seconds(CSRF_STATE_TTL_SECS),
    );
    let mut response = Redirect::temporary(&auth_url).into_response();
    append_set_cookie(&mut response, binding_cookie);
    Ok(response)
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
    let clear_cookie = clear_browser_binding_cookie(&state, &provider, &params.state);
    let mut response = callback_inner(&state, &provider, &params, &jar).await?;
    append_set_cookie(&mut response, clear_cookie);
    Ok(response)
}

/// Router-facing callback wrapper that also clears the transaction cookie when
/// the callback terminates with an error.
pub async fn callback_route_handler(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(params): Query<CallbackQuery>,
    jar: CookieJar,
) -> axum::response::Response {
    let clear_cookie = clear_browser_binding_cookie(&state, &provider, &params.state);
    match callback_handler(State(state), Path(provider), Query(params), jar).await {
        Ok(response) => response,
        Err(error) => {
            let mut response = error.into_response();
            append_set_cookie(&mut response, clear_cookie);
            response
        }
    }
}

async fn callback_inner(
    state: &AppState,
    provider: &str,
    params: &CallbackQuery,
    jar: &CookieJar,
) -> Result<axum::response::Response, OAuthError> {
    let prov = validate_provider(provider)?;

    let binding_cookie_name = browser_binding_cookie_name(&params.state);
    let binding_secret = jar
        .get(&binding_cookie_name)
        .ok_or_else(|| binding_rejection(state, OAuthError::BrowserBindingMissing, "missing"))?;
    if !is_valid_browser_binding_secret(binding_secret.value()) {
        return Err(binding_rejection(
            state,
            OAuthError::BrowserBindingMalformed,
            "malformed",
        ));
    }
    let supplied_digest = browser_binding_digest(binding_secret.value());

    // The row lock and digest comparison occur before deletion. A wrong browser
    // leaves the transaction available to the browser that initiated it.
    let auth_state =
        match db::consume_bound_auth_state(&state.pool, &params.state, &supplied_digest)
            .await
            .map_err(|error| OAuthError::DbError(error.to_string()))?
        {
            db::BoundAuthStateConsume::Consumed(auth_state) => auth_state,
            db::BoundAuthStateConsume::StateNotFound => return Err(OAuthError::StateNotFound),
            db::BoundAuthStateConsume::BindingMismatch => {
                return Err(binding_rejection(
                    state,
                    OAuthError::BrowserBindingMismatch,
                    "mismatch",
                ));
            }
        };

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
        _ => return Err(OAuthError::InvalidProvider(provider.to_string())),
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
            let token = jar
                .get(configured_cookie_name.as_ref())
                .ok_or(OAuthError::LinkSessionRequired)?
                .value();
            linking::link_to_user(&state.pool, user_id, token, user_info, refresh_token).await
        }
    };
    let link_result = match link_result {
        Ok(result) => result,
        Err(linking::LinkError::Session(error)) => {
            return Err(match error {
                session::SessionError::InvalidToken
                | session::SessionError::Expired
                | session::SessionError::SessionInvalid => OAuthError::LinkSessionRequired,
                error => OAuthError::SessionError(error.to_string()),
            });
        }
        Err(linking::LinkError::AccountAlreadyLinked(provider)) => {
            let redirect_uri = format!(
                "/settings/accounts?error=account_already_linked&provider={}",
                provider
            );
            return Ok((StatusCode::SEE_OTHER, [("location", redirect_uri)]).into_response());
        }
        Err(linking::LinkError::EmailAlreadyInUse) => {
            return Err(OAuthError::EmailAlreadyInUse(redirect_uri));
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
    fn email_conflict_redirects_to_login_preserving_the_return_destination() {
        let error = OAuthError::EmailAlreadyInUse("/search?q=soup&scope=recipes".into());
        assert_eq!(error.log_code(), "email_already_in_use");
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        let url = url::Url::parse(&format!("https://noms.example{location}")).unwrap();
        assert_eq!(url.path(), "/login");
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(
            pairs,
            vec![
                ("error".into(), "email_already_in_use".into()),
                ("redirect_uri".into(), "/search?q=soup&scope=recipes".into())
            ]
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

        const TEST_BROWSER_BINDING_SECRET: &str =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

        #[allow(clippy::too_many_arguments)]
        async fn insert_callback_state(
            pool: &PgPool,
            state_id: &str,
            provider: &str,
            redirect_uri: &str,
            code_verifier: &str,
            purpose: &str,
            user_id: Option<Uuid>,
        ) {
            db::insert_bound_auth_state(
                pool,
                state_id,
                provider,
                redirect_uri,
                code_verifier,
                purpose,
                user_id,
                &browser_binding_digest(TEST_BROWSER_BINDING_SECRET),
            )
            .await
            .unwrap();
        }

        fn add_test_browser_binding(jar: CookieJar, state_id: &str) -> CookieJar {
            jar.add(cookie::Cookie::new(
                browser_binding_cookie_name(state_id),
                TEST_BROWSER_BINDING_SECRET,
            ))
        }

        async fn start_test_login(pool: PgPool) -> (String, cookie::Cookie<'static>) {
            let response = start_handler(
                State(test_app_state(pool)),
                Path("google".to_string()),
                Query(StartQuery {
                    redirect_uri: "/dashboard".to_string(),
                    purpose: "login".to_string(),
                }),
                CookieJar::new(),
            )
            .await
            .unwrap()
            .into_response();

            let location = response
                .headers()
                .get(axum::http::header::LOCATION)
                .unwrap()
                .to_str()
                .unwrap();
            let state_id = url::Url::parse(location)
                .unwrap()
                .query_pairs()
                .find_map(|(name, value)| (name == "state").then(|| value.into_owned()))
                .expect("authorization redirect should contain state");
            let set_cookie = response
                .headers()
                .get(axum::http::header::SET_COOKIE)
                .expect("OAuth start should bind the transaction to a browser")
                .to_str()
                .unwrap();
            let cookie = cookie::Cookie::parse(set_cookie.to_string())
                .unwrap()
                .into_owned();
            (state_id, cookie)
        }

        fn test_app_state(pool: PgPool) -> AppState {
            let (google_client, github_client) = build_oauth_clients("http://localhost:3000");
            AppState {
                pool,
                base_url_origin: url::Url::parse("http://localhost:3000").unwrap().origin(),
                auth_security_metrics: AuthSecurityMetrics::default(),
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
                    base_url_origin: url::Url::parse("http://localhost:3000").unwrap().origin(),
                    auth_security_metrics: AuthSecurityMetrics::default(),
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

        /// Hold the token exchange until the test explicitly releases it, rather
        /// than assuming a fixed delay is long enough for a database mutation.
        struct GatedGoogleProvider {
            state: AppState,
            admitted: std::sync::Arc<tokio::sync::Notify>,
            release: std::sync::Arc<tokio::sync::Notify>,
            _userinfo_server: wiremock::MockServer,
            token_task: tokio::task::JoinHandle<()>,
        }

        impl Drop for GatedGoogleProvider {
            fn drop(&mut self) {
                self.token_task.abort();
            }
        }

        async fn gated_google_state(pool: PgPool, email: &str) -> GatedGoogleProvider {
            let (mut state, userinfo_server) = mock_google_state(pool, email).await;
            let admitted = std::sync::Arc::new(tokio::sync::Notify::new());
            let release = std::sync::Arc::new(tokio::sync::Notify::new());
            let gate = (admitted.clone(), release.clone());
            let router = axum::Router::new().route(
                "/token",
                axum::routing::post(move || {
                    let (admitted, release) = gate.clone();
                    async move {
                        admitted.notify_one();
                        release.notified().await;
                        axum::Json(serde_json::json!({
                            "access_token": "test-access-token",
                            "token_type": "Bearer"
                        }))
                    }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            state.google_client = state.google_client.set_token_uri(
                TokenUrl::new(format!("http://{}/token", listener.local_addr().unwrap())).unwrap(),
            );
            let token_task = tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
            GatedGoogleProvider {
                state,
                admitted,
                release,
                _userinfo_server: userinfo_server,
                token_task,
            }
        }

        fn response_session_cookie(response: &axum::response::Response) -> cookie::Cookie<'static> {
            response
                .headers()
                .get_all(axum::http::header::SET_COOKIE)
                .iter()
                .map(|value| {
                    cookie::Cookie::parse(value.to_str().unwrap().to_string())
                        .unwrap()
                        .into_owned()
                })
                .find(|cookie| cookie.name() == session::cookie_name().as_ref())
                .expect("login response should set the configured session cookie")
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
        async fn oauth_start_sets_host_only_callback_scoped_browser_binding_cookie() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let (_state_id, cookie) = start_test_login(pool).await;

            assert!(cookie.name().starts_with("noms_oauth_tx_"));
            assert_eq!(cookie.path(), Some("/auth/google/callback"));
            assert_eq!(
                cookie.domain(),
                None,
                "binding cookie must remain host-only"
            );
            assert!(cookie.http_only().unwrap_or(false));
            assert!(!cookie.secure().unwrap_or(false));
            assert_eq!(cookie.same_site(), Some(cookie::SameSite::Lax));
            assert_eq!(
                cookie.max_age().unwrap().whole_seconds(),
                CSRF_STATE_TTL_SECS
            );
        }

        #[tokio::test]
        async fn oauth_binding_cookie_is_secure_for_https_origin() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let mut state = test_app_state(pool);
            state.base_url_origin = url::Url::parse("https://noms.example.com")
                .unwrap()
                .origin();

            let cookie = build_browser_binding_cookie(
                &state,
                "google",
                "state-id",
                TEST_BROWSER_BINDING_SECRET.to_string(),
                TimeDuration::seconds(CSRF_STATE_TTL_SECS),
            );
            assert!(cookie.secure().unwrap_or(false));
        }

        #[tokio::test]
        async fn transferred_callback_cannot_authorize_another_browser_or_consume_state() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let (state_id, browser_cookie) = start_test_login(pool.clone()).await;
            let email = format!("browser-bound{}@example.com", test_utils::uid());
            let (callback_state, token_server) = mock_google_state(pool.clone(), &email).await;

            let transferred = callback_handler(
                State(callback_state.clone()),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "attacker-visible-code".to_string(),
                }),
                CookieJar::new(),
            )
            .await;
            assert!(transferred.is_err());
            assert!(token_server.received_requests().await.unwrap().is_empty());

            let initiating_browser = CookieJar::new().add(browser_cookie.clone());
            let completed = callback_handler(
                State(callback_state.clone()),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "valid-code".to_string(),
                }),
                initiating_browser.clone(),
            )
            .await
            .unwrap();
            assert_eq!(completed.status(), StatusCode::SEE_OTHER);
            assert!(completed
                .headers()
                .get(axum::http::header::SET_COOKIE)
                .is_some());

            let replay = callback_handler(
                State(callback_state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "valid-code".to_string(),
                }),
                initiating_browser,
            )
            .await;
            assert!(matches!(replay, Err(OAuthError::StateNotFound)));
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn forced_get_logout_then_transferred_callback_cannot_swap_victim_session() {
            use axum::body::Body;
            use axum::http::Request;
            use tower::ServiceExt;

            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let victim = test_user(&pool, "forced_logout_victim").await;
            let victim_token = session::create_session(&pool, victim.id).await.unwrap();
            let (state_id, _initiating_browser_cookie) = start_test_login(pool.clone()).await;
            let email = format!("forced-logout-attacker{}@example.com", test_utils::uid());
            let (callback_state, token_server) = mock_google_state(pool.clone(), &email).await;

            let logout_router = axum::Router::new()
                .route(
                    "/auth/logout",
                    axum::routing::post(crate::auth::logout::handle_logout),
                )
                .with_state(callback_state.clone());
            let forced_logout = logout_router
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/logout")
                        .header("Cookie", format!("noms_session={victim_token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(forced_logout.status(), StatusCode::METHOD_NOT_ALLOWED);
            assert!(!forced_logout
                .headers()
                .contains_key(axum::http::header::SET_COOKIE));

            let victim_jar = CookieJar::new().add(session::build_session_cookie(&victim_token));
            let transferred = callback_handler(
                State(callback_state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "attacker-visible-code".to_string(),
                }),
                victim_jar,
            )
            .await;
            assert!(matches!(
                transferred,
                Err(OAuthError::BrowserBindingMissing)
            ));
            assert!(token_server.received_requests().await.unwrap().is_empty());
            assert_eq!(
                session::verify_session(&pool, &victim_token).await.unwrap(),
                victim.id
            );
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn malformed_browser_binding_is_rejected_before_exchange_without_consuming_state() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let (state_id, browser_cookie) = start_test_login(pool.clone()).await;
            let email = format!("malformed-binding{}@example.com", test_utils::uid());
            let (callback_state, token_server) = mock_google_state(pool, &email).await;
            let malformed_cookie = cookie::Cookie::new(
                browser_cookie.name().to_string(),
                "not-a-valid-browser-secret",
            );

            let rejected = callback_handler(
                State(callback_state.clone()),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "unused-code".to_string(),
                }),
                CookieJar::new().add(malformed_cookie),
            )
            .await;
            assert!(rejected.is_err());
            assert!(token_server.received_requests().await.unwrap().is_empty());

            let completed = callback_handler(
                State(callback_state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "valid-code".to_string(),
                }),
                CookieJar::new().add(browser_cookie),
            )
            .await;
            assert!(completed.is_ok());
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn mismatched_browser_binding_does_not_consume_the_initiators_state() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let (state_id, browser_cookie) = start_test_login(pool.clone()).await;
            let email = format!("mismatched-binding{}@example.com", test_utils::uid());
            let (callback_state, token_server) = mock_google_state(pool, &email).await;
            let wrong_cookie = cookie::Cookie::new(
                browser_cookie.name().to_string(),
                "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            );

            let rejected = callback_handler(
                State(callback_state.clone()),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "unused-code".to_string(),
                }),
                CookieJar::new().add(wrong_cookie),
            )
            .await;
            assert!(matches!(rejected, Err(OAuthError::BrowserBindingMismatch)));
            assert!(token_server.received_requests().await.unwrap().is_empty());

            let completed = callback_handler(
                State(callback_state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "valid-code".to_string(),
                }),
                CookieJar::new().add(browser_cookie),
            )
            .await;
            assert!(completed.is_ok());
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn expired_callback_is_rejected_before_exchange_and_clears_binding_cookie() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let (state_id, browser_cookie) = start_test_login(pool.clone()).await;
            sqlx::query(
                "UPDATE auth_states SET created_at = NOW() - INTERVAL '20 minutes' WHERE id = $1",
            )
            .bind(&state_id)
            .execute(&pool)
            .await
            .unwrap();

            let response = callback_route_handler(
                State(test_app_state(pool)),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id,
                    code: "unused-code".to_string(),
                }),
                CookieJar::new().add(browser_cookie),
            )
            .await;

            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let set_cookie = response
                .headers()
                .get(axum::http::header::SET_COOKIE)
                .unwrap()
                .to_str()
                .unwrap();
            assert!(set_cookie.contains(BROWSER_BINDING_COOKIE_PREFIX));
            assert!(set_cookie.contains("Max-Age=0"));
            assert!(!set_cookie.contains(session::cookie_name().as_ref()));
        }

        #[tokio::test]
        async fn concurrent_oauth_attempts_use_independent_browser_bindings() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let (first_state_id, first_cookie) = start_test_login(pool.clone()).await;
            let (second_state_id, second_cookie) = start_test_login(pool.clone()).await;
            assert_ne!(first_cookie.name(), second_cookie.name());

            let email = format!("concurrent-binding{}@example.com", test_utils::uid());
            let (callback_state, _token_server) = mock_google_state(pool, &email).await;
            for (state_id, browser_cookie) in [
                (first_state_id, first_cookie),
                (second_state_id, second_cookie),
            ] {
                let response = callback_handler(
                    State(callback_state.clone()),
                    Path("google".to_string()),
                    Query(CallbackQuery {
                        state: state_id,
                        code: "valid-code".to_string(),
                    }),
                    CookieJar::new().add(browser_cookie),
                )
                .await
                .unwrap();
                assert_eq!(response.status(), StatusCode::SEE_OTHER);
            }
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

            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-characters-long-enough",
                "login",
                None,
            )
            .await;
            let jar = add_test_browser_binding(jar, &state_id);

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
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/\\evil.example",
                "test-verifier-minimum-43-characters-long-enough",
                "login",
                None,
            )
            .await;

            let result = callback_handler(
                State(test_app_state(pool)),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "unused-code".to_string(),
                }),
                add_test_browser_binding(CookieJar::new(), &state_id),
            )
            .await;

            assert!(matches!(result, Err(OAuthError::InvalidRedirectUri(_))));
        }

        #[tokio::test]
        async fn link_callback_without_session_is_rejected_before_linking() {
            let (_db, pool) = test_utils::setup_test_db().await;
            let user = test_user(&pool, "link_owner").await;
            let state_id = format!("link-no-session-{}", test_utils::uid());
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(user.id),
            )
            .await;

            let result = callback_handler(
                State(test_app_state(pool.clone())),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "code".to_string(),
                }),
                add_test_browser_binding(CookieJar::new(), &state_id),
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
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(owner.id),
            )
            .await;
            let jar = add_test_browser_binding(jar, &state_id);

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
        async fn revoked_link_session_clears_only_transaction_cookie_without_linking() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let owner = test_user(&pool, "revoked_link_owner").await;
            let replacement = test_user(&pool, "replacement").await;
            let original_token = session::create_session(&pool, owner.id).await.unwrap();
            let replacement_token = session::create_session(&pool, replacement.id)
                .await
                .unwrap();
            let (state, provider) = mock_google_state(pool.clone(), &owner.email).await;
            let state_id = format!("revoked-link-{}", test_utils::uid());
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(owner.id),
            )
            .await;
            session::revoke_session(&pool, &original_token)
                .await
                .unwrap();
            let jar = add_test_browser_binding(
                CookieJar::new().add(session::build_session_cookie(&original_token)),
                &state_id,
            );
            let response = callback_route_handler(
                State(state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "unused-code".to_string(),
                }),
                jar,
            )
            .await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!response.headers().contains_key("location"));
            let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();
            assert_eq!(cookies.len(), 1);
            let cleared_cookie = cookie::Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
            assert_eq!(
                cleared_cookie.name(),
                browser_binding_cookie_name(&state_id)
            );
            assert_eq!(cleared_cookie.max_age(), Some(time::Duration::ZERO));
            assert!(provider.received_requests().await.unwrap().is_empty());
            assert_eq!(db::count_oauth_accounts(&pool, owner.id).await.unwrap(), 0);
            assert_eq!(
                db::count_oauth_accounts(&pool, replacement.id)
                    .await
                    .unwrap(),
                0
            );
            assert_eq!(
                session::verify_session(&pool, &replacement_token)
                    .await
                    .unwrap(),
                replacement.id
            );
            assert!(db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .is_none());
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn admitted_link_rejects_session_revocation_before_provider_completion() {
            assert_admitted_link_rejects_invalidated_session(false).await;
        }

        #[tokio::test]
        async fn admitted_link_rejects_session_expiry_before_provider_completion() {
            assert_admitted_link_rejects_invalidated_session(true).await;
        }

        async fn assert_admitted_link_rejects_invalidated_session(expire: bool) {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let owner = test_user(&pool, "link_owner").await;
            let replacement = test_user(&pool, "link_replacement").await;
            let original_token = session::create_session(&pool, owner.id).await.unwrap();
            let provider = gated_google_state(pool.clone(), &owner.email).await;
            let state_id = format!("admitted-link-{}", test_utils::uid());
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(owner.id),
            )
            .await;
            let callback = callback_route_handler(
                State(provider.state.clone()),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "valid-code".to_string(),
                }),
                add_test_browser_binding(
                    CookieJar::new().add(session::build_session_cookie(&original_token)),
                    &state_id,
                ),
            );
            let replace_session = async {
                provider.admitted.notified().await;
                assert_eq!(db::count_oauth_accounts(&pool, owner.id).await.unwrap(), 0);
                if expire {
                    sqlx::query("UPDATE sessions SET expires_at = NOW() - INTERVAL '1 second' WHERE user_id = $1")
                        .bind(owner.id).execute(&pool).await.unwrap();
                } else {
                    session::revoke_session(&pool, &original_token)
                        .await
                        .unwrap();
                }
                let replacement_token = session::create_session(&pool, replacement.id)
                    .await
                    .unwrap();
                provider.release.notify_one();
                replacement_token
            };
            let (response, replacement_token) =
                tokio::time::timeout(std::time::Duration::from_secs(15), async {
                    tokio::join!(callback, replace_session)
                })
                .await
                .expect("gated callback should complete");

            // User-authorized contract change: admission alone is insufficient.
            // A rejected link must not redirect or overwrite the replacement cookie.
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!response.headers().contains_key("location"));
            let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();
            assert_eq!(cookies.len(), 1);
            let cleared = cookie::Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
            assert_eq!(cleared.name(), browser_binding_cookie_name(&state_id));
            assert_eq!(cleared.max_age(), Some(time::Duration::ZERO));
            assert_eq!(db::count_oauth_accounts(&pool, owner.id).await.unwrap(), 0);
            assert_eq!(
                db::count_oauth_accounts(&pool, replacement.id)
                    .await
                    .unwrap(),
                0
            );
            assert!(session::verify_session(&pool, &original_token)
                .await
                .is_err());
            assert_eq!(
                session::verify_session(&pool, &replacement_token)
                    .await
                    .unwrap(),
                replacement.id
            );
            assert!(db::delete_auth_state(&pool, &state_id)
                .await
                .unwrap()
                .is_none());
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn admitted_link_rechecks_session_after_provider_identity_lock_wait() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let owner = test_user(&pool, "locked_link").await;
            let token = session::create_session(&pool, owner.id).await.unwrap();
            let (state, _server) = mock_google_state(pool.clone(), &owner.email).await;
            let state_id = format!("locked-link-{}", test_utils::uid());
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(owner.id),
            )
            .await;
            let mut identity_holder = pool.begin().await.unwrap();
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
                .bind(format!("google:subject-{}", owner.email))
                .execute(&mut *identity_holder)
                .await
                .unwrap();
            let callback = callback_route_handler(
                State(state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "valid-code".to_string(),
                }),
                add_test_browser_binding(
                    CookieJar::new().add(session::build_session_cookie(&token)),
                    &state_id,
                ),
            );
            let revoke_while_waiting = async {
                // Observe the actual provider-resolution wait before revoking,
                // so a check made before that await cannot satisfy this regression.
                loop {
                    let waiting: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype = 'advisory' AND NOT granted AND database = (SELECT oid FROM pg_database WHERE datname = current_database()))"
                    ).fetch_one(&pool).await.unwrap();
                    if waiting {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                session::revoke_session(&pool, &token).await.unwrap();
                identity_holder.rollback().await.unwrap();
            };
            let (response, ()) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
                tokio::join!(callback, revoke_while_waiting)
            })
            .await
            .expect("link and revocation should serialize without deadlock");
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!response.headers().contains_key("location"));
            assert_eq!(db::count_oauth_accounts(&pool, owner.id).await.unwrap(), 0);
            assert!(session::verify_session(&pool, &token).await.is_err());
            let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();
            assert_eq!(cookies.len(), 1);
            let cleared = cookie::Cookie::parse(cookies[0].to_str().unwrap()).unwrap();
            assert_eq!(cleared.name(), browser_binding_cookie_name(&state_id));
            assert_eq!(cleared.max_age(), Some(time::Duration::ZERO));
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn competing_guest_logins_issue_independent_sessions_in_response_completion_order() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let first_email = format!("delayed-login{}@example.com", test_utils::uid());
            let second_email = format!("later-login{}@example.com", test_utils::uid());
            let first = gated_google_state(pool.clone(), &first_email).await;
            let (second_state, _second_server) =
                mock_google_state(pool.clone(), &second_email).await;
            let first_id = format!("delayed-login-{}", test_utils::uid());
            let second_id = format!("later-login-{}", test_utils::uid());
            for state_id in [&first_id, &second_id] {
                insert_callback_state(
                    &pool,
                    state_id,
                    "google",
                    "/dashboard",
                    "test-verifier-minimum-43-characters-long-enough",
                    "login",
                    None,
                )
                .await;
            }
            let first_callback = callback_route_handler(
                State(first.state.clone()),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: first_id.clone(),
                    code: "first-code".to_string(),
                }),
                add_test_browser_binding(CookieJar::new(), &first_id),
            );
            let later_login = async {
                first.admitted.notified().await;
                let response = callback_route_handler(
                    State(second_state),
                    Path("google".to_string()),
                    Query(CallbackQuery {
                        state: second_id.clone(),
                        code: "second-code".to_string(),
                    }),
                    add_test_browser_binding(CookieJar::new(), &second_id),
                )
                .await;
                assert_eq!(response.status(), StatusCode::SEE_OTHER);
                assert!(
                    session::verify_session(&pool, response_session_cookie(&response).value())
                        .await
                        .is_ok()
                );
                first.release.notify_one();
                response
            };
            let (first_response, second_response) =
                tokio::time::timeout(std::time::Duration::from_secs(15), async {
                    tokio::join!(first_callback, later_login)
                })
                .await
                .expect("competing callbacks should complete");
            assert_eq!(first_response.status(), StatusCode::SEE_OTHER);
            let first_cookie = response_session_cookie(&first_response);
            let second_cookie = response_session_cookie(&second_response);
            let first_user = session::verify_session(&pool, first_cookie.value())
                .await
                .unwrap();
            let second_user = session::verify_session(&pool, second_cookie.value())
                .await
                .unwrap();
            assert_ne!(first_user, second_user);
            assert_eq!(
                db::get_user_by_id(&pool, first_user)
                    .await
                    .unwrap()
                    .unwrap()
                    .email,
                first_email
            );
            assert_eq!(
                db::get_user_by_id(&pool, second_user)
                    .await
                    .unwrap()
                    .unwrap()
                    .email,
                second_email
            );

            // Explicit header-delivery replay, not a claim about native browser
            // scheduling: the most recently applied cookie determines identity.
            let jar = CookieJar::new()
                .add(second_cookie.clone())
                .add(first_cookie.clone());
            assert_eq!(
                session::verify_session(
                    &pool,
                    jar.get(session::cookie_name().as_ref()).unwrap().value()
                )
                .await
                .unwrap(),
                first_user
            );
            let jar = jar.add(second_cookie);
            assert_eq!(
                session::verify_session(
                    &pool,
                    jar.get(session::cookie_name().as_ref()).unwrap().value()
                )
                .await
                .unwrap(),
                second_user
            );
            let sessions: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE revoked = FALSE")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(sessions, 2);
            for state_id in [&first_id, &second_id] {
                assert!(db::delete_auth_state(&pool, state_id)
                    .await
                    .unwrap()
                    .is_none());
            }
            session::clear_test_secret();
        }

        #[tokio::test]
        async fn unauthenticated_login_callback_creates_session() {
            session::set_test_secret("test-secret-32-bytes-long-enough!!");
            let (_db, pool) = test_utils::setup_test_db().await;
            let email = format!("login{}@example.com", test_utils::uid());
            let (state, _server) = mock_google_state(pool.clone(), &email).await;
            let state_id = format!("login-success-{}", test_utils::uid());
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/dashboard",
                "test-verifier-minimum-43-characters-long-enough",
                "login",
                None,
            )
            .await;

            let response = callback_handler(
                State(state),
                Path("google".to_string()),
                Query(CallbackQuery {
                    state: state_id.clone(),
                    code: "valid-code".to_string(),
                }),
                add_test_browser_binding(CookieJar::new(), &state_id),
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
            insert_callback_state(
                &pool,
                &state_id,
                "google",
                "/settings/accounts",
                "test-verifier-minimum-43-characters-long-enough",
                "link",
                Some(user.id),
            )
            .await;
            let jar = add_test_browser_binding(jar, &state_id);

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
            let set_cookies: Vec<_> = response
                .headers()
                .get_all(axum::http::header::SET_COOKIE)
                .iter()
                .map(|value| value.to_str().unwrap())
                .collect();
            assert_eq!(set_cookies.len(), 1);
            assert!(set_cookies[0].contains(BROWSER_BINDING_COOKIE_PREFIX));
            assert!(set_cookies[0].contains("Max-Age=0"));
            assert!(!set_cookies[0].contains(session::cookie_name().as_ref()));
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

            // Use the thread-local override so parallel tests cannot remove the
            // process-wide environment variable while this test is running.
            session::set_test_secret("test-secret-32-bytes-long-enough!!");

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
                &jwt,
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

            session::clear_test_secret();

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

            // Use the thread-local override so parallel tests cannot remove the
            // process-wide environment variable while this test is running.
            session::set_test_secret("test-secret-32-bytes-long-enough!!");

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

            session::clear_test_secret();
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
