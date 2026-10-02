//! AuthRequired component for protected pages.
//!
//! Renders a login prompt when the user is not authenticated,
//! instead of showing empty or inaccessible UI.

use dioxus::prelude::*;

use crate::auth::context::use_auth;

/// Props for the [`AuthRequired`] component.
#[derive(Props, Clone, PartialEq)]
pub struct AuthRequiredProps {
    /// Protected content rendered when authenticated.
    pub children: Element,
}

/// Renders children if authenticated, otherwise shows a login prompt. Keep an
/// already-mounted editor hidden on sign-out so its fields and owned previews
/// survive until its original account returns. Fresh guests never mount it.
#[component]
pub fn AuthRequired(props: AuthRequiredProps) -> Element {
    let auth = use_auth();
    let mut previously_authenticated = use_signal(|| auth.is_authenticated);
    let signed_in = auth.is_authenticated;
    use_effect(use_reactive(&signed_in, move |signed_in| {
        if signed_in {
            previously_authenticated.set(true);
        }
    }));

    rsx! {
        if !auth.is_authenticated { SignInPrompt {} }
        // Presence, not the value, makes a native inert attribute active.
        div { hidden: !auth.is_authenticated, inert: (!auth.is_authenticated).then_some(true),
            if auth.is_authenticated || previously_authenticated() { {&props.children} }
        }
    }
}

