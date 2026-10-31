use dioxus::prelude::*;

#[cfg(test)]
mod navigation_tests;

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
use crate::components::base::{
    Avatar, AvatarSize, BrandMark, NavigationArtwork, PageIconKind, SharedModal,
};
use crate::components::kitchen::SectionLink;
use crate::utils::theme::UseTheme;
use crate::Route;

#[cfg(target_arch = "wasm32")]
use crate::components::base::account_menu::{dispose_account_menu, mount_account_menu};

/// N1/N4 opaque raised shell; the account menu and navigation dialog alone use glass.
#[component]
pub fn Navbar(theme: UseTheme) -> Element {
    let mut menu_open = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    use_drop(|| dispose_account_menu("navbar-user-dropdown"));
    let auth = use_auth();
    let identity = auth.current_user_id;
    use_effect(use_reactive(&identity, move |_| {
        menu_open.set(false);
        #[cfg(target_arch = "wasm32")]
        crate::components::base::account_menu::close_account_menu("navbar-user-dropdown");
    }));
    let current_route = use_route::<Route>();
    let editor_back = matches!(
        current_route,
        Route::RecipeNew {} | Route::RecipeEdit { .. } | Route::RecipeDraftEdit { .. }
    );
    // Browser Back/Forward or another route transition must not leave a stale dialog.
    use_effect(use_reactive(&current_route, move |_| menu_open.set(false)));
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
        nav { class: "navbar", aria_label: "Primary navigation",
            div { class: "navbar-inner",
                if let Some((heading, parent)) = crate::components::mobile::route_heading(&current_route) {
                    div { class: "mobile-route-heading",
                        Link { to: parent, class: "mobile-back-link touch-target", aria_label: "Back", onclick_only: editor_back,
                            onclick: move |event: MouseEvent| {
                                if editor_back {
                                    event.prevent_default();
                                    #[cfg(target_arch = "wasm32")]
                                    spawn(async { let _ = document::eval("document.getElementById('recipe-editor-exit')?.click()").await; });
                                }
                            },
                            crate::components::mobile::MobileIcon { kind: "back" }
                        }
                        span { "{heading}" }
                    }
                }
                Link { to: Route::Home {}, class: "navbar-logo", aria_label: "Noms introduction",
                    BrandMark {}
                    span { "Noms" }
                }
                div { class: "navbar-links",
                    for (route, label) in primary_navigation(is_signed_in) {
                        SectionLink {
                            class: if navigation_is_current(&route, &current_route) { "navbar-link navbar-link--current" } else { "navbar-link" },
                            current: navigation_is_current(&route, &current_route),
                            to: route,
                            "{label}"
                        }
                    }
                }
                NavigationSearch { compact: true }
                Link { to: Route::Search { query: Default::default() }, class: "mobile-search-link touch-target", aria_label: "Search Noms", crate::components::mobile::MobileIcon { kind: "search" } }
                div { class: "navbar-actions",
                    if is_signed_in {
                        Link { to: Route::RecipeNew {}, class: "btn btn-primary navbar-create", "New recipe" }
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
                                class: "navbar-user-dropdown btn btn-primary navbar-contained-action",
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
                        Link { to: Route::Login {}, class: "navbar-link navbar-sign-in btn btn-primary navbar-contained-action", "Sign In" }
                    }
                    button {
                        r#type: "button", class: "navbar-theme-toggle btn btn-primary navbar-contained-action touch-target",
                        onclick: move |_| theme.toggle(), aria_label: "Toggle theme",
                        if theme.is_dark() { "☀️" } else { "🌙" }
                    }
                }
                button {
                    id: "navigation-menu-trigger", r#type: "button", class: "navbar-menu-toggle btn btn-primary navbar-contained-action",
                    aria_label: "Open navigation menu", aria_haspopup: "dialog", aria_expanded: menu_open(),
                    onclick: move |_| menu_open.set(true),
                    span { "aria-hidden": "true", "☰" }
                    span { "Menu" }
                }
            }
        }
        if menu_open() {
            SharedModal {
                aria_labelledby: "navigation-menu-title",
                focus_selector: "#navigation-menu-close",
                on_dismiss: move |_| menu_open.set(false),
                div { class: "feedback-surface navigation-drawer", onclick: move |event| event.stop_propagation(),
                    div { class: "navigation-drawer-header",
                        h2 { id: "navigation-menu-title", class: "navigation-artwork-label", NavigationArtwork {} span { "Navigation" } }
                        button { id: "navigation-menu-close", r#type: "button", class: "navigation-close",
                            aria_label: "Close navigation menu", onclick: move |_| menu_open.set(false), "✕" }
                    }
                    if is_signed_in {
                        div { class: "navigation-identity",
                            Avatar { size: AvatarSize::Small, src: avatar_src.clone(), username: display_name.clone() }
                            strong { "{display_name}" }
                        }
                    }
                    NavigationSearch { compact: false, on_navigate: move |_| menu_open.set(false) }
                    nav { class: "navigation-drawer-links", aria_label: "All destinations",
                        for (route, label, icon) in [
                            (if is_signed_in { Route::CommunityHome {} } else { Route::Home {} }, "Home", Some(PageIconKind::Home)),
                            (Route::Dashboard {}, "Dashboard", Some(PageIconKind::Dashboard)),
                            (Route::CollectionList {}, "Collections", Some(PageIconKind::Collections)),
                            (Route::Pantry {}, "Kitchen", None),
                            (Route::Explore {}, "Explore", Some(PageIconKind::Explore)),
                            (Route::Search { query: Default::default() }, "Search", Some(PageIconKind::Search)),
                            (Route::RecipeNew {}, "New recipe", Some(PageIconKind::Editor)),
                            (Route::About {}, "About", Some(PageIconKind::About)),
                            (Route::HowTo {}, "How to", Some(PageIconKind::HowTo)),
                        ] {
                            SectionLink {
                                class: if navigation_is_current(&route, &current_route) { "navbar-link navbar-link--current" } else { "navbar-link" },
                                current: navigation_is_current(&route, &current_route),
                                to: route, on_navigate: move |_| menu_open.set(false),
                                span { class: "navigation-artwork-slot", aria_hidden: "true",
                                    if let Some(kind) = icon { NavigationArtwork { kind: Some(kind) } }
                                }
                                span { "{label}" }
                            }
                        }
                    }
                    if is_signed_in {
                        Link { to: Route::Home {}, class: "navbar-link", onclick: move |_| menu_open.set(false), NavigationArtwork { kind: Some(PageIconKind::Home) } span { "Noms introduction" } }
                    }
                    div { class: "navigation-drawer-account",
                        if is_signed_in {
                            a { href: format!("/u/{}", auth.current_user.as_ref().map(|u| u.username.clone()).unwrap_or_default()),
                                class: "navbar-link", NavigationArtwork { kind: Some(PageIconKind::Profile) } span { "My profile" } }
                            Link { to: Route::SettingsProfile {}, class: "navbar-link", onclick: move |_| menu_open.set(false), NavigationArtwork { kind: Some(PageIconKind::Settings) } span { "Settings" } }
                            form { class: "navbar-logout-form", method: "post", action: "/auth/logout?redirect_uri=/",
                                button { r#type: "submit", class: "navbar-link",
                                    // Keep the form mounted until native POST completes.
                                    "Sign Out"
                                }
                            }
                        } else {
                            Link { to: Route::Login {}, class: "navbar-link", onclick: move |_| menu_open.set(false), "Sign In" }
                        }
                        button { r#type: "button", class: "navbar-link", onclick: move |_| theme.toggle(),
                            if theme.is_dark() { "☀️ Light Mode" } else { "🌙 Dark Mode" }
                        }
                    }
                }
            }
        }
    }
}

