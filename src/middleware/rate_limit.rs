//! Sliding-window rate limiting middleware for OAuth endpoints.
//!
//! Protects `/auth/{provider}/start` and `/auth/{provider}/callback` from
//! abuse by enforcing per-IP request limits within a 60-second window.
//!
//! Only compiled when the `server` feature is enabled.

#![cfg(feature = "server")]

use std::collections::VecDeque;
use std::net::{IpAddr, SocketAddr};
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

/// The client address header supplied by Railway and some other reverse proxies.
static X_REAL_IP: LazyLock<HeaderName> = LazyLock::new(|| HeaderName::from_static("x-real-ip"));

/// Internal header overwritten from trusted connection metadata before a
/// request reaches a server function. Clients cannot choose this value.
pub(crate) static RESOLVED_CLIENT_IP: LazyLock<HeaderName> =
    LazyLock::new(|| HeaderName::from_static("x-noms-resolved-client-ip"));

/// Maximum OAuth callback requests per IP per 60-second window.
const CALLBACK_LIMIT_PER_MIN: usize = 5;

/// Sliding window duration in seconds.
const WINDOW_SECS: u64 = 60;

/// Time-to-live for map entries in seconds.
///
/// Entries with no timestamps within this window are removed by the cleanup
/// task to prevent unbounded memory growth.
const ENTRY_TTL_SECS: u64 = 300;

/// A list of trusted proxy IP networks (single IPs or CIDR ranges).
///
/// The default is deliberately empty. A reverse proxy may influence client
/// identity only when its direct connection address is explicitly configured.
#[derive(Clone, Debug, Default)]
struct TrustedProxyList {
    networks: Arc<Vec<IpNet>>,
}

impl TrustedProxyList {
    /// Check whether an IP address is in the trusted proxy list.
    fn is_trusted(&self, ip: &IpAddr) -> bool {
        self.networks.iter().any(|net| net.contains(ip))
    }

    /// Strictly parse comma-separated IP addresses or CIDR ranges.
    fn parse(value: &str) -> Result<Self, String> {
        if value.trim().is_empty() {
            return Ok(Self::default());
        }

        let mut networks = Vec::new();
        for raw_entry in value.split(',') {
            let entry = raw_entry.trim();
            if entry.is_empty() {
                return Err("TRUSTED_PROXIES contains an empty entry".to_string());
            }

            let network = entry
                .parse::<IpNet>()
                .or_else(|_| entry.parse::<IpAddr>().map(IpNet::from))
                .map_err(|_| format!("invalid trusted proxy address or CIDR: {entry}"))?;
            networks.push(network);
        }

        Ok(Self {
            networks: Arc::new(networks),
        })
    }
}

fn load_trusted_proxy_configuration(
    configured: Option<&str>,
    proxy_hosted_runtime: bool,
) -> Result<TrustedProxyList, String> {
    if proxy_hosted_runtime && configured.is_none_or(|value| value.trim().is_empty()) {
        return Err(
            "TRUSTED_PROXIES must be explicitly configured on a proxy-hosted runtime".to_string(),
        );
    }

    configured.map_or_else(|| Ok(TrustedProxyList::default()), TrustedProxyList::parse)
}

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
    trusted_proxies: TrustedProxyList,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            start_windows: Arc::new(DashMap::new()),
            callback_windows: Arc::new(DashMap::new()),
            trusted_proxies: TrustedProxyList::default(),
        }
    }
}

impl RateLimitState {
    /// Build the limiter with an explicit proxy trust boundary.
    fn with_trusted_proxies(trusted_proxies: TrustedProxyList) -> Self {
        Self {
            trusted_proxies,
            ..Self::default()
        }
    }

