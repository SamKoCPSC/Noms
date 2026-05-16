use dioxus::prelude::*;

/// Props for the [`PageHeader`] component.
#[derive(Props, Clone, PartialEq)]
pub struct PageHeaderProps {
    /// Page title.
    pub title: String,
    /// Optional action element (e.g. a button) displayed to the right.
    #[props(default)]
    pub action: Option<Element>,
}

/// A consistent page header with a title and optional action slot.
#[component]
pub fn PageHeader(props: PageHeaderProps) -> Element {
    rsx! {
        div {
            display: "flex",
            align_items: "center",
            justify_content: "space-between",
            flex_wrap: "wrap",
            gap: "var(--space-md)",
            margin_bottom: "var(--space-lg)",
            h1 {
                font_size: "28px",
                color: "var(--text-primary)",
                "{props.title}"
            }
            if let Some(action) = props.action {
                {action}
            }
        }
    }
}
