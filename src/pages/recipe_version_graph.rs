//! Dedicated shell for the recipe version graph.
//!
//! This initial page loads only the selected recipe. Graph data and layout are
//! intentionally kept separate from the shell.

use dioxus::prelude::*;

use crate::api::recipe::get_recipe_version_graph;
use crate::components::base::{Card, EditedRecipeImage, EmptyState, LoadingSpinner, PageHeader};
use crate::types::{VersionGraph, VersionGraphEdge, VersionGraphEdgeKind, VersionGraphNode};
use crate::Route;

fn server_error_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message.clone(),
        _ => error.to_string(),
    }
}

fn is_recipe_unavailable_error(message: &str) -> bool {
    matches!(message, "Recipe not found" | "Not authenticated")
}

const GRAPH_NODE_WIDTH_PX: f64 = 240.0;
const GRAPH_NODE_GAP_PX: f64 = 32.0;

fn connector_center_percent(index: usize, node_count: usize) -> f64 {
    let graph_width = node_count as f64 * GRAPH_NODE_WIDTH_PX
        + node_count.saturating_sub(1) as f64 * GRAPH_NODE_GAP_PX;
    (index as f64 * (GRAPH_NODE_WIDTH_PX + GRAPH_NODE_GAP_PX) + GRAPH_NODE_WIDTH_PX / 2.0)
        / graph_width
        * 100.0
}

fn fork_endpoint_label(
    endpoint: crate::types::VersionGraphNodeId,
    nodes: &[VersionGraphNode],
) -> String {
    nodes
        .iter()
        .find(|node| node.id() == endpoint)
        .map(|node| {
            node.title()
                .unwrap_or(if node.is_deleted() {
                    "Deleted recipe"
                } else {
                    "Private recipe"
                })
                .to_string()
        })
        .unwrap_or_else(|| "Private recipe".to_string())
}

fn fork_connection_label(edge: &VersionGraphEdge, nodes: &[VersionGraphNode]) -> String {
    let source = edge.source();
    let target = edge.target();
    format!(
        "{} v{} forked into {} v{}",
        fork_endpoint_label(source, nodes),
        source.version_number,
        fork_endpoint_label(target, nodes),
        target.version_number,
    )
}

#[component]
fn VersionGraphSnapshotCard(
    node: VersionGraphNode,
    is_fork_source: bool,
    is_fork_target: bool,
) -> Element {
    let version_number = node.id().version_number;
    let mut node_class = if node.is_current_version() {
        "recipe-version-graph__node recipe-version-graph__node--current".to_string()
    } else if node.is_deleted() {
        "recipe-version-graph__node recipe-version-graph__node--deleted".to_string()
    } else if node.is_redacted() {
        "recipe-version-graph__node recipe-version-graph__node--redacted".to_string()
    } else {
        "recipe-version-graph__node".to_string()
    };
    if is_fork_source {
        node_class.push_str(" recipe-version-graph__node--fork-source");
    }
    if is_fork_target {
        node_class.push_str(" recipe-version-graph__node--fork-target");
    }
    let title = node
        .title()
        .unwrap_or(if node.is_deleted() {
            "Deleted recipe"
        } else {
            "Private recipe"
        })
        .to_string();
    let image = node.image().cloned();
    let owner_username = node.owner_username().map(str::to_string);
    let change_summary = node.change_summary().map(str::to_string);
    let published_at = node
        .created_at()
        .map(|created_at| created_at.format("%Y-%m-%d %H:%M UTC").to_string());
    let relationship_role = match (is_fork_source, is_fork_target) {
        (true, true) => ", fork source and target",
        (true, false) => ", fork source",
        (false, true) => ", fork target",
        (false, false) => "",
    };

    rsx! {
        article {
            class: node_class,
            aria_label: format!("Version {version_number}: {title}{relationship_role}"),
            div { class: "recipe-version-graph__thumbnail",
                if let Some(image) = image {
                    EditedRecipeImage {
                        image,
                        class: "recipe-version-graph__edited-image",
                        alt: format!("Snapshot image for {title}"),
                        loading: "lazy".to_string(),
                        prefer_thumbnail: true,
                        on_error: move |_| {},
                    }
                } else {
                    div {
                        class: "recipe-version-graph__thumbnail-placeholder",
                        aria_label: "No snapshot image",
                        span { aria_hidden: "true", "🍽" }
                    }
                }
            }
            div { class: "recipe-version-graph__node-body",
                div { class: "recipe-version-graph__node-heading",
                    span { class: "recipe-version-graph__version", "Version {version_number}" }
                    if node.is_current_version() {
                        span { class: "recipe-version-graph__current-badge", "Current" }
                    } else if node.is_deleted() {
                        span { class: "recipe-version-graph__deleted-badge", "Deleted recipe" }
                    }
                }
                h2 { "{title}" }
                if let Some(owner_username) = owner_username {
                    p { class: "recipe-version-graph__owner", "@{owner_username}" }
                }
                if let Some(published_at) = published_at {
                    time { "{published_at}" }
                }
                if let Some(change_summary) = change_summary {
                    p { class: "recipe-version-graph__summary", "{change_summary}" }
                } else if node.is_redacted() {
                    p { class: "recipe-version-graph__summary", "Snapshot details are private." }
                }
            }
        }
    }
}

