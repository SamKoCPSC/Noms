use super::{PageIcon, PageIconKind};
use dioxus::prelude::*;

/// Props for the [`PageHeader`] component.
#[derive(Props, Clone, PartialEq)]
pub struct PageHeaderProps {
    /// Page title.
    pub title: String,
    /// Optional action element (e.g. a button) displayed to the right.
    #[props(default)]
    pub action: Option<Element>,
    /// Optional painted identity; the title remains the accessible heading.
    #[props(default)]
    pub icon: Option<PageIconKind>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painted_header_preserves_heading_text_and_action() {
        let html = dioxus_ssr::render_element(rsx! {
            PageHeader {
                title: "Recipe version graph",
                icon: Some(PageIconKind::History),
                action: Some(rsx! { button { "Back to recipe" } }),
            }
        });
        assert!(html.contains("history-v1"), "{html}");
        assert!(
            html.contains("<span>Recipe version graph</span></h1>"),
            "{html}"
        );
        assert!(html.contains("<button>Back to recipe</button>"), "{html}");
        assert!(html.contains("alt=\"\""), "{html}");
    }
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
                class: if props.icon.is_some() { "brand-title" } else { "" },
                font_size: "28px",
                color: "var(--text-primary)",
                if let Some(kind) = props.icon { PageIcon { kind } }
                span { "{props.title}" }
            }
            if let Some(action) = props.action {
                {action}
            }
        }
    }
}
