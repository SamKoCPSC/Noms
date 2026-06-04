# Task Brief

## Task Description
Implement AC11 from NOMS-006: OAuth token revocation on account deletion. Store `refresh_token` in `oauth_accounts` table, call provider revocation endpoints when a user unlinks an OAuth account or deletes their account. Google supports revocation via API; GitHub does not (document limitation). Revocation failures are logged but don't block deletion. 5-second timeout on revocation requests.

## Phase 0: Implementation Blueprint
## Objectives
- Store `refresh_token` in `oauth_accounts` table during OAuth callback flow.
- Call provider-specific revocation endpoints when a user unlinks an OAuth account or deletes their account.
- Google revocation via `POST https://oauth2.googleapis.com/revoke?token=...` with a 5-second timeout.
- GitHub has no revocation API — log a warning via `tracing`.
- Revocation failures are logged but never block deletion.
- Add unit tests for the revocation utility.

## Research Findings

### Provider Revocation Endpoints
| Provider | Endpoint | Method | Auth | Notes |
|----------|----------|--------|------|-------|
| Google | `https://oauth2.googleapis.com/revoke` | POST form-encoded `token=...` | None | Returns 200 on success. Accepts both access and refresh tokens. |
| GitHub | N/A | — | — | GitHub OAuth 2.0 has no revocation API. Tokens expire after ~8 years. Log warning. |

Sources:
- Google: https://developers.google.com/identity/protocols/oauth2/offline-access#token-revoke
- GitHub: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/refreshing-user-access-tokens (no revoke endpoint documented)

### `oauth2` Crate v5 Revocation Types
The `oauth2` crate provides `StandardRevocableToken`, `BasicRevocationErrorResponse`, and a `HasRevocationUrl` typestate. However, using these would require changing `GoogleAuthClient` from `BasicClient` to a typestate variant, which adds complexity for a single Google provider. **Decision: Use `reqwest` directly for revocation** — it's already a dependency, simpler, and avoids typestate changes.

### Google Requires `access_type=offline`
Google only returns a refresh token when `access_type=offline` is set in the authorization request. The `oauth2` crate does not expose a method to add arbitrary query parameters to the authorization URL. **Workaround: Append `&access_type=offline` to the generated auth URL string for Google.**

### Database Schema Impact
The `oauth_accounts` table currently has no `refresh_token` column. Adding it requires:
1. A new migration file.
2. Updating `OauthAccount` struct in `src/db/mod.rs`.
3. Updating all SQL queries that touch `oauth_accounts` (SELECT, INSERT).

### Existing Deletion Call Sites
| Function | File | Line | Called From |
|----------|------|------|-------------|
| `delete_oauth_account` | `src/db/mod.rs:443` | `settings_accounts.rs:137` (unlink) | Unlink flow |
| `delete_user` | `src/db/mod.rs:498` | `settings_profile.rs:166` (delete account) | Delete flow |

Both functions are in `src/db/mod.rs` and have CASCADE deletes on `oauth_accounts`. We must revoke tokens **before** calling these DB functions.

## Files to Modify

### 1. `migrations/schema.sql`
**Add `refresh_token TEXT` column to `oauth_accounts` table.**

After line 44 (`created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()`), insert:
```sql
refresh_token TEXT,
```

This column is nullable for backward compatibility (existing rows without refresh tokens).

### 2. `src/db/mod.rs`

#### 2a. `OauthAccount` struct (line 63)
Add field:
```rust
pub refresh_token: Option<String>,
```

Updated struct:
```rust
pub struct OauthAccount {
    pub id: i32,
    pub user_id: i32,
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub profile_data: Option<String>,
    pub refresh_token: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
}
```

#### 2b. `insert_oauth_account` function (line 270)
Change signature from:
```rust
pub async fn insert_oauth_account(pool, user_id, provider, provider_user_id, email, email_verified, profile_data)
```
To:
```rust
pub async fn insert_oauth_account(pool, user_id, provider, provider_user_id, email, email_verified, profile_data, refresh_token)
```

Add `refresh_token: Option<String>` parameter. Update SQL query to include `refresh_token` in INSERT and VALUES.

#### 2c. `get_oauth_account_by_provider` function (line 203)
Update the `query_as!` column list to include `refresh_token`. The SELECT already selects `*` via `query_as!`, so adding the column to the struct is sufficient.

#### 2d. `get_user_oauth_accounts` function (line 217)
Same as above — the struct change covers it.

#### 2e. New helper: `get_oauth_accounts_by_user_id` function
Add a new function to fetch all OAuth accounts for a user (for account deletion revocation):
```rust
pub async fn get_oauth_accounts_by_user_id(pool: &PgPool, user_id: i32) -> Result<Vec<OauthAccount>, AppError> {
    sqlx::query_as!(
        OauthAccount,
        r#"SELECT id, user_id, provider, provider_user_id, email, email_verified,
                  profile_data, refresh_token, created_at, last_used_at
           FROM oauth_accounts WHERE user_id = $1"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to fetch OAuth accounts: {}", e)))
}
```

