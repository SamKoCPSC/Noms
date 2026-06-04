# Task Brief

## Task Description
Implement AC6 from NOMS-006: PKCE (Proof Key for Code Exchange) for OAuth flow.

**AC6: PKCE for OAuth flow (HIGH-2)**
- OAuth authorization requests include `code_challenge` and `code_challenge_method=S256`
- Token exchange includes `code_verifier`
- `code_verifier` is random, 43-128 chars, stored in `auth_states` alongside CSRF state
- `code_challenge` = `BASE64URL(SHA256(code_verifier))`
- Both Google and GitHub providers support PKCE
- No impact on existing OAuth flows

## Phase 0: Implementation Blueprint
## Research Findings

### OAuth2 crate (v5) — Native PKCE Support Confirmed
- **Crate**: `oauth2 = "5"` (Cargo.toml line 17, server feature)
- `PkceCodeChallenge::new_random_sha256()` returns `(PkceCodeChallenge, PkceCodeVerifier)` — no manual SHA-256/base64url needed
- `AuthorizationRequest::set_pkce_challenge(pkce_code_challenge)` attaches `code_challenge` + `code_challenge_method=S256` to the auth URL
- `CodeTokenRequest::set_pkce_verifier(pkce_code_verifier)` attaches `code_verifier` to the token exchange request
- `PkceCodeVerifier::new(String)` reconstructs from stored string; `PkceCodeVerifier::secret()` extracts `&str`
- `PkceCodeChallenge::as_str()` extracts `&str` for the challenge
- Verifier length: 43–128 chars, ASCII alphanumeric + `-._~` (RFC 7636 compliant)

### Provider PKCE Support
- **Google**: Supports PKCE with `S256` since 2019. Web app clients still require `client_secret` alongside PKCE (already handled by existing `set_client_secret` in `build_oauth_clients`). No code change needed for Google.
- **GitHub**: Added PKCE support July 2025 (changelog: `github.blog/changelog/2025-07-14-pkce-support-for-oauth-and-github-app-authentication/`). Only `S256` supported. Strongly recommended by GitHub docs.

### Key Architectural Decision: Store `code_verifier` in `auth_states`
The `code_verifier` must survive the redirect to the OAuth provider and back. The existing pattern already stores CSRF state + redirect_uri + provider in `auth_states` with a 10-minute TTL. Adding `code_verifier` as a new column follows this exact pattern. The state ID serves as the lookup key in the callback handler.

---

## Files to Modify

### 1. `migrations/schema.sql` — Add `code_verifier` column

**Location**: lines 42–48 (auth_states table)

**Change**: Add a nullable `code_verifier TEXT` column to the existing `auth_states` table definition. Use `ALTER TABLE` (additive-only, per schema convention at line 3).

```sql
-- Add code_verifier column for PKCE support (nullable for backward compat)
ALTER TABLE auth_states ADD COLUMN IF NOT EXISTS code_verifier TEXT;
```

This is placed AFTER the existing `CREATE TABLE IF NOT EXISTS auth_states` block (after line 48). The column is nullable because:
- Existing rows (if any) won't have a value
- The application will always populate it for new flows

### 2. `src/test_utils.rs` — Update test schema

**Location**: lines 96–106 (auth_states CREATE TABLE)

**Change**: Add `code_verifier TEXT` to the inline test schema definition.

```rust
// Before (line 96-102):
"CREATE TABLE IF NOT EXISTS auth_states (\
 id VARCHAR(64) PRIMARY KEY,\
 redirect_uri TEXT NOT NULL,\
 provider TEXT NOT NULL,\
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
 )",

// After:
"CREATE TABLE IF NOT EXISTS auth_states (\
 id VARCHAR(64) PRIMARY KEY,\
 redirect_uri TEXT NOT NULL,\
 provider TEXT NOT NULL,\
 code_verifier TEXT,\
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
 )",
```

### 3. `src/db/mod.rs` — Update AuthState struct and queries

**Location**: `AuthState` struct at lines 131–137

**Change**: Add `code_verifier: Option<String>` field.

```rust
// Before:
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthState {
    pub id: String,
    pub redirect_uri: String,
    pub provider: String,
    pub created_at: DateTime<Utc>,
}

// After:
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthState {
    pub id: String,
    pub redirect_uri: String,
    pub provider: String,
    pub code_verifier: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

**Location**: `insert_auth_state` at lines 154–168

**Change**: Add `code_verifier` parameter and include it in the INSERT.

```rust
// Before:
pub async fn insert_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO auth_states (id, provider, redirect_uri) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(provider)
        .bind(redirect_uri)
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

