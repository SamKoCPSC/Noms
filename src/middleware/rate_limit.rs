//! Sliding-window rate limiting middleware for OAuth endpoints.
//!
//! Protects `/auth/{provider}/start` and `/auth/{provider}/callback` from
//! abuse by enforcing per-IP request limits within a 60-second window.
//!
//! Only compiled when the `server` feature is enabled.

#![cfg(feature = "server")]

use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::http::header::{self, HeaderMap, HeaderName, HeaderValue, RETRY_AFTER};
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use dashmap::DashMap;
use ipnet::IpNet;

use crate::auth::oauth::AppState;

// ── Constants ────────────────────────────────────────────────────────────────

/// Maximum OAuth start requests per IP per 60-second window.
const START_LIMIT_PER_MIN: usize = 10;

/// The `X-Forwarded-For` header name (not a standard header in the `http` crate).
static X_FORWARDED_FOR: LazyLock<HeaderName> =
    LazyLock::new(|| HeaderName::from_static("x-forwarded-for"));

/// Maximum OAuth callback requests per IP per 60-second window.
const CALLBACK_LIMIT_PER_MIN: usize = 5;

/// Sliding window duration in seconds.
const WINDOW_SECS: u64 = 60;

/// Time-to-live for map entries in seconds.
///
/// Entries with no timestamps within this window are removed by the cleanup
/// task to prevent unbounded memory growth.
const ENTRY_TTL_SECS: u64 = 300;

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

        Self {
            networks: Arc::new(networks),
        }
    }
}

/// Globally shared trusted proxy list, initialized once at first access.
static TRUSTED_PROXIES: LazyLock<TrustedProxyList> = LazyLock::new(TrustedProxyList::load);

// ── Endpoint categories ──────────────────────────────────────────────────────

/// Which category of OAuth endpoint a request targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EndpointCategory {
    /// `/auth/{provider}/start` — 10 requests/minute per IP.
    Start,
    /// `/auth/{provider}/callback` — 5 requests/minute per IP.
    Callback,
}

impl EndpointCategory {
    /// Return the per-IP request limit for this category.
    fn limit(self) -> usize {
        match self {
            Self::Start => START_LIMIT_PER_MIN,
            Self::Callback => CALLBACK_LIMIT_PER_MIN,
        }
    }
}

// ── Rate limit state ─────────────────────────────────────────────────────────

/// Shared in-memory rate limit store.
///
/// Maintains two independent sliding-window maps (one per endpoint category),
/// keyed by client IP address. Each value is a `VecDeque` of timestamps for
/// that IP within the current window.
///
/// Clone is cheap — it only clones the outer `Arc`s.
#[derive(Clone)]
pub struct RateLimitState {
    start_windows: Arc<DashMap<IpAddr, VecDeque<Instant>>>,
    callback_windows: Arc<DashMap<IpAddr, VecDeque<Instant>>>,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            start_windows: Arc::new(DashMap::new()),
            callback_windows: Arc::new(DashMap::new()),
        }
    }
}

impl RateLimitState {
    /// Check whether a request from `ip` in the given category is allowed.
    ///
    /// Returns `Ok(())` if the request is within the limit, or `Err(retry_after_secs)`
    /// if the limit has been exceeded. The error value is the number of seconds
    /// the client should wait before retrying (clamped to [1, 60]).
    ///
    /// The sliding window algorithm:
    /// 1. Prune all timestamps older than `WINDOW_SECS` from the front of the deque.
    /// 2. If deque length >= limit, reject with `Retry-After` based on the oldest entry.
    /// 3. Otherwise, push the current timestamp and allow.
    fn check(&self, ip: IpAddr, category: EndpointCategory) -> Result<(), u64> {
        let map = match category {
            EndpointCategory::Start => &self.start_windows,
            EndpointCategory::Callback => &self.callback_windows,
        };

        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(WINDOW_SECS);
        let limit = category.limit();

        let mut entry = map.entry(ip).or_insert_with(VecDeque::new);

        // Prune expired timestamps from the front
        while let Some(&front) = entry.front() {
            if front < cutoff {
                entry.pop_front();
            } else {
                break;
            }
        }

        if entry.len() >= limit {
            // Rate limited — compute retry-after from the oldest timestamp
            let oldest = entry.front().copied().unwrap_or(now);
            let wait = (oldest + std::time::Duration::from_secs(WINDOW_SECS))
                .saturating_duration_since(now)
                .as_secs();
            let retry_after = wait.clamp(1, 60);
            Err(retry_after)
        } else {
            entry.push_back(now);
            Ok(())
        }
    }