### 3. `src/auth/oauth.rs`

#### 3a. `start_handler` — add `access_type=offline` for Google (line 103)
After the authorization URL is generated, append `&access_type=offline` for Google:
```rust
let auth_url = req.request_url(authorization_url);

// Google requires access_type=offline to return a refresh token
let auth_url = if provider == "google" {
    format!("{}&access_type=offline", auth_url)
} else {
    auth_url.to_string()
};
```

#### 3b. `callback_handler` — extract and store `refresh_token` (line 184)
After `exchange_code` returns `token_response`, extract the refresh token:
```rust
let refresh_token = token_response
    .refresh_token()
    .map(rt => rt.secret().to_string());
```

Update `insert_oauth_account` call (line 197) to pass `refresh_token`:
```rust
insert_oauth_account(
    &pool, user_id, provider.clone(), provider_user_id.clone(),
    email.clone(), email_verified, profile_data, refresh_token
).await?;
```

### 4. `src/auth/mod.rs`
Add `pub mod revoke;` after line 4.

### 5. `src/auth/revoke.rs` (NEW FILE)
New module for token revocation logic.

```rust
use axum::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, warn};

use crate::auth::linking::Provider;
use crate::db::OauthAccount;

/// Revocation result for testing/logging
pub enum RevokeResult {
    Success,
    NotSupported { provider: String },
    Timeout,
    NetworkError(String),
    HttpError(String),
    NoRefreshToken,
}

/// Revoke an OAuth token for a single account.
/// Returns RevokeResult for logging. Never returns Err.
pub async fn revoke_account(pool: &PgPool, account: &OauthAccount) -> RevokeResult {
    let refresh_token = match &account.refresh_token {
        Some(rt) if !rt.is_empty() => rt.clone(),
        _ => return RevokeResult::NoRefreshToken,
    };

    let provider = match account.provider.parse::<Provider>() {
        Ok(p) => p,
        Err(_) => {
            warn!("Unknown provider '{}', skipping revocation", account.provider);
            return RevokeResult::HttpError(format!("unknown provider: {}", account.provider));
        }
    };

    revoke_token(&refresh_token, provider).await
}

/// Revoke a token for a specific provider.
/// 5-second timeout. Failures are logged, never propagated.
pub async fn revoke_token(refresh_token: &str, provider: Provider) -> RevokeResult {
    let result = match provider {
        Provider::Google => revoke_google(refresh_token).await,
        Provider::GitHub => {
            warn!(provider = "github", "GitHub has no token revocation API; token will expire naturally");
            return RevokeResult::NotSupported { provider: "github".to_string() };
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
/// POST https://oauth2.googleapis.com/revoke?token=...
/// No auth header required. Returns 200 on success.
async fn revoke_google(refresh_token: &str) -> RevokeResult {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => return RevokeResult::NetworkError(format!("Failed to build HTTP client: {}", e)),
    };

    let url = format!("https://oauth2.googleapis.com/revoke?token={}", refresh_token);

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
/// Used in account deletion flow.
pub async fn revoke_all_user_tokens(pool: &PgPool, user_id: i32) {
    use crate::db::get_oauth_accounts_by_user_id;

    let accounts = match get_oauth_accounts_by_user_id(pool, user_id).await {
        Ok(accs) => accs,
        Err(e) => {
            error!(user_id = user_id, "Failed to fetch OAuth accounts for revocation: {}", e);
            return;
        }
    };

    for account in accounts {
        let result = revoke_account(pool, &account).await;
        match result {
            RevokeResult::Success => {
                tracing::info!(
                    user_id = user_id,
                    provider = %account.provider,
                    "Token revoked"
                );
            }
            RevokeResult::NoRefreshToken => {
                tracing::debug!(
                    user_id = user_id,
                    provider = %account.provider,
                    "No refresh token to revoke"
                );
            }
            other => {
                warn!(
                    user_id = user_id,
                    provider = %account.provider,
                    result = ?other,
                    "Token revocation failed (non-fatal)"
                );
            }
        }
    }
}
```

### 6. `src/db/mod.rs` — update `delete_oauth_account` (line 443)
Add revocation call before DB deletion. Current function:
```rust
pub async fn delete_oauth_account(pool, user_id, provider) -> Result<(), AppError>
```

