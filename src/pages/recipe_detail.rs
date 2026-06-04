use dioxus::prelude::*;

use crate::components::base::{Card, LoadingSpinner, PageHeader};
use crate::components::AuthRequired;

/// Single recipe view — placeholder.
#[component]
pub fn RecipeDetail(id: i32) -> Element {
    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "Recipe #{id}",
                }
                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "300px",
                    Card {
                        LoadingSpinner {}
                        p {
                            margin_top: "var(--space-md)",
                            color: "var(--text-secondary)",
                            "Recipe content will appear here."
                        }
                    }
                }
            }
        }
    }
}
