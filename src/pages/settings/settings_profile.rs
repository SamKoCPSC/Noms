use dioxus::prelude::*;

use crate::components::base::{Card, Input, PageHeader};

/// User profile settings — placeholder.
#[component]
pub fn SettingsProfile() -> Element {
    rsx! {
        div { class: "container",
            PageHeader {
                title: "Profile Settings",
            }
            div {
                max_width: "500px",
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-md)",
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Display Name"
                            }
                            Input {
                                placeholder: "Your name",
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
                                "Bio"
                            }
                            textarea {
                                class: "neumo-inset input",
                                placeholder: "Tell us about yourself...",
                                rows: "3",
                                padding: "var(--space-sm) var(--space-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-primary)",
                                background_color: "var(--surface)",
                                outline: "none",
                                resize: "vertical",
                            }
                        }
                    }
                }
            }
        }
    }
}
