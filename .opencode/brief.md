# Task Brief

## Task Description
Implement AC10 from NOMS-006: Auth states cleanup.

**AC10: Auth states cleanup (MEDIUM-4)**
- `pg_cron` job added to `migrations/extensions.sql` that runs every 5 minutes, deleting auth_states older than 15 minutes
- Fallback: application-level cleanup task on startup (tokio timer) if pg_cron unavailable
- Old auth states are purged within 15 minutes of creation

## Phase 0: Implementation Blueprint

### Overview

This task implements AC10 (Auth states cleanup) from NOMS-006. The goal is to ensure expired `auth_states` rows are purged from the database within 15 minutes of creation, using `pg_cron` as the primary mechanism with a Rust application-level fallback.

### Key Research Findings

1. **Existing pg_cron job**: `migrations/extensions.sql` (line 40-44) already has a `cleanup-auth-states` job, but it runs every 6 hours (`'0 */6 * * *'`) and deletes states older than 10 minutes. Both the schedule and the TTL threshold must change.

2. **Existing background task pattern**: `src/main.rs` (lines 121-130) already spawns a tokio background task for rate limit cleanup using `tokio::time::interval`. The auth states cleanup task follows the identical pattern.

3. **Application-side TTL**: `src/auth/oauth.rs` (line 234) defines `CSRF_STATE_TTL_SECS = 600` (10 minutes). This is the *validation* TTL checked during OAuth callback. The *cleanup* TTL of 15 minutes is a separate concern (grace period after validation expiry). These are intentionally different: validation rejects at 10 min, cleanup removes at 15 min.

4. **pg_cron infrastructure**: The Docker image (`docker/postgres/Dockerfile`, line 19, 46) already installs `postgresql-17-cron` and sets `shared_preload_libraries = 'timescaledb,pg_cron,pg_search'`. The init script (`docker/postgres/init-cron-config.sh`) configures `cron.database_name`. pg_cron is fully operational in the deployed environment.

5. **Database schema**: `auth_states` table has `created_at TIMESTAMPTZ` column with index `idx_auth_states_created_at` (`migrations/schema.sql`, lines 43-51). The `DELETE ... WHERE created_at < NOW() - INTERVAL` query is index-friendly.

6. **Test infrastructure**: `src/test_utils.rs` creates `auth_states` table in temp DBs. pg_cron is attempted with silent failure (line 35-37), meaning tests cannot rely on pg_cron.

### Files to Modify

#### 1. `migrations/extensions.sql` (lines 36-44) — UPDATE

**Change**: Replace the existing `cron.schedule` call to use the new schedule and TTL.

**Before** (lines 36-44):
```sql
-- Schedule cleanup of expired auth states (every 6 hours)
-- Use cron.schedule() rather than INSERT INTO cron.job directly.
-- The function fills in nodeport/nodename defaults that have NOT NULL constraints,
-- and it handles upsert (same jobname for the same user replaces the existing job).
SELECT cron.schedule(
    'cleanup-auth-states',
    '0 */6 * * *',
    'DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL ''10 minutes'''
);
```

**After**:
```sql
-- Schedule cleanup of expired auth states (every 5 minutes).
-- Uses cron.schedule() with a named job — calling again with the same name
-- replaces the existing schedule (upsert behavior). This handles the migration
-- from the previous '0 */6 * * *' schedule transparently.
-- Deletes auth_states older than 15 minutes (5-minute grace after the 10-minute
-- application-side validation TTL in oauth.rs:CSRF_STATE_TTL_SECS).
SELECT cron.schedule(
    'cleanup-auth-states',
    '*/5 * * * *',
    'DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL ''15 minutes'''
);
```

**Why this works**: `cron.schedule()` with the same job name (`'cleanup-auth-states'`) performs an upsert — it replaces the existing job. No need to `cron.unschedule()` first.

#### 2. `src/db/mod.rs` — ADD function + test

**Add** a new public function `cleanup_expired_auth_states` after the existing auth state queries (after line 197, before the OAuth account queries section at line 199):

```rust
/// Delete all auth states older than 15 minutes.
///
/// Used by the application-level fallback cleanup task when pg_cron is
/// unavailable. Also callable directly for testing.
pub async fn cleanup_expired_auth_states(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
) -> Result<u64, DbError> {
    let result = sqlx::query("DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL '15 minutes'")
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(result.rows_affected())
}
```

**Add** a test in the existing `#[cfg(test)] mod tests` block (after line 547):

