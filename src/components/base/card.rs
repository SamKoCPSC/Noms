use dioxus::prelude::*;

/// Props for the [`Card`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CardProps {
    /// Card content.
    pub children: Element,
}

/// A neumorphic card container with padding.
#[component]
pub fn Card(props: CardProps) -> Element {
    rsx! {
        div {
            class: "neumo-card",
            padding: "var(--space-lg)",
            background_color: "var(--surface)",
            {&props.children}
        }
    }
}
