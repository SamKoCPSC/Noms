use dioxus::prelude::*;

use crate::components::base::{Card, LoadingSpinner, PageHeader};
use crate::components::AuthRequired;

/// Single collection view — placeholder.
#[component]
pub fn CollectionDetail(id: i32) -> Element {
    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "Collection #{id}",
                }
                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "200px",
                    Card {
                        LoadingSpinner {}
                        p {
                            margin_top: "var(--space-md)",
                            color: "var(--text-secondary)",
                            "Collection content will appear here."
                        }
                    }
                }
            }
        }
    }
}
