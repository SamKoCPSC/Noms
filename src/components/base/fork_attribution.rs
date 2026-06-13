//! Fork attribution bar displayed on recipe detail pages.
//!
//! Shows "Forked from Recipe" or "Variant of Recipe" with a clickable link
//! to the original recipe.

use dioxus::prelude::*;

/// Props for the [`ForkAttribution`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ForkAttributionProps {
    /// The recipe ID of the original (source) recipe.
    pub original_recipe_id: uuid::Uuid,
    /// The display name of the original recipe's owner.
    pub original_owner_name: String,
    /// Optional message left by the forker.
    pub message: Option<String>,
    /// If true, shows "Variant of" instead of "Forked from" (same-user fork).
    pub is_variant: bool,
}

/// A subtle bar indicating this recipe was forked from another.
#[component]
pub fn ForkAttribution(props: ForkAttributionProps) -> Element {
    let label = if props.is_variant {
        "Variant of"
    } else {
        "Forked from"
    };

    let owner_label = format!("{}'s recipe", props.original_owner_name);

    rsx! {
        div {
            class: "fork-attribution",
            "{label} "
            a {
                href: format!("/recipes/{}", props.original_recipe_id),
                class: "fork-attribution__link",
                "{owner_label}"
            }
            if let Some(msg) = props.message {
                span {
                    class: "fork-attribution__message",
                    "{msg}"
                }
            }
        }
    }
}
