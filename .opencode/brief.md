# Task Brief

## Phase 0: Clarified Requirements
<!-- written by @refine -->

## Phase 1: Research Findings

### Key Findings

The project has a well-structured foundation for Checkpoint 4 (Account Linking): the DB schema (`users` + `oauth_accounts` tables) already exists in `migrations/schema.sql`, the `src/db/mod.rs` module already has all the required query functions (`insert_user`, `insert_oauth_account`, `get_oauth_account_by_provider`, `get_oauth_account_by_email`, `get_user_by_id`, `update_oauth_last_used`), and the test pattern using `pgtemp` for ephemeral PostgreSQL is already established. What's needed is a new `src/auth/linking.rs` module that orchestrates these DB calls within a transaction, plus username generation logic. The `unicode-normalization` crate is available as a transitive dependency but must be added explicitly to `Cargo.toml` if used.

### 1. Project Structure

```
src/
├── auth/
│   ├── mod.rs          # #![cfg(feature = "server")], #![allow(dead_code)], pub mod session
│   └── session.rs      # JWT session management (complete, tested)
├── components/
│   ├── mod.rs
│   ├── app_layout.rs
│   ├── error_fallback.rs
│   ├── footer.rs
│   ├── navbar.rs
│   └── base/           # Avatar, Button, Card, EmptyState, Input, LoadingSpinner, PageHeader
├── db/
│   └── mod.rs          # SQLx pool, types (User, OauthAccount, AuthState), queries, tests
├── pages/
│   ├── mod.rs
│   ├── home.rs, login.rs, dashboard.rs, explore.rs, recipe_*.rs, collection_*.rs
│   └── settings/       # settings_accounts.rs (placeholder), settings_profile.rs
├── utils/
│   ├── mod.rs          # pub mod theme
│   └── theme.rs
└── main.rs             # Dioxus app entry, db::create_pool() on server start
```

### 2. Database Schema (migrations/schema.sql)

**`users` table** (lines 10-19):
- `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
- `username VARCHAR(30) UNIQUE NOT NULL`
- `display_name VARCHAR(100) NOT NULL`
- `email VARCHAR(255) UNIQUE NOT NULL`
- `avatar_url TEXT`
- `bio TEXT`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`

**`oauth_accounts` table** (lines 22-34):
- `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
- `user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE`
- `provider VARCHAR(20) NOT NULL CHECK (provider IN ('google', 'apple', 'github'))`
- `provider_user_id VARCHAR(255) NOT NULL`
- `email VARCHAR(255)`
- `email_verified BOOLEAN NOT NULL DEFAULT FALSE`
- `profile_data JSONB`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
- `UNIQUE(provider, provider_user_id)`

**Indexes**: `idx_oauth_accounts_email ON oauth_accounts(email)`, `idx_oauth_accounts_user_id ON oauth_accounts(user_id)`

**`auth_states` table** (lines 42-47): CSRF nonce storage with `id VARCHAR(64) PK`, `redirect_uri TEXT NOT NULL`, `created_at TIMESTAMPTZ`.

### 3. sqlx Configuration & Usage

- **Runtime**: `runtime-tokio-rustls` feature (async with tokio runtime)
- **Connection pool type**: `PgPool` (from `sqlx::PgPool`)
- **Pool creation**: `db::create_pool()` in `src/db/mod.rs:49` reads `DATABASE_URL` env var
- **Query pattern**: All query functions accept `impl sqlx::Executor<'_, Database = Postgres>` — this allows passing either `&PgPool` or `&mut sqlx::Transaction<'_, Postgres>` as executor
- **`query_as!` macro**: Used for typed queries (e.g., `get_oauth_account_by_provider`, `get_oauth_account_by_email`, `insert_user`, `insert_oauth_account`, `get_user_by_id`)
- **`.sqlx/` offline mode**: 6 compiled query JSONs exist; `SQLX_OFFLINE=true` is used for `cargo check`/`cargo clippy`/`cargo test`
- **Transaction pattern**: Not yet used anywhere in the codebase. New code in `linking.rs` will be the first to use `sqlx::Transaction<'_, Postgres>`

### 4. Auth Module Code

