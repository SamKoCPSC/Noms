use dioxus::prelude::*;

use crate::components::base::{EmptyState, PageHeader};
use crate::components::AuthRequired;

/// Public recipe discovery — placeholder.
#[component]
pub fn Explore() -> Element {
    rsx! {
        AuthRequired {
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
}
