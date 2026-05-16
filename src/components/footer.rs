use dioxus::prelude::*;

/// Minimal footer with copyright and credit.
#[component]
pub fn Footer() -> Element {
    rsx! {
        footer {
            class: "footer",
            div { class: "container",
                p { "© 2026 Noms. Built with Dioxus." }
            }
        }
    }
}