**`src/auth/mod.rs`** (6 lines):
```rust
#![cfg(feature = "server")]
#![allow(dead_code)]
pub mod session;
```
- Gated behind `server` feature flag
- Has `#![allow(dead_code)]` (same as db/mod.rs) — currently allows unused code
- New `pub mod linking;` will be added here

**`src/auth/session.rs`** (367 lines): Complete JWT session management with:
- `SessionClaims { sub: Uuid, exp: usize, iat: usize }`
- `SessionError` enum: `MissingSecret`, `InvalidToken`, `Expired`
- `create_session(user_id: Uuid) -> Result<String, SessionError>`
- `verify_session(token: &str) -> Result<Uuid, SessionError>`
- `build_session_cookie(token: &str) -> Cookie<'static>`
- `clear_session_cookie() -> Cookie<'static>`
- `should_refresh(token: &str) -> Result<bool, SessionError>`
- Thread-local `TEST_SECRET` override for unit tests
- Comprehensive unit tests (10 test cases)

### 5. DB Module Code

**`src/db/mod.rs`** (524 lines): Complete DB layer with:

**Error type** (lines 14-41):
```rust
pub enum DbError {
    MissingUrl,           // DATABASE_URL not set
    Connection(sqlx::Error),  // Pool connection failed
    Query(sqlx::Error),       // Query execution failed
}
```
Implements `Display` and `std::error::Error` (with `source()`).

**Rust model types** (lines 57-89):
- `User` — `#[derive(Debug, Clone, sqlx::FromRow)]`, fields: `id, username, display_name, email, avatar_url, bio, created_at, updated_at`
- `OauthAccount` — `#[derive(Debug, Clone, sqlx::FromRow)]`, fields: `id, user_id, provider, provider_user_id, email, email_verified, profile_data, created_at, last_used_at`
- `AuthState` — `#[derive(Debug, Clone, sqlx::FromRow)]`, fields: `id, redirect_uri, created_at`

**Query functions** (all accept `impl sqlx::Executor<'_, Database = Postgres>`):
- `insert_auth_state(executor, id, redirect_uri)` → `Result<(), DbError>`
- `get_auth_state(executor, id)` → `Result<Option<AuthState>, DbError>`
- `delete_auth_state(executor, id)` → `Result<bool, DbError>`
- `get_oauth_account_by_provider(executor, provider, provider_user_id)` → `Result<Option<OauthAccount>, DbError>`
- `get_oauth_account_by_email(executor, email)` → `Result<Option<OauthAccount>, DbError>`
- `update_oauth_last_used(executor, id)` → `Result<(), DbError>`
- `insert_user(executor, username, display_name, email, avatar_url)` → `Result<User, DbError>`
- `insert_oauth_account(executor, user_id, provider, provider_user_id, email, profile_data)` → `Result<OauthAccount, DbError>`
- `get_user_by_id(executor, id)` → `Result<Option<User>, DbError>`

**Missing query**: No `get_user_by_email()` function exists yet, but `get_oauth_account_by_email()` can be used to find a user ID from an email via the oauth_accounts table.

### 6. Test Structure

- **Test framework**: `#[cfg(test)] mod tests` inside each module
- **DB test pattern** (in `src/db/mod.rs:261-367`):
  - Uses `pgtemp::PgTempDB` (dev-dependency in `Cargo.toml`) to spawn an ephemeral PostgreSQL per test
  - `setup_test_db()` creates a fresh temp DB, connects `PgPool`, and runs `apply_test_schema()` which creates all tables from scratch (raw SQL, not `query_as!`)
  - `apply_test_schema(pool)` creates extensions + all 3 tables + indexes
  - `uid()` helper generates unique 8-char suffixes for test data
  - All DB tests are `#[tokio::test]`
  - Tests are in the same file as the code (`src/db/mod.rs`)
- **Session tests**: Pure unit tests in `src/auth/session.rs` using thread-local secret override

### 7. Cargo.toml Dependencies (server feature)

Key server-side crates:
- `sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "json"], optional = true }`
- `uuid = { version = "1", features = ["v4", "serde"], optional = true }`
- `serde = { version = "1", features = ["derive"], optional = true }`
- `serde_json = { version = "1", optional = true }`
- `chrono = { version = "0.4", features = ["serde"], optional = true }`
- `oauth2`, `jsonwebtoken`, `axum-extra` (with `cookie`), `reqwest` (with `json`), `cookie`, `time`, `tokio`

