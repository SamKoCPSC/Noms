use dioxus::prelude::*;

use crate::auth::context::use_auth;
use crate::components::base::{Avatar, AvatarSize};
use crate::utils::theme::UseTheme;
use crate::Route;

/// Top navigation bar with glassmorphic styling.
///
/// Responsive: collapses to a hamburger menu on screens < 768px.
/// Auth state is read from the `AuthContext` provided by the server.
#[component]
pub fn Navbar(theme: UseTheme) -> Element {
    let mut menu_open = use_signal(|| false);
    let auth = use_auth();

    let is_signed_in = auth.is_authenticated;
    let display_name = auth
        .current_user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "User".to_string());
    let avatar_src = auth
        .current_user
        .as_ref()
        .and_then(|u| u.avatar_url.clone());

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
                    if is_signed_in {
                        div { class: "navbar-user",
                            Avatar {
                                size: AvatarSize::Small,
                                src: avatar_src.clone(),
                                username: display_name.clone(),
                            }
                            span { class: "navbar-username", "{display_name}" }
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
                        onclick: move |_| theme.toggle(),
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
                            if !is_signed_in {
                                Link {
                                    to: Route::Login {},
                                    class: "navbar-drawer-link",
                                    onclick: move |_| menu_open.set(false),
                                    "Sign In"
                                }
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
