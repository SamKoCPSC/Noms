//! Dedicated shell for the recipe version graph.
//!
//! This initial page loads only the selected recipe. Graph data and layout are
//! intentionally kept separate from the shell.

use dioxus::prelude::*;

use crate::api::recipe::get_recipe_version_graph;
use crate::components::base::{Card, EmptyState, LoadingSpinner, PageHeader};
use crate::types::{VersionGraph, VersionGraphEdgeKind, VersionGraphNode};
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

#[component]
fn VersionGraphSnapshotCard(node: VersionGraphNode) -> Element {
    let version_number = node.id().version_number;
    let node_class = if node.is_current_version() {
        "recipe-version-graph__node recipe-version-graph__node--current"
    } else if node.is_redacted() {
        "recipe-version-graph__node recipe-version-graph__node--redacted"
    } else {
        "recipe-version-graph__node"
    };
    let title = node.title().unwrap_or("Private snapshot").to_string();
    let thumbnail_url = node.thumbnail_url().map(str::to_string);
    let owner_username = node.owner_username().map(str::to_string);
    let change_summary = node.change_summary().map(str::to_string);
    let published_at = node
        .created_at()
        .map(|created_at| created_at.format("%Y-%m-%d %H:%M UTC").to_string());

    rsx! {
        article {
            class: node_class,
            aria_label: format!("Version {version_number}: {title}"),
            div { class: "recipe-version-graph__thumbnail",
                if let Some(thumbnail_url) = thumbnail_url {
                    img {
                        src: thumbnail_url,
                        alt: format!("Snapshot image for {title}"),
                        loading: "lazy",
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
    let connector_lines = graph
        .edges
        .iter()
        .filter(|edge| edge.kind() == VersionGraphEdgeKind::Version)
        .filter_map(|edge| {
            let source_index = graph
                .nodes
                .iter()
                .position(|node| node.id() == edge.source())?;
            let target_index = graph
                .nodes
                .iter()
                .position(|node| node.id() == edge.target())?;
            Some((
                connector_center_percent(source_index, graph.nodes.len()),
                connector_center_percent(target_index, graph.nodes.len()),
            ))
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "container",
            PageHeader {
                title: heading_title,
                action: Some(back_to_recipe),
            }
            Card {
                div { class: "recipe-version-graph",
                    p { class: "recipe-version-graph__description",
                        "Explore each immutable snapshot in this recipe's history."
                    }
                    div {
                        class: "recipe-version-graph__viewport",
                        style: "--version-graph-node-count: {graph.nodes.len()}",
                        svg {
                            class: "recipe-version-graph__edges",
                            "aria-hidden": "true",
                            view_box: "0 0 100 10",
                            preserve_aspect_ratio: "none",
                            for (index, (source_x, target_x)) in connector_lines.iter().enumerate() {
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
                            for node in graph.nodes.clone() {
                                li {
                                    key: "{node.id().recipe_id}-{node.id().version_number}",
                                    if node.is_redacted() {
                                        VersionGraphSnapshotCard { node }
                                    } else {
                                        Link {
                                            to: Route::RecipeVersionDetail {
                                                id: node.id().recipe_id.to_string(),
                                                version_number: node.id().version_number,
                                            },
                                            class: "recipe-version-graph__node-link",
                                            VersionGraphSnapshotCard { node }
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
