//! Session management: JWT creation, verification, cookie building.
//!
//! All session operations are backed by the `sessions` database table.
//! The JWT `sub` claim is the session ID (UUID of a row in `sessions`),
//! not the user ID. `verify_session` looks up the DB row to get the `user_id`
//! and check `revoked` / `expires_at`.

use cookie::{Cookie, CookieBuilder, SameSite};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use time::Duration as TimeDuration;
use uuid::Uuid;

/// Cookie name for the session token.
pub const COOKIE_NAME: &str = "noms_session";

/// Session lifetime: 15 minutes.
const SESSION_LIFETIME_SECS: u64 = 900;

/// Rolling refresh threshold: refresh when the token is older than 10 minutes.
const REFRESH_THRESHOLD_SECS: usize = 600;

/// JWT claims for a session token.
///
/// The `sub` field is the **session ID** (UUID of a row in the `sessions` table),
/// NOT the user ID. To get the user ID, look up the session in the database.
#[derive(Debug, Serialize, Deserialize)]
struct SessionClaims {
    sub: Uuid,
    exp: usize,
    iat: usize,
}

/// Errors from session operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // All variants used by callers
pub enum SessionError {
    /// The `SESSION_SECRET` environment variable is not set.
    MissingSecret,
    /// The token signature is invalid or the token is malformed.
    InvalidToken,
    /// The token has expired.
    Expired,
    /// The session was not found in the database, revoked, or expired.
    SessionInvalid,
    /// Database error during session operation.
    DbError(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::MissingSecret => write!(f, "SESSION_SECRET not set"),
            SessionError::InvalidToken => write!(f, "invalid session token"),
            SessionError::Expired => write!(f, "session token expired"),
            SessionError::SessionInvalid => write!(f, "session is invalid, revoked, or expired"),
            SessionError::DbError(e) => write!(f, "database error: {e}"),
        }
    }
}

impl std::error::Error for SessionError {}

