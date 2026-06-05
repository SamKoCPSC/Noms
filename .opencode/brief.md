# Task Brief

## Task Description
Remove `get_auth_state` entirely from `src/db/mod.rs` and update/remove all associated test code. Tests that used `get_auth_state` for read-back verification should be refactored to use `delete_auth_state` (which returns `Option<AuthState>` via `DELETE ... RETURNING *`) instead.

## Phase 0: Implementation Blueprint

### Overview

Remove the `get_auth_state` function (currently at `src/db/mod.rs` lines 175-188, gated with `#[cfg(test)]`) and refactor all 10 test call sites across two files to use `delete_auth_state` instead.

`delete_auth_state` already returns `Option<AuthState>` via `DELETE ... RETURNING *`, making it a perfect drop-in replacement: `Some(row)` means the state existed (and is now consumed), `None` means it didn't exist. This matches every test's verification intent.

### Key Research Findings

- **`get_auth_state`** (`src/db/mod.rs:175-188`): Already gated with `#[cfg(test)]` — it is test-only and has zero production call sites. The production callback handler at `src/auth/oauth.rs:315` uses `delete_auth_state`.
- **`AuthState` struct** (`src/db/mod.rs:132-139`): Derives `sqlx::FromRow`; used by both `get_auth_state` and `delete_auth_state`. No changes needed.
- **Test infrastructure** (`src/test_utils.rs`): Each test gets a fresh temporary PostgreSQL database via `pgtemp`. No shared state between tests.
- **Total call sites**: 1 function definition + 10 test usages = 11 locations to modify.

### Complete Call Site Inventory

#### File: `src/db/mod.rs` (3 tests, 6 call sites)

| Line | Test | What `get_auth_state` does | Replacement strategy |
|------|------|---------------------------|---------------------|
| 175-188 | *(definition)* | Function body | **Delete entirely** (lines 174-189 including doc comment and blank line) |
| 592 | `test_insert_and_get_auth_state` | Read back inserted state to verify fields | Replace with `delete_auth_state`; assert on returned `AuthState` fields |
| 621 | `test_delete_auth_state` | Verify row is gone after `delete_auth_state` | Replace with `delete_auth_state(&pool, &state_id)`; assert `.is_none()` |
| 669 | `test_cleanup_expired_auth_states` | Verify fresh state still exists after cleanup | Replace with `delete_auth_state`; assert `.is_some()` |
| 673 | `test_cleanup_expired_auth_states` | Verify stale state was deleted by cleanup | Replace with `delete_auth_state`; assert `.is_none()` |

#### File: `src/auth/oauth.rs` (6 tests, 6 call sites)

| Line | Test | What `get_auth_state` does | Replacement strategy |
|------|------|---------------------------|---------------------|
| 693 | `test_insert_auth_state_with_provider` | Read back to verify provider + redirect_uri | Replace with `db::delete_auth_state`; assert on returned fields |
| 713 | `test_provider_mismatch_detection` | Read back to verify provider stored as "google" | Replace with `db::delete_auth_state`; assert on `.provider` |
| 735 | `test_state_expiry_check` | Read back to verify `created_at` is recent | Replace with `db::delete_auth_state`; assert on elapsed time |
| 753 | `test_auth_state_stores_code_verifier` | Read back to verify `code_verifier` field | Replace with `db::delete_auth_state`; assert on `.code_verifier` |
| 779 | `test_callback_retrieves_verifier_before_delete` | Verify state is gone after first `delete_auth_state` | Replace with `db::delete_auth_state`; assert `.is_none()` |
| 988 | `test_expired_state_returns_state_expired` | Verify expired state was consumed by `delete_auth_state` | Replace with `db::delete_auth_state`; assert `.is_none()` |

### Detailed Step-by-Step Implementation

#### Step 1: Remove `get_auth_state` from `src/db/mod.rs`

**File:** `src/db/mod.rs`
**Action:** Delete lines 174-189 (the blank line, doc comment, `#[cfg(test)]` attribute, and full function body).