Dev-dependencies:
- `pgtemp = "0.7"`

**NOT in Cargo.toml yet**: `unicode-normalization` — available as transitive dependency in `Cargo.lock` but must be added explicitly to `Cargo.toml` under `[dependencies]` with `optional = true` and added to the `server` feature if the implementer wants to use it for username normalization.

Lint config:
- `[lints.rust] warnings = "deny"`
- `[lints.clippy] all = "warn"`

### 8. Existing Models/Types Related to Users and OAuth

All in `src/db/mod.rs`:
- `User` struct (line 57-67): maps 1:1 to `users` table
- `OauthAccount` struct (line 70-81): maps 1:1 to `oauth_accounts` table, includes `provider: String` (not an enum)
- `AuthState` struct (line 84-89): maps 1:1 to `auth_states` table
- `DbError` enum (line 14-22): `MissingUrl`, `Connection(sqlx::Error)`, `Query(sqlx::Error)`

### 9. Error Handling Patterns

Two separate error types exist:
- `src/db/mod.rs`: `DbError` — `MissingUrl`, `Connection(sqlx::Error)`, `Query(sqlx::Error)` — implements `Display` + `std::error::Error`
- `src/auth/session.rs`: `SessionError` — `MissingSecret`, `InvalidToken`, `Expired` — implements `Display` + `std::error::Error`

For `linking.rs`, a new error type will be needed. Based on the existing patterns, it should likely wrap `DbError` while also adding linking-specific variants (e.g., `UsernameCollision`).

### 10. Dead Code Allowances & Clippy Config

- `src/auth/mod.rs`: `#![allow(dead_code)]` — allows unused code within the auth module during development
- `src/db/mod.rs`: `#![allow(dead_code)]` — same, with TODO comment: "Remove after checkpoint 4 wires up callers (account linking, OAuth handlers)"
- `src/components/base/avatar.rs` and `button.rs`: Per-item `#[allow(dead_code)]` on unused enum variants
- `clippy.toml`: Only configures `await-holding-invalid-types` for Dioxus signal types
- Global lint: `warnings = "deny"` at crate level, `clippy::all = "warn"`

### Implementation Notes for Checkpoint 4

1. **File to create**: `src/auth/linking.rs` — the `pub mod linking;` declaration must be added to `src/auth/mod.rs`
2. **Transaction requirement**: The account linking logic must run in a single `sqlx::Transaction<'_, Postgres>` to be atomic. All existing db functions accept `impl sqlx::Executor`, which works with both `&PgPool` and `&mut Transaction`. The linking function should:
   - Acquire a transaction from the pool (`pool.begin().await`)
   - Pass `&mut tx` to each db function
   - Commit on success (`tx.commit().await`) or rollback on error (drop tx)
3. **Username generation**: The schema requires `username VARCHAR(30) UNIQUE NOT NULL`. The plan says: derive from display_name, lowercase, strip non-alphanumeric (non-ASCII → NFKD decompose + strip combining marks), hyphenate words, truncate to 22 chars, append `-` + 4-char hex suffix (from `Uuid::new_v4()`). Max 3 collision retries, then fallback `user-{8hex}`.
4. **`insert_user` function signature**: `insert_user(executor, username, display_name, email, avatar_url)` — note it requires `display_name` and `email` as separate params.
5. **`unicode-normalization` crate**: Not yet in `Cargo.toml`. Must be added as an optional server dependency for NFKD normalization, then added to the `server` feature list.
6. **`provider` column**: `VARCHAR(20)` with `CHECK (provider IN ('google', 'apple', 'github'))`. Should be represented as a Rust enum in `linking.rs` (or `db/mod.rs`).
7. **No `get_user_by_email` query exists**: The linking logic uses `get_oauth_account_by_email()` to find an existing user by email, then `get_user_by_id()` to get user details. Alternatively, a new query could be added.
8. **`.sqlx/` offline data**: New `query_as!` calls in `linking.rs` that reference existing DB functions won't need new `.sqlx` entries since the queries are already in `db/mod.rs`. But if new queries are added, `cargo sqlx prepare` must be run with a live database to update offline metadata.
9. **Test pattern to follow**: Create `setup_test_db()` in `linking.rs` tests (or import from `db`), use `#[tokio::test]`, use `uid()` for unique test data.

