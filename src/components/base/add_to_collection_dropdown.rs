use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::collection::{
    add_recipe_to_collection, get_recipe_collections, load_all_my_collections,
    remove_recipe_from_collection,
};
use crate::types::CollectionSummary;

const COLLECTION_DROPDOWN_SURFACE: &str = "var(--surface)";

/// Build stable root-to-leaf labels without trusting hierarchy data to be complete or acyclic.
fn collection_breadcrumb_labels(collections: &[CollectionSummary]) -> HashMap<Uuid, String> {
    let collections_by_id = collections
        .iter()
        .map(|collection| (collection.id, collection))
        .collect::<HashMap<_, _>>();

    collections
        .iter()
        .map(|collection| {
            let mut path = Vec::new();
            let mut current = Some(collection);
            let mut visited = HashSet::new();

            while let Some(folder) = current {
                if !visited.insert(folder.id) {
                    break;
                }
                path.push(folder.name.clone());
                current = folder
                    .parent_collection_id
                    .and_then(|parent_id| collections_by_id.get(&parent_id).copied());
            }

            path.reverse();
            (collection.id, path.join(" / "))
        })
        .collect()
}

fn sort_collections_by_breadcrumb_path(
    mut collections: Vec<CollectionSummary>,
) -> Vec<CollectionSummary> {
    let labels = collection_breadcrumb_labels(&collections);
    collections.sort_by_cached_key(|collection| {
        labels
            .get(&collection.id)
            .map(|label| label.to_lowercase())
            .unwrap_or_else(|| collection.name.to_lowercase())
    });
    collections
}

#[derive(Props, Clone, PartialEq)]
pub struct AddToCollectionDropdownProps {
    pub recipe_id: String,
}

