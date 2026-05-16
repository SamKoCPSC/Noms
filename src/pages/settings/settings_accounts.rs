use dioxus::prelude::*;

use crate::components::base::{Button, ButtonVariant, EmptyState, PageHeader};

/// Linked OAuth accounts — placeholder.
#[component]
pub fn SettingsAccounts() -> Element {
    rsx! {
        div { class: "container",
            PageHeader {
                title: "Linked Accounts",
            }
            EmptyState {
                icon: rsx! { "🔗" },
                title: "No linked accounts",
                description: "Connect your Google or GitHub account for easy sign-in.",
                action: rsx! {
                    div {
                        display: "flex",
                        gap: "var(--space-md)",
                        flex_wrap: "wrap",
                        justify_content: "center",
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| {},
                            "Connect Google"
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| {},
                            "Connect GitHub"
                        }
                    }
                },
            }
        }
    }
}