## Phase 2: Implementation Blueprint
<!-- written by @architect -->

## Phase 3: Implementation Details

### Summary

Implemented three follow-up changes to the Checkpoint 4 (Account Linking) implementation:

1. **Follow-up 3 (shared test infrastructure)**: Extracted `setup_test_db()`, `apply_test_schema()`, and `uid()` from `src/db/mod.rs` and `src/auth/linking.rs` into a new `src/test_utils.rs` module. Both test modules now use `crate::test_utils` instead of local duplicates. The `apply_test_schema()` in `test_utils.rs` uses the comprehensive version from `db/mod.rs` (including optional extensions and `auth_states` table).

2. **Follow-up 2 (fallback uniqueness check)**: Added `UsernameGenerationFailed` variant to `LinkError` enum with `Display` and `Error` impls. Replaced the direct `user-{8hex}` fallback return in `generate_unique_username()` with a 3-attempt loop that checks uniqueness against the database. Returns `LinkError::UsernameGenerationFailed` if all 3 fallback attempts collide.

3. **Follow-up 1 (`email: None` test)**: Added `test_brand_new_user_without_email` integration test that creates an `OauthUserInfo` with `email: None`, verifies the placeholder email pattern (`noreply+{uuid}@placeholder`), validates username generation from display name, and confirms the OAuth account email is `None`. Also added `test_username_fallback_with_many_collisions` integration test creating 10 users with the same display name to exercise collision handling extensively.

All 42 tests pass (10 unit + 6 integration for linking, 6 db tests, 10 session tests, 10 avatar tests), clippy is clean, and compilation succeeds.

### New Files

- **`src/test_utils.rs`** (~133 lines): Shared test infrastructure module gated with `#[cfg(all(feature = "server", test))]`. Contains:
  - `pub async fn setup_test_db() -> (pgtemp::PgTempDB, PgPool)` — spawns ephemeral PostgreSQL and applies schema
  - `pub async fn apply_test_schema(pool: &PgPool)` — creates all tables, indexes, and extensions (comprehensive version from `db/mod.rs` including `auth_states`)
  - `pub fn uid() -> String` — generates unique 8-char suffixes for test data

### Modified Files

- **`src/main.rs`**: Added `#[cfg(all(feature = "server", test))] mod test_utils;` declaration.
- **`src/auth/linking.rs`**:
  - Added `UsernameGenerationFailed` variant to `LinkError` enum
  - Updated `Display` impl with `"failed to generate a unique username after all attempts"` message
  - Updated `Error` impl with `None` source for `UsernameGenerationFailed`
  - Replaced direct `user-{8hex}` fallback return with 3-attempt loop checking uniqueness, returning `LinkError::UsernameGenerationFailed` on exhaustion
  - Removed local `setup_test_db()`, `apply_test_schema()`, `uid()` functions and `use sqlx::PgPool;` import
  - Added `use crate::test_utils;` in test module
  - Replaced all `setup_test_db()` / `uid()` calls with `test_utils::setup_test_db()` / `test_utils::uid()`
  - Added `test_brand_new_user_without_email` integration test (follow-up 1)
  - Added `test_username_fallback_with_many_collisions` integration test (follow-up 2)
- **`src/db/mod.rs`**:
  - Removed local `setup_test_db()`, `apply_test_schema()`, `uid()` functions and `use pgtemp::PgTempDB;` import
  - Added `use crate::test_utils;` in test module
  - Replaced all `setup_test_db()` / `uid()` calls with `test_utils::setup_test_db()` / `test_utils::uid()`

### Tests

- 10 unit tests for `generate_username_from_display_name` (unchanged)
- 6 integration tests for linking flows:
  - `test_existing_provider_login` — existing OAuth account returns same user, `is_new_user=false`
  - `test_new_provider_same_email` — new provider with matching email links to existing user
  - `test_brand_new_user` — creates new user with generated username from `Some(email)`
  - `test_username_collision_retry` — three users with same display name get unique usernames
  - `test_brand_new_user_without_email` — **NEW** — creates user with `email: None`, verifies placeholder email pattern and OAuth email is None
  - `test_username_fallback_with_many_collisions` — **NEW** — 10 users with same display name, all get unique usernames
