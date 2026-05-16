use dioxus::prelude::*;

use crate::components::base::{Button, ButtonVariant, Input, PageHeader};

/// Create recipe page — empty form shell.
#[component]
pub fn RecipeNew() -> Element {
    rsx! {
        div { class: "container",
            PageHeader {
                title: "New Recipe",
            }
            div {
                display: "flex",
                flex_direction: "column",
                gap: "var(--space-md)",
                max_width: "600px",
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-sm)",
                    label {
                        font_size: "14px",
                        font_weight: "600",
                        color: "var(--text-secondary)",
                        "Recipe Name"
                    }
                    Input {
                        placeholder: "e.g. Grandma's Chocolate Chip Cookies",
                        oninput: move |_| {},
                    }
                }
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-sm)",
                    label {
                        font_size: "14px",
                        font_weight: "600",
                        color: "var(--text-secondary)",
                        "Description"
                    }
                    textarea {
                        class: "neumo-inset input",
                        placeholder: "Brief description of the recipe...",
                        rows: "4",
                        padding: "var(--space-sm) var(--space-md)",
                        font_family: "var(--font-body)",
                        font_size: "14px",
                        color: "var(--text-primary)",
                        background_color: "var(--surface)",
                        outline: "none",
                        resize: "vertical",
                    }
                }
                div {
                    display: "flex",
                    gap: "var(--space-md)",
                    margin_top: "var(--space-md)",
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| {},
                        "Save Recipe"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {},
                        "Cancel"
                    }
                }
            }
        }
    }
}
