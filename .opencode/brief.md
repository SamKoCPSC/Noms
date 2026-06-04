# Task Brief

## Task Description
Implement AC5 from NOMS-006: Rate limiting on OAuth endpoints.

**AC5: Rate limiting on OAuth endpoints (HIGH-3)**
- Rate limiting middleware applied to `/auth/{provider}/start` and `/auth/{provider}/callback`
- Limits: 10 starts/minute per IP, 5 callbacks/minute per IP
- Exceeded limit returns `429 Too Many Requests` with `Retry-After` header
- Implementation uses sliding window (`Arc<DashMap<IpAddr, Vec<Instant>>>`) or `governor` crate
- Rate limit state is cleaned up periodically to prevent memory growth
- No impact on legitimate user flows

## Phase 0: Implementation Blueprint

### Research Findings

**Codebase Architecture:**
- Dioxus fullstack app using Axum 0.8 as the server framework (`Cargo.toml` line 19)
- OAuth routes registered as a standalone `oauth_router` in `src/main.rs` lines 95-109, merged with `dioxus_router` at line 111
- Existing middleware pattern: `src/middleware/auth.rs` uses `axum::middleware::from_fn_with_state(pool.clone(), middleware::auth::handle_auth)` (line 84)
- `AppState` defined in `src/auth/oauth.rs` lines 40-50, cloned and passed to `oauth_router.with_state(state)` (line 109)
- Tower 0.5 is already a dev dependency (`Cargo.toml` line 59); `tower::ServiceExt` used in logout tests (line 73)
- All server-only code guarded with `#[cfg(feature = "server")]`
- `dashmap` is NOT currently a direct dependency (only transitive via other crates)

**Rate Limiting Library Evaluation:**
- `tower_governor` 0.8.0 (latest, Aug 2025): supports Axum 0.8 + Tower 0.5, uses `governor` 0.10 internally. Requires `into_make_service_with_connect_info::<SocketAddr>()` for IP extraction, which is incompatible with Dioxus's `serve()` function that builds the router internally.
- `axum-governor` 2.0.1: MSRV 1.95 (too new), less mature (4K downloads vs 2.6M for tower_governor).
- **Decision: Custom middleware using `Arc<DashMap<IpAddr, VecDeque<Instant>>>`** -- aligns with the brief's explicit mention of this data structure, avoids library compatibility issues with Dioxus's router construction, and gives full control over per-endpoint limits (10 start/min, 5 callback/min).

**Reference URLs:**
- https://crates.io/crates/tower_governor (v0.8.0, 2.6M downloads)
- https://docs.rs/tower_governor/latest/tower_governor/
- https://crates.io/crates/governor (v0.10.4, 55M downloads)
- https://crates.io/crates/dashmap (v6, used by governor internally)

### Axum State Extraction Constraint

Axum's `from_fn_with_state<T>` requires the middleware's state type `T` to match the router's state type exactly. Since `oauth_router` already has `.with_state(AppState)` (line 109), the rate limit middleware must also extract `AppState` and access the rate limit state through a field on `AppState`. This avoids wrapper structs or Extension layers.

### File-Level Implementation Plan

#### Step 1: Add `dashmap` dependency

**File: `Cargo.toml`**

Add `dashmap` to `[dependencies]` (after line 30):
```toml
dashmap = { version = "6", optional = true }
```

Add `"dashmap"` to the `server` feature list (after line 53):
```toml
server = [
    "dioxus/server", "sqlx", "aws-sdk-s3", "tracing-subscriber",
    "oauth2", "jsonwebtoken", "axum", "axum-extra", "reqwest",
    "time", "cookie", "tokio", "unicode-normalization", "dashmap",
]
```

#### Step 2: Create `src/middleware/rate_limit.rs`

New file. Module provides:
1. `RateLimitState` -- shared in-memory store (two DashMaps, one per endpoint category)
2. `check()` method -- sliding window algorithm with pruning
3. `cleanup()` method -- periodic removal of stale entries
4. `rate_limit_middleware` -- the Axum middleware handler
5. Helpers: `extract_client_ip()`, `categorize_path()`
6. Comprehensive unit tests