The section to remove:
```rust

/// Get an auth state by ID.
#[cfg(test)]
pub async fn get_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
) -> Result<Option<AuthState>, DbError> {
    sqlx::query_as!(
        AuthState,
        "SELECT id, redirect_uri, provider, code_verifier, created_at FROM auth_states WHERE id = $1",
        id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}
```

After removal, the `insert_auth_state` function (ending at line 172) should be followed directly by the `delete_auth_state` function (starting at line 190).

#### Step 2: Refactor `test_insert_and_get_auth_state` in `src/db/mod.rs`

**File:** `src/db/mod.rs`, lines 583-599
**What the test verifies:** `insert_auth_state` correctly persists all fields (id, redirect_uri, provider, code_verifier).
**Refactoring:** Replace `get_auth_state` with `delete_auth_state`. The test name can stay the same (it still tests insert + read-back), or be renamed to `test_insert_and_delete_auth_state` for accuracy.

Change line 592 from:
```rust
let state = get_auth_state(&pool, &state_id).await.unwrap();
```
to:
```rust
let state = delete_auth_state(&pool, &state_id).await.unwrap();
```
The rest of the assertions (lines 593-598) remain unchanged since `delete_auth_state` returns the same `Option<AuthState>` type.

#### Step 3: Refactor `test_delete_auth_state` in `src/db/mod.rs`

**File:** `src/db/mod.rs`, lines 601-627
**What the test verifies:** `delete_auth_state` returns the row on first call, then `None` on second call, and the row is actually gone.
**Refactoring:** Replace the `get_auth_state` call on line 621 with a second `delete_auth_state` call.

Change lines 620-622 from:
```rust
        // Should be gone
        let state = get_auth_state(&pool, &state_id).await.unwrap();
        assert!(state.is_none());
```
to:
```rust
        // Should be gone (second delete returns None)
        let gone = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(gone.is_none());
```
Note: The subsequent `deleted_again` check on lines 624-626 already uses `delete_auth_state` and remains unchanged.

#### Step 4: Refactor `test_cleanup_expired_auth_states` in `src/db/mod.rs`

**File:** `src/db/mod.rs`, lines 629-675
**What the test verifies:** `cleanup_expired_auth_states` deletes only stale rows, preserving fresh ones.
**Refactoring:** Replace both `get_auth_state` calls with `delete_auth_state`.

Change lines 668-670 from:
```rust
        // Fresh state should still exist
        let fresh = get_auth_state(&pool, &fresh_id).await.unwrap();
        assert!(fresh.is_some());
```
to:
```rust
        // Fresh state should still exist
        let fresh = delete_auth_state(&pool, &fresh_id).await.unwrap();
        assert!(fresh.is_some());
```

Change lines 672-674 from:
```rust
        // Stale state should be gone
        let stale = get_auth_state(&pool, &stale_id).await.unwrap();
        assert!(stale.is_none());
```
to:
```rust
        // Stale state should be gone
        let stale = delete_auth_state(&pool, &stale_id).await.unwrap();
        assert!(stale.is_none());
```

#### Step 5: Refactor `test_insert_auth_state_with_provider` in `src/auth/oauth.rs`

**File:** `src/auth/oauth.rs`, lines 678-696
**What the test verifies:** `insert_auth_state` correctly stores provider and redirect_uri.
**Refactoring:** Replace `db::get_auth_state` with `db::delete_auth_state`.

Change line 693 from:
```rust
let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
```
to:
```rust
let state = db::delete_auth_state(&pool, &state_id).await.unwrap().unwrap();
```

#### Step 6: Refactor `test_provider_mismatch_detection` in `src/auth/oauth.rs`

**File:** `src/auth/oauth.rs`, lines 698-718
**What the test verifies:** Provider is stored correctly and can be compared for mismatch detection.
**Refactoring:** Replace `db::get_auth_state` with `db::delete_auth_state`.

Change line 713 from:
```rust
let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
```
to:
```rust
let state = db::delete_auth_state(&pool, &state_id).await.unwrap().unwrap();
```

#### Step 7: Refactor `test_state_expiry_check` in `src/auth/oauth.rs`

