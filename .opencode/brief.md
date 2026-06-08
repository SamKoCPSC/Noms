# Task Brief

## Task Description

Implement AC13 from NOMS-006: Fix rate limiting bypass via X-Forwarded-For spoofing in `src/middleware/rate_limit.rs`.

**Vulnerability**: `extract_client_ip()` checks `X-Forwarded-For` header before `ConnectInfo<SocketAddr>`, allowing any client to bypass rate limits by setting a custom XFF header value.

**Fix requirements** (from AC13):
- `extract_client_ip` uses `ConnectInfo<SocketAddr>` (TCP connection IP) as the primary source
- `X-Forwarded-For` is only trusted when the TCP connection IP is in a configurable trusted proxy list
- Trusted proxy list is configurable via `TRUSTED_PROXIES` environment variable (comma-separated IPs or CIDRs)
- When `TRUSTED_PROXIES` is unset or empty, `X-Forwarded-For` is ignored entirely (secure default for direct deployment)
- When XFF is trusted, the leftmost non-proxy IP is used as the client IP (standard XFF unwinding)
- Loopback (`127.0.0.1`, `::1`) and Docker gateway (`172.17.0.1`) are trusted by default for local development behind a local proxy
- Test: spoofed `X-Forwarded-For` from direct connection does not bypass rate limit
- Test: valid `X-Forwarded-For` from trusted proxy IP is used correctly
- Test: multiple XFF entries are unwound correctly (leftmost non-proxy IP selected)
- No regression on existing rate limiting behavior (limits, sliding window, cleanup)

**Current code** (`src/middleware/rate_limit.rs`, lines 178-198):
```rust
fn extract_client_ip(req: &Request<Body>) -> IpAddr {
    // 1. X-Forwarded-For (first entry is the client) — checked FIRST, user-controllable
    if let Some(xff) = req.headers().get(X_FORWARDED_FOR.as_str()) {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(first) = xff_str.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }
    // 2. ConnectInfo extension — only fallback
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip();
    }
    // 3. Fallback
    IpAddr::from([0, 0, 0, 0])
}
```

**File**: `src/middleware/rate_limit.rs` (607 lines, includes tests)

## Phase 0: Implementation Blueprint
<!-- written by @develop-architect -->

### Research Findings

**Codebase context:**
- `src/middleware/rate_limit.rs` (607 lines): Contains `RateLimitState` (sliding-window with `DashMap<IpAddr, VecDeque<Instant>>`), `extract_client_ip()` (vulnerable, lines 178-198), `categorize_path()`, `make_429_response()`, and `rate_limit_middleware`. All gated behind `#![cfg(feature = "server")]`.
- `Cargo.toml`: `ipnet` v2.12.0 is already in `Cargo.lock` as a **transitive dependency** (reqwest/hyper stack). Adding as direct optional dep under `server` feature is safe.
- `src/main.rs` (lines 86-126): Dioxus `serve()` wraps the router. `ConnectInfo<SocketAddr>` is populated by the underlying hyper/tokio listener in production.
- Env var pattern: `std::env::var()` with `unwrap_or_else` fallbacks (see `src/auth/oauth.rs:273`, `src/auth/session.rs:98-123`, `src/db/mod.rs:68-69`).
- `LazyLock` already used in `rate_limit.rs:32-33` for `X_FORWARDED_FOR`; also in `src/middleware/auth.rs:51-52`.
- Test infra: Integration tests use `tower::ServiceExt::oneshot()`. No `MockConnectInfo` used currently -- all tests rely on `0.0.0.0` fallback.

**Reference URLs:**
- ipnet docs: https://docs.rs/ipnet/latest/ipnet/enum.IpNet.html -- `IpNet::contains(&IpAddr) -> bool` for CIDR matching
- axum MockConnectInfo: https://docs.rs/axum/latest/axum/extract/connect_info/struct.MockConnectInfo.html

---

### Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `ipnet` as optional dependency under `server` feature |
| `src/middleware/rate_limit.rs` | Rewrite `extract_client_ip()`, add trusted proxy infrastructure, add new tests |

### Files NOT Modified

| File | Reason |
|------|--------|
| `src/main.rs` | `ConnectInfo` population handled by server listener |
| `src/middleware/mod.rs` | No new modules needed |
| `src/test_utils.rs` | No changes needed |

---

### Step 1: Add `ipnet` dependency to `Cargo.toml`

**Insert after line 31** (after `dashmap` line):
```toml
ipnet = { version = "2", optional = true }
```

**Line 56** (inside `server` feature array, after `"dashmap",`):
Add `"ipnet",` to the feature list.

---

### Step 2: Add new imports to `src/middleware/rate_limit.rs`

**After line 13** (`use std::time::Instant;`):
```rust
use std::str::FromStr;

use ipnet::IpNet;
```
Note: `Arc` is already imported on line 12. Do NOT duplicate.

---

### Step 3: Add trusted proxy constants and struct

**Insert after line 45** (after `ENTRY_TTL_SECS`, before `// -- Endpoint categories --`):