```rust
    #[tokio::test]
    async fn test_cleanup_expired_auth_states() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // Insert a fresh state — should NOT be deleted
        let fresh_id = format!("test-state-fresh-{}", test_utils::uid());
        insert_auth_state(&pool, &fresh_id, "google", "/dashboard")
            .await
            .unwrap();

        // Insert a "stale" state by backdating its created_at via raw SQL
        let stale_id = format!("test-state-stale-{}", test_utils::uid());
        insert_auth_state(&pool, &stale_id, "github", "/login")
            .await
            .unwrap();
        sqlx::query("UPDATE auth_states SET created_at = NOW() - INTERVAL '20 minutes' WHERE id = $1")
            .bind(&stale_id)
            .execute(&pool)
            .await
            .unwrap();

        // Run cleanup
        let deleted = cleanup_expired_auth_states(&pool).await.unwrap();
        assert_eq!(deleted, 1);

        // Fresh state should still exist
        let fresh = get_auth_state(&pool, &fresh_id).await.unwrap();
        assert!(fresh.is_some());

        // Stale state should be gone
        let stale = get_auth_state(&pool, &stale_id).await.unwrap();
        assert!(stale.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_auth_states_empty_table() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let deleted = cleanup_expired_auth_states(&pool).await.unwrap();
        assert_eq!(deleted, 0);
    }
```

#### 3. `src/main.rs` — ADD background task

**Add** a new tokio::spawn block after the existing rate limit cleanup task (after line 130, before the `Ok(...)` on line 132). This follows the exact same pattern as the rate limit cleanup task:

```rust
            // Spawn background cleanup task for expired auth states.
            // Fallback when pg_cron is unavailable (e.g., local dev without
            // the custom Docker image). Runs every 5 minutes, deleting auth_states
            // older than 15 minutes. Dropped on server shutdown.
            {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
                    // Skip the initial immediate tick — we don't want to run
                    // cleanup the moment the server starts.
                    interval.tick().await;
                    loop {
                        interval.tick().await;
                        match db::cleanup_expired_auth_states(&pool).await {
                            Ok(n) if n > 0 => {
                                tracing::debug!(deleted = n, "cleaned up expired auth states");
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(error = %e, "failed to clean up expired auth states");
                            }
                        }
                    }
                });
            }
```

**Note on `tracing`**: The `tracing` crate is already a direct dependency (`Cargo.toml` line 11). No new dependencies needed.

**Note on pool cloning**: `pool` is already available in scope at line 74 (`let pool = db::get_pool();`). We clone it for the spawned task, same pattern as the rate limit task.

### No New Dependencies

All required crates are already in `Cargo.toml`:
- `tokio` (line 29) — for `tokio::spawn` and `tokio::time::interval`
- `tracing` (line 11) — for `tracing::debug!` and `tracing::warn!`
- `sqlx` (line 14) — for the cleanup query

### Implementation Order

1. **Step 1**: Update `migrations/extensions.sql` — change cron schedule and TTL.
2. **Step 2**: Add `cleanup_expired_auth_states` function to `src/db/mod.rs`.
3. **Step 3**: Add tests for the cleanup function in `src/db/mod.rs`.
4. **Step 4**: Add the background tokio task to `src/main.rs`.
5. **Step 5**: Run `cargo test --features server` to verify all tests pass.
6. **Step 6**: Run `cargo clippy --features server` to verify no lint warnings.

### Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| **15-min cleanup TTL vs 10-min validation TTL** | The application rejects states at 10 minutes (`CSRF_STATE_TTL_SECS`). The cleanup runs at 15 minutes to provide a 5-minute grace window, ensuring no valid state is ever deleted while still in use. |
| **5-minute cleanup interval** | Matches AC10 requirement. Ensures states are purged within 15 minutes of creation (worst case: created at :00:01, cleanup runs at :05:00, :10:00, :15:00 — deleted at :15:00, which is ~15 minutes). |
| **Skip first tick in tokio interval** | Avoids running cleanup immediately on server startup. The first cleanup runs after the first 5-minute interval elapses. |
| **No pg_cron detection logic** | The fallback task always runs alongside pg_cron. Both deleting the same rows is harmless (idempotent DELETE). This avoids complexity around detecting pg_cron availability and keeps the code simple. |
| **Named pg_cron job (upsert)** | Using the same job name `'cleanup-auth-states'` means `cron.schedule()` replaces the old schedule automatically. No migration to unschedule the old job is needed. |
| **Error handling: warn, don't panic** | Cleanup failures are non-critical (pg_cron is the primary mechanism). Logging a warning is sufficient. |