**File:** `src/auth/oauth.rs`, lines 720-741
**What the test verifies:** A freshly created auth state has a `created_at` timestamp that is recent (not expired).
**Refactoring:** Replace `db::get_auth_state` with `db::delete_auth_state`.

Change line 735 from:
```rust
let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
```
to:
```rust
let state = db::delete_auth_state(&pool, &state_id).await.unwrap().unwrap();
```

#### Step 8: Refactor `test_auth_state_stores_code_verifier` in `src/auth/oauth.rs`

**File:** `src/auth/oauth.rs`, lines 743-755
**What the test verifies:** The PKCE `code_verifier` field is stored and retrievable.
**Refactoring:** Replace `db::get_auth_state` with `db::delete_auth_state`.

Change line 753 from:
```rust
let state = db::get_auth_state(&pool, &state_id).await.unwrap().unwrap();
```
to:
```rust
let state = db::delete_auth_state(&pool, &state_id).await.unwrap().unwrap();
```

#### Step 9: Refactor `test_callback_retrieves_verifier_before_delete` in `src/auth/oauth.rs`

**File:** `src/auth/oauth.rs`, lines 757-785
**What the test verifies:** The atomic flow — `delete_auth_state` returns the row (including verifier), and subsequent reads return None.
**Refactoring:** This test already uses `delete_auth_state` for the primary flow. Only the final verification on line 779 uses `get_auth_state`.

Change lines 778-780 from:
```rust
            // State is now gone
            let state = db::get_auth_state(&pool, &state_id).await.unwrap();
            assert!(state.is_none());
```
to:
```rust
            // State is now gone (second delete returns None)
            let state = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(state.is_none());
```

#### Step 10: Refactor `test_expired_state_returns_state_expired` in `src/auth/oauth.rs`

**File:** `src/auth/oauth.rs`, lines 942-993
**What the test verifies:** An expired state is still consumed by `delete_auth_state`, and subsequent reads confirm it's gone.
**Refactoring:** Replace the final `db::get_auth_state` call with `db::delete_auth_state`.

Change lines 987-991 from:
```rust
            // Verify state is gone (consumed) even though it was expired
            let still_exists = db::get_auth_state(&pool, &state_id).await.unwrap();
            assert!(
                still_exists.is_none(),
                "expired state should still be consumed"
            );
```
to:
```rust
            // Verify state is gone (consumed) even though it was expired
            let still_exists = db::delete_auth_state(&pool, &state_id).await.unwrap();
            assert!(
                still_exists.is_none(),
                "expired state should still be consumed"
            );
```

### Files to Modify

| File | Changes |
|------|---------|
| `src/db/mod.rs` | Remove `get_auth_state` function (13 lines), refactor 3 tests (5 call sites) |
| `src/auth/oauth.rs` | Refactor 6 tests (6 call sites) |

### Files NOT Modified

| File | Reason |
|------|--------|
| `src/test_utils.rs` | No changes needed; test infrastructure unchanged |
| `src/auth/session.rs` | No `get_auth_state` usage |
| `src/auth/linking.rs` | No `get_auth_state` usage |
| `roadmap/implementation-plans/NOMS-004-oauth-auth.md` | Historical documentation; mentions `get_auth_state` in passing but is not code |

### No New Files, Dependencies, or Schema Changes

- No new files to create.
- No new dependencies to install.
- No database schema changes — `delete_auth_state` already uses the existing `auth_states` table.
- The `AuthState` struct remains unchanged.

### Implementation Order

1. **Step 1** — Remove the function definition first (cleanest to do before test changes).
2. **Steps 2-4** — Refactor the three tests in `src/db/mod.rs`.
3. **Steps 5-10** — Refactor the six tests in `src/auth/oauth.rs`.

All changes are independent of each other (each test uses its own isolated temp database), so steps 2-10 can technically be done in any order after step 1.

### Verification

After all changes:
- `grep -r "get_auth_state" src/` should return zero results.
- `cargo test --features server` should pass all existing tests with no behavior change.
- The production `callback_handler` in `src/auth/oauth.rs:315` already uses `delete_auth_state` and is unaffected.

### Architectural Notes