// Thread-local overrides for test isolation (used in tests).
// When set, takes precedence over the environment variables.
#[cfg(test)]
thread_local! {
    static TEST_SECRET: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
thread_local! {
    static TEST_COOKIE_DOMAIN: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Set the thread-local test secret. Used by tests outside this module.
#[cfg(test)]
pub fn set_test_secret(secret: &'static str) {
    TEST_SECRET.with(|f| {
        *f.borrow_mut() = Some(secret.as_bytes().to_vec());
    });
}

/// Clear the thread-local test secret. Used by tests outside this module.
#[cfg(test)]
#[allow(dead_code)]
pub fn clear_test_secret() {
    TEST_SECRET.with(|f| {
        *f.borrow_mut() = None;
    });
}

/// Reads the session secret from the `SESSION_SECRET` environment variable.
#[cfg(not(test))]
fn read_secret() -> Result<Vec<u8>, SessionError> {
    std::env::var("SESSION_SECRET").map_or(Err(SessionError::MissingSecret), |s| Ok(s.into_bytes()))
}

#[cfg(test)]
fn read_secret() -> Result<Vec<u8>, SessionError> {
    if let Some(secret) = TEST_SECRET.with(|f| f.borrow().clone()) {
        return Ok(secret);
    }
    std::env::var("SESSION_SECRET").map_or(Err(SessionError::MissingSecret), |s| Ok(s.into_bytes()))
}

/// Reads the cookie domain from the `COOKIE_DOMAIN` environment variable.
/// Returns `None` if not set or if the value is empty/whitespace-only.
#[cfg(not(test))]
fn read_cookie_domain() -> Option<String> {
    std::env::var("COOKIE_DOMAIN")
        .ok()
        .filter(|d| !d.trim().is_empty())
}

#[cfg(test)]
fn read_cookie_domain() -> Option<String> {
    if let Some(domain) = TEST_COOKIE_DOMAIN.with(|f| f.borrow().clone()) {
        return Some(domain).filter(|d| !d.trim().is_empty());
    }
    std::env::var("COOKIE_DOMAIN")
        .ok()
        .filter(|d| !d.trim().is_empty())
}

/// Current unix timestamp in seconds.
#[inline]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Create a signed JWT session token for the given user.
///
/// Inserts a row into the `sessions` table, then returns a compact JWT
/// with `sub = session_id`. The JWT is valid for [`SESSION_LIFETIME_SECS`] seconds.
pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, SessionError> {
    let secret = read_secret()?;
    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::seconds(SESSION_LIFETIME_SECS as i64);

    // Insert session row into DB
    let session_row = crate::db::insert_session(pool, user_id, expires_at)
        .await
        .map_err(|e| SessionError::DbError(e.to_string()))?;

    let session_id = session_row.id;
    let now_secs = now_secs() as usize;
    let claims = SessionClaims {
        sub: session_id, // <-- session_id, NOT user_id
        exp: now_secs + SESSION_LIFETIME_SECS as usize,
        iat: now_secs,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
    .map_err(|_| SessionError::InvalidToken)
}

/// Verify a session token and return the user ID.
///
/// Decodes the JWT to get the session_id (`sub` claim), then looks up the
/// session in the database. Returns the `user_id` from the DB row if the
/// session exists, is not revoked, and is not expired.
#[allow(dead_code)] // Used by session refresh and auth middleware
pub async fn verify_session(pool: &PgPool, token: &str) -> Result<Uuid, SessionError> {
    let secret = read_secret()?;
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    // Allow decoding expired tokens so we can return a specific error variant.
    validation.validate_exp = false;

    let token_data =
        decode::<SessionClaims>(token, &DecodingKey::from_secret(&secret), &validation).map_err(
            |e| {
                if *e.kind() == jsonwebtoken::errors::ErrorKind::ExpiredSignature {
                    SessionError::Expired
                } else {
                    SessionError::InvalidToken
                }
            },
        )?;

    // Manual expiry check
    let now = now_secs() as usize;
    if token_data.claims.exp < now {
        return Err(SessionError::Expired);
    }

    // Look up session in DB
    let session_id = token_data.claims.sub;
    let session_row = crate::db::get_active_session(pool, session_id)
        .await
        .map_err(|e| SessionError::DbError(e.to_string()))?
        .ok_or(SessionError::SessionInvalid)?;

    Ok(session_row.user_id)
}

/// Build an HttpOnly, Secure session cookie from a JWT token.
///
/// The cookie is set to `/`, has `SameSite=Lax`, and a max-age matching
/// the session lifetime. In local development (detected via `NOMS_ENV=local`),
/// the `Secure` flag is disabled so cookies work over HTTP.
pub fn build_session_cookie(token: &str) -> Cookie<'static> {
    let is_local = std::env::var("NOMS_ENV").ok() == Some("local".to_string());
    let domain = read_cookie_domain();
    let mut builder = CookieBuilder::new(COOKIE_NAME, token.to_owned())
        .http_only(true)
        .secure(!is_local)
        .path("/")
        .max_age(TimeDuration::seconds(SESSION_LIFETIME_SECS as i64))
        .same_site(SameSite::Lax);

    if let Some(d) = domain {
        builder = builder.domain(d);
    }

    builder.build()
}

/// Build a cookie that deletes the existing session cookie.
///
/// Sets the same name/path with max-age 0 so the browser discards it immediately.
pub fn clear_session_cookie() -> Cookie<'static> {
    let is_local = std::env::var("NOMS_ENV").ok() == Some("local".to_string());
    let domain = read_cookie_domain();
    let mut builder = CookieBuilder::new(COOKIE_NAME, "")
        .http_only(true)
        .secure(!is_local)
        .path("/")
        .max_age(TimeDuration::ZERO)
        .same_site(SameSite::Lax);

    if let Some(d) = domain {
        builder = builder.domain(d);
    }

    builder.build()
}

/// Extract the authenticated user ID from `FullstackContext` request headers.
///
/// Reads the session cookie from the `Cookie` header of the current request,
/// verifies the JWT, and returns the user ID if valid.
/// Returns `None` if no valid session is found.
///
/// This is the reliable way to authenticate inside server functions, since
/// `FullstackContext::extension::<AuthUser>()` may not propagate extensions
/// from the auth middleware correctly.
#[cfg(feature = "server")]
pub async fn extract_user_id_from_fullstack() -> Option<uuid::Uuid> {
    use dioxus::fullstack::FullstackContext;

    // Extract cookie data synchronously to avoid holding the FullstackContext
    // mutex guard across the .await point below. Clone the token to own the
    // string since the references into the headers don't outlive this block.
    let session_token = {
        let fsc = FullstackContext::current()?;
        let parts = fsc.parts_mut();
        let cookie_header = parts.headers.get(axum::http::header::COOKIE)?;
        let cookie_str = cookie_header.to_str().ok()?;
        parse_cookie_value(cookie_str, COOKIE_NAME)?.to_string()
    };

    let pool = crate::db::get_pool();
    verify_session(&pool, &session_token).await.ok()
}

/// Parse a specific cookie value from a `Cookie` header string.
///
/// Cookie header format: `name1=value1; name2=value2; ...`
fn parse_cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some((k, v)) = pair.split_once('=') {
            if k.trim() == name {
                return Some(v.trim());
            }
        }
    }
    None
}