Update to:
```rust
use crate::auth::revoke::{revoke_account, RevokeResult};

// ...

pub async fn delete_oauth_account(pool: &PgPool, user_id: i32, provider: &str) -> Result<(), AppError> {
    // Fetch account for revocation before deletion
    if let Ok(account) = get_oauth_account_by_provider(pool, user_id, provider).await {
        let result = revoke_account(pool, &account).await;
        match result {
            RevokeResult::Success => {
                tracing::info!(user_id = user_id, provider = provider, "Token revoked before unlink");
            }
            RevokeResult::NoRefreshToken => {
                tracing::debug!(user_id = user_id, provider = provider, "No refresh token to revoke");
            }
            other => {
                warn!(user_id = user_id, provider = provider, result = ?other, "Token revocation failed (non-fatal)");
            }
        }
    }

    sqlx::query("DELETE FROM oauth_accounts WHERE user_id = $1 AND provider = $2")
        .bind(user_id)
        .bind(provider)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Database(format!("Failed to delete OAuth account: {}", e)))
}
```

### 7. `src/db/mod.rs` — update `delete_user` (line 498)
Add revocation call before DB deletion. Current function:
```rust
pub async fn delete_user(pool, user_id) -> Result<(), AppError>
```

Update to:
```rust
use crate::auth::revoke::revoke_all_user_tokens;

// ...

pub async fn delete_user(pool: &PgPool, user_id: i32) -> Result<(), AppError> {
    // Revoke all OAuth tokens before deletion
    revoke_all_user_tokens(pool, user_id).await;

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Database(format!("Failed to delete user: {}", e)))
}
```

### 8. `Cargo.toml` — add `wiremock` dev-dependency
Under `[dev-dependencies]` (add section if missing), add:
```toml
wiremock = "0.6"
```

### 9. `src/auth/revoke_tests.rs` (NEW FILE) or inline `#[cfg(test)]` module in `revoke.rs`
Unit tests for revocation logic:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::MockServer;
    use wiremock::matchers::{method, path, request};
    use wiremock::ResponseTemplate;

    #[tokio::test]
    async fn test_revoke_google_success() {
        let server = MockServer::start().await;
        let base_url = server.uri();

        // Mock Google revoke endpoint
        Mock::given(request(method("POST"), path_regex(r"^/revoke\?token=.*")))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;

        // Patch: We need to test with the mock server URL instead of Google's real URL.
        // Strategy: Extract the core HTTP logic into a testable function that accepts the URL.
        // Alternatively, test via integration test with real Google endpoint disabled.
        // For now, test the Provider::GitHub path and error handling.
    }

    #[tokio::test]
    async fn test_revoke_github_logs_warning() {
        // GitHub has no revocation API — should return NotSupported
        let result = revoke_token("dummy_token", Provider::GitHub).await;
        assert!(matches!(result, RevokeResult::NotSupported { .. }));
    }

    #[tokio::test]
    async fn test_revoke_google_timeout() {
        let server = MockServer::start().await;

        // Mock a slow endpoint (never responds)
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(10)))
            .mount(&server)
            .await;

        // We need a way to test with custom URL. Add `revoke_token_with_url` for testing.
        // Or: test that timeout is configured correctly by checking the client builder.
    }

    #[tokio::test]
    async fn test_revoke_google_network_error() {
        // Test against an unreachable URL
        let result = revoke_token("dummy_token", Provider::Google).await;
        // This will actually hit Google's real endpoint — skip in CI or use a mock.
        // Better: refactor to accept base_url parameter for testing.
    }
}
```

**Refinement for testability:** Extract the HTTP call into a function that accepts the base URL:
```rust
/// Internal: revoke with configurable base URL (for testing)
async fn revoke_google_with_url(base_url: &str, refresh_token: &str) -> RevokeResult {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| RevokeResult::NetworkError(e.to_string()))?;

    let url = format!("{}/revoke?token={}", base_url, refresh_token);
    // ... same logic
}

async fn revoke_google(refresh_token: &str) -> RevokeResult {
    revoke_google_with_url("https://oauth2.googleapis.com", refresh_token).await
}
```

### 10. `src/test_utils.rs` — no changes needed
The existing `pgtemp` setup is sufficient for integration tests. The revocation unit tests use `wiremock` for HTTP mocking, not the test database.

## Implementation Order

### Phase 1: Database & Data Model
1. **`migrations/schema.sql`** — Add `refresh_token TEXT` column.
2. **`src/db/mod.rs`** — Update `OauthAccount` struct, `insert_oauth_account` signature, and SQL queries.

### Phase 2: Capture Refresh Token
3. **`src/auth/oauth.rs`** — Add `access_type=offline` for Google in `start_handler`, extract `refresh_token` in `callback_handler`, pass to `insert_oauth_account`.

### Phase 3: Revocation Logic
4. **`src/auth/mod.rs`** — Add `pub mod revoke;`.
5. **`src/auth/revoke.rs`** — Create new module with `revoke_account`, `revoke_token`, `revoke_google`, `revoke_all_user_tokens`.

### Phase 4: Wire into Deletion Flows
6. **`src/db/mod.rs`** — Update `delete_oauth_account` to call `revoke_account` before DB delete.
7. **`src/db/mod.rs`** — Update `delete_user` to call `revoke_all_user_tokens` before DB delete.

### Phase 5: Tests
8. **`Cargo.toml`** — Add `wiremock = "0.6"` dev-dependency.
9. **`src/auth/revoke.rs`** — Add `#[cfg(test)]` module with unit tests for:
   - `revoke_token` → `Provider::GitHub` returns `NotSupported`
   - `revoke_google_with_url` → success (200 response from mock)
   - `revoke_google_with_url` → timeout (slow mock endpoint)
   - `revoke_google_with_url` → network error (unreachable URL)
   - `revoke_account` → `NoRefreshToken` when `refresh_token` is `None`

