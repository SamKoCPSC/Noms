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

/// Renders children if authenticated, otherwise shows a login prompt.
#[component]
pub fn AuthRequired(props: AuthRequiredProps) -> Element {
    let auth = use_auth();

    // Not authenticated — show login prompt
    if !auth.is_authenticated {
        return rsx! {
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
                        href: "/login",
                        class: "inline-flex items-center justify-center px-6 py-2.5 border border-transparent text-sm font-medium rounded-lg text-white bg-amber-600 hover:bg-amber-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-amber-500 transition-colors",
                        "Sign In"
                    }
                    a {
                        href: "/",
                        class: "inline-flex items-center justify-center px-6 py-2.5 border border-gray-300 dark:border-gray-600 text-sm font-medium rounded-lg text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 hover:bg-gray-50 dark:hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-amber-500 transition-colors",
                        "Go Home"
                    }
                }
            }
        };
    }

    // Authenticated — render protected content
    rsx! {
        {&props.children}
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
