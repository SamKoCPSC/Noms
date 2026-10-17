//! Mobile presentation shares the existing routes, session and write handlers.
use crate::auth::context::use_auth;
use crate::components::base::{Avatar, AvatarSize, NavigationArtwork, PageIconKind, SharedModal};
use crate::utils::theme::UseTheme;
use crate::Route;
use dioxus::prelude::*;

pub fn focused_route(route: &Route) -> bool {
    matches!(
        route,
        Route::Home {}
            | Route::Login {}
            | Route::About {}
            | Route::HowTo {}
            | Route::RecipeNew {}
            | Route::RecipeImport {}
            | Route::RecipeEdit { .. }
            | Route::RecipeDraftEdit { .. }
            | Route::RecipeDetail { .. }
            | Route::RecipeVersionDetail { .. }
            | Route::RecipeVersionGraph { .. }
            | Route::CollectionNew { .. }
            | Route::CollectionEdit { .. }
            | Route::NotFound { .. }
    )
}

pub fn is_mobile_layout() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.inner_width().ok())
            .and_then(|width| width.as_f64())
            .is_some_and(|width| width <= 760.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn route_heading(route: &Route) -> Option<(&'static str, Route)> {
    match route {
        Route::Login {} => Some(("Sign in", Route::Home {})),
        Route::About {} => Some(("About Noms", Route::MobileYou {})),
        Route::HowTo {} => Some(("How to use Noms", Route::MobileYou {})),
        Route::RecipeNew {} => Some(("New recipe", Route::Dashboard {})),
        Route::RecipeImport {} => Some(("Import recipe", Route::Dashboard {})),
        Route::RecipeEdit { id } => Some(("Edit recipe", Route::RecipeDetail { id: id.clone() })),
        Route::RecipeDraftEdit { .. } => Some(("Edit private draft", Route::Dashboard {})),
        Route::RecipeDetail { .. } => Some(("Recipe", Route::Explore {})),
        Route::RecipeVersionDetail { id, .. } => {
            Some(("Earlier version", Route::RecipeDetail { id: id.clone() }))
        }
        Route::RecipeVersionGraph { id } => {
            Some(("Version history", Route::RecipeDetail { id: id.clone() }))
        }
        Route::CollectionNew { parent } => Some((
            "New collection",
            parent
                .map(|id| Route::CollectionDetail { id: id.to_string() })
                .unwrap_or(Route::CollectionList {}),
        )),
        Route::CollectionEdit { id } => Some((
            "Edit collection",
            Route::CollectionDetail { id: id.clone() },
        )),
        Route::CollectionDetail { .. } => Some(("Collection", Route::CollectionList {})),
        Route::SettingsProfile {} => Some(("Profile settings", Route::MobileYou {})),
        Route::SettingsAccounts {} => Some(("Linked accounts", Route::MobileYou {})),
        Route::UserProfile { .. } => Some(("Cook profile", Route::CommunityHome {})),
        _ => None,
    }
}

fn selected_tab(route: &Route) -> &'static str {
    match route {
        Route::Dashboard {} | Route::CollectionList {} | Route::CollectionDetail { .. } => {
            "Library"
        }
        Route::Explore {} | Route::Search { .. } => "Explore",
        Route::MobileYou {}
        | Route::SettingsProfile {}
        | Route::SettingsAccounts {}
        | Route::UserProfile { .. } => "You",
        _ => "Home",
    }
}