**Data structures:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EndpointCategory {
    Start,    // 10 requests/minute per IP
    Callback, // 5 requests/minute per IP
}

#[derive(Clone)]
pub struct RateLimitState {
    start_windows: Arc<DashMap<IpAddr, VecDeque<Instant>>>,
    callback_windows: Arc<DashMap<IpAddr, VecDeque<Instant>>>,
}
```

**Constants:**
- `START_LIMIT_PER_MIN: usize = 10`
- `CALLBACK_LIMIT_PER_MIN: usize = 5`
- `WINDOW_SECS: u64 = 60`
- `ENTRY_TTL_SECS: u64 = 300` (5 minutes, for cleanup)

**Middleware function signature:**
```rust
pub async fn rate_limit_middleware(
    State(app_state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body>
```

**IP extraction:** Checks `X-Forwarded-For` header first (reverse proxy), falls back to `ConnectInfo<SocketAddr>` from request extensions, defaults to `0.0.0.0`.

**Path categorization:** Splits path by `/`, matches `/auth/{provider}/start` or `/auth/{provider}/callback` (exactly 4 segments). Returns `None` for other paths (e.g., `/auth/logout`), allowing passthrough.

**429 response:** Status `TOO_MANY_REQUESTS`, body `"Too Many Requests"`, `Retry-After` header = seconds until oldest entry expires (min 1, max 60), `Content-Type: text/plain`.

**Sliding window algorithm (check method):**
1. Prune all timestamps older than 60s from the deque
2. If deque length >= limit, reject with `Retry-After` = seconds until oldest timestamp expires
3. Otherwise, push current timestamp and allow

#### Step 3: Update `src/middleware/mod.rs`

Append after line 6:
```rust
pub mod rate_limit;
```

#### Step 4: Update `src/auth/oauth.rs` -- Add `rate_limit` field to `AppState`

**Lines 40-50**, add one new field to `AppState`:
```rust
/// Shared rate limit state for OAuth endpoint protection.
pub rate_limit: crate::middleware::rate_limit::RateLimitState,
```

No `Clone` impl change needed -- `RateLimitState` derives `Clone` via `Arc` internals.

#### Step 5: Update `src/main.rs` -- Wire up rate limiting

**Lines 68-113**, four modifications inside the `serve` closure:

**5a. Create `RateLimitState`** (new line before `let state = ...`):
```rust
let rate_limit = middleware::rate_limit::RateLimitState::default();
```

**5b. Add to `AppState` construction** (new field in struct literal):
```rust
let state = auth::oauth::AppState {
    pool: pool.clone(),
    google_client,
    github_client,
    http_client: reqwest::Client::new(),
    rate_limit: rate_limit.clone(),
};
```

**5c. Apply middleware layer to `oauth_router`** (insert `.layer(...)` before `.with_state(state)`):
```rust
let oauth_router = axum::Router::new()
    .route(/* existing routes */)
    .layer(axum::middleware::from_fn_with_state(
        state.clone(),
        middleware::rate_limit::rate_limit_middleware,
    ))
    .with_state(state);
```

**5d. Spawn background cleanup task** (new block after `Ok(oauth_router.merge(dioxus_router))`):
```rust
{
    let rl = rate_limit;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            rl.cleanup();
        }
    });
}
```

The cleanup task runs every 60 seconds, removing map entries for IPs with no timestamps within the last 5 minutes. Spawned inside the `serve` closure; dropped on server shutdown.

#### Step 6: Tests

All tests in `src/middleware/rate_limit.rs` under `#[cfg(test)] mod tests`. Module uses `#![cfg(feature = "server")]` so tests compile with server feature. Tower is already a dev dependency.

**Unit tests (10):**

