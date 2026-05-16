use dioxus::prelude::*;

/// Props for the [`EmptyState`] component.
#[derive(Props, Clone, PartialEq)]
pub struct EmptyStateProps {
    /// Icon element (e.g. an emoji or SVG).
    pub icon: Element,
    /// Short title describing the empty state.
    pub title: String,
    /// Longer description or hint.
    pub description: String,
    /// Optional action button or link.
    #[props(default)]
    pub action: Option<Element>,
}

/// A reusable "nothing here yet" message with optional call-to-action.
#[component]
pub fn EmptyState(props: EmptyStateProps) -> Element {
    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            align_items: "center",
            justify_content: "center",
            text_align: "center",
            padding: "var(--space-2xl) var(--space-md)",
            color: "var(--text-secondary)",
            div {
                font_size: "48px",
                margin_bottom: "var(--space-md)",
                {&props.icon}
            }
            h3 {
                color: "var(--text-primary)",
                margin_bottom: "var(--space-xs)",
                "{props.title}"
            }
            p {
                margin_bottom: "var(--space-lg)",
                max_width: "400px",
                "{props.description}"
            }
            if let Some(action) = props.action {
                {action}
            }
        }
    }
}