#[component]
pub fn MobileIcon(kind: &'static str) -> Element {
    let path = match kind {
        "home" => "M3 10 12 3l9 7v11h-6v-7H9v7H3Z",
        "explore" => "m16 8-3 5-5 3 3-5z M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20",
        "library" => "M4 4h4v17H4z M10 4h4v17h-4z M17 4l4 16-4 1-4-16",
        "user" => "M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0 M4 21v-2a8 5 0 0 1 16 0v2",
        "back" => "m14 6-6 6 6 6",
        "search" => "M20 20l-5-5 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
        "plus" => "M12 5v14 M5 12h14",
        _ => "M4 6h16 M4 12h16 M4 18h16",
    };
    rsx! { svg { view_box: "0 0 24 24", width: "22", height: "22", fill: "none", stroke: "currentColor", stroke_width: "1.7", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true", path { d: path } } }
}

#[component]
pub fn MobileNavigation() -> Element {
    let route = use_route::<Route>();
    let auth = use_auth();
    if focused_route(&route) {
        return rsx! {};
    }
    let active = selected_tab(&route);
    rsx! {
        nav { class: "mobile-bottom-nav", aria_label: "Mobile navigation",
            for (label, kind, destination) in [
                ("Home", "home", if auth.is_authenticated { Route::CommunityHome {} } else { Route::Home {} }),
                ("Explore", "explore", Route::Explore {}), ("Library", "library", Route::Dashboard {}), ("You", "user", Route::MobileYou {}),
            ] {
                Link { to: destination, class: if active == label { "mobile-tab mobile-tab--active" } else { "mobile-tab" }, aria_current: if active == label { "page" } else { "false" },
                    span { class: "mobile-tab-icon", MobileIcon { kind } } span { "{label}" }
                }
            }
        }
    }
}

#[component]
pub fn MobileCreateButton() -> Element {
    let mut open = use_signal(|| false);
    let route = use_route::<Route>();
    use_effect(use_reactive(&route, move |_| open.set(false)));
    rsx! {
        button { r#type: "button", class: "mobile-only mobile-create-button btn btn-primary touch-target", aria_label: "Add to your library", aria_haspopup: "dialog", onclick: move |_| open.set(true), MobileIcon { kind: "plus" } }
        if open() {
            SharedModal { aria_labelledby: "mobile-create-title", focus_selector: "#mobile-create-close", on_dismiss: move |_| open.set(false),
                div { class: "modal-card mobile-action-sheet", onclick: move |event| event.stop_propagation(),
                    div { class: "mobile-sheet-heading", h2 { id: "mobile-create-title", "Make something yours." } button { id: "mobile-create-close", class: "btn btn-ghost touch-target", onclick: move |_| open.set(false), aria_label: "Close creation menu", "✕" } }
                    for (label, detail, kind, target) in [
                        ("New recipe", "Start with a title and a little inspiration.", PageIconKind::Editor, Route::RecipeNew {}),
                        ("Import recipe", "Paste content from a recipe page.", PageIconKind::Import, Route::RecipeImport {}),
                        ("New collection", "Give your recipes a place.", PageIconKind::Collections, Route::CollectionNew { parent: None }),
                    ] {
                        Link { to: target, class: "mobile-hub-row", onclick: move |_| open.set(false), NavigationArtwork { kind: Some(kind) } div { strong { "{label}" } small { "{detail}" } } span { aria_hidden: "true", "›" } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MobileYou() -> Element {
    let auth = use_auth();
    let theme = use_context::<UseTheme>();
    rsx! {
        div { class: "container mobile-you-page",
            header { class: "mobile-you-heading",
                if let Some(user) = &auth.current_user {
                    Avatar { size: AvatarSize::Large, src: user.avatar_url.clone(), username: user.username.clone() }
                    div { h1 { if user.display_name.is_empty() { "{user.username}" } else { "{user.display_name}" } } p { "@{user.username}" } Link { to: Route::UserProfile { username: user.username.clone() }, "View public profile →" } }
                } else {
                    div { h1 { "Make yourself at home." } p { "Sign in to keep your recipes and make them your own." } Link { to: Route::Login {}, class: "btn btn-primary touch-target", "Sign in" } }
                }
            }
            if auth.is_authenticated {
                section { class: "neumo-card mobile-hub-group", aria_label: "Your account",
                    Link { to: Route::SettingsProfile {}, class: "mobile-hub-row", NavigationArtwork { kind: Some(PageIconKind::Profile) } div { strong { "Profile settings" } small { "Your name, photo, and bio" } } span { aria_hidden: "true", "›" } }
                    Link { to: Route::SettingsAccounts {}, class: "mobile-hub-row", NavigationArtwork { kind: Some(PageIconKind::Settings) } div { strong { "Linked accounts" } small { "Manage your ways to sign in" } } span { aria_hidden: "true", "›" } }
                }
            }
            section { class: "neumo-card mobile-hub-group", aria_label: "Appearance and help",
                button { class: "mobile-hub-row", r#type: "button", role: "switch", aria_checked: theme.is_dark(), onclick: move |_| theme.toggle(), NavigationArtwork { kind: Some(PageIconKind::Settings) } div { strong { "Dark appearance" } small { if theme.is_dark() { "On" } else { "Off" } } } span { aria_hidden: "true", if theme.is_dark() { "☾" } else { "☼" } } }
                for (label, detail, kind, destination) in [
                    ("How to use Noms", "Create, organize, and evolve recipes", PageIconKind::HowTo, Route::HowTo {}), ("About Noms", "Good recipes deserve a home", PageIconKind::About, Route::About {}),
                ] { Link { to: destination, class: "mobile-hub-row", NavigationArtwork { kind: Some(kind) } div { strong { "{label}" } small { "{detail}" } } span { aria_hidden: "true", "›" } } }
                Link { to: Route::Home {}, class: "mobile-hub-row", div { strong { "Noms introduction" } small { "The public welcome page" } } span { aria_hidden: "true", "›" } }
            }
            if auth.is_authenticated { form { method: "post", action: "/auth/logout?redirect_uri=/", button { r#type: "submit", class: "btn btn-secondary touch-target mobile-you-signout", "Sign Out" } } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focused_flows_do_not_compete_with_global_tabs() {
        for route in [
            Route::RecipeNew {},
            Route::RecipeDraftEdit { id: "draft".into() },
            Route::RecipeDetail {
                id: "recipe".into(),
            },
            Route::CollectionNew { parent: None },
        ] {
            assert!(focused_route(&route));
        }
        assert!(!focused_route(&Route::Dashboard {}));
        assert!(!focused_route(&Route::MobileYou {}));
        assert_eq!(
            selected_tab(&Route::CollectionDetail { id: "album".into() }),
            "Library"
        );
        assert_eq!(selected_tab(&Route::SettingsAccounts {}), "You");
    }
}
