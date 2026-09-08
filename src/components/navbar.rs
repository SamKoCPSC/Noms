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

/// Top navigation bar with glassmorphic styling.
///
/// Responsive: collapses to a hamburger menu on screens < 768px.
/// Auth state is read from the `AuthContext` provided by the server.
#[component]
pub fn Navbar(theme: UseTheme) -> Element {
    let mut menu_open = use_signal(|| false);
    let mut dropdown_open = use_signal(|| false);
    let mut sidebar_open = use_signal(|| false);

    // Close dropdown when clicking outside — document-level listener via web_sys
    // Only runs on web platform; no-op on server (SSR)
    #[cfg(target_arch = "wasm32")]
    {
        use dioxus::core::use_hook;
        use dioxus::prelude::use_drop;
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let dropdown_id = "navbar-user-dropdown";

        // Store the closure for cleanup in use_drop
        let closure_signal = use_signal(|| None::<Closure<dyn FnMut(web_sys::MouseEvent)>>);

        use_hook({
            let mut closure_sig = closure_signal;
            move || {
                let window = web_sys::window().expect("no window");
                let document = window.document().expect("no document");
                let mut dropdown_signal = dropdown_open;
                // Clone document for use inside the closure (Document is a thin JS wrapper)
                let doc_for_closure = document.clone();

                let closure: Closure<dyn FnMut(web_sys::MouseEvent)> =
                    Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                        // If dropdown is not open, nothing to do
                        if !dropdown_signal() {
                            return;
                        }

                        // Find the dropdown container in the DOM
                        let Some(dropdown_el) = doc_for_closure.get_element_by_id(dropdown_id)
                        else {
                            return;
                        };

                        // Get the click target
                        let Some(target) = event.target() else {
                            return;
                        };

                        // Check if the target is a descendant of (or is) the dropdown container
                        let target_node: Option<web_sys::Node> = target.dyn_into().ok();
                        if let Some(target_node) = target_node {
                            if !dropdown_el.contains(Some(&target_node)) {
                                // Click was outside — close the dropdown
                                dropdown_signal.set(false);
                            }
                        }
                    })
                        as Box<dyn FnMut(web_sys::MouseEvent)>);

                document
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .expect("failed to add click listener");

                // Store closure for cleanup
                closure_sig.set(Some(closure));
            }
        });

        // Cleanup: remove the event listener when component unmounts
        use_drop({
            let mut closure_sig = closure_signal;
            move || {
                if let Some(closure) = closure_sig.take() {
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            let _ = document.remove_event_listener_with_callback(
                                "click",
                                closure.as_ref().unchecked_ref(),
                            );
                        }
                    }
                }
            }
        });
    }

    let auth = use_auth();

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
                    Link { to: Route::Home {}, class: "navbar-link", "Home" }
                    Link { to: Route::Dashboard {}, class: "navbar-link", "Dashboard" }
                    Link { to: Route::Search { query: Default::default() }, class: "navbar-link", "Search" }
                    Link { to: Route::Explore {}, class: "navbar-link", "Explore" }
                    Link { to: Route::RecipeNew {}, class: "navbar-link", "New Recipe" }
                }

                // Right side: auth + theme toggle (desktop)
                div { class: "navbar-actions",
                    if is_signed_in {
                        // User dropdown container with stable id for outside-click detection
                        div {
                            class: "navbar-user-menu",
                            id: "navbar-user-dropdown",
                            // User dropdown trigger
                            div {
                                class: "navbar-user-dropdown",
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    dropdown_open.set(!dropdown_open());
                                },
                                Avatar {
                                    size: AvatarSize::Small,
                                    src: avatar_src.clone(),
                                    username: display_name.clone(),
                                }
                                span { class: "navbar-username", "{display_name}" }
                            }

                            // User dropdown menu
                            if dropdown_open() {
                                div {
                                    class: "navbar-dropdown-menu",
                                    onclick: move |evt| evt.stop_propagation(),
                                    div { class: "navbar-dropdown-header",
                                        "{display_name}"
                                    }
                                    div { class: "navbar-dropdown-divider" }
                                    a {
                                        href: format!("/u/{}", auth.current_user.as_ref().map(|u| u.username.clone()).unwrap_or_default()),
                                        class: "navbar-dropdown-item",
                                        onclick: move |_| dropdown_open.set(false),
                                        "View Profile"
                                    }
                                    Link {
                                        to: Route::SettingsProfile {},
                                        class: "navbar-dropdown-item",
                                        onclick: move |_| dropdown_open.set(false),
                                        "Settings"
                                    }
                                    form {
                                        class: "navbar-logout-form",
                                        method: "post",
                                        action: "/auth/logout?redirect_uri=/",
                                        button {
                                            r#type: "submit",
                                            class: "navbar-dropdown-item navbar-dropdown-item-danger",
                                            // Keep the form mounted until the browser submits it.
                                            onclick: move |evt| evt.stop_propagation(),
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