    /// Remove stale entries from both maps.
    ///
    /// An entry is considered stale if it has no timestamps within the last
    /// `ENTRY_TTL_SECS` seconds. This prevents unbounded memory growth from
    /// IPs that are no longer making requests.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let ttl_cutoff = now - std::time::Duration::from_secs(ENTRY_TTL_SECS);

        for map in [&self.start_windows, &self.callback_windows] {
            // Collect keys to remove (can't mutate while iterating)
            let mut to_remove = Vec::new();
            for mut entry in map.iter_mut() {
                // Prune old timestamps first
                while let Some(&front) = entry.value().front() {
                    if front < ttl_cutoff {
                        entry.pop_front();
                    } else {
                        break;
                    }
                }
                if entry.value().is_empty() {
                    to_remove.push(*entry.key());
                }
            }
            for key in to_remove {
                map.remove(&key);
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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

/// Categorize a request path into an endpoint category, if applicable.
///
/// Matches exactly 4-segment paths of the form `/auth/{provider}/start` or
/// `/auth/{provider}/callback`. Returns `None` for all other paths (e.g.
/// `/auth/logout`, `/dashboard`), allowing them to pass through without
/// rate limiting.
fn categorize_path(path: &str) -> Option<EndpointCategory> {
    let segments: Vec<&str> = path.split('/').collect();
    // /auth/{provider}/{action} => ["", "auth", "{provider}", "{action}"]
    if segments.len() != 4 {
        return None;
    }
    if segments[1] != "auth" {
        return None;
    }
    match segments[3] {
        "start" => Some(EndpointCategory::Start),
        "callback" => Some(EndpointCategory::Callback),
        _ => None,
    }
}

/// Build a 429 response with a `Retry-After` header.
fn make_429_response(retry_after_secs: u64) -> Response<Body> {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from(retry_after_secs));
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    (StatusCode::TOO_MANY_REQUESTS, headers, "Too Many Requests").into_response()
}

// ── Middleware ───────────────────────────────────────────────────────────────

/// Rate limiting middleware for OAuth endpoints.
///
/// Extracts the client IP, categorizes the request path, and enforces the
/// appropriate per-IP rate limit. Returns a 429 response if the limit is
/// exceeded, otherwise passes the request through to the next handler.
pub async fn rate_limit_middleware(
    State(app_state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = req.uri().path().to_string();

    // Categorize the path — if it's not a rate-limited endpoint, pass through
    let category = match categorize_path(&path) {
        Some(cat) => cat,
        None => return next.run(req).await,
    };

    let ip = extract_client_ip(&req);

    match app_state.rate_limit.check(ip, category) {
        Ok(()) => next.run(req).await,
        Err(retry_after) => make_429_response(retry_after),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── Unit tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_start_allows_up_to_limit() {
        let state = RateLimitState::default();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // First 10 requests should succeed
        for _ in 0..START_LIMIT_PER_MIN {
            assert!(state.check(ip, EndpointCategory::Start).is_ok());
        }
        // 11th request should be rejected
        assert!(state.check(ip, EndpointCategory::Start).is_err());
    }

    #[test]
    fn test_callback_allows_up_to_limit() {
        let state = RateLimitState::default();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // First 5 requests should succeed
        for _ in 0..CALLBACK_LIMIT_PER_MIN {
            assert!(state.check(ip, EndpointCategory::Callback).is_ok());
        }
        // 6th request should be rejected
        assert!(state.check(ip, EndpointCategory::Callback).is_err());
    }

    #[test]
    fn test_start_and_callback_independent() {
        let state = RateLimitState::default();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // Exhaust the start limit
        for _ in 0..START_LIMIT_PER_MIN {
            state.check(ip, EndpointCategory::Start).unwrap();
        }
        assert!(state.check(ip, EndpointCategory::Start).is_err());

        // Callback should still work (independent counter)
        for _ in 0..CALLBACK_LIMIT_PER_MIN {
            assert!(state.check(ip, EndpointCategory::Callback).is_ok());
        }
    }

    #[test]
    fn test_different_ips_independent() {
        let state = RateLimitState::default();
        let ip1 = IpAddr::from([127, 0, 0, 1]);
        let ip2 = IpAddr::from([192, 168, 1, 1]);

        // Exhaust ip1's start limit
        for _ in 0..START_LIMIT_PER_MIN {
            state.check(ip1, EndpointCategory::Start).unwrap();
        }
        assert!(state.check(ip1, EndpointCategory::Start).is_err());

        // ip2 should still have full quota
        for _ in 0..START_LIMIT_PER_MIN {
            assert!(state.check(ip2, EndpointCategory::Start).is_ok());
        }
    }

    #[test]
    fn test_retry_after_positive() {
        let state = RateLimitState::default();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // Exhaust the limit
        for _ in 0..START_LIMIT_PER_MIN {
            state.check(ip, EndpointCategory::Start).unwrap();
        }

        let err = state.check(ip, EndpointCategory::Start).unwrap_err();
        assert!(err >= 1, "retry_after should be >= 1, got {err}");
        assert!(err <= 60, "retry_after should be <= 60, got {err}");
    }

    #[test]
    fn test_cleanup_removes_empty() {
        let state = RateLimitState::default();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // Add an entry
        state.check(ip, EndpointCategory::Start).unwrap();
        assert_eq!(state.start_windows.len(), 1);

        // Manually advance time by artificially inserting an old timestamp
        // We do this by removing the entry, inserting one with a very old time,
        // then checking cleanup removes it.
        state.start_windows.remove(&ip);
        let mut old_deque = VecDeque::new();
        old_deque.push_back(Instant::now() - Duration::from_secs(600)); // 10 min ago
        state.start_windows.insert(ip, old_deque);

        // Cleanup should remove the stale entry
        state.cleanup();
        assert_eq!(state.start_windows.len(), 0);
    }

    #[test]
    fn test_cleanup_preserves_recent() {
        let state = RateLimitState::default();
        let ip = IpAddr::from([127, 0, 0, 1]);

        // Add a recent entry
        state.check(ip, EndpointCategory::Start).unwrap();
        assert_eq!(state.start_windows.len(), 1);

        // Cleanup should NOT remove it (it's recent)
        state.cleanup();
        assert_eq!(state.start_windows.len(), 1);
    }

    #[test]
    fn test_categorize_path_start() {
        assert_eq!(
            categorize_path("/auth/google/start"),
            Some(EndpointCategory::Start)
        );
        assert_eq!(
            categorize_path("/auth/github/start"),
            Some(EndpointCategory::Start)
        );
    }

    #[test]
    fn test_categorize_path_callback() {
        assert_eq!(
            categorize_path("/auth/google/callback"),
            Some(EndpointCategory::Callback)
        );
        assert_eq!(
            categorize_path("/auth/github/callback"),
            Some(EndpointCategory::Callback)
        );
    }

    #[test]
    fn test_categorize_path_non_oauth() {
        assert_eq!(categorize_path("/auth/logout"), None);
        assert_eq!(categorize_path("/dashboard"), None);
        assert_eq!(categorize_path("/"), None);
        assert_eq!(categorize_path("/auth/google/start/extra"), None);
        assert_eq!(categorize_path("/auth/google"), None);
    }

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

    // ── Integration tests ────────────────────────────────────────────────────

    mod integration_tests {
        use super::*;
        use crate::auth::oauth::build_oauth_clients;
        use crate::test_utils;
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        /// Build a test `AppState` with a real database pool.
        async fn make_test_state() -> AppState {
            let (_db, pool) = test_utils::setup_test_db().await;
            let (google_client, github_client) = build_oauth_clients("http://localhost:8080");
            AppState {
                pool,
                google_client,
                github_client,
                http_client: reqwest::Client::new(),
                rate_limit: RateLimitState::default(),
            }
        }

        /// Build a minimal router with the rate limit middleware and a pass-through handler.
        fn make_router(state: AppState) -> axum::Router {
            async fn passthrough_handler() -> &'static str {
                "ok"
            }

            axum::Router::new()
                .route(
                    "/auth/{provider}/start",
                    axum::routing::get(passthrough_handler),
                )
                .route(
                    "/auth/{provider}/callback",
                    axum::routing::get(passthrough_handler),
                )
                .route("/auth/logout", axum::routing::get(passthrough_handler))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    rate_limit_middleware,
                ))
                .with_state(state)
        }

        /// Build a router with `MockConnectInfo` set to a specific address.
        fn make_router_with_connect_info(state: AppState, mock_addr: SocketAddr) -> axum::Router {
            async fn passthrough_handler() -> &'static str {
                "ok"
            }

            // Middleware that injects a ConnectInfo<SocketAddr> extension into the request.
            // This is needed because MockConnectInfo is an extractor layer that only
            // provides the value during route extraction, not during middleware processing.
            let inject_mw =
                axum::middleware::from_fn(move |req: Request<Body>, next: Next| async move {
                    let mut req = req;
                    req.extensions_mut().insert(ConnectInfo(mock_addr));
                    next.run(req).await
                });

            axum::Router::new()
                .route(
                    "/auth/{provider}/start",
                    axum::routing::get(passthrough_handler),
                )
                .route(
                    "/auth/{provider}/callback",
                    axum::routing::get(passthrough_handler),
                )
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    rate_limit_middleware,
                ))
                .layer(inject_mw)
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
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/start")
                            .header(X_FORWARDED_FOR.as_str(), "198.51.100.1")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            // Different spoofed XFF should still be 429 (keyed on TCP IP)
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), "203.0.113.99")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "spoofed XFF should not bypass rate limit"
            );
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
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/start")
                            .header(X_FORWARDED_FOR.as_str(), real_client)
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            // Same client IP should be rate limited
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), real_client)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "real client IP from trusted proxy should be rate limited"
            );
            // Different client IP via same proxy should NOT be limited
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), "203.0.113.51")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "different client IP should have independent rate limit"
            );
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
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/start")
                            .header(X_FORWARDED_FOR.as_str(), xff_value)
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .header(X_FORWARDED_FOR.as_str(), xff_value)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "unwound client IP should be rate limited"
            );
        }

        #[tokio::test]
        async fn test_no_connect_info_falls_back_to_zero_ip() {
            // No ConnectInfo extension -- extract_client_ip returns 0.0.0.0
            let state = make_test_state().await;
            let app = make_router(state); // No MockConnectInfo

            for _ in 0..START_LIMIT_PER_MIN {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/start")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "should be rate limited via 0.0.0.0 fallback"
            );
        }

        #[tokio::test]
        async fn test_middleware_429_start_exceeded() {
            let state = make_test_state().await;
            let app = make_router(state);

            // Send 10 requests — all should succeed
            for _ in 0..START_LIMIT_PER_MIN {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/start")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }

            // 11th request should be 429
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            assert!(
                response.headers().contains_key(RETRY_AFTER),
                "429 response should have Retry-After header"
            );
        }

        #[tokio::test]
        async fn test_middleware_429_callback_exceeded() {
            let state = make_test_state().await;
            let app = make_router(state);

            // Send 5 requests — all should succeed
            for _ in 0..CALLBACK_LIMIT_PER_MIN {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/callback")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }

            // 6th request should be 429
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/callback")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        #[tokio::test]
        async fn test_middleware_passes_through_logout() {
            let state = make_test_state().await;
            let app = make_router(state);

            // /auth/logout is not rate limited — should always pass through
            for _ in 0..20 {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/logout")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(response.status(), StatusCode::OK);
            }
        }

        #[tokio::test]
        async fn test_middleware_retry_after_header() {
            let state = make_test_state().await;
            let app = make_router(state);

            // Exhaust the start limit
            for _ in 0..START_LIMIT_PER_MIN {
                let _ = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/auth/google/start")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
            }

            // Next request should be 429 with valid Retry-After
            let response = app
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri("/auth/google/start")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .expect("Retry-After header should be present");
            let retry_secs: u64 = retry_after
                .to_str()
                .expect("Retry-After should be a valid string")
                .parse()
                .expect("Retry-After should be a valid number");
            assert!(
                retry_secs >= 1,
                "Retry-After should be >= 1, got {retry_secs}"
            );
            assert!(
                retry_secs <= 60,
                "Retry-After should be <= 60, got {retry_secs}"
            );
        }
    }
}