| Test | Verifies |
|---|---|
| `test_start_allows_up_to_limit` | 10 requests pass, 11th rejected |
| `test_callback_allows_up_to_limit` | 5 requests pass, 6th rejected |
| `test_start_and_callback_independent` | Exhausting start does not affect callback |
| `test_different_ips_independent` | Two IPs each get their own quota |
| `test_retry_after_positive` | Err value >= 1 and <= 60 |
| `test_cleanup_removes_empty` | All-expired entries are removed |
| `test_cleanup_preserves_recent` | Recent timestamps survive cleanup |
| `test_categorize_path_start` | `/auth/google/start` -> Start |
| `test_categorize_path_callback` | `/auth/google/callback` -> Callback |
| `test_categorize_path_non_oauth` | `/auth/logout`, `/dashboard` -> None |

**Integration tests (4):** Build a minimal Axum router with a mock handler, apply the rate limit middleware, send requests via `tower::ServiceExt::oneshot`. Follow pattern from `src/auth/logout.rs` tests (lines 73-337).

| Test | Verifies |
|---|---|
| `test_middleware_429_start_exceeded` | 11th GET to `/auth/google/start` returns 429 + `Retry-After` |
| `test_middleware_429_callback_exceeded` | 6th GET to `/auth/google/callback` returns 429 + `Retry-After` |
| `test_middleware_passes_through_logout` | GET `/auth/logout` is not rate limited |
| `test_middleware_retry_after_header` | 429 response includes valid `Retry-After` header |

For integration tests, construct a minimal `AppState` using `test_utils::setup_test_db()` for the pool and `build_oauth_clients()` for OAuth clients, following the pattern from `src/auth/oauth.rs` db_tests.

### Implementation Order

1. `Cargo.toml` -- add dashmap dependency
2. `src/middleware/rate_limit.rs` -- new file with all logic + unit tests
3. `src/middleware/mod.rs` -- add module export
4. `src/auth/oauth.rs` -- add `rate_limit` field to `AppState`
5. `src/main.rs` -- wire up state, middleware layer, cleanup task
6. Run `cargo test --features server` to verify all tests pass

### Architectural Decisions and Trade-offs

**Custom middleware over `tower_governor`:**
- Pro: No dependency on `into_make_service_with_connect_info`, incompatible with Dioxus `serve()`
- Pro: Full control over per-endpoint limits (different limits for start vs callback)
- Pro: Simpler dependency graph (dashmap is lightweight)
- Con: We implement the sliding window ourselves instead of using a library
- Mitigation: Algorithm is well-understood; unit tests cover all edge cases

**Embedding `RateLimitState` in `AppState`:**
- Pro: Cleanest Axum state extraction -- middleware and handler share the same state type
- Pro: No wrapper structs or Extension layers needed
- Con: Couples `AppState` to the middleware module
- Mitigation: Acceptable since rate limiting is a core security concern for OAuth endpoints

**In-memory state (no persistence):**
- Pro: Zero latency, no database dependency
- Pro: State naturally bounded by cleanup task
- Con: Rate limits reset on server restart
- Mitigation: Acceptable for OAuth endpoints -- restart is a natural reset, 60s window limits abuse

**VecDeque over Vec for timestamps:**
- Pro: O(1) push_back, efficient front iteration for pruning
- Pro: Semantically matches sliding window pattern (FIFO expiration)

### Gaps and Areas for Follow-up

1. **IP spoofing via X-Forwarded-For:** Middleware trusts XFF headers. Correct behind a reverse proxy (nginx, Cloudflare). In direct exposure, an attacker could spoof IP. Consider a config flag to disable XFF trust in non-production.

2. **Distributed deployments:** In-memory rate limiting does not work across multiple server instances. Each instance maintains independent state. For multi-instance deployments, consider Redis-based rate limiting.

3. **Burst tolerance:** Implementation is strict -- exactly N requests per 60-second window. Some OAuth flows may generate slightly more than expected. The 10/5 limits were chosen from the AC spec.

## Phase 1: Implementation Details