- `get_auth_state` was already `#[cfg(test)]` — it was never part of the production API. Removing it has zero runtime impact.
- Using `delete_auth_state` for test verification is semantically correct: it confirms the row exists (via `Some`) or doesn't (via `None`), which is exactly what all the tests need. The destructive nature (deleting the row) is irrelevant in test context since each test gets a fresh temporary database.
- The concurrent test (`test_concurrent_state_consumption` in `src/auth/oauth.rs:880-940`) already exclusively uses `delete_auth_state` and requires no changes.

## Phase 1: Implementation Details
<!-- written by @develop-implement -->

### Summary

Removed the `get_auth_state` function (13 lines including doc comment and `#[cfg(test)]` attribute) from `src/db/mod.rs` and replaced all 10 test call sites with `delete_auth_state` across two files. `delete_auth_state` returns the same `Option<AuthState>` type via `DELETE ... RETURNING *`, making it a drop-in replacement for read-back verification in tests.

### Files Modified

| File | Changes |
|------|---------|
| `src/db/mod.rs` | Deleted `get_auth_state` function (lines 174-189). Refactored 3 tests: `test_insert_and_get_auth_state` (line 592), `test_delete_auth_state` (line 621), and `test_cleanup_expired_auth_states` (lines 669, 673) — all `get_auth_state` calls replaced with `delete_auth_state`. |
| `src/auth/oauth.rs` | Refactored 6 tests: `test_insert_auth_state_with_provider` (line 693), `test_provider_mismatch_detection` (line 713), `test_state_expiry_check` (line 735), `test_auth_state_stores_code_verifier` (line 753), `test_callback_retrieves_verifier_before_delete` (line 779), and `test_expired_state_returns_state_expired` (line 988) — all `db::get_auth_state` calls replaced with `db::delete_auth_state`. |

### Tests

- **No new tests written** — all existing tests were refactored in place.
- **Verification:** `cargo test --features server` — **139 passed, 0 failed**.
- Post-edit `grep` confirms zero remaining `get_auth_state` call sites (only the test function name `test_insert_and_get_auth_state` remains, which is intentional per the blueprint).

### No New Files, Dependencies, or Schema Changes

As planned: no new files created, no new dependencies added, no database schema changes. The `AuthState` struct and `delete_auth_state` function remain unchanged.

## Phase 2: Review Verdict

**Verdict: PASS**

### Requirements Coverage

All objectives from the Task Description are met:

- `get_auth_state` is fully removed from `src/db/mod.rs` — the function definition (including doc comment, `#[cfg(test)]` attribute, and body) is gone.
- All 10 test call sites across two files are refactored to use `delete_auth_state`.
- `grep -r "get_auth_state" src/` returns zero code references (only the test function name `test_insert_and_get_auth_state` remains, which is intentional per the blueprint).
- Historical documentation (`roadmap/implementation-plans/NOMS-004-oauth-auth.md`) mentions `get_auth_state` in passing but is non-code and was correctly left untouched.

### Issue Analysis

No issues found. All 10 call sites were correctly refactored:

**`src/db/mod.rs` (3 tests, 5 call sites):**

1. `test_insert_and_get_auth_state` (line 576) — `delete_auth_state` replaces `get_auth_state` for read-back verification. All field assertions (`id`, `redirect_uri`, `provider`, `code_verifier`) remain identical. ✓
2. `test_delete_auth_state` (line 605) — second `delete_auth_state` call replaces `get_auth_state` for "should be gone" check. Asserts `.is_none()`. ✓
3. `test_cleanup_expired_auth_states` (lines 653, 657) — both `get_auth_state` calls replaced. Fresh state asserts `.is_some()`, stale asserts `.is_none()`. ✓

**`src/auth/oauth.rs` (6 tests, 5 call sites):**