/// Extract the authenticated user ID from request headers (legacy).
///
/// Reads the session cookie from the `Cookie` header, verifies the JWT,
/// and returns the user ID if valid. Returns `None` if no valid session.
#[cfg(feature = "server")]
#[allow(dead_code)] // Kept for potential future use
pub async fn extract_user_id_from_headers(
    pool: &PgPool,
    headers: &axum::http::HeaderMap,
) -> Option<uuid::Uuid> {
    use axum_extra::extract::cookie::CookieJar;

    let jar = CookieJar::from_headers(headers);
    let session_token = jar.get(COOKIE_NAME)?;
    verify_session(pool, session_token.value()).await.ok()
}

/// Check if a valid session token is old enough to warrant a rolling refresh.
///
/// Decodes the JWT to get the session_id, looks up the session in the DB,
/// and checks if the session is within the last 10 minutes of expiry.
/// Returns `true` if refresh is needed.
pub async fn should_refresh(pool: &PgPool, token: &str) -> Result<bool, SessionError> {
    let secret = read_secret()?;
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;

    let token_data =
        decode::<SessionClaims>(token, &DecodingKey::from_secret(&secret), &validation)
            .map_err(|_| SessionError::InvalidToken)?;

    let now = now_secs() as usize;
    if token_data.claims.exp < now {
        return Err(SessionError::Expired);
    }

    // Look up session in DB to check actual DB-side expiry
    let session_id = token_data.claims.sub;
    let session_row = crate::db::get_active_session(pool, session_id)
        .await
        .map_err(|e| SessionError::DbError(e.to_string()))?
        .ok_or(SessionError::SessionInvalid)?;

    // Check if within last 10 minutes of expiry
    let now_utc = chrono::Utc::now();
    let time_until_expiry = session_row.expires_at.signed_duration_since(now_utc);
    Ok(time_until_expiry.num_seconds() <= REFRESH_THRESHOLD_SECS as i64)
}

/// Revoke a session by extracting the session_id from the JWT and setting
/// `revoked = TRUE` in the database.
pub async fn revoke_session(pool: &PgPool, token: &str) -> Result<(), SessionError> {
    let secret = read_secret()?;
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;

    let token_data =
        decode::<SessionClaims>(token, &DecodingKey::from_secret(&secret), &validation)
            .map_err(|_| SessionError::InvalidToken)?;

    let session_id = token_data.claims.sub;
    crate::db::revoke_session(pool, session_id)
        .await
        .map_err(|e| SessionError::DbError(e.to_string()))?;

    Ok(())
}

