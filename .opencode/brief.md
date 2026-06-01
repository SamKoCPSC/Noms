# Task Brief

## Task Description
Implement NOMS-005 AC1 + AC2 (User Profile & Account Management):

**AC1: Profile data loads on every authenticated request**
- Fetch the full user profile in the auth middleware alongside `AuthUser`
- Inject profile into request extensions
- Update `build_context_from_fullstack()` to read the profile and populate `current_user: Option<UserProfile>` in AuthContext
- Navbar should display the user's real display name (currently shows "User" placeholder)

**AC2: Profile settings page is functional**
- Settings profile page (`/settings/profile`) displays current display name and bio
- User can edit display name and bio via form inputs
- Save button persists changes via a server function
- Optimistic UI updates with rollback on failure
- Display name: 2-30 chars, trimmed
- Bio: max 160 chars, trimmed

**Constraints**
- All code must pass `cargo fmt`, `cargo clippy -D warnings` (wasm32 + x86_64), and full test suite
- Profile fetch must remain synchronous in middleware (no async server function) — reuse the DB pool
- Keep the existing `AuthUser` extension; add a new extension for the profile or embed profile data into AuthUser

## Phase 0: Implementation Blueprint
## Phase 0: Implementation Blueprint

### Research Findings

**Architecture**: Dioxus 0.7.1 fullstack (SSR + hydration), PostgreSQL via `sqlx::PgPool`, session cookies (JWT) verified in Axum middleware → injected into request extensions → read by `build_context_from_fullstack()` → provided as Dioxus context.

**Pool lifecycle**: Created in `main()` on dedicated thread, stored in `auth::oauth::AppState`, currently only used by OAuth router handlers.

**Dioxus 0.7 server function patterns** (verified via official docs):
- Server function state access: `FullstackContext::extract().await?` with `Extension<T>` extractor
- Router Extension layer: `.layer(Extension(pool))` makes pool available to all handlers
- Middleware with state: `axum::middleware::from_fn_with_state(pool)(handler)` passes pool as `State` extractor

### Key Gaps Identified

| Gap | Location | Detail |
|-----|----------|--------|
| `UserProfile` missing `bio` | `src/auth/context.rs:29-34` | DB schema has `bio TEXT` but struct doesn't |
| No pool access in middleware | `src/middleware/auth.rs` | Cannot fetch profile without DB connection |
| Navbar shows username | `src/components/navbar.rs:21` | Reads `u.username` instead of `u.display_name` |
| Settings profile placeholder | `src/pages/settings/settings_profile.rs` | No state, validation, or save logic |
| `current_user: None` | `src/auth/context.rs:79` | TODO comment, never populated |
| No profile update query | `src/db/mod.rs` | Missing `update_user_profile` function |

### Files to Modify (7 files)

#### 1. `src/auth/context.rs` — Add bio + new profile extension + wire context

**Line 29-34**: Add `pub bio: Option<String>` to `UserProfile` after `avatar_url`

**After line 21**: New struct:
```rust
#[derive(Debug, Clone)]
pub struct AuthUserProfile {
    pub profile: UserProfile,
}
```

**Lines 66-82**: Rewrite `build_context_from_fullstack()`:
```rust
let Some(auth_user) = fsc.extension::<AuthUser>() else {
    return AuthContext::default();
};
let Some(profile_ext) = fsc.extension::<AuthUserProfile>() else {
    return AuthContext {
        current_user_id: Some(auth_user.user_id),
        current_user: None,
        is_authenticated: true,
    };
};
AuthContext {
    current_user_id: Some(auth_user.user_id),
    current_user: Some(profile_ext.profile),
    is_authenticated: true,
}
```

#### 2. `src/middleware/auth.rs` — Fetch profile in middleware

**New imports**: `sqlx::PgPool`, `crate::db`, `crate::auth::context::AuthUserProfile`, plus `axum::extract::State`

