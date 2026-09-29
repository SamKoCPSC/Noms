use dioxus::prelude::*;

use crate::Route;

const SIGN_IN_CSS: Asset = asset!("/assets/sign-in.css");
const GOOGLE_MARK: Asset = asset!("/assets/sign-in/google-g.png");
const GITHUB_MARK: Asset = asset!("/assets/sign-in/GitHub_Invertocat_Black.svg");

/// Sign-in page with OAuth provider buttons.
///
/// Extracts `redirect_uri` from the query string (set by the auth middleware
/// when bouncing an unauthenticated user from a protected route) and passes
/// it through to the OAuth start endpoint.
#[component]
pub fn Login() -> Element {
    let redirect_uri = use_hook(extract_redirect_uri);

    rsx! {
        document::Link { rel: "stylesheet", href: SIGN_IN_CSS }
        section { class: "sign-in-page", aria_labelledby: "sign-in-title",
            header { class: "sign-in-heading",
                crate::components::base::BrandMark { large: true }
                h1 { id: "sign-in-title", "Welcome back" }
                p { "Sign in to access your recipe library." }
            }
            div { class: "sign-in-providers neumo-raised", role: "group", aria_label: "Sign-in providers",
                // Native links preserve full-page OAuth navigation, browser pending/error
                // handling, and modifier-click behavior without a sticky client busy state.
                for (provider, label, mark) in [("google", "Google", GOOGLE_MARK), ("github", "GitHub", GITHUB_MARK)] {
                    a {
                        key: "{provider}",
                        href: oauth_start_url(provider, &redirect_uri),
                        class: "btn btn-secondary touch-target sign-in-provider",
                        span { class: "sign-in-provider__icon", aria_hidden: "true",
                            img { src: mark, alt: "", width: "20", height: "20" }
                        }
                        span { "Continue with {label}" }
                    }
                }
            }
            Link { to: Route::Explore {}, class: "sign-in-guest-link", "Explore without an account →" }
            div { class: "sign-in-features",
                section { class: "sign-in-feature-panel neumo-raised", aria_labelledby: "sign-in-guest-features",
                    h2 { id: "sign-in-guest-features", "Without an account" }
                    p { "Access follows each recipe’s sharing settings." }
                    ul {
                        for feature in [
                            "Browse and search public recipes in Explore",
                            "View shared recipes and creator profiles",
                            "Read published history on recipes you can access",
                        ] {
                            SignInFeature { key: "{feature}", text: feature }
                        }
                    }
                }
                section { class: "sign-in-feature-panel neumo-raised", aria_labelledby: "sign-in-account-features",
                    h2 { id: "sign-in-account-features", "With an account" }
                    p { "Everything you can browse, plus:" }
                    ul {
                        for feature in [
                            "Create recipes with private autosaved drafts",
                            "Organize recipes into collections",
                            "Fork recipes to make them your own",
                            "Keep recipes private or choose how to share",
                            "Search across recipes, collections, and cooks",
                        ] {
                            SignInFeature { key: "{feature}", text: feature }
                        }
                    }
                }
            }
            Link { to: Route::Home {}, class: "sign-in-home-link", "← Back to home" }
        }
    }
}

#[component]
fn SignInFeature(text: &'static str) -> Element {
    rsx! {
        li {
            svg { class: "sign-in-feature-check", view_box: "0 0 24 24", fill: "none", stroke: "currentColor",
                stroke_width: "1.6", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true",
                circle { cx: "12", cy: "12", r: "9" }
                path { d: "m8 12 3 3 5-6" }
            }
            span { "{text}" }
        }
    }
}

fn oauth_start_url(provider: &str, redirect_uri: &str) -> String {
    let encoded =
        percent_encoding::utf8_percent_encode(redirect_uri, percent_encoding::NON_ALPHANUMERIC);
    format!("/auth/{provider}/start?redirect_uri={encoded}&purpose=login")
}

/// Extract the `redirect_uri` query parameter from the current URL.
///
/// On WASM, reads from `window.location.search`. On the server, reads
/// from the `FullstackContext` request URI. Falls back to `"/home"`.
fn extract_redirect_uri() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            if let Ok(search) = location.search() {
                return parse_redirect_uri(&search);
            }
        }
        "/home".to_string()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(fsc) = dioxus_fullstack::FullstackContext::current() {
            let parts = fsc.parts_mut();
            if let Some(query) = parts.uri.query() {
                return parse_redirect_uri(query);
            }
        }
        "/home".to_string()
    }
}

