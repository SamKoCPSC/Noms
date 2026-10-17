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
    #[cfg(target_arch = "wasm32")]
    use_effect(|| {
        spawn(async {
            let module = asset!("/assets/mobile-runtime.mjs").to_string();
            let script = format!(
                "import({}).then(module => module.mountMobileViewport())",
                serde_json::to_string(&module).unwrap()
            );
            let _ = document::eval(&script).await;
        });
    });
    #[cfg(target_arch = "wasm32")]
    use_drop(|| {
        spawn(async {
            let _ = document::eval("window.nomsMobileViewportCleanup?.()").await;
        });
    });

    rsx! {
        div {
            class: if crate::components::mobile::focused_route(&route) { "app-shell mobile-focused" } else { "app-shell mobile-tabbed" },
            Navbar { theme }
            main {
                class: "main-content bg-gradient-animated",
                AuthRefresh {}
                SessionPage {}
            }
            Footer {}
            crate::components::mobile::MobileNavigation {}
        }
    }
}