4. `test_insert_auth_state_with_provider` (line 693) — asserts `provider` and `redirect_uri` on returned row. ✓
5. `test_provider_mismatch_detection` (line 713) — asserts `provider` equality/inequality. ✓
6. `test_state_expiry_check` (line 735) — asserts `created_at` elapsed time. ✓
7. `test_auth_state_stores_code_verifier` (line 753) — asserts `code_verifier` field. ✓
8. `test_callback_retrieves_verifier_before_delete` (line 779) — second `delete_auth_state` replaces `get_auth_state` for "state is gone" check. ✓
9. `test_expired_state_returns_state_expired` (line 988) — second `delete_auth_state` replaces `get_auth_state` for consumed-state verification. ✓

### Behavior Change Analysis

The key concern with switching from a read-only `get_auth_state` to a destructive `delete_auth_state` is whether any test reads the same state row multiple times and expects it to persist between reads. After reviewing all 10 call sites:

- **Single-read tests** (6 of 10): `test_insert_and_get_auth_state`, `test_insert_auth_state_with_provider`, `test_provider_mismatch_detection`, `test_state_expiry_check`, `test_auth_state_stores_code_verifier`, and the "fresh state" check in `test_cleanup_expired_auth_states` — each reads the row exactly once. No behavior change.
- **Multi-delete tests** (4 remaining calls): In `test_delete_auth_state`, `test_callback_retrieves_verifier_before_delete`, `test_expired_state_returns_state_expired`, and the "stale state" check in `test_cleanup_expired_auth_states` — the second call already expects `None`. The old `get_auth_state` would return `None` (row gone); the new `delete_auth_state` also returns `None` (row already deleted). Behavior is identical.

**No test behaves differently after the refactoring.**

### Verification

- **Compilation:** `cargo check --features server` — clean, zero warnings.
- **Test suite:** `cargo test --features server` — **139 passed, 0 failed**.

### Positive Findings

- The transition from `insert_auth_state` (line 172) to `delete_auth_state` (line 174) is clean: one blank line, no orphaned code or artifacts.
- The `AuthState` struct and `delete_auth_state` function were correctly left unchanged — the refactoring only touched call sites.
- Each test uses an isolated temporary database via `pgtemp`, so the destructive nature of `delete_auth_state` has no cross-test contamination risk.
- The production `callback_handler` (line 315) already uses `delete_auth_state` and is unaffected.
- The concurrent test (`test_concurrent_state_consumption`) already exclusively used `delete_auth_state` and required no changes, confirming the design was sound.

### Summary

Clean, surgical refactoring. All 10 call sites correctly updated, zero dangling references, all invariants preserved, full test suite passes.

## Phase 3: Synthesis

### Summary

The test-only `get_auth_state` function was removed from `src/db/mod.rs` and all 10 of its test call sites were refactored to use `delete_auth_state` instead. This is a cleanup refactoring with zero production impact: `get_auth_state` was already gated behind `#[cfg(test)]` and never called by production code. The production `callback_handler` already uses `delete_auth_state`, which returns `Option<AuthState>` via PostgreSQL's `DELETE ... RETURNING *` — making it a semantically correct drop-in replacement for read-back verification in tests.

**Result:** 13 lines removed, 10 call sites updated, 139 tests pass, zero warnings.

### Files Modified

| File | Changes |
|------|---------|
| `src/db/mod.rs` | Deleted `get_auth_state` function definition (13 lines: blank line, doc comment, `#[cfg(test)]` attribute, function body). Refactored 3 tests (`test_insert_and_get_auth_state`, `test_delete_auth_state`, `test_cleanup_expired_auth_states`) — 5 `get_auth_state` calls replaced with `delete_auth_state`. |
| `src/auth/oauth.rs` | Refactored 6 tests — 5 `db::get_auth_state` calls replaced with `db::delete_auth_state`. |

### Step-by-Step Walkthrough

#### `src/db/mod.rs`

1. **Removed `get_auth_state` function** (previously lines 174-189). This was a `#[cfg(test)]`-gated async function that ran `SELECT ... FROM auth_states WHERE id = $1` and returned `Result<Option<AuthState>, DbError>`. After removal, `insert_auth_state` is followed directly by `delete_auth_state` with one blank line between them.

2. **`test_insert_and_get_auth_state`** — The read-back call `get_auth_state(&pool, &state_id)` was replaced with `delete_auth_state(&pool, &state_id)`. All field assertions (`id`, `redirect_uri`, `provider`, `code_verifier`) remain unchanged because both functions return `Option<AuthState>` with identical struct fields.

