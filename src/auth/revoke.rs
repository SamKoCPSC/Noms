//! OAuth token revocation on account deletion.
//!
//! Calls provider-specific revocation endpoints when a user unlinks an OAuth
//! account or deletes their account. Failures are logged but never block
//! deletion. Google supports revocation via API; GitHub does not (documented
//! limitation). All HTTP calls use a 5-second timeout.

use std::time::Duration;
use tracing::{error, warn};

use crate::auth::linking::Provider;
use crate::db::OauthAccount;

/// Result of a revocation attempt, for logging purposes.
#[derive(Debug)]
#[allow(dead_code)]
pub enum RevokeResult {
    /// Token was revoked successfully.
    Success,
    /// Provider does not support revocation (e.g., GitHub).
    NotSupported { provider: String },
    /// The HTTP request timed out (5s).
    Timeout,
    /// A network error occurred (DNS failure, connection refused, etc.).
    NetworkError(String),
    /// The provider returned an HTTP error.
    HttpError(String),
    /// No refresh token was available to revoke.
    NoRefreshToken,
}

/// Revoke the OAuth token for a single account.
///
/// Returns a [`RevokeResult`] for logging. Never returns `Err`.
pub async fn revoke_account(account: &OauthAccount) -> RevokeResult {
    let refresh_token = match &account.refresh_token {
        Some(rt) if !rt.is_empty() => rt.clone(),
        _ => return RevokeResult::NoRefreshToken,
    };

    let provider = match account.provider.parse::<Provider>() {
        Ok(p) => p,
        Err(_) => {
            warn!(provider = %account.provider, "Unknown provider, skipping revocation");
            return RevokeResult::HttpError(format!("unknown provider: {}", account.provider));
        }
    };

    revoke_token(&refresh_token, provider).await
}

/// Revoke a token for a specific provider.
///
/// Uses a 5-second timeout. Failures are logged but never propagated.
pub async fn revoke_token(refresh_token: &str, provider: Provider) -> RevokeResult {
    let result = match provider {
        Provider::Google => revoke_google(refresh_token).await,
        Provider::GitHub => {
            warn!(
                provider = "github",
                "GitHub has no token revocation API; token will expire naturally (~8 years)"
            );
            return RevokeResult::NotSupported {
                provider: "github".to_string(),
            };
        }
        Provider::Apple => {
            warn!(
                provider = "apple",
                "Apple token revocation not yet implemented"
            );
            return RevokeResult::NotSupported {
                provider: "apple".to_string(),
            };
        }
    };

    match &result {
        RevokeResult::Success => {
            tracing::info!(provider = ?provider, "Token revoked successfully");
        }
        other => {
            error!(provider = ?provider, result = ?other, "Token revocation failed");
        }
    }

    result
}

/// Revoke a Google OAuth token.
///
/// POST https://oauth2.googleapis.com/revoke?token=...
/// No auth header required. Returns 200 on success.
/// Accepts both access tokens and refresh tokens.
async fn revoke_google(refresh_token: &str) -> RevokeResult {
    revoke_google_with_url("https://oauth2.googleapis.com", refresh_token).await
}

/// Internal: revoke with configurable base URL (for testing).
async fn revoke_google_with_url(base_url: &str, refresh_token: &str) -> RevokeResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => return RevokeResult::NetworkError(format!("Failed to build HTTP client: {e}")),
    };

    let url = format!("{base_url}/revoke?token={refresh_token}");

    let response = match client.post(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            if e.is_timeout() {
                return RevokeResult::Timeout;
            }
            return RevokeResult::NetworkError(e.to_string());
        }
    };

    if response.status().is_success() {
        RevokeResult::Success
    } else {
        RevokeResult::HttpError(format!(
            "HTTP {}: {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ))
    }
}

/// Revoke tokens for all OAuth accounts of a user.
///
/// Used in the account deletion flow. Fetches all accounts, then revokes
/// each one. Failures are logged but never block the deletion.
pub async fn revoke_all_user_tokens(pool: &sqlx::PgPool, user_id: uuid::Uuid) {
    let accounts = match crate::db::get_oauth_accounts_by_user_id(pool, user_id).await {
        Ok(accs) => accs,
        Err(e) => {
            error!(user_id = %user_id, "Failed to fetch OAuth accounts for revocation: {e}");
            return;
        }
    };

    for account in accounts {
        let result = revoke_account(&account).await;
        match result {
            RevokeResult::Success => {
                tracing::info!(
                    user_id = %user_id,
                    provider = %account.provider,
                    "Token revoked"
                );
            }
            RevokeResult::NoRefreshToken => {
                tracing::debug!(
                    user_id = %user_id,
                    provider = %account.provider,
                    "No refresh token to revoke"
                );
            }
            other => {
                warn!(
                    user_id = %user_id,
                    provider = %account.provider,
                    result = ?other,
                    "Token revocation failed (non-fatal)"
                );
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;

    #[tokio::test]
    async fn test_revoke_github_not_supported() {
        let result = revoke_token("dummy_token", Provider::GitHub).await;
        assert!(matches!(result, RevokeResult::NotSupported { .. }));
    }

    #[tokio::test]
    async fn test_revoke_google_success() {
        let server = MockServer::start().await;

        wiremock::Mock::given(method("POST"))
            .and(path("/revoke"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;

        let result = revoke_google_with_url(&server.uri(), "test_refresh_token").await;
        assert!(matches!(result, RevokeResult::Success));
    }

    #[tokio::test]
    async fn test_revoke_google_timeout() {
        let server = MockServer::start().await;

        // Mock a slow endpoint that never responds within the 5s timeout
        wiremock::Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{}")
                    .set_delay(Duration::from_secs(10)),
            )
            .mount(&server)
            .await;

        let result = revoke_google_with_url(&server.uri(), "test_refresh_token").await;
        assert!(matches!(result, RevokeResult::Timeout));
    }

    #[tokio::test]
    async fn test_revoke_google_http_error() {
        let server = MockServer::start().await;

        wiremock::Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_string("invalid_grant"))
            .mount(&server)
            .await;

        let result = revoke_google_with_url(&server.uri(), "bad_token").await;
        match result {
            RevokeResult::HttpError(msg) => {
                assert!(msg.contains("400"), "expected 400 in error: {msg}");
            }
            other => panic!("expected HttpError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_revoke_google_network_error() {
        // Point at an unreachable URL — should produce a network error, not a timeout
        // (the connection will fail quickly)
        let result = revoke_google_with_url("http://localhost:59999", "test_token").await;
        match result {
            RevokeResult::NetworkError(msg) => {
                // Could be connection refused, connection timed out, etc.
                assert!(!msg.is_empty(), "error message should not be empty");
            }
            RevokeResult::Timeout => {
                // On some systems this might time out instead — still acceptable
            }
            other => panic!("expected NetworkError or Timeout, got {other:?}"),
        }
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!("google".parse::<Provider>().unwrap(), Provider::Google);
        assert_eq!("github".parse::<Provider>().unwrap(), Provider::GitHub);
        assert_eq!("apple".parse::<Provider>().unwrap(), Provider::Apple);
        assert!("unknown".parse::<Provider>().is_err());
    }
}
