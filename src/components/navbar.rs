use dioxus::prelude::*;

#[cfg(test)]
mod logout_tests {
    /// Both responsive forms must survive the click until native POST submission.
    #[test]
    fn logout_forms_do_not_unmount_before_native_submission() {
        let source = include_str!("navbar.rs")
            .split_once("\npub fn Navbar(")
            .unwrap()
            .1;
        let forms: Vec<_> = source
            .split("form {")
            .skip(1)
            .filter_map(|section| section.split_once("\"Sign Out\"").map(|(form, _)| form))
            .collect();
        assert_eq!(forms.len(), 2);
        for form in forms {
            assert!(form.contains("method: \"post\""));
            assert!(form.contains("action: \"/auth/logout?redirect_uri=/\""));
            assert!(form.contains("r#type: \"submit\""));
            assert!(!form.contains("dropdown_open.set(false)"));
            assert!(!form.contains("menu_open.set(false)"));
        }
    }
}

use crate::auth::context::use_auth;
use crate::components::base::{Avatar, AvatarSize};
use crate::utils::theme::UseTheme;
use crate::Route;

#[cfg(target_arch = "wasm32")]
use crate::components::base::account_menu::{dispose_account_menu, mount_account_menu};

/// Top navigation bar with glassmorphic styling.
///
/// Responsive: collapses to a hamburger menu on screens < 768px.
/// Auth state is read from the `AuthContext` provided by the server.
#[component]
pub fn Navbar(theme: UseTheme) -> Element {
    let mut menu_open = use_signal(|| false);
    let mut sidebar_open = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    use_drop(|| dispose_account_menu("navbar-user-dropdown"));

    let auth = use_auth();
    let current_route = use_route::<Route>();
    let home_is_current = current_route == (Route::Home {});
    let search_is_current = matches!(current_route, Route::Search { .. });
    let explore_is_current = current_route == (Route::Explore {});
    let about_is_current = current_route == (Route::About {});
    let how_to_is_current = current_route == (Route::HowTo {});

    let is_signed_in = auth.is_authenticated;
    let display_name = auth
        .current_user
        .as_ref()
        .map(|u| {
            if u.display_name.is_empty() {
                u.username.clone()
            } else {
                u.display_name.clone()
            }
        })
        .unwrap_or_else(|| "User".to_string());
    let avatar_src = auth
        .current_user
        .as_ref()
        .and_then(|u| u.avatar_url.clone());

    rsx! {
        nav {
            class: "navbar",
            div { class: "navbar-inner container",
                // Left drawer toggle (top left)
                button {
                    class: "navbar-drawer-toggle touch-target",
                    aria_label: "Open sidebar",
                    onclick: move |_| sidebar_open.set(!sidebar_open()),
                    "☰"
                }

                // Logo (left)
                Link {
                    to: Route::Home {},
                    class: "navbar-logo",
                    "🍴 Noms"
                }

                // Desktop nav links (center) — hidden on mobile
                div { class: "navbar-links",
                    Link { to: Route::Home {}, class: if home_is_current { "navbar-link navbar-link--current" } else { "navbar-link" }, aria_current: if home_is_current { "page" } else { "false" }, "Home" }
                    Link { to: Route::Search { query: Default::default() }, class: if search_is_current { "navbar-link navbar-link--current" } else { "navbar-link" }, aria_current: if search_is_current { "page" } else { "false" }, "Search" }
                    Link { to: Route::Explore {}, class: if explore_is_current { "navbar-link navbar-link--current" } else { "navbar-link" }, aria_current: if explore_is_current { "page" } else { "false" }, "Explore" }
                    Link { to: Route::About {}, class: if about_is_current { "navbar-link navbar-link--current" } else { "navbar-link" }, aria_current: if about_is_current { "page" } else { "false" }, "About" }
                    Link { to: Route::HowTo {}, class: if how_to_is_current { "navbar-link navbar-link--current" } else { "navbar-link" }, aria_current: if how_to_is_current { "page" } else { "false" }, "How to" }
                }

                // Right side: auth + theme toggle (desktop)
                div { class: "navbar-actions",
                    if is_signed_in {
                        // Persistent P1 menu: hiding must not unmount the logout form.
                        div {
                            class: "navbar-user-menu",
                            id: "navbar-user-dropdown",
                            onmounted: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                mount_account_menu("navbar-user-dropdown");
                            },
                            // User dropdown trigger
                            button {
                                r#type: "button",
                                class: "navbar-user-dropdown",
                                "data-account-trigger": "",
                                id: "account-menu-trigger",
                                aria_label: "Account menu",
                                aria_haspopup: "menu",
                                aria_expanded: "false",
                                aria_controls: "account-menu-panel",
                                Avatar {
                                    size: AvatarSize::Small,
                                    src: avatar_src.clone(),
                                    username: display_name.clone(),
                                }
                                span { class: "navbar-username", "{display_name}" }
                            }

                            // User dropdown menu
                            div {
                                id: "account-menu-panel",
                                class: "feedback-surface account-menu-panel",
                                "data-account-panel": "",
                                hidden: true,
                                div { class: "account-menu-identity",
                                    Avatar { size: AvatarSize::Medium, src: avatar_src.clone(), username: display_name.clone() }
                                    div { class: "account-menu-identity-text",
                                        strong { "{display_name}" }
                                        span { "@{auth.current_user.as_ref().map(|u| u.username.clone()).unwrap_or_default()}" }
                                    }
                                }
                                div { class: "account-menu-divider" }
                                div {
                                    role: "menu",
                                    aria_labelledby: "account-menu-trigger",
                                    Link {
                                        to: Route::Dashboard {}, class: "account-menu-item", role: "menuitem", tabindex: "-1",
                                        AccountMenuIcon { kind: "dashboard" }
                                        "Dashboard"
                                    }
                                    a {
                                        href: format!("/u/{}", auth.current_user.as_ref().map(|u| u.username.clone()).unwrap_or_default()),
                                        class: "account-menu-item", role: "menuitem", tabindex: "-1",
                                        AccountMenuIcon { kind: "profile" }
                                        "My profile"
                                    }
                                    Link {
                                        to: Route::SettingsProfile {},
                                        class: "account-menu-item", role: "menuitem", tabindex: "-1",
                                        AccountMenuIcon { kind: "settings" }
                                        "Settings"
                                    }
                                    div { class: "account-menu-divider", role: "separator" }
                                    form {
                                        class: "navbar-logout-form",
                                        method: "post",
                                        action: "/auth/logout?redirect_uri=/",
                                        button {
                                            r#type: "submit",
                                            class: "account-menu-item", role: "menuitem", tabindex: "-1",
                                            // Keep the form mounted until the browser submits it.
                                            AccountMenuIcon { kind: "logout" }
                                            "Sign Out"
                                        }
                                    }
                                }
                            }
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
        }

        // Mobile slide-out drawer — rendered OUTSIDE the <nav> so that the
        // navbar's backdrop-filter does not create a containing block that
        // clips the drawer's `position: fixed; bottom: 0` to the navbar height.
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
                            to: Route::Home {},
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "Home"
                        }
                        Link {
                            to: Route::Dashboard {},
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "Dashboard"
                        }
                        Link {
                            to: Route::Search { query: Default::default() },
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "Search"
                        }
                        Link {
                            to: Route::Explore {},
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "Explore"
                        }
                        Link {
                            to: Route::About {},
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "About"
                        }
                        Link {
                            to: Route::HowTo {},
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "How to"
                        }
                        Link {
                            to: Route::RecipeNew {},
                            class: "navbar-drawer-link",
                            onclick: move |_| menu_open.set(false),
                            "New Recipe"
                        }
                        if is_signed_in {
                            Link {
                                to: Route::SettingsProfile {},
                                class: "navbar-drawer-link",
                                onclick: move |_| menu_open.set(false),
                                "Settings"
                            }
                            form {
                                class: "navbar-logout-form",
                                method: "post",
                                action: "/auth/logout?redirect_uri=/",
                                button {
                                    r#type: "submit",
                                    class: "navbar-drawer-link navbar-drawer-link-danger",
                                    // Closing the drawer here would remove the form before POST.
                                    onclick: move |evt| evt.stop_propagation(),
                                    "Sign Out"
                                }
                            }
                        } else {
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

        // Left sidebar drawer — rendered OUTSIDE the <nav> so backdrop-filter
        // does not create a containing block that clips the drawer.
        if sidebar_open() {
            div {
                class: "sidebar-overlay",
                onclick: move |_| sidebar_open.set(false),
                div {
                    class: "sidebar-drawer",
                    onclick: move |evt| evt.stop_propagation(),
                    // Close button
                    button {
                        class: "sidebar-close touch-target",
                        onclick: move |_| sidebar_open.set(false),
                        aria_label: "Close sidebar",
                        "✕"
                    }

                    // Sidebar nav links
                    div { class: "sidebar-links",
                        Link {
                            to: Route::Home {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "Home"
                        }
                        Link {
                            to: Route::Dashboard {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "Dashboard"
                        }
                        Link {
                            to: Route::Search { query: Default::default() },
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "Search"
                        }
                        Link {
                            to: Route::Explore {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "Explore"
                        }
                        Link {
                            to: Route::RecipeNew {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "New Recipe"
                        }
                        Link {
                            to: Route::CollectionList {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "Collections"
                        }
                        div { class: "sidebar-divider" }
                        Link {
                            to: Route::About {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "About"
                        }
                        Link {
                            to: Route::HowTo {},
                            class: "sidebar-link",
                            onclick: move |_| sidebar_open.set(false),
                            "How to"
                        }
                        if is_signed_in {
                            div { class: "sidebar-divider" }
                            Link {
                                to: Route::SettingsProfile {},
                                class: "sidebar-link",
                                onclick: move |_| sidebar_open.set(false),
                                "Settings"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AccountMenuIcon(kind: &'static str) -> Element {
    let path = match kind {
        "dashboard" => "M3 10 12 3l9 7v11h-6v-7H9v7H3Z",
        "profile" => "M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM4 21v-2a8 5 0 0 1 16 0v2Z",
        "settings" => "M4 5h16M4 12h16M4 19h16M8 2v6M16 9v6M10 16v6",
        _ => "M14 4H4v16h10M10 12h11m-4-4 4 4-4 4",
    };
    rsx! { svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true", path { d: path } } }
}