**Line 46**: Change signature:
```rust
pub async fn handle_auth(
    State(pool): State<PgPool>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
```

**Lines 78-80**: Replace with profile fetch:
```rust
if let Some(user_id) = verified_user_id {
    req.extensions_mut().insert(AuthUser { user_id });
    if let Ok(Some(user)) = db::get_user_by_id(&pool, user_id).await {
        let profile = UserProfile {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            bio: user.bio,
        };
        req.extensions_mut().insert(AuthUserProfile { profile });
    }
}
```

#### 3. `src/db/mod.rs` — Add profile update function

**After line 309** (after `get_user_by_email`):
```rust
pub async fn update_user_profile(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    display_name: &str,
    bio: Option<&str>,
) -> Result<User, DbError> {
    sqlx::query_as!(
        User,
        "UPDATE users SET display_name = $2, bio = $3, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
        user_id,
        display_name,
        bio,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}
```

**Add test** in `mod tests`: `test_update_user_profile` — update display_name and bio, verify returned User

#### 4. `src/main.rs` — Wire pool into dioxus_router

**New import**: `use axum::Extension;`

**Lines 89-95**: Modify dioxus_router:
```rust
let dioxus_router = axum::Router::new()
    .layer(Extension(pool.clone()))
    .layer(axum::middleware::from_fn_with_state(pool.clone())(middleware::auth::handle_auth))
    .serve_dioxus_application(
        ServeConfig::new()
            .context_provider(auth::context::build_context_from_fullstack),
        App,
    );
```

#### 5. `src/components/navbar.rs` — Fix display name

**Line 21**: `u.username.clone()` → `u.display_name.clone()`

#### 6. `src/pages/settings/settings_profile.rs` — Functional profile page (complete rewrite)

**Signals**: `display_name`, `bio`, `is_saving`, `error`, `saved_message`

**On mount**: Load profile from `use_auth().current_user`

**Validation**: display_name 2-30 chars (trimmed), bio max 160 chars (trimmed)

**Server function** (defined in same file):
```rust
#[server]
pub async fn save_profile(
    display_name: String,
    bio: Option<String>,
) -> Result<UserProfile, ServerFnError> {
    use dioxus::fullstack::FullstackContext;
    use dioxus::server::axum::Extension;

    let Extension(AuthUser { user_id }): Extension<AuthUser> = 
        FullstackContext::extract().await?;
    let Extension(pool): Extension<PgPool> = 
        FullstackContext::extract().await?;

    let trimmed_name = display_name.trim().to_string();
    if trimmed_name.len() < 2 || trimmed_name.len() > 30 {
        return Err(ServerFnError::new("Display name must be 2-30 characters"));
    }
    let trimmed_bio = bio.map(|b| b.trim().to_string());
    if let Some(ref b) = trimmed_bio {
        if b.len() > 160 {
            return Err(ServerFnError::new("Bio must be 160 characters or less"));
        }
    }

    let updated = crate::db::update_user_profile(
        &pool, user_id, &trimmed_name, trimmed_bio.as_deref(),
    ).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(UserProfile {
        id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        avatar_url: updated.avatar_url,
        bio: updated.bio,
    })
}
```

**Optimistic UI pattern**: Save old profile → update signals immediately → call server function → on error rollback to old profile

**UI components**: Use existing `Input`, `Button`, `Card`, `PageHeader` from `src/components/base/`

#### 7. `src/components/base/input.rs` — Add `maxlength` prop (if needed)

If `Input` doesn't support `maxlength`, either add it to `InputProps` or use raw `<input>` for fields needing it.

### Implementation Order