    /// Load the explicit proxy trust boundary from `TRUSTED_PROXIES`.
    ///
    /// Invalid configuration is an error rather than a partially applied list,
    /// because silently changing the trust boundary can merge users into a
    /// proxy-wide bucket or make forwarding headers spoofable.
    pub fn from_environment() -> Result<Self, String> {
        let configured = match std::env::var("TRUSTED_PROXIES") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err("TRUSTED_PROXIES is not valid Unicode".to_string());
            }
        };
        let proxy_hosted_runtime = std::env::var_os("RAILWAY_PROJECT_ID").is_some()
            || std::env::var_os("RAILWAY_ENVIRONMENT_ID").is_some();
        let trusted_proxies =
            load_trusted_proxy_configuration(configured.as_deref(), proxy_hosted_runtime)?;

        Ok(Self::with_trusted_proxies(trusted_proxies))
    }

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
/// An untrusted direct peer is always its own identity, regardless of forwarding
/// headers. For an explicitly trusted peer, all supplied forwarding headers are
/// parsed strictly and must resolve to the same rightmost non-proxy address.
///
/// Missing peer information or an unusable trusted-proxy chain returns `None`,
/// causing rate-limited endpoints to fail closed instead of sharing a fallback.
fn extract_client_ip(req: &Request<Body>, trusted_proxies: &TrustedProxyList) -> Option<IpAddr> {
    let direct_peer = normalize_ip(req.extensions().get::<ConnectInfo<SocketAddr>>()?.0.ip());

    if !trusted_proxies.is_trusted(&direct_peer) {
        return Some(direct_peer);
    }

    let forwarded_for = parse_x_forwarded_for(req.headers(), trusted_proxies).ok()?;
    let real_ip = parse_single_ip_header(req.headers(), &X_REAL_IP).ok()?;

    match (forwarded_for, real_ip) {
        (Some(forwarded), Some(real)) if forwarded == real => Some(forwarded),
        (Some(_), Some(_)) => None,
        (Some(forwarded), None) => Some(forwarded),
        (None, Some(real)) if !trusted_proxies.is_trusted(&real) => Some(real),
        _ => None,
    }
}

fn parse_x_forwarded_for(
    headers: &HeaderMap,
    trusted_proxies: &TrustedProxyList,
) -> Result<Option<IpAddr>, ()> {
    let Some(value) = exactly_one_header(headers, &X_FORWARDED_FOR)? else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| ())?;
    let entries: Vec<IpAddr> = value
        .split(',')
        .map(|entry| entry.trim().parse::<IpAddr>().map(normalize_ip))
        .collect::<Result<_, _>>()
        .map_err(|_| ())?;

    if entries.is_empty() {
        return Err(());
    }

    Ok(entries
        .into_iter()
        .rev()
        .find(|ip| !trusted_proxies.is_trusted(ip)))
}

fn parse_single_ip_header(headers: &HeaderMap, name: &HeaderName) -> Result<Option<IpAddr>, ()> {
    let Some(value) = exactly_one_header(headers, name)? else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| ())?.trim();
    if value.is_empty() || value.contains(',') {
        return Err(());
    }
    value
        .parse::<IpAddr>()
        .map(normalize_ip)
        .map(Some)
        .map_err(|_| ())
}

fn exactly_one_header<'a>(
    headers: &'a HeaderMap,
    name: &HeaderName,
) -> Result<Option<&'a HeaderValue>, ()> {
    let mut values = headers.get_all(name).iter();
    let first = values.next();
    if values.next().is_some() {
        return Err(());
    }
    Ok(first)
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ipv6) => ipv6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(ipv6)),
        IpAddr::V4(_) => ip,
    }
}

/// Build a fail-closed response when no unique client identity is available.
fn make_503_response() -> Response<Body> {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    (
        StatusCode::SERVICE_UNAVAILABLE,
        headers,
        "Client identity unavailable",
    )
        .into_response()
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

    let Some(ip) = extract_client_ip(&req, &app_state.rate_limit.trusted_proxies) else {
        tracing::warn!(
            path,
            "OAuth request rejected because no trustworthy client identity was available"
        );
        return make_503_response();
    };

    match app_state.rate_limit.check(ip, category) {
        Ok(()) => next.run(req).await,
        Err(retry_after) => make_429_response(retry_after),
    }
}

