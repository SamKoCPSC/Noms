use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::collection::load_all_my_collections;
use crate::components::base::LoadRecovery;
use crate::types::CollectionSummary;
use crate::Route;

const MAX_VISUAL_COLLECTION_DEPTH: usize = 6;
const COLLECTION_INDENTATION_STEP_PX: usize = 12;
const NEW_SUBCOLLECTION_ACTION_OFFSET_PX: usize = 36;

#[derive(Clone, Debug, PartialEq)]
struct CollectionTreeNode {
    collection: CollectionSummary,
    children: Vec<CollectionTreeNode>,
}

fn collection_indentation(depth: usize) -> usize {
    depth.min(MAX_VISUAL_COLLECTION_DEPTH) * COLLECTION_INDENTATION_STEP_PX
}

fn new_subcollection_indentation(parent_depth: usize) -> usize {
    collection_indentation(parent_depth.saturating_add(1)) + NEW_SUBCOLLECTION_ACTION_OFFSET_PX
}

fn compare_collections(left: &CollectionSummary, right: &CollectionSummary) -> Ordering {
    left.sort_order
        .cmp(&right.sort_order)
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.id.cmp(&right.id))
}

fn collection_toggle_label(name: &str, expanded: bool) -> String {
    if expanded {
        format!("Collapse {name}")
    } else {
        format!("Expand {name}")
    }
}

fn build_children(
    parent_id: Option<Uuid>,
    collections_by_parent: &mut HashMap<Option<Uuid>, Vec<CollectionSummary>>,
    visited: &mut HashSet<Uuid>,
) -> Vec<CollectionTreeNode> {
    let mut siblings = collections_by_parent.remove(&parent_id).unwrap_or_default();
    siblings.sort_by(compare_collections);
    siblings
        .into_iter()
        .filter_map(|collection| {
            if !visited.insert(collection.id) {
                return None;
            }
            let children = build_children(Some(collection.id), collections_by_parent, visited);
            Some(CollectionTreeNode {
                collection,
                children,
            })
        })
        .collect()
}

fn build_collection_tree(collections: Vec<CollectionSummary>) -> Vec<CollectionTreeNode> {
    let mut collections_by_parent = HashMap::<Option<Uuid>, Vec<CollectionSummary>>::new();
    for collection in collections {
        collections_by_parent
            .entry(collection.parent_collection_id)
            .or_default()
            .push(collection);
    }

    build_children(None, &mut collections_by_parent, &mut HashSet::new())
}

#[derive(Props, Clone, PartialEq)]
struct CollectionTreeItemProps {
    node: CollectionTreeNode,
    depth: usize,
    expanded: Signal<HashSet<Uuid>>,
}

#[component]
fn CollectionTreeItem(mut props: CollectionTreeItemProps) -> Element {
    let collection_id = props.node.collection.id;
    let is_expanded = props.expanded.read().contains(&collection_id);
    let toggle_label = collection_toggle_label(&props.node.collection.name, is_expanded);
    let indentation = collection_indentation(props.depth);
    let new_subcollection_indentation = new_subcollection_indentation(props.depth);

    rsx! {
        li {
            list_style: "none",
            div {
                display: "flex",
                align_items: "center",
                gap: "var(--space-xs)",
                min_width: "0",
                padding_left: "{indentation}px",
                button {
                    r#type: "button",
                    class: "btn touch-target",
                    aria_label: "{toggle_label}",
                    aria_expanded: is_expanded,
                    onclick: move |_| {
                        let mut expanded = props.expanded.write();
                        if !expanded.remove(&collection_id) {
                            expanded.insert(collection_id);
                        }
                    },
                    if is_expanded { "▾" } else { "▸" }
                }
                Link {
                    to: Route::CollectionDetail { id: collection_id.to_string() },
                    display: "flex",
                    align_items: "center",
                    gap: "var(--space-xs)",
                    min_width: "0",
                    flex: "1",
                    padding: "var(--space-xs) 0",
                    color: "var(--text-primary)",
                    text_decoration: "none",
                    span {
                        overflow: "hidden",
                        text_overflow: "ellipsis",
                        white_space: "nowrap",
                        "{props.node.collection.name}"
                    }
                    span {
                        flex_shrink: "0",
                        color: "var(--text-secondary)",
                        font_size: "var(--font-size-sm)",
                        aria_label: "{props.node.collection.recipe_count} recipes",
                        "{props.node.collection.recipe_count}"
                    }
                }
            }
            if is_expanded {
                ul {
                    margin: "0",
                    padding: "0",
                    for child in props.node.children.iter().cloned() {
                        CollectionTreeItem {
                            key: "{child.collection.id}",
                            node: child,
                            depth: props.depth.saturating_add(1),
                            expanded: props.expanded,
                        }
                    }
                    li {
                        list_style: "none",
                        padding_left: "{new_subcollection_indentation}px",
                        a {
                            href: "/collections/new?parent={collection_id}",
                            display: "inline-block",
                            padding: "var(--space-xs) 0",
                            color: "var(--text-secondary)",
                            font_size: "var(--font-size-sm)",
                            text_decoration: "none",
                            "+ New subcollection"
                        }
                    }
                }
            }
        }
    }
}

