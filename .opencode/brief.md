# Task Brief

## Task Description
Implement NOMS-005 AC6: Account deletion with 3-layer confirmation.

From the issue (roadmap/issues/NOMS-005-user-profile.md):
- "Delete Account" section at bottom of `/settings/profile` with Danger-styled button
- 3-layer confirmation flow:
  1. Initial confirmation dialog: "Are you sure? This will permanently delete your account and all associated data."
  2. Typed confirmation: user must type exactly `delete <username>` to proceed (input validated in real-time)
  3. Final confirmation: "This cannot be undone. Delete now?" with explicit Delete button
- Server function `delete_account()` deletes the user row from `users` table
- All associated data is deleted: `oauth_accounts` rows cascade via `ON DELETE CASCADE`
- Session cookie is invalidated on successful deletion (set expired `Set-Cookie`)
- User is redirected to `/` after deletion with unauthenticated state
- Error handling: if deletion fails, dialog closes with error message displayed inline

New DB query: `delete_user(id)` - Delete user row
New server function: `delete_account()` POST - Delete user + cascade, invalidate session, redirect

## Phase 0: Implementation Blueprint
<!-- written by @architect -->

### Overview
AC6 requires a 3-layer account deletion flow at the bottom of the profile settings page. The flow protects against accidental deletion by requiring progressive confirmation.

### DB Layer (`src/db/mod.rs`)
Add `delete_user` query that deletes a user by ID. oauth_accounts cascade automatically via ON DELETE CASCADE FK.
Add test: `test_delete_user_cascades_oauth_accounts` - insert user + 2 oauth accounts, delete user, verify both are gone.
Add test: `test_delete_nonexistent_user` - verify error on deleting non-existent user.

### Server Function (`src/pages/settings/settings_profile.rs`)
Add `delete_account` server function that extracts auth user and pool, calls `db::delete_user`, returns Ok/Err.
Client handles redirect + cookie clearing via POST to `/auth/logout` then `window.location().set_href("/")`.

### UI Layer - 3-Layer Confirmation Dialog
- `DeleteStep` enum: Confirming | Typing | Final
- Danger Zone card at bottom of page with red border
- Layer 1: "Are you sure?" modal with Cancel/Confirm
- Layer 2: Type `delete {username}` to continue, real-time validation
- Layer 3: "This cannot be undone" with Go Back / Delete My Account
- Modal overlay with backdrop click to dismiss (except Typing step)
- Error display in Layer 3 if deletion fails

### CSS
- `--error-bg` and `--success-bg` CSS variables (light + dark mode)
- `.danger-zone .neumo-card` — red border + tinted background
- `.modal-overlay` — fixed full-screen semi-transparent backdrop
- `.modal-card` — centered neumorphic card with slide-in animation
- `.modal-card code` — styled code block for typed confirmation hint

## Phase 1: Implementation Details
<!-- written by @implement -->

### Summary
Implemented the complete 3-layer account deletion flow for AC6 (NOMS-005). The implementation spans three layers: database query, server function, and UI component with modal dialogs.

### Files Modified

**`src/db/mod.rs`**
- Added `delete_user(executor, user_id)` — deletes a user row, returns error if not found
- Added `test_delete_user_cascades_oauth_accounts` — verifies ON DELETE CASCADE removes linked oauth_accounts
- Added `test_delete_nonexistent_user` — verifies error on deleting non-existent user

**`src/pages/settings/settings_profile.rs`**
- Added `DeleteStep` enum (Confirming, Typing, Final) for the 3-layer flow
- Added `delete_account()` server function — extracts auth context, calls `db::delete_user`
- Added 4 state signals: `delete_step`, `delete_input`, `deleting`, `delete_error`
- Added 5 handlers: `on_delete_confirm`, `on_delete_cancel`, `on_delete_continue`, `on_delete_go_back`, `on_delete_final`
- Added Danger Zone card section below the profile form
- Added modal overlay with 3 conditional layers (Confirming/Typing/Final)
- On successful deletion: POST to `/auth/logout` then `window.location().set_href("/")`

**`assets/main.css`**
- Added `--error-bg` and `--success-bg` CSS variables (both light and dark mode)
- Added `.danger-zone .neumo-card` — red border + error-tinted background
- Added `.modal-overlay` — fixed full-screen backdrop with fade-in
- Added `.modal-card` — centered neumorphic card with scale+translate slide-in animation
- Added `.modal-card code` — inline code styling for the typed confirmation hint

**`Cargo.toml`** (Phase 2 fix)
- Removed stray leading whitespace on line 31 before `web-sys` dependency

**`src/pages/settings/settings_profile.rs`** (Phase 2 fix)
- Ran `cargo fmt` to fix formatting inconsistency on the `auth.current_user.as_ref().map(...)` chain (lines 98-101)

### Tests
- `test_delete_user_cascades_oauth_accounts` — creates user + 2 oauth accounts, deletes user, verifies both user and accounts are gone
- `test_delete_nonexistent_user` — verifies `delete_user` returns error for non-existent ID
- All 68 tests pass (`cargo test --features server`)

### Verification
- `cargo fmt --check` — passes (no formatting issues)
- `cargo clippy --features server` — passes (no warnings)
- `cargo test --features server` — all 68 tests pass
- No new warnings introduced

## Phase 2: Review Verdict

**Verdict:** PASS

### Quality Gate Results