#[component]
fn SignInPrompt() -> Element {
    // Not authenticated — show login prompt
    let sign_in_href = try_router()
        .and_then(|router| {
            // This read subscribes to navigation and retains the raw query
            // rather than serializing a route that might drop parameters.
            crate::auth::redirect::normalize_same_origin_redirect(&router.full_route_string())
        })
        .map(|target| {
            let encoded =
                percent_encoding::utf8_percent_encode(&target, percent_encoding::NON_ALPHANUMERIC);
            format!("/login?redirect_uri={encoded}")
        })
        .unwrap_or_else(|| "/login".to_string());
    rsx! {
        div {
            class: "flex flex-col items-center justify-center min-h-[50vh] text-center px-4",
            div {
                class: "inline-flex items-center justify-center w-16 h-16 rounded-full bg-amber-100 dark:bg-amber-900/30 mb-6",
                span {
                    class: "text-3xl",
                    aria_hidden: "true",
                    "🔒"
                }
            }
            h2 {
                class: "text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2",
                "Sign in to continue"
            }
            p {
                class: "text-gray-600 dark:text-gray-400 mb-8 max-w-md",
                "This page requires you to be signed in. Please log in with your account to access your content."
            }
            div {
                class: "flex flex-col sm:flex-row gap-3",
                a {
                    href: sign_in_href,
                    class: "btn btn-primary touch-target",
                    "Sign In"
                }
                a {
                    href: "/",
                    class: "btn btn-secondary touch-target",
                    "Go Home"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::context::AuthContext;
    use dioxus_core::NoOpMutations;

    // ── Helper: root component providing AuthContext ──────────────────────

    #[derive(Props, Clone, PartialEq)]
    struct TestRootProps {
        is_authenticated: bool,
        children: Element,
    }

    #[component]
    fn TestRoot(props: TestRootProps) -> Element {
        let auth_signal = use_memo(move || {
            Signal::new(AuthContext {
                is_authenticated: props.is_authenticated,
                ..Default::default()
            })
        });

        provide_context(*auth_signal.read());
        rsx! { {&props.children} }
    }

    fn render_with_auth(is_authenticated: bool, children: Element) -> String {
        let mut vdom = VirtualDom::new_with_props(
            TestRoot,
            TestRootProps {
                is_authenticated,
                children,
            },
        );
        vdom.rebuild(&mut NoOpMutations);
        dioxus_ssr::render(&vdom)
    }

    // ── Test: renders children when authenticated ────────────────────────

    #[test]
    fn renders_children_when_authenticated() {
        let html = render_with_auth(
            true,
            rsx! { AuthRequired { div { class: "protected-content", "Dashboard" } } },
        );
        assert!(html.contains("protected-content"), "HTML: {html}");
        assert!(html.contains("Dashboard"), "HTML: {html}");
        // Login prompt should NOT be present
        assert!(!html.contains("Sign in to continue"), "HTML: {html}");
    }

    // ── Test: shows login prompt when not authenticated ──────────────────

    #[test]
    fn shows_login_prompt_when_not_authenticated() {
        let html = render_with_auth(
            false,
            rsx! { AuthRequired { div { class: "protected-content", "Dashboard" } } },
        );
        // Login prompt elements should be present
        assert!(html.contains("Sign in to continue"), "HTML: {html}");
        assert!(html.contains("🔒"), "HTML: {html}");
        assert!(
            html.contains("This page requires you to be signed in"),
            "HTML: {html}"
        );
        // Child content should NOT be present
        assert!(!html.contains("protected-content"), "HTML: {html}");
        assert!(!html.contains("Dashboard"), "HTML: {html}");
    }

    // ── Test: Sign In link points to /login ──────────────────────────────

    #[test]
    fn sign_in_link_points_to_login() {
        let html = render_with_auth(false, rsx! { AuthRequired { div { "Dashboard" } } });
        assert!(html.contains(r#"href="/login""#), "HTML: {html}");
        assert!(html.contains("Sign In"), "HTML: {html}");
    }

    // Preserve arbitrary URLs in this test router, including intentionally
    // unsafe paths that the real application would send to NotFound instead.
    #[derive(Clone)]
    struct GateTestRoute(String);

    impl std::str::FromStr for GateTestRoute {
        type Err = std::convert::Infallible;

        fn from_str(path: &str) -> Result<Self, Self::Err> {
            Ok(Self(path.to_string()))
        }
    }

    impl std::fmt::Display for GateTestRoute {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(&self.0)
        }
    }

    impl Routable for GateTestRoute {
        const SITE_MAP: &'static [dioxus::router::routable::SiteMapSegment] = &[];

        fn render(&self, _level: usize) -> Element {
            rsx! { GateProbe {} }
        }
    }

    #[component]
    fn GateProbe() -> Element {
        rsx! { AuthRequired { p { "Protected content" } } }
    }

    #[component]
    fn RoutedGateTestRoot(initial_path: String) -> Element {
        use_context_provider(|| Signal::new(AuthContext::default()));
        rsx! {
            dioxus::router::components::HistoryProvider {
                history: move |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path(&initial_path)) as std::rc::Rc<dyn History>,
                Router::<GateTestRoute> {}
            }
        }
    }

    fn render_routed_gate(path: &str) -> String {
        let mut dom = VirtualDom::new_with_props(
            RoutedGateTestRoot,
            RoutedGateTestRootProps {
                initial_path: path.to_string(),
            },
        );
        dom.rebuild(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn routed_sign_in_gate_preserves_the_path_and_encoded_query() {
        for target in [
            "/search?q=tomato%20%26%20basil&scope=recipes&vis=mine",
            "/collections/new?parent=7f535c8e-d0a1-4052-87a5-65b203479917",
            "/recipes/recipe-id/edit",
            "/dashboard",
        ] {
            let html = render_routed_gate(target);
            let encoded =
                percent_encoding::utf8_percent_encode(target, percent_encoding::NON_ALPHANUMERIC);
            assert!(
                html.contains(&format!(r#"href="/login?redirect_uri={encoded}""#)),
                "{html}"
            );
            assert!(!html.contains("Protected content"), "{html}");
        }
    }

    #[test]
    fn routed_sign_in_gate_rejects_unsafe_return_paths() {
        for target in ["/%5cevil.example", "/.//evil.example", "/%255cevil.example"] {
            let html = render_routed_gate(target);
            assert!(html.contains(r#"href="/login""#), "{html}");
            assert!(!html.contains("redirect_uri="), "{html}");
        }
        let too_long = format!(
            "/search?q={}",
            "a".repeat(crate::auth::redirect::REDIRECT_URI_MAX_LEN)
        );
        let html = render_routed_gate(&too_long);
        assert!(html.contains(r#"href="/login""#), "{html}");
        assert!(!html.contains("redirect_uri="), "{html}");
    }

    #[component]
    fn ApplicationRouteTestRoot(initial_path: String) -> Element {
        use_context_provider(|| Signal::new(AuthContext::default()));
        rsx! {
            dioxus::router::components::HistoryProvider {
                history: move |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path(&initial_path)) as std::rc::Rc<dyn History>,
                Router::<crate::Route> {}
            }
        }
    }

    #[test]
    fn application_router_renders_the_layout_and_preserves_search_gate_context() {
        let target = "/search?q=tomato%20%26%20basil&scope=recipes&vis=mine";
        let mut dom = VirtualDom::new_with_props(
            ApplicationRouteTestRoot,
            ApplicationRouteTestRootProps {
                initial_path: target.to_string(),
            },
        );
        dom.rebuild(&mut NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        let encoded =
            percent_encoding::utf8_percent_encode(target, percent_encoding::NON_ALPHANUMERIC);
        assert!(html.contains("Primary navigation"), "{html}");
        assert!(html.contains("<main"), "{html}");
        assert!(html.contains("Footer navigation"), "{html}");
        assert!(html.contains("Sign in to continue"), "{html}");
        assert!(
            html.contains(&format!(r#"href="/login?redirect_uri={encoded}""#)),
            "{html}"
        );
    }

    // ── Test: Go Home link points to / ───────────────────────────────────

    #[test]
    fn go_home_link_points_to_root() {
        let html = render_with_auth(false, rsx! { AuthRequired { div { "Dashboard" } } });
        // href="/" appears on the Go Home link
        assert!(
            html.contains(r#"href="/" "#) || html.contains(r#"href="/""#),
            "HTML: {html}"
        );
        assert!(html.contains("Go Home"), "HTML: {html}");
    }

    // ── Test: re-renders when auth state changes ─────────────────────────

    #[test]
    fn re_renders_when_auth_state_changes() {
        // Unauthenticated: shows login prompt
        let html_unauth = render_with_auth(
            false,
            rsx! { AuthRequired { div { class: "protected-content", "Dashboard" } } },
        );
        assert!(
            html_unauth.contains("Sign in to continue"),
            "Unauth HTML: {html_unauth}"
        );
        assert!(
            !html_unauth.contains("protected-content"),
            "Unauth HTML: {html_unauth}"
        );

        // Authenticated: shows children
        let html_auth = render_with_auth(
            true,
            rsx! { AuthRequired { div { class: "protected-content", "Dashboard" } } },
        );
        assert!(
            html_auth.contains("protected-content"),
            "Auth HTML: {html_auth}"
        );
        assert!(html_auth.contains("Dashboard"), "Auth HTML: {html_auth}");
        assert!(
            !html_auth.contains("Sign in to continue"),
            "Auth HTML: {html_auth}"
        );
    }
}