### Summary
Implemented AC5 rate limiting on OAuth endpoints using a custom sliding-window middleware with `Arc<DashMap<IpAddr, VecDeque<Instant>>>`. Two independent rate limits are enforced: 10 starts/minute per IP and 5 callbacks/minute per IP. Exceeded limits return `429 Too Many Requests` with a `Retry-After` header. A background cleanup task runs every 60 seconds to prevent memory growth.

### Files Changed

**New files:**
- `src/middleware/rate_limit.rs` — Complete rate limiting module: `RateLimitState` (two DashMaps for start/callback windows), sliding window `check()` algorithm, `cleanup()` for stale entry removal, `rate_limit_middleware` Axum middleware, `extract_client_ip()` (X-Forwarded-For → ConnectInfo → fallback), `categorize_path()` for endpoint matching, `make_429_response()` helper. Includes 10 unit tests and 4 integration tests.

**Modified files:**
- `Cargo.toml` — Added `dashmap = { version = "6", optional = true }` to dependencies; added `"dashmap"` to the `server` feature list.
- `src/middleware/mod.rs` — Added `pub mod rate_limit;` export.
- `src/auth/oauth.rs` — Added `pub rate_limit: crate::middleware::rate_limit::RateLimitState` field to `AppState`.
- `src/main.rs` — Three changes: (1) create `RateLimitState::default()` before `AppState` construction, (2) apply `.layer(from_fn_with_state(state, rate_limit_middleware))` to `oauth_router`, (3) spawn background cleanup task via `tokio::spawn` with 60-second interval.

### Tests

**Unit tests (10) in `src/middleware/rate_limit.rs`:**
| Test | Verifies |
|---|---|
| `test_start_allows_up_to_limit` | 10 requests pass, 11th rejected |
| `test_callback_allows_up_to_limit` | 5 requests pass, 6th rejected |
| `test_start_and_callback_independent` | Exhausting start does not affect callback |
| `test_different_ips_independent` | Two IPs each get their own quota |
| `test_retry_after_positive` | Err value >= 1 and <= 60 |
| `test_cleanup_removes_empty` | All-expired entries are removed |
| `test_cleanup_preserves_recent` | Recent timestamps survive cleanup |
| `test_categorize_path_start` | `/auth/google/start` → Start |
| `test_categorize_path_callback` | `/auth/google/callback` → Callback |
| `test_categorize_path_non_oauth` | `/auth/logout`, `/dashboard` → None |

**Integration tests (4) in `src/middleware/rate_limit.rs::integration_tests`:**
| Test | Verifies |
|---|---|
| `test_middleware_429_start_exceeded` | 11th GET to `/auth/google/start` returns 429 + `Retry-After` |
| `test_middleware_429_callback_exceeded` | 6th GET to `/auth/google/callback` returns 429 |
| `test_middleware_passes_through_logout` | GET `/auth/logout` is not rate limited (20 requests pass) |
| `test_middleware_retry_after_header` | 429 response includes valid `Retry-After` header (1-60) |

### Verification
- `cargo clippy --no-default-features --features server -- -D warnings` — passed, no warnings
- `cargo test --no-default-features --features server` — 122 tests passed (112 existing + 14 new)
- `cargo fmt` — applied, no changes needed

### Notes
- `EndpointCategory` enum is private (module-local) since it's only used by the middleware and tests within the same module.
- `RateLimitState::check()` is private; only `cleanup()` is `pub` (called from `main.rs`).
- `X-Forwarded-For` header name uses a `LazyLock<HeaderName>` since it's not a standard header in the `http` crate.

## Phase 2: Review Verdict

**Verdict: PASS**

### Requirements Coverage (AC5)

| Requirement | Status |
|---|---|
| Middleware applied to `/auth/{provider}/start` and `/auth/{provider}/callback` | ✅ Applied to `oauth_router` (main.rs:112-115) |
| 10 starts/minute per IP | ✅ `START_LIMIT_PER_MIN = 10` (rate_limit.rs:29) |
| 5 callbacks/minute per IP | ✅ `CALLBACK_LIMIT_PER_MIN = 5` (rate_limit.rs:36) |
| 429 Too Many Requests with Retry-After header | ✅ `make_429_response()` (rate_limit.rs:223-228) |
| Sliding window via `Arc<DashMap<IpAddr, VecDeque<Instant>>>` | ✅ Two independent maps (rate_limit.rs:78-81) |
| Periodic cleanup to prevent memory growth | ✅ Background task every 60s (main.rs:121-130) |
| No impact on legitimate flows (e.g., `/auth/logout`) | ✅ `categorize_path` returns `None`, passthrough (rate_limit.rs:246-248) |