/// Parse the `redirect_uri` query parameter from a query string.
///
/// Validates the value to prevent open redirects: only relative paths
/// starting with `/` are accepted. Invalid values fall back to `"/home"`.
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
    "/home".to_string()
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
    use dioxus::router::components::HistoryProvider;
    use dioxus_core::NoOpMutations;

    #[test]
    fn sign_in_renders_the_painted_brand_mark_as_decorative_artwork() {
        let mut dom = VirtualDom::new(LoginTestRoot);
        dom.rebuild(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("noms-logo-painted-s3-v2"), "{html}");
        assert!(html.contains("alt=\"\""), "{html}");
        assert!(html.contains("Welcome back"), "{html}");
    }

    #[derive(Clone, PartialEq, Routable)]
    enum LoginTestRoute {
        #[route("/")]
        Login {},
    }

    #[component]
    fn LoginTestRoot() -> Element {
        rsx! {
            HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<LoginTestRoute> {}
            }
        }
    }

    #[test]
    fn rendered_sign_in_has_provider_and_guest_destinations_with_qualified_access_copy() {
        let mut dom = VirtualDom::new(LoginTestRoot);
        dom.rebuild(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        for provider in ["google", "github"] {
            assert!(
                html.contains(&format!(
                    "/auth/{provider}/start?redirect_uri=%2Fhome&#38;purpose=login"
                )),
                "{html}"
            );
        }
        assert!(html.contains("href=\"/explore\""), "{html}");
        assert!(html.contains("Explore without an account"), "{html}");
        assert!(html.contains("Without an account"), "{html}");
        assert!(html.contains("With an account"), "{html}");
        assert!(html.contains("recipes you can access"), "{html}");
        assert!(html.contains("Everything you can browse, plus:"), "{html}");
        assert!(
            html.contains("Browse and search public recipes in Explore"),
            "{html}"
        );
        assert!(
            html.contains("Search across recipes, collections, and cooks"),
            "{html}"
        );
        assert_eq!(html.matches("<li>").count(), 8, "{html}");
    }

    #[test]
    fn provider_links_preserve_nested_return_query_without_injecting_oauth_parameters() {
        let target = parse_redirect_uri(
            "redirect_uri=%2Fsearch%3Fq%3Dsoup%2520%2526%2520herbs%26scope%3Drecipes",
        );
        for provider in ["google", "github"] {
            let url = url::Url::parse(&format!(
                "https://noms.example{}",
                oauth_start_url(provider, &target)
            ))
            .unwrap();
            let pairs: Vec<_> = url.query_pairs().collect();
            assert_eq!(pairs.len(), 2);
            assert_eq!(pairs[0].0, "redirect_uri");
            assert_eq!(pairs[0].1, "/search?q=soup%20%26%20herbs&scope=recipes");
            assert_eq!(pairs[1], ("purpose".into(), "login".into()));
        }
    }

    #[test]
    fn default_sign_in_destination_is_community_home() {
        assert_eq!(parse_redirect_uri(""), "/home");
        assert_eq!(parse_redirect_uri("redirect_uri=//evil.example"), "/home");
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2Fdashboard"),
            "/dashboard"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2Frecipes%2Fnew"),
            "/recipes/new"
        );
    }

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
        assert_eq!(parse_redirect_uri("redirect_uri=https://evil.com"), "/home");
        assert_eq!(parse_redirect_uri("redirect_uri=//evil.com"), "/home");
        assert_eq!(
            parse_redirect_uri("redirect_uri=/%255cevil.example"),
            "/home"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2F.%2F%2Fevil.example"),
            "/home"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=%2F%252e%252e%2F%2Fevil.example"),
            "/home"
        );
        assert_eq!(
            parse_redirect_uri("redirect_uri=javascript:alert(1)"),
            "/home"
        );
        assert_eq!(parse_redirect_uri("redirect_uri="), "/home");
        assert_eq!(parse_redirect_uri("foo=bar"), "/home");
    }
}
