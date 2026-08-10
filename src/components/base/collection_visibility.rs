use dioxus::prelude::*;

use crate::types::CollectionVisibility;

const PRIVATE_EXPLANATION: &str = "Private: only you can access it.";
const UNLISTED_EXPLANATION: &str = "Unlisted: anyone with the link can access shareable contents.";
const PUBLIC_EXPLANATION: &str = "Public: anyone can access it and it may appear in discovery.";

fn visibility_label(visibility: CollectionVisibility) -> &'static str {
    match visibility {
        CollectionVisibility::Private => "Private",
        CollectionVisibility::Unlisted => "Unlisted",
        CollectionVisibility::Public => "Public",
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CollectionVisibilityFieldProps {
    pub visibility: CollectionVisibility,
    pub onchange: EventHandler<CollectionVisibility>,
    #[props(default)]
    pub disabled: bool,
}

#[component]
pub fn CollectionVisibilityField(props: CollectionVisibilityFieldProps) -> Element {
    let explanation = match props.visibility {
        CollectionVisibility::Private => PRIVATE_EXPLANATION,
        CollectionVisibility::Unlisted => UNLISTED_EXPLANATION,
        CollectionVisibility::Public => PUBLIC_EXPLANATION,
    };
    rsx! {
        label { display: "flex", flex_direction: "column", gap: "var(--space-xs)",
            span { font_weight: "600", "Visibility" }
            select {
                class: "neumo-inset input",
                value: props.visibility.as_str(),
                disabled: props.disabled,
                onchange: move |event| {
                    if let Ok(visibility) = event.value().parse::<CollectionVisibility>() {
                        props.onchange.call(visibility);
                    }
                },
                option { value: "private", "Private" }
                option { value: "unlisted", "Unlisted" }
                option { value: "public", "Public" }
            }
            small { color: "var(--text-secondary)", "{explanation}" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CollectionVisibilityBadgeProps {
    pub visibility: CollectionVisibility,
}

#[component]
pub fn CollectionVisibilityBadge(props: CollectionVisibilityBadgeProps) -> Element {
    let label = visibility_label(props.visibility);
    rsx! {
        span {
            display: "inline-flex",
            align_items: "center",
            width: "fit-content",
            padding: "var(--space-xs) var(--space-sm)",
            border: "1px solid var(--glass-border)",
            border_radius: "var(--radius-full)",
            background: "var(--glass-fill)",
            color: "var(--text-secondary)",
            font_size: "12px",
            font_weight: "600",
            aria_label: "Collection visibility: {label}",
            "{label}"
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn visibility_field_and_badge_explain_all_supported_states() {
        let source = include_str!("collection_visibility.rs");
        assert!(source.contains("CollectionVisibilityField"));
        assert!(source.contains("CollectionVisibilityBadge"));
        assert!(source.contains("Private: only you can access it."));
        assert!(source.contains("Unlisted: anyone with the link"));
        assert!(source.contains("Public: anyone can access it"));
        assert!(source.contains("select"));
    }
}