/// Membership picker for the authenticated viewer's private collections.
#[component]
pub fn AddToCollectionDropdown(props: AddToCollectionDropdownProps) -> Element {
    let recipe_id = props.recipe_id;
    let recipe_id_for_resource = recipe_id.clone();
    let mut membership = use_resource(move || {
        let recipe_id = recipe_id_for_resource.clone();
        async move {
            let collections = load_all_my_collections().await?;
            let memberships = get_recipe_collections(recipe_id).await?;
            Ok::<_, ServerFnError>((collections, memberships))
        }
    });
    let mut saving_collection = use_signal(|| None::<Uuid>);
    let mut save_error = use_signal(|| None::<String>);

    let loaded = membership.read().clone();

    rsx! {
        details {
            position: "relative",
            summary {
                class: "btn btn-secondary touch-target",
                cursor: "pointer",
                "Add to Collection"
            }
            div {
                class: "neumo-card collection-membership-dropdown",
                background_color: COLLECTION_DROPDOWN_SURFACE,
                position: "absolute",
                right: "0",
                z_index: "20",
                width: "min(360px, calc(100vw - 2rem))",
                max_height: "360px",
                overflow_y: "auto",
                padding: "var(--space-md)",
                margin_top: "var(--space-xs)",
                match loaded {
                    None => rsx! {
                        p { role: "status", "Loading collections…" }
                    },
                    Some(Err(error)) => rsx! {
                        p {
                            role: "alert",
                            color: "var(--error)",
                            "Could not load collections: {error}"
                        }
                    },
                    Some(Ok((collections, memberships))) => {
                        let collections = sort_collections_by_breadcrumb_path(collections);
                        let labels = collection_breadcrumb_labels(&collections);
                        let memberships = memberships.into_iter().collect::<HashSet<_>>();
                        if collections.is_empty() {
                            rsx! {
                                p { "Create a collection before adding this recipe." }
                                Link {
                                    to: crate::Route::CollectionNew {},
                                    class: "btn btn-secondary touch-target",
                                    "New Collection"
                                }
                            }
                        } else {
                            rsx! {
                                fieldset {
                                    border: "0",
                                    padding: "0",
                                    margin: "0",
                                    disabled: saving_collection().is_some(),
                                    legend {
                                        font_weight: "600",
                                        margin_bottom: "var(--space-sm)",
                                        "Collections"
                                    }
                                    div {
                                        display: "flex",
                                        flex_direction: "column",
                                        gap: "var(--space-xs)",
                                        for collection in collections {
                                            {
                                                let collection_id = collection.id;
                                                let collection_id_string = collection_id.to_string();
                                                let recipe_id = recipe_id.clone();
                                                let is_member = memberships.contains(&collection_id);
                                                let label = labels
                                                    .get(&collection_id)
                                                    .cloned()
                                                    .unwrap_or(collection.name);
                                                rsx! {
                                                    label {
                                                        key: "{collection_id}",
                                                        display: "flex",
                                                        align_items: "center",
                                                        gap: "var(--space-sm)",
                                                        min_height: "44px",
                                                        input {
                                                            r#type: "checkbox",
                                                            checked: is_member,
                                                            disabled: saving_collection().is_some(),
                                                            onclick: move |event| {
                                                                // Keep the controlled checkbox unchanged until the server confirms
                                                                // the session-authorized membership mutation.
                                                                event.prevent_default();
                                                                saving_collection.set(Some(collection_id));
                                                                save_error.set(None);
                                                                let collection_id = collection_id_string.clone();
                                                                let recipe_id = recipe_id.clone();
                                                                spawn(async move {
                                                                    let result = if is_member {
                                                                        remove_recipe_from_collection(collection_id, recipe_id).await
                                                                    } else {
                                                                        add_recipe_to_collection(collection_id, recipe_id).await
                                                                    };
                                                                    match result {
                                                                        Ok(()) => membership.restart(),
                                                                        Err(error) => save_error.set(Some(error.to_string())),
                                                                    }
                                                                    saving_collection.set(None);
                                                                });
                                                            }
                                                        }
                                                        span { "{label}" }
                                                        if saving_collection() == Some(collection_id) {
                                                            span {
                                                                role: "status",
                                                                color: "var(--text-secondary)",
                                                                "Saving…"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(error) = save_error() {
                    p {
                        role: "alert",
                        color: "var(--error)",
                        margin_top: "var(--space-sm)",
                        "Could not update collection membership: {error}"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{
        collection_breadcrumb_labels, sort_collections_by_breadcrumb_path,
        COLLECTION_DROPDOWN_SURFACE,
    };
    use crate::types::CollectionSummary;

    fn summary(id: Uuid, parent_collection_id: Option<Uuid>, name: &str) -> CollectionSummary {
        CollectionSummary {
            id,
            parent_collection_id,
            name: name.to_string(),
            description: None,
            image: None,
            preview_images: Vec::new(),
            visibility: crate::types::CollectionVisibility::Private,
            sort_order: 0,
            child_count: 0,
            recipe_count: 0,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn collection_labels_distinguish_same_named_folders_by_breadcrumb() {
        let meals = Uuid::new_v4();
        let favorites = Uuid::new_v4();
        let meals_weeknight = Uuid::new_v4();
        let favorites_weeknight = Uuid::new_v4();

        let labels = collection_breadcrumb_labels(&[
            summary(meals, None, "Meals"),
            summary(favorites, None, "Favorites"),
            summary(meals_weeknight, Some(meals), "Weeknight"),
            summary(favorites_weeknight, Some(favorites), "Weeknight"),
        ]);

        assert_eq!(labels[&meals_weeknight], "Meals / Weeknight");
        assert_eq!(labels[&favorites_weeknight], "Favorites / Weeknight");
    }

    #[test]
    fn collection_labels_terminate_safely_for_missing_or_cyclic_parents() {
        let orphan = Uuid::new_v4();
        let missing_parent = Uuid::new_v4();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();

        let labels = collection_breadcrumb_labels(&[
            summary(orphan, Some(missing_parent), "Orphan"),
            summary(first, Some(second), "First"),
            summary(second, Some(first), "Second"),
        ]);

        assert_eq!(labels[&orphan], "Orphan");
        assert!(labels[&first].contains("First"));
        assert!(labels[&second].contains("Second"));
    }

    #[test]
    fn collection_dropdown_uses_an_opaque_surface() {
        let source = include_str!("add_to_collection_dropdown.rs");
        let styles = include_str!("../../../assets/main.css");

        assert!(source.contains("collection-membership-dropdown"));
        assert_eq!(COLLECTION_DROPDOWN_SURFACE, "var(--surface)");
        assert!(source.contains("background_color: COLLECTION_DROPDOWN_SURFACE"));
        assert!(styles.contains(".collection-membership-dropdown"));
        assert!(styles.contains("background: var(--surface) !important;"));
    }

    #[test]
    fn collections_are_sorted_by_case_insensitive_full_breadcrumb_path() {
        let kitchen = Uuid::new_v4();
        let meal_planning = Uuid::new_v4();
        let quick_dinners = Uuid::new_v4();
        let thirty_minutes = Uuid::new_v4();
        let vegetarian_picks = Uuid::new_v4();
        let kitchen_weeknight = Uuid::new_v4();
        let alpha = Uuid::new_v4();
        let beta = Uuid::new_v4();
        let collections = vec![
            summary(meal_planning, None, "Meal Planning"),
            summary(quick_dinners, Some(meal_planning), "Quick Dinners"),
            summary(thirty_minutes, Some(quick_dinners), "30 Minutes or Less"),
            summary(vegetarian_picks, Some(meal_planning), "vegetarian picks"),
            summary(kitchen, None, "Kitchen Favorites"),
            summary(kitchen_weeknight, Some(kitchen), "Weeknight Favorites"),
            summary(beta, None, "Beta"),
            summary(alpha, None, "alpha"),
        ];

        let labels = collection_breadcrumb_labels(&collections);
        let ordered = sort_collections_by_breadcrumb_path(collections);
        let ordered_labels = ordered
            .iter()
            .map(|collection| labels[&collection.id].clone())
            .collect::<Vec<_>>();

        assert_eq!(
            ordered_labels,
            vec![
                "alpha",
                "Beta",
                "Kitchen Favorites",
                "Kitchen Favorites / Weeknight Favorites",
                "Meal Planning",
                "Meal Planning / Quick Dinners",
                "Meal Planning / Quick Dinners / 30 Minutes or Less",
                "Meal Planning / vegetarian picks",
            ]
        );
    }
}