- 6 db integration tests (using `test_utils::setup_test_db`)
- 10 session unit tests, 10 avatar tests (unchanged)

### Verification

- `cargo check --features server` ✅ (compiles cleanly)
- `cargo clippy --features server -- -D warnings` ✅ (no warnings)
- `cargo test --features server` ✅ (42/42 tests pass, including all integration tests via pgtemp)


## Phase 4: Review Verdict

### Verdict: PASS

All three follow-up changes are correctly implemented, fully tested, and introduce no regressions. Clippy is clean, 42 tests pass, and the code is well-structured.

### Issues

1. **SUGGESTION** — `src/auth/linking.rs:154` — Main retry loop still uses `0..5` iterations: The original Phase 0 spec stated "Max 3 collision retries," and the Phase 4 review called this out. The follow-up implementation added the fallback uniqueness loop (which is excellent), but didn't adjust the main loop from `0..5` to match the spec's intended count. This is not a blocker — 5 attempts is more robust — but the deviation from the Phase 0 spec remains. Consider aligning to `0..4` (1 initial + 3 retries) for spec compliance, or explicitly documenting the deviation.

2. **SUGGESTION** — `src/auth/linking.rs:127` — Username base truncation at 24 vs spec's 22: Carried over from the original implementation. With the `-{4hex}` suffix, max username length is 29 chars, which fits `VARCHAR(30)`. Not a practical issue, just a spec alignment note.

### Positive Findings

- **Test infrastructure properly extracted**: `src/test_utils.rs` is a clean, well-documented module gated with `#[cfg(all(feature = "server", test))]` in `main.rs`. It contains all three helpers (`setup_test_db`, `apply_test_schema`, `uid`) with the comprehensive `apply_test_schema` version (including `auth_states` table and optional extensions). Both `db/mod.rs` and `linking.rs` now use `crate::test_utils` with zero remaining duplication. Verified by searching for local `fn setup_test_db` / `fn apply_test_schema` / `fn uid` — none found in either file.

- **`UsernameGenerationFailed` error variant well-designed**: The new `LinkError::UsernameGenerationFailed` variant follows the established error type patterns in the codebase (matching `DbError` and `SessionError` conventions). `Display` message is clear: "failed to generate a unique username after all attempts". `Error::source()` correctly returns `None`. The `From<DbError> for LinkError` impl means `?` propagation still works seamlessly.

- **Fallback uniqueness loop is defensive and correct**: The `for _ in 0..3` loop at lines 164-169 properly checks uniqueness for each `user-{8hex}` candidate against the database, returning `Err(LinkError::UsernameGenerationFailed)` on exhaustion. This eliminates the theoretical (astronomically unlikely) possibility of a username collision on the fallback path.

- **`test_brand_new_user_without_email` thoroughly validates the `email: None` path**: The test creates `OauthUserInfo` with `email: None`, verifies `is_new_user` is true, asserts the placeholder email starts with `noreply+` and ends with `@placeholder`, checks the username derivation from `display_name`, and crucially confirms that the OAuth account's `email` field is stored as `None` (not the placeholder). This covers all the important aspects of the `email: None` branch.

- **`test_username_fallback_with_many_collisions` exercises collision handling extensively**: Creating 10 users with the same display name stresses both the main retry loop and the fallback path. The assertion that usernames start with either `"collision-test-"` or `"user-"` correctly accounts for both paths. The uniqueness verification (HashSet size == 10) is a good final check.

- **No stale imports or dead code removed**: The refactoring from local test helpers to `crate::test_utils` was clean — no leftover `use pgtemp::PgTempDB` or `use sqlx::PgPool` imports in the test modules. All calls properly prefixed with `test_utils::`.

### Requirements Coverage (Follow-Up Tasks)