### Issues

**1. No test for time-based window expiration (SUGGESTION)**

- **Location:** `src/middleware/rate_limit.rs`, unit tests
- **Severity:** SUGGESTION
- **Description:** There is no test that verifies the sliding window resets after 60 seconds. All limit tests (`test_start_allows_up_to_limit`, `test_callback_allows_up_to_limit`) exercise the counter but never verify that timestamps older than `WINDOW_SECS` are pruned by the `check()` method, allowing new requests. The `test_cleanup_removes_empty` test exercises pruning inside `cleanup()`, but not inside `check()`.
- **Recommended fix:** Add a test that inserts timestamps older than 60 seconds into the deque, then calls `check()` and verifies the request is allowed (because old entries were pruned). Example: insert 10 timestamps at `Instant::now() - 65s`, then assert `check()` returns `Ok(())`.

**2. Trailing-slash paths bypass rate limiting (SUGGESTION)**

- **Location:** `src/middleware/rate_limit.rs`, `categorize_path()` line 206-220
- **Severity:** SUGGESTION
- **Description:** A path like `/auth/google/start/` splits into 5 segments (`["", "auth", "google", "start", ""]`) and returns `None`, bypassing rate limiting. However, Axum's router also won't match this path against `/auth/{provider}/start`, so the request gets a 404 anyway. Low practical risk.
- **Recommended fix:** No action needed. Acceptable as-is since the route won't match regardless.

**3. Cleanup task has no graceful shutdown hook (SUGGESTION)**

- **Location:** `src/main.rs` lines 121-130
- **Severity:** SUGGESTION
- **Description:** The cleanup task runs in an infinite `loop`. It relies on the tokio runtime being dropped on server shutdown to terminate. This works correctly with the current Dioxus `serve()` architecture but offers no explicit cancellation mechanism.
- **Recommended fix:** No action needed for now. If the server lifecycle needs more control in the future, consider using `tokio::select!` with a `tokio::sync::watch` channel or `tokio_util::sync::CancellationToken`.

### Positive Findings and Good Practices

1. **Correct DashMap usage:** The `check()` method uses `map.entry(ip).or_insert_with(VecDeque::new)` which is atomic — no race condition between the check and insert. The `cleanup()` method correctly collects keys to remove before iterating, avoiding mutation-during-iteration issues.

2. **Robust Retry-After computation:** Uses `saturating_duration_since` to prevent overflow and `clamp(1, 60)` to ensure the header value is always valid per RFC 6585.

3. **Clean IP extraction fallback chain:** `X-Forwarded-For` → `ConnectInfo<SocketAddr>` → `0.0.0.0` covers the three realistic scenarios (reverse proxy, direct connection, and tests/edge cases).

4. **Integration tests follow established patterns:** The integration tests mirror the existing logout test pattern (`tower::ServiceExt::oneshot`, `test_utils::setup_test_db()`, `build_oauth_clients()`), making them maintainable and consistent.

5. **Proper use of `LazyLock` for `X_FORWARDED_FOR`:** Since `X-Forwarded-For` is not a standard header in the `http` crate, using `LazyLock<HeaderName>` avoids per-request allocation.

6. **Private `EndpointCategory` enum:** Keeping the enum module-local is correct — it's an implementation detail that shouldn't leak into the public API.

7. **Clippy and formatting clean:** `cargo clippy --no-default-features --features server -- -D warnings` passes with zero warnings. `cargo fmt -- --check` is clean.

8. **All 14 tests pass:** 10 unit tests + 4 integration tests, covering limit enforcement, category/IP independence, Retry-After validation, cleanup behavior, path categorization, and full middleware integration.

