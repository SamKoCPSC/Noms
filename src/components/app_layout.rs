use dioxus::prelude::*;

use crate::auth::refresh::AuthRefresh;
use crate::auth::session_page::SessionPage;
use crate::components::{Footer, Navbar};
use crate::utils::theme::use_theme;

/// Shared application shell: Navbar → page content → Footer.
#[component]
pub fn AppLayout() -> Element {
    let theme = use_theme();

    rsx! {
        div {
            class: "app-shell",
            Navbar { theme }
            main {
                class: "main-content bg-gradient-animated",
                AuthRefresh {}
                SessionPage {}
            }
            Footer {}
        }
    }
}
