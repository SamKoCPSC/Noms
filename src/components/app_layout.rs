use dioxus::prelude::*;

use crate::components::{Footer, Navbar};
use crate::utils::theme::use_theme;
use crate::Route;

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
                Outlet::<Route> {}
            }
            Footer {}
        }
    }
}
