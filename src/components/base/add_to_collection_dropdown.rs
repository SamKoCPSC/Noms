use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::collection::{
    add_recipe_to_collection, get_recipe_collections, load_all_my_collections,
};
use crate::types::CollectionSummary;

use super::{
    collection_picker_popover::CollectionPickerPopover, collection_picker_state::PickerSelection,
    FeedbackToast,
};

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
    pub recipe_title: String,
}

/// Staged, additive picker. Collection pages retain membership removal.
#[component]
pub fn AddToCollectionDropdown(props: AddToCollectionDropdownProps) -> Element {
    let mut open = use_signal(|| false);
    let pending = use_signal(|| false);
    let mut feedback = use_signal(|| None::<Result<String, String>>);
    let mut generation = use_signal(|| 0u64);
    let trigger_id = use_hook(|| format!("collection-picker-{}", Uuid::new_v4()));
    let close_trigger = trigger_id.clone();
    let close = EventHandler::new(move |restore: bool| {
        if pending() {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        if restore {
            use super::overlay_dom as dom;
            let focus_inside =
                dom::element(&format!("{close_trigger}-panel")).is_some_and(|panel| {
                    dom::document()
                        .active_element()
                        .is_some_and(|active| panel.contains(Some(active.as_ref())))
                });
            // A request may finish after the user tabs elsewhere; do not steal that focus.
            if focus_inside {
                if let Some(trigger) = dom::element(&close_trigger) {
                    dom::focus(&trigger, true);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&close_trigger, restore);
        open.set(false);
    });
    rsx! {
        div {
            button { id: trigger_id.clone(), r#type: "button", class: "btn btn-secondary touch-target",
                aria_haspopup: "dialog", aria_expanded: open(), aria_controls: "{trigger_id}-panel",
                onclick: move |_| { if !pending() { open.set(!open()); } },
                "Add to Collection"
            }
            if open() {
                CollectionPickerPopover { trigger_id: trigger_id.clone(), pending, on_dismiss: close,
                    PickerContent { recipe_id: props.recipe_id, recipe_title: props.recipe_title,
                        pending, on_close: close,
                        on_result: move |result| { feedback.set(Some(result)); generation += 1; }
                    }
                }
            }
        }
        FeedbackToast { message: feedback(), generation: generation(), on_dismiss: move |_| feedback.set(None) }
    }
}

#[component]
fn PickerContent(
    recipe_id: String,
    recipe_title: String,
    pending: Signal<bool>,
    on_close: EventHandler<bool>,
    on_result: EventHandler<Result<String, String>>,
) -> Element {
    let resource_recipe = recipe_id.clone();
    let mut membership = use_resource(move || {
        let recipe_id = resource_recipe.clone();
        async move {
            let collections = load_all_my_collections().await?;
            let memberships = get_recipe_collections(recipe_id).await?;
            Ok::<_, ServerFnError>((collections, memberships))
        }
    });
    let loaded = membership.read().clone();
    rsx! {
        header { class: "collection-picker__header",
            h2 { "Add to collection" }
            p { "{recipe_title}" }
            p { class: "collection-picker__help", "Choose collections to add this recipe to. Existing memberships stay unchanged." }
        }
        match loaded {
            None => rsx! { p { role: "status", "Loading collections…" } button { r#type: "button", class: "btn btn-secondary", onclick: move |_| on_close.call(true), "Cancel" } },
            Some(Err(error)) => rsx! {
                p { role: "alert", "Could not load collections: {error}" }
                button { r#type: "button", class: "btn btn-secondary", onclick: move |_| membership.restart(), "Retry loading" }
                button { r#type: "button", class: "btn btn-secondary", onclick: move |_| on_close.call(true), "Cancel" }
            },
            Some(Ok((collections, memberships))) => rsx! {
                PickerSelectionForm { recipe_id, collections, memberships, pending, on_close, on_result }
            },
        }
    }
}

/// Search shows full paths; browsing hides descendants of collapsed parents.
fn visible_collection_rows(
    collections: &[CollectionSummary],
    expanded: &HashSet<Uuid>,
    query: &str,
) -> Vec<(CollectionSummary, String, usize, bool)> {
    let labels = collection_breadcrumb_labels(collections);
    let by_id: HashMap<_, _> = collections.iter().map(|c| (c.id, c)).collect();
    let query = query.trim().to_lowercase();
    sort_collections_by_breadcrumb_path(collections.to_vec())
        .into_iter()
        .filter_map(|collection| {
            let label = labels[&collection.id].clone();
            let has_children = collections.iter().any(|child| {
                child.parent_collection_id == Some(collection.id) && child.id != collection.id
            });
            let mut depth = 0usize;
            let mut visible = true;
            let mut visited = HashSet::from([collection.id]);
            let mut parent = collection.parent_collection_id;
            while let Some(ancestor) = parent.and_then(|id| by_id.get(&id)) {
                if !visited.insert(ancestor.id) {
                    visible = true;
                    depth = 0;
                    break;
                }
                depth += 1;
                visible &= expanded.contains(&ancestor.id);
                parent = ancestor.parent_collection_id;
            }
            if (!query.is_empty() && label.to_lowercase().contains(&query))
                || (query.is_empty() && visible)
            {
                Some((
                    collection,
                    label,
                    if query.is_empty() { depth.min(6) } else { 0 },
                    has_children,
                ))
            } else {
                None
            }
        })
        .collect()
}

#[component]
fn PickerSelectionForm(
    recipe_id: String,
    collections: Vec<CollectionSummary>,
    memberships: Vec<Uuid>,
    mut pending: Signal<bool>,
    on_close: EventHandler<bool>,
    on_result: EventHandler<Result<String, String>>,
) -> Element {
    let mut selection = use_signal(|| PickerSelection::new(memberships.into_iter().collect()));
    let mut query = use_signal(String::new);
    let mut expanded = use_signal(HashSet::<Uuid>::new);
    let rows = visible_collection_rows(&collections, &expanded(), &query());
    let selected_count = selection.read().selected.len();
    let error_count = selection.read().errors.len();
    let submit = move |_| {
        let Some(ids) = selection.write().begin() else {
            return;
        };
        pending.set(true);
        let recipe_id = recipe_id.clone();
        spawn(async move {
            let total = ids.len();
            let mut confirmed = HashSet::new();
            let mut errors = Vec::new();
            // Bounded sequential requests preserve server budgets; never resend known successes.
            for id in ids {
                match add_recipe_to_collection(id.to_string(), recipe_id.clone()).await {
                    Ok(()) => {
                        confirmed.insert(id);
                    }
                    Err(error) => errors.push((id, error.to_string())),
                }
            }
            // A failed response may follow a committed write (or another tab's add).
            // Reconcile through the owner-scoped read instead of parsing error strings.
            if !errors.is_empty() {
                if let Ok(current) = get_recipe_collections(recipe_id).await {
                    confirmed.extend(current);
                }
            }
            selection.write().finish(&confirmed, errors);
            pending.set(false);
            let failed = selection.read().errors.len();
            if failed == 0 {
                on_result.call(Ok(format!("Added to {total} collection(s).")));
                on_close.call(true);
            } else {
                on_result.call(Err(format!("{} of {total} added. {failed} could not be confirmed; review the selected collections and retry.", total - failed)));
            }
        });
    };
    rsx! {
        label { class: "collection-picker__search",
            span { "Find a collection" }
            input { r#type: "search", class: "input input--shared", placeholder: "Search collection paths", value: query(), disabled: pending(),
                onmounted: move |event| { spawn(async move { let _ = event.data().set_focus(true).await; }); },
                oninput: move |event| { if !pending() { query.set(event.value()); } }
            }
        }
        if collections.is_empty() {
            p { "No collections yet. Create one from Collections, then return here." }
        } else if rows.is_empty() {
            p { role: "status", "No matching collections. Try a different search." }
        }
        fieldset { class: "collection-picker__list", disabled: pending(),
            legend { "Collections" }
            for (collection, label, depth, has_children) in rows {
                {
                    let id = collection.id;
                    let confirmed = selection.read().confirmed.contains(&id);
                    let checked = confirmed || selection.read().selected.contains(&id);
                    let is_expanded = expanded.read().contains(&id);
                    let error = selection.read().errors.get(&id).cloned();
                    rsx! {
                        div { key: "{id}", class: "collection-picker__row", style: "margin-left: {depth * 12}px;",
                            if has_children && query().trim().is_empty() {
                                button { r#type: "button", class: "collection-picker__expand",
                                    aria_label: if is_expanded { format!("Collapse {label}") } else { format!("Expand {label}") },
                                    aria_expanded: is_expanded, disabled: pending(),
                                    onclick: move |_| { if !pending() { let mut set = expanded.write(); if !set.remove(&id) { set.insert(id); } } },
                                    if is_expanded { "▾" } else { "▸" }
                                }
                            } else { span { class: "collection-picker__spacer", aria_hidden: "true" } }
                            label { class: if checked { "collection-picker__choice collection-picker__choice--checked" } else { "collection-picker__choice" },
                                input { r#type: "checkbox", checked, disabled: pending() || confirmed, aria_label: label.clone(),
                                    onchange: move |_| selection.write().toggle(id)
                                }
                                span {
                                    span { if query().trim().is_empty() { "{collection.name}" } else { "{label}" } }
                                    if confirmed { small { "Already added" } }
                                    if let Some(error) = error { small { class: "collection-picker__error", "{error}" } }
                                }
                            }
                        }
                    }
                }
            }
        }
        if error_count > 0 { p { role: "alert", "{error_count} collection(s) could not be confirmed. Successful additions are retained; retry adds only the remaining selections." } }
        footer { class: "collection-picker__footer",
            span { role: "status", if pending() { "Adding…" } else { "{selected_count} selected" } }
            button { r#type: "button", class: "btn btn-secondary", disabled: pending(), onclick: move |_| on_close.call(true), "Cancel" }
            button { r#type: "button", class: "btn btn-primary", disabled: pending() || selected_count == 0, onclick: submit, "Add to {selected_count}" }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn collection_browsing_and_search_keep_expansion_separate_from_selection() {
        let parent = Uuid::from_u128(1);
        let child = Uuid::from_u128(2);
        let rows = vec![
            summary(parent, None, "Meals"),
            summary(child, Some(parent), "Soup"),
        ];
        let collapsed = super::visible_collection_rows(&rows, &Default::default(), "");
        assert_eq!(collapsed.len(), 1);
        assert!(collapsed[0].3);
        let expanded = super::visible_collection_rows(&rows, &[parent].into(), "");
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[1].2, 1);
        let search = super::visible_collection_rows(&rows, &Default::default(), "  SOUP ");
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].1, "Meals / Soup");
        assert_eq!(search[0].2, 0);
        assert!(super::visible_collection_rows(&rows, &Default::default(), "no match").is_empty());
        let grandchild = Uuid::from_u128(3);
        let mut nested = rows;
        nested.push(summary(grandchild, Some(child), "Tomato"));
        assert_eq!(
            super::visible_collection_rows(&nested, &[parent].into(), "").len(),
            2
        );
        assert_eq!(
            super::visible_collection_rows(&nested, &[parent, child].into(), "").len(),
            3
        );
        let search = super::visible_collection_rows(&nested, &Default::default(), "MEALS / SOUP");
        assert_eq!(search.len(), 2);
        assert!(search.iter().all(|row| row.2 == 0));
    }

    #[test]
    fn orphaned_and_cyclic_collections_remain_reachable() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let orphan = Uuid::from_u128(3);
        let rows = vec![
            summary(first, Some(second), "A"),
            summary(second, Some(first), "B"),
            summary(orphan, Some(Uuid::from_u128(4)), "C"),
        ];
        assert_eq!(
            super::visible_collection_rows(&rows, &Default::default(), "").len(),
            3
        );
    }
    use chrono::Utc;
    use uuid::Uuid;

    use super::{collection_breadcrumb_labels, sort_collections_by_breadcrumb_path};
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
    fn collection_picker_uses_glass_with_opaque_fallback_and_no_removal() {
        let source = include_str!("add_to_collection_dropdown.rs");
        let styles = include_str!("../../../assets/main.css");
        assert!(source.contains("CollectionPickerPopover"));
        assert!(!source
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .contains("remove_recipe_from_collection"));
        assert!(styles.contains(".collection-picker"));
        assert!(styles.contains("background: var(--overlay-opaque)"));
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