3. **`test_delete_auth_state`** — The "should be gone" verification (third call in the test) was changed from `get_auth_state` to a second `delete_auth_state`. The test now calls `delete_auth_state` three times total: first returns `Some(row)`, second returns `None`, third also returns `None`.

4. **`test_cleanup_expired_auth_states`** — Both verification calls replaced. The fresh-state check now uses `delete_auth_state` and asserts `.is_some()`. The stale-state check now uses `delete_auth_state` and asserts `.is_none()`.

#### `src/auth/oauth.rs`

5. **`test_insert_auth_state_with_provider`** — `db::get_auth_state` → `db::delete_auth_state`. Asserts `provider` and `redirect_uri` on the returned row.

6. **`test_provider_mismatch_detection`** — `db::get_auth_state` → `db::delete_auth_state`. Asserts `provider` equality/inequality.

7. **`test_state_expiry_check`** — `db::get_auth_state` → `db::delete_auth_state`. Asserts `created_at` elapsed time is recent.

8. **`test_auth_state_stores_code_verifier`** — `db::get_auth_state` → `db::delete_auth_state`. Asserts `code_verifier` field matches.

9. **`test_callback_retrieves_verifier_before_delete`** — The final "state is gone" check changed from `db::get_auth_state` to `db::delete_auth_state`. Asserts `.is_none()`.

10. **`test_expired_state_returns_state_expired`** — The final "state is consumed" check changed from `db::get_auth_state` to `db::delete_auth_state`. Asserts `.is_none()`.

### Key Patterns and Notes

- **`DELETE ... RETURNING *`**: The `delete_auth_state` function uses PostgreSQL's `RETURNING` clause to atomically delete and return the row. This is the same pattern used by the production `callback_handler` and is the correct approach for OAuth state tokens (which are single-use by design).
- **`#[cfg(test)]` gating**: `get_auth_state` was compile-time excluded from production builds. Removing it has zero runtime or binary-size impact on production artifacts.
- **Isolated test databases**: Each test gets a fresh temporary PostgreSQL instance via `pgtemp`. The destructive nature of `delete_auth_state` has no cross-test contamination risk.
- **No behavior change**: The review confirmed that no test reads the same row multiple times expecting it to persist between reads. Tests that verify "row is gone" already expect `None`, which both `get_auth_state` (row deleted by prior `delete_auth_state`) and `delete_auth_state` (row already deleted) return identically.
- **Test name preserved**: `test_insert_and_get_auth_state` retains its name despite the implementation change, as noted in the blueprint. This is intentional — renaming was considered but deemed unnecessary.

### Follow-Up Recommendations

- **Monitor**: No specific monitoring needed. This is a pure test refactoring with no production code changes.
- **Consider**: If desired in a future cleanup, `test_insert_and_get_auth_state` could be renamed to `test_insert_and_delete_auth_state` for naming accuracy, but this is cosmetic.
- **Documentation**: The historical doc `roadmap/implementation-plans/NOMS-004-oauth-auth.md` mentions `get_auth_state` in passing. It was correctly left untouched as it is non-code documentation, but could be updated in a separate docs pass.

### Commit Message

```
refactor(test): remove get_auth_state, use delete_auth_state for verification

The get_auth_state function was a #[cfg(test)]-only helper that performed
a SELECT on auth_states for read-back verification in tests. It has been
removed entirely and all 10 test call sites now use delete_auth_state
instead, which returns Option<AuthState> via DELETE ... RETURNING *.

This is semantically correct for OAuth state tokens (single-use by
design) and matches the production callback_handler pattern. Each test
uses an isolated temporary database, so the destructive nature of
delete_auth_state has no cross-test impact.

Changes:
- src/db/mod.rs: Deleted get_auth_state function (13 lines). Refactored
  3 tests (5 call sites) to use delete_auth_state.
- src/auth/oauth.rs: Refactored 6 tests (5 call sites) to use
  db::delete_auth_state.

All 139 tests pass. Zero production code impact.
```
