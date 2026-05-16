use dioxus::prelude::*;

use crate::Route;
use crate::components::base::{Avatar, AvatarSize};
use crate::utils::theme::UseTheme;

/// Placeholder user for visual testing (not wired to real auth).
struct MockUser {
    username: &'static str,
}

/// Top navigation bar with glassmorphic styling.
///
/// Responsive: collapses to a hamburger menu on screens < 768px.
#[component]
pub fn Navbar(theme: UseTheme) -> Element {
    let mut menu_open = use_signal(|| false);

    // Hardcoded mock user for visual testing
    let mock_user: Option<MockUser> = None;
    // Toggle to `Some` to test the signed-in state:
    // let mock_user: Option<MockUser> = Some(MockUser { username: "Chef" });

    rsx! {
        nav {
            class: "navbar",
            div { class: "navbar-inner container",
                // Logo (left)
                Link {
                    to: Route::Home {},
                    class: "navbar-logo",
                    "🍴 Noms"
                }

                // Desktop nav links (center) — hidden on mobile
                div { class: "navbar-links",
                    Link { to: Route::Dashboard {}, class: "navbar-link", "Dashboard" }
                    Link { to: Route::Explore {}, class: "navbar-link", "Explore" }
                    Link { to: Route::RecipeNew {}, class: "navbar-link", "New Recipe" }
                }

                // Right side: auth + theme toggle (desktop)
                div { class: "navbar-actions",
                    if let Some(ref user) = mock_user {
                        div { class: "navbar-user",
                            Avatar {
                                size: AvatarSize::Small,
                                username: user.username.to_string(),
                            }
                            span { class: "navbar-username", "{user.username}" }
                        }
                    } else {
                        Link {
                            to: Route::Login {},
                            class: "navbar-link",
                            "Sign In"
                        }
                    }

                    // Theme toggle
                    button {
                        class: "navbar-theme-toggle touch-target",
                        onclick: move |_| {
                            let _ = document::eval("console.log('theme toggle clicked')");
                            theme.toggle();
                            let _ = document::eval(&format!("console.log('is_dark: {}')", theme.is_dark()));
                        },
                        aria_label: "Toggle theme",
                        if theme.is_dark() {
                            "☀️"
                        } else {
                            "🌙"
                        }
                    }
                }

                // Hamburger button (mobile only)
                button {
                    class: "navbar-hamburger touch-target",
                    onclick: move |_| menu_open.toggle(),
                    aria_label: "Toggle menu",
                    span { class: "hamburger-line" }
                    span { class: "hamburger-line" }
                    span { class: "hamburger-line" }
                }
            }

            // Mobile slide-out drawer
            if menu_open() {
                div {
                    class: "navbar-drawer",
                    onclick: move |_| menu_open.set(false),
                    div {
                        class: "navbar-drawer-content",
                        onclick: move |evt| evt.stop_propagation(),
                        // Close button
                        button {
                            class: "navbar-drawer-close touch-target",
                            onclick: move |_| menu_open.set(false),
                            aria_label: "Close menu",
                            "✕"
                        }

                        // Drawer nav links
                        div { class: "navbar-drawer-links",
                            Link {
                                to: Route::Dashboard {},
                                class: "navbar-drawer-link",
                                onclick: move |_| menu_open.set(false),
                                "Dashboard"
                            }
                            Link {
                                to: Route::Explore {},
                                class: "navbar-drawer-link",
                                onclick: move |_| menu_open.set(false),
                                "Explore"
                            }
                            Link {
                                to: Route::RecipeNew {},
                                class: "navbar-drawer-link",
                                onclick: move |_| menu_open.set(false),
                                "New Recipe"
                            }
                            Link {
                                to: Route::Login {},
                                class: "navbar-drawer-link",
                                onclick: move |_| menu_open.set(false),
                                "Sign In"
                            }
                        }

                        // Drawer theme toggle
                        button {
                            class: "navbar-drawer-theme touch-target",
                            onclick: move |_| {
                                theme.toggle();
                                menu_open.set(false);
                            },
                            if theme.is_dark() {
                                "☀️ Light Mode"
                            } else {
                                "🌙 Dark Mode"
                            }
                        }
                    }
                }
            }
        }
    }
}
