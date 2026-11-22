use dioxus::prelude::*;

use crate::auth::refresh::AuthRefresh;
use crate::auth::session_page::SessionPage;
use crate::components::{Footer, Navbar};
use crate::utils::theme::use_theme;

/// Shared application shell: Navbar → page content → Footer.
#[component]
pub fn AppLayout() -> Element {
    let theme = use_theme();
    use_context_provider(|| theme);
    let route = use_route::<crate::Route>();
    // The hook owns the registrations until this layout unmounts.
    #[cfg(target_arch = "wasm32")]
    let _mobile_viewport = use_hook(|| std::rc::Rc::new(crate::utils::mobile_viewport::mount()));

    rsx! {
        div {
            class: if crate::components::mobile::focused_route(&route) { "app-shell mobile-focused" } else { "app-shell mobile-tabbed" },
            Navbar { theme }
            main {
                class: "main-content bg-gradient-animated",
                if try_use_context::<crate::auth::staging_context::StagingEnvironment>().unwrap_or_default().0 {
                    div { class: "staging-environment-banner", role: "note", "Staging environment — test data only" }
                }
                AuthRefresh {}
                SessionPage {}
            }
            Footer {}
            crate::components::mobile::MobileNavigation {}
        }
    }
}