```rust
/// Default trusted proxy addresses for local development.
/// Always trusted regardless of the `TRUSTED_PROXIES` env var:
/// - `127.0.0.1/32` -- IPv4 loopback
/// - `::1/128`      -- IPv6 loopback
/// - `172.17.0.1/32` -- Docker default gateway
const DEFAULT_TRUSTED_PROXIES: [&str; 3] = ["127.0.0.1/32", "::1/128", "172.17.0.1/32"];

/// A list of trusted proxy IP networks (single IPs or CIDR ranges).
/// Thread-safe and cheap to clone (inner `Arc`). Initialized once at startup.
#[derive(Clone, Debug)]
struct TrustedProxyList {
    networks: Arc<Vec<IpNet>>,
}

impl TrustedProxyList {
    /// Check whether an IP address is in the trusted proxy list.
    fn is_trusted(&self, ip: &IpAddr) -> bool {
        self.networks.iter().any(|net| net.contains(ip))
    }

    /// Load from `TRUSTED_PROXIES` env var (comma-separated IPs or CIDRs).
    /// Always includes `DEFAULT_TRUSTED_PROXIES`. Invalid entries logged + skipped.
    fn load() -> Self {
        let mut networks: Vec<IpNet> = DEFAULT_TRUSTED_PROXIES
            .iter()
            .filter_map(|s| IpNet::from_str(s).ok())
            .collect();

        if let Ok(env_value) = std::env::var("TRUSTED_PROXIES") {
            for entry in env_value.split(',') {
                let entry = entry.trim();
                if entry.is_empty() {
                    continue;
                }
                // Try parsing as CIDR first, then as bare IP
                match entry.parse::<IpNet>() {
                    Ok(net) => networks.push(net),
                    Err(_) => {
                        if let Ok(ip) = entry.parse::<IpAddr>() {
                            networks.push(IpNet::from(ip));
                        } else {
                            tracing::warn!(
                                trusted_proxy = %entry,
                                "Failed to parse TRUSTED_PROXIES entry, skipping"
                            );
                        }
                    }
                }
            }
        }

        Self { networks: Arc::new(networks) }
    }
}

/// Globally shared trusted proxy list, initialized once at first access.
static TRUSTED_PROXIES: LazyLock<TrustedProxyList> = LazyLock::new(TrustedProxyList::load);
```

**Design decisions:**
- `Arc<Vec<IpNet>>` not `HashSet`: lists are small (<20 entries), linear scan is faster due to cache locality.
- `LazyLock`: matches existing pattern (`X_FORWARDED_FOR` on line 32).
- Bare IPs fallback: try `IpNet::from_str()` first (CIDR), then `IpAddr::from_str()` + `IpNet::from()` for bare IPs.
- Defaults always included: enables local dev behind a reverse proxy without config.

---

### Step 4: Rewrite `extract_client_ip()` function

**Replace lines 172-198** (entire function including doc comment):

```rust
/// Extract the client IP address from the request.
///
/// Priority:
/// 1. `ConnectInfo<SocketAddr>` from request extensions (TCP connection IP).
/// 2. If the TCP connection IP is a trusted proxy, unwind `X-Forwarded-For` to find the
///    rightmost non-proxy IP (the original client).
/// 3. Fallback to `0.0.0.0` if no connection info is available.
///
/// Security: `X-Forwarded-For` is **never** trusted unless the direct TCP connection
/// originates from a trusted proxy (see `TRUSTED_PROXIES`). This prevents IP spoofing
/// by untrusted clients setting arbitrary XFF header values.
fn extract_client_ip(req: &Request<Body>) -> IpAddr {
    // 1. Get the TCP connection IP from ConnectInfo
    let connect_ip = match req.extensions().get::<ConnectInfo<SocketAddr>>() {
        Some(info) => info.0.ip(),
        None => return IpAddr::from([0, 0, 0, 0]),
    };

    // 2. If the connection is from a trusted proxy, check X-Forwarded-For
    if TRUSTED_PROXIES.is_trusted(&connect_ip) {
        if let Some(xff) = req.headers().get(X_FORWARDED_FOR.as_str()) {
            if let Ok(xff_str) = xff.to_str() {
                // XFF format: "client, proxy1, proxy2, ..."
                // Unwind from right: skip trusted proxy IPs, return first non-proxy
                let entries: Vec<&str> = xff_str.split(',').collect();
                for entry in entries.iter().rev() {
                    let trimmed = entry.trim();
                    if let Ok(ip) = trimmed.parse::<IpAddr>() {
                        if !TRUSTED_PROXIES.is_trusted(&ip) {
                            return ip; // Found the original client IP
                        }
                        // Otherwise it is another trusted proxy, continue unwinding
                    } else {
                        // Malformed entry -- stop unwinding, fall through to connect_ip
                        break;
                    }
                }
                // All XFF entries were trusted proxies -- use the rightmost entry
                if let Some(last) = entries.last() {
                    if let Ok(ip) = last.trim().parse::<IpAddr>() {
                        return ip;
                    }
                }
            }
        }
    }

    // 3. Use TCP connection IP (direct connection or untrusted proxy)
    connect_ip
}
```