| Follow-Up Requirement | Status |
|---|---|
| Shared test infrastructure extracted to `src/test_utils.rs` | ✅ All 3 helpers extracted, comprehensive schema used |
| `mod test_utils` gated with `#[cfg(all(feature = "server", test))]` | ✅ Correct in `main.rs` line 8-9 |
| No remaining test helper duplication in `db/mod.rs` or `linking.rs` | ✅ Verified — zero local definitions remain |
| `UsernameGenerationFailed` variant with Display + Error impls | ✅ Lines 65, 72-73, 83 |
| Fallback loop checks uniqueness 3 times | ✅ `for _ in 0..3` with DB uniqueness check |
| `LinkError::UsernameGenerationFailed` returned on exhaustion | ✅ Line 170 |
| `email: None` test (`test_brand_new_user_without_email`) | ✅ Lines 473-521, verifies placeholder email, username, OAuth email=None |
| `test_username_fallback_with_many_collisions` added | ✅ Lines 523-564, 10 users with same display name |
| Clippy clean (`cargo clippy --features server -- -D warnings`) | ✅ Passes with zero warnings |
| 42 tests passing | ✅ All pass |
| No regressions in existing tests | ✅ All prior tests still pass |

### Summary

All three follow-up changes are cleanly implemented and thoroughly verified. The shared test infrastructure in `src/test_utils.rs` eliminates all duplication while preserving the comprehensive schema (including `auth_states` and optional extensions). The fallback uniqueness loop with `UsernameGenerationFailed` error is a significant robustness improvement over the previous unverified fallback. The two new integration tests provide meaningful coverage of the `email: None` path and collision-heavy scenarios. The two remaining spec deviations (retry count 5 vs 3, truncation 24 vs 22) are unchanged from the original implementation and remain as minor suggestions for future alignment.

## Phase 5: Synthesis
<!-- written by @synthesize -->

## Follow-Up Task: Implement Review Suggestions 1, 2, and 3

### Context
The Phase 4 review of Checkpoint 4 (Account Linking) passed with three actionable suggestions (suggestion 4 about spec alignment is explicitly excluded by the user):

1. **Add test for `email: None` placeholder path** — The `OauthUserInfo` struct has an optional `email` field. When `None`, a `noreply+{uuid}@placeholder` email is generated. This branch is currently untested and needs an integration test.

2. **Add uniqueness check on `user-{8hex}` fallback username** — The `generate_unique_username()` function falls back to `user-{8hex}` when all retry attempts are exhausted, but doesn't verify this fallback is unique. A defensive existence check should be added.

3. **Extract shared test infrastructure** — Both `src/db/mod.rs` and `src/auth/linking.rs` define their own `setup_test_db()`, `apply_test_schema()`, and `uid()` helper functions. These should be extracted into a common test module to reduce duplication.

### Follow-Up Research: Detailed Implementation Audit for Review Suggestions 1, 2, and 3

#### 1. `generate_unique_username()` — Fallback Logic After Retries Exhausted

**File**: `src/auth/linking.rs`, lines 131–160

```rust
// Lines 131-160
async fn generate_unique_username(
    tx: &mut Transaction<'_, Postgres>,
    display_name: &str,
) -> Result<String, LinkError> {
    let base = {
        let b = generate_username_from_display_name(display_name);
        if b.is_empty() {
            "user".to_string()
        } else {
            b
        }
    };

    for _ in 0..5 {
        let suffix = &Uuid::new_v4().to_string()[..4];
        let candidate = format!("{base}-{suffix}");

        if !db::get_user_by_username(tx.deref_mut(), &candidate).await? {
            return Ok(candidate);
        }
    }

    // Fallback: user-{8hex}
    let fallback = format!("user-{}", &Uuid::new_v4().to_string()[..8]);
    Ok(fallback)
}
```

**Key observation**: After the retry loop (5 attempts), the fallback on **line 158** generates `user-{8hex}` and returns it **without checking uniqueness** against the DB. This is the exact spot where a `get_user_by_username` check should be added. The simplest fix is to wrap the fallback in a loop that checks uniqueness (with a small retry count, e.g., 3).

**The `db::get_user_by_username` function** (lines 258–271 of `src/db/mod.rs`) returns `Result<bool, DbError>` where `true` means "username exists" (i.e., is taken). The call pattern in `generate_unique_username` correctly uses `!db::get_user_by_username(...)` to check availability.

**Implementation approach for suggestion #2**: Add a loop around the fallback that regenerates + checks uniqueness, something like:
```rust
// After the 0..5 retry loop:
for _ in 0..3 {
    let fallback = format!("user-{}", &Uuid::new_v4().to_string()[..8]);
    if !db::get_user_by_username(tx.deref_mut(), &fallback).await? {
        return Ok(fallback);
    }
}
// If all fallbacks collide (astronomically unlikely), return an error
return Err(LinkError::Db(db::DbError::Query(/* or a new LinkError variant */)));
```
A new `LinkError` variant (e.g., `UsernameGenerationFailed`) would need to be added to `LinkError` on lines 60-64, along with corresponding `Display` and `Error` impls.

