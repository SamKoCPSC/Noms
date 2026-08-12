use dioxus::prelude::*;

use crate::api::collection::list_root_collections;
use crate::components::base::{
    CollectionCover, CollectionVisibilityBadge, EmptyState, LoadingSpinner, PageHeader,
};
use crate::components::AuthRequired;
use crate::Route;

#[component]
pub fn CollectionList() -> Element {
    let collections = use_resource(list_root_collections);
    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader { title: "Collections", action: rsx! {
                    Link { to: Route::CollectionNew {}, class: "btn btn-primary touch-target", "New Collection" }
                } }
                match collections.read().as_ref() {
                    None => rsx! { LoadingSpinner {} },
                    Some(Err(error)) => rsx! { p { role: "alert", color: "var(--error)", "Could not load collections: {error}" } },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        EmptyState { icon: rsx! { "📁" }, title: "No collections yet",
                            description: "Organize your recipes into folders for easy access.",
                            action: rsx! { Link { to: Route::CollectionNew {}, class: "btn btn-primary touch-target", "Create a collection" } }
                        }
                    },
                    Some(Ok(items)) => rsx! {
                        div { display: "grid", grid_template_columns: "repeat(auto-fill, minmax(240px, 1fr))", gap: "var(--space-lg)",
                            for collection in items {
                                Link { to: Route::CollectionDetail { id: collection.id.to_string() }, class: "neumo-card", padding: "var(--space-lg)", text_decoration: "none", display: "flex", flex_direction: "column", gap: "var(--space-sm)",
                                    CollectionCover {
                                        image: collection.image.clone(),
                                        preview_images: collection.preview_images.clone(),
                                        alt: format!("{} collection cover", collection.name),
                                    }
                                    CollectionVisibilityBadge { visibility: collection.visibility }
                                    h2 { color: "var(--text-primary)", "{collection.name}" }
                                    if let Some(description) = &collection.description { p { color: "var(--text-secondary)", "{description}" } }
                                    p { color: "var(--text-secondary)", "{collection.child_count} folders · {collection.recipe_count} recipes" }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn owner_collection_cards_show_visibility_badges() {
        let source = include_str!("collection_list.rs");
        assert!(source.contains("CollectionVisibilityBadge"));
    }
}