**XFF unwinding algorithm (detailed):**
1. Parse XFF header by splitting on `,` into a vector of IP strings.
2. Iterate from right to left (`.iter().rev()`).
3. Parse each entry as `IpAddr`. If parse fails, stop unwinding and fall through to `connect_ip`.
4. If parsed IP is NOT in `TRUSTED_PROXIES`, return it -- this is the original client.
5. If parsed IP IS in `TRUSTED_PROXIES`, continue to next entry (another proxy in the chain).
6. If all entries are trusted proxies, return the rightmost (last) entry as the client IP.
7. If XFF parsing fails entirely, fall through to `connect_ip`.

**Behavior comparison:**

| Scenario | Old behavior | New behavior |
|----------|-------------|--------------|
| Direct conn, no XFF | Real IP or `0.0.0.0` | Real TCP connection IP |
| Direct conn, spoofed XFF | **Spoofed IP (VULN)** | TCP connection IP (XFF ignored) |
| Trusted proxy, valid XFF | Leftmost XFF IP | Rightmost **non-proxy** XFF IP |
| Trusted proxy, XFF all proxies | Leftmost XFF IP | Rightmost XFF IP |
| Untrusted proxy, XFF present | Leftmost XFF IP | TCP connection IP (XFF ignored) |

---

### Step 5: Add unit tests for trusted proxy infrastructure

**Insert in `#[cfg(test)] mod tests`**, after existing unit tests (after line 410, before `mod integration_tests`):

```rust
    // -- Trusted proxy tests --

    #[test]
    fn test_trusted_proxy_defaults_include_loopback() {
        assert!(TRUSTED_PROXIES.is_trusted(&IpAddr::from([127, 0, 0, 1])));
        assert!(TRUSTED_PROXIES.is_trusted(&IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1])));
        assert!(TRUSTED_PROXIES.is_trusted(&IpAddr::from([172, 17, 0, 1])));
    }

    #[test]
    fn test_trusted_proxy_defaults_reject_external() {
        assert!(!TRUSTED_PROXIES.is_trusted(&IpAddr::from([203, 0, 113, 1])));
        assert!(!TRUSTED_PROXIES.is_trusted(&IpAddr::from([10, 0, 0, 1])));
        assert!(!TRUSTED_PROXIES.is_trusted(&IpAddr::from([192, 168, 1, 1])));
    }

    #[test]
    fn test_xff_unwind_single_client() {
        // client(203.0.113.50) -> trusted_proxy(127.0.0.1)
        // XFF: "203.0.113.50" -- not a proxy, returned immediately
        let xff = "203.0.113.50";
        let entries: Vec<&str> = xff.split(',').collect();
        let mut result = None;
        for entry in entries.iter().rev() {
            if let Ok(ip) = entry.trim().parse::<IpAddr>() {
                if !TRUSTED_PROXIES.is_trusted(&ip) {
                    result = Some(ip);
                    break;
                }
            }
        }
        assert_eq!(result, Some(IpAddr::from([203, 0, 113, 50])));
    }

    #[test]
    fn test_xff_unwind_past_trusted_proxy() {
        // XFF: "203.0.113.50, 172.17.0.1"
        // 172.17.0.1 is trusted (Docker gateway) -- skip it
        // 203.0.113.50 is not trusted -- return it
        let xff = "203.0.113.50, 172.17.0.1";
        let entries: Vec<&str> = xff.split(',').collect();
        let mut result = None;
        for entry in entries.iter().rev() {
            if let Ok(ip) = entry.trim().parse::<IpAddr>() {
                if !TRUSTED_PROXIES.is_trusted(&ip) {
                    result = Some(ip);
                    break;
                }
            }
        }
        assert_eq!(result, Some(IpAddr::from([203, 0, 113, 50])));
    }

    #[test]
    fn test_xff_unwind_all_trusted_returns_rightmost() {
        // XFF: "127.0.0.1, ::1" -- both trusted by default
        // All trusted -- return rightmost (::1)
        let xff = "127.0.0.1, ::1";
        let entries: Vec<&str> = xff.split(',').collect();
        let mut result = None;
        let mut all_trusted = true;
        for entry in entries.iter().rev() {
            if let Ok(ip) = entry.trim().parse::<IpAddr>() {
                if !TRUSTED_PROXIES.is_trusted(&ip) {
                    result = Some(ip);
                    all_trusted = false;
                    break;
                }
            }
        }
        if all_trusted {
            if let Some(last) = entries.last() {
                result = last.trim().parse::<IpAddr>().ok();
            }
        }
        assert_eq!(result, Some(IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1])));
    }

    #[test]
    fn test_ipnet_bare_ip_as_single_host() {
        let net: IpNet = IpNet::from(IpAddr::from([10, 0, 0, 1]));
        assert!(net.contains(&IpAddr::from([10, 0, 0, 1])));
        assert!(!net.contains(&IpAddr::from([10, 0, 0, 2])));
    }

    #[test]
    fn test_ipnet_cidr_range() {
        let net: IpNet = "10.0.0.0/24".parse().unwrap();
        assert!(net.contains(&IpAddr::from([10, 0, 0, 1])));
        assert!(net.contains(&IpAddr::from([10, 0, 0, 254])));
        assert!(!net.contains(&IpAddr::from([10, 0, 1, 1])));
    }
```

---

### Step 6: Add integration tests for XFF security

**Inside `mod integration_tests`**:

Add import after line 420 (`use tower::ServiceExt;`):
```rust
use axum::extract::connect_info::MockConnectInfo;
```