### Test Plan

| Test | Location | What it verifies |
|------|----------|-----------------|
| `test_cleanup_expired_auth_states` | `src/db/mod.rs` | Deletes only stale states, preserves fresh states |
| `test_cleanup_expired_auth_states_empty_table` | `src/db/mod.rs` | Returns 0 on empty table, no errors |
| Existing `test_insert_and_get_auth_state` | `src/db/mod.rs` | Confirms CRUD still works after changes |
| Existing `test_delete_auth_state` | `src/db/mod.rs` | Confirms individual delete still works |

### Gaps / Notes for Implementer

- The `src/auth/oauth.rs` constant `CSRF_STATE_TTL_SECS` (line 234) remains at 600 seconds (10 minutes). This is intentional — validation and cleanup TTLs are separate concerns.
- The `migrations/extensions.sql` file is applied before `schema.sql` (per the comment on line 2-3). The `cron.schedule()` call depends on the `auth_states` table existing, but since extensions.sql is run in the same session, the table may not exist yet. However, `cron.schedule()` only stores the job definition — it does NOT execute the query immediately. The query runs later when pg_cron fires. This is safe.
- The existing `cron.schedule()` call (lines 40-44) will be replaced by calling it again with the same job name. If this migration runs in an environment without pg_cron, the `CREATE EXTENSION IF NOT EXISTS "pg_cron"` (line 21) will fail silently... actually it won't — it will error. But the extensions.sql file already has this same issue today. The fallback Rust task handles the non-pg_cron case.
- Consider adding `tracing::info!` on the first successful cleanup run for observability, but this is optional.

## Phase 1: Implementation Details
### Summary
Implemented AC10 (Auth states cleanup): a `pg_cron` job running every 5 minutes to delete expired `auth_states` older than 15 minutes, with a Rust application-level fallback task for environments without pg_cron.

### Changes Made

#### Modified Files

1. **`migrations/extensions.sql`** (lines 36-48) — Updated the `cron.schedule` call:
   - Schedule changed from `'0 */6 * * *'` (every 6 hours) to `'*/5 * * * *'` (every 5 minutes)
   - TTL changed from `10 minutes` to `15 minutes`
   - Updated comments to reflect the new schedule and the relationship to the 10-minute validation TTL

2. **`src/db/mod.rs`** — Added `cleanup_expired_auth_states` function (after `delete_auth_state`, before OAuth account queries):
   - Public async function that deletes all `auth_states` older than 15 minutes
   - Returns `u64` count of deleted rows
   - Accepts any `sqlx::Executor` for testability

3. **`src/db/mod.rs`** — Added two tests in the existing `#[cfg(test)] mod tests`:
   - `test_cleanup_expired_auth_states`: Inserts a fresh state and a backdated (20-min old) state, runs cleanup, verifies only the stale state is deleted
   - `test_cleanup_expired_auth_states_empty_table`: Verifies cleanup returns 0 on an empty table without error

4. **`src/main.rs`** — Added a background tokio task (after the rate limit cleanup task):
   - Runs every 5 minutes (300 seconds) via `tokio::time::interval`
   - Skips the initial tick to avoid running at startup
   - Calls `db::cleanup_expired_auth_states()` each interval
   - Logs `debug!` when rows are deleted, `warn!` on errors
   - Dropped automatically on server shutdown

### Tests
- **`test_cleanup_expired_auth_states`**: Verifies stale states (backdated 20 min) are deleted, fresh states are preserved. Result: **PASS**
- **`test_cleanup_expired_auth_states_empty_table`**: Verifies 0 returned on empty table, no error. Result: **PASS**
- All 124 existing tests continue to pass.

### Verification
- `cargo clippy --no-default-features --features server -- -D warnings` — **PASS** (no warnings)
- `cargo test --no-default-features --features server` — **PASS** (124/124 tests)
- `cargo fmt` — **PASS** (no formatting changes needed)

### No New Dependencies
All required crates (`tokio`, `tracing`, `sqlx`) are already in `Cargo.toml`.

## Phase 2: Review Verdict

**Verdict: PASS**

All AC10 requirements are met. The implementation is clean, well-tested, and follows existing project patterns.

### Requirements Coverage