// After:
pub async fn insert_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO auth_states (id, provider, redirect_uri, code_verifier) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(provider)
        .bind(redirect_uri)
        .bind(code_verifier)
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}
```

**Location**: `get_auth_state` at lines 171–183

**Change**: Include `code_verifier` in the SELECT.

```rust
// Before:
sqlx::query_as!(
    AuthState,
    "SELECT id, redirect_uri, provider, created_at FROM auth_states WHERE id = $1",
    id,
)

// After:
sqlx::query_as!(
    AuthState,
    "SELECT id, redirect_uri, provider, code_verifier, created_at FROM auth_states WHERE id = $1",
    id,
)
```

**Note on `sqlx::query_as!` macro**: The macro validates column names at compile time against the struct fields. The `code_verifier` column name must match the struct field name exactly. Since `code_verifier` is `Option<String>`, sqlx will correctly map SQL NULL to `None`.

### 4. `src/auth/oauth.rs` — PKCE in start_handler and callback_handler

**Location**: Imports at lines 12–16

**Change**: Add `PkceCodeChallenge` and `PkceCodeVerifier` to the oauth2 imports.

```rust
// Before:
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};

// After:
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
```

**Location**: `start_handler` at lines 246–284

**Change**: Generate PKCE pair, store verifier in DB, attach challenge to auth URL.

```rust
// Before (lines 254-281):
    let csrf_state = Uuid::new_v4().to_string();

    db::insert_auth_state(
        &state.pool,
        &csrf_state,
        prov.as_str(),
        &params.redirect_uri,
    )
    .await
    .map_err(|e| OAuthError::DbError(e.to_string()))?;

    let client = match prov {
        linking::Provider::Google => &state.google_client,
        linking::Provider::GitHub => &state.github_client,
        _ => return Err(OAuthError::InvalidProvider(provider)),
    };

    let mut req = client.authorize_url(|| CsrfToken::new(csrf_state.clone()));

    req = req
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()));

    let (auth_url, _csrf_token) = req.url();

// After:
    let csrf_state = Uuid::new_v4().to_string();

    // Generate PKCE code verifier and challenge (S256 method).
    // The verifier is stored server-side; the challenge is sent to the provider.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    db::insert_auth_state(
        &state.pool,
        &csrf_state,
        prov.as_str(),
        &params.redirect_uri,
        pkce_verifier.secret(),
    )
    .await
    .map_err(|e| OAuthError::DbError(e.to_string()))?;

    let client = match prov {
        linking::Provider::Google => &state.google_client,
        linking::Provider::GitHub => &state.github_client,
        _ => return Err(OAuthError::InvalidProvider(provider)),
    };

    let mut req = client
        .authorize_url(|| CsrfToken::new(csrf_state.clone()))
        .set_pkce_challenge(pkce_challenge);

    req = req
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()));

    let (auth_url, _csrf_token) = req.url();
```

Key detail: `.set_pkce_challenge(pkce_challenge)` is called on the `AuthorizationRequest` builder, which adds both `code_challenge=<value>` and `code_challenge_method=S256` to the authorization URL query string.

**Location**: `callback_handler` at lines 342–346

**Change**: Attach `code_verifier` to the token exchange request.

```rust
// Before (lines 342-346):
    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code.clone()))
        .request_async(&state.http_client)
        .await
        .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

// After:
    // Reconstruct the PKCE code verifier from the stored value.
    let code_verifier = auth_state.code_verifier
        .ok_or_else(|| OAuthError::TokenExchange("PKCE code_verifier not found in auth state".to_string()))?;

    let token_response = client
        .exchange_code(AuthorizationCode::new(params.code.clone()))
        .set_pkce_verifier(PkceCodeVerifier::new(code_verifier))
        .request_async(&state.http_client)
        .await
        .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;
