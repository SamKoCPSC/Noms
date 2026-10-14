use dioxus::prelude::*;

use crate::components::base::{CollectionCover, CollectionVisibilityBadge};
use crate::types::CollectionSummary;
use crate::Route;

#[derive(Props, Clone, PartialEq)]
pub struct CollectionAlbumProps {
    pub collection: CollectionSummary,
    #[props(default)]
    pub compact: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CollectionVisibility;
    use dioxus::router::components::HistoryProvider;
    use dioxus_core::NoOpMutations;

    #[derive(Clone, PartialEq, Routable)]
    enum AlbumTestRoute {
        #[route("/")]
        AlbumTestPage {},
    }

    #[component]
    fn AlbumTestPage() -> Element {
        let props = use_context::<CollectionAlbumProps>();
        rsx! { CollectionAlbum { collection: props.collection, compact: props.compact } }
    }

    #[allow(non_snake_case)]
    fn AlbumTestRoot(props: CollectionAlbumProps) -> Element {
        use_context_provider(|| props);
        rsx! {
            HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<AlbumTestRoute> {}
            }
        }
    }

    #[test]
    fn album_variants_render_real_counts_visibility_and_one_navigation_target() {
        for compact in [false, true] {
            let collection = CollectionSummary {
                id: uuid::Uuid::nil(),
                parent_collection_id: None,
                name: "Family & friends".into(),
                description: Some("Recipes <worth> keeping".into()),
                image: None,
                preview_images: vec![],
                visibility: CollectionVisibility::Private,
                sort_order: 0,
                child_count: 2,
                recipe_count: 7,
                updated_at: chrono::Utc::now(),
            };
            let mut dom = VirtualDom::new_with_props(
                AlbumTestRoot,
                CollectionAlbumProps {
                    collection,
                    compact,
                },
            );
            dom.rebuild(&mut NoOpMutations);
            let html = dioxus_ssr::render(&dom);
            assert_eq!(html.matches("<a ").count(), 1, "{html}");
            assert!(
                html.contains("/collections/00000000-0000-0000-0000-000000000000"),
                "{html}"
            );
            assert!(html.contains("7 recipes · 2 subcollections"), "{html}");
            assert!(html.contains("Collection visibility: Private"), "{html}");
            assert!(html.contains("collection-cover__fallback"), "{html}");
            assert!(html.contains("&#60;worth&#62;"), "{html}");
            assert!(html.contains(if compact { "<h3" } else { "<h2" }), "{html}");
            assert!(
                !html.contains("<button"),
                "reorder controls must remain outside the link: {html}"
            );
        }
    }
}

/// Counts and cover previews come from the permission-filtered API response.
#[component]
pub fn CollectionAlbum(props: CollectionAlbumProps) -> Element {
    let collection = props.collection;
    let class = if props.compact {
        "collection-album collection-album--compact"
    } else {
        "collection-album"
    };
    rsx! {
        Link {
            to: Route::CollectionDetail { id: collection.id.to_string() },
            class,
            CollectionCover {
                image: collection.image,
                preview_images: collection.preview_images,
                alt: format!("{} collection cover", collection.name),
            }
            div { class: "collection-album__body",
                span { class: "collection-album__bookmark", aria_hidden: "true" }
                if props.compact { h3 { "{collection.name}" } } else { h2 { "{collection.name}" } }
                p { class: "collection-album__counts", "{collection.recipe_count} recipes · {collection.child_count} subcollections" }
                if let Some(description) = collection.description {
                    p { class: "collection-album__description", "{description}" }
                }
                CollectionVisibilityBadge { visibility: collection.visibility }
            }
        }
    }
}