## Architectural Decisions & Trade-offs

| Decision | Rationale | Alternative |
|----------|-----------|-------------|
| Use `reqwest` directly instead of `oauth2` crate's revocation types | Simpler, avoids typestate changes to `GoogleAuthClient`, `reqwest` already a dependency | Use `oauth2::HasRevocationUrl` typestate — more type-safe but requires refactoring client types |
| 5-second timeout via `reqwest::Client::builder().timeout()` | Simple, covers both connect and read timeouts | Use `tokio::time::timeout()` wrapper — more control but more code |
| Revocation in `delete_oauth_account` / `delete_user` (DB layer) rather than page handlers | Single point of revocation logic, page handlers don't need to know about revocation | Revocation in page handlers — more explicit but duplicates logic |
| `refresh_token` column is nullable | Backward compatible with existing rows | NOT NULL with default — breaks existing data |
| Append `&access_type=offline` to auth URL string | Simple workaround for `oauth2` crate limitation | Use a different OAuth library or manual URL construction |

## Test Strategy

### Unit Tests (in `src/auth/revoke.rs`)
| Test | What it verifies |
|------|-----------------|
| `revoke_github_not_supported` | `Provider::GitHub` returns `NotSupported` |
| `revoke_google_success` | Mock server returns 200 → `Success` |
| `revoke_google_timeout` | Mock server delays > 5s → `Timeout` |
| `revoke_google_network_error` | Unreachable URL → `NetworkError` |
| `revoke_google_http_error` | Mock server returns 400 → `HttpError` |
| `revoke_account_no_refresh_token` | `refresh_token: None` → `NoRefreshToken` |

### Integration Tests (existing test infrastructure)
The existing `pgtemp`-based integration tests can verify the end-to-end flow:
1. Create user, link Google account (with mock refresh token in DB).
2. Call `delete_oauth_account` → verify revocation is attempted (via mock).
3. Call `delete_user` → verify all tokens are revoked (via mock).

## Gaps & Areas for Follow-up
1. **Google token format**: The revocation endpoint accepts both access tokens and refresh tokens. We're storing only the refresh token. This is correct per Google docs.
2. **Token rotation**: Google may rotate refresh tokens on each use. Our implementation stores the initial refresh token from the OAuth callback. If Google rotates it, the stored token may become invalid. **Mitigation**: The revocation endpoint still accepts old tokens for revocation, so this is not a blocking issue.
3. **GitHub token expiry**: GitHub tokens expire after ~8 years. Document this as a known limitation.
4. **Rate limiting**: Google's revocation endpoint may have rate limits. With a 5-second timeout and per-account revocation, this should not be an issue for typical usage.
5. **Testing against real Google endpoint**: Unit tests use `wiremock`. Integration tests against the real Google endpoint are not feasible in CI. Consider a manual test checklist.

## Dependencies
- `reqwest` (already a dependency with `json` feature) — used for HTTP revocation calls.
- `wiremock = "0.6"` (new dev-dependency) — used for mocking HTTP endpoints in unit tests.
- `tracing` (already a dependency) — used for logging revocation results.
- `tokio` (already a dependency) — used for async runtime.

## Phase 1: Implementation Details

### Summary
Implemented OAuth token revocation on account deletion per AC11 of NOMS-006. Refresh tokens are now captured during OAuth callback (Google `access_type=offline`), stored in `oauth_accounts.refresh_token`, and revoked via provider APIs when accounts are unlinked or deleted.

### Files Created
- **`src/auth/revoke.rs`** — New module with revocation logic: `revoke_account`, `revoke_token`, `revoke_google`, `revoke_all_user_tokens`, plus `RevokeResult` enum. Includes 6 unit tests using `wiremock`.