1. `src/auth/context.rs` — Add `bio` to `UserProfile`, add `AuthUserProfile` struct
2. `src/db/mod.rs` — Add `update_user_profile` + test
3. `src/middleware/auth.rs` — Add `State(pool)` param, fetch profile, inject `AuthUserProfile`
4. `src/main.rs` — Add `Extension(pool)` layer + `from_fn_with_state(pool)` for middleware
5. `src/auth/context.rs` — Update `build_context_from_fullstack()` to read `AuthUserProfile`
6. `src/components/navbar.rs` — Fix `u.username` → `u.display_name`
7. `src/pages/settings/settings_profile.rs` — Full rewrite with signals, validation, server function, optimistic UI
8. **Verify**: `cargo fmt`, `cargo clippy -D warnings --all-targets`, `cargo test --features server`

### Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| Keep `AuthUser` separate from `AuthUserProfile` | Minimal change; `AuthUser` only carries `user_id` for JWT verification |
| Fetch profile in middleware (sync) vs server function (async) | Brief requires synchronous fetch; avoids extra round-trip per page load |
| Pool via `Extension` layer for server functions | Dioxus 0.7 recommended pattern per official docs |
| Pool via `from_fn_with_state` for middleware | Axum 0.8 pattern; cleaner than manual Extension extraction |
| Reuse `db::get_user_by_id` in middleware | Already returns `db::User` with all fields including `bio` |
| Server function returns `UserProfile` | Enables optimistic UI rollback |

### Test Plan

**Existing tests to verify**: All tests in `src/db/mod.rs`, `src/auth/session.rs`, `src/auth/oauth.rs`

**New tests**:
1. `test_update_user_profile` — update display_name and bio, verify returned User
2. `test_update_user_profile_nonexistent` — returns error for non-existent user ID
3. Server function validation: name too short, name too long, bio too long

### Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| `from_fn_with_state` conflicts with Dioxus state type | Fall back to `Extension` layer + manual extraction in middleware |
| `Extension&lt;PgPool&gt;` type conflict with Dioxus internals | Wrap in newtype: `struct DbPool(PgPool)` |
| sqlx compile-time queries need `DATABASE_URL` | Already `#[cfg(feature = "server")]`; use `sqlx::query_as!` offline mode or set env var |
| Middleware async DB call blocking | Middleware is already `async fn`; `.await` is fine |

## Phase 1: Implementation Details

### Summary of Changes
Implemented AC1 (profile data loads on every authenticated request) and AC2 (functional profile settings page) for NOMS-005. The auth middleware now fetches the full user profile from the database and injects it into request extensions, making it available to all Dioxus components via `AuthContext`. The settings profile page was completely rewritten with signals, validation, a server function, and optimistic UI with rollback.

### Files Modified

**`src/auth/context.rs`** — Added `bio: Option<String>` field to `UserProfile`. Added new `AuthUserProfile` wrapper struct for request extensions. Rewrote `build_context_from_fullstack()` to read both `AuthUser` and `AuthUserProfile` extensions, populating `current_user` with the full profile instead of `None`.

**`src/db/mod.rs`** — Added `update_user_profile()` function with a compile-time-checked sqlx query that updates `display_name`, `bio`, and `updated_at`, returning the updated `User` record. Added 3 new tests: `test_update_user_profile` (happy path), `test_update_user_profile_clears_bio` (sets bio to NULL), `test_update_user_profile_nonexistent` (returns error for missing user).

**`src/middleware/auth.rs`** — Added `State(PgPool)` parameter to `handle_auth`. After verifying the JWT, fetches the full user record via `db::get_user_by_id()` and injects both `AuthUser` and `AuthUserProfile` into request extensions.

**`src/main.rs`** — Added `Extension(pool.clone())` layer to the dioxus router for server function access. Changed middleware to use `from_fn_with_state(pool.clone(), handle_auth)`. Cloned pool before moving into `AppState` to allow reuse.

**`src/components/navbar.rs`** — Changed `u.username.clone()` to `u.display_name.clone()` so the navbar shows the user's real display name.

