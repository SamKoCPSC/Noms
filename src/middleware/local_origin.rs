//! Keep local OAuth transactions and host-only cookies on one loopback origin.

use axum::extract::{Request, State};
use axum::http::{header, uri::Authority, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

use crate::config::{DeploymentMode, RuntimeConfig};

pub(crate) async fn canonicalize_local_origin(
    State(config): State<RuntimeConfig>,
    request: Request,
    next: Next,
) -> Response {
    // Canonicalize before any route can set a host-only session/transaction
    // cookie. Do not change production routing or accept arbitrary Host values
    // as callback origins; OAuth and logout retain the configured BASE_URL.
    if config.mode() != DeploymentMode::Local {
        return next.run(request).await;
    }
    let base = url::Url::parse(config.base_url()).expect("validated local BASE_URL");
    let is_loopback = |host: &str| {
        host.eq_ignore_ascii_case("localhost") || matches!(host, "127.0.0.1" | "[::1]")
    };
    let Some(canonical_host) = base.host_str().filter(|host| is_loopback(host)) else {
        return next.run(request).await;
    };
    // Local configuration permits URLs with paths, but only a plain origin is
    // eligible for loopback alias normalization.
    if !base.username().is_empty()
        || base.password().is_some()
        || base.path() != "/"
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return next.run(request).await;
    }
    let mut hosts = request.headers().get_all(header::HOST).iter();
    let Some(host) = hosts.next() else {
        return next.run(request).await;
    };
    if hosts.next().is_some() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(authority) = host
        .to_str()
        .ok()
        .and_then(|host| host.parse::<Authority>().ok())
    else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if authority.as_str().contains('@')
        || (authority.as_str() != authority.host() && authority.port_u16().is_none())
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let request_port = authority.port_u16().or_else(|| match base.scheme() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    });
    if !is_loopback(authority.host())
        || authority.host().eq_ignore_ascii_case(canonical_host)
        || request_port != base.port_or_known_default()
    {
        return next.run(request).await;
    }
    // Never replay a mutation across origins. Normal page navigation puts the
    // browser on the canonical host before forms, fetches or logout are used.
    if !matches!(*request.method(), Method::GET | Method::HEAD) {
        return (
            StatusCode::BAD_REQUEST,
            "Open the app at its configured local address before submitting this request.",
        )
            .into_response();
    }
    let path_and_query = request
        .uri()
        .path_and_query()
        .map_or("/", |path| path.as_str());
    let target = format!("{}{path_and_query}", base.origin().ascii_serialization());
    Redirect::temporary(&target).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Method, StatusCode};
    use axum::routing::any;
    use tower::ServiceExt;

    fn local_config(base_url: &str) -> RuntimeConfig {
        RuntimeConfig::from_lookup(|name| match name {
            "NOMS_ENV" => Some("local".into()),
            "BASE_URL" => Some(base_url.into()),
            _ => None,
        })
        .unwrap()
    }

    async fn request(config: RuntimeConfig, method: Method, hosts: &[&str]) -> Response {
        // A cookie marks downstream execution: alias requests must stop before
        // OAuth can issue a browser-binding cookie on the wrong host.
        let router = axum::Router::new()
            .fallback(any(|| async {
                [(header::SET_COOKIE, "downstream=executed")]
            }))
            .layer(axum::middleware::from_fn_with_state(
                config,
                canonicalize_local_origin,
            ));
        let mut request = Request::builder()
            .method(method)
            .uri("/auth/google/start?redirect_uri=%2Fdashboard%3Ftab%3Drecipes&purpose=login")
            .header("x-forwarded-host", "attacker.example:32768");
        for host in hosts {
            request = request.header(header::HOST, *host);
        }
        router
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn local_loopback_alias_redirect_precedes_cookie_and_preserves_path_query() {
        for (base, alias) in [
            ("http://127.0.0.1:32768", "localhost:32768"),
            ("http://127.0.0.1:32768", "LOCALHOST:32768"),
            ("http://localhost:32768", "127.0.0.1:32768"),
            ("http://127.0.0.1:32768", "[::1]:32768"),
            ("http://[::1]:32768", "localhost:32768"),
            ("https://localhost", "127.0.0.1:443"),
        ] {
            for method in [Method::GET, Method::HEAD] {
                let response = request(local_config(base), method, &[alias]).await;
                assert_eq!(
                    response.status(),
                    StatusCode::TEMPORARY_REDIRECT,
                    "{base} {alias}"
                );
                assert_eq!(response.headers()[header::LOCATION], format!("{base}/auth/google/start?redirect_uri=%2Fdashboard%3Ftab%3Drecipes&purpose=login"));
                assert!(!response.headers().contains_key(header::SET_COOKIE));
            }
        }
    }

    #[tokio::test]
    async fn canonical_host_and_unrelated_hosts_or_ports_are_not_aliases() {
        for hosts in [
            vec!["127.0.0.1:32768"],
            vec!["localhost:32769"],
            vec!["localhost"],
            vec!["localhost.attacker.example:32768"],
            vec!["attacker.example:32768"],
            vec!["127.0.0.2:32768"],
            vec!["127.1:32768"],
            vec![],
        ] {
            let response =
                request(local_config("http://127.0.0.1:32768"), Method::GET, &hosts).await;
            assert_eq!(response.status(), StatusCode::OK, "{hosts:?}");
            assert!(!response.headers().contains_key(header::LOCATION));
            assert!(response.headers().contains_key(header::SET_COOKIE));
        }
        let response = request(
            local_config("http://dev.example:32768"),
            Method::GET,
            &["localhost:32768"],
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unsafe_alias_requests_and_ambiguous_host_never_reach_downstream() {
        for method in [
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ] {
            let response = request(
                local_config("http://127.0.0.1:32768"),
                method,
                &["localhost:32768"],
            )
            .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(!response.headers().contains_key(header::LOCATION));
            assert!(!response.headers().contains_key(header::SET_COOKIE));
        }
        let response = request(
            local_config("http://127.0.0.1:32768"),
            Method::GET,
            &["localhost:32768", "localhost:32768"],
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
    }

    #[tokio::test]
    async fn malformed_host_is_rejected_without_cookie_or_redirect() {
        for host in [
            "localhost:99999",
            "localhost:not-a-port",
            "user@localhost:32768",
            "localhost:32768/path",
        ] {
            let response =
                request(local_config("http://127.0.0.1:32768"), Method::GET, &[host]).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{host}");
            assert!(!response.headers().contains_key(header::LOCATION));
            assert!(!response.headers().contains_key(header::SET_COOKIE));
        }
    }
}
