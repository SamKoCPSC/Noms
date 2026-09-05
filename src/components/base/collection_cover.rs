use dioxus::prelude::*;

use crate::components::base::EditedRecipeImage;
use crate::types::RecipeImage;

#[derive(Props, Clone, PartialEq)]
pub struct CollectionCoverProps {
    pub image: Option<RecipeImage>,
    #[props(default)]
    pub preview_images: Vec<RecipeImage>,
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
            } else if !props.preview_images.is_empty() {
                CollectionRecipeMosaic {
                    images: props.preview_images.into_iter().take(4).collect(),
                    alt: props.alt,
                }
            } else {
                span { class: "collection-cover__fallback", aria_hidden: "true", "📁" }
            }
        }
    }
}

fn mosaic_layout_class(image_count: usize) -> &'static str {
    match image_count {
        1 => "collection-cover__mosaic--one",
        2 => "collection-cover__mosaic--two",
        3 => "collection-cover__mosaic--three",
        _ => "collection-cover__mosaic--four",
    }
}

fn mosaic_tile_key(index: usize, image: &RecipeImage) -> String {
    format!("{index}:{}", image.url)
}

#[derive(Props, Clone, PartialEq)]
struct CollectionRecipeMosaicProps {
    images: Vec<RecipeImage>,
    alt: String,
}

#[component]
fn CollectionRecipeMosaic(props: CollectionRecipeMosaicProps) -> Element {
    let layout_class = mosaic_layout_class(props.images.len());
    rsx! {
        div {
            class: "collection-cover__mosaic {layout_class}",
            role: "img",
            aria_label: "{props.alt}",
            for (tile_key, image) in props.images.into_iter().enumerate().map(|(index, image)| {
                (mosaic_tile_key(index, &image), image)
            }) {
                CollectionRecipeMosaicTile {
                    key: "{tile_key}",
                    image,
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct CollectionRecipeMosaicTileProps {
    image: RecipeImage,
}

/// A broken recipe image only affects its tile, leaving the rest of the mosaic intact.
#[component]
fn CollectionRecipeMosaicTile(props: CollectionRecipeMosaicTileProps) -> Element {
    let mut image_failed = use_signal(|| false);
    rsx! {
        div { class: "collection-cover__mosaic-tile", aria_hidden: "true",
            if image_failed() {
                span { class: "collection-cover__tile-fallback", aria_hidden: "true", "📁" }
            } else {
                EditedRecipeImage {
                    image: props.image,
                    alt: String::new(),
                    prefer_thumbnail: true,
                    on_error: move |_| image_failed.set(true),
                }
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

#[cfg(test)]
mod tests {
    use super::{mosaic_layout_class, mosaic_tile_key};
    use crate::types::RecipeImage;

    #[test]
    fn recipe_mosaic_layout_adapts_to_one_through_four_images() {
        assert_eq!(mosaic_layout_class(1), "collection-cover__mosaic--one");
        assert_eq!(mosaic_layout_class(2), "collection-cover__mosaic--two");
        assert_eq!(mosaic_layout_class(3), "collection-cover__mosaic--three");
        assert_eq!(mosaic_layout_class(4), "collection-cover__mosaic--four");
        assert_eq!(mosaic_layout_class(8), "collection-cover__mosaic--four");
    }

    #[test]
    fn recipe_mosaic_has_one_accessible_label_and_decorative_tiles() {
        let source = include_str!("collection_cover.rs");
        assert!(source.contains("role: \"img\""));
        assert!(source.contains("aria_label: \"{props.alt}\""));
        assert!(source.contains("aria_hidden: \"true\""));
        assert!(source.contains("alt: String::new()"));
    }

    #[test]
    fn recipe_mosaic_keys_remain_unique_when_recipes_share_an_image() {
        let image = RecipeImage {
            asset_id: None,
            url: "https://example.com/shared.webp".to_string(),
            thumb_url: None,
            file_name: None,
            file_size: None,
            edits: Vec::new(),
        };

        assert_ne!(mosaic_tile_key(0, &image), mosaic_tile_key(1, &image));
    }

    #[test]
    fn collection_detail_hero_preserves_the_sixteen_by_nine_cover_ratio() {
        let styles = include_str!("../../../assets/main.css");
        let hero_rule = styles
            .split(".collection-cover--hero {")
            .nth(1)
            .and_then(|remaining| remaining.split('}').next())
            .expect("collection cover hero styles should exist");

        assert!(hero_rule.contains("aspect-ratio: 16 / 9;"));
    }
}