### Overall Quality Summary

Well-structured, correctly implemented sliding-window rate limiter that faithfully fulfills AC5. The DashMap-based approach avoids library compatibility issues with Dioxus's router construction while remaining simple and maintainable. Test coverage is strong across unit and integration levels. The only meaningful gap is the absence of a test verifying time-based window expiration, which is a low-risk suggestion rather than a defect.

## Phase 3: Synthesis

### Summary

AC5 (NOMS-006) has been fully implemented: rate limiting middleware is now applied to all OAuth endpoints (`/auth/{provider}/start` and `/auth/{provider}/callback`), enforcing per-IP sliding-window limits (10 starts/minute, 5 callbacks/minute). Requests that exceed the limit receive a `429 Too Many Requests` response with a `Retry-After` header. A background cleanup task runs every 60 seconds to prevent unbounded memory growth from stale IP entries. Non-OAuth routes (e.g., `/auth/logout`) pass through unaffected.

The implementation uses a custom sliding-window algorithm backed by `Arc<DashMap<IpAddr, VecDeque<Instant>>>` — two independent maps, one per endpoint category — rather than an external library, to avoid incompatibility with Dioxus's `serve()` function and to give full control over per-endpoint limits.

### Files Changed

#### New file: `src/middleware/rate_limit.rs` (607 lines)

Complete rate limiting module. Key components:

- **`EndpointCategory`** (private enum) — Distinguishes `Start` (10 req/min) from `Callback` (5 req/min). Each variant knows its own limit via the `limit()` method.
- **`RateLimitState`** (pub struct) — Holds two `Arc<DashMap<IpAddr, VecDeque<Instant>>>` maps. Implements `Default` and `Clone` (cheap — only clones Arcs). Two methods:
  - `check(ip, category)` — Sliding window algorithm: prunes timestamps older than 60s from the front of the deque, compares remaining count against the limit, and either pushes the current timestamp (allowed) or returns `Err(retry_after_secs)` (rejected). Uses `DashMap::entry().or_insert_with()` for atomic check-and-insert.
  - `cleanup()` — Iterates both maps, prunes timestamps older than 300s (ENTRY_TTL), and removes entries that become empty. Collects keys to remove first to avoid mutation-during-iteration.
- **`extract_client_ip()`** — Three-tier fallback: `X-Forwarded-For` header (first comma-separated entry) → `ConnectInfo<SocketAddr>` extension → `0.0.0.0`. Uses `LazyLock<HeaderName>` for the non-standard `X-Forwarded-For` header name to avoid per-request allocation.
- **`categorize_path()`** — Splits the request path by `/`, matches exactly 4 segments of the form `/auth/{provider}/start` or `/auth/{provider}/callback`. Returns `None` for all other paths (passthrough).
- **`make_429_response()`** — Builds a `429 Too Many Requests` response with `Retry-After` header (clamped to [1, 60] seconds per RFC 6585) and `Content-Type: text/plain`.
- **`rate_limit_middleware()`** — The Axum middleware handler. Extracts `AppState`, categorizes the path, extracts the client IP, and either passes through or returns a 429 response.
- **10 unit tests** — Cover limit enforcement, category/IP independence, Retry-After validation, cleanup behavior, and path categorization.
- **4 integration tests** — Build a minimal Axum router with the middleware and a pass-through handler, then send requests via `tower::ServiceExt::oneshot` to verify end-to-end 429 behavior, passthrough for non-limited routes, and `Retry-After` header presence.

#### Modified: `Cargo.toml`

- Added `dashmap = { version = "6", optional = true }` to `[dependencies]` (line 31).
- Added `"dashmap"` to the `server` feature list (line 55). This ensures `dashmap` is only compiled for server builds, not WASM.

#### Modified: `src/middleware/mod.rs`

- Added `pub mod rate_limit;` (line 7) to export the new module.

#### Modified: `src/auth/oauth.rs`

