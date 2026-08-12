use dioxus::prelude::*;

use crate::components::base::EditedRecipeImage;
use crate::types::RecipeImage;

#[derive(Props, Clone, PartialEq)]
pub struct CollectionCoverProps {
    pub image: Option<RecipeImage>,
    pub alt: String,
    #[props(default)]
    pub class: String,
}

/// A collection cover with a stable folder fallback for absent or broken images.
#[component]
pub fn CollectionCover(props: CollectionCoverProps) -> Element {
    let class = if props.class.is_empty() {
        "collection-cover".to_string()
    } else {
        format!("collection-cover {}", props.class)
    };

    rsx! {
        div { class: "{class}",
            if let Some(image) = props.image {
                CollectionCoverImage {
                    key: "{image.url}",
                    image,
                    alt: props.alt,
                }
            } else {
                span { class: "collection-cover__fallback", aria_hidden: "true", "📁" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CollectionCoverImageProps {
    image: RecipeImage,
    alt: String,
}

/// Keying this component by URL resets its failure state when a cover changes.
#[component]
fn CollectionCoverImage(props: CollectionCoverImageProps) -> Element {
    let mut image_failed = use_signal(|| false);
    if image_failed() {
        return rsx! {
            span { class: "collection-cover__fallback", aria_hidden: "true", "📁" }
        };
    }

    rsx! {
        EditedRecipeImage {
            image: props.image,
            alt: props.alt,
            prefer_thumbnail: true,
            on_error: move |_| image_failed.set(true),
        }
    }
}