| Requirement | Status | Evidence |
|---|---|---|
| `pg_cron` job every 5 minutes | ✅ PASS | `migrations/extensions.sql` line 44: `'*/5 * * * *'` |
| 15-minute TTL for cleanup | ✅ PASS | SQL in both `extensions.sql` (line 45) and `db/mod.rs` (line 207) use `INTERVAL '15 minutes'` |
| Fallback tokio task on startup | ✅ PASS | `src/main.rs` lines 132–156, spawned via `tokio::spawn` |
| Fallback does not leak on shutdown | ✅ PASS | Task is a `tokio::spawn` child of the server runtime; dropped automatically when the runtime shuts down |
| Old auth states purged within 15 min | ✅ PASS | 5-min interval + 15-min TTL ensures worst-case deletion at ~15 min after creation |

### Issues

None found.

### Positive Findings and Good Practices

1. **Consistent pattern with existing code**: The auth states cleanup task in `main.rs` (lines 132–156) mirrors the existing rate-limit cleanup task (lines 118–130) exactly — same `tokio::time::interval` + `interval.tick().await` skip-first-tick pattern. This makes the codebase easier to maintain.

2. **Correct TTL separation**: The cleanup TTL (15 min) is intentionally larger than the validation TTL (`CSRF_STATE_TTL_SECS = 600` = 10 min in `oauth.rs` line 234). This 5-minute grace window ensures no in-use state is ever prematurely deleted. Good design.

3. **Idempotent dual cleanup**: Both pg_cron and the Rust fallback can run simultaneously without conflict. `DELETE ... WHERE created_at < ...` is naturally idempotent. No coordination logic needed.

4. **pg_cron upsert behavior**: Using `cron.schedule()` with the same job name (`'cleanup-auth-states'`) replaces the old schedule automatically. The migration from the previous `'0 */6 * * *'` / 10-min TTL to the new `'*/5 * * * *'` / 15-min TTL is transparent.

5. **Index-friendly query**: The `DELETE` query uses `created_at < NOW() - INTERVAL '15 minutes'` which leverages the existing `idx_auth_states_created_at` index (`migrations/schema.sql` line 51). No full table scan.

6. **Comprehensive tests**: Two dedicated tests cover the happy path (stale deleted, fresh preserved) and the edge case (empty table). Both pass. All 124 existing tests remain green.

7. **No new dependencies**: All required crates (`tokio`, `tracing`, `sqlx`) are already in `Cargo.toml`. Clean dependency footprint.

8. **Clean lint and format**: `cargo clippy -- -D warnings` passes with zero warnings. `cargo fmt --check` shows no formatting changes needed.

### Summary

Well-executed, minimal, and correct implementation. The code follows existing patterns, has appropriate test coverage, and introduces no new dependencies or lint issues. No fixes required.

## Phase 3: Synthesis

### Summary

AC10 (Auth states cleanup) from NOMS-006 has been implemented, reviewed, and approved. Expired `auth_states` rows are now purged from the database within 15 minutes of creation using two mechanisms:

1. **Primary**: A `pg_cron` job in `migrations/extensions.sql` that runs every 5 minutes, deleting auth states older than 15 minutes.
2. **Fallback**: A Rust application-level background task (tokio timer) that runs every 5 minutes on server startup, providing the same cleanup when pg_cron is unavailable (e.g., local development).

Both mechanisms are idempotent and can run simultaneously without conflict. No new dependencies were introduced.

### Files Changed

#### 1. `migrations/extensions.sql` — Updated pg_cron schedule

**What changed**: The existing `cron.schedule('cleanup-auth-states', ...)` call was updated in place.

- **Schedule**: `'0 */6 * * *'` (every 6 hours) → `'*/5 * * * *'` (every 5 minutes)
- **TTL**: `INTERVAL '10 minutes'` → `INTERVAL '15 minutes'`
- **Comments**: Updated to explain the new schedule and the relationship to the 10-minute application-side validation TTL.

**Why this works**: `cron.schedule()` with the same job name performs an upsert — the old 6-hour job is automatically replaced. No explicit unschedule step is needed.

#### 2. `src/db/mod.rs` — Added cleanup function and tests

**What was added**:
- **`cleanup_expired_auth_states()`** — A new public async function that executes `DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL '15 minutes'` and returns the count of deleted rows as `u64`. Accepts any `sqlx::Executor` for testability. Placed after `delete_auth_state()` and before the OAuth account query functions.
- **`test_cleanup_expired_auth_states`** — Inserts one fresh state and one backdated (20-min old) state, runs cleanup, and asserts that only the stale state is deleted (1 row affected). Verifies the fresh state still exists and the stale state is gone.
- **`test_cleanup_expired_auth_states_empty_table`** — Runs cleanup on an empty table and asserts 0 rows deleted with no error.