#[component]
pub fn RecipeVersionGraph(id: String) -> Element {
    let recipe_id_is_valid = uuid::Uuid::parse_str(&id).is_ok();
    let resource_recipe_id_is_valid = recipe_id_is_valid;
    let id_for_graph = id.clone();
    let graph_resource = use_resource(move || {
        let recipe_id = id_for_graph.clone();
        async move {
            if !resource_recipe_id_is_valid {
                return Err(ServerFnError::new("Invalid recipe ID"));
            }
            get_recipe_version_graph(recipe_id).await
        }
    });

    let back_to_recipe = rsx! {
        Link {
            to: Route::RecipeDetail { id: id.clone() },
            class: "btn btn-secondary touch-target",
            "Back to recipe"
        }
    };
    // Subscribe to the completed value before an early loading return. Reading
    // only `pending()` can leave a client-side route in its initial state.
    let graph_result: Option<Result<VersionGraph, ServerFnError>> = graph_resource.read().clone();

    if !recipe_id_is_valid {
        return rsx! {
            div { class: "container",
                Card {
                    EmptyState {
                        icon: rsx! { span { aria_hidden: "true", "↩" } },
                        title: "Invalid recipe link",
                        description: "This version graph link does not contain a valid recipe ID.",
                        action: back_to_recipe,
                    }
                }
            }
        };
    }

    if graph_resource.pending() {
        return rsx! {
            div { class: "container",
                PageHeader { title: "Recipe version graph" }
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "240px",
                        LoadingSpinner {}
                        p {
                            margin_top: "var(--space-md)",
                            color: "var(--text-secondary)",
                            "Loading version graph..."
                        }
                    }
                }
            }
        };
    }

    let Some(graph_result) = graph_result else {
        return rsx! {
            div { class: "container",
                Card {
                    EmptyState {
                        icon: rsx! { span { aria_hidden: "true", "○" } },
                        title: "No versions to display yet",
                        description: "Version graph data is not available for this recipe yet.",
                        action: back_to_recipe,
                    }
                }
            }
        };
    };

    let graph = match graph_result {
        Ok(graph) => graph,
        Err(error) => {
            let error_message = server_error_message(&error);
            let recipe_is_unavailable = is_recipe_unavailable_error(&error_message);
            let state_icon = if recipe_is_unavailable { "🔒" } else { "!" };
            let state_title = if recipe_is_unavailable {
                "Version graph unavailable"
            } else {
                "Unable to load version graph"
            };
            let state_description = if recipe_is_unavailable {
                "This recipe doesn't exist or you don't have access to it."
            } else {
                "We couldn't load the version graph. Please try again."
            };
            return rsx! {
                div { class: "container",
                    PageHeader { title: "Recipe version graph" }
                    Card {
                        EmptyState {
                            icon: rsx! { span { aria_hidden: "true", "{state_icon}" } },
                            title: state_title,
                            description: state_description,
                            action: back_to_recipe,
                        }
                    }
                }
            };
        }
    };

    if graph.nodes.is_empty() {
        return rsx! {
            div { class: "container",
                PageHeader {
                    title: "Recipe version graph",
                    action: Some(back_to_recipe),
                }
                Card {
                    EmptyState {
                        icon: rsx! { span { aria_hidden: "true", "⑂" } },
                        title: "No versions to display yet",
                        description: "Publish the first version to begin this recipe's visual history.",
                    }
                }
            }
        };
    }

    let heading_title = graph
        .nodes
        .iter()
        .find(|node| node.is_current_version())
        .and_then(VersionGraphNode::title)
        .map(|title| format!("Version graph for {title}"))
        .unwrap_or_else(|| "Recipe version graph".to_string());
    let mut lanes = Vec::<Vec<VersionGraphNode>>::new();
    for node in graph.nodes.clone() {
        if let Some(lane) = lanes.iter_mut().find(|lane| {
            lane.first()
                .is_some_and(|first| first.id().recipe_id == node.id().recipe_id)
        }) {
            lane.push(node);
        } else {
            lanes.push(vec![node]);
        }
    }
    let fork_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.kind() == VersionGraphEdgeKind::Fork)
        .cloned()
        .collect::<Vec<_>>();
    let selected_recipe_id = graph
        .nodes
        .iter()
        .find(|node| node.is_selected_recipe())
        .map(|node| node.id().recipe_id);
    lanes.sort_by_key(|lane| {
        let recipe_id = lane
            .first()
            .map(|node| node.id().recipe_id)
            .unwrap_or(uuid::Uuid::nil());
        let lane_order = if Some(recipe_id) == selected_recipe_id {
            1
        } else if fork_edges.iter().any(|edge| {
            Some(edge.target().recipe_id) == selected_recipe_id
                && edge.source().recipe_id == recipe_id
        }) {
            0
        } else if fork_edges.iter().any(|edge| {
            Some(edge.source().recipe_id) == selected_recipe_id
                && edge.target().recipe_id == recipe_id
        }) {
            2
        } else {
            3
        };
        (lane_order, recipe_id)
    });

    rsx! {
        div { class: "container",
            PageHeader {
                title: heading_title,
                action: Some(back_to_recipe),
            }
            Card {
                div { class: "recipe-version-graph",
                    p { class: "recipe-version-graph__description",
                        "Explore immutable snapshots and immediate recipe forks."
                    }
                    div { class: "recipe-version-graph__lanes",
                        for lane in lanes {
                            {
                                let lane_node_count = lane.len();
                                let lane_recipe_id = lane
                                    .first()
                                    .map(|node| node.id().recipe_id)
                                    .unwrap_or(uuid::Uuid::nil());
                                let incoming_fork = fork_edges
                                    .iter()
                                    .find(|edge| edge.target().recipe_id == lane_recipe_id)
                                    .cloned();
                                let lane_title = lane
                                    .iter()
                                    .find_map(VersionGraphNode::title)
                                    .unwrap_or(if lane.iter().any(VersionGraphNode::is_deleted) {
                                        "Deleted recipe"
                                    } else {
                                        "Private recipe"
                                    })
                                    .to_string();
                                let lane_connectors = lane
                                    .windows(2)
                                    .map(|pair| {
                                        (
                                            connector_center_percent(
                                                pair[0].id().version_number.saturating_sub(1) as usize,
                                                lane_node_count,
                                            ),
                                            connector_center_percent(
                                                pair[1].id().version_number.saturating_sub(1) as usize,
                                                lane_node_count,
                                            ),
                                        )
                                    })
                                    .collect::<Vec<_>>();
                                rsx! {
                                    div { class: "recipe-version-graph__lane-group",
                                        if let Some(edge) = incoming_fork {
                                            div {
                                                class: "recipe-version-graph__fork-connection",
                                                role: "note",
                                                aria_label: fork_connection_label(&edge, &graph.nodes),
                                                span {
                                                    class: "recipe-version-graph__fork-connection-line",
                                                    aria_hidden: "true",
                                                }
                                                strong { "{fork_connection_label(&edge, &graph.nodes)}" }
                                            }
                                        }
                                        section {
                                            class: "recipe-version-graph__lane",
                                            aria_label: format!("Recipe lane: {lane_title}"),
                                            h2 { class: "recipe-version-graph__lane-title", "{lane_title}" }
                                            div {
                                                class: "recipe-version-graph__viewport",
                                                style: "--version-graph-node-count: {lane_node_count}",
                                                svg {
                                                    class: "recipe-version-graph__edges recipe-version-graph__edge--version",
                                                    "aria-hidden": "true",
                                                    view_box: "0 0 100 10",
                                                    preserve_aspect_ratio: "none",
                                                    for (index, (source_x, target_x)) in lane_connectors.iter().enumerate() {
                                                        line {
                                                            key: "{index}",
                                                            x1: "{source_x}",
                                                            y1: "5",
                                                            x2: "{target_x}",
                                                            y2: "5",
                                                        }
                                                    }
                                                }
                                                ol {
                                                    class: "recipe-version-graph__nodes",
                                                    for node in lane {
                                                        {
                                                            let is_fork_source = fork_edges
                                                                .iter()
                                                                .any(|edge| edge.source() == node.id());
                                                            let is_fork_target = fork_edges
                                                                .iter()
                                                                .any(|edge| edge.target() == node.id());
                                                            rsx! {
                                                                li {
                                                                    key: "{node.id().recipe_id}-{node.id().version_number}",
                                                                    if node.is_redacted() || node.is_deleted() {
                                                                        VersionGraphSnapshotCard {
                                                                            node,
                                                                            is_fork_source,
                                                                            is_fork_target,
                                                                        }
                                                                    } else {
                                                                        Link {
                                                                            to: Route::RecipeVersionDetail {
                                                                                id: node.id().recipe_id.to_string(),
                                                                                version_number: node.id().version_number,
                                                                            },
                                                                            class: "recipe-version-graph__node-link",
                                                                            VersionGraphSnapshotCard {
                                                                                node,
                                                                                is_fork_source,
                                                                                is_fork_target,
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
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_recipe_unavailable_error;
    use crate::types::{
        VersionGraphEdge, VersionGraphNode, VersionGraphNodeContent, VersionGraphNodeId,
        VersionGraphNodeState,
    };
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn visible_node(recipe_id: Uuid, version_number: i32, title: &str) -> VersionGraphNode {
        VersionGraphNode::visible(
            VersionGraphNodeId {
                recipe_id,
                version_number,
            },
            VersionGraphNodeContent {
                title: title.to_string(),
                owner_username: Some("cook".to_string()),
                image: None,
                created_at: Utc.with_ymd_and_hms(2026, 7, 24, 12, 0, 0).unwrap(),
                change_summary: "Initial version".to_string(),
            },
            VersionGraphNodeState {
                is_selected_recipe: false,
                is_current_version: true,
                is_deleted: false,
            },
        )
    }

    #[test]
    fn graph_shell_renders_navigation_and_all_resource_states() {
        let source = include_str!("recipe_version_graph.rs");

        assert!(source.contains("Recipe version graph"));
        assert!(source.contains("Back to recipe"));
        assert!(source.contains("Loading version graph"));
        assert!(source.contains("No versions to display yet"));
        assert!(source.contains("doesn't exist or you don't have access"));
        assert!(source.contains("We couldn't load the version graph"));
        assert!(source.contains("Invalid recipe link"));
        assert!(source.contains("Route::RecipeDetail"));
    }

    #[test]
    fn only_privacy_safe_lookup_failures_use_the_unavailable_state() {
        assert!(is_recipe_unavailable_error("Recipe not found"));
        assert!(is_recipe_unavailable_error("Not authenticated"));
        assert!(!is_recipe_unavailable_error("database connection failed"));
    }

    #[test]
    fn graph_shell_subscribes_to_the_recipe_result_before_returning_loading_ui() {
        let source = include_str!("recipe_version_graph.rs");
        let result_read = source.find("graph_resource.read().clone()").unwrap();
        let loading_return = source.find("if graph_resource.pending()").unwrap();

        assert!(result_read < loading_return);
    }

    #[test]
    fn graph_page_renders_accessible_snapshot_cards_and_separate_connectors() {
        let source = include_str!("recipe_version_graph.rs");

        assert!(source.contains("get_recipe_version_graph"));
        assert!(source.contains("recipe-version-graph__edges"));
        assert!(source.contains("aria_hidden: \"true\""));
        assert!(source.contains("recipe-version-graph__node--current"));
        assert!(source.contains("Route::RecipeVersionDetail"));
        assert!(source.contains("Version {version_number}"));
        assert!(source.contains("No snapshot image"));
        assert!(source.contains("EditedRecipeImage"));
        assert!(source.contains("prefer_thumbnail: true"));
    }

    #[test]
    fn graph_page_visually_distinguishes_fork_connections() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("VersionGraphEdgeKind::Fork"));
        assert!(page.contains("recipe-version-graph__fork-connection"));
        assert!(page.contains("recipe-version-graph__node--fork-source"));
        assert!(page.contains("recipe-version-graph__node--fork-target"));
        assert!(page.contains("fork_connection_label"));
        assert!(page.contains("forked into"));
        assert!(page.contains("Deleted recipe"));
        assert!(!page.contains("class: \"recipe-version-graph__fork-edges\""));
    }

    #[test]
    fn fork_connection_labels_identify_both_endpoint_recipes_and_versions() {
        let source_recipe_id = Uuid::new_v4();
        let forked_recipe_id = Uuid::new_v4();
        let edge = VersionGraphEdge::fork(source_recipe_id, 2, forked_recipe_id).unwrap();
        let nodes = vec![
            visible_node(source_recipe_id, 2, "Original soup"),
            visible_node(forked_recipe_id, 1, "Spicy soup"),
        ];

        assert_eq!(
            super::fork_connection_label(&edge, &nodes),
            "Original soup v2 forked into Spicy soup v1"
        );
    }

    #[test]
    fn connector_centers_follow_the_same_card_and_gap_geometry_as_the_grid() {
        let first = super::connector_center_percent(0, 3);
        let second = super::connector_center_percent(1, 3);
        let third = super::connector_center_percent(2, 3);

        assert!((first - (120.0 / 784.0 * 100.0)).abs() < f64::EPSILON);
        assert!((second - 50.0).abs() < f64::EPSILON);
        assert!((third - (664.0 / 784.0 * 100.0)).abs() < f64::EPSILON);
    }
}