| Gate | Command | Result |
|---|---|---|
| Formatting | `cargo fmt --check` | ✅ Pass (no formatting issues) |
| Linting | `cargo clippy --features server` | ✅ Pass (no warnings) |
| Tests | `cargo test --features server` | ✅ Pass (68/68 tests passed) |

### Resolved Issues from Previous Review

1. **`cargo fmt` formatting inconsistency** — ✅ Fixed: `cargo fmt` applied to `src/pages/settings/settings_profile.rs`
2. **Leading whitespace in `Cargo.toml`** — ✅ Fixed: Stray space on line 31 before `web-sys` removed

### Positive Findings and Good Practices

- **`delete_user` query is well-implemented:** Properly checks `rows_affected` and returns `RowNotFound` when the user doesn't exist, preventing silent failures.
- **Comprehensive test coverage:** Both `test_delete_user_cascades_oauth_accounts` and `test_delete_nonexistent_user` are well-structured, using the shared test utilities (`test_utils::setup_test_db`, `test_utils::uid`) for isolation.
- **3-layer confirmation flow is correctly implemented:** `DeleteStep` enum cleanly models the state machine; backdrop click is disabled on the Typing step to prevent accidental dismissal during the critical typing phase.
- **Real-time input validation:** The `input_matches` signal correctly gates the "Continue" button, preventing progression until the exact `delete <username>` string is typed.
- **Session invalidation is handled correctly:** POST to `/auth/logout` (which sets `Max-Age=0` cookie) followed by `window.location().set_href("/")` for hard navigation — this properly clears the Dioxus client-side state too.
- **Loading state prevents double-click:** The `deleting()` signal disables the final button and shows "Deleting..." text, preventing duplicate deletion requests.
- **Error handling in Layer 3:** Errors from the server function are displayed inline in the modal, and the `deleting` flag is reset so the user can retry.
- **CSS is well-structured:** `--error-bg` and `--success-bg` variables are defined for both light and dark modes; `.modal-overlay` uses `z-index: 300` (above navbar's `z-index: 100`); `.modal-card` has a nice scale+translate slide-in animation.
- **Schema confirms CASCADE:** `oauth_accounts.user_id` has `REFERENCES users(id) ON DELETE CASCADE` — verified in both `migrations/schema.sql` and `src/test_utils.rs`.

### Requirements Coverage (from Task Description)

| Requirement | Status |
|---|---|
| "Delete Account" section at bottom of `/settings/profile` with Danger-styled button | ✅ Implemented as Danger Zone card with `ButtonVariant::Danger` |
| Layer 1: Initial confirmation dialog | ✅ Modal with "Are you sure?" message, Cancel/Confirm buttons |
| Layer 2: Typed confirmation (`delete <username>`) | ✅ Real-time validation, input + Continue button |
| Layer 3: Final confirmation | ✅ "This cannot be undone" with Go Back / Delete My Account |
| Server function `delete_account()` deletes user row | ✅ `#[server]` function calls `db::delete_user` |
| OAuth accounts cascade via `ON DELETE CASCADE` | ✅ Verified in schema and tested |
| Session cookie invalidated on deletion | ✅ POST to `/auth/logout` clears cookie |
| Redirect to `/` after deletion | ✅ `window.location().set_href("/")` |
| Error handling: dialog closes with inline error | ✅ Error displayed in Layer 3 modal |

### Summary

Solid implementation that faithfully follows the AC6 specification. All three quality gates pass cleanly. The 3-layer confirmation flow is well-structured with proper state management, the DB layer correctly leverages PostgreSQL CASCADE, and the CSS integrates cleanly with the existing design system. All previous issues have been resolved.

## Phase 3: Synthesis
<!-- written by @synthesize -->

### Feature Summary

NOMS-005 AC6 (Account Deletion with 3-Layer Confirmation) is **fully implemented and approved**. The feature adds a complete account deletion flow at the bottom of `/settings/profile`, protected by a progressive 3-layer confirmation dialog to prevent accidental deletion.

### Architecture

The implementation follows the established 3-layer pattern:

| Layer | Component | Responsibility |
|---|---|---|
| **DB** | `delete_user()` in `src/db/mod.rs` | Deletes user row; CASCADE handles oauth_accounts |
| **Server** | `delete_account()` server function | Extracts auth context, delegates to DB layer |
| **UI** | `DeleteStep` state machine + modal overlay | 3-layer confirmation flow with real-time validation |

### Key Design Decisions

1. **State machine over nested modals** — A single `DeleteStep` enum (`Confirming` | `Typing` | `Final`) drives conditional rendering within one modal overlay, avoiding modal stacking complexity.
2. **Typing step blocks backdrop dismiss** — The critical typed-confirmation step disables click-to-dismiss, preventing accidental cancellation mid-typing.
3. **Hard navigation after deletion** — POST to `/auth/logout` followed by `window.location().set_href("/")` ensures both server-side session invalidation and full Dioxus client-side state reset.
4. **CASCADE over manual cleanup** — Relies on PostgreSQL `ON DELETE CASCADE` for `oauth_accounts`, keeping the DB layer simple and atomic.

### Quality Assurance

All three quality gates pass cleanly:
- `cargo fmt --check` — no formatting issues
- `cargo clippy --features server` — no warnings
- `cargo test --features server` — 68/68 tests pass (including 2 new deletion tests)

### Coverage

All 9 acceptance criteria from the task description are satisfied. No open issues or follow-ups required.

### Status: COMPLETE