#### 2. `OauthUserInfo` Struct — `email` Field Type and `None` Branch

**File**: `src/auth/linking.rs`, lines 43–49

```rust
pub struct OauthUserInfo {
    pub provider: Provider,
    pub provider_uid: String,
    pub email: Option<String>,   // <-- Option<String>
    pub display_name: String,
    pub avatar_url: Option<String>,
}
```

The `email` field is `Option<String>`. When `None`, `link_or_create()` handles it at **lines 213–220**:

```rust
// (c) Brand-new user
let username = generate_unique_username(&mut tx, &info.display_name).await?;
let placeholder_email;
let email = match &info.email {
    Some(e) => e.as_str(),
    None => {
        placeholder_email = format!("noreply+{}@placeholder", Uuid::new_v4());
        placeholder_email.as_str()
    }
};
```

This creates `noreply+{uuid}@placeholder` and uses it as the user's email. The `placeholder_email` variable is declared before the match to solve the lifetime issue (the `String` needs to live long enough). The placeholder then feeds into `insert_user` on line 222.

**No existing test exercises this branch** — all 4 integration tests pass `Some(email)`. A new test should construct `OauthUserInfo { email: None, ... }` and verify:
- The user's email starts with `noreply+`
- The user's email ends with `@placeholder`
- `is_new_user` is true
- The user can be retrieved by ID

#### 3. Test Infrastructure in `src/auth/linking.rs`

**File**: `src/auth/linking.rs`, lines 314–384 (inside `mod tests`)

Three helper functions:

**`setup_test_db()`** — lines 317–324:
```rust
async fn setup_test_db() -> (pgtemp::PgTempDB, PgPool) {
    let db = pgtemp::PgTempDB::async_new().await;
    let pool = PgPool::connect(&db.connection_uri())
        .await
        .expect("failed to connect to temp database");
    apply_test_schema(&pool).await;
    (db, pool)
}
```