/// Propagate the already-normalized client identity into Fullstack request
/// parts, whose extension propagation is not reliable across all Dioxus paths.
pub async fn resolved_client_ip_middleware(
    State(rate_limit): State<RateLimitState>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    req.headers_mut().remove(&*RESOLVED_CLIENT_IP);
    if let Some(ip) = extract_client_ip(&req, &rate_limit.trusted_proxies) {
        if let Ok(value) = HeaderValue::from_str(&ip.to_string()) {
            req.headers_mut().insert(RESOLVED_CLIENT_IP.clone(), value);
        }
    }
    next.run(req).await
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
    fn test_trusted_proxy_defaults_are_empty() {
        let trusted_proxies = TrustedProxyList::default();
        assert!(!trusted_proxies.is_trusted(&IpAddr::from([127, 0, 0, 1])));
        assert!(!trusted_proxies.is_trusted(&IpAddr::from([172, 17, 0, 1])));
    }

    #[test]
    fn test_trusted_proxy_defaults_reject_external() {
        let trusted_proxies = TrustedProxyList::default();
        assert!(!trusted_proxies.is_trusted(&IpAddr::from([203, 0, 113, 1])));
        assert!(!trusted_proxies.is_trusted(&IpAddr::from([10, 0, 0, 1])));
        assert!(!trusted_proxies.is_trusted(&IpAddr::from([192, 168, 1, 1])));
    }

    #[test]
    fn test_proxy_hosted_runtime_requires_explicit_trusted_proxy_configuration() {
        assert!(load_trusted_proxy_configuration(None, true).is_err());
        assert!(load_trusted_proxy_configuration(Some(""), true).is_err());
        assert!(load_trusted_proxy_configuration(Some("100.0.0.0/8"), true).is_ok());
    }

    #[test]
    fn test_direct_runtime_allows_empty_trusted_proxy_configuration() {
        let trusted_proxies = load_trusted_proxy_configuration(None, false).unwrap();
        assert!(!trusted_proxies.is_trusted(&IpAddr::from([127, 0, 0, 1])));
    }

    #[test]
    fn test_xff_unwind_single_client() {
        let trusted_proxies = TrustedProxyList::parse("127.0.0.1/32").unwrap();
        let request = Request::builder()
            .header(X_FORWARDED_FOR.as_str(), "203.0.113.50")
            .body(Body::empty())
            .unwrap();
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

        assert_eq!(
            extract_client_ip(&request, &trusted_proxies),
            Some(IpAddr::from([203, 0, 113, 50]))
        );
    }

    #[test]
    fn test_xff_unwind_past_trusted_proxy() {
        // XFF: "203.0.113.50, 172.17.0.1"
        // 172.17.0.1 is trusted (Docker gateway) -- skip it
        // 203.0.113.50 is not trusted -- return it
        let trusted_proxies = TrustedProxyList::parse("127.0.0.1/32,172.17.0.1/32").unwrap();
        let request = Request::builder()
            .header(X_FORWARDED_FOR.as_str(), "203.0.113.50, 172.17.0.1")
            .body(Body::empty())
            .unwrap();
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

        assert_eq!(
            extract_client_ip(&request, &trusted_proxies),
            Some(IpAddr::from([203, 0, 113, 50]))
        );
    }

    #[test]
    fn test_xff_unwind_all_trusted_rejects_missing_client_identity() {
        let trusted_proxies = TrustedProxyList::parse("127.0.0.1/32,::1/128").unwrap();
        let request = Request::builder()
            .header(X_FORWARDED_FOR.as_str(), "127.0.0.1, ::1")
            .body(Body::empty())
            .unwrap();
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

        assert_eq!(extract_client_ip(&request, &trusted_proxies), None);
    }

    #[test]
    fn test_trusted_proxy_accepts_x_real_ip() {
        let trusted_proxies = TrustedProxyList::parse("127.0.0.1/32").unwrap();
        let request = Request::builder()
            .header("x-real-ip", "203.0.113.60")
            .body(Body::empty())
            .unwrap();
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

        assert_eq!(
            extract_client_ip(&request, &trusted_proxies),
            Some(IpAddr::from([203, 0, 113, 60]))
        );
    }

    #[test]
    fn test_conflicting_trusted_proxy_headers_fail_closed() {
        let trusted_proxies = TrustedProxyList::parse("127.0.0.1/32").unwrap();
        let request = Request::builder()
            .header(X_FORWARDED_FOR.as_str(), "203.0.113.60")
            .header("x-real-ip", "203.0.113.61")
            .body(Body::empty())
            .unwrap();
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 8080))));

        assert_eq!(extract_client_ip(&request, &trusted_proxies), None);
    }

    #[test]
    fn test_untrusted_peer_cannot_spoof_forwarded_headers() {
        let trusted_proxies = TrustedProxyList::parse("10.0.0.0/8").unwrap();
        let direct_peer = IpAddr::from([192, 0, 2, 5]);
        let request = Request::builder()
            .header(X_FORWARDED_FOR.as_str(), "203.0.113.60")
            .header("x-real-ip", "203.0.113.60")
            .body(Body::empty())
            .unwrap();
        let mut request = request;
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::new(direct_peer, 8080)));

        assert_eq!(
            extract_client_ip(&request, &trusted_proxies),
            Some(direct_peer)
        );
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
        use tokio::sync::oneshot;
        use tower::ServiceExt;

        /// Build a test `AppState` with a real database pool.
        async fn make_test_state() -> AppState {
            make_test_state_with_rate_limit(RateLimitState::default()).await
        }

        async fn make_test_state_with_rate_limit(rate_limit: RateLimitState) -> AppState {
            let (_database, state) = make_test_state_with_database(rate_limit).await;
            state
        }

        async fn make_test_state_with_database(
            rate_limit: RateLimitState,
        ) -> (pgtemp::PgTempDB, AppState) {
            let (database, pool) = test_utils::setup_test_db().await;
            let (google_client, github_client) = build_oauth_clients("http://localhost:8080");

            (
                database,
                AppState {
                    pool,
                    base_url_origin: url::Url::parse("http://localhost:8080").unwrap().origin(),
                    auth_security_metrics: crate::auth::oauth::AuthSecurityMetrics::default(),
                    google_client,
                    github_client,
                    http_client: reqwest::Client::new(),
                    google_userinfo_url: "http://localhost:8082/google/userinfo".to_string(),
                    github_userinfo_url: "http://localhost:8082/github/userinfo".to_string(),
                    github_emails_url: "http://localhost:8082/github/emails".to_string(),
                    github_extra_scopes: Vec::new(),
                    rate_limit,
                    storage: crate::storage::StorageClient::for_test(),
                },
            )
        }

        async fn spawn_production_oauth_server(
            rate_limit: RateLimitState,
        ) -> (SocketAddr, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
            let (database, state) = make_test_state_with_database(rate_limit).await;
            let router = crate::build_oauth_router(state);
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener.local_addr().unwrap();
            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            let server = tokio::spawn(async move {
                let _database = database;
                crate::serve_router_with_connect_info(listener, router, async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("production server path should run");
            });

            (address, shutdown_tx, server)
        }

        fn oauth_http_client() -> reqwest::Client {
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap()
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
            let state = make_test_state_with_rate_limit(RateLimitState::with_trusted_proxies(
                TrustedProxyList::parse("127.0.0.1/32").unwrap(),
            ))
            .await;
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
            let state = make_test_state_with_rate_limit(RateLimitState::with_trusted_proxies(
                TrustedProxyList::parse("127.0.0.1/32,172.17.0.1/32").unwrap(),
            ))
            .await;
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
        async fn test_no_connect_info_fails_closed_without_global_bucket() {
            let state = make_test_state().await;
            let app = make_router(state); // No MockConnectInfo

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
                StatusCode::SERVICE_UNAVAILABLE,
                "missing peer identity must fail closed instead of sharing a global bucket"
            );
        }

        #[tokio::test]
        async fn production_tcp_path_ignores_spoofed_xff_from_untrusted_peer() {
            let (address, shutdown_tx, server) =
                spawn_production_oauth_server(RateLimitState::default()).await;
            let client = oauth_http_client();
            let endpoint = format!("http://{address}/auth/google/start");

            for request_number in 0..START_LIMIT_PER_MIN {
                let response = client
                    .get(&endpoint)
                    .header(
                        X_FORWARDED_FOR.as_str(),
                        format!("203.0.113.{}", request_number + 1),
                    )
                    .send()
                    .await
                    .unwrap();
                assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
                assert_ne!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            }

            let response = client
                .get(&endpoint)
                .header(X_FORWARDED_FOR.as_str(), "198.51.100.200")
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

            let _ = shutdown_tx.send(());
            server.await.unwrap();
        }

        #[tokio::test]
        async fn production_tcp_path_keeps_trusted_forwarded_clients_independent() {
            let trusted_proxies = TrustedProxyList::parse("127.0.0.1/32").unwrap();
            let rate_limit = RateLimitState::with_trusted_proxies(trusted_proxies);
            let (address, shutdown_tx, server) = spawn_production_oauth_server(rate_limit).await;
            let client = oauth_http_client();
            let start_endpoint = format!("http://{address}/auth/google/start");
            let callback_endpoint =
                format!("http://{address}/auth/google/callback?code=invalid&state=invalid");
            let attacker_ip = "203.0.113.10";
            let other_client_ip = "203.0.113.11";

            for _ in 0..START_LIMIT_PER_MIN {
                let response = client
                    .get(&start_endpoint)
                    .header(X_FORWARDED_FOR.as_str(), attacker_ip)
                    .send()
                    .await
                    .unwrap();
                assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            }
            assert_eq!(
                client
                    .get(&start_endpoint)
                    .header(X_FORWARDED_FOR.as_str(), attacker_ip)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::TOO_MANY_REQUESTS
            );
            assert_ne!(
                client
                    .get(&start_endpoint)
                    .header(X_FORWARDED_FOR.as_str(), other_client_ip)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                StatusCode::TOO_MANY_REQUESTS
            );

            for _ in 0..CALLBACK_LIMIT_PER_MIN {
                let response = client
                    .get(&callback_endpoint)
                    .header("x-real-ip", attacker_ip)
                    .send()
                    .await
                    .unwrap();
                assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            }
            let limited = client
                .get(&callback_endpoint)
                .header("x-real-ip", attacker_ip)
                .send()
                .await
                .unwrap();
            assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
            assert!(limited.headers().contains_key(RETRY_AFTER));

            let other_client = client
                .get(&callback_endpoint)
                .header("x-real-ip", other_client_ip)
                .send()
                .await
                .unwrap();
            assert_ne!(other_client.status(), StatusCode::TOO_MANY_REQUESTS);

            let _ = shutdown_tx.send(());
            server.await.unwrap();
        }

        #[tokio::test]
        async fn test_middleware_429_start_exceeded() {
            let state = make_test_state().await;
            let app =
                make_router_with_connect_info(state, SocketAddr::from(([192, 0, 2, 10], 54321)));

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
            let app =
                make_router_with_connect_info(state, SocketAddr::from(([192, 0, 2, 10], 54321)));

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
            let app =
                make_router_with_connect_info(state, SocketAddr::from(([192, 0, 2, 10], 54321)));

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