**`src/pages/settings/settings_profile.rs`** — Complete rewrite. Added signals for `display_name`, `bio`, `is_saving`, `error`, `saved_message`. Loads profile from `use_auth().current_user` on mount via `use_effect`. Server function `save_profile()` with server-side validation (display name 2-30 chars, bio max 160 chars). Optimistic UI: saves old values before calling server function, rolls back on error. Character counters for both fields. Uses existing `Input`, `Button`, `Card`, `PageHeader` components.

**`src/components/base/input.rs`** — No changes needed; `Input` component already supports all required props. Used raw `<textarea>` with `maxlength="160"` for the bio field.

### Tests
- **3 new tests** added to `src/db/mod.rs`:
  - `test_update_user_profile` — updates display_name and bio, verifies returned User and persistence
  - `test_update_user_profile_clears_bio` — sets bio to None, verifies it's NULL in DB
  - `test_update_user_profile_nonexistent` — returns error for non-existent user ID
- **All 60 tests pass** (57 existing + 3 new)

### Verification Results
- `cargo fmt` — applied, no formatting changes needed
- `cargo build --features server` — **passes** (clean compilation)
- `cargo build --target wasm32-unknown-unknown` — **passes** (clean compilation)
- `cargo clippy --features server -- -D warnings` — **passes** (no warnings)
- `cargo clippy --target wasm32-unknown-unknown -- -D warnings` — **passes** (no warnings)
- `cargo test --features server` — **60 passed, 0 failed**

### Notes / Follow-up
- Saved message auto-clear was removed (no WASM-compatible sleep available without adding dependencies). The message persists until the user edits the form or navigates away.
- `AuthUserProfile` and `AuthUser` imports in `settings_profile.rs` are conditionally compiled with `#[cfg(feature = "server")]` since they're only used in the `#[server]` function.

## Phase 2: Review Verdict

**Verdict: PASS** — all three review findings have been correctly fixed.

### Fixes Verified

#### 1. `use_effect` → `use_hook` for one-time initialization (FIXED ✅)
- **Location:** `src/pages/settings/settings_profile.rs`, line 61
- **Before:** `use_effect(move || { ... })` — ran on every render, overwriting user input on each keystroke.
- **After:** `use_hook(move || { ... })` — runs once on component creation, consistent with the pattern in `src/pages/login.rs:13`.
- **Status:** Correctly fixed. The closure captures `auth`, `display_name`, and `bio` signals and initializes them from the auth context exactly once.

#### 2. Optimistic update applies trimmed values and uses returned UserProfile (FIXED ✅)
- **Location:** `src/pages/settings/settings_profile.rs`, lines 90–120
- **Before:** Signals were never updated before the server call; `Ok(_)` discarded the returned `UserProfile`.
- **After:**
  - Lines 94–95: `display_name.set(trimmed_name.clone())` and `bio.set(trimmed_bio.clone().unwrap_or_default())` apply trimmed values immediately before the server call.
  - Lines 105–109: `Ok(profile)` captures the returned `UserProfile` and applies server-authoritative values: `display_name.set(profile.display_name)` and `bio.set(profile.bio.unwrap_or_default())`.
  - Lines 113–116: Rollback to `old_name`/`old_bio` on error preserved.
- **Status:** Correctly fixed. The full optimistic update cycle (apply trimmed → call server → apply authoritative on success / rollback on error) is now complete.

#### 3. Character counters show trimmed length (FIXED ✅)
- **Location:** `src/pages/settings/settings_profile.rs`, lines 160 and 195
- **Before:** `display_name().len()` and `bio().len()` — raw length including whitespace.
- **After:** `display_name().trim().len()` and `bio().trim().len()` — matches the server-side validation logic.
- **Status:** Correctly fixed. A user entering `"  a  "` now sees `1/30` and understands why validation fails.

### Positive Findings and Good Practices