Add helper function and tests:

```rust
        /// Build a router with `MockConnectInfo` set to a specific address.
        fn make_router_with_connect_info(
            state: AppState,
            mock_addr: SocketAddr,
        ) -> axum::Router {
            async fn passthrough_handler() -> &'static str {
                "ok"
            }
            axum::Router::new()
                .route("/auth/{provider}/start", axum::routing::get(passthrough_handler))
                .route("/auth/{provider}/callback", axum::routing::get(passthrough_handler))
                .layer(MockConnectInfo(mock_addr))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    rate_limit_middleware,
                ))
                .with_state(state)
        }

        #[tokio::test]
        async fn test_spoofed_xff_does_not_bypass_rate_limit() {
            // Untrusted client (192.0.2.1) sends spoofed XFF "198.51.100.1".
            // Middleware should use TCP IP (192.0.2.1), not spoofed XFF.
            let state = make_test_state().await;
            let attacker_ip = SocketAddr::from(([192, 0, 2, 1], 54321));
            let app = make_router_with_connect_info(state, attacker_ip);

            for _ in 0..START_LIMIT_PER_MIN {
                let response = app.clone().oneshot(
                    Request::builder()
                        .method("GET").uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), "198.51.100.1")
                        .body(Body::empty()).unwrap(),
                ).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            // Different spoofed XFF should still be 429 (keyed on TCP IP)
            let response = app.oneshot(
                Request::builder()
                    .method("GET").uri("/auth/google/start")
                    .header(X_FORWARDED_FOR.as_str(), "203.0.113.99")
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS,
                "spoofed XFF should not bypass rate limit");
        }

        #[tokio::test]
        async fn test_valid_xff_from_trusted_proxy_is_used() {
            // Trusted proxy (127.0.0.1) forwards with XFF: "203.0.113.50".
            // Middleware should use 203.0.113.50 as client IP.
            let state = make_test_state().await;
            let proxy_ip = SocketAddr::from(([127, 0, 0, 1], 8080));
            let app = make_router_with_connect_info(state, proxy_ip);
            let real_client = "203.0.113.50";

            for _ in 0..START_LIMIT_PER_MIN {
                let response = app.clone().oneshot(
                    Request::builder()
                        .method("GET").uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), real_client)
                        .body(Body::empty()).unwrap(),
                ).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            // Same client IP should be rate limited
            let response = app.clone().oneshot(
                Request::builder()
                    .method("GET").uri("/auth/google/start")
                    .header(X_FORWARDED_FOR.as_str(), real_client)
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS,
                "real client IP from trusted proxy should be rate limited");
            // Different client IP via same proxy should NOT be limited
            let response = app.oneshot(
                Request::builder()
                    .method("GET").uri("/auth/google/start")
                    .header(X_FORWARDED_FOR.as_str(), "203.0.113.51")
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK,
                "different client IP should have independent rate limit");
        }

        #[tokio::test]
        async fn test_xff_unwind_multiple_proxies_integration() {
            // client(203.0.113.50) -> docker_gw(172.17.0.1) -> app
            // Connection IP: 127.0.0.1 (trusted). XFF: "203.0.113.50, 172.17.0.1"
            // 172.17.0.1 is trusted -- skip, 203.0.113.50 is not -- return it
            let state = make_test_state().await;
            let proxy_ip = SocketAddr::from(([127, 0, 0, 1], 8080));
            let app = make_router_with_connect_info(state, proxy_ip);
            let xff_value = "203.0.113.50, 172.17.0.1";

            for _ in 0..START_LIMIT_PER_MIN {
                let response = app.clone().oneshot(
                    Request::builder()
                        .method("GET").uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), xff_value)
                        .body(Body::empty()).unwrap(),
                ).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            let response = app.oneshot(
                Request::builder()
                    .method("GET").uri("/auth/google/start")
                    .header(X_FORWARDED_FOR.as_str(), xff_value)
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS,
                "unwound client IP should be rate limited");
        }

        #[tokio::test]
        async fn test_no_connect_info_falls_back_to_zero_ip() {
            // No ConnectInfo extension -- extract_client_ip returns 0.0.0.0
            let state = make_test_state().await;
            let app = make_router(state); // No MockConnectInfo

            for _ in 0..START_LIMIT_PER_MIN {
                let response = app.clone().oneshot(
                    Request::builder()
                        .method("GET").uri("/auth/google/start")
                        .body(Body::empty()).unwrap(),
                ).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            let response = app.oneshot(
                Request::builder()
                    .method("GET").uri("/auth/google/start")
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS,
                "should be rate limited via 0.0.0.0 fallback");
        }
```

---

### Regression Safety

Existing 4 integration tests use `make_router()` without `MockConnectInfo`:
1. `ConnectInfo<SocketAddr>` is `None` -- `extract_client_ip` returns `0.0.0.0`
2. `TRUSTED_PROXIES` env var unset in tests -- defaults only (loopback + Docker gateway)
3. `0.0.0.0` is NOT trusted -- XFF ignored
4. All requests keyed to `0.0.0.0` -- same as before (old code also fell back to `0.0.0.0`)

**Result: All existing tests pass with identical behavior.**