/// Owner-scoped collection navigation for the recipe dashboard.
#[component]
pub fn CollectionSidebar(#[props(default)] embedded: bool) -> Element {
    let mut collections = use_resource(load_all_my_collections);
    let expanded = use_signal(HashSet::<Uuid>::new);
    let pending = *collections.state().read() == UseResourceState::Pending;
    let failed = collections
        .read()
        .as_ref()
        .is_some_and(|result| result.is_err());
    let tree = collections
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|items| build_collection_tree(items.clone()))
        .unwrap_or_default();

    if tree.is_empty() {
        return rsx! {
            aside {
                class: if embedded { "dashboard-collection-tree" } else { "neumo-card" },
                padding: if embedded { "0" } else { "var(--space-md)" },
                min_width: "0",
                aria_label: "Recipe collections",
                // Retain the same recovery node while retrying so focus isn't
                // lost to a different loading branch. Never hide load failures.
                if failed {
                    LoadRecovery {
                        pending,
                        message: "Collections couldn’t be loaded. Please try again.",
                        retry_label: "Retry collections",
                        onretry: move |_| { if !collections.pending() { collections.restart(); } },
                    }
                } else if pending {
                    p { class: "dashboard-muted", role: "status", "Loading collections…" }
                } else {
                    p { class: "dashboard-muted", "No collections yet." }
                }
                if !embedded {
                    Link { to: Route::CollectionNew { parent: None }, class: "btn btn-secondary touch-target", "New Collection" }
                }
            }
        };
    }

    rsx! {
        aside {
            class: if embedded { "dashboard-collection-tree" } else { "neumo-card" },
            padding: if embedded { "0" } else { "var(--space-md)" },
            flex: "1 1 220px",
            min_width: "0",
            max_width: if embedded { "100%" } else { "280px" },
            align_self: "start",
            aria_label: "Recipe collections",
            nav {
                display: "flex",
                flex_direction: "column",
                gap: "var(--space-sm)",
                if !embedded { Link {
                    to: Route::Dashboard {},
                    color: "var(--text-primary)",
                    font_weight: "600",
                    text_decoration: "none",
                    "All Recipes"
                } }
                if !embedded { div {
                    display: "flex",
                    align_items: "center",
                    gap: "var(--space-sm)",
                    h2 { margin: "0", font_size: "var(--font-size-md)", "Collections" }
                } }
                ul {
                    margin: "0",
                    padding: "0",
                    overflow_x: "hidden",
                    for node in tree {
                        CollectionTreeItem {
                            key: "{node.collection.id}",
                            node,
                            depth: 0,
                            expanded,
                        }
                    }
                }
                if !embedded { Link {
                    to: Route::CollectionNew { parent: None },
                    class: "btn btn-secondary touch-target",
                    "New Collection"
                } }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn collection_sidebar_failure_remains_visible_and_retry_is_guarded() {
        let source = include_str!("collection_sidebar.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(source.contains("LoadRecovery"));
        assert!(source.contains("collections.state().read()"));
        assert!(source.contains("if !collections.pending()"));
        assert!(!source.contains("return rsx! {};"));
    }

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::{
        build_collection_tree, collection_indentation, collection_toggle_label,
        new_subcollection_indentation,
    };
    use crate::types::CollectionSummary;

    fn summary(
        id: Uuid,
        parent_collection_id: Option<Uuid>,
        name: &str,
        sort_order: i32,
        recipe_count: i64,
        age_minutes: i64,
    ) -> CollectionSummary {
        CollectionSummary {
            id,
            parent_collection_id,
            name: name.to_string(),
            description: None,
            image: None,
            preview_images: Vec::new(),
            visibility: crate::types::CollectionVisibility::Private,
            sort_order,
            child_count: 0,
            recipe_count,
            updated_at: Utc::now() - Duration::minutes(age_minutes),
        }
    }

    #[test]
    fn collection_tree_nests_children_and_preserves_counts() {
        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let tree = build_collection_tree(vec![
            summary(child_id, Some(root_id), "Child", 0, 4, 0),
            summary(root_id, None, "Root", 0, 2, 0),
        ]);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].collection.id, root_id);
        assert_eq!(tree[0].collection.recipe_count, 2);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].collection.id, child_id);
        assert_eq!(tree[0].children[0].collection.recipe_count, 4);
    }

    #[test]
    fn collection_tree_orders_siblings_by_order_then_recent_update() {
        let first_id = Uuid::new_v4();
        let newer_id = Uuid::new_v4();
        let older_id = Uuid::new_v4();
        let tree = build_collection_tree(vec![
            summary(older_id, None, "Older", 1, 0, 20),
            summary(first_id, None, "First", 0, 0, 50),
            summary(newer_id, None, "Newer", 1, 0, 5),
        ]);

        assert_eq!(
            tree.iter()
                .map(|node| node.collection.id)
                .collect::<Vec<_>>(),
            vec![first_id, newer_id, older_id]
        );
    }

    #[test]
    fn disclosure_labels_identify_the_collection_and_state() {
        assert_eq!(
            collection_toggle_label("Weeknight meals", false),
            "Expand Weeknight meals"
        );
        assert_eq!(
            collection_toggle_label("Weeknight meals", true),
            "Collapse Weeknight meals"
        );
    }

    #[test]
    fn new_collection_action_follows_the_tree() {
        let source = include_str!("collection_sidebar.rs");
        let tree_position = source.find("for node in tree").unwrap();
        let action_position = source.rfind("\"New Collection\"").unwrap();
        assert!(action_position > tree_position);
        assert!(source.contains("class: \"btn btn-secondary touch-target\""));
    }

    #[test]
    fn very_deep_collection_trees_keep_indentation_bounded() {
        assert_eq!(collection_indentation(0), 0);
        assert_eq!(collection_indentation(6), 72);
        assert_eq!(collection_indentation(100_000), 72);
        assert_eq!(collection_indentation(usize::MAX), 72);
        assert_eq!(new_subcollection_indentation(usize::MAX), 108);
    }
}
