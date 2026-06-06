use dioxus::prelude::*;

use crate::components::base::{EmptyState, PageHeader};

/// Public recipe discovery — placeholder.
#[component]
pub fn Explore() -> Element {
    rsx! {
        div { class: "container",
            PageHeader {
                title: "Explore",
            }
            EmptyState {
                icon: rsx! { "🔍" },
                title: "Discover recipes",
                description: "Browse recipes from the community. Coming soon!",
            }
        }
    }
}
