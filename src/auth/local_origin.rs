//! Public local origin shared through SSR hydration, never OAuth credentials.

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct LocalCanonicalOrigin(pub Option<String>);

impl LocalCanonicalOrigin {
    pub(crate) fn on_canonical_origin(&self, path: &str) -> String {
        format!("{}{path}", self.0.as_deref().unwrap_or(""))
    }
}

pub(crate) fn from_fullstack() -> LocalCanonicalOrigin {
    #[cfg(feature = "server")]
    if let Some(context) = dioxus::fullstack::FullstackContext::current() {
        return context
            .extension::<LocalCanonicalOrigin>()
            .unwrap_or_default();
    }
    LocalCanonicalOrigin::default()
}

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn canonical_local_navigation_target(canonical: &str, current: &str) -> Option<String> {
    let canonical = url::Url::parse(canonical).ok()?;
    let current = url::Url::parse(current).ok()?;
    let is_loopback =
        |url: &url::Url| matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !matches!(canonical.scheme(), "http" | "https")
        || !is_loopback(&canonical)
        || !is_loopback(&current)
        || canonical.scheme() != current.scheme()
        || canonical.port_or_known_default() != current.port_or_known_default()
        || canonical.origin() == current.origin()
        || !canonical.username().is_empty()
        || canonical.password().is_some()
        || !current.username().is_empty()
        || current.password().is_some()
        || canonical.path() != "/"
        || canonical.query().is_some()
        || canonical.fragment().is_some()
    {
        return None;
    }
    // Preserve encoded routing data and the fragment rather than rebuilding
    // query parameters or trusting the current host as an OAuth callback URL.
    Some(format!(
        "{}{}",
        canonical.origin().ascii_serialization(),
        &current[url::Position::BeforePath..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_navigation_preserves_encoded_path_query_and_fragment() {
        assert_eq!(
            canonical_local_navigation_target(
                "http://127.0.0.1:32768",
                "http://localhost:32768/search?q=soup%20%26%20herbs&scope=recipes#results"
            ),
            Some("http://127.0.0.1:32768/search?q=soup%20%26%20herbs&scope=recipes#results".into())
        );
        assert_eq!(
            canonical_local_navigation_target(
                "http://localhost:32768",
                "http://127.0.0.1:32768/login?redirect_uri=%2Fdashboard"
            ),
            Some("http://localhost:32768/login?redirect_uri=%2Fdashboard".into())
        );
    }

    #[test]
    fn local_navigation_does_not_redirect_canonical_or_unrelated_origins() {
        for current in [
            "http://127.0.0.1:32768/",
            "http://localhost:32769/",
            "https://localhost:32768/",
            "http://localhost.attacker.example:32768/",
            "http://127.0.0.2:32768/",
            "not a URL",
        ] {
            assert_eq!(
                canonical_local_navigation_target("http://127.0.0.1:32768", current),
                None,
                "{current}"
            );
        }
        assert_eq!(
            canonical_local_navigation_target("https://noms.example", "https://localhost/"),
            None
        );
    }

    #[test]
    fn local_origin_prefix_is_optional_and_preserves_oauth_intent() {
        let path = "/auth/google/start?redirect_uri=%2Fdashboard&purpose=login";
        assert_eq!(
            LocalCanonicalOrigin::default().on_canonical_origin(path),
            path
        );
        assert_eq!(
            LocalCanonicalOrigin(Some("http://127.0.0.1:32768".into())).on_canonical_origin(path),
            format!("http://127.0.0.1:32768{path}")
        );
    }
}
