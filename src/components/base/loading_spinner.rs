use dioxus::prelude::*;

/// A simple CSS-animated loading spinner.
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div {
            class: "spinner",
            role: "status",
            aria_label: "Loading",
        }
    }
}