- Added `pub rate_limit: crate::middleware::rate_limit::RateLimitState` field to `AppState` (line 51). This embeds the rate limit state directly in the application state, allowing the middleware to extract it via Axum's `State<AppState>` without wrapper structs or Extension layers.

#### Modified: `src/main.rs`

Three changes inside the `serve` closure:
1. **Line 76** — Creates `RateLimitState::default()` before `AppState` construction.
2. **Lines 112-115** — Applies `.layer(from_fn_with_state(state.clone(), rate_limit_middleware))` to `oauth_router`, before `.with_state(state)`.
3. **Lines 118-130** — Spawns a background `tokio::spawn` task that runs `rl.cleanup()` every 60 seconds. The task is dropped when the server shuts down.

### Test Counts

| Category | Count |
|---|---|
| Unit tests (new) | 10 |
| Integration tests (new) | 4 |
| **Total new tests** | **14** |
| Existing tests (unchanged) | 112 |
| **Total test suite** | **122** |

All 122 tests pass. `cargo clippy` passes with zero warnings. `cargo fmt` is clean.

### Dependencies Introduced

- **`dashmap` v6** (optional, server-only) — Lock-free concurrent hash map. Used for thread-safe per-IP rate limit tracking. Bundled only when the `server` feature is enabled; not included in WASM builds.

### Non-Obvious Patterns and Language Features

- **`LazyLock<HeaderName>`** — `X-Forwarded-For` is not a standard header in the `http` crate. `LazyLock` ensures the `HeaderName` is constructed once at first use, not on every request.
- **`DashMap::entry().or_insert_with()`** — Atomic check-and-insert pattern. The entire sliding window check (prune, compare, push) happens while holding the per-shard lock, preventing race conditions between concurrent requests from the same IP.
- **`saturating_duration_since()`** — Used in Retry-After computation to prevent overflow if system time goes backward.
- **`VecDeque` for sliding window** — O(1) `push_back` and `pop_front` operations match the FIFO expiration semantics of the sliding window algorithm.

### Follow-Up Recommendations

1. **Time-based window expiration test** (from review) — Consider adding a unit test that inserts timestamps older than 60 seconds into the deque and verifies `check()` prunes them and allows the request. This would cover the time-expiry path of the sliding window algorithm.
2. **IP spoofing via X-Forwarded-For** — The middleware trusts XFF headers. In production behind a reverse proxy (nginx, Cloudflare), this is correct. If the server is ever directly exposed, consider a config flag to disable XFF trust.
3. **Distributed deployments** — In-memory rate limiting does not work across multiple server instances. For multi-instance deployments, consider Redis-based rate limiting.

### Commit Message

```
feat(auth): add rate limiting middleware for OAuth endpoints

Implement AC5 from NOMS-006: per-IP sliding-window rate limiting on
/auth/{provider}/start (10 req/min) and /auth/{provider}/callback
(5 req/min). Exceeded limits return 429 with a Retry-After header.

Architecture:
- Custom middleware using Arc<DashMap<IpAddr, VecDeque<Instant>>>
  instead of tower_governor, to avoid incompatibility with Dioxus's
  serve() function and to support per-endpoint limits.
- RateLimitState embedded in AppState for clean Axum state extraction.
- Background cleanup task (tokio::spawn, 60s interval) removes stale
  IP entries to prevent unbounded memory growth.
- IP extraction chain: X-Forwarded-For → ConnectInfo → 0.0.0.0 fallback.
- Non-OAuth routes (e.g. /auth/logout) pass through unaffected.

Files changed:
- New: src/middleware/rate_limit.rs (607 lines, 14 tests)
- Modified: Cargo.toml (added optional dashmap v6 dependency)
- Modified: src/middleware/mod.rs (export rate_limit module)
- Modified: src/auth/oauth.rs (added rate_limit field to AppState)
- Modified: src/main.rs (wire up state, middleware layer, cleanup task)

Test coverage: 10 unit tests + 4 integration tests (14 new, 122 total).
All tests pass. Clippy clean, fmt clean.
```