1. **SQL injection protection:** All queries use `sqlx::query_as!` with parameterized bindings. No string interpolation in SQL. ✅
2. **Graceful middleware degradation:** If `db::get_user_by_id` fails, the user remains authenticated but lacks a profile extension. `build_context_from_fullstack()` handles this gracefully. ✅
3. **Auth enforcement in server function:** `save_profile()` extracts `AuthUser` from `FullstackContext` — `user_id` comes from the verified JWT, not user input. ✅
4. **Server-side validation:** Display name (2-30 chars) and bio (≤160 chars) validated independently on the server before any DB operation. ✅
5. **Comprehensive test coverage:** 3 new tests (happy path, clear bio to NULL, nonexistent user returns error). All 60 tests pass. ✅
6. **Clean WASM boundary:** `#[cfg(feature = "server")]` guards on server-only imports. WASM build passes cleanly. ✅
7. **Consistent error handling:** `DbError` properly wrapped; server function converts to user-friendly `ServerFnError` messages. ✅
8. **Pool wiring follows blueprint:** `Extension(pool)` layer for server functions + `from_fn_with_state(pool)` for middleware. Matches Dioxus 0.7 + Axum 0.8 patterns. ✅

### Requirements Coverage

| Requirement | Status |
|-------------|--------|
| AC1: Profile fetches in middleware | ✅ `db::get_user_by_id` called after JWT verification |
| AC1: Profile injected into extensions | ✅ `AuthUserProfile` inserted via `req.extensions_mut()` |
| AC1: `build_context_from_fullstack` reads profile | ✅ Reads both `AuthUser` and `AuthUserProfile` |
| AC1: Navbar shows display name | ✅ Changed from `u.username` to `u.display_name` |
| AC2: Settings page displays current values | ✅ Loads from `use_auth().current_user` via `use_hook` |
| AC2: Editable display name and bio | ✅ Input fields with signals |
| AC2: Server function persists changes | ✅ `save_profile()` calls `db::update_user_profile` |
| AC2: Optimistic UI with rollback | ✅ Full cycle: trimmed apply → server call → authoritative update / rollback |
| AC2: Display name 2-30 chars, trimmed | ✅ Both client and server validation |
| AC2: Bio max 160 chars, trimmed | ✅ Both client and server validation |
| Constraint: cargo fmt | ✅ Passes |
| Constraint: cargo clippy -D warnings (both targets) | ✅ Passes |
| Constraint: cargo test --features server | ✅ 60 passed, 0 failed |
| Constraint: Profile fetch synchronous in middleware | ✅ Uses `db::get_user_by_id().await` (middleware is async) |
| Constraint: Keep existing AuthUser extension | ✅ `AuthUser` unchanged, `AuthUserProfile` added separately |

### Summary

All three review findings (1 BLOCKER, 1 WARNING, 1 SUGGESTION) have been correctly addressed. The implementation is solid, follows the blueprint faithfully, and demonstrates good security and UX practices throughout. Ready to merge.
## Phase 3: Synthesis

### User-Facing Summary

**NOMS-005: User Profile & Account Management** AC1 + AC2 implemented and reviewed.

**AC1 — Profile data loads on every authenticated request:** Auth middleware fetches full user profile from PostgreSQL alongside JWT verification. New `AuthUserProfile` extension carries profile (including previously-missing `bio` field) through request pipeline. `build_context_from_fullstack()` reads both `AuthUser` and `AuthUserProfile` to populate `AuthContext.current_user`. Navbar displays real display name instead of "User" placeholder.

**AC2 — Profile settings page is functional:** `/settings/profile` rewritten with reactive signals, server function `save_profile()`, optimistic UI with rollback, and client+server validation (display name 2-30 chars trimmed, bio max 160 chars trimmed).

### Files Changed (6 files)