/// Reuse the landing page's canonical search route and ordinary browser history.
#[component]
fn NavigationSearch(compact: bool, #[props(default)] on_navigate: EventHandler<()>) -> Element {
    let mut text = use_signal(String::new);
    let mut submit = move |value: String| {
        text.set(value);
        let route = crate::pages::search::landing_search_route(&text()).unwrap_or(Route::Search {
            query: Default::default(),
        });
        navigator().push(route);
        on_navigate.call(());
    };
    rsx! {
        form {
            class: if compact { "navigation-search navigation-search--header" } else { "navigation-search" },
            role: "search", aria_label: if compact { "Site search" } else { "Navigation search" },
            method: "get", action: "/search",
            onsubmit: move |event| {
                event.prevent_default();
                submit(text());
            },
            crate::components::recipe_title_autocomplete::RecipeTitleAutocomplete {
                id: if compact { "site-search-query" } else { "drawer-search-query" },
                accessible_name: "Search Noms", placeholder: "Search Noms…", input_class: "",
                value: text(), oninput: move |value| text.set(value), onsubmit: submit,
            }
            button { r#type: "submit", aria_label: "Submit search",
                svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "currentColor", stroke_width: "2", "aria-hidden": "true",
                    circle { cx: "10", cy: "10", r: "6" }
                    path { d: "m15 15 5 5" }
                }
            }
        }
    }
}

fn primary_navigation(signed_in: bool) -> Vec<(Route, &'static str)> {
    if signed_in {
        vec![
            (Route::CommunityHome {}, "Home"),
            (Route::Dashboard {}, "Dashboard"),
            (Route::CollectionList {}, "Collections"),
            (Route::Pantry {}, "Kitchen"),
            (Route::Explore {}, "Explore"),
        ]
    } else {
        vec![
            (Route::Home {}, "Home"),
            (Route::Explore {}, "Explore"),
            (Route::About {}, "About"),
            (Route::HowTo {}, "How to"),
        ]
    }
}

fn navigation_is_current(destination: &Route, current: &Route) -> bool {
    match (destination, current) {
        (
            Route::CollectionList {},
            Route::CollectionList {}
            | Route::CollectionNew { .. }
            | Route::CollectionDetail { .. }
            | Route::CollectionEdit { .. },
        ) => true,
        (Route::Pantry {}, route) if super::kitchen::KitchenView::for_route(route).is_some() => {
            true
        }
        (Route::Search { .. }, Route::Search { .. }) => true,
        _ => destination == current,
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
