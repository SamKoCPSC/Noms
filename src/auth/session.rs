//! Session management: JWT creation, verification, cookie building.
//!
//! Pure logic — no database dependency. All functions read the session secret
//! from the `SESSION_SECRET` environment variable at first use.

use cookie::{Cookie, CookieBuilder, SameSite};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
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
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::MissingSecret => write!(f, "SESSION_SECRET not set"),
            SessionError::InvalidToken => write!(f, "invalid session token"),
            SessionError::Expired => write!(f, "session token expired"),
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
/// Returns a compact JWT string valid for [`SESSION_LIFETIME_SECS`] seconds.
pub fn create_session(user_id: Uuid) -> Result<String, SessionError> {
    let secret = read_secret()?;
    let now = now_secs() as usize;
    let claims = SessionClaims {
        sub: user_id,
        exp: now + SESSION_LIFETIME_SECS as usize,
        iat: now,
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
/// Validates the signature and checks that the token has not expired.
#[allow(dead_code)] // Used by session refresh and auth middleware (not yet wired)
pub fn verify_session(token: &str) -> Result<Uuid, SessionError> {
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

    // Manual expiry check with specific error variant
    let now = now_secs() as usize;
    if token_data.claims.exp < now {
        return Err(SessionError::Expired);
    }

    Ok(token_data.claims.sub)
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
pub fn extract_user_id_from_fullstack() -> Option<uuid::Uuid> {
    use dioxus::fullstack::FullstackContext;

    let fsc = FullstackContext::current()?;
    let parts = fsc.parts_mut();
    let cookie_header = parts.headers.get(axum::http::header::COOKIE)?;
    let cookie_str = cookie_header.to_str().ok()?;

    // Parse cookies manually to find our session cookie
    let session_token = parse_cookie_value(cookie_str, COOKIE_NAME)?;
    verify_session(session_token).ok()
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
pub fn extract_user_id_from_headers(headers: &axum::http::HeaderMap) -> Option<uuid::Uuid> {
    use axum_extra::extract::cookie::CookieJar;

    let jar = CookieJar::from_headers(headers);
    let session_token = jar.get(COOKIE_NAME)?;
    verify_session(session_token.value()).ok()
}

/// Check if a valid session token is old enough to warrant a rolling refresh.
///
/// Returns `true` if the token was issued more than [`REFRESH_THRESHOLD_SECS`]
/// seconds ago. Returns an error if the token is invalid or expired.
pub fn should_refresh(token: &str) -> Result<bool, SessionError> {
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

    let age = now.saturating_sub(token_data.claims.iat);
    Ok(age >= REFRESH_THRESHOLD_SECS)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-32-bytes-long-enough!!";

    fn test_user_id() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

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

    #[test]
    fn create_and_verify_session() {
        with_secret(TEST_SECRET);
        let user_id = test_user_id();
        let token = create_session(user_id).unwrap();

        // Token is a non-empty string (3 base64url segments separated by dots)
        assert!(token.len() > 10);
        assert_eq!(token.matches('.').count(), 2);

        let verified_id = verify_session(&token).unwrap();
        assert_eq!(verified_id, user_id);
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        with_secret(TEST_SECRET);
        let user_id = test_user_id();
        let token = create_session(user_id).unwrap();

        // Switch to a different secret on this thread
        with_secret("different-secret-for-testing!!!");
        let result = verify_session(&token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }

    #[test]
    fn verify_rejects_malformed_token() {
        with_secret(TEST_SECRET);
        let result = verify_session("not.a.valid.token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }

    #[test]
    fn verify_rejects_expired_token() {
        with_secret(TEST_SECRET);
        let user_id = test_user_id();

        // Manually create an expired token
        let secret = read_secret().unwrap();
        let past = (now_secs() - SESSION_LIFETIME_SECS - 60) as usize;
        let claims = SessionClaims {
            sub: user_id,
            exp: past + SESSION_LIFETIME_SECS as usize, // expired 60s ago
            iat: past,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&secret),
        )
        .unwrap();

        let result = verify_session(&token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::Expired);
    }

    #[test]
    fn missing_secret_returns_error() {
        without_secret();
        let result = create_session(test_user_id());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::MissingSecret);
    }

    #[test]
    fn session_cookie_has_correct_attributes() {
        with_secret(TEST_SECRET);
        let token = create_session(test_user_id()).unwrap();
        let cookie = build_session_cookie(&token);

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
        let token = create_session(test_user_id()).unwrap();
        let cookie = build_session_cookie(&token);

        assert_eq!(cookie.domain(), Some("example.com"));
        without_cookie_domain();
    }

    #[test]
    fn session_cookie_has_no_domain_when_unset() {
        with_secret(TEST_SECRET);
        without_cookie_domain();
        let token = create_session(test_user_id()).unwrap();
        let cookie = build_session_cookie(&token);

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
        let token = create_session(test_user_id()).unwrap();
        let cookie = build_session_cookie(&token);

        assert_eq!(cookie.domain(), None);
        without_cookie_domain();
    }

    #[test]
    fn cookie_domain_whitespace_only_is_treated_as_unset() {
        with_secret(TEST_SECRET);
        with_cookie_domain("   ");
        let token = create_session(test_user_id()).unwrap();
        let cookie = build_session_cookie(&token);

        assert_eq!(cookie.domain(), None);
        without_cookie_domain();
    }

    #[test]
    fn fresh_token_does_not_need_refresh() {
        with_secret(TEST_SECRET);
        let token = create_session(test_user_id()).unwrap();
        let needs_refresh = should_refresh(&token).unwrap();
        assert!(!needs_refresh, "fresh token should not need refresh");
    }

    #[test]
    fn old_token_needs_refresh() {
        with_secret(TEST_SECRET);
        let user_id = test_user_id();

        // Create a token that was issued just past the refresh threshold
        let secret = read_secret().unwrap();
        let now = now_secs() as usize;
        let old_iat = now - (REFRESH_THRESHOLD_SECS + 1);
        let claims = SessionClaims {
            sub: user_id,
            exp: old_iat + SESSION_LIFETIME_SECS as usize,
            iat: old_iat,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&secret),
        )
        .unwrap();

        let needs_refresh = should_refresh(&token).unwrap();
        assert!(needs_refresh, "old token should need refresh");
    }

    #[test]
    fn should_refresh_rejects_invalid_token() {
        with_secret(TEST_SECRET);
        let result = should_refresh("garbage");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::InvalidToken);
    }

    #[test]
    fn should_refresh_rejects_expired_token() {
        with_secret(TEST_SECRET);
        let secret = read_secret().unwrap();
        let past = (now_secs() - SESSION_LIFETIME_SECS - 60) as usize;
        let claims = SessionClaims {
            sub: test_user_id(),
            exp: past + SESSION_LIFETIME_SECS as usize,
            iat: past,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&secret),
        )
        .unwrap();

        let result = should_refresh(&token);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), SessionError::Expired);
    }

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