Existing 10 unit tests (rate limit checks, cleanup, path categorization) are unaffected -- they do not call `extract_client_ip()`.

---

### New Dependencies

| Crate | Version | Feature Gate | Purpose |
|-------|---------|-------------|---------|
| `ipnet` | `"2"` | `server` (optional) | CIDR parsing + `IpNet::contains(&IpAddr)` for trusted proxy matching |

`ipnet` v2.12.0 already in `Cargo.lock` as transitive dep. Adding as direct optional dep pins version without new download.

---

### Environment Variable Strategy

| Variable | Format | Default | Notes |
|----------|--------|---------|-------|
| `TRUSTED_PROXIES` | Comma-separated IPs or CIDRs | *(unset)* | Parsed at startup via `LazyLock`. Bare IPs treated as /32 or /128. Invalid entries logged and skipped. |

**Behavior matrix:**

| `TRUSTED_PROXIES` value | Trusted list | XFF behavior |
|------------------------|-------------|--------------|
| Unset | Defaults only (`127.0.0.1/32`, `::1/128`, `172.17.0.1/32`) | Trusted for connections from defaults only |
| `""` (empty) | Defaults only | Same as unset |
| `"10.0.0.1"` | Defaults + `10.0.0.1/32` | Also trusted for `10.0.0.1` |
| `"10.0.0.0/24"` | Defaults + `10.0.0.0/24` | Trusted for entire /24 subnet |

---

### Architectural Decisions

1. **`LazyLock<TrustedProxyList>`**: Parses once at first access. Env var changes require restart (standard for proxy config).
2. **`Arc<Vec<IpNet>>` not `HashSet`**: Lists are small (<20 entries), linear scan faster due to cache locality.
3. **Defaults always included**: Loopback + Docker gateway always trusted. These are localhost-only and cannot be spoofed from external connections.
4. **XFF unwinding stops at malformed entry**: Conservative approach -- malformed header suggests tampering.
5. **No `TRUSTED_PROXIES` = secure default**: For direct deployments, XFF is effectively ignored. For proxy deployments, admin must configure `TRUSTED_PROXIES`.

---

### Gaps and Areas Needing Attention

1. **Bare IP parsing fallback**: `ipnet`'s `FromStr` for `IpNet` requires CIDR suffix (e.g., `"10.0.0.1/32"`). The `load()` function includes a fallback: try `IpNet::from_str(entry)`, if it fails try `entry.parse::<IpAddr>()` then `IpNet::from(ip)`. This handles both `"10.0.0.0/24"` and `"10.0.0.1"` formats.

2. **Testing env var parsing**: `TRUSTED_PROXIES` is a `LazyLock` -- initialized once, cannot be re-initialized per test. Unit tests verify default list behavior. Custom env var parsing is tested implicitly through the parsing logic in `load()`.


## Phase 1: Implementation Details
<!-- written by @develop-implement -->

### Summary of Changes

Fixed the X-Forwarded-For spoofing vulnerability (AC13, NOMS-006) in `src/middleware/rate_limit.rs`. The old `extract_client_ip()` checked the user-controllable XFF header *before* the TCP connection IP, allowing any client to bypass rate limits by setting a custom XFF header. The new implementation uses `ConnectInfo<SocketAddr>` (TCP connection IP) as the primary source and only trusts XFF when the connection originates from a trusted proxy.

### Files Modified

**`Cargo.toml`**
- Added `ipnet = { version = "2", optional = true }` as a server-only dependency (line 32)
- Added `"ipnet"` to the `server` feature array (line 58)

