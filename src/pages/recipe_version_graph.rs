//! Dedicated shell for the recipe version graph.
//!
//! The page renders a bounded connected lineage family. Graph discovery and
//! presentation layout remain separate so each can evolve independently.

use dioxus::html::{geometry::WheelDelta, input_data::MouseButton};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::api::recipe::get_recipe_version_graph;
use crate::components::base::{Card, EditedRecipeImage, EmptyState, LoadingSpinner, PageHeader};
use crate::types::{
    VersionGraph, VersionGraphEdge, VersionGraphEdgeKind, VersionGraphNode, VersionGraphNodeId,
};
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
const GRAPH_NODE_HEIGHT_PX: f64 = 300.0;
const GRAPH_COLUMN_GAP_PX: f64 = 96.0;
const GRAPH_ROW_GAP_PX: f64 = 56.0;
const GRAPH_LAYOUT_PADDING_PX: f64 = 32.0;
const GRAPH_MIN_ZOOM: f64 = 0.001;
const GRAPH_MAX_ZOOM: f64 = 1.8;
const GRAPH_ZOOM_FACTOR: f64 = 1.2;
const GRAPH_WHEEL_ZOOM_SENSITIVITY: f64 = 0.002;
const GRAPH_WHEEL_LINE_HEIGHT_PX: f64 = 40.0;
const GRAPH_PAN_STEP_PX: f64 = 96.0;
const GRAPH_DEFAULT_VIEWPORT: (f64, f64) = (900.0, 560.0);
const GRAPH_CONTEXT_MENU_WIDTH_PX: f64 = 224.0;
const GRAPH_CONTEXT_MENU_HEIGHT_PX: f64 = 250.0;
const GRAPH_SNAPSHOT_CONTEXT_MENU_HEIGHT_PX: f64 = 336.0;
const GRAPH_CONTEXT_MENU_MARGIN_PX: f64 = 8.0;
const GRAPH_SNAPSHOT_FOCUS_ZOOM: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct VersionGraphTransform {
    scale: f64,
    offset_x: f64,
    offset_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum VersionGraphContextMenuTarget {
    Background,
    Snapshot(VersionGraphNodeId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct VersionGraphContextMenu {
    anchor: (f64, f64),
    position: (f64, f64),
    target: VersionGraphContextMenuTarget,
}

impl VersionGraphContextMenu {
    fn at(anchor: (f64, f64), viewport: (f64, f64), target: VersionGraphContextMenuTarget) -> Self {
        let menu_height = match target {
            VersionGraphContextMenuTarget::Background => GRAPH_CONTEXT_MENU_HEIGHT_PX,
            VersionGraphContextMenuTarget::Snapshot(_) => GRAPH_SNAPSHOT_CONTEXT_MENU_HEIGHT_PX,
        };
        Self {
            anchor,
            position: (
                anchor.0.clamp(
                    GRAPH_CONTEXT_MENU_MARGIN_PX,
                    (viewport.0 - GRAPH_CONTEXT_MENU_WIDTH_PX - GRAPH_CONTEXT_MENU_MARGIN_PX)
                        .max(GRAPH_CONTEXT_MENU_MARGIN_PX),
                ),
                anchor.1.clamp(
                    GRAPH_CONTEXT_MENU_MARGIN_PX,
                    (viewport.1 - menu_height - GRAPH_CONTEXT_MENU_MARGIN_PX)
                        .max(GRAPH_CONTEXT_MENU_MARGIN_PX),
                ),
            ),
            target,
        }
    }
}

impl Default for VersionGraphTransform {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct VersionGraphLayoutNode {
    id: VersionGraphNodeId,
    x: f64,
    y: f64,
    rank: usize,
    row: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct VersionGraphLayoutEdge {
    source: VersionGraphNodeId,
    target: VersionGraphNodeId,
    kind: VersionGraphEdgeKind,
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
    path: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct VersionGraphLayout {
    nodes: Vec<VersionGraphLayoutNode>,
    edges: Vec<VersionGraphLayoutEdge>,
    width: f64,
    height: f64,
}

impl VersionGraphLayout {
    fn node(&self, id: VersionGraphNodeId) -> Option<&VersionGraphLayoutNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    fn fit_transform(&self, viewport: (f64, f64)) -> VersionGraphTransform {
        if self.width <= 0.0 || self.height <= 0.0 || viewport.0 <= 0.0 || viewport.1 <= 0.0 {
            return VersionGraphTransform::default();
        }

        let scale = (viewport.0 / self.width)
            .min(viewport.1 / self.height)
            .min(1.0)
            .clamp(GRAPH_MIN_ZOOM, 1.0);
        VersionGraphTransform {
            scale,
            offset_x: ((viewport.0 - self.width * scale) / 2.0).max(0.0),
            offset_y: ((viewport.1 - self.height * scale) / 2.0).max(0.0),
        }
    }

    fn center_transform(
        &self,
        id: VersionGraphNodeId,
        viewport: (f64, f64),
        scale: f64,
    ) -> VersionGraphTransform {
        let Some(node) = self.node(id) else {
            return self.fit_transform(viewport);
        };
        let scale = scale.clamp(GRAPH_MIN_ZOOM, GRAPH_MAX_ZOOM);
        VersionGraphTransform {
            scale,
            offset_x: viewport.0 / 2.0 - (node.x + GRAPH_NODE_WIDTH_PX / 2.0) * scale,
            offset_y: viewport.1 / 2.0 - (node.y + GRAPH_NODE_HEIGHT_PX / 2.0) * scale,
        }
    }
}

fn version_graph_node_key(id: VersionGraphNodeId) -> (uuid::Uuid, i32) {
    (id.recipe_id, id.version_number)
}

fn version_graph_edge_key(
    source: VersionGraphNodeId,
    target: VersionGraphNodeId,
    kind: VersionGraphEdgeKind,
) -> ((uuid::Uuid, i32), (uuid::Uuid, i32), u8) {
    (
        version_graph_node_key(source),
        version_graph_node_key(target),
        match kind {
            VersionGraphEdgeKind::Version => 0,
            VersionGraphEdgeKind::Fork => 1,
        },
    )
}

fn stable_recipe_rows(
    recipe_ids: &[uuid::Uuid],
    edges: &[(VersionGraphNodeId, VersionGraphNodeId, VersionGraphEdgeKind)],
) -> HashMap<uuid::Uuid, usize> {
    let mut children = HashMap::<uuid::Uuid, Vec<uuid::Uuid>>::new();
    let mut incoming = recipe_ids
        .iter()
        .copied()
        .map(|recipe_id| (recipe_id, 0usize))
        .collect::<HashMap<_, _>>();

    for (source, target, kind) in edges {
        if *kind != VersionGraphEdgeKind::Fork || source.recipe_id == target.recipe_id {
            continue;
        }
        let targets = children.entry(source.recipe_id).or_default();
        if !targets.contains(&target.recipe_id) {
            targets.push(target.recipe_id);
            *incoming.entry(target.recipe_id).or_default() += 1;
        }
    }
    for targets in children.values_mut() {
        targets.sort_unstable();
    }

    let mut roots = recipe_ids
        .iter()
        .copied()
        .filter(|recipe_id| incoming.get(recipe_id).copied().unwrap_or_default() == 0)
        .collect::<Vec<_>>();
    roots.sort_unstable();
    let mut queue = roots.into_iter().collect::<VecDeque<_>>();
    let mut visited = HashSet::new();
    let mut ordered_recipes = Vec::with_capacity(recipe_ids.len());

    while let Some(recipe_id) = queue.pop_front() {
        if !visited.insert(recipe_id) {
            continue;
        }
        ordered_recipes.push(recipe_id);
        if let Some(recipe_children) = children.get(&recipe_id) {
            queue.extend(recipe_children.iter().copied());
        }
    }

    // Cyclic components have no root. Seed each remaining component from its
    // stable key and reuse the same guarded breadth-first ordering.
    for recipe_id in recipe_ids {
        if visited.contains(recipe_id) {
            continue;
        }
        queue.push_back(*recipe_id);
        while let Some(cyclic_recipe_id) = queue.pop_front() {
            if !visited.insert(cyclic_recipe_id) {
                continue;
            }
            ordered_recipes.push(cyclic_recipe_id);
            if let Some(recipe_children) = children.get(&cyclic_recipe_id) {
                queue.extend(recipe_children.iter().copied());
            }
        }
    }

    ordered_recipes
        .into_iter()
        .enumerate()
        .map(|(row, recipe_id)| (recipe_id, row))
        .collect()
}

fn build_version_graph_layout(graph: &VersionGraph) -> VersionGraphLayout {
    let mut node_ids = graph
        .nodes
        .iter()
        .map(VersionGraphNode::id)
        .collect::<Vec<_>>();
    node_ids.sort_by_key(|id| version_graph_node_key(*id));
    node_ids.dedup();
    if node_ids.is_empty() {
        return VersionGraphLayout::default();
    }

    let node_id_set = node_ids.iter().copied().collect::<HashSet<_>>();
    let mut edges = graph
        .edges
        .iter()
        .filter(|edge| node_id_set.contains(&edge.source()) && node_id_set.contains(&edge.target()))
        .map(|edge| (edge.source(), edge.target(), edge.kind()))
        .collect::<Vec<_>>();
    edges.sort_by_key(|(source, target, kind)| version_graph_edge_key(*source, *target, *kind));
    edges.dedup();

    let mut rank_edges = edges
        .iter()
        .map(|(source, target, _)| (*source, *target))
        .collect::<Vec<_>>();
    let mut nodes_by_recipe = HashMap::<uuid::Uuid, Vec<VersionGraphNodeId>>::new();
    for id in &node_ids {
        nodes_by_recipe.entry(id.recipe_id).or_default().push(*id);
    }
    for recipe_nodes in nodes_by_recipe.values_mut() {
        recipe_nodes.sort_by_key(|id| id.version_number);
        rank_edges.extend(recipe_nodes.windows(2).map(|pair| (pair[0], pair[1])));
    }
    rank_edges.sort_by_key(|(source, target)| {
        (
            version_graph_node_key(*source),
            version_graph_node_key(*target),
        )
    });
    rank_edges.dedup();

    let mut incoming = node_ids
        .iter()
        .copied()
        .map(|id| (id, 0usize))
        .collect::<HashMap<_, _>>();
    let mut outgoing = HashMap::<VersionGraphNodeId, Vec<VersionGraphNodeId>>::new();
    for (source, target) in &rank_edges {
        outgoing.entry(*source).or_default().push(*target);
        *incoming.entry(*target).or_default() += 1;
    }
    for targets in outgoing.values_mut() {
        targets.sort_by_key(|id| version_graph_node_key(*id));
    }

    let mut ready = node_ids
        .iter()
        .copied()
        .filter(|id| incoming.get(id).copied().unwrap_or_default() == 0)
        .collect::<Vec<_>>();
    ready.sort_by_key(|id| version_graph_node_key(*id));
    let mut ranks = node_ids
        .iter()
        .copied()
        .map(|id| (id, 0usize))
        .collect::<HashMap<_, _>>();
    let mut ranked = HashSet::new();

    while !ready.is_empty() {
        let source = ready.remove(0);
        if !ranked.insert(source) {
            continue;
        }
        let source_rank = ranks.get(&source).copied().unwrap_or_default();
        if let Some(targets) = outgoing.get(&source) {
            for target in targets {
                ranks
                    .entry(*target)
                    .and_modify(|rank| *rank = (*rank).max(source_rank.saturating_add(1)));
                let target_incoming = incoming.entry(*target).or_default();
                *target_incoming = target_incoming.saturating_sub(1);
                if *target_incoming == 0 {
                    ready.push(*target);
                }
            }
            ready.sort_by_key(|id| version_graph_node_key(*id));
        }
    }

    // Nodes left after Kahn's algorithm belong to, or depend on, a malformed
    // cycle. Give each a finite stable fallback rank.
    let mut fallback_rank = ranks
        .iter()
        .filter(|(id, _)| ranked.contains(id))
        .map(|(_, rank)| *rank)
        .max()
        .unwrap_or_default()
        .saturating_add(1);
    for id in &node_ids {
        if !ranked.contains(id) {
            ranks.insert(*id, fallback_rank);
            fallback_rank = fallback_rank.saturating_add(1);
        }
    }

    let mut recipe_ids = node_ids.iter().map(|id| id.recipe_id).collect::<Vec<_>>();
    recipe_ids.sort_unstable();
    recipe_ids.dedup();
    let recipe_rows = stable_recipe_rows(&recipe_ids, &edges);

    let mut layout_nodes = node_ids
        .iter()
        .map(|id| {
            let rank = ranks.get(id).copied().unwrap_or_default();
            let row = recipe_rows.get(&id.recipe_id).copied().unwrap_or_default();
            VersionGraphLayoutNode {
                id: *id,
                x: GRAPH_LAYOUT_PADDING_PX
                    + rank as f64 * (GRAPH_NODE_WIDTH_PX + GRAPH_COLUMN_GAP_PX),
                y: GRAPH_LAYOUT_PADDING_PX + row as f64 * (GRAPH_NODE_HEIGHT_PX + GRAPH_ROW_GAP_PX),
                rank,
                row,
            }
        })
        .collect::<Vec<_>>();
    layout_nodes.sort_by_key(|node| (node.rank, node.row, version_graph_node_key(node.id)));
    let positioned_nodes = layout_nodes
        .iter()
        .copied()
        .map(|node| (node.id, node))
        .collect::<HashMap<_, _>>();

    let mut layout_edges = Vec::with_capacity(edges.len());
    for (source, target, kind) in edges {
        let (Some(source_node), Some(target_node)) =
            (positioned_nodes.get(&source), positioned_nodes.get(&target))
        else {
            continue;
        };
        let source_x = source_node.x + GRAPH_NODE_WIDTH_PX;
        let source_y = source_node.y + GRAPH_NODE_HEIGHT_PX / 2.0;
        let target_x = target_node.x;
        let target_y = target_node.y + GRAPH_NODE_HEIGHT_PX / 2.0;
        let control_distance = if target_x > source_x {
            (target_x - source_x) / 2.0
        } else {
            GRAPH_COLUMN_GAP_PX / 2.0 + source_node.row.abs_diff(target_node.row) as f64 * 12.0
        };
        layout_edges.push(VersionGraphLayoutEdge {
            source,
            target,
            kind,
            source_x,
            source_y,
            target_x,
            target_y,
            path: format!(
                "M {source_x:.2} {source_y:.2} C {:.2} {source_y:.2}, {:.2} {target_y:.2}, {target_x:.2} {target_y:.2}",
                source_x + control_distance,
                target_x - control_distance,
            ),
        });
    }

    let max_rank = layout_nodes
        .iter()
        .map(|node| node.rank)
        .max()
        .unwrap_or_default();
    let max_row = layout_nodes
        .iter()
        .map(|node| node.row)
        .max()
        .unwrap_or_default();
    VersionGraphLayout {
        nodes: layout_nodes,
        edges: layout_edges,
        width: GRAPH_LAYOUT_PADDING_PX * 2.0
            + (max_rank + 1) as f64 * GRAPH_NODE_WIDTH_PX
            + max_rank as f64 * GRAPH_COLUMN_GAP_PX,
        height: GRAPH_LAYOUT_PADDING_PX * 2.0
            + (max_row + 1) as f64 * GRAPH_NODE_HEIGHT_PX
            + max_row as f64 * GRAPH_ROW_GAP_PX,
    }
}

fn zoom_version_graph(
    transform: VersionGraphTransform,
    viewport: (f64, f64),
    zoom_factor: f64,
) -> VersionGraphTransform {
    zoom_version_graph_at(transform, (viewport.0 / 2.0, viewport.1 / 2.0), zoom_factor)
}

fn zoom_version_graph_at(
    transform: VersionGraphTransform,
    anchor: (f64, f64),
    zoom_factor: f64,
) -> VersionGraphTransform {
    let scale = (transform.scale * zoom_factor).clamp(GRAPH_MIN_ZOOM, GRAPH_MAX_ZOOM);
    let graph_anchor_x = (anchor.0 - transform.offset_x) / transform.scale;
    let graph_anchor_y = (anchor.1 - transform.offset_y) / transform.scale;
    VersionGraphTransform {
        scale,
        offset_x: anchor.0 - graph_anchor_x * scale,
        offset_y: anchor.1 - graph_anchor_y * scale,
    }
}

fn center_version_graph_at(
    transform: VersionGraphTransform,
    context_point: (f64, f64),
    viewport: (f64, f64),
) -> VersionGraphTransform {
    let graph_point_x = (context_point.0 - transform.offset_x) / transform.scale;
    let graph_point_y = (context_point.1 - transform.offset_y) / transform.scale;
    VersionGraphTransform {
        offset_x: viewport.0 / 2.0 - graph_point_x * transform.scale,
        offset_y: viewport.1 / 2.0 - graph_point_y * transform.scale,
        ..transform
    }
}

fn next_context_menu_focus_index(
    current: usize,
    direction: i32,
    enabled_actions: [bool; 5],
) -> usize {
    let action_count = enabled_actions.len();
    for distance in 1..=action_count {
        let candidate = if direction < 0 {
            (current + action_count - distance) % action_count
        } else {
            (current + distance) % action_count
        };
        if enabled_actions[candidate] {
            return candidate;
        }
    }
    current.min(action_count - 1)
}

fn first_context_menu_focus_index(enabled_actions: [bool; 5]) -> usize {
    enabled_actions
        .iter()
        .position(|enabled| *enabled)
        .unwrap_or(0)
}

fn snapshot_context_menu_route(node_id: VersionGraphNodeId, action_index: usize) -> Option<Route> {
    let recipe_id = node_id.recipe_id.to_string();
    match action_index {
        0 => Some(Route::RecipeVersionDetail {
            id: recipe_id,
            version_number: node_id.version_number,
        }),
        1 => Some(Route::RecipeDetail { id: recipe_id }),
        2 => Some(Route::RecipeVersionGraph { id: recipe_id }),
        _ => None,
    }
}

fn focus_mounted_element(element: Option<MountedEvent>) {
    if let Some(element) = element {
        spawn(async move {
            let _ = element.data().set_focus(true).await;
        });
    }
}

fn restore_context_menu_focus(
    target: VersionGraphContextMenuTarget,
    graph_canvas: Option<MountedEvent>,
    graph_nodes: &HashMap<VersionGraphNodeId, MountedEvent>,
) {
    let target_element = match target {
        VersionGraphContextMenuTarget::Background => graph_canvas,
        VersionGraphContextMenuTarget::Snapshot(node_id) => {
            graph_nodes.get(&node_id).cloned().or(graph_canvas)
        }
    };
    focus_mounted_element(target_element);
}

fn pan_version_graph(
    transform: VersionGraphTransform,
    mouse_delta: (f64, f64),
) -> VersionGraphTransform {
    VersionGraphTransform {
        offset_x: transform.offset_x + mouse_delta.0,
        offset_y: transform.offset_y + mouse_delta.1,
        ..transform
    }
}

fn wheel_delta_pixels(delta: WheelDelta, viewport_height: f64) -> f64 {
    match delta {
        WheelDelta::Pixels(delta) => delta.y,
        WheelDelta::Lines(delta) => delta.y * GRAPH_WHEEL_LINE_HEIGHT_PX,
        WheelDelta::Pages(delta) => delta.y * viewport_height,
    }
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

fn graph_connection_label(edge: &VersionGraphLayoutEdge, nodes: &[VersionGraphNode]) -> String {
    let source = edge.source;
    let target = edge.target;
    match edge.kind {
        VersionGraphEdgeKind::Fork => {
            let edge =
                VersionGraphEdge::fork(source.recipe_id, source.version_number, target.recipe_id)
                    .expect("layout fork edges always connect distinct recipes");
            fork_connection_label(&edge, nodes)
        }
        VersionGraphEdgeKind::Version => format!(
            "{} v{} advances to {} v{}",
            fork_endpoint_label(source, nodes),
            source.version_number,
            fork_endpoint_label(target, nodes),
            target.version_number,
        ),
    }
}

fn version_graph_node_class(
    node: &VersionGraphNode,
    is_fork_source: bool,
    is_fork_target: bool,
) -> String {
    let mut classes = vec!["recipe-version-graph__node"];
    if node.is_current_version() {
        classes.push("recipe-version-graph__node--current");
    }
    if node.is_deleted() {
        classes.push("recipe-version-graph__node--deleted");
    }
    if node.is_redacted() {
        classes.push("recipe-version-graph__node--redacted");
    }
    if node.is_selected_recipe() {
        classes.push("recipe-version-graph__node--main-recipe");
    }
    if is_fork_source {
        classes.push("recipe-version-graph__node--fork-source");
    }
    if is_fork_target {
        classes.push("recipe-version-graph__node--fork-target");
    }
    classes.join(" ")
}

#[component]
fn VersionGraphSnapshotCard(
    node: VersionGraphNode,
    is_fork_source: bool,
    is_fork_target: bool,
) -> Element {
    let version_number = node.id().version_number;
    let node_class = version_graph_node_class(&node, is_fork_source, is_fork_target);
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
    let main_recipe_role = if node.is_selected_recipe() {
        ", main recipe"
    } else {
        ""
    };

    rsx! {
        article {
            class: node_class,
            aria_label: format!(
                "Version {version_number}: {title}{relationship_role}{main_recipe_role}"
            ),
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
                    div { class: "recipe-version-graph__badges",
                        if node.is_selected_recipe() {
                            span {
                                class: "recipe-version-graph__main-recipe-badge",
                                "Main recipe"
                            }
                        }
                        if node.is_current_version() {
                            span { class: "recipe-version-graph__current-badge", "Current" }
                        } else if node.is_deleted() {
                            span { class: "recipe-version-graph__deleted-badge", "Deleted recipe" }
                        }
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
    let navigator = navigator();
    let recipe_id_is_valid = uuid::Uuid::parse_str(&id).is_ok();
    let graph_resource = use_resource(use_reactive((&id,), move |(recipe_id,)| async move {
        if uuid::Uuid::parse_str(&recipe_id).is_err() {
            return Err(ServerFnError::new("Invalid recipe ID"));
        }
        get_recipe_version_graph(recipe_id).await
    }));
    let mut graph_transform = use_signal(VersionGraphTransform::default);
    let mut graph_viewport = use_signal(|| GRAPH_DEFAULT_VIEWPORT);
    let mut initial_graph_fit_complete = use_signal(|| false);
    let mut graph_canvas = use_signal(|| None::<MountedEvent>);
    // Present only while an auxiliary-button drag is active.
    let mut pan_mouse_position = use_signal(|| None::<(f64, f64)>);
    let mut graph_context_menu = use_signal(|| None::<VersionGraphContextMenu>);
    let mut graph_context_menu_focus_index = use_signal(|| 0usize);
    let mut graph_node_elements = use_signal(HashMap::<VersionGraphNodeId, MountedEvent>::new);
    let mut context_zoom_in = use_signal(|| None::<MountedEvent>);
    let mut context_zoom_out = use_signal(|| None::<MountedEvent>);
    let mut context_center_here = use_signal(|| None::<MountedEvent>);
    let mut context_center_current = use_signal(|| None::<MountedEvent>);
    let mut context_fit = use_signal(|| None::<MountedEvent>);

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
    let layout = build_version_graph_layout(&graph);
    let fork_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.kind() == VersionGraphEdgeKind::Fork)
        .cloned()
        .collect::<Vec<_>>();
    let current_snapshot_id = graph
        .nodes
        .iter()
        .find(|node| node.is_selected_recipe() && node.is_current_version())
        .or_else(|| graph.nodes.iter().find(|node| node.is_current_version()))
        .map(VersionGraphNode::id);
    let nodes_by_id = graph
        .nodes
        .iter()
        .cloned()
        .map(|node| (node.id(), node))
        .collect::<HashMap<_, _>>();
    let transform = graph_transform();
    let is_mouse_panning = pan_mouse_position().is_some();
    let canvas_class = if is_mouse_panning {
        "recipe-version-graph__canvas-viewport recipe-version-graph__canvas-viewport--panning"
    } else {
        "recipe-version-graph__canvas-viewport"
    };
    let stage_style = format!(
        "width: {:.2}px; height: {:.2}px; transform: translate({:.2}px, {:.2}px) scale({:.5});",
        layout.width, layout.height, transform.offset_x, transform.offset_y, transform.scale,
    );
    let card_layer_style = format!(
        "width: {:.2}px; height: {:.2}px;",
        layout.width, layout.height,
    );
    let view_box = format!("0 0 {:.2} {:.2}", layout.width, layout.height);
    let layout_for_mount = layout.clone();
    let layout_for_resize = layout.clone();
    let layout_for_fit = layout.clone();
    let layout_for_center = layout.clone();
    let layout_for_context_fit = layout.clone();
    let layout_for_context_center_current = layout.clone();
    let layout_for_snapshot_center = layout.clone();
    let layout_for_snapshot_zoom = layout.clone();
    let active_context_menu = graph_context_menu();
    let snapshot_context_menu = active_context_menu
        .and_then(|context_menu| match context_menu.target {
            VersionGraphContextMenuTarget::Background => None,
            VersionGraphContextMenuTarget::Snapshot(node_id) => Some(node_id),
        })
        .and_then(|node_id| {
            nodes_by_id.get(&node_id).map(|node| {
                let title = node
                    .title()
                    .unwrap_or(if node.is_deleted() {
                        "Deleted recipe"
                    } else {
                        "Private recipe"
                    })
                    .to_string();
                (node_id, title, !node.is_redacted() && !node.is_deleted())
            })
        });
    let context_menu_actions_enabled = match snapshot_context_menu.as_ref() {
        Some((_, _, navigation_available)) => [
            *navigation_available,
            *navigation_available,
            *navigation_available,
            true,
            true,
        ],
        None => [true, true, true, current_snapshot_id.is_some(), true],
    };
    let is_snapshot_context_menu = snapshot_context_menu.is_some();

    rsx! {
        div { class: "container",
            PageHeader {
                title: heading_title,
                action: Some(back_to_recipe),
            }
            Card {
                div {
                    class: "recipe-version-graph",
                    onclick: move |_| {
                        if let Some(context_menu) = graph_context_menu() {
                            graph_context_menu.set(None);
                            restore_context_menu_focus(
                                context_menu.target,
                                graph_canvas(),
                                &graph_node_elements(),
                            );
                        }
                    },
                    onkeydown: move |event| {
                        if event.data().key() == Key::Escape {
                            if let Some(context_menu) = graph_context_menu() {
                                graph_context_menu.set(None);
                                restore_context_menu_focus(
                                    context_menu.target,
                                    graph_canvas(),
                                    &graph_node_elements(),
                                );
                            }
                        }
                    },
                    p { class: "recipe-version-graph__description",
                        "Explore immutable snapshots across the connected fork family."
                    }
                    p { class: "recipe-version-graph__help",
                        "Scroll to zoom; middle-click and drag to pan; right-click the background for more navigation options."
                    }
                    div {
                        class: "recipe-version-graph__toolbar",
                        aria_label: "Version graph navigation",
                        div { class: "recipe-version-graph__controls",
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control",
                                aria_label: "Pan graph left",
                                onclick: move |_| graph_transform.with_mut(|transform| {
                                    transform.offset_x += GRAPH_PAN_STEP_PX;
                                }),
                                "←"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control",
                                aria_label: "Pan graph right",
                                onclick: move |_| graph_transform.with_mut(|transform| {
                                    transform.offset_x -= GRAPH_PAN_STEP_PX;
                                }),
                                "→"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control",
                                aria_label: "Pan graph up",
                                onclick: move |_| graph_transform.with_mut(|transform| {
                                    transform.offset_y += GRAPH_PAN_STEP_PX;
                                }),
                                "↑"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control",
                                aria_label: "Pan graph down",
                                onclick: move |_| graph_transform.with_mut(|transform| {
                                    transform.offset_y -= GRAPH_PAN_STEP_PX;
                                }),
                                "↓"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control",
                                aria_label: "Zoom in",
                                onclick: move |_| {
                                    let next = zoom_version_graph(
                                        graph_transform(),
                                        graph_viewport(),
                                        GRAPH_ZOOM_FACTOR,
                                    );
                                    graph_transform.set(next);
                                },
                                "+"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control",
                                aria_label: "Zoom out",
                                onclick: move |_| {
                                    let next = zoom_version_graph(
                                        graph_transform(),
                                        graph_viewport(),
                                        1.0 / GRAPH_ZOOM_FACTOR,
                                    );
                                    graph_transform.set(next);
                                },
                                "−"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control recipe-version-graph__control--text",
                                aria_label: "Fit graph to viewport",
                                onclick: move |_| {
                                    graph_transform.set(
                                        layout_for_fit.fit_transform(graph_viewport())
                                    );
                                },
                                "Fit"
                            }
                            button {
                                r#type: "button",
                                class: "recipe-version-graph__control recipe-version-graph__control--text",
                                aria_label: "Center current snapshot",
                                disabled: current_snapshot_id.is_none(),
                                onclick: move |_| {
                                    if let Some(current_snapshot_id) = current_snapshot_id {
                                        graph_transform.set(layout_for_center.center_transform(
                                            current_snapshot_id,
                                            graph_viewport(),
                                            graph_transform().scale,
                                        ));
                                    }
                                },
                                "Current"
                            }
                        }
                        div {
                            class: "recipe-version-graph__legend",
                            role: "note",
                            aria_label: "Graph connection legend",
                            span { class: "recipe-version-graph__legend-item",
                                span {
                                    class: "recipe-version-graph__legend-line recipe-version-graph__legend-line--version",
                                    aria_hidden: "true",
                                }
                                "Version history"
                            }
                            span { class: "recipe-version-graph__legend-item",
                                span {
                                    class: "recipe-version-graph__legend-line recipe-version-graph__legend-line--fork",
                                    aria_hidden: "true",
                                }
                                "Fork"
                            }
                        }
                    }
                    div {
                        class: canvas_class,
                        tabindex: "0",
                        aria_label: "Interactive recipe version graph",
                        onmounted: move |event| {
                            graph_canvas.set(Some(event.clone()));
                            let mounted = event.data();
                            let layout = layout_for_mount.clone();
                            spawn(async move {
                                if let Ok(rect) = mounted.get_client_rect().await {
                                    let viewport = (rect.size.width, rect.size.height);
                                    if viewport.0 > 0.0 && viewport.1 > 0.0 {
                                        graph_viewport.set(viewport);
                                        graph_transform.set(layout.fit_transform(viewport));
                                        initial_graph_fit_complete.set(true);
                                    }
                                }
                            });
                        },
                        onresize: move |event| {
                            if graph_context_menu().is_some() {
                                graph_context_menu.set(None);
                                focus_mounted_element(graph_canvas());
                            }
                            if let Ok(size) = event.data().get_content_box_size() {
                                let viewport = (size.width, size.height);
                                if viewport.0 > 0.0 && viewport.1 > 0.0 {
                                    graph_viewport.set(viewport);
                                    if !initial_graph_fit_complete() {
                                        graph_transform.set(
                                            layout_for_resize.fit_transform(viewport)
                                        );
                                        initial_graph_fit_complete.set(true);
                                    }
                                }
                            }
                        },
                        onwheel: move |event| {
                            event.prevent_default();
                            if graph_context_menu().is_some() {
                                graph_context_menu.set(None);
                                focus_mounted_element(graph_canvas());
                            }
                            let Some(canvas) = graph_canvas() else {
                                return;
                            };
                            let client_position = event.data().client_coordinates();
                            let wheel_delta = event.data().delta();
                            spawn(async move {
                                let Ok(rect) = canvas.data().get_client_rect().await else {
                                    return;
                                };
                                let viewport = (rect.size.width, rect.size.height);
                                if viewport.0 <= 0.0 || viewport.1 <= 0.0 {
                                    return;
                                }
                                let anchor = (
                                    client_position.x - rect.origin.x,
                                    client_position.y - rect.origin.y,
                                );
                                let delta_pixels =
                                    wheel_delta_pixels(wheel_delta, viewport.1).clamp(-600.0, 600.0);
                                let zoom_factor =
                                    (-delta_pixels * GRAPH_WHEEL_ZOOM_SENSITIVITY).exp();
                                graph_transform.set(zoom_version_graph_at(
                                    graph_transform(),
                                    anchor,
                                    zoom_factor,
                                ));
                            });
                        },
                        oncontextmenu: move |event| {
                            event.prevent_default();
                            let Some(canvas) = graph_canvas() else {
                                return;
                            };
                            let client_position = event.data().client_coordinates();
                            spawn(async move {
                                let Ok(rect) = canvas.data().get_client_rect().await else {
                                    return;
                                };
                                let viewport = (rect.size.width, rect.size.height);
                                if viewport.0 <= 0.0 || viewport.1 <= 0.0 {
                                    return;
                                }
                                let anchor = (
                                    client_position.x - rect.origin.x,
                                    client_position.y - rect.origin.y,
                                );
                                graph_context_menu_focus_index.set(0);
                                graph_context_menu.set(Some(VersionGraphContextMenu::at(
                                    anchor,
                                    viewport,
                                    VersionGraphContextMenuTarget::Background,
                                )));
                            });
                        },
                        onmousedown: move |event| {
                            if event.data().trigger_button() == Some(MouseButton::Auxiliary) {
                                event.prevent_default();
                                if graph_context_menu().is_some() {
                                    graph_context_menu.set(None);
                                    focus_mounted_element(graph_canvas());
                                }
                                let position = event.data().client_coordinates();
                                pan_mouse_position.set(Some((position.x, position.y)));
                            }
                        },
                        onmousemove: move |event| {
                            let Some(previous_position) = pan_mouse_position() else {
                                return;
                            };
                            if !event
                                .data()
                                .held_buttons()
                                .contains(MouseButton::Auxiliary)
                            {
                                pan_mouse_position.set(None);
                                return;
                            }
                            event.prevent_default();
                            let position = event.data().client_coordinates();
                            graph_transform.set(pan_version_graph(
                                graph_transform(),
                                (
                                    position.x - previous_position.0,
                                    position.y - previous_position.1,
                                ),
                            ));
                            pan_mouse_position.set(Some((position.x, position.y)));
                        },
                        onmouseup: move |event| {
                            if event.data().trigger_button() == Some(MouseButton::Auxiliary) {
                                event.prevent_default();
                                pan_mouse_position.set(None);
                            }
                        },
                        onmouseleave: move |_| pan_mouse_position.set(None),
                        onauxclick: move |event| event.prevent_default(),
                        div {
                            class: "recipe-version-graph__canvas-stage",
                            style: stage_style,
                            svg {
                                class: "recipe-version-graph__edge-layer",
                                "aria-hidden": "true",
                                width: "{layout.width:.2}",
                                height: "{layout.height:.2}",
                                view_box,
                                defs {
                                    marker {
                                        id: "recipe-version-graph-arrow-version",
                                        marker_width: "10",
                                        marker_height: "10",
                                        ref_x: "9",
                                        ref_y: "5",
                                        orient: "auto",
                                        marker_units: "strokeWidth",
                                        path { d: "M 0 0 L 10 5 L 0 10 z" }
                                    }
                                    marker {
                                        id: "recipe-version-graph-arrow-fork",
                                        marker_width: "10",
                                        marker_height: "10",
                                        ref_x: "9",
                                        ref_y: "5",
                                        orient: "auto",
                                        marker_units: "strokeWidth",
                                        path { d: "M 0 0 L 10 5 L 0 10 z" }
                                    }
                                }
                                for edge in &layout.edges {
                                    path {
                                        key: "{edge.source.recipe_id}-{edge.source.version_number}-{edge.target.recipe_id}-{edge.target.version_number}-{edge.kind:?}",
                                        class: if edge.kind == VersionGraphEdgeKind::Fork {
                                            "recipe-version-graph__edge recipe-version-graph__edge--fork"
                                        } else {
                                            "recipe-version-graph__edge recipe-version-graph__edge--version"
                                        },
                                        d: edge.path.clone(),
                                        marker_end: if edge.kind == VersionGraphEdgeKind::Fork {
                                            "url(#recipe-version-graph-arrow-fork)"
                                        } else {
                                            "url(#recipe-version-graph-arrow-version)"
                                        },
                                    }
                                }
                            }
                            div {
                                class: "recipe-version-graph__card-layer",
                                style: card_layer_style,
                                for layout_node in &layout.nodes {
                                    if let Some(node) = nodes_by_id.get(&layout_node.id).cloned() {
                                        {
                                            let node_id = node.id();
                                            let is_fork_source = fork_edges
                                                .iter()
                                                .any(|edge| edge.source() == node_id);
                                            let is_fork_target = fork_edges
                                                .iter()
                                                .any(|edge| edge.target() == node_id);
                                            let snapshot_navigation_available =
                                                !node.is_redacted() && !node.is_deleted();
                                            let node_style = format!(
                                                "left: {:.2}px; top: {:.2}px;",
                                                layout_node.x,
                                                layout_node.y,
                                            );
                                            rsx! {
                                                div {
                                                    key: "{node_id.recipe_id}-{node_id.version_number}",
                                                    class: "recipe-version-graph__canvas-node",
                                                    style: node_style,
                                                    oncontextmenu: move |event| {
                                                        event.prevent_default();
                                                        event.stop_propagation();
                                                        let Some(canvas) = graph_canvas() else {
                                                            return;
                                                        };
                                                        let client_position =
                                                            event.data().client_coordinates();
                                                        spawn(async move {
                                                            let Ok(rect) =
                                                                canvas.data().get_client_rect().await
                                                            else {
                                                                return;
                                                            };
                                                            let viewport =
                                                                (rect.size.width, rect.size.height);
                                                            if viewport.0 <= 0.0
                                                                || viewport.1 <= 0.0
                                                            {
                                                                return;
                                                            }
                                                            let anchor = (
                                                                client_position.x - rect.origin.x,
                                                                client_position.y - rect.origin.y,
                                                            );
                                                            let enabled_actions = [
                                                                snapshot_navigation_available,
                                                                snapshot_navigation_available,
                                                                snapshot_navigation_available,
                                                                true,
                                                                true,
                                                            ];
                                                            graph_context_menu_focus_index.set(
                                                                first_context_menu_focus_index(
                                                                    enabled_actions,
                                                                ),
                                                            );
                                                            graph_context_menu.set(Some(
                                                                VersionGraphContextMenu::at(
                                                                    anchor,
                                                                    viewport,
                                                                    VersionGraphContextMenuTarget::Snapshot(
                                                                        node_id,
                                                                    ),
                                                                ),
                                                            ));
                                                        });
                                                    },
                                                    if node.is_redacted() || node.is_deleted() {
                                                        div {
                                                            class: "recipe-version-graph__static-card",
                                                            tabindex: "0",
                                                            onmounted: move |event| {
                                                                graph_node_elements
                                                                    .write()
                                                                    .insert(node_id, event);
                                                            },
                                                            VersionGraphSnapshotCard {
                                                                node,
                                                                is_fork_source,
                                                                is_fork_target,
                                                            }
                                                        }
                                                    } else {
                                                        Link {
                                                            to: Route::RecipeVersionDetail {
                                                                id: node_id.recipe_id.to_string(),
                                                                version_number: node_id.version_number,
                                                            },
                                                            class: "recipe-version-graph__node-link",
                                                            onmounted: move |event| {
                                                                graph_node_elements
                                                                    .write()
                                                                    .insert(node_id, event);
                                                            },
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
                        if let Some(context_menu) = active_context_menu {
                            div {
                                class: "recipe-version-graph__context-menu",
                                role: "menu",
                                aria_label: if is_snapshot_context_menu {
                                    "Snapshot context menu"
                                } else {
                                    "Graph navigation menu"
                                },
                                style: "left: {context_menu.position.0:.2}px; top: {context_menu.position.1:.2}px;",
                                onclick: move |event| event.stop_propagation(),
                                onkeydown: move |event| {
                                    let next_index = match event.data().key() {
                                        Key::ArrowDown => next_context_menu_focus_index(
                                            graph_context_menu_focus_index(),
                                            1,
                                            context_menu_actions_enabled,
                                        ),
                                        Key::ArrowUp => next_context_menu_focus_index(
                                            graph_context_menu_focus_index(),
                                            -1,
                                            context_menu_actions_enabled,
                                        ),
                                        Key::Home => first_context_menu_focus_index(
                                            context_menu_actions_enabled,
                                        ),
                                        Key::End => next_context_menu_focus_index(
                                            first_context_menu_focus_index(
                                                context_menu_actions_enabled,
                                            ),
                                            -1,
                                            context_menu_actions_enabled,
                                        ),
                                        Key::Escape => {
                                            event.prevent_default();
                                            event.stop_propagation();
                                            graph_context_menu.set(None);
                                            restore_context_menu_focus(
                                                context_menu.target,
                                                graph_canvas(),
                                                &graph_node_elements(),
                                            );
                                            return;
                                        }
                                        _ => return,
                                    };
                                    event.prevent_default();
                                    event.stop_propagation();
                                    graph_context_menu_focus_index.set(next_index);
                                    let target = match next_index {
                                        0 => context_zoom_in(),
                                        1 => context_zoom_out(),
                                        2 => context_center_here(),
                                        3 => context_center_current(),
                                        _ => context_fit(),
                                    };
                                    focus_mounted_element(target);
                                },
                                if let Some((
                                    snapshot_id,
                                    snapshot_title,
                                    snapshot_navigation_available,
                                )) = snapshot_context_menu {
                                    div {
                                        class: "recipe-version-graph__context-menu-heading",
                                        role: "presentation",
                                        strong { "{snapshot_title}" }
                                        span { "Version {snapshot_id.version_number}" }
                                    }
                                    if !snapshot_navigation_available {
                                        p {
                                            class: "recipe-version-graph__context-menu-unavailable",
                                            role: "note",
                                            "Snapshot navigation unavailable"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        disabled: !snapshot_navigation_available,
                                        tabindex: if graph_context_menu_focus_index() == 0 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_zoom_in.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 0 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            if snapshot_navigation_available {
                                                if let Some(route) =
                                                    snapshot_context_menu_route(snapshot_id, 0)
                                                {
                                                    graph_context_menu.set(None);
                                                    restore_context_menu_focus(
                                                        context_menu.target,
                                                        graph_canvas(),
                                                        &graph_node_elements(),
                                                    );
                                                    let _ = navigator.push(route);
                                                }
                                            }
                                        },
                                        "View snapshot"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        disabled: !snapshot_navigation_available,
                                        tabindex: if graph_context_menu_focus_index() == 1 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_zoom_out.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 1 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            if snapshot_navigation_available {
                                                if let Some(route) =
                                                    snapshot_context_menu_route(snapshot_id, 1)
                                                {
                                                    graph_context_menu.set(None);
                                                    restore_context_menu_focus(
                                                        context_menu.target,
                                                        graph_canvas(),
                                                        &graph_node_elements(),
                                                    );
                                                    let _ = navigator.push(route);
                                                }
                                            }
                                        },
                                        "View recipe"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        disabled: !snapshot_navigation_available,
                                        tabindex: if graph_context_menu_focus_index() == 2 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_center_here.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 2 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            if snapshot_navigation_available {
                                                if let Some(route) =
                                                    snapshot_context_menu_route(snapshot_id, 2)
                                                {
                                                    graph_context_menu.set(None);
                                                    restore_context_menu_focus(
                                                        context_menu.target,
                                                        graph_canvas(),
                                                        &graph_node_elements(),
                                                    );
                                                    let _ = navigator.push(route);
                                                }
                                            }
                                        },
                                        "Focus recipe graph"
                                    }
                                    div {
                                        class: "recipe-version-graph__context-menu-separator",
                                        role: "separator",
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 3 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_center_current.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 3 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            graph_transform.set(
                                                layout_for_snapshot_center.center_transform(
                                                    snapshot_id,
                                                    graph_viewport(),
                                                    graph_transform().scale,
                                                ),
                                            );
                                            graph_context_menu.set(None);
                                            restore_context_menu_focus(
                                                context_menu.target,
                                                graph_canvas(),
                                                &graph_node_elements(),
                                            );
                                        },
                                        "Center snapshot"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 4 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_fit.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 4 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            graph_transform.set(
                                                layout_for_snapshot_zoom.center_transform(
                                                    snapshot_id,
                                                    graph_viewport(),
                                                    graph_transform()
                                                        .scale
                                                        .max(GRAPH_SNAPSHOT_FOCUS_ZOOM),
                                                ),
                                            );
                                            graph_context_menu.set(None);
                                            restore_context_menu_focus(
                                                context_menu.target,
                                                graph_canvas(),
                                                &graph_node_elements(),
                                            );
                                        },
                                        "Zoom to snapshot"
                                    }
                                } else {
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 0 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_zoom_in.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 0 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            graph_transform.set(zoom_version_graph_at(
                                                graph_transform(),
                                                context_menu.anchor,
                                                GRAPH_ZOOM_FACTOR,
                                            ));
                                            graph_context_menu.set(None);
                                            focus_mounted_element(graph_canvas());
                                        },
                                        "Zoom in here"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 1 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_zoom_out.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 1 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            graph_transform.set(zoom_version_graph_at(
                                                graph_transform(),
                                                context_menu.anchor,
                                                1.0 / GRAPH_ZOOM_FACTOR,
                                            ));
                                            graph_context_menu.set(None);
                                            focus_mounted_element(graph_canvas());
                                        },
                                        "Zoom out here"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 2 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_center_here.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 2 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            graph_transform.set(center_version_graph_at(
                                                graph_transform(),
                                                context_menu.anchor,
                                                graph_viewport(),
                                            ));
                                            graph_context_menu.set(None);
                                            focus_mounted_element(graph_canvas());
                                        },
                                        "Center here"
                                    }
                                    div {
                                        class: "recipe-version-graph__context-menu-separator",
                                        role: "separator",
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 3 { "0" } else { "-1" },
                                        disabled: current_snapshot_id.is_none(),
                                        onmounted: move |event| {
                                            context_center_current.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 3 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            if let Some(current_snapshot_id) = current_snapshot_id {
                                                graph_transform.set(
                                                    layout_for_context_center_current.center_transform(
                                                        current_snapshot_id,
                                                        graph_viewport(),
                                                        graph_transform().scale,
                                                    )
                                                );
                                            }
                                            graph_context_menu.set(None);
                                            focus_mounted_element(graph_canvas());
                                        },
                                        "Center current snapshot"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 4 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_fit.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 4 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            graph_transform.set(
                                                layout_for_context_fit.fit_transform(graph_viewport())
                                            );
                                            graph_context_menu.set(None);
                                            focus_mounted_element(graph_canvas());
                                        },
                                        "Fit graph to viewport"
                                    }
                                }
                            }
                        }
                    }
                    details { class: "recipe-version-graph__outline",
                        summary { "Accessible graph outline" }
                        ol { class: "recipe-version-graph__outline-nodes",
                            for layout_node in &layout.nodes {
                                if let Some(node) = nodes_by_id.get(&layout_node.id) {
                                    {
                                        let node_id = node.id();
                                        let title = node.title().unwrap_or(if node.is_deleted() {
                                            "Deleted recipe"
                                        } else {
                                            "Private recipe"
                                        });
                                        rsx! {
                                            li {
                                                key: "outline-{node_id.recipe_id}-{node_id.version_number}",
                                                if node.is_redacted() || node.is_deleted() {
                                                    span { "{title}, version {node_id.version_number}" }
                                                } else {
                                                    Link {
                                                        to: Route::RecipeVersionDetail {
                                                            id: node_id.recipe_id.to_string(),
                                                            version_number: node_id.version_number,
                                                        },
                                                        "{title}, version {node_id.version_number}"
                                                    }
                                                }
                                                if node.is_current_version() {
                                                    span {
                                                        class: "recipe-version-graph__outline-current",
                                                        " (current)"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if !layout.edges.is_empty() {
                            h3 { "Relationships" }
                            ul { class: "recipe-version-graph__outline-relationships",
                                for edge in &layout.edges {
                                    li {
                                        key: "relationship-{edge.source.recipe_id}-{edge.source.version_number}-{edge.target.recipe_id}-{edge.target.version_number}-{edge.kind:?}",
                                        "{graph_connection_label(edge, &graph.nodes)}"
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
        VersionGraph, VersionGraphEdge, VersionGraphEdgeKind, VersionGraphNode,
        VersionGraphNodeContent, VersionGraphNodeId, VersionGraphNodeState,
    };
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn visible_node(recipe_id: Uuid, version_number: i32, title: &str) -> VersionGraphNode {
        visible_node_with_selection(recipe_id, version_number, title, false)
    }

    fn visible_node_with_selection(
        recipe_id: Uuid,
        version_number: i32,
        title: &str,
        is_selected_recipe: bool,
    ) -> VersionGraphNode {
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
                is_selected_recipe,
                is_current_version: true,
                is_deleted: false,
            },
        )
    }

    fn branched_graph() -> VersionGraph {
        let root_recipe_id = Uuid::from_u128(1);
        let first_branch_id = Uuid::from_u128(2);
        let second_branch_id = Uuid::from_u128(3);
        VersionGraph {
            nodes: vec![
                visible_node(root_recipe_id, 1, "Root v1"),
                visible_node(root_recipe_id, 2, "Root v2"),
                visible_node(first_branch_id, 1, "First branch v1"),
                visible_node(first_branch_id, 2, "First branch v2"),
                visible_node(second_branch_id, 1, "Second branch v1"),
            ],
            edges: vec![
                VersionGraphEdge::version(
                    VersionGraphNodeId {
                        recipe_id: root_recipe_id,
                        version_number: 1,
                    },
                    VersionGraphNodeId {
                        recipe_id: root_recipe_id,
                        version_number: 2,
                    },
                )
                .unwrap(),
                VersionGraphEdge::version(
                    VersionGraphNodeId {
                        recipe_id: first_branch_id,
                        version_number: 1,
                    },
                    VersionGraphNodeId {
                        recipe_id: first_branch_id,
                        version_number: 2,
                    },
                )
                .unwrap(),
                VersionGraphEdge::fork(root_recipe_id, 1, first_branch_id).unwrap(),
                VersionGraphEdge::fork(root_recipe_id, 2, second_branch_id).unwrap(),
            ],
            truncation: Default::default(),
        }
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
    fn graph_resource_reacts_to_recipe_route_changes() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("use_resource(use_reactive((&id,)"));
    }

    #[test]
    fn graph_page_renders_accessible_snapshot_cards_and_separate_connectors() {
        let source = include_str!("recipe_version_graph.rs");

        assert!(source.contains("get_recipe_version_graph"));
        assert!(source.contains("recipe-version-graph__edge-layer"));
        assert!(source.contains("aria_hidden: \"true\""));
        assert!(source.contains("recipe-version-graph__node--current"));
        assert!(source.contains("Route::RecipeVersionDetail"));
        assert!(source.contains("Version {version_number}"));
        assert!(source.contains("No snapshot image"));
        assert!(source.contains("EditedRecipeImage"));
        assert!(source.contains("prefer_thumbnail: true"));
    }

    #[test]
    fn selected_recipe_snapshots_receive_a_distinct_main_recipe_treatment() {
        let selected = visible_node_with_selection(Uuid::from_u128(1), 2, "Selected", true);
        let related = visible_node(Uuid::from_u128(2), 1, "Related");

        let selected_class = super::version_graph_node_class(&selected, false, false);
        let related_class = super::version_graph_node_class(&related, false, false);

        assert!(selected_class.contains("recipe-version-graph__node--main-recipe"));
        assert!(selected_class.contains("recipe-version-graph__node--current"));
        assert!(!related_class.contains("recipe-version-graph__node--main-recipe"));

        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");
        let css = include_str!("../../assets/main.css");

        assert!(page.contains("recipe-version-graph__main-recipe-badge"));
        assert!(page.contains("Main recipe"));
        assert!(page.contains(", main recipe"));
        assert!(css.contains(".recipe-version-graph__node--main-recipe"));
        assert!(css.contains("background: color-mix(in srgb, var(--success) 24%, var(--surface));"));
        assert!(css.contains(
            ".recipe-version-graph__node-link:hover .recipe-version-graph__node--main-recipe"
        ));
        assert!(
            css.contains(".recipe-version-graph__node--main-recipe .recipe-version-graph__summary")
        );
        assert!(css.contains("@keyframes recipe-version-graph-main-recipe-breathe"));
        assert!(css.contains(
            "animation: recipe-version-graph-main-recipe-breathe 2s ease-in-out infinite;"
        ));
        assert!(css.contains("var(--success) 12%, var(--surface)"));
        assert!(css.contains("var(--success) 42%, var(--surface)"));
        assert!(css.contains("@media (prefers-reduced-motion: reduce)"));
        assert!(css.contains(".recipe-version-graph__node--main-recipe::after"));
        assert!(css.contains("outline: 9px solid color-mix(in srgb, var(--success) 88%, white);"));
        assert!(css.contains("inset: 10px;"));
        assert!(css.contains("pointer-events: none;"));
        assert!(css.contains(".recipe-version-graph__main-recipe-badge"));
    }

    #[test]
    fn graph_page_visually_distinguishes_fork_connections() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("VersionGraphEdgeKind::Fork"));
        assert!(page.contains("recipe-version-graph__edge--fork"));
        assert!(page.contains("recipe-version-graph__edge--version"));
        assert!(page.contains("recipe-version-graph__node--fork-source"));
        assert!(page.contains("recipe-version-graph__node--fork-target"));
        assert!(page.contains("fork_connection_label"));
        assert!(page.contains("forked into"));
        assert!(page.contains("Deleted recipe"));
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
    fn layered_layout_advances_time_and_separates_fork_branches() {
        let graph = branched_graph();
        let layout = super::build_version_graph_layout(&graph);

        for edge in &layout.edges {
            let source = layout
                .node(edge.source)
                .expect("layout source for every edge");
            let target = layout
                .node(edge.target)
                .expect("layout target for every edge");
            assert!(
                target.x > source.x,
                "directed graph time must advance left to right"
            );
            assert_eq!(edge.source_x, source.x + super::GRAPH_NODE_WIDTH_PX);
            assert_eq!(edge.target_x, target.x);
            assert!(edge.path.starts_with('M'));
            assert!(edge.path.contains('C'));
            if edge.kind == VersionGraphEdgeKind::Version {
                assert_eq!(source.y, target.y);
            } else {
                assert_ne!(source.y, target.y);
            }
        }

        let first_branch = layout
            .node(VersionGraphNodeId {
                recipe_id: Uuid::from_u128(2),
                version_number: 1,
            })
            .unwrap();
        let second_branch = layout
            .node(VersionGraphNodeId {
                recipe_id: Uuid::from_u128(3),
                version_number: 1,
            })
            .unwrap();
        assert_ne!(first_branch.y, second_branch.y);
    }

    #[test]
    fn fit_and_center_transforms_keep_layout_targets_in_view() {
        let layout = super::build_version_graph_layout(&branched_graph());
        let viewport = (900.0, 560.0);
        let fit = layout.fit_transform(viewport);

        assert!(fit.scale >= super::GRAPH_MIN_ZOOM);
        assert!(fit.scale <= 1.0);
        assert!(fit.offset_x >= 0.0);
        assert!(fit.offset_y >= 0.0);
        assert!(fit.offset_x + layout.width * fit.scale <= viewport.0 + f64::EPSILON);
        assert!(fit.offset_y + layout.height * fit.scale <= viewport.1 + f64::EPSILON);

        let target_id = VersionGraphNodeId {
            recipe_id: Uuid::from_u128(2),
            version_number: 2,
        };
        let target = layout.node(target_id).unwrap();
        let centered = layout.center_transform(target_id, viewport, 0.85);
        assert!(
            (centered.offset_x + (target.x + super::GRAPH_NODE_WIDTH_PX / 2.0) * centered.scale
                - viewport.0 / 2.0)
                .abs()
                < f64::EPSILON
        );
        assert!(
            (centered.offset_y + (target.y + super::GRAPH_NODE_HEIGHT_PX / 2.0) * centered.scale
                - viewport.1 / 2.0)
                .abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn fit_transform_contains_the_largest_supported_graph() {
        let layout = super::VersionGraphLayout {
            width: 356_000.0,
            height: 356_000.0,
            ..Default::default()
        };
        let viewport = (900.0, 560.0);

        let fit = layout.fit_transform(viewport);

        assert!(fit.offset_x + layout.width * fit.scale <= viewport.0 + f64::EPSILON);
        assert!(fit.offset_y + layout.height * fit.scale <= viewport.1 + f64::EPSILON);
    }

    #[test]
    fn cursor_anchored_zoom_keeps_the_point_beneath_the_cursor_stationary() {
        let transform = super::VersionGraphTransform {
            scale: 0.75,
            offset_x: 80.0,
            offset_y: -24.0,
        };
        let cursor = (320.0, 180.0);
        let graph_point = (
            (cursor.0 - transform.offset_x) / transform.scale,
            (cursor.1 - transform.offset_y) / transform.scale,
        );

        let zoomed = super::zoom_version_graph_at(transform, cursor, 1.2);

        assert!((zoomed.offset_x + graph_point.0 * zoomed.scale - cursor.0).abs() < f64::EPSILON);
        assert!((zoomed.offset_y + graph_point.1 * zoomed.scale - cursor.1).abs() < f64::EPSILON);
    }

    #[test]
    fn panning_moves_the_graph_by_the_mouse_delta() {
        let transform = super::VersionGraphTransform {
            scale: 0.75,
            offset_x: 80.0,
            offset_y: -24.0,
        };

        let panned = super::pan_version_graph(transform, (18.0, -12.0));

        assert_eq!(panned.scale, transform.scale);
        assert_eq!(panned.offset_x, 98.0);
        assert_eq!(panned.offset_y, -36.0);
    }

    #[test]
    fn centering_a_context_point_preserves_zoom_and_moves_it_to_the_viewport_center() {
        let transform = super::VersionGraphTransform {
            scale: 0.75,
            offset_x: 80.0,
            offset_y: -24.0,
        };
        let context_point = (320.0, 180.0);
        let viewport = (900.0, 560.0);
        let graph_point = (
            (context_point.0 - transform.offset_x) / transform.scale,
            (context_point.1 - transform.offset_y) / transform.scale,
        );

        let centered = super::center_version_graph_at(transform, context_point, viewport);

        assert_eq!(centered.scale, transform.scale);
        assert!(
            (centered.offset_x + graph_point.0 * centered.scale - viewport.0 / 2.0).abs()
                < f64::EPSILON
        );
        assert!(
            (centered.offset_y + graph_point.1 * centered.scale - viewport.1 / 2.0).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn context_menu_keyboard_navigation_wraps_and_skips_disabled_actions() {
        let all_actions = [true; 5];
        let private_snapshot_actions = [false, false, false, true, true];

        assert_eq!(super::next_context_menu_focus_index(0, 1, all_actions), 1);
        assert_eq!(super::next_context_menu_focus_index(4, 1, all_actions), 0);
        assert_eq!(super::next_context_menu_focus_index(0, -1, all_actions), 4);
        assert_eq!(
            super::first_context_menu_focus_index(private_snapshot_actions),
            3
        );
        assert_eq!(
            super::next_context_menu_focus_index(3, 1, private_snapshot_actions),
            4
        );
        assert_eq!(
            super::next_context_menu_focus_index(4, 1, private_snapshot_actions),
            3
        );
    }

    #[test]
    fn snapshot_context_menu_clamps_inside_the_graph_viewport() {
        let node_id = VersionGraphNodeId {
            recipe_id: Uuid::from_u128(42),
            version_number: 3,
        };
        let menu = super::VersionGraphContextMenu::at(
            (895.0, 555.0),
            (900.0, 560.0),
            super::VersionGraphContextMenuTarget::Snapshot(node_id),
        );

        assert_eq!(menu.anchor, (895.0, 555.0));
        assert_eq!(menu.position, (668.0, 216.0));
    }

    #[test]
    fn snapshot_context_menu_routes_target_the_selected_recipe_and_version() {
        let recipe_id = Uuid::from_u128(42);
        let node_id = VersionGraphNodeId {
            recipe_id,
            version_number: 3,
        };

        assert!(matches!(
            super::snapshot_context_menu_route(node_id, 0),
            Some(crate::Route::RecipeVersionDetail {
                id,
                version_number: 3,
            }) if id == recipe_id.to_string()
        ));
        assert!(matches!(
            super::snapshot_context_menu_route(node_id, 1),
            Some(crate::Route::RecipeDetail { id }) if id == recipe_id.to_string()
        ));
        assert!(matches!(
            super::snapshot_context_menu_route(node_id, 2),
            Some(crate::Route::RecipeVersionGraph { id }) if id == recipe_id.to_string()
        ));
        assert!(super::snapshot_context_menu_route(node_id, 3).is_none());
    }

    #[test]
    fn malformed_cycles_produce_a_finite_layout_with_every_node_once() {
        let first_recipe_id = Uuid::from_u128(10);
        let second_recipe_id = Uuid::from_u128(11);
        let graph = VersionGraph {
            nodes: vec![
                visible_node(first_recipe_id, 1, "First"),
                visible_node(second_recipe_id, 1, "Second"),
            ],
            edges: vec![
                VersionGraphEdge::fork(first_recipe_id, 1, second_recipe_id).unwrap(),
                VersionGraphEdge::fork(second_recipe_id, 1, first_recipe_id).unwrap(),
            ],
            truncation: Default::default(),
        };

        let layout = super::build_version_graph_layout(&graph);

        assert_eq!(layout.nodes.len(), 2);
        assert_eq!(layout.edges.len(), 2);
        assert!(layout.width.is_finite());
        assert!(layout.height.is_finite());
        assert!(layout
            .nodes
            .iter()
            .all(|node| node.x.is_finite() && node.y.is_finite()));
    }

    #[test]
    fn graph_page_exposes_navigation_controls_and_an_outline_fallback() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        for label in [
            "Pan graph left",
            "Pan graph right",
            "Pan graph up",
            "Pan graph down",
            "Zoom in",
            "Zoom out",
            "Fit graph to viewport",
            "Center current snapshot",
        ] {
            assert!(page.contains(label), "missing graph control: {label}");
        }
        assert!(page.contains("Accessible graph outline"));
        assert!(page.contains("recipe-version-graph__outline"));
        assert!(page.contains("tabindex: \"0\""));
    }

    #[test]
    fn graph_canvas_exposes_wheel_zoom_and_middle_mouse_panning() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("onwheel:"));
        assert!(page.contains("onmousedown:"));
        assert!(page.contains("onmousemove:"));
        assert!(page.contains("onmouseup:"));
        assert!(page.contains("MouseButton::Auxiliary"));
        assert!(page.contains("Scroll to zoom"));
        assert!(page.contains("middle-click and drag to pan"));
    }

    #[test]
    fn graph_canvas_exposes_a_right_click_navigation_menu() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("oncontextmenu:"));
        assert!(page.contains("role: \"menu\""));
        assert!(page.contains("Zoom in here"));
        assert!(page.contains("Zoom out here"));
        assert!(page.contains("Center here"));
        assert!(page.contains("Center current snapshot"));
        assert!(page.contains("Fit graph to viewport"));
        assert!(page.contains("event.stop_propagation()"));
        assert!(page.contains("Key::ArrowDown"));
        assert!(page.contains("Key::ArrowUp"));
        assert!(page.contains("set_focus(true)"));
    }

    #[test]
    fn graph_cards_expose_snapshot_specific_context_actions() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("VersionGraphContextMenuTarget::Snapshot"));
        assert!(page.contains("View snapshot"));
        assert!(page.contains("View recipe"));
        assert!(page.contains("Focus recipe graph"));
        assert!(page.contains("Center snapshot"));
        assert!(page.contains("Zoom to snapshot"));
        assert!(page.contains("Snapshot navigation unavailable"));
        assert!(page.contains("event.prevent_default()"));
        assert!(page.contains("event.stop_propagation()"));
    }

    #[test]
    fn graph_controls_and_outline_remain_available_on_narrow_viewports() {
        let css = include_str!("../../assets/main.css");

        assert!(css.contains(".recipe-version-graph__toolbar"));
        assert!(css.contains(".recipe-version-graph__canvas-viewport"));
        assert!(css.contains(".recipe-version-graph__canvas-viewport--panning"));
        assert!(css.contains(".recipe-version-graph__context-menu"));
        assert!(css.contains(".recipe-version-graph__context-menu-item"));
        assert!(css.contains("cursor: grab"));
        assert!(css.contains("cursor: grabbing"));
        assert!(css.contains(".recipe-version-graph__outline"));
        assert!(css.contains("flex-wrap: wrap"));
        assert!(css.contains("@media (max-width: 760px)"));
    }

    #[test]
    fn graph_border_color_is_defined_for_light_and_dark_themes() {
        let css = include_str!("../../assets/main.css");

        assert_eq!(css.matches("--border-color: rgba(").count(), 2);
    }
}