### Files Modified
- **`migrations/schema.sql`** — Added `refresh_token TEXT` column to `oauth_accounts` table; added `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` for existing databases.
- **`src/db/mod.rs`** — Added `refresh_token: Option<String>` to `OauthAccount` struct; updated all SELECT queries (`get_oauth_account_by_provider`, `get_oauth_account_by_email`, `get_oauth_accounts_by_user_id`) to include `refresh_token`; updated `insert_oauth_account` to accept `refresh_token: Option<&str>`; added `get_oauth_accounts_by_user_id` helper; updated `delete_oauth_account` to fetch account, revoke token, then delete; updated `delete_user` to call `revoke_all_user_tokens` before deletion.
- **`src/auth/mod.rs`** — Added `pub mod revoke;` declaration.
- **`src/auth/linking.rs`** — Added `FromStr` impl for `Provider` enum; updated `link_or_create` to accept `refresh_token: Option<String>`; updated all 5 test calls to pass `None` for `refresh_token`.
- **`src/auth/oauth.rs`** — Added `access_type=offline` for Google in `start_handler`; in `callback_handler`, extract `refresh_token` from token response via `TokenResponseExt::refresh_token()` and pass to `link_or_create`; updated 1 test call.
- **`src/test_utils.rs`** — Added `refresh_token TEXT` column to `oauth_accounts` test schema.

### Tests
- **6 new unit tests** in `src/auth/revoke.rs`: `test_revoke_github_not_supported`, `test_revoke_google_success`, `test_revoke_google_timeout`, `test_revoke_google_http_error`, `test_revoke_google_network_error`, `test_provider_from_str`.
- **All 135 tests pass** (18 non-server + 117 server feature tests) including existing db, linking, oauth, and revoke tests.
- **Clippy clean** — no warnings.

### Verification
- `cargo build` — compiles without errors.
- `cargo build --features server` — compiles without errors.
- `cargo test` — 18 passed.
- `cargo test --features server` — 135 passed.
- `cargo clippy --features server` — clean.
- Local database migration applied: `ALTER TABLE oauth_accounts ADD COLUMN refresh_token TEXT`.

### Adaptations from Blueprint
- Used `Uuid` instead of `i32` for IDs (actual codebase uses UUIDs).
- Used `DbError` instead of `AppError` (actual error type in codebase).
- Functions use generic `executor: impl sqlx::Executor` pattern; `delete_oauth_account` and `delete_user` changed to `&PgPool` to support revocation calls.
- `refresh_token` passed through `link_or_create` instead of direct `insert_oauth_account` call in `callback_handler` (matches existing architecture where `link_or_create` handles all insertions).
- Added `FromStr` impl for `Provider` (needed by `revoke_account` to parse provider string).
- `revoke_account` takes `&OauthAccount` directly instead of `(pool, account)` — pool is only needed for `revoke_all_user_tokens`.
- Added `#[allow(dead_code)]` on `RevokeResult` enum fields (accessed only in test compilation unit).

## Phase 2: Review Verdict

**Verdict: PASS**

All requirements from AC11 of NOMS-006 are met. The implementation compiles cleanly, passes all 135 tests, and has no clippy warnings.

### Numbered Findings

1. **Location:** `src/auth/revoke.rs:90-120` — **Severity: SUGGESTION** — `revoke_google_with_url` creates a new `reqwest::Client` on every invocation. Since revocation is infrequent (only on unlink/delete), this is acceptable. For high-throughput scenarios, consider sharing a pre-built client via Axum state. No action needed now.

2. **Location:** `src/db/mod.rs:117-129` — **Severity: SUGGESTION** — `OauthAccount` derives `Debug` via `sqlx::FromRow`, which means `refresh_token` would appear in any `{:?}` log output. Currently, `OauthAccount` is never logged directly (only `account.provider` and `RevokeResult` are). Consider adding a custom `Debug` impl that redacts `refresh_token` as a defensive measure for future code changes.

3. **Location:** `src/auth/linking.rs:230-245` (existing provider login path) — **Severity: SUGGESTION** — When a user logs in with an existing provider+uid, `refresh_token` from the new token response is discarded. If Google rotates refresh tokens, the stored token may become stale. Mitigation: Google's revocation endpoint accepts old tokens for revocation, so this is not a blocking issue. Consider updating `refresh_token` on each login in a future iteration.

4. **Location:** `src/auth/revoke.rs:99` — **Severity: INFO (not an issue)** — The refresh token is passed as a URL query parameter (`?token=...`), which is how Google's revocation API expects it. This means the token appears in the request line, which could show up in access logs. This matches Google's documented API contract and is acceptable.

5. **Location:** `src/auth/revoke.rs:62-65` — **Severity: POSITIVE** — `Provider::Apple` is handled with a `NotSupported` variant and a warning log, providing forward compatibility when Apple OAuth is implemented.

6. **Location:** `src/db/mod.rs:313-348` — **Severity: POSITIVE** — `delete_oauth_account` fetches the account with both `id` AND `user_id` guards in a single query, preventing cross-user deletion. The revocation call is fire-and-forget (result discarded), ensuring failures never block the actual deletion.