| File | Change |
|------|--------|
| `src/auth/context.rs` | Added `bio` to `UserProfile`, added `AuthUserProfile` extension, wired `build_context_from_fullstack()` |
| `src/db/mod.rs` | Added `update_user_profile()` + 3 tests |
| `src/middleware/auth.rs` | Added `State(PgPool)`, fetches profile, injects `AuthUserProfile` |
| `src/main.rs` | Wired pool via `Extension` layer + `from_fn_with_state` |
| `src/components/navbar.rs` | Changed `u.username` → `u.display_name` |
| `src/pages/settings/settings_profile.rs` | Full rewrite: signals, validation, server function, optimistic UI |

### Verification

`cargo fmt` ✅ | `cargo build` (server + wasm) ✅ | `cargo clippy` (both targets, -D warnings) ✅ | `cargo test --features server` ✅ **60 passed**

### Commit Message

```
feat: implement user profile fetch in middleware and settings page (NOMS-005)

AC1 — Profile fetch in auth middleware:
- Add `bio: Option<String>` to `UserProfile` to match DB schema
- Add `AuthUserProfile` extension to carry full profile in request
- Wire `PgPool` into middleware via `from_fn_with_state(pool)`
- Fetch profile via `db::get_user_by_id()` after JWT verification
- Update `build_context_from_fullstack()` to read both `AuthUser`
  and `AuthUserProfile` extensions, populating `current_user`
- Add `Extension(pool)` layer for server function pool access
- Navbar now displays `display_name` instead of `username`

AC2 — Functional profile settings page:
- Rewrite `/settings/profile` with reactive signals for display_name,
  bio, is_saving, error, and saved_message
- Initialize profile from `use_auth().current_user` via `use_hook`
- Add `save_profile` server function with server-side validation:
  display name 2-30 chars (trimmed), bio max 160 chars (trimmed)
- Implement optimistic UI: apply trimmed values immediately,
  apply server-authoritative values on success, rollback on error
- Character counters reflect trimmed length matching validation logic

DB:
- Add `update_user_profile()` with compile-time-checked sqlx query
- Update `display_name`, `bio`, and `updated_at`; return updated User
- Add 3 tests: happy path, clear bio to NULL, nonexistent user error

All 60 tests pass. Clippy clean on both x86_64 and wasm32 targets.
```

## Phase 1: Fix Details (Review Fixes)

### Summary of Fixes
Addressed all three review findings in `src/pages/settings/settings_profile.rs`: a BLOCKER preventing user input, a WARNING on incomplete optimistic updates, and a SUGGESTION on misleading character counters.

### Fixes Applied

**1. BLOCKER: `use_effect` replaced with `use_hook`** (line 61)
- `use_effect` runs on every render, causing the profile initialization to overwrite user input on each keystroke.
- Replaced with `use_hook` which runs only once on component creation (consistent with the pattern already used in `src/pages/login.rs`).

**2. WARNING: Optimistic update completed** (lines 90-120)
- Added immediate signal updates with trimmed values before the server call: `display_name.set(trimmed_name.clone())` and `bio.set(trimmed_bio.clone().unwrap_or_default())`.
- On success, the returned `UserProfile` is now captured and applied to signals (`display_name.set(profile.display_name)`, `bio.set(profile.bio.unwrap_or_default())`) instead of being discarded.
- Rollback on error remains unchanged (restores `old_name`/`old_bio`).

**3. SUGGESTION: Character counters show trimmed length** (lines 160, 195)
- Changed `display_name().len()` to `display_name().trim().len()` and `bio().len()` to `bio().trim().len()` in the counter spans, matching the validation logic.

### Files Modified
- **`src/pages/settings/settings_profile.rs`** — Only file changed. Three targeted edits within the `SettingsProfile` component.

### Verification Results
- `cargo fmt` — applied, no formatting changes needed
- `cargo build --features server` — **passes** (clean compilation)
- `cargo build --target wasm32-unknown-unknown` — **passes** (clean compilation)
- `cargo clippy --features server -- -D warnings` — **passes** (no warnings)
- `cargo clippy --target wasm32-unknown-unknown -- -D warnings` — **passes** (no warnings)
- `cargo test --features server` — **60 passed, 0 failed**
