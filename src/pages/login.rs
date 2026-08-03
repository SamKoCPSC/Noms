use dioxus::prelude::*;

use crate::components::base::Card;
use crate::Route;

/// Sign-in page with OAuth provider buttons.
///
/// Extracts `redirect_uri` from the query string (set by the auth middleware
/// when bouncing an unauthenticated user from a protected route) and passes
/// it through to the OAuth start endpoint.
#[component]
pub fn Login() -> Element {
    let redirect_uri = use_hook(extract_redirect_uri);

    let oauth_url = |provider: &str| -> String {
        let encoded = percent_encoding::utf8_percent_encode(
            &redirect_uri,
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        format!(
            "/auth/{}/start?redirect_uri={}&purpose=login",
            provider, encoded
        )
    };

    rsx! {
        div { class: "container",
            div {
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                justify_content: "center",
                min_height: "60vh",
                text_align: "center",
                h1 {
                    font_size: "32px",
                    color: "var(--text-primary)",
                    margin_bottom: "var(--space-md)",
                    "Welcome back"
                }
                p {
                    color: "var(--text-secondary)",
                    margin_bottom: "var(--space-xl)",
                    "Sign in to access your recipe library."
                }
                div {
                    max_width: "400px",
                    width: "100%",
                    Card {
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-md)",
                            padding: "var(--space-xl)",
                            a {
                                href: oauth_url("google"),
                                class: "btn btn-secondary touch-target",
                                width: "100%",
                                "Continue with Google"
                            }
                            a {
                                href: oauth_url("github"),
                                class: "btn btn-secondary touch-target",
                                width: "100%",
                                "Continue with GitHub"
                            }
                        }
                    }
                }
                p {
                    margin_top: "var(--space-lg)",
                    font_size: "14px",
                    color: "var(--text-secondary)",
                    Link {
                        to: Route::Home {},
                        "← Back to home"
                    }
                }
            }
        }
    }
}

/// Extract the `redirect_uri` query parameter from the current URL.
///
/// On WASM, reads from `window.location.search`. On the server, reads
/// from the `FullstackContext` request URI. Falls back to `"/dashboard"`.
fn extract_redirect_uri() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(search) = location.search() {
                return parse_redirect_uri(&search);
            }
        }
        "/dashboard".to_string()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(fsc) = dioxus_fullstack::FullstackContext::current() {
            let parts = fsc.parts_mut();
            if let Some(query) = parts.uri.query() {
                return parse_redirect_uri(query);
            }
        }
        "/dashboard".to_string()
    }
}

/// Parse the `redirect_uri` query parameter from a query string.
///
/// Validates the value to prevent open redirects: only relative paths
/// starting with `/` are accepted. Invalid values fall back to `"/dashboard"`.
fn parse_redirect_uri(query: &str) -> String {
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("redirect_uri=") {
            if let Ok(decoded) = percent_encoding::percent_decode_str(value).decode_utf8() {
                if let Some(redirect) =
                    crate::auth::redirect::normalize_same_origin_redirect(&decoded)
                {
                    return redirect;
                }
            }
        }
    }
    "/dashboard".to_string()
}

/// Validate that a redirect URI is safe (relative path only).
///
/// Rejects absolute URLs, protocol-relative URLs, data URIs, and
/// javascript: URIs to prevent open redirect attacks.
#[cfg(test)]
fn is_safe_redirect_uri(uri: &str) -> bool {
    crate::auth::redirect::normalize_same_origin_redirect(uri).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_redirect_uris() {
        assert!(is_safe_redirect_uri("/dashboard"));
        assert!(is_safe_redirect_uri("/recipes/new"));
        assert!(is_safe_redirect_uri("/recipes/new?category=seafood"));
        assert!(is_safe_redirect_uri("/recipes/123/edit?draft=true"));
        assert!(is_safe_redirect_uri("/"));
    }

    #[test]
    fn unsafe_redirect_uris() {
        // Absolute URLs
        assert!(!is_safe_redirect_uri("https://evil.com"));
        assert!(!is_safe_redirect_uri("http://evil.com"));
        assert!(!is_safe_redirect_uri("https://evil.com/login"));

        // Protocol-relative URLs
        assert!(!is_safe_redirect_uri("//evil.com"));
        assert!(!is_safe_redirect_uri("//evil.com/phishing"));

        // Raw, encoded, and double-encoded slash/backslash confusion
        for uri in [
            "/\\evil.example",
            "/%5cevil.example",
            "/%2fevil.example",
            "/%255cevil.example",
            "/%252fevil.example",
            "/.//evil.example",
            "/%2e%2e//evil.example",
            "/%",
            "/%zz",
            "/%FF",
        ] {
            assert!(
                !is_safe_redirect_uri(uri),
                "unsafe redirect accepted: {uri}"
            );
        }

        // Data URIs
        assert!(!is_safe_redirect_uri(
            "data:text/html,<script>alert(1)</script>"
        ));

        // JavaScript URIs
        assert!(!is_safe_redirect_uri("javascript:alert(document.cookie)"));

        // Empty / whitespace only
        assert!(!is_safe_redirect_uri(""));
        assert!(!is_safe_redirect_uri("   "));
        assert!(!is_safe_redirect_uri(" /dashboard "));
    }

    #[test]
    fn parse_redirect_uri_valid() {
        assert_eq!(parse_redirect_uri("redirect_uri=/dashboard"), "/dashboard");
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2Fdashboard"),
            "/dashboard"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=/recipes/new?category=seafood"),
            "/recipes/new?category=seafood"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2Frecipes%2Fnew%3Fcategory%3Dseafood"),
            "/recipes/new?category=seafood"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2Fsearch%3Fq%3D100%2525"),
            "/search?q=100%25"
        );
        assert_eq!(
            parse_redirect_uri("foo=bar&redirect_uri=/recipes/new&baz=qux"),
            "/recipes/new"
        );
    }

    #[test]
    fn parse_redirect_uri_invalid_falls_back() {
        assert_eq!(
            parse_redirect_uri("redirect_uri=https://evil.com"),
            "/dashboard"
        );
        assert_eq!(parse_redirect_uri("redirect_uri=//evil.com"), "/dashboard");
        assert_eq!(
            parse_redirect_uri("redirect_uri=/%255cevil.example"),
            "/dashboard"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2F.%2F%2Fevil.example"),
            "/dashboard"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2F%252e%252e%2F%2Fevil.example"),
            "/dashboard"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=javascript:alert(1)"),
            "/dashboard"
        );
        assert_eq!(parse_redirect_uri("redirect_uri="), "/dashboard");
        assert_eq!(parse_redirect_uri("foo=bar"), "/dashboard");
    }
}