/// Refresh a session: extend its expiry and return a new JWT.
///
/// Updates the DB row (`expires_at`, `refreshed_at`), then creates a new JWT.
#[allow(dead_code)] // Public API - used by auth middleware rolling refresh path
pub async fn refresh_session(pool: &PgPool, token: &str) -> Result<String, SessionError> {
    let secret = read_secret()?;
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;

    let token_data =
        decode::<SessionClaims>(token, &DecodingKey::from_secret(&secret), &validation)
            .map_err(|_| SessionError::InvalidToken)?;

    let session_id = token_data.claims.sub;

    // Update DB row
    let new_expires_at =
        chrono::Utc::now() + chrono::Duration::seconds(SESSION_LIFETIME_SECS as i64);
    let _session_row = crate::db::refresh_session(pool, session_id, new_expires_at)
        .await
        .map_err(|e| SessionError::DbError(e.to_string()))?
        .ok_or(SessionError::SessionInvalid)?;

    // Create new JWT with same session_id but new expiry
    let now_secs = now_secs() as usize;
    let claims = SessionClaims {
        sub: session_id,
        exp: now_secs + SESSION_LIFETIME_SECS as usize,
        iat: now_secs,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    )
    .map_err(|_| SessionError::InvalidToken)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    const TEST_SECRET: &str = "test-secret-32-bytes-long-enough!!";

    /// Set the thread-local secret. Each test thread gets its own value.
    fn with_secret(secret: &'static str) {
        super::TEST_SECRET.with(|f| {
            *f.borrow_mut() = Some(secret.as_bytes().to_vec());
        });
    }

    /// Clear the thread-local secret (simulates missing env var).
    fn without_secret() {
        super::TEST_SECRET.with(|f| {
            *f.borrow_mut() = None;
        });
        // Also remove env var so the fallback path is tested
        std::env::remove_var("SESSION_SECRET");
    }

    // ── DB-backed session tests ──────────────────────────────────────────

    #[tokio::test]
    async fn create_and_verify_session() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a test user
        let user = crate::db::insert_user(
            &pool,
            &format!("sessionuser_{u}"),
            "Session User",
            &format!("session{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Create session
        let token = create_session(&pool, user.id).await.unwrap();

        // Token is a non-empty string (3 base64url segments separated by dots)
        assert!(token.len() > 10);
        assert_eq!(token.matches('.').count(), 2);

        // Verify returns the user ID
        let verified_id = verify_session(&pool, &token).await.unwrap();
        assert_eq!(verified_id, user.id);
    }

    #[tokio::test]
    async fn create_session_inserts_db_row() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("dbrow_{u}"),
            "DB Row User",
            &format!("dbrow{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let _token = create_session(&pool, user.id).await.unwrap();

        // Verify a row exists in the sessions table
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn verify_rejects_wrong_secret() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("wrongsec_{u}"),
            "Wrong Secret User",
            &format!("wrongsec{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let token = create_session(&pool, user.id).await.unwrap();

        // Switch to a different secret on this thread
        with_secret("different-secret-for-testing!!!");
        let result = verify_session(&pool, &token).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }

    #[tokio::test]
    async fn verify_rejects_malformed_token() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;

        let result = verify_session(&pool, "not.a.valid.token").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }

    #[tokio::test]
    async fn verify_rejects_expired_token() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("expired_{u}"),
            "Expired User",
            &format!("expired{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Create a session
        let token = create_session(&pool, user.id).await.unwrap();

        // Backdate the session in the DB to make it expired
        sqlx::query(
            "UPDATE sessions SET expires_at = NOW() - INTERVAL '1 hour' WHERE user_id = $1",
        )
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();

        let result = verify_session(&pool, &token).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::SessionInvalid);
    }

    #[tokio::test]
    async fn verify_rejects_revoked_session() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("revoked_{u}"),
            "Revoked User",
            &format!("revoked{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let token = create_session(&pool, user.id).await.unwrap();

        // Verify it works first
        let verified = verify_session(&pool, &token).await.unwrap();
        assert_eq!(verified, user.id);

        // Revoke the session
        revoke_session(&pool, &token).await.unwrap();

        // Now verify should fail
        let result = verify_session(&pool, &token).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::SessionInvalid);
    }

    #[tokio::test]
    async fn test_revoke_session() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("revoke_{u}"),
            "Revoke User",
            &format!("revoke{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let token = create_session(&pool, user.id).await.unwrap();

        // Revoke
        revoke_session(&pool, &token).await.unwrap();

        // Check DB: session should be revoked
        let session = crate::db::get_active_session(
            &pool,
            Uuid::parse_str(
                &jsonwebtoken::decode::<SessionClaims>(
                    &token,
                    &DecodingKey::from_secret(TEST_SECRET.as_bytes()),
                    &Validation::new(jsonwebtoken::Algorithm::HS256),
                )
                .unwrap()
                .claims
                .sub
                .to_string(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
        assert!(
            session.is_none(),
            "revoked session should not be found as active"
        );
    }

    #[tokio::test]
    async fn test_refresh_session_updates_db() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("refresh_{u}"),
            "Refresh User",
            &format!("refresh{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let token = create_session(&pool, user.id).await.unwrap();

        // Refresh the session
        let new_token = refresh_session(&pool, &token).await.unwrap();

        // New token should be valid
        let verified = verify_session(&pool, &new_token).await.unwrap();
        assert_eq!(verified, user.id);

        // Only one session row should exist (refresh updates in place)
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE user_id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn missing_secret_returns_error() {
        without_secret();
        // create_session requires a PgPool, so we can't call it without a runtime.
        // But we can verify the read_secret path directly.
        let result = read_secret();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::MissingSecret);
    }

    // ── Cookie tests (no DB needed) ──────────────────────────────────────

    #[test]
    fn session_cookie_has_correct_attributes() {
        with_secret(TEST_SECRET);
        let token = "test-jwt-token-string";
        let cookie = build_session_cookie(token);

        assert_eq!(cookie.name(), COOKIE_NAME);
        assert_eq!(cookie.value(), token);
        assert!(cookie.http_only().unwrap_or(false));
        assert!(cookie.secure().unwrap_or(false));
        assert!(cookie.path() == Some("/"));
        assert_eq!(
            cookie.max_age(),
            Some(TimeDuration::seconds(SESSION_LIFETIME_SECS as i64))
        );
    }

    #[test]
    fn clear_cookie_has_zero_max_age() {
        with_secret(TEST_SECRET);
        let cookie = clear_session_cookie();

        assert_eq!(cookie.name(), COOKIE_NAME);
        assert_eq!(cookie.value(), "");
        assert_eq!(cookie.max_age(), Some(TimeDuration::ZERO));
        assert!(cookie.http_only().unwrap_or(false));
        assert!(cookie.secure().unwrap_or(false));
    }

    /// Set the thread-local cookie domain. Each test thread gets its own value.
    fn with_cookie_domain(domain: &'static str) {
        super::TEST_COOKIE_DOMAIN.with(|f| {
            *f.borrow_mut() = Some(domain.to_string());
        });
    }

    /// Clear the thread-local cookie domain.
    fn without_cookie_domain() {
        super::TEST_COOKIE_DOMAIN.with(|f| {
            *f.borrow_mut() = None;
        });
        std::env::remove_var("COOKIE_DOMAIN");
    }

    #[test]
    fn session_cookie_includes_domain_when_set() {
        with_secret(TEST_SECRET);
        with_cookie_domain(".example.com");
        let cookie = build_session_cookie("test-token");

        assert_eq!(cookie.domain(), Some("example.com"));
        without_cookie_domain();
    }

    #[test]
    fn session_cookie_has_no_domain_when_unset() {
        with_secret(TEST_SECRET);
        without_cookie_domain();
        let cookie = build_session_cookie("test-token");

        assert_eq!(cookie.domain(), None);
    }

    #[test]
    fn clear_cookie_includes_domain_when_set() {
        with_secret(TEST_SECRET);
        with_cookie_domain(".example.com");
        let cookie = clear_session_cookie();

        assert_eq!(cookie.domain(), Some("example.com"));
        assert_eq!(cookie.max_age(), Some(TimeDuration::ZERO));
        without_cookie_domain();
    }

    #[test]
    fn clear_cookie_has_no_domain_when_unset() {
        with_secret(TEST_SECRET);
        without_cookie_domain();
        let cookie = clear_session_cookie();

        assert_eq!(cookie.domain(), None);
        assert_eq!(cookie.max_age(), Some(TimeDuration::ZERO));
    }

    #[test]
    fn cookie_domain_empty_string_is_treated_as_unset() {
        with_secret(TEST_SECRET);
        with_cookie_domain("");
        let cookie = build_session_cookie("test-token");

        assert_eq!(cookie.domain(), None);
        without_cookie_domain();
    }

    #[test]
    fn cookie_domain_whitespace_only_is_treated_as_unset() {
        with_secret(TEST_SECRET);
        with_cookie_domain("   ");
        let cookie = build_session_cookie("test-token");

        assert_eq!(cookie.domain(), None);
        without_cookie_domain();
    }

    // ── DB-backed refresh tests ──────────────────────────────────────────

    #[tokio::test]
    async fn fresh_token_does_not_need_refresh() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("fresh_{u}"),
            "Fresh User",
            &format!("fresh{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let token = create_session(&pool, user.id).await.unwrap();
        let needs_refresh = should_refresh(&pool, &token).await.unwrap();
        assert!(!needs_refresh, "fresh token should not need refresh");
    }

    #[tokio::test]
    async fn should_refresh_rejects_invalid_token() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;

        let result = should_refresh(&pool, "garbage").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }

    #[tokio::test]
    async fn should_refresh_rejects_expired_token() {
        with_secret(TEST_SECRET);
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = crate::db::insert_user(
            &pool,
            &format!("srefresh_{u}"),
            "Should Refresh User",
            &format!("srefresh{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let token = create_session(&pool, user.id).await.unwrap();

        // Backdate the session in the DB to make it expired
        sqlx::query(
            "UPDATE sessions SET expires_at = NOW() - INTERVAL '1 hour' WHERE user_id = $1",
        )
        .bind(user.id)
        .execute(&pool)
        .await
        .unwrap();

        let result = should_refresh(&pool, &token).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::SessionInvalid);
    }

    // ── Cookie parsing tests (no DB needed) ──────────────────────────────

    #[test]
    fn parse_cookie_value_finds_single_cookie() {
        let val = parse_cookie_value("noms_session=abc123", "noms_session");
        assert_eq!(val, Some("abc123"));
    }

    #[test]
    fn parse_cookie_value_finds_among_multiple() {
        let val = parse_cookie_value("other=val; noms_session=abc123; another=x", "noms_session");
        assert_eq!(val, Some("abc123"));
    }

    #[test]
    fn parse_cookie_value_handles_spaces() {
        let val = parse_cookie_value("  other = val ;  noms_session = abc123 ; ", "noms_session");
        assert_eq!(val, Some("abc123"));
    }

    #[test]
    fn parse_cookie_value_missing_returns_none() {
        let val = parse_cookie_value("other=val; another=x", "noms_session");
        assert_eq!(val, None);
    }

    #[test]
    fn parse_cookie_value_empty_header_returns_none() {
        let val = parse_cookie_value("", "noms_session");
        assert_eq!(val, None);
    }
}