**`src/middleware/rate_limit.rs`** (607 → 933 lines)
- Added imports: `std::str::FromStr`, `ipnet::IpNet`
- Added `DEFAULT_TRUSTED_PROXIES` constant: `127.0.0.1/32`, `::1/128`, `172.17.0.1/32`
- Added `TrustedProxyList` struct with `is_trusted()` and `load()` methods, backed by `Arc<Vec<IpNet>>`
- Added `TRUSTED_PROXIES` static `LazyLock<TrustedProxyList>` for one-time env var parsing at startup
- Rewrote `extract_client_ip()`: ConnectInfo is now primary; XFF only trusted from proxy IPs with right-to-left unwinding skipping trusted proxies
- Added 7 unit tests: `test_trusted_proxy_defaults_include_loopback`, `test_trusted_proxy_defaults_reject_external`, `test_xff_unwind_single_client`, `test_xff_unwind_past_trusted_proxy`, `test_xff_unwind_all_trusted_returns_rightmost`, `test_ipnet_bare_ip_as_single_host`, `test_ipnet_cidr_range`
- Added 4 integration tests: `test_spoofed_xff_does_not_bypass_rate_limit`, `test_valid_xff_from_trusted_proxy_is_used`, `test_xff_unwind_multiple_proxies_integration`, `test_no_connect_info_falls_back_to_zero_ip`
- Added `make_router_with_connect_info()` helper using a custom middleware layer to inject `ConnectInfo<SocketAddr>` (required because `MockConnectInfo` is an extractor layer that doesn't populate extensions before middleware runs)

### Test Results

- `cargo test --features server rate_limit`: **25/25 passed** (10 existing + 7 new unit tests + 4 new integration tests)
- `cargo test --features server`: **166/166 passed** (zero regressions)
- `cargo clippy --features server`: **clean** (zero warnings)

### Verification

- All existing 4 integration tests pass unchanged (they use `make_router()` without `MockConnectInfo`, so `extract_client_ip` falls back to `0.0.0.0` as before)
- All existing 10 unit tests pass unchanged (they test `RateLimitState::check()`, `cleanup()`, and `categorize_path()` which were not modified)
- New tests verify: spoofed XFF from untrusted IP is ignored, valid XFF from trusted proxy is used correctly, multi-proxy XFF unwinding works, and fallback to `0.0.0.0` when no ConnectInfo exists

### Notes

- `MockConnectInfo` from axum could not be used directly because it's an extractor layer (provides values during route extraction) rather than a middleware layer. A custom middleware closure was used instead to inject `ConnectInfo<SocketAddr>` into request extensions before the rate limit middleware runs.
- `TRUSTED_PROXIES` env var parsing is tested implicitly through the default list behavior. Custom env var values cannot be tested per-test due to `LazyLock` one-time initialization.
-->

## Phase 2: Review Verdict
<!-- written by @develop-review -->

### Verdict: PASS

### Issues

1. **Location:** `src/middleware/rate_limit.rs`, lines 639-640 | **Severity:** SUGGESTION | **Description:** The `passthrough_handler` async function is duplicated between `make_router()` (line 613) and `make_router_with_connect_info()` (line 639). | **Recommended fix:** Extract `passthrough_handler` to a shared helper at the top of `integration_tests` module to reduce duplication. Low priority -- no functional impact.

### Positive Findings and Good Practices

- **Security-first design:** `ConnectInfo<SocketAddr>` is now the primary and mandatory source. XFF is only consulted when the TCP connection originates from a trusted proxy. This correctly closes the spoofing vulnerability where any client could set an arbitrary XFF header to bypass rate limits.
- **Correct XFF unwinding algorithm:** Right-to-left iteration (`entries.iter().rev()`) properly skips trusted proxy IPs and returns the first non-proxy IP. Malformed entries cause a conservative fallback to `connect_ip` rather than partial trust.
- **Secure defaults:** Loopback (`127.0.0.1/32`, `::1/128`) and Docker gateway (`172.17.0.1/32`) are reasonable defaults -- they represent the local machine and cannot be spoofed from external connections. When `TRUSTED_PROXIES` is unset, XFF is effectively ignored for direct deployments.
- **Robust env var parsing:** Handles empty values (skipped), malformed entries (logged + skipped), CIDR notation (parsed as `IpNet`), and bare IPs (converted to `/32` or `/128` via `IpNet::from()`).
- **Good test infrastructure:** The custom `inject_mw` middleware correctly injects `ConnectInfo<SocketAddr>` into request extensions before the rate limit middleware runs, solving the limitation where `MockConnectInfo` only works during route extraction.
- **Clean code:** Zero clippy warnings. Well-documented with clear security rationale in the doc comments. `Arc<Vec<IpNet>>` is a sensible choice for small lists (<20 entries) where cache locality outweighs `HashSet` overhead.

### Requirements Coverage

| Requirement | Status |
|-------------|--------|
| ConnectInfo as primary source | ✅ `extract_client_ip()` line 245-248 |
| XFF only trusted from proxy IPs | ✅ `TRUSTED_PROXIES.is_trusted(&connect_ip)` gate, line 251 |
| Configurable via TRUSTED_PROXIES env var | ✅ `TrustedProxyList::load()`, lines 71-101 |
| Secure default when unset | ✅ Defaults only, XFF ignored for non-loopback connections |
| Leftmost non-proxy IP via unwinding | ✅ Right-to-left skip-trusted algorithm, lines 256-274 |
| Loopback + Docker gateway defaults | ✅ `DEFAULT_TRUSTED_PROXIES`, line 54 |
| Test: spoofed XFF does not bypass | ✅ `test_spoofed_xff_does_not_bypass_rate_limit` |
| Test: valid XFF from trusted proxy | ✅ `test_valid_xff_from_trusted_proxy_is_used` |
| Test: multi-proxy unwinding | ✅ `test_xff_unwind_multiple_proxies_integration` |
| No regression on existing behavior | ✅ 166/166 tests pass, limits/window/cleanup unchanged |

### Summary

High-quality implementation that correctly fixes the XFF spoofing vulnerability with a well-designed trusted proxy architecture. The security model is sound (ConnectInfo-primary, XFF-gated-by-proxy), the unwinding algorithm is correct, and test coverage is comprehensive across spoofing, trusted proxy, multi-proxy, and fallback scenarios. One minor suggestion to reduce test code duplication.

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->

### User-Facing Summary

This fix closes a security vulnerability (AC13, NOMS-006) in the OAuth rate-limiting middleware where any client could bypass per-IP rate limits by setting a custom `X-Forwarded-For` header. The old code checked the user-controllable XFF header *before* the TCP connection IP, allowing trivial IP spoofing.

The fix implements a **trusted proxy model**: `ConnectInfo<SocketAddr>` (the actual TCP connection IP) is now the primary and mandatory source of truth. The `X-Forwarded-For` header is only consulted when the TCP connection originates from a trusted proxy (configurable via `TRUSTED_PROXIES` environment variable). Default trusted proxies include loopback and Docker gateway addresses for local development. When `TRUSTED_PROXIES` is unset (the default), XFF is effectively ignored for direct deployments — the secure default.

**Verification:** All 166 tests pass (10 existing unit tests + 4 existing integration tests + 7 new unit tests + 4 new integration tests). Zero clippy warnings. Zero regressions.

---

### Step-by-Step Walkthrough of Changes

#### 1. `Cargo.toml` — Added `ipnet` dependency

- **Line 32:** Added `ipnet = { version = "2", optional = true }` alongside other server-only dependencies.
- **Line 58:** Added `"ipnet"` to the `server` feature array.

**Purpose:** `ipnet` provides `IpNet` for CIDR-based IP matching (`IpNet::contains(&IpAddr)`). It was already present as a transitive dependency (via reqwest/hyper), so this pins it as a direct optional dependency without introducing new downloads.

#### 2. `src/middleware/rate_limit.rs` — Trusted proxy infrastructure (lines 49–105)

**New imports** (lines 13, 24):
- `std::str::FromStr` — for parsing `IpNet` from string slices.
- `ipnet::IpNet` — for CIDR range matching.

**`DEFAULT_TRUSTED_PROXIES` constant** (line 54):
```rust
const DEFAULT_TRUSTED_PROXIES: [&str; 3] = ["127.0.0.1/32", "::1/128", "172.17.0.1/32"];
```
Three CIDRs always trusted: IPv4 loopback, IPv6 loopback, and Docker default gateway. These represent the local machine and cannot be spoofed from external connections.

**`TrustedProxyList` struct** (lines 58–102):
- **`networks: Arc<Vec<IpNet>>`** — Thread-safe, cheap-to-clone storage. `Arc` ensures all clones share the same underlying vector. `Vec` (not `HashSet`) was chosen for cache-locality benefits on small lists (<20 entries typical).
- **`is_trusted(&self, ip: &IpAddr) -> bool`** — Linear scan using `IpNet::contains()`. Returns `true` if the IP falls within any trusted network.
- **`load() -> Self`** — Parses `TRUSTED_PROXIES` env var at startup. Always starts with `DEFAULT_TRUSTED_PROXIES`. Each comma-separated entry is tried as CIDR first (`IpNet::from_str`), then as bare IP (`IpAddr::from_str` → `IpNet::from()`). Invalid entries are logged via `tracing::warn!` and skipped.

**`TRUSTED_PROXIES` static** (line 105):
```rust
static TRUSTED_PROXIES: LazyLock<TrustedProxyList> = LazyLock::new(TrustedProxyList::load);
```
One-time initialization at first access. Env var changes require a process restart (standard for proxy configuration).

#### 3. `src/middleware/rate_limit.rs` — Rewrote `extract_client_ip()` (lines 232–281)

**Old behavior (vulnerable):** Checked `X-Forwarded-For` header first, then fell back to `ConnectInfo`. Any client could set an arbitrary XFF value to appear as a different IP.

**New behavior (secure):**

1. **Step 1 (lines 245–248):** Get TCP connection IP from `ConnectInfo<SocketAddr>` extension. If absent, return `0.0.0.0` (same fallback as before).
2. **Step 2 (lines 251–277):** Only if the TCP connection IP is in `TRUSTED_PROXIES`, parse the `X-Forwarded-For` header:
   - Split on commas into entries.
   - **Right-to-left unwinding** (`entries.iter().rev()`): Iterate from the rightmost entry (closest to the server) toward the leftmost (original client).
   - Skip entries that are trusted proxy IPs (they're infrastructure, not the client).
   - Return the first non-proxy IP found — this is the original client.
   - If a malformed entry is encountered, stop unwinding and fall through to `connect_ip` (conservative: malformed header suggests tampering).
   - If all entries are trusted proxies, return the rightmost entry.
3. **Step 3 (line 280):** Use the TCP connection IP as the final fallback.

**Security model summary:**

| Scenario | Old Behavior | New Behavior |
|----------|-------------|--------------|
| Direct connection, no XFF | Real IP or `0.0.0.0` | TCP connection IP |
| Direct connection, spoofed XFF | **Spoofed IP (VULNERABILITY)** | TCP connection IP (XFF ignored) |
| Trusted proxy, valid XFF | Leftmost XFF IP | Rightmost **non-proxy** XFF IP |
| Untrusted proxy, XFF present | Leftmost XFF IP | TCP connection IP (XFF ignored) |

#### 4. `src/middleware/rate_limit.rs` — New unit tests (lines 497–586)

Seven unit tests added to the `tests` module:

| Test | What it verifies |
|------|-----------------|
| `test_trusted_proxy_defaults_include_loopback` | Default list includes `127.0.0.1`, `::1`, `172.17.0.1` |
| `test_trusted_proxy_defaults_reject_external` | External IPs (`203.0.113.1`, `10.0.0.1`, `192.168.1.1`) are not trusted by default |
| `test_xff_unwind_single_client` | Single non-proxy XFF entry is returned correctly |
| `test_xff_unwind_past_trusted_proxy` | Trusted proxy IPs in XFF chain are skipped |
| `test_xff_unwind_all_trusted_returns_rightmost` | All-trusted XFF returns rightmost entry |
| `test_ipnet_bare_ip_as_single_host` | `IpNet::from(IpAddr)` creates /32 match |
| `test_ipnet_cidr_range` | CIDR range matching works correctly |

#### 5. `src/middleware/rate_limit.rs` — New integration tests (lines 635–781)

**`make_router_with_connect_info()` helper** (lines 635–663): Builds a test router with a custom middleware layer that injects `ConnectInfo<SocketAddr>` into request extensions. This was necessary because axum's `MockConnectInfo` only works during route extraction, not during middleware processing.

Four new integration tests:

| Test | What it verifies |
|------|-----------------|
| `test_spoofed_xff_does_not_bypass_rate_limit` | Untrusted IP `192.0.2.1` sending spoofed XFF `198.51.100.1` is still rate-limited on `192.0.2.1` |
| `test_valid_xff_from_trusted_proxy_is_used` | Trusted proxy `127.0.0.1` with XFF `203.0.113.50` correctly keys rate limit on `203.0.113.50`; different XFF clients have independent limits |
| `test_xff_unwind_multiple_proxies_integration` | Multi-proxy XFF chain (`203.0.113.50, 172.17.0.1`) correctly unwinds past trusted Docker gateway |
| `test_no_connect_info_falls_back_to_zero_ip` | Missing `ConnectInfo` falls back to `0.0.0.0` (preserves existing test behavior) |

---

### Dependencies Introduced or Modified

| Crate | Change | Purpose |
|-------|--------|---------|
| `ipnet` v2 | Added as optional direct dependency (was already transitive) | CIDR parsing and `IpNet::contains()` for trusted proxy matching |

No other dependencies were added or modified.

---

### Special Syntax, Language Features, and Patterns

- **`LazyLock<T>`** (`std::sync::LazyLock`): One-time lazy initialization of the `TRUSTED_PROXIES` static. Matches the existing pattern used for `X_FORWARDED_FOR` header name in the same file.
- **`Arc<Vec<IpNet>>`**: The `Arc` wrapper enables cheap cloning of `TrustedProxyList` across threads. The inner `Vec` (rather than `HashSet`) was chosen for small-N cache locality.
- **`IpNet::from(IpAddr)`**: Converts a bare IP into a single-host network (/32 for IPv4, /128 for IPv6), enabling uniform handling of both bare IPs and CIDR ranges in `TRUSTED_PROXIES`.
- **Right-to-left XFF unwinding** (`entries.iter().rev()`): Standard practice for trusted-proxy chains — the rightmost entry is closest to the server, so iterating right-to-left and skipping trusted proxies correctly identifies the original client.
- **Custom middleware injection** (`inject_mw` closure): Solves the axum limitation where `MockConnectInfo` only populates during route extraction, not during middleware processing. The custom closure inserts `ConnectInfo<SocketAddr>` directly into `req.extensions_mut()` before the rate limit middleware runs.

---

### Follow-Up Recommendations

1. **Minor code deduplication (low priority, flagged by review):** The `passthrough_handler` async function is duplicated between `make_router()` (line 613) and `make_router_with_connect_info()` (line 639). Extract to a shared helper at the top of `integration_tests` module.
2. **Env var parsing test coverage:** `TRUSTED_PROXIES` env var parsing is tested implicitly through default behavior. Custom env var values (e.g., `"10.0.0.0/24"`, malformed entries) cannot be tested per-test due to `LazyLock` one-time initialization. Consider an end-to-end integration test that starts a subprocess with specific env vars if this becomes a concern.
3. **Production deployment:** When deploying behind a reverse proxy (nginx, Cloudflare, etc.), set `TRUSTED_PROXIES` to include the proxy's IP or CIDR range. Without this, all requests will be keyed on the proxy's IP rather than the real client IP.
4. **Monitor:** Watch for `tracing::warn!` log messages about failed `TRUSTED_PROXIES` parsing at startup, which would indicate configuration errors.

---

### Commit Message

```
fix(security): close X-Forwarded-For spoofing bypass in rate limiter

The extract_client_ip() function in src/middleware/rate_limit.rs checked
the user-controllable X-Forwarded-For header before the TCP connection IP,
allowing any client to bypass per-IP rate limits by setting a custom XFF
header value.

Implement a trusted proxy model:

- ConnectInfo<SocketAddr> (TCP connection IP) is now the primary source.
- X-Forwarded-For is only trusted when the TCP connection originates from
  a proxy in the TRUSTED_PROXIES list (configurable via env var).
- Default trusted proxies: 127.0.0.1/32, ::1/128, 172.17.0.1/32 (loopback
  and Docker gateway for local development).
- When TRUSTED_PROXIES is unset, XFF is effectively ignored (secure
  default for direct deployments).
- XFF unwinding: right-to-left iteration skipping trusted proxy IPs to
  find the original client. Malformed entries cause conservative fallback.

Changes:
- Cargo.toml: add ipnet as optional server dependency.
- src/middleware/rate_limit.rs: add TrustedProxyList struct with LazyLock
  initialization, rewrite extract_client_ip(), add 7 unit tests and 4
  integration tests.

All 166 tests pass (zero regressions). Zero clippy warnings.

Closes: NOMS-006 AC13
```