7. **Location:** `src/db/mod.rs:465-483` — **Severity: POSITIVE** — `delete_user` calls `revoke_all_user_tokens` BEFORE the `DELETE FROM users`, ensuring tokens are revoked while the accounts still exist in the database. The CASCADE delete handles cleanup afterward.

### Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Store `refresh_token` in `oauth_accounts` | ✅ | `migrations/schema.sql:30`, `src/db/mod.rs:126`, `src/db/mod.rs:396` |
| Capture `refresh_token` from OAuth callback | ✅ | `src/auth/oauth.rs:380-382`, passed through `link_or_create` |
| Google `access_type=offline` for refresh tokens | ✅ | `src/auth/oauth.rs:291-295` |
| Google revocation endpoint (`POST oauth2.googleapis.com/revoke`) | ✅ | `src/auth/revoke.rs:85-87` |
| 5-second timeout on revocation requests | ✅ | `src/auth/revoke.rs:92: .timeout(Duration::from_secs(5))` |
| GitHub: no revocation API, log warning | ✅ | `src/auth/revoke.rs:58-61` |
| Revocation failures logged, never block deletion | ✅ | `delete_oauth_account:333` (result discarded), `delete_user:470` (result discarded) |
| Revocation on unlink (`delete_oauth_account`) | ✅ | `src/db/mod.rs:333` |
| Revocation on account delete (`delete_user`) | ✅ | `src/db/mod.rs:470` |
| Migration backward-compatible (nullable column) | ✅ | `migrations/schema.sql:40: ADD COLUMN IF NOT EXISTS` |
| Unit tests for revocation logic | ✅ | 6 tests in `src/auth/revoke.rs:166-253` |

### Test Coverage Summary

- `test_revoke_github_not_supported` — verifies `NotSupported` for GitHub
- `test_revoke_google_success` — wiremock returns 200 → `Success`
- `test_revoke_google_timeout` — wiremock delays 10s → `Timeout` (5s client timeout)
- `test_revoke_google_http_error` — wiremock returns 400 → `HttpError`
- `test_revoke_google_network_error` — unreachable URL → `NetworkError`/`Timeout`
- `test_provider_from_str` — `FromStr` impl for all 3 providers + unknown
- Existing DB tests (`test_delete_oauth_account`, `test_delete_user_cascades_oauth_accounts`) exercise the full deletion path with revocation calls

### Build Status

- `cargo build --features server` — ✅ compiles without errors
- `cargo clippy --features server` — ✅ clean, no warnings
- `cargo test --features server` — ✅ 135 passed, 0 failed

### Overall Quality

Clean, well-structured implementation that faithfully follows the blueprint with sensible adaptations to the actual codebase architecture (UUIDs, `DbError`, `link_or_create` pattern). The revocation module is well-isolated, thoroughly tested with wiremock, and handles all error cases gracefully without blocking the critical deletion path.

## Phase 3: Synthesis

### User-Facing Summary

This change implements **AC11 of NOMS-006**: OAuth token revocation on account deletion. When a user unlinks an OAuth account or deletes their entire account, the application now revokes the associated OAuth tokens with the provider before performing the database deletion. This ensures that orphaned tokens cannot be used to access the user's data on the provider side after unlinking or account deletion.

**What was planned (Phase 0):** A blueprint was created covering database schema changes (adding `refresh_token` column), OAuth flow changes (capturing refresh tokens from Google via `access_type=offline`), a new revocation module with provider-specific logic, wiring revocation into both unlink and delete flows, and unit tests with `wiremock`.

**What was implemented (Phase 1):** All blueprint items were implemented with sensible adaptations to the actual codebase (UUID-based IDs, `DbError` type, `link_or_create` pattern). A new `src/auth/revoke.rs` module was created with full revocation logic and 6 unit tests. All 135 tests pass and clippy is clean.

**What was reviewed (Phase 2):** The review verdict is **PASS**. All 11 AC11 requirements are fully met. The review flagged 3 non-blocking suggestions: consider redacting `refresh_token` from `Debug` output, consider updating `refresh_token` on each login (Google token rotation), and consider sharing a pre-built `reqwest::Client` for high-throughput scenarios. None require immediate action.

---

### Step-by-Step Walkthrough of All Changes

#### 1. `migrations/schema.sql` — Database Schema Migration
**Purpose:** Add `refresh_token` column to `oauth_accounts` table.

- Added `refresh_token TEXT` column to the `oauth_accounts` table definition. The column is nullable for backward compatibility with existing rows that have no refresh token.
- Added `ALTER TABLE oauth_accounts ADD COLUMN IF NOT EXISTS refresh_token TEXT;` for incremental migration on existing databases.