**`apply_test_schema()`** — lines 327–379:
Creates `pgcrypto` extension + `users` table + `oauth_accounts` table + two indexes. Does **NOT** create the `auth_states` table (which linking tests don't need).

```rust
async fn apply_test_schema(pool: &PgPool) {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .execute(pool).await.expect("...");
    sqlx::query("CREATE TABLE IF NOT EXISTS users (...)").execute(pool).await.expect("...");
    sqlx::query("CREATE TABLE IF NOT EXISTS oauth_accounts (...)").execute(pool).await.expect("...");
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_accounts_email ON oauth_accounts(email)")
        .execute(pool).await.expect("...");
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user_id ON oauth_accounts(user_id)")
        .execute(pool).await.expect("...");
}
```

**`uid()`** — lines 382–384:
```rust
fn uid() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}
```

#### 4. Test Infrastructure in `src/db/mod.rs`

**File**: `src/db/mod.rs`, lines 291–402 (inside `mod tests`)

Three helper functions:

**`setup_test_db()`** — lines 297–304:
```rust
async fn setup_test_db() -> (PgTempDB, PgPool) {
    let db = PgTempDB::async_new().await;
    let pool = PgPool::connect(&db.connection_uri())
        .await
        .expect("failed to connect to temp database");
    apply_test_schema(&pool).await;
    (db, pool)
}
```

**`apply_test_schema()`** — lines 308–397:
More comprehensive than linking.rs — also creates `pg_cron`, `pg_trgm`, `vector`, `pg_search`, `timescaledb` extensions (silently ignoring failures), and the `auth_states` table + its index. This is the "canonical" schema creation function.

```rust
async fn apply_test_schema(pool: &PgPool) {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")...
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_cron")...  // optional
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")...   // optional
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")...    // optional
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search")...  // optional
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS timescaledb")... // optional
    sqlx::query("CREATE TABLE IF NOT EXISTS users (...)")...
    sqlx::query("CREATE TABLE IF NOT EXISTS oauth_accounts (...)")...
    sqlx::query("CREATE INDEX ...")...  // email, user_id indexes
    sqlx::query("CREATE TABLE IF NOT EXISTS auth_states (...)")...
    sqlx::query("CREATE INDEX ... auth_states(created_at)")...
}
```

**`uid()`** — lines 400–402:
```rust
fn uid() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}
```

**Key difference**: The `db/mod.rs` version creates optional extensions (`pg_cron`, `pg_trgm`, `vector`, `pg_search`, `timescaledb`) and the `auth_states` table. The `linking.rs` version only creates what it needs (`pgcrypto`, `users`, `oauth_accounts`, two indexes). Both use `pgtemp::PgTempDB` (import path differs: linking uses `pgtemp::PgTempDB`, db uses `PgTempDB` with explicit `use pgtemp::PgTempDB`).

#### 5. Shared Test Modules or Utility Files

**No shared test utility module exists in the project.** There is no `src/test_utils.rs`, `tests/` directory, or any other centralized test infrastructure. The project structure contains no `*test*` or `*util*` files outside the two `mod tests` blocks.

The four files with `mod tests` blocks are:
- `src/auth/linking.rs` (line 245) — integration + unit tests
- `src/db/mod.rs` (line 291) — integration tests only
- `src/auth/session.rs` (line 183) — pure unit tests (no DB, uses thread-local secret)
- `src/components/base/avatar.rs` (line 104) — frontend unit tests

Only `linking.rs` and `db/mod.rs` have the duplicated `setup_test_db()`, `apply_test_schema()`, and `uid()` functions.

#### 6. Integration Test Section in `src/auth/linking.rs`

**File**: `src/auth/linking.rs`, lines 312–531

The integration test section contains:
- Lines 314–315: `use sqlx::PgPool;` import
- Lines 317–324: `setup_test_db()` function
- Lines 327–379: `apply_test_schema()` function
- Lines 382–384: `uid()` helper
- Lines 386–431: `test_existing_provider_login` — tests path (a) existing provider login
- Lines 433–472: `test_new_provider_same_email` — tests path (b) email-based linking
- Lines 474–498: `test_brand_new_user` — tests path (c) brand new user with email
- Lines 500–530: `test_username_collision_retry` — tests username uniqueness on collision

**No test for `email: None` path exists.** All tests pass `email: Some(...)`.

#### 7. Summary of Duplicates Across Test Modules

| Helper Function | `src/db/mod.rs` | `src/auth/linking.rs` | Identical? |
|---|---|---|---|
| `setup_test_db()` | Lines 297–304 | Lines 317–324 | **Functionally identical** (different import style) |
| `apply_test_schema()` | Lines 308–397 | Lines 327–379 | **Different**: db version includes `auth_states` table + optional extensions |
| `uid()` | Lines 400–402 | Lines 382–384 | **Identical** (same logic) |

#### Actionable Implementation Details

**Suggestion 1 (Add `email: None` test)**: Add a new `#[tokio::test]` after line 530 in `src/auth/linking.rs`. The test should:
- Call `link_or_create` with `OauthUserInfo { email: None, ... }`
- Assert `result.is_new_user == true`
- Retrieve the user and verify `user.email.starts_with("noreply+")` and `user.email.ends_with("@placeholder")`

**Suggestion 2 (Add fallback uniqueness check)**: Modify `generate_unique_username()` at lines 148–159 in `src/auth/linking.rs`. After the `for _ in 0..5` loop, instead of returning the fallback directly, loop with uniqueness checks. Also add a new `LinkError` variant (e.g., `UsernameCollision`) to handle the (astronomically unlikely) case where even fallback attempts collide.

**Suggestion 3 (Extract shared test infrastructure)**: Create a new `src/db/test_helpers.rs` module (or `src/test_helpers.rs`) that contains:
- `pub async fn setup_test_db() -> (pgtemp::PgTempDB, PgPool)` — calls `apply_test_schema`
- `pub async fn apply_test_schema(pool: &PgPool)` — the comprehensive version from `db/mod.rs`
- `pub fn uid() -> String` — shared unique suffix generator
- Both `src/db/mod.rs` and `src/auth/linking.rs` would then import from this shared module instead of defining their own helpers.
- The module should be `#[cfg(test)]` gated or conditionally compiled.