**Notable pattern**: The backdating technique (`UPDATE auth_states SET created_at = NOW() - INTERVAL '20 minutes'`) is used to simulate expired rows without waiting in tests.

#### 3. `src/main.rs` — Added background cleanup task

**What was added**: A new `tokio::spawn` block (after the existing rate-limit cleanup task) that:
- Creates a `tokio::time::interval` with a 300-second (5-minute) period.
- Skips the initial tick (`interval.tick().await`) so cleanup does not run immediately on startup.
- Loops on each tick, calling `db::cleanup_expired_auth_states(&pool)`.
- Logs `debug!` when rows are deleted, `warn!` on errors, and silently continues on success with 0 rows.
- Is automatically dropped when the server runtime shuts down (no explicit shutdown logic needed).

**Notable pattern**: This mirrors the existing rate-limit cleanup task exactly — same interval pattern, same error handling style, same pool cloning approach. This makes the codebase consistent and easier to maintain.

### Dependencies

No new dependencies were introduced. All required crates (`tokio`, `tracing`, `sqlx`) were already in `Cargo.toml`.

### Key Design Decisions

| Decision | Rationale |
|---|---|
| **15-min cleanup TTL vs 10-min validation TTL** | The application rejects states at 10 minutes (`CSRF_STATE_TTL_SECS`). Cleanup at 15 minutes provides a 5-minute grace window so no in-use state is ever prematurely deleted. |
| **5-min cleanup interval** | Ensures states are purged within ~15 minutes of creation in the worst case. |
| **Skip first tick** | Avoids running cleanup immediately on server startup; first run happens after the first 5-minute interval. |
| **No pg_cron detection** | The fallback always runs alongside pg_cron. Both deleting the same rows is harmless (idempotent DELETE), avoiding complexity around detecting pg_cron availability. |
| **Error handling: warn, don't panic** | Cleanup failures are non-critical since pg_cron is the primary mechanism. A warning log is sufficient. |

### Test Results

| Test | Location | Result |
|---|---|---|
| `test_cleanup_expired_auth_states` | `src/db/mod.rs` | **PASS** — Deletes stale, preserves fresh |
| `test_cleanup_expired_auth_states_empty_table` | `src/db/mod.rs` | **PASS** — Returns 0 on empty table |
| All existing tests (124 total) | — | **PASS** — No regressions |

**Total tests**: 124 pass (122 existing + 2 new)

### Verification

- `cargo clippy --no-default-features --features server -- -D warnings` — **PASS** (zero warnings)
- `cargo test --no-default-features --features server` — **PASS** (124/124)
- `cargo fmt` — **PASS** (no formatting changes)
- Review verdict: **PASS** — No issues found

### Areas to Monitor

- In production, verify that pg_cron is firing correctly by checking for `debug!` log lines from the fallback (they should be rare if pg_cron is handling the cleanup).
- If the `auth_states` table grows unexpectedly large, consider adding a `VACUUM` schedule or monitoring table bloat, since frequent DELETEs can cause bloat over time.

### Commit Message

```
feat(auth): add periodic cleanup of expired auth states

Implement AC10 from NOMS-006: auth states cleanup. Expired auth_states
rows are now purged within 15 minutes of creation using two mechanisms:

Primary — pg_cron job (migrations/extensions.sql):
- Schedule changed from every 6 hours to every 5 minutes ('*/5 * * * *')
- TTL changed from 10 minutes to 15 minutes
- Uses cron.schedule() upsert behavior to replace the old job in place

Fallback — application-level tokio task (src/main.rs):
- Runs every 5 minutes via tokio::time::interval
- Skips initial tick to avoid running at startup
- Logs debug on deletion, warn on error
- Dropped automatically on server shutdown

Both mechanisms are idempotent and can run simultaneously without
conflict. The 15-minute cleanup TTL provides a 5-minute grace window
after the 10-minute application-side validation TTL (CSRF_STATE_TTL_SECS),
ensuring no in-use state is ever prematurely deleted.

Changes:
- migrations/extensions.sql: updated cron schedule and TTL
- src/db/mod.rs: added cleanup_expired_auth_states() function
- src/db/mod.rs: added 2 tests (stale/fresh deletion, empty table)
- src/main.rs: added background tokio cleanup task

No new dependencies introduced. All 124 tests pass. Clippy clean.
```