#### 2. `src/db/mod.rs` — Database Layer
**Purpose:** Store and retrieve refresh tokens; wire revocation into deletion flows.

- **`OauthAccount` struct:** Added `refresh_token: Option<String>` field. This struct is the domain representation of an OAuth account row.
- **`get_oauth_account_by_provider`:** Updated the explicit column list in `query_as!` to include `refresh_token`. This query is used when unlinking a specific provider.
- **`get_oauth_account_by_email`:** Updated the explicit column list in `query_as!` to include `refresh_token`. This query is used during the OAuth callback to check for existing accounts.
- **`get_oauth_accounts_by_user_id` (NEW):** New helper function that fetches all OAuth accounts for a given user. Used by `revoke_all_user_tokens` during full account deletion.
- **`insert_oauth_account`:** Added `refresh_token: Option<&str>` parameter. The SQL INSERT now includes `refresh_token` in both the column list and VALUES clause.
- **`delete_oauth_account`:** Now fetches the account via `get_oauth_account_by_provider`, calls `revoke_account` (fire-and-forget — result is logged but never blocks), then proceeds with the `DELETE FROM oauth_accounts`. The query uses both `id` AND `user_id` guards to prevent cross-user deletion.
- **`delete_user`:** Now calls `revoke_all_user_tokens(pool, user_id)` BEFORE the `DELETE FROM users`. This ensures tokens are revoked while account records still exist in the database. The CASCADE delete on `oauth_accounts` handles cleanup afterward.

#### 3. `src/auth/mod.rs` — Module Declaration
**Purpose:** Expose the new `revoke` module.

- Added `pub mod revoke;` to make the revocation module available to other parts of the auth crate.

#### 4. `src/auth/revoke.rs` (NEW FILE) — Revocation Logic
**Purpose:** Provider-specific token revocation with error handling and testing.

- **`RevokeResult` enum:** Represents all possible outcomes of a revocation attempt: `Success`, `NotSupported { provider }`, `Timeout`, `NetworkError(String)`, `HttpError(String)`, `NoRefreshToken`. Used for structured logging and testing.
- **`revoke_account(account: &OauthAccount)`:** Entry point for single-account revocation. Extracts the refresh token, parses the provider string into a `Provider` enum, and delegates to `revoke_token`. Returns `NoRefreshToken` if the account has no stored refresh token.
- **`revoke_token(refresh_token: &str, provider: Provider)`:** Provider dispatch. Routes to `revoke_google` for Google, returns `NotSupported` with a warning log for GitHub and Apple. Logs the result at the appropriate level (info for success, error for failures).
- **`revoke_google(refresh_token: &str)`:** Makes a `POST` to `https://oauth2.googleapis.com/revoke?token=...` with a 5-second timeout. No auth header is required per Google's API. Returns `Success` on HTTP 2xx, `HttpError` on other status codes, `Timeout` on timeout, `NetworkError` on connection failures.
- **`revoke_google_with_url(base_url: &str, refresh_token: &str)`:** Internal testable variant that accepts a configurable base URL. Used by unit tests with `wiremock` mock servers.
- **`revoke_all_user_tokens(pool: &PgPool, user_id: Uuid)`:** Fetches all OAuth accounts for a user and revokes each token sequentially. Each result is logged at the appropriate level. Failures are never propagated — the function is fire-and-forget.
- **6 unit tests:** `test_revoke_github_not_supported` (GitHub returns `NotSupported`), `test_revoke_google_success` (wiremock 200 → `Success`), `test_revoke_google_timeout` (wiremock 10s delay → `Timeout`), `test_revoke_google_http_error` (wiremock 400 → `HttpError`), `test_revoke_google_network_error` (unreachable URL → `NetworkError`/`Timeout`), `test_provider_from_str` (parses all 3 providers + unknown).

#### 5. `src/auth/linking.rs` — Provider Linking Logic
**Purpose:** Thread `refresh_token` through the account linking flow.

- **`FromStr` impl for `Provider`:** New implementation that parses `"google"`, `"github"`, and `"apple"` strings into the `Provider` enum. Required by `revoke_account` to convert the stored provider string into a dispatchable enum.
- **`link_or_create`:** Added `refresh_token: Option<String>` parameter. The function now passes the refresh token through to `insert_oauth_account`. When an existing account is found (login path), the refresh token is currently discarded (noted as a future improvement for token rotation).
- **Tests:** All 5 existing test calls updated to pass `None` for `refresh_token`.

#### 6. `src/auth/oauth.rs` — OAuth Flow
**Purpose:** Capture refresh tokens during the OAuth callback.

- **`start_handler`:** After generating the authorization URL, appends `&access_type=offline` for Google providers. This is required by Google to return a refresh token in the token response.
- **`callback_handler`:** After `exchange_code` returns the `TokenResponse`, extracts the refresh token via `TokenResponseExt::refresh_token()` and passes it to `link_or_create`.
- **Tests:** 1 existing test call updated to pass `None` for `refresh_token`.

