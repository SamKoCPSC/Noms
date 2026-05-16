use dioxus::prelude::*;

use crate::components::base::{EmptyState, PageHeader};

/// Collection browser — placeholder.
#[component]
pub fn CollectionList() -> Element {
    rsx! {
        div { class: "container",
            PageHeader {
                title: "Collections",
            }
            EmptyState {
                icon: rsx! { "📁" },
                title: "No collections yet",
                description: "Organize your recipes into collections for easy access.",
            }
        }
    }
}