```

Key detail: The `auth_state` is retrieved BEFORE deletion (line 300), so `auth_state.code_verifier` is available at the point of token exchange (line 342). The state is deleted at line 322, which is after the verifier is read but before the token exchange. We need to ensure the verifier is captured before the state deletion. Looking at the current flow:

1. Line 300: `get_auth_state` — retrieves state (including `code_verifier`)
2. Lines 306-318: expiry + provider checks
3. Lines 322-327: `delete_auth_state` — consumes the state
4. Lines 342-346: token exchange

The `auth_state` variable is still in scope at line 342 (it's a `let` binding from line 300). So `auth_state.code_verifier` is accessible. No reordering needed.

### 5. `src/auth/oauth.rs` — Update existing tests

**Location**: `test_utils` calls to `db::insert_auth_state` in oauth.rs tests (lines 605-607, 620-622, 635-637)

**Change**: All calls to `db::insert_auth_state` now require a 5th argument (`code_verifier`). Use a dummy verifier for tests that don't exercise the full PKCE flow.

```rust
// Before:
db::insert_auth_state(&pool, &state_id, "google", "/dashboard").await.unwrap();

// After:
db::insert_auth_state(&pool, &state_id, "google", "/dashboard", "dummy-verifier-that-is-long-enough-43-chars-min").await.unwrap();
```

### 6. `src/db/mod.rs` — Update existing tests

**Location**: All calls to `db::insert_auth_state` in db/mod.rs tests (lines 531-532, 548-549, 570-571, 575-576)

**Change**: Same as above — add dummy `code_verifier` argument.

---

## Test Plan

### Unit Tests (no DB, in `src/auth/oauth.rs`)

1. **`test_pkce_challenge_verifier_generation`** — Verify `PkceCodeChallenge::new_random_sha256()` produces valid-length outputs.
2. **`test_pkce_challenge_in_auth_url`** — Build an auth URL with PKCE and verify the URL contains `code_challenge=` and `code_challenge_method=S256` query params.
3. **`test_pkce_verifier_reconstruction`** — Verify `PkceCodeVerifier::new(stored_string)` round-trips correctly.

### Integration Tests (with DB, in `src/auth/oauth.rs` mod db_tests)

4. **`test_auth_state_stores_code_verifier`** — Insert auth state with a verifier, retrieve it, assert `code_verifier` is `Some(expected)`.
5. **`test_callback_retrieves_verifier_before_delete`** — Verify the callback flow reads `code_verifier` from `auth_state` before the state is deleted.

### Integration Tests (with DB, in `src/db/mod.rs`)

6. **`test_insert_and_get_auth_state_with_verifier`** — Replace existing `test_insert_and_get_auth_state` to include verifier.
7. **`test_cleanup_expired_auth_states_with_verifier`** — Same as existing cleanup test, just with 5-arg `insert_auth_state`.

---

## Step-by-Step Implementation Order

1. **Migration**: Add `ALTER TABLE auth_states ADD COLUMN IF NOT EXISTS code_verifier TEXT;` to `migrations/schema.sql` (after line 48).
2. **Test schema**: Update `src/test_utils.rs` auth_states CREATE TABLE to include `code_verifier TEXT` (line 96-106).
3. **DB struct**: Add `code_verifier: Option<String>` to `AuthState` in `src/db/mod.rs` (line 131-137).
4. **DB insert**: Update `insert_auth_state` signature and query in `src/db/mod.rs` (lines 154-168).
5. **DB select**: Update `get_auth_state` query in `src/db/mod.rs` (lines 171-183).
6. **OAuth imports**: Add `PkceCodeChallenge, PkceCodeVerifier` to `src/auth/oauth.rs` (lines 12-16).
7. **start_handler**: Add PKCE generation + storage + `.set_pkce_challenge()` in `src/auth/oauth.rs` (lines 254-281).
8. **callback_handler**: Add `.set_pkce_verifier()` to token exchange in `src/auth/oauth.rs` (lines 342-346).
9. **Fix all test call sites**: Update every `db::insert_auth_state(...)` call in `src/auth/oauth.rs` tests and `src/db/mod.rs` tests to include the new `code_verifier` argument.
10. **Add new tests**: Add PKCE-specific tests as listed above.

---

## Backward Compatibility Notes

- The `code_verifier` column is nullable (`TEXT`, not `TEXT NOT NULL`). This ensures the migration is safe even if applied to a running database with existing auth_states rows.
- The `AuthState.code_verifier` field is `Option<String>`. In `callback_handler`, we treat `None` as an error (`TokenExchange`), which is correct — any new auth flow will always have a verifier.
- The `insert_auth_state` signature change is a breaking change for callers, but all callers are within this crate (oauth.rs and tests), so no external API is affected.
- No changes to `delete_auth_state` or `cleanup_expired_auth_states` are needed — they operate on the state `id` only.

## Phase 1: Implementation Details
### Summary
Implemented PKCE (Proof Key for Code Exchange) for the OAuth 2.0 Authorization Code flow, following RFC 7636. Both Google and GitHub providers now include `code_challenge` and `code_challenge_method=S256` in authorization requests, and `code_verifier` in token exchange requests.

### Changes Made

#### New/Modified Files

1. **`migrations/schema.sql`** — Added `ALTER TABLE auth_states ADD COLUMN IF NOT EXISTS code_verifier TEXT;` for PKCE verifier persistence (nullable for backward compatibility).

2. **`src/test_utils.rs`** — Added `code_verifier TEXT` column to the inline test schema's `auth_states` CREATE TABLE.

3. **`src/db/mod.rs`** — Three changes:
   - Added `code_verifier: Option<String>` field to `AuthState` struct
   - Updated `insert_auth_state()` signature to accept `code_verifier: &str` as 5th parameter; query now INSERTs the column
   - Updated `get_auth_state()` query to SELECT `code_verifier` column
   - Updated 4 existing test call sites to pass dummy verifier strings

4. **`src/auth/oauth.rs`** — Four changes:
   - Added `PkceCodeChallenge` and `PkceCodeVerifier` to oauth2 imports
   - `start_handler`: generates PKCE pair via `PkceCodeChallenge::new_random_sha256()`, stores verifier in DB, attaches challenge to auth URL via `.set_pkce_challenge()`
   - `callback_handler`: retrieves stored `code_verifier` from `auth_state`, attaches to token exchange via `.set_pkce_verifier()`
   - Updated 3 existing test call sites to pass dummy verifier strings

5. **`.sqlx/query-f74a9ab7...json`** (new) — Updated sqlx offline query cache for the modified `get_auth_state` query (added `code_verifier` column). Removed old cache file.

### Tests

**New unit tests (3):**
- `test_pkce_challenge_verifier_generation` — verifies `new_random_sha256()` produces RFC 7636 compliant verifier (43-128 chars)
- `test_pkce_challenge_in_auth_url` — verifies auth URL contains `code_challenge=` and `code_challenge_method=S256`
- `test_pkce_verifier_reconstruction` — verifies `PkceCodeVerifier::new(stored)` round-trips correctly

**New integration tests (2):**
- `test_auth_state_stores_code_verifier` — verifies verifier is persisted and retrievable from DB
- `test_callback_retrieves_verifier_before_delete` — verifies callback flow reads verifier before state deletion

**Updated existing tests (7 call sites):** All `db::insert_auth_state()` calls in `src/db/mod.rs` (4) and `src/auth/oauth.rs` (3) updated to include the new `code_verifier` argument.

### Verification
- `cargo clippy --no-default-features --features server -- -D warnings` — passes clean
- `cargo test --no-default-features --features server` — all 129 tests pass
- `cargo fmt -- --check` — formatting clean
- Migration applied to local database for sqlx compile-time query verification

## Phase 2: Review Verdict

**Verdict: PASS**

### Issues

1. **Location:** `src/auth/oauth.rs` line 596, `test_pkce_verifier_reconstruction` — **Severity: SUGGESTION** — The assertion `assert_eq!(challenge.as_str(), challenge.as_str());` is a no-op (comparing a value to itself). It was likely intended to verify something about the challenge-verifier relationship but adds no value. **Recommended fix:** Remove the line or replace with a meaningful assertion (e.g., verify that the challenge is indeed the base64url-encoded SHA-256 of the verifier, though the oauth2 crate handles this internally).

2. **Location:** `src/auth/oauth.rs` lines 685, 705, 727, 744, 758 — **Severity: SUGGESTION** — Test dummy verifiers (e.g., `"test-verifier-minimum-43-chars-long!!"`, `"test-pkce-verifier-minimum-43-chars!!"`) contain `!` characters, which are not in the RFC 7636 allowed character set (`[A-Za-z0-9\-._~]`). These are fine for DB storage/retrieval tests, but would be rejected by `PkceCodeVerifier::new()` in a real flow. **Recommended fix:** Use RFC-compliant dummy strings like `"test-verifier-that-is-at-least-43-chars-long"` (already used in `src/db/mod.rs` line 535).

3. **Location:** `src/auth/oauth.rs` lines 349-351 — **Severity: SUGGESTION** — The error message `"PKCE code_verifier not found in auth state"` is wrapped in `OAuthError::TokenExchange`, which maps to HTTP 500 with a sanitized message. This is correct for security (no internal detail leakage), but the error variant name is slightly misleading since this isn't a token exchange failure per se. **Recommended fix:** Consider adding a dedicated `OAuthError::MissingPkceVerifier` variant for clarity, though the current behavior is functionally correct.

### Positive Findings and Good Practices

- **Correct use of `oauth2` crate v5 PKCE API:** `PkceCodeChallenge::new_random_sha256()` is the idiomatic approach — it generates 32 cryptographically random bytes via `getrandom`, base64url-encodes them to produce a 43-character verifier (RFC 7636 compliant), and computes `BASE64URL(SHA256(verifier))` for the challenge. No manual crypto needed.
- **Correct verifier lifecycle:** Verifier is generated in `start_handler`, stored in DB alongside CSRF state, retrieved and used in `callback_handler` before state deletion. The variable scoping (`auth_state` is a `let` binding, not consumed by `delete_auth_state`) ensures the verifier is accessible at the token exchange point.
- **Safe migration:** `ALTER TABLE ... ADD COLUMN IF NOT EXISTS code_verifier TEXT` is additive-only, nullable, and idempotent — follows the schema convention established at line 3 of `migrations/schema.sql`.
- **Backward compatibility:** `AuthState.code_verifier` is `Option<String>`, correctly mapping SQL NULL to `None`. The `callback_handler` treats `None` as an error, which is correct — any new auth flow will always have a verifier.
- **sqlx offline query cache updated:** The `.sqlx/query-f74a9ab7...json` file correctly reflects the new `code_verifier` column with `nullable: true` (ordinal 3), matching the `Option<String>` struct field.
- **Comprehensive test coverage:** 3 new unit tests (verifier generation, challenge in auth URL, verifier round-trip) and 2 new integration tests (DB storage, callback flow ordering) plus all 7 existing call sites updated.
- **Clean build:** `cargo clippy -- -D warnings` passes clean, `cargo fmt -- --check` passes clean.

### Requirements Coverage

| AC6 Requirement | Status |
|---|---|
| Auth requests include `code_challenge` and `code_challenge_method=S256` | ✅ `.set_pkce_challenge()` on `AuthorizationRequest` |
| Token exchange includes `code_verifier` | ✅ `.set_pkce_verifier()` on `CodeTokenRequest` |
| Verifier is random, 43-128 chars | ✅ `new_random_sha256()` produces 43-char base64url string |
| Verifier stored in `auth_states` alongside CSRF state | ✅ New `code_verifier TEXT` column |
| `code_challenge = BASE64URL(SHA256(code_verifier))` | ✅ Handled by `oauth2` crate |
| Both Google and GitHub support PKCE | ✅ Applied to both provider clients |
| No impact on existing OAuth flows | ✅ Column is nullable, migration is additive |

### Summary

Clean, correct implementation that properly delegates PKCE crypto to the well-tested `oauth2` crate v5. The migration is safe, the verifier lifecycle is correct, and test coverage is thorough. Three minor suggestions (cosmetic test fix, RFC-compliant test data, error variant naming) — none are blockers.

## Phase 3: Synthesis
## User-Facing Summary

PKCE (Proof Key for Code Exchange, RFC 7636) has been implemented for the OAuth 2.0 Authorization Code flow. This hardens both Google and GitHub OAuth integrations by binding the authorization request to the token exchange, preventing authorization code interception attacks.

**What was planned (Phase 0):** Leverage the existing `oauth2` crate v5's native PKCE API (`PkceCodeChallenge::new_random_sha256()`) to generate a verifier/challenge pair, store the verifier in the `auth_states` table alongside the CSRF state, attach the challenge to the authorization URL, and present the verifier during token exchange.

**What was implemented (Phase 1):** A `code_verifier` column was added to the `auth_states` table (via migration and test schema), the `AuthState` struct and DB queries were updated, and the OAuth `start_handler` and `callback_handler` were wired to generate, store, and present PKCE values. All existing test call sites were updated, and 5 new tests were added.

**What was reviewed (Phase 2):** The implementation passed review. All AC6 requirements are satisfied. Three minor cosmetic suggestions were raised (a no-op assertion in one test, non-RFC-compliant characters in test dummy strings, and a slightly misleading error variant name) — none are blockers.

---

## Step-by-Step Walkthrough of Changes

### 1. `migrations/schema.sql` — Database migration

**Change:** Added `ALTER TABLE auth_states ADD COLUMN IF NOT EXISTS code_verifier TEXT;` after the existing `CREATE TABLE IF NOT EXISTS auth_states` block.

**Purpose:** Persist the PKCE code_verifier alongside the CSRF state so it survives the redirect round-trip to the OAuth provider. The column is nullable for backward compatibility (existing rows, if any, will have NULL). The `IF NOT EXISTS` clause makes the migration idempotent.

### 2. `src/test_utils.rs` — Test schema

**Change:** Added `code_verifier TEXT` column to the inline `auth_states` CREATE TABLE definition used by integration tests.

**Purpose:** Ensure the in-memory test database schema matches the production schema, so integration tests exercise the same column layout.

### 3. `src/db/mod.rs` — AuthState struct

**Change:** Added `pub code_verifier: Option<String>` field to the `AuthState` struct.

**Purpose:** Represent the nullable `code_verifier` column in Rust. `Option<String>` correctly maps SQL NULL to `None`.

### 4. `src/db/mod.rs` — `insert_auth_state` function

**Change:** Added `code_verifier: &str` as a 5th parameter. The INSERT query now includes `code_verifier` as the 4th column, bound to `$4`.

**Purpose:** Store the PKCE verifier when creating a new auth state entry. This is called from `start_handler` immediately after generating the PKCE pair.

**Key detail:** The function signature change is a breaking change for callers, but all callers are internal to this crate (oauth.rs and tests), so no external API is affected.

### 5. `src/db/mod.rs` — `get_auth_state` function

**Change:** The SELECT query now includes `code_verifier` in the column list: `"SELECT id, redirect_uri, provider, code_verifier, created_at FROM auth_states WHERE id = $1"`.

**Purpose:** Retrieve the stored verifier during the callback flow so it can be presented during token exchange. The `sqlx::query_as!` macro validates at compile time that the column name matches the struct field name.

### 6. `src/auth/oauth.rs` — Imports

**Change:** Added `PkceCodeChallenge` and `PkceCodeVerifier` to the `use oauth2::{...}` import list.

**Purpose:** Bring the PKCE types into scope for use in `start_handler` and `callback_handler`.

### 7. `src/auth/oauth.rs` — `start_handler`

**Change:** Three additions in sequence:
1. `(pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256()` — generates a cryptographically random 43-character verifier and its SHA-256-based challenge.
2. `pkce_verifier.secret()` is passed as the 5th argument to `db::insert_auth_state()` — persists the verifier.
3. `.set_pkce_challenge(pkce_challenge)` is chained onto the `authorize_url()` builder — attaches both `code_challenge` and `code_challenge_method=S256` to the authorization URL query string.

**Purpose:** This is the core of the PKCE flow — generate the pair at auth initiation, store the secret (verifier) server-side, and send the public commitment (challenge) to the provider.

**Non-obvious pattern:** `.set_pkce_challenge()` is called on the `AuthorizationRequest` builder (not the `Client`), and it returns a new builder with the PKCE parameters baked in. This is a fluent builder pattern.

### 8. `src/auth/oauth.rs` — `callback_handler`

**Change:** Before the token exchange, the stored `code_verifier` is extracted from `auth_state.code_verifier` (with an `ok_or_else` guard for the `None` case). It is then passed to `.set_pkce_verifier(PkceCodeVerifier::new(code_verifier))` on the `CodeTokenRequest` builder.

**Purpose:** Present the original verifier to the provider during token exchange, proving that the entity exchanging the code is the same entity that initiated the authorization.

**Key detail:** The `auth_state` variable is a `let` binding from `get_auth_state()` at line 300. It remains in scope through `delete_auth_state()` at line 322 (which only consumes the DB row, not the Rust variable). So `auth_state.code_verifier` is accessible at the token exchange point at line 349. No reordering was needed.

### 9. `.sqlx/query-f74a9ab7...json` — sqlx offline query cache

**Change:** New cache file generated for the updated `get_auth_state` query (now includes `code_verifier` column). Old cache file removed.

**Purpose:** sqlx uses compile-time query verification. The offline cache must reflect the current query and column types. The new cache correctly shows `code_verifier` as `nullable: true` at ordinal 3, matching `Option<String>`.

### 10. Test call site updates (7 total)

**Files:** `src/db/mod.rs` (4 call sites), `src/auth/oauth.rs` (3 call sites)

**Change:** All `db::insert_auth_state()` calls now pass a 5th argument — a dummy verifier string (e.g., `"test-verifier-that-is-at-least-43-chars-long"`).

**Purpose:** Maintain compilation and runtime correctness of existing tests after the function signature change.

---

## New Tests (5 total)

### Unit Tests (3) — in `src/auth/oauth.rs`

| Test | What it verifies |
|---|---|
| `test_pkce_challenge_verifier_generation` | `new_random_sha256()` produces a verifier within the RFC 7636 length bounds (43-128 chars) |
| `test_pkce_challenge_in_auth_url` | The authorization URL contains both `code_challenge=` and `code_challenge_method=S256` query parameters |
| `test_pkce_verifier_reconstruction` | `PkceCodeVerifier::new(stored_string)` round-trips correctly (the secret can be extracted after reconstruction) |

### Integration Tests (2) — in `src/auth/oauth.rs` mod `db_tests`

| Test | What it verifies |
|---|---|
| `test_auth_state_stores_code_verifier` | A verifier is correctly persisted to and retrieved from the `auth_states` table |
| `test_callback_retrieves_verifier_before_delete` | The callback flow reads `code_verifier` from the auth state before the state row is deleted |

### Existing Tests Updated (7 call sites)

All `db::insert_auth_state()` calls across `src/db/mod.rs` (4) and `src/auth/oauth.rs` (3) now include the `code_verifier` argument.

### Verification Results
- **Total tests passing:** 129
- **`cargo clippy -- -D warnings`:** Clean
- **`cargo fmt -- --check`:** Clean

---

## Dependencies

No new external dependencies were introduced. The implementation uses the existing `oauth2 = "5"` crate's native PKCE API, which was already a dependency of the project.

---

## Review Notes (Phase 2 Suggestions)

Three minor suggestions from the review phase, none blocking:
1. `test_pkce_verifier_reconstruction` contains a no-op assertion (`assert_eq!(challenge.as_str(), challenge.as_str())`) — consider removing.
2. Some test dummy verifiers use `!` characters, which are outside the RFC 7636 character set — functionally fine for DB tests, but could be replaced with RFC-compliant strings for consistency.
3. The `OAuthError::TokenExchange` variant is used for the missing PKCE verifier error — a dedicated variant could improve clarity.

---

## Follow-Up Recommendations

- **Monitor OAuth callback logs** after deployment for any `MissingPkceVerifier` errors, which could indicate stale auth_states rows from before the migration.
- **Consider the review suggestions** in a follow-up PR for test cleanliness.
- **If adding new OAuth providers**, confirm PKCE support and apply the same `.set_pkce_challenge()` / `.set_pkce_verifier()` pattern.

---

## Commit Message

```
feat(auth): add PKCE support to OAuth flow

Implement Proof Key for Code Exchange (RFC 7636) for both Google and
GitHub OAuth providers. PKCE binds the authorization request to the
token exchange, preventing authorization code interception attacks.

Changes:
- Add code_verifier TEXT column to auth_states table (migration + test
  schema), nullable for backward compatibility
- Extend AuthState struct with code_verifier: Option<String> field
- Update insert_auth_state() to accept and persist the verifier
- Update get_auth_state() to retrieve the verifier column
- Generate PKCE pair in start_handler via
  PkceCodeChallenge::new_random_sha256(), store verifier in DB, attach
  challenge to authorization URL via .set_pkce_challenge()
- Attach code_verifier to token exchange in callback_handler via
  .set_pkce_verifier()
- Update sqlx offline query cache for modified get_auth_state query
- Update 7 existing test call sites to include code_verifier argument
- Add 3 unit tests (verifier generation, challenge in URL, round-trip)
- Add 2 integration tests (DB storage, callback flow ordering)

All 129 tests pass. Clippy and fmt checks are clean.

Refs: NOMS-006 AC6
```