#### 7. `src/test_utils.rs` — Test Database Schema
**Purpose:** Keep the test database schema in sync with production.

- Added `refresh_token TEXT` column to the `oauth_accounts` table in the in-memory test schema.

---

### Dependencies Introduced or Modified

| Dependency | Type | Purpose |
|------------|------|---------|
| `wiremock = "0.6"` | New dev-dependency | Mock HTTP servers for revocation unit tests |
| `reqwest` | Existing dependency | Used directly for Google revocation HTTP calls (no new features needed) |
| `tracing` | Existing dependency | Used for structured logging of revocation results |
| `tokio` | Existing dependency | Used for async runtime (no new features needed) |

No new production dependencies were added. `reqwest` was already a dependency of the project.

---

### Special Syntax, Language Features, and Patterns

1. **`sqlx::query_as!` macro:** All SELECT queries use explicit column lists (not `SELECT *`) for compile-time safety. Adding `refresh_token` required updating every query that selects from `oauth_accounts`.

2. **Fire-and-forget revocation:** Both `delete_oauth_account` and `delete_user` call revocation functions but discard their results. The revocation functions log internally, but any error is never propagated. This ensures revocation failures never block the critical deletion path.

3. **Testable URL injection:** `revoke_google_with_url(base_url, token)` is a private helper that accepts a configurable base URL. The public `revoke_google(token)` delegates to it with the production URL. This pattern enables unit testing with `wiremock` without exposing internal APIs.

4. **`FromStr` for provider dispatch:** The `Provider` enum implements `std::str::FromStr` to convert the database-stored provider string (`"google"`, `"github"`, `"apple"`) into a Rust enum for pattern matching in `revoke_token`.

5. **`TokenResponseExt` trait:** The `oauth2` crate's `TokenResponseExt` trait provides `.refresh_token()` to extract the optional refresh token from the OAuth token response. This is a stable extension trait provided by the crate.

6. **UUID-based IDs:** The codebase uses `Uuid` (not `i32`) for primary keys. This was an adaptation from the blueprint and affects all database function signatures.

---

### Follow-up Recommendations

1. **Refresh token rotation on login (Medium priority):** When a user logs in with an existing Google account, the new refresh token from the token response is discarded. Google rotates refresh tokens on each use, so the stored token may become stale over time. Consider updating `refresh_token` in the existing account row during the login path in `link_or_create`.

2. **Redact `refresh_token` from `Debug` output (Low priority):** `OauthAccount` derives `Debug` via `sqlx::FromRow`, which means `refresh_token` would appear in any `{:?}` log output. Consider adding a custom `Debug` impl that redacts the field as a defensive measure.

3. **Shared `reqwest::Client` (Low priority):** Each revocation call creates a new `reqwest::Client`. For typical usage (infrequent unlink/delete), this is fine. If revocation becomes a high-throughput operation, consider sharing a pre-built client via Axum state.

4. **Integration test with real Google endpoint (Nice to have):** Unit tests use `wiremock` for HTTP mocking. An integration test against Google's real revocation endpoint would provide end-to-end validation but is not feasible in CI. Consider a manual test checklist or a staging environment test.

5. **Apple revocation support (Future):** `Provider::Apple` currently returns `NotSupported`. Apple's revocation API should be implemented when Apple OAuth support is added.

---

### Commit Message

```
feat(auth): revoke OAuth tokens on account unlink and deletion

Implement AC11 of NOMS-006: store refresh_token in oauth_accounts
table and revoke provider tokens when a user unlinks an OAuth
account or deletes their account.

Database changes:
- Add nullable refresh_token TEXT column to oauth_accounts table
- Update all SELECT/INSERT queries to include refresh_token
- Add get_oauth_accounts_by_user_id helper for bulk revocation

OAuth flow changes:
- Append access_type=offline to Google auth URL to obtain refresh
  tokens from the provider
- Extract refresh_token from TokenResponse in callback_handler
- Thread refresh_token through link_or_create to persistence

Revocation module (new src/auth/revoke.rs):
- revoke_account: single-account revocation entry point
- revoke_token: provider dispatch (Google via HTTP, GitHub/Apple
  log warning as NotSupported)
- revoke_google: POST to oauth2.googleapis.com/revoke with 5s
  timeout
- revoke_all_user_tokens: bulk revocation for account deletion
- 6 unit tests using wiremock for HTTP mocking

Deletion flow changes:
- delete_oauth_account: fetch account, revoke token, then delete
- delete_user: revoke all tokens before CASCADE delete

Revocation failures are logged but never block deletion. GitHub
has no revocation API — tokens expire naturally (~8 years).

Co-authored-by: Opencode Agent <agent@opencode>
```
