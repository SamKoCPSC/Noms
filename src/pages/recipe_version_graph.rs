//! Dedicated shell for the recipe version graph.
//!
//! The page renders a bounded connected lineage family. Graph discovery and
//! presentation layout remain separate so each can evolve independently.

use dioxus::html::{geometry::WheelDelta, input_data::MouseButton};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::api::recipe::{
    compare_version_graph_snapshots_api, expand_recipe_version_graph, get_recipe_version_graph,
};
use crate::components::base::{Card, EditedRecipeImage, EmptyState, LoadingSpinner, PageHeader};
use crate::types::{
    RecipeVersionChange, VersionGraph, VersionGraphComparison, VersionGraphEdge,
    VersionGraphEdgeKind, VersionGraphFrontier, VersionGraphFrontierKind, VersionGraphLineageCount,
    VersionGraphNode, VersionGraphNodeId,
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

fn format_comparison_path(path: &str) -> String {
    let mut labels = Vec::<String>::new();
    for token in path.split('/').filter(|token| !token.is_empty()) {
        let token = token.replace("~1", "/").replace("~0", "~");
        if let Ok(index) = token.parse::<usize>() {
            if let Some(label) = labels.last_mut() {
                label.push_str(&format!(" #{}", index + 1));
            }
            continue;
        }
        let words = match token.as_str() {
            "prep_time_minutes" => "Prep time".to_string(),
            "cook_time_minutes" => "Cook time".to_string(),
            "instructions" => "Steps".to_string(),
            "sub_steps" => "Sub-steps".to_string(),
            _ => token.replace('_', " "),
        };
        let mut characters = words.chars();
        labels.push(match characters.next() {
            Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
            None => String::new(),
        });
    }
    if labels.is_empty() {
        "Recipe".to_string()
    } else {
        labels.join(" · ")
    }
}

fn comparison_value(value: Option<&String>) -> &str {
    value.map(String::as_str).unwrap_or("—")
}

#[component]
fn VersionGraphComparisonPanel(comparison: VersionGraphComparison, pair_label: String) -> Element {
    rsx! {
        section { class: "recipe-version-graph__comparison-result", aria_label: "Snapshot comparison result",
            h3 { "Comparison result" }
            p { class: "recipe-version-graph__comparison-pair", "{pair_label}" }
            if comparison.changes.is_empty() {
                p { class: "recipe-version-graph__comparison-empty", "No content differences between these snapshots." }
            } else {
                div { class: "recipe-version-graph__comparison-changes",
                    for change in &comparison.changes {
                        VersionGraphComparisonChange { change: change.clone() }
                    }
                }
            }
        }
    }
}

#[component]
fn VersionGraphComparisonChange(change: RecipeVersionChange) -> Element {
    let field = format_comparison_path(&change.path);
    let operation = match change.op.as_str() {
        "add" => "Added",
        "remove" => "Removed",
        _ => "Changed",
    };
    rsx! {
        article { class: "recipe-version-graph__comparison-change",
            header {
                div {
                    strong { "{field}" }
                    code { class: "recipe-version-graph__comparison-path", "{change.path}" }
                }
                span { "{operation}" }
            }
            div { class: "recipe-version-graph__comparison-values",
                div { class: "recipe-version-graph__comparison-value recipe-version-graph__comparison-value--before",
                    strong { "Before" }
                    pre { "{comparison_value(change.old_value.as_ref())}" }
                }
                div { class: "recipe-version-graph__comparison-value recipe-version-graph__comparison-value--after",
                    strong { "After" }
                    pre { "{comparison_value(change.value.as_ref())}" }
                }
            }
        }
    }
}

const GRAPH_NODE_WIDTH_PX: f64 = 304.0;
const GRAPH_NODE_HEIGHT_PX: f64 = 228.0;
const GRAPH_COLUMN_GAP_PX: f64 = 216.0;
const GRAPH_ROW_GAP_PX: f64 = 56.0;
const GRAPH_LAYOUT_PADDING_PX: f64 = 32.0;
const GRAPH_MIN_ZOOM: f64 = 0.001;
const GRAPH_MAX_ZOOM: f64 = 1.8;
const GRAPH_ZOOM_FACTOR: f64 = 1.2;
const GRAPH_WHEEL_ZOOM_SENSITIVITY: f64 = 0.002;
const GRAPH_WHEEL_LINE_HEIGHT_PX: f64 = 40.0;
const GRAPH_PAN_STEP_PX: f64 = 96.0;
const GRAPH_DEFAULT_VIEWPORT: (f64, f64) = (900.0, 560.0);
const GRAPH_CONTEXT_MENU_WIDTH_PX: f64 = 256.0;
const GRAPH_CONTEXT_MENU_HEIGHT_PX: f64 = 360.0;
// The menu scrolls when its expanded action set exceeds this positioning box.
const GRAPH_SNAPSHOT_CONTEXT_MENU_HEIGHT_PX: f64 = 460.0;
const GRAPH_CONTEXT_MENU_MARGIN_PX: f64 = 8.0;
const GRAPH_SNAPSHOT_FOCUS_ZOOM: f64 = 1.0;
const GRAPH_FRONTIER_CONTROL_GAP_PX: f64 = 12.0;
const GRAPH_FRONTIER_CONTROL_HALF_WIDTH_PX: f64 = 132.0;
const GRAPH_FRONTIER_CONTROL_HALF_HEIGHT_PX: f64 = 22.0;
#[cfg(target_arch = "wasm32")]
const GRAPH_TOUCH_CONTEXT_MENU_DELAY_MS: u32 = 550;
const GRAPH_TOUCH_CONTEXT_MENU_MOVE_TOLERANCE_PX: f64 = 12.0;

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct VersionGraphTouchContextCandidate {
    node_id: VersionGraphNodeId,
    token: u64,
    origin: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VersionGraphFrontierId {
    anchor: VersionGraphNodeId,
    kind: VersionGraphFrontierKind,
}

impl From<VersionGraphFrontier> for VersionGraphFrontierId {
    fn from(frontier: VersionGraphFrontier) -> Self {
        Self {
            anchor: frontier.anchor,
            kind: frontier.kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct VersionGraphExpansionRecord {
    frontier: VersionGraphFrontier,
    parent: Option<VersionGraphFrontierId>,
    batch: Option<VersionGraph>,
    is_expanded: bool,
    is_loading: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionGraphRecipeRelationship {
    MainRecipe,
    Ancestor,
    ImmediateDescendant,
    Unrelated,
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

fn merge_version_graph_batches<'a>(
    initial: &VersionGraph,
    batches: impl IntoIterator<Item = &'a VersionGraph>,
) -> VersionGraph {
    let mut merged = initial.clone();
    let mut node_ids = merged
        .nodes
        .iter()
        .map(VersionGraphNode::id)
        .collect::<HashSet<_>>();
    let mut edge_ids = merged
        .edges
        .iter()
        .map(|edge| version_graph_edge_key(edge.source(), edge.target(), edge.kind()))
        .collect::<HashSet<_>>();
    let mut frontier_indexes = merged
        .frontiers
        .iter()
        .copied()
        .enumerate()
        .map(|(index, frontier)| (VersionGraphFrontierId::from(frontier), index))
        .collect::<HashMap<_, _>>();

    for batch in batches {
        for node in &batch.nodes {
            if node_ids.insert(node.id()) {
                merged.nodes.push(node.clone());
            }
        }
        for edge in &batch.edges {
            if edge_ids.insert(version_graph_edge_key(
                edge.source(),
                edge.target(),
                edge.kind(),
            )) {
                merged.edges.push(edge.clone());
            }
        }
        for frontier in &batch.frontiers {
            let frontier_id = VersionGraphFrontierId::from(*frontier);
            if let Some(index) = frontier_indexes.get(&frontier_id).copied() {
                merged.frontiers[index] = *frontier;
            } else {
                frontier_indexes.insert(frontier_id, merged.frontiers.len());
                merged.frontiers.push(*frontier);
            }
        }
    }

    merged
}

fn retain_cached_version_graph_dependencies(
    mut visible: VersionGraph,
    cached: &VersionGraph,
) -> VersionGraph {
    let mut visible_node_ids = visible
        .nodes
        .iter()
        .map(VersionGraphNode::id)
        .collect::<HashSet<_>>();
    let required_node_ids = visible
        .edges
        .iter()
        .flat_map(|edge| [edge.source(), edge.target()])
        .chain(visible.frontiers.iter().map(|frontier| frontier.anchor))
        .collect::<HashSet<_>>();
    for node in &cached.nodes {
        if required_node_ids.contains(&node.id()) && visible_node_ids.insert(node.id()) {
            visible.nodes.push(node.clone());
        }
    }
    visible
}

fn version_graph_expansion_loaded_node_ids(graph: &VersionGraph) -> Vec<VersionGraphNodeId> {
    graph.nodes.iter().map(VersionGraphNode::id).collect()
}

fn expansion_record_is_visible(
    record_id: VersionGraphFrontierId,
    records: &[VersionGraphExpansionRecord],
) -> bool {
    let mut next = Some(record_id);
    let mut visited = HashSet::new();
    while let Some(id) = next {
        if !visited.insert(id) {
            return false;
        }
        let Some(record) = records
            .iter()
            .find(|record| VersionGraphFrontierId::from(record.frontier) == id)
        else {
            return false;
        };
        if !record.is_expanded || record.batch.is_none() {
            return false;
        }
        next = record.parent;
    }
    true
}

fn expansion_record_source_is_visible(
    record: &VersionGraphExpansionRecord,
    records: &[VersionGraphExpansionRecord],
) -> bool {
    record
        .parent
        .is_none_or(|parent| expansion_record_is_visible(parent, records))
}

fn visible_expansion_frontiers(
    initial: &VersionGraph,
    records: &[VersionGraphExpansionRecord],
) -> Vec<(VersionGraphFrontier, Option<VersionGraphFrontierId>)> {
    let mut frontiers = Vec::new();
    let mut frontier_indexes = HashMap::new();
    for frontier in &initial.frontiers {
        let frontier_id = VersionGraphFrontierId::from(*frontier);
        if let std::collections::hash_map::Entry::Vacant(entry) =
            frontier_indexes.entry(frontier_id)
        {
            entry.insert(frontiers.len());
            frontiers.push((*frontier, None));
        }
    }
    for record in records {
        let record_id = VersionGraphFrontierId::from(record.frontier);
        if !expansion_record_is_visible(record_id, records) {
            continue;
        }
        if let Some(batch) = &record.batch {
            for frontier in &batch.frontiers {
                let frontier_id = VersionGraphFrontierId::from(*frontier);
                let parent = if frontier_id == record_id {
                    record.parent
                } else {
                    Some(record_id)
                };
                if let Some(index) = frontier_indexes.get(&frontier_id).copied() {
                    frontiers[index] = (*frontier, parent);
                } else {
                    frontier_indexes.insert(frontier_id, frontiers.len());
                    frontiers.push((*frontier, parent));
                }
            }
        }
    }
    frontiers
}

fn frontier_direction_label(kind: VersionGraphFrontierKind) -> &'static str {
    match kind {
        VersionGraphFrontierKind::OlderVersions => "older versions",
        VersionGraphFrontierKind::NewerVersions => "newer versions",
        VersionGraphFrontierKind::ParentFork => "parent fork",
        VersionGraphFrontierKind::ChildForks => "child forks",
    }
}

fn version_graph_frontier_control_scale(transform: VersionGraphTransform) -> f64 {
    transform.scale.min(1.0)
}

fn version_graph_anchor_intersects_viewport(
    anchor: VersionGraphLayoutNode,
    transform: VersionGraphTransform,
    viewport: (f64, f64),
) -> bool {
    let left = transform.offset_x + anchor.x * transform.scale;
    let top = transform.offset_y + anchor.y * transform.scale;
    let right = left + GRAPH_NODE_WIDTH_PX * transform.scale;
    let bottom = top + GRAPH_NODE_HEIGHT_PX * transform.scale;

    left < viewport.0 && right > 0.0 && top < viewport.1 && bottom > 0.0
}

fn version_graph_frontier_control_position(
    anchor: VersionGraphLayoutNode,
    transform: VersionGraphTransform,
    kind: VersionGraphFrontierKind,
) -> (f64, f64) {
    let control_scale = version_graph_frontier_control_scale(transform);
    let center_x = transform.offset_x + (anchor.x + GRAPH_NODE_WIDTH_PX / 2.0) * transform.scale;
    let center_y = transform.offset_y + (anchor.y + GRAPH_NODE_HEIGHT_PX / 2.0) * transform.scale;
    let horizontal_distance = GRAPH_NODE_WIDTH_PX / 2.0 * transform.scale
        + (GRAPH_FRONTIER_CONTROL_GAP_PX + GRAPH_FRONTIER_CONTROL_HALF_WIDTH_PX) * control_scale;
    let vertical_distance = GRAPH_NODE_HEIGHT_PX / 2.0 * transform.scale
        + (GRAPH_FRONTIER_CONTROL_GAP_PX + GRAPH_FRONTIER_CONTROL_HALF_HEIGHT_PX) * control_scale;

    match kind {
        VersionGraphFrontierKind::OlderVersions => (center_x, center_y - vertical_distance),
        VersionGraphFrontierKind::NewerVersions => (center_x + horizontal_distance, center_y),
        VersionGraphFrontierKind::ParentFork => (center_x - horizontal_distance, center_y),
        VersionGraphFrontierKind::ChildForks => (center_x, center_y + vertical_distance),
    }
}

fn version_graph_frontier_port_class(kind: VersionGraphFrontierKind) -> &'static str {
    match kind {
        VersionGraphFrontierKind::OlderVersions => "recipe-version-graph__frontier-control--older",
        VersionGraphFrontierKind::NewerVersions => "recipe-version-graph__frontier-control--newer",
        VersionGraphFrontierKind::ParentFork => "recipe-version-graph__frontier-control--parent",
        VersionGraphFrontierKind::ChildForks => "recipe-version-graph__frontier-control--children",
    }
}

fn clamp_version_graph_frontier_control_position(
    position: (f64, f64),
    viewport: (f64, f64),
    control_scale: f64,
) -> (f64, f64) {
    let minimum_x =
        GRAPH_FRONTIER_CONTROL_HALF_WIDTH_PX * control_scale + GRAPH_CONTEXT_MENU_MARGIN_PX;
    let maximum_x = (viewport.0 - minimum_x).max(minimum_x);
    let minimum_y =
        GRAPH_FRONTIER_CONTROL_HALF_HEIGHT_PX * control_scale + GRAPH_CONTEXT_MENU_MARGIN_PX;
    let maximum_y = (viewport.1 - minimum_y).max(minimum_y);
    (
        position.0.clamp(minimum_x, maximum_x),
        position.1.clamp(minimum_y, maximum_y),
    )
}

fn begin_version_graph_expansion(
    recipe_id: String,
    frontier: VersionGraphFrontier,
    parent: Option<VersionGraphFrontierId>,
    loaded_node_ids: Vec<VersionGraphNodeId>,
    mut records: Signal<Vec<VersionGraphExpansionRecord>>,
    active_recipe_id: Signal<String>,
    continue_expansion: bool,
) {
    let frontier_id = VersionGraphFrontierId::from(frontier);
    let mut should_fetch = false;
    records.with_mut(|records| {
        if let Some(record) = records
            .iter_mut()
            .find(|record| VersionGraphFrontierId::from(record.frontier) == frontier_id)
        {
            if record.is_loading {
                return;
            }
            record.parent = parent;
            record.error = None;
            if continue_expansion {
                record.is_loading = true;
                should_fetch = true;
            } else if record.batch.is_some() {
                record.is_expanded = true;
            } else {
                record.is_loading = true;
                should_fetch = true;
            }
        } else {
            records.push(VersionGraphExpansionRecord {
                frontier,
                parent,
                batch: None,
                is_expanded: false,
                is_loading: true,
                error: None,
            });
            should_fetch = true;
        }
    });
    if !should_fetch {
        return;
    }

    spawn(async move {
        let requested_recipe_id = recipe_id.clone();
        let result = expand_recipe_version_graph(recipe_id, frontier, loaded_node_ids).await;
        if active_recipe_id() != requested_recipe_id {
            return;
        }
        records.with_mut(|records| {
            let Some(record) = records
                .iter_mut()
                .find(|record| VersionGraphFrontierId::from(record.frontier) == frontier_id)
            else {
                return;
            };
            record.is_loading = false;
            match result {
                Ok(batch) => {
                    let mut merged_batch = record.batch.as_ref().map_or_else(
                        || batch.clone(),
                        |cached| merge_version_graph_batches(cached, [&batch]),
                    );
                    // The response describes every boundary after this page.
                    // Replacing this list removes a fully consumed frontier.
                    merged_batch.frontiers = batch.frontiers;
                    record.batch = Some(merged_batch);
                    record.is_expanded = true;
                    record.error = None;
                }
                Err(error) => {
                    if !continue_expansion {
                        record.is_expanded = false;
                    }
                    record.error = Some(server_error_message(&error));
                }
            }
        });
    });
}

fn collapse_version_graph_expansion(
    frontier: VersionGraphFrontier,
    mut records: Signal<Vec<VersionGraphExpansionRecord>>,
) {
    let frontier_id = VersionGraphFrontierId::from(frontier);
    records.with_mut(|records| {
        if let Some(record) = records
            .iter_mut()
            .find(|record| VersionGraphFrontierId::from(record.frontier) == frontier_id)
        {
            record.is_expanded = false;
            record.error = None;
        }
    });
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

fn context_menu_typeahead(
    key: &str,
    current: usize,
    enabled: [bool; 8],
    snapshot: bool,
) -> Option<usize> {
    if key.chars().count() != 1 || key.trim().is_empty() {
        return None;
    }
    let labels = if snapshot {
        [
            "View snapshot",
            "View recipe",
            "Set as main recipe",
            "Select as Before",
            "Select as After",
            "Compare with current",
            "Center snapshot",
            "Zoom to snapshot",
        ]
    } else {
        [
            "Zoom in here",
            "Zoom out here",
            "Center here",
            "Center current snapshot",
            "Fit graph to viewport",
            "Show",
            "",
            "",
        ]
    };
    (1..=8).map(|offset| (current + offset) % 8).find(|&index| {
        enabled[index]
            && labels[index]
                .to_lowercase()
                .starts_with(&key.to_lowercase())
    })
}

fn next_context_menu_focus_index<const ACTION_COUNT: usize>(
    current: usize,
    direction: i32,
    enabled_actions: [bool; ACTION_COUNT],
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
    current.min(action_count.saturating_sub(1))
}

fn first_context_menu_focus_index<const ACTION_COUNT: usize>(
    enabled_actions: [bool; ACTION_COUNT],
) -> usize {
    enabled_actions
        .iter()
        .position(|enabled| *enabled)
        .unwrap_or(0)
}

fn snapshot_context_menu_actions_enabled(
    navigation_available: bool,
    current_snapshot_id: Option<VersionGraphNodeId>,
    snapshot_id: VersionGraphNodeId,
    comparison_is_loading: bool,
) -> [bool; 8] {
    [
        navigation_available,
        navigation_available,
        navigation_available,
        navigation_available && !comparison_is_loading,
        navigation_available && !comparison_is_loading,
        navigation_available
            && current_snapshot_id.is_some()
            && current_snapshot_id != Some(snapshot_id)
            && !comparison_is_loading,
        true,
        true,
    ]
}

async fn snapshot_context_menu_at_element(
    canvas: MountedEvent,
    anchor_element: MountedEvent,
    snapshot_id: VersionGraphNodeId,
) -> Option<VersionGraphContextMenu> {
    let canvas_rect = canvas.data().get_client_rect().await.ok()?;
    let anchor_rect = anchor_element.data().get_client_rect().await.ok()?;
    let viewport = (canvas_rect.size.width, canvas_rect.size.height);
    if viewport.0 <= 0.0 || viewport.1 <= 0.0 {
        return None;
    }

    // Place keyboard and touch menus immediately below their visible trigger.
    let anchor = (
        anchor_rect.origin.x - canvas_rect.origin.x,
        anchor_rect.origin.y + anchor_rect.size.height - canvas_rect.origin.y,
    );
    Some(VersionGraphContextMenu::at(
        anchor,
        viewport,
        VersionGraphContextMenuTarget::Snapshot(snapshot_id),
    ))
}

fn snapshot_context_menu_route(node_id: VersionGraphNodeId, action_index: usize) -> Option<Route> {
    let recipe_id = node_id.recipe_id.to_string();
    match action_index {
        0 => Some(Route::RecipeVersionDetail {
            id: recipe_id,
            version_number: node_id.version_number,
        }),
        1 => Some(Route::RecipeDetail { id: recipe_id }),
        _ => None,
    }
}

fn set_main_recipe_route(node_id: VersionGraphNodeId) -> Route {
    Route::RecipeVersionGraph {
        id: node_id.recipe_id.to_string(),
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

fn comparison_snapshot_label(
    id: VersionGraphNodeId,
    nodes: &HashMap<VersionGraphNodeId, VersionGraphNode>,
) -> String {
    nodes
        .get(&id)
        .and_then(VersionGraphNode::title)
        .map(|title| format!("{title} · v{}", id.version_number))
        .unwrap_or_else(|| format!("Snapshot v{}", id.version_number))
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

fn version_graph_lineage_only(graph: &VersionGraph) -> VersionGraph {
    let Some(main_recipe_id) = graph
        .nodes
        .iter()
        .find(|node| node.is_selected_recipe())
        .map(|node| node.id().recipe_id)
    else {
        return graph.clone();
    };
    let fork_recipe_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.kind() == VersionGraphEdgeKind::Fork)
        .map(|edge| (edge.source().recipe_id, edge.target().recipe_id))
        .collect::<Vec<_>>();

    let mut included_recipe_ids = HashSet::from([main_recipe_id]);
    let mut ancestors_to_visit = vec![main_recipe_id];
    let mut visited_ancestors = HashSet::new();
    while let Some(recipe_id) = ancestors_to_visit.pop() {
        if !visited_ancestors.insert(recipe_id) {
            continue;
        }
        for (source_recipe_id, target_recipe_id) in &fork_recipe_edges {
            if *target_recipe_id == recipe_id {
                included_recipe_ids.insert(*source_recipe_id);
                ancestors_to_visit.push(*source_recipe_id);
            }
        }
    }

    let mut descendants_to_visit = vec![main_recipe_id];
    let mut visited_descendants = HashSet::new();
    while let Some(recipe_id) = descendants_to_visit.pop() {
        if !visited_descendants.insert(recipe_id) {
            continue;
        }
        for (source_recipe_id, target_recipe_id) in &fork_recipe_edges {
            if *source_recipe_id == recipe_id {
                included_recipe_ids.insert(*target_recipe_id);
                descendants_to_visit.push(*target_recipe_id);
            }
        }
    }

    let nodes = graph
        .nodes
        .iter()
        .filter(|node| included_recipe_ids.contains(&node.id().recipe_id))
        .cloned()
        .collect::<Vec<_>>();
    let included_node_ids = nodes
        .iter()
        .map(VersionGraphNode::id)
        .collect::<HashSet<_>>();
    let edges = graph
        .edges
        .iter()
        .filter(|edge| {
            included_node_ids.contains(&edge.source()) && included_node_ids.contains(&edge.target())
        })
        .cloned()
        .collect();

    VersionGraph {
        nodes,
        edges,
        frontiers: graph
            .frontiers
            .iter()
            .filter(|frontier| included_node_ids.contains(&frontier.anchor))
            .copied()
            .collect(),
        truncation: graph.truncation,
        lineage_counts: graph.lineage_counts,
    }
}

fn format_version_graph_lineage_count(count: VersionGraphLineageCount) -> String {
    format!("{}{}", count.count, if count.is_capped { "+" } else { "" })
}

fn version_graph_recipe_relationships(
    nodes: &[VersionGraphNode],
    edges: &[VersionGraphEdge],
) -> HashMap<VersionGraphNodeId, VersionGraphRecipeRelationship> {
    let mut relationships = nodes
        .iter()
        .map(|node| (node.id(), VersionGraphRecipeRelationship::Unrelated))
        .collect::<HashMap<_, _>>();
    let Some(main_recipe_id) = nodes
        .iter()
        .find(|node| node.is_selected_recipe())
        .map(|node| node.id().recipe_id)
    else {
        return relationships;
    };

    let fork_edges = edges
        .iter()
        .filter(|edge| edge.kind() == VersionGraphEdgeKind::Fork)
        .collect::<Vec<_>>();

    // Descendants intentionally stop after one hop and apply only to the exact
    // fork target snapshots whose source recipe is the selected recipe.
    for edge in &fork_edges {
        let source = edge.source();
        let target = edge.target();
        if source.recipe_id == main_recipe_id {
            relationships.insert(target, VersionGraphRecipeRelationship::ImmediateDescendant);
        }
    }

    // Walk backwards by recipe, but classify only the exact source endpoint of
    // each fork. Visiting recipes (rather than snapshots) lets every parent fork
    // participate while keeping malformed cycles finite.
    let mut recipes_to_visit = vec![main_recipe_id];
    let mut visited_recipes = HashSet::new();
    while let Some(recipe_id) = recipes_to_visit.pop() {
        if !visited_recipes.insert(recipe_id) {
            continue;
        }
        for edge in &fork_edges {
            let source = edge.source();
            if edge.target().recipe_id != recipe_id {
                continue;
            }
            relationships.insert(source, VersionGraphRecipeRelationship::Ancestor);
            if !visited_recipes.contains(&source.recipe_id) {
                recipes_to_visit.push(source.recipe_id);
            }
        }
    }

    // Every selected-recipe snapshot remains main even if malformed graph data
    // also presents one as the opposite side of a relationship edge.
    for node in nodes
        .iter()
        .filter(|node| node.id().recipe_id == main_recipe_id)
    {
        relationships.insert(node.id(), VersionGraphRecipeRelationship::MainRecipe);
    }
    relationships
}

fn version_graph_node_class(
    node: &VersionGraphNode,
    recipe_relationship: VersionGraphRecipeRelationship,
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
    let recipe_relationship = if node.is_selected_recipe() {
        VersionGraphRecipeRelationship::MainRecipe
    } else {
        recipe_relationship
    };
    match recipe_relationship {
        VersionGraphRecipeRelationship::MainRecipe => {
            classes.push("recipe-version-graph__node--main-recipe");
        }
        VersionGraphRecipeRelationship::Ancestor => {
            classes.push("recipe-version-graph__node--ancestor");
        }
        VersionGraphRecipeRelationship::ImmediateDescendant => {
            classes.push("recipe-version-graph__node--immediate-descendant");
        }
        VersionGraphRecipeRelationship::Unrelated => {}
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
    recipe_relationship: VersionGraphRecipeRelationship,
    is_fork_source: bool,
    is_fork_target: bool,
) -> Element {
    let version_number = node.id().version_number;
    let recipe_relationship = if node.is_selected_recipe() {
        VersionGraphRecipeRelationship::MainRecipe
    } else {
        recipe_relationship
    };
    let node_class =
        version_graph_node_class(&node, recipe_relationship, is_fork_source, is_fork_target);
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
    let recipe_relationship_role = match recipe_relationship {
        VersionGraphRecipeRelationship::MainRecipe => ", main recipe",
        VersionGraphRecipeRelationship::Ancestor => ", ancestor",
        VersionGraphRecipeRelationship::ImmediateDescendant => ", immediate descendant",
        VersionGraphRecipeRelationship::Unrelated => "",
    };

    rsx! {
        article {
            class: node_class,
            aria_label: format!(
                "Version {version_number}: {title}{relationship_role}{recipe_relationship_role}"
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
                        if recipe_relationship == VersionGraphRecipeRelationship::MainRecipe {
                            span {
                                class: "recipe-version-graph__main-recipe-badge",
                                "Main recipe"
                            }
                        } else if recipe_relationship == VersionGraphRecipeRelationship::Ancestor {
                            span {
                                class: "recipe-version-graph__relationship-badge recipe-version-graph__relationship-badge--ancestor",
                                "Ancestor"
                            }
                        } else if recipe_relationship == VersionGraphRecipeRelationship::ImmediateDescendant {
                            span {
                                class: "recipe-version-graph__relationship-badge recipe-version-graph__relationship-badge--descendant",
                                "Immediate descendant"
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
    let mut touch_context_candidate = use_signal(|| None::<VersionGraphTouchContextCandidate>);
    let mut touch_context_token = use_signal(|| 0u64);
    let mut touch_context_menu_opened = use_signal(|| None::<VersionGraphNodeId>);
    let mut context_zoom_in = use_signal(|| None::<MountedEvent>);
    let mut context_zoom_out = use_signal(|| None::<MountedEvent>);
    let mut context_center_here = use_signal(|| None::<MountedEvent>);
    let mut context_center_current = use_signal(|| None::<MountedEvent>);
    let mut context_fit = use_signal(|| None::<MountedEvent>);
    let mut context_lineage_toggle = use_signal(|| None::<MountedEvent>);
    let mut lineage_only = use_signal(|| false);
    let mut comparison_before = use_signal(|| None::<VersionGraphNodeId>);
    let mut comparison_after = use_signal(|| None::<VersionGraphNodeId>);
    let mut comparison_result = use_signal(|| None::<VersionGraphComparison>);
    let mut comparison_error = use_signal(|| None::<String>);
    let mut comparison_is_loading = use_signal(|| false);
    let mut context_select_before = use_signal(|| None::<MountedEvent>);
    let mut context_select_after = use_signal(|| None::<MountedEvent>);
    let mut context_compare_current = use_signal(|| None::<MountedEvent>);
    let mut expansion_records = use_signal(Vec::<VersionGraphExpansionRecord>::new);
    let mut expansion_recipe_id = use_signal(|| id.clone());

    use_effect(use_reactive((&id,), move |(recipe_id,)| {
        expansion_records.set(Vec::new());
        expansion_recipe_id.set(recipe_id);
        initial_graph_fit_complete.set(false);
        comparison_before.set(None);
        comparison_after.set(None);
        comparison_result.set(None);
        comparison_error.set(None);
    }));

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

    let initial_graph = match graph_result {
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

    if initial_graph.nodes.is_empty() {
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

    let expansion_records_snapshot = if expansion_recipe_id() == id {
        expansion_records.read().clone()
    } else {
        Vec::new()
    };
    let cached_graph = merge_version_graph_batches(
        &initial_graph,
        expansion_records_snapshot
            .iter()
            .filter_map(|record| record.batch.as_ref()),
    );
    let visible_expansion_graph = merge_version_graph_batches(
        &initial_graph,
        expansion_records_snapshot
            .iter()
            .filter(|record| {
                expansion_record_is_visible(record.frontier.into(), &expansion_records_snapshot)
            })
            .filter_map(|record| record.batch.as_ref()),
    );
    let full_graph =
        retain_cached_version_graph_dependencies(visible_expansion_graph.clone(), &cached_graph);
    let expansion_frontiers =
        visible_expansion_frontiers(&initial_graph, &expansion_records_snapshot);

    let heading_title = full_graph
        .nodes
        .iter()
        .find(|node| node.is_current_version())
        .and_then(VersionGraphNode::title)
        .map(|title| format!("Version graph for {title}"))
        .unwrap_or_else(|| "Recipe version graph".to_string());
    let graph = if lineage_only() {
        version_graph_lineage_only(&full_graph)
    } else {
        full_graph.clone()
    };
    let layout = build_version_graph_layout(&graph);
    let fork_edges = graph
        .edges
        .iter()
        .filter(|edge| edge.kind() == VersionGraphEdgeKind::Fork)
        .cloned()
        .collect::<Vec<_>>();
    let recipe_relationships = version_graph_recipe_relationships(&graph.nodes, &graph.edges);
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
    let loaded_node_ids = version_graph_expansion_loaded_node_ids(&visible_expansion_graph);
    let expansion_loading_count = expansion_records_snapshot
        .iter()
        .filter(|record| {
            record.is_loading
                && expansion_record_source_is_visible(record, &expansion_records_snapshot)
        })
        .count();
    let expansion_error = expansion_records_snapshot
        .iter()
        .filter(|record| expansion_record_source_is_visible(record, &expansion_records_snapshot))
        .find_map(|record| record.error.as_deref());
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
    let full_graph_for_lineage_toggle = full_graph.clone();
    let full_graph_for_toolbar_toggle = full_graph.clone();
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
        Some((snapshot_id, _, navigation_available)) => snapshot_context_menu_actions_enabled(
            *navigation_available,
            current_snapshot_id,
            *snapshot_id,
            comparison_is_loading(),
        ),
        None => [
            true,
            true,
            true,
            current_snapshot_id.is_some(),
            true,
            true,
            false,
            false,
        ],
    };
    let is_snapshot_context_menu = snapshot_context_menu.is_some();

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/recipe-version-graph.css") }
        div { class: "container recipe-version-graph-page",
            PageHeader {
                title: heading_title,
                action: Some(back_to_recipe),
            }
            div { class: "recipe-version-graph__shell",
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
                    ul {
                        class: "recipe-version-graph__lineage-counts",
                        aria_label: "Recipe lineage counts",
                        li { class: "recipe-version-graph__lineage-count",
                            strong { {format_version_graph_lineage_count(graph.lineage_counts.ancestors)} }
                            span { "Ancestors" }
                        }
                        li { class: "recipe-version-graph__lineage-count",
                            strong { {format_version_graph_lineage_count(graph.lineage_counts.immediate_descendants)} }
                            span { "Forks" }
                        }
                        li { class: "recipe-version-graph__lineage-count",
                            strong { {format_version_graph_lineage_count(graph.lineage_counts.descendants)} }
                            span { "Descendants" }
                        }
                    }
                    div {
                        class: "recipe-version-graph__toolbar",
                        aria_label: "Version graph navigation",
                        div { class: "recipe-version-graph__controls-group",
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
                                label { class: "recipe-version-graph__lineage-toggle",
                                    input {
                                        r#type: "checkbox",
                                        checked: lineage_only(),
                                        onchange: move |_| {
                                            let next = !lineage_only();
                                            let graph = if next { version_graph_lineage_only(&full_graph_for_toolbar_toggle) } else { full_graph_for_toolbar_toggle.clone() };
                                            lineage_only.set(next);
                                            graph_transform.set(build_version_graph_layout(&graph).fit_transform(graph_viewport()));
                                        },
                                    }
                                    "Lineage only"
                                }
                            }
                            ul {
                                class: "recipe-version-graph__mouse-controls",
                                aria_label: "Mouse controls",
                                li { class: "recipe-version-graph__mouse-control",
                                    kbd { "Scroll wheel" }
                                    span { "Zoom in and out" }
                                }
                                li { class: "recipe-version-graph__mouse-control",
                                    kbd { "Middle-click + drag" }
                                    span { "Pan around the graph" }
                                }
                                li { class: "recipe-version-graph__mouse-control",
                                    kbd { "Right-click" }
                                    span { "Open more options" }
                                }
                            }
                        }
                        div {
                            class: "recipe-version-graph__legend",
                            role: "note",
                            aria_label: "Version graph legend",
                            span { class: "recipe-version-graph__legend-item",
                                span {
                                    class: "recipe-version-graph__legend-swatch recipe-version-graph__legend-swatch--main-recipe",
                                    aria_hidden: "true",
                                }
                                "Main recipe"
                            }
                            span { class: "recipe-version-graph__legend-item",
                                span {
                                    class: "recipe-version-graph__legend-swatch recipe-version-graph__legend-swatch--ancestor",
                                    aria_hidden: "true",
                                }
                                "Ancestor snapshot"
                            }
                            span { class: "recipe-version-graph__legend-item",
                                span {
                                    class: "recipe-version-graph__legend-swatch recipe-version-graph__legend-swatch--descendant",
                                    aria_hidden: "true",
                                }
                                "Immediate descendant snapshot"
                            }
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
                            id: "recipe-version-graph-canvas-stage",
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
                                            let recipe_relationship = recipe_relationships
                                                .get(&node_id)
                                                .copied()
                                                .unwrap_or(VersionGraphRecipeRelationship::Unrelated);
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
                                                    "data-before": (comparison_before() == Some(node_id)).to_string(),
                                                    "data-after": (comparison_after() == Some(node_id)).to_string(),
                                                    style: node_style,
                                                    onkeydown: move |event| {
                                                        let shift_is_pressed =
                                                            event.data().modifiers().shift();
                                                        let opens_context_menu =
                                                            event.data().key() == Key::ContextMenu
                                                                || (event.data().key() == Key::F10
                                                                    && shift_is_pressed);
                                                        if !opens_context_menu {
                                                            return;
                                                        }
                                                        event.prevent_default();
                                                        event.stop_propagation();
                                                        let Some(canvas) = graph_canvas() else {
                                                            return;
                                                        };
                                                        let Some(anchor_element) = graph_node_elements
                                                            .read()
                                                            .get(&node_id)
                                                            .cloned()
                                                        else {
                                                            return;
                                                        };
                                                        spawn(async move {
                                                            let Some(menu) = snapshot_context_menu_at_element(
                                                                canvas,
                                                                anchor_element,
                                                                node_id,
                                                            )
                                                            .await
                                                            else {
                                                                return;
                                                            };
                                                            let enabled_actions =
                                                                snapshot_context_menu_actions_enabled(
                                                                    snapshot_navigation_available,
                                                                    current_snapshot_id,
                                                                    node_id,
                                                                    comparison_is_loading(),
                                                                );
                                                            graph_context_menu_focus_index.set(
                                                                first_context_menu_focus_index(
                                                                    enabled_actions,
                                                                ),
                                                            );
                                                            graph_context_menu.set(Some(menu));
                                                        });
                                                    },
                                                    ontouchstart: move |event| {
                                                        let touches = event.data().touches();
                                                        let Some(touch) = touches.first() else {
                                                            return;
                                                        };
                                                        let position = touch.client_coordinates();
                                                        let token = touch_context_token()
                                                            .wrapping_add(1);
                                                        touch_context_token.set(token);
                                                        let candidate =
                                                            VersionGraphTouchContextCandidate {
                                                                node_id,
                                                                token,
                                                                origin: (position.x, position.y),
                                                            };
                                                        touch_context_candidate.set(Some(candidate));
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            let Some(canvas) = graph_canvas() else {
                                                                return;
                                                            };
                                                            let Some(anchor_element) = graph_node_elements
                                                                .read()
                                                                .get(&node_id)
                                                                .cloned()
                                                            else {
                                                                return;
                                                            };
                                                            spawn(async move {
                                                                gloo_timers::future::TimeoutFuture::new(
                                                                    GRAPH_TOUCH_CONTEXT_MENU_DELAY_MS,
                                                                )
                                                                .await;
                                                                if touch_context_candidate()
                                                                    != Some(candidate)
                                                                {
                                                                    return;
                                                                }
                                                                let Some(menu) = snapshot_context_menu_at_element(
                                                                    canvas,
                                                                    anchor_element,
                                                                    node_id,
                                                                )
                                                                .await
                                                                else {
                                                                    return;
                                                                };
                                                                // Geometry lookup is asynchronous. The
                                                                // finger may have moved or lifted while it
                                                                // was in flight, so validate the gesture a
                                                                // second time before opening anything.
                                                                if touch_context_candidate()
                                                                    != Some(candidate)
                                                                {
                                                                    return;
                                                                }
                                                                let enabled_actions =
                                                                    snapshot_context_menu_actions_enabled(
                                                                        snapshot_navigation_available,
                                                                        current_snapshot_id,
                                                                        node_id,
                                                                        comparison_is_loading(),
                                                                    );
                                                                graph_context_menu_focus_index.set(
                                                                    first_context_menu_focus_index(
                                                                        enabled_actions,
                                                                    ),
                                                                );
                                                                graph_context_menu.set(Some(menu));
                                                                touch_context_menu_opened.set(Some(node_id));
                                                            });
                                                        }
                                                    },
                                                    ontouchmove: move |event| {
                                                        let Some(candidate) = touch_context_candidate()
                                                        else {
                                                            return;
                                                        };
                                                        if candidate.node_id != node_id {
                                                            return;
                                                        }
                                                        let touches = event.data().touches();
                                                        let Some(touch) = touches.first() else {
                                                            touch_context_candidate.set(None);
                                                            return;
                                                        };
                                                        let position = touch.client_coordinates();
                                                        let distance = (position.x - candidate.origin.0)
                                                            .hypot(position.y - candidate.origin.1);
                                                        if distance
                                                            > GRAPH_TOUCH_CONTEXT_MENU_MOVE_TOLERANCE_PX
                                                        {
                                                            touch_context_candidate.set(None);
                                                        }
                                                    },
                                                    ontouchend: move |event| {
                                                        touch_context_candidate.set(None);
                                                        if touch_context_menu_opened()
                                                            == Some(node_id)
                                                        {
                                                            event.prevent_default();
                                                            event.stop_propagation();
                                                            touch_context_menu_opened.set(None);
                                                        }
                                                    },
                                                    ontouchcancel: move |_| {
                                                        touch_context_candidate.set(None);
                                                        touch_context_menu_opened.set(None);
                                                    },
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
                                                            let enabled_actions =
                                                                snapshot_context_menu_actions_enabled(
                                                                    snapshot_navigation_available,
                                                                    current_snapshot_id,
                                                                    node_id,
                                                                    comparison_is_loading(),
                                                                );
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
                                                    button {
                                                        r#type: "button",
                                                        class: "recipe-version-graph__snapshot-menu",
                                                        aria_label: "Snapshot actions",
                                                        aria_haspopup: "menu",
                                                        aria_expanded: active_context_menu.is_some_and(|menu| menu.target == VersionGraphContextMenuTarget::Snapshot(node_id)),
                                                        onclick: move |event| {
                                                            event.stop_propagation();
                                                            let Some(canvas) = graph_canvas() else { return; };
                                                            let Some(anchor) = graph_node_elements.read().get(&node_id).cloned() else { return; };
                                                            spawn(async move {
                                                                if let Some(menu) = snapshot_context_menu_at_element(canvas, anchor, node_id).await {
                                                                    graph_context_menu_focus_index.set(first_context_menu_focus_index(
                                                                        snapshot_context_menu_actions_enabled(snapshot_navigation_available, current_snapshot_id, node_id, comparison_is_loading()),
                                                                    ));
                                                                    graph_context_menu.set(Some(menu));
                                                                }
                                                            });
                                                        },
                                                        "⋯"
                                                    }
                                                    if comparison_before() == Some(node_id) || comparison_after() == Some(node_id) {
                                                        div { class: "recipe-version-graph__selection-roles",
                                                            if comparison_before() == Some(node_id) { span { "Before" } }
                                                            if comparison_after() == Some(node_id) { span { "After" } }
                                                        }
                                                    }
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
                                                                recipe_relationship,
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
                                                                recipe_relationship,
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
                        div {
                            class: "recipe-version-graph__frontier-layer",
                            aria_label: "Graph expansion controls",
                            for (frontier, parent) in &expansion_frontiers {
                                {
                                    let frontier = *frontier;
                                    let parent = *parent;
                                    let frontier_id = VersionGraphFrontierId::from(frontier);
                                    let record = expansion_records_snapshot.iter().find(|record| {
                                        VersionGraphFrontierId::from(record.frontier) == frontier_id
                                    });
                                    let is_expanded = record.is_some_and(|_| {
                                        expansion_record_is_visible(
                                            frontier_id,
                                            &expansion_records_snapshot,
                                        )
                                    });
                                    let is_loading = record.is_some_and(|record| record.is_loading);
                                    let can_continue = is_expanded
                                        && !is_loading
                                        && record.is_some_and(|record| {
                                            record.batch.as_ref().is_some_and(|batch| {
                                                batch.frontiers.iter().any(|candidate| {
                                                    VersionGraphFrontierId::from(*candidate)
                                                        == frontier_id
                                                })
                                            })
                                        });
                                    let error = record.and_then(|record| record.error.as_deref());
                                    let direction = frontier_direction_label(frontier.kind);
                                    let anchor_title = nodes_by_id
                                        .get(&frontier.anchor)
                                        .and_then(|node| node.title())
                                        .unwrap_or("snapshot");
                                    let label = if is_expanded {
                                        format!(
                                            "Collapse {direction} from {anchor_title}, version {}",
                                            frontier.anchor.version_number,
                                        )
                                    } else {
                                        format!(
                                            "Expand {} {direction} from {anchor_title}, version {}",
                                            frontier.hidden_count,
                                            frontier.anchor.version_number,
                                        )
                                    };
                                    let button_text = if is_loading {
                                        "Loading…".to_string()
                                    } else if is_expanded {
                                        format!("− {direction}")
                                    } else {
                                        format!("+ {} {direction}", frontier.hidden_count)
                                    };
                                    let control_scale =
                                        version_graph_frontier_control_scale(transform);
                                    let viewport = graph_viewport();
                                    let control_position = layout
                                        .node(frontier.anchor)
                                        .copied()
                                        .filter(|node| {
                                            version_graph_anchor_intersects_viewport(
                                                *node, transform, viewport,
                                            )
                                        })
                                        .map(|node| {
                                            clamp_version_graph_frontier_control_position(
                                                version_graph_frontier_control_position(
                                                    node,
                                                    transform,
                                                    frontier.kind,
                                                ),
                                                viewport,
                                                control_scale,
                                            )
                                        });
                                    if let Some((left, top)) = control_position {
                                        let control_class = format!(
                                            "recipe-version-graph__frontier-control {}",
                                            version_graph_frontier_port_class(frontier.kind),
                                        );
                                        let expand_recipe_id = id.clone();
                                        let expand_loaded_node_ids = loaded_node_ids.clone();
                                        let continuation_recipe_id = id.clone();
                                        let continuation_loaded_node_ids = loaded_node_ids.clone();
                                        rsx! {
                                            div {
                                                key: "frontier-{frontier.anchor.recipe_id}-{frontier.anchor.version_number}-{frontier.kind:?}",
                                                class: control_class,
                                                style: "left: {left:.2}px; top: {top:.2}px; --recipe-version-graph-frontier-scale: {control_scale:.4};",
                                                onclick: move |event| event.stop_propagation(),
                                                span { class: "recipe-version-graph__frontier-item",
                                                    button {
                                                        r#type: "button",
                                                        class: "recipe-version-graph__frontier-button",
                                                        aria_label: label,
                                                        aria_controls: "recipe-version-graph-canvas-stage",
                                                        aria_expanded: is_expanded,
                                                        disabled: is_loading,
                                                        onclick: move |event| {
                                                            event.stop_propagation();
                                                            if is_expanded {
                                                                collapse_version_graph_expansion(
                                                                    frontier,
                                                                    expansion_records,
                                                                );
                                                            } else {
                                                                begin_version_graph_expansion(
                                                                    expand_recipe_id.clone(),
                                                                    frontier,
                                                                    parent,
                                                                    expand_loaded_node_ids.clone(),
                                                                    expansion_records,
                                                                    expansion_recipe_id,
                                                                    false,
                                                                );
                                                            }
                                                        },
                                                        if is_loading {
                                                            span {
                                                                class: "recipe-version-graph__frontier-spinner",
                                                                aria_hidden: "true",
                                                            }
                                                        }
                                                        span { "{button_text}" }
                                                    }
                                                    if can_continue {
                                                        button {
                                                            r#type: "button",
                                                            class: "recipe-version-graph__frontier-button recipe-version-graph__frontier-button--continue",
                                                            aria_label: format!(
                                                                "Show next 5 {direction} from {anchor_title}, version {}; {} remain",
                                                                frontier.anchor.version_number,
                                                                frontier.hidden_count,
                                                            ),
                                                            aria_controls: "recipe-version-graph-canvas-stage",
                                                            onclick: move |event| {
                                                                event.stop_propagation();
                                                                begin_version_graph_expansion(
                                                                    continuation_recipe_id.clone(),
                                                                    frontier,
                                                                    parent,
                                                                    continuation_loaded_node_ids.clone(),
                                                                    expansion_records,
                                                                    expansion_recipe_id,
                                                                    true,
                                                                );
                                                            },
                                                            "Show next 5"
                                                        }
                                                    }
                                                    if error.is_some() {
                                                        span {
                                                            class: "recipe-version-graph__frontier-error-marker",
                                                            aria_hidden: "true",
                                                            "!"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {}
                                    }
                                }
                            }
                        }
                        if let Some(context_menu) = active_context_menu {
                          crate::components::base::ActionMenuBoundary {
                            on_dismiss: move |_| graph_context_menu.set(None),
                            div {
                                class: "feedback-surface recipe-version-graph__context-menu",
                                role: "menu",
                                aria_label: if is_snapshot_context_menu {
                                    "Snapshot context menu"
                                } else {
                                    "Graph navigation menu"
                                },
                                style: "left: {context_menu.position.0:.2}px; top: {context_menu.position.1:.2}px; --context-menu-top: {context_menu.position.1:.2}px;",
                                onclick: move |event| event.stop_propagation(),
                                // Scroll the menu, not the graph canvas behind it.
                                onwheel: move |event| event.stop_propagation(),
                                onmousedown: move |event| event.stop_propagation(),
                                oncontextmenu: move |event| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                },
                                onkeydown: move |event| {
                                    let next_index = match event.data().key() {
                                        Key::Tab => {
                                            // Leave native sequential focus traversal intact. Outside
                                            // focus dismisses the menu through ActionMenuBoundary.
                                            return;
                                        }
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
                                        Key::Character(character) if !event.modifiers().ctrl() && !event.modifiers().alt() && !event.modifiers().meta() => {
                                            let Some(index) = context_menu_typeahead(
                                                &character, graph_context_menu_focus_index(),
                                                context_menu_actions_enabled, is_snapshot_context_menu,
                                            ) else { return; };
                                            index
                                        }
                                        _ => return,
                                    };
                                    event.prevent_default();
                                    event.stop_propagation();
                                    graph_context_menu_focus_index.set(next_index);
                                    let target = if is_snapshot_context_menu {
                                        match next_index {
                                            0 => context_zoom_in(),
                                            1 => context_zoom_out(),
                                            2 => context_center_here(),
                                            3 => context_select_before(),
                                            4 => context_select_after(),
                                            5 => context_compare_current(),
                                            6 => context_center_current(),
                                            _ => context_fit(),
                                        }
                                    } else {
                                        match next_index {
                                            0 => context_zoom_in(),
                                            1 => context_zoom_out(),
                                            2 => context_center_here(),
                                            3 => context_center_current(),
                                            4 => context_fit(),
                                            _ => context_lineage_toggle(),
                                        }
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
                                                let route = set_main_recipe_route(snapshot_id);
                                                graph_context_menu.set(None);
                                                restore_context_menu_focus(
                                                    context_menu.target,
                                                    graph_canvas(),
                                                    &graph_node_elements(),
                                                );
                                                let _ = navigator.push(route);
                                            }
                                        },
                                        "Set as main recipe"
                                    }
                                    div {
                                        class: "recipe-version-graph__context-menu-separator",
                                        role: "separator",
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        disabled: !snapshot_navigation_available || comparison_is_loading(),
                                        tabindex: if graph_context_menu_focus_index() == 3 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_select_before.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 3 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            comparison_before.set(Some(snapshot_id));
                                            comparison_result.set(None);
                                            comparison_error.set(None);
                                            graph_context_menu.set(None);
                                            restore_context_menu_focus(context_menu.target, graph_canvas(), &graph_node_elements());
                                        },
                                        "Select as Before"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        disabled: !snapshot_navigation_available || comparison_is_loading(),
                                        tabindex: if graph_context_menu_focus_index() == 4 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_select_after.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 4 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            comparison_after.set(Some(snapshot_id));
                                            comparison_result.set(None);
                                            comparison_error.set(None);
                                            graph_context_menu.set(None);
                                            restore_context_menu_focus(context_menu.target, graph_canvas(), &graph_node_elements());
                                        },
                                        "Select as After"
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        disabled: !snapshot_navigation_available || current_snapshot_id.is_none() || current_snapshot_id == Some(snapshot_id) || comparison_is_loading(),
                                        tabindex: if graph_context_menu_focus_index() == 5 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_compare_current.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 5 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            if let Some(current_snapshot_id) = current_snapshot_id {
                                                comparison_before.set(Some(snapshot_id));
                                                comparison_after.set(Some(current_snapshot_id));
                                                comparison_result.set(None);
                                                comparison_error.set(None);
                                                comparison_is_loading.set(true);
                                                spawn(async move {
                                                    match compare_version_graph_snapshots_api(
                                                        snapshot_id,
                                                        current_snapshot_id,
                                                    )
                                                    .await
                                                    {
                                                        Ok(comparison) => {
                                                            comparison_result.set(Some(comparison))
                                                        }
                                                        Err(error) => comparison_error.set(Some(
                                                            server_error_message(&error),
                                                        )),
                                                    }
                                                    comparison_is_loading.set(false);
                                                });
                                            }
                                            graph_context_menu.set(None);
                                            restore_context_menu_focus(context_menu.target, graph_canvas(), &graph_node_elements());
                                        },
                                        "Compare with current"
                                    }
                                    div {
                                        class: "recipe-version-graph__context-menu-separator",
                                        role: "separator",
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitem",
                                        tabindex: if graph_context_menu_focus_index() == 6 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_center_current.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 6 {
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
                                        tabindex: if graph_context_menu_focus_index() == 7 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_fit.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 7 {
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
                                    div {
                                        class: "recipe-version-graph__context-menu-separator",
                                        role: "separator",
                                    }
                                    button {
                                        r#type: "button",
                                        class: "recipe-version-graph__context-menu-item",
                                        role: "menuitemcheckbox",
                                        aria_checked: lineage_only(),
                                        tabindex: if graph_context_menu_focus_index() == 5 { "0" } else { "-1" },
                                        onmounted: move |event| {
                                            context_lineage_toggle.set(Some(event.clone()));
                                            if graph_context_menu_focus_index() == 5 {
                                                focus_mounted_element(Some(event));
                                            }
                                        },
                                        onclick: move |_| {
                                            let next_lineage_only = !lineage_only();
                                            let next_graph = if next_lineage_only {
                                                version_graph_lineage_only(
                                                    &full_graph_for_lineage_toggle,
                                                )
                                            } else {
                                                full_graph_for_lineage_toggle.clone()
                                            };
                                            let next_layout = build_version_graph_layout(&next_graph);
                                            lineage_only.set(next_lineage_only);
                                            graph_transform.set(
                                                next_layout.fit_transform(graph_viewport())
                                            );
                                            graph_context_menu.set(None);
                                            focus_mounted_element(graph_canvas());
                                        },
                                        if lineage_only() {
                                            "Show full graph"
                                        } else {
                                            "Show ancestor and descendant lineage"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    }
                    div {
                        class: "recipe-version-graph__expansion-live",
                        role: "status",
                        aria_live: "polite",
                        aria_atomic: "true",
                        if expansion_loading_count > 0 {
                            "Loading {expansion_loading_count} graph expansion(s)."
                        } else if expansion_error.is_none() {
                            "Graph expansions are ready."
                        }
                    }
                    if let Some(error) = expansion_error {
                        p {
                            class: "recipe-version-graph__expansion-error",
                            role: "alert",
                            "Unable to expand the version graph: {error}"
                        }
                    }
                    section { class: "recipe-version-graph__comparison", aria_label: "Compare graph snapshots",
                        div { class: "recipe-version-graph__comparison-heading",
                            div {
                                h2 { "Compare snapshots" }
                                p { "Choose snapshots in their card menus, then compare them in exact Before → After order." }
                            }
                            div { class: "recipe-version-graph__comparison-actions",
                                button {
                                    r#type: "button",
                                    class: "btn btn-secondary",
                                    disabled: comparison_before().is_none() || comparison_after().is_none() || comparison_is_loading(),
                                    onclick: move |_| {
                                        let before = comparison_before();
                                        comparison_before.set(comparison_after());
                                        comparison_after.set(before);
                                        comparison_result.set(None);
                                        comparison_error.set(None);
                                    },
                                    "Swap direction"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn btn-secondary",
                                    disabled: comparison_is_loading()
                                        || (comparison_before().is_none()
                                            && comparison_after().is_none()
                                            && comparison_result().is_none()),
                                    onclick: move |_| {
                                        comparison_before.set(None);
                                        comparison_after.set(None);
                                        comparison_result.set(None);
                                        comparison_error.set(None);
                                    },
                                    "Clear comparison"
                                }
                            }
                        }
                        div { class: "recipe-version-graph__comparison-selection", role: "status",
                            div {
                                span { "Before" }
                                strong {
                                    {comparison_before()
                                        .map(|snapshot| comparison_snapshot_label(snapshot, &nodes_by_id))
                                        .unwrap_or_else(|| "Not selected".to_string())}
                                }
                            }
                            span { class: "recipe-version-graph__comparison-arrow", aria_hidden: "true", "→" }
                            div {
                                span { "After" }
                                strong {
                                    {comparison_after()
                                        .map(|snapshot| comparison_snapshot_label(snapshot, &nodes_by_id))
                                        .unwrap_or_else(|| "Not selected".to_string())}
                                }
                            }
                        }
                        if comparison_before().is_none() || comparison_after().is_none() {
                            p { class: "recipe-version-graph__comparison-help", "Choose both Before and After snapshots to compare." }
                        } else if comparison_before() == comparison_after() {
                            p { class: "recipe-version-graph__comparison-help", "Choose two different snapshots to compare." }
                        }
                        button {
                            r#type: "button",
                            class: "btn btn-primary recipe-version-graph__comparison-submit",
                            disabled: comparison_before().is_none()
                                || comparison_after().is_none()
                                || comparison_before() == comparison_after()
                                || comparison_is_loading(),
                            onclick: move |_| {
                                let (Some(from), Some(to)) = (comparison_before(), comparison_after()) else {
                                    return;
                                };
                                comparison_is_loading.set(true);
                                comparison_error.set(None);
                                comparison_result.set(None);
                                spawn(async move {
                                    match compare_version_graph_snapshots_api(from, to).await {
                                        Ok(comparison) => comparison_result.set(Some(comparison)),
                                        Err(error) => comparison_error.set(Some(server_error_message(&error))),
                                    }
                                    comparison_is_loading.set(false);
                                });
                            },
                            if comparison_is_loading() {
                                "Comparing snapshots..."
                            } else if comparison_error().is_some() {
                                "Retry comparison"
                            } else {
                                "Compare selected snapshots"
                            }
                        }
                        if comparison_is_loading() {
                            div { class: "recipe-version-graph__comparison-status", role: "status",
                                LoadingSpinner {}
                                span { "Loading comparison..." }
                            }
                        }
                        if let Some(error) = comparison_error() {
                            p { class: "recipe-version-graph__comparison-error", role: "alert",
                                "Unable to compare these snapshots: {error}"
                            }
                        }
                    }
                    if let Some(comparison) = comparison_result() {
                        VersionGraphComparisonPanel {
                            pair_label: format!("{} → {}", comparison_snapshot_label(comparison.from, &nodes_by_id), comparison_snapshot_label(comparison.to, &nodes_by_id)),
                            comparison,
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
                                                for (frontier, parent) in expansion_frontiers
                                                    .iter()
                                                    .filter(|(frontier, _)| frontier.anchor == node_id)
                                                {
                                                    {
                                                        let frontier = *frontier;
                                                        let parent = *parent;
                                                        let frontier_id = VersionGraphFrontierId::from(frontier);
                                                        let record = expansion_records_snapshot.iter().find(|record| {
                                                            VersionGraphFrontierId::from(record.frontier) == frontier_id
                                                        });
                                                        let is_expanded = record.is_some_and(|_| {
                                                            expansion_record_is_visible(
                                                                frontier_id,
                                                                &expansion_records_snapshot,
                                                            )
                                                        });
                                                        let is_loading = record.is_some_and(|record| record.is_loading);
                                                        let can_continue = is_expanded
                                                            && !is_loading
                                                            && record.is_some_and(|record| {
                                                                record.batch.as_ref().is_some_and(|batch| {
                                                                    batch.frontiers.iter().any(|candidate| {
                                                                        VersionGraphFrontierId::from(*candidate)
                                                                            == frontier_id
                                                                    })
                                                                })
                                                            });
                                                        let direction = frontier_direction_label(frontier.kind);
                                                        let label = if is_expanded {
                                                            format!("Collapse {direction} from {title}, version {}", node_id.version_number)
                                                        } else {
                                                            format!(
                                                                "Expand {} {direction} from {title}, version {}",
                                                                frontier.hidden_count,
                                                                node_id.version_number,
                                                            )
                                                        };
                                                        let expand_recipe_id = id.clone();
                                                        let expand_loaded_node_ids = loaded_node_ids.clone();
                                                        let continuation_recipe_id = id.clone();
                                                        let continuation_loaded_node_ids = loaded_node_ids.clone();
                                                        rsx! {
                                                            button {
                                                                r#type: "button",
                                                                class: "recipe-version-graph__outline-frontier-button",
                                                                aria_label: label,
                                                                aria_controls: "recipe-version-graph-canvas-stage",
                                                                aria_expanded: is_expanded,
                                                                disabled: is_loading,
                                                                onclick: move |_| {
                                                                    if is_expanded {
                                                                        collapse_version_graph_expansion(
                                                                            frontier,
                                                                            expansion_records,
                                                                        );
                                                                    } else {
                                                                        begin_version_graph_expansion(
                                                                            expand_recipe_id.clone(),
                                                                            frontier,
                                                                            parent,
                                                                            expand_loaded_node_ids.clone(),
                                                                            expansion_records,
                                                                            expansion_recipe_id,
                                                                            false,
                                                                        );
                                                                    }
                                                                },
                                                                if is_loading {
                                                                    "Loading…"
                                                                } else if is_expanded {
                                                                    "Collapse {direction}"
                                                                } else {
                                                                    "Expand {frontier.hidden_count} {direction}"
                                                                }
                                                            }
                                                            if can_continue {
                                                                button {
                                                                    r#type: "button",
                                                                    class: "recipe-version-graph__outline-frontier-button recipe-version-graph__outline-frontier-button--continue",
                                                                    aria_label: format!(
                                                                        "Show next 5 {direction} from {title}, version {}; {} remain",
                                                                        node_id.version_number,
                                                                        frontier.hidden_count,
                                                                    ),
                                                                    aria_controls: "recipe-version-graph-canvas-stage",
                                                                    onclick: move |_| {
                                                                        begin_version_graph_expansion(
                                                                            continuation_recipe_id.clone(),
                                                                            frontier,
                                                                            parent,
                                                                            continuation_loaded_node_ids.clone(),
                                                                            expansion_records,
                                                                            expansion_recipe_id,
                                                                            true,
                                                                        );
                                                                    },
                                                                    "Show next 5 {direction}"
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
    #[test]
    fn comparison_change_keeps_exact_path_and_multiline_read_only_values() {
        use dioxus::prelude::*;
        let mut dom = VirtualDom::new(|| {
            rsx! {
                super::VersionGraphComparisonChange {
                    change: crate::types::RecipeVersionChange {
                        op: "replace".to_string(),
                        path: "/instructions/0/description".to_string(),
                        old_value: Some("Simmer.\nStir.".to_string()),
                        value: Some("Blend <gently>.".to_string()),
                    }
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("/instructions/0/description"));
        assert!(html.contains("Simmer.\nStir."));
        // Dioxus SSR uses numeric HTML entities for dynamic text.
        assert!(html.contains("Blend &#60;gently&#62;."));
        assert!(!html.contains("<gently>"));
        assert!(html.contains("Changed"));
        assert!(!html.contains("<input"));
        assert!(!html.contains("<textarea"));
    }

    #[test]
    fn balanced_card_css_and_layout_dimensions_stay_in_sync() {
        let stylesheet = include_str!("../../assets/recipe-version-graph.css");
        assert!(stylesheet.contains(&format!(
            "width: {}px; height: {}px;",
            super::GRAPH_NODE_WIDTH_PX,
            super::GRAPH_NODE_HEIGHT_PX
        )));
    }

    #[test]
    fn action_menu_typeahead_cycles_and_skips_unavailable_actions() {
        let mut enabled = [true; 8];
        assert_eq!(
            super::context_menu_typeahead("v", 0, enabled, true),
            Some(1)
        );
        assert_eq!(
            super::context_menu_typeahead("V", 1, enabled, true),
            Some(0)
        );
        enabled[5] = false;
        assert_eq!(
            super::context_menu_typeahead("c", 4, enabled, true),
            Some(6)
        );
        assert_eq!(super::context_menu_typeahead(" ", 0, enabled, true), None);
        assert_eq!(super::context_menu_typeahead("xx", 0, enabled, true), None);
        assert_eq!(
            super::context_menu_typeahead("z", 0, [false; 8], false),
            None
        );
        assert_eq!(
            super::context_menu_typeahead("z", 0, enabled, false),
            Some(1)
        );
    }
    use super::is_recipe_unavailable_error;
    use crate::types::{
        VersionGraph, VersionGraphEdge, VersionGraphEdgeKind, VersionGraphFrontier,
        VersionGraphFrontierKind, VersionGraphNode, VersionGraphNodeContent, VersionGraphNodeId,
        VersionGraphNodeState,
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
            frontiers: vec![],
            truncation: Default::default(),
            lineage_counts: Default::default(),
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

        let selected_class = super::version_graph_node_class(
            &selected,
            super::VersionGraphRecipeRelationship::MainRecipe,
            false,
            false,
        );
        let related_class = super::version_graph_node_class(
            &related,
            super::VersionGraphRecipeRelationship::Unrelated,
            false,
            false,
        );

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
        assert!(css
            .contains(".recipe-version-graph__node--main-recipe .recipe-version-graph__node-body"));
        assert!(css.contains("gap: 2px;"));
        assert!(
            css.contains(".recipe-version-graph__node--main-recipe .recipe-version-graph__summary")
        );
        assert!(css.contains("margin-top: 0 !important;"));
        assert!(css.contains(".recipe-version-graph__main-recipe-badge"));
    }

    #[test]
    fn fork_relationships_highlight_the_ancestor_chain_and_immediate_descendant_endpoints() {
        let grandancestor_recipe_id = Uuid::from_u128(6);
        let ancestor_recipe_id = Uuid::from_u128(1);
        let selected_recipe_id = Uuid::from_u128(2);
        let child_recipe_id = Uuid::from_u128(3);
        let grandchild_recipe_id = Uuid::from_u128(4);
        let unrelated_recipe_id = Uuid::from_u128(5);
        let nodes = vec![
            visible_node(grandancestor_recipe_id, 1, "Grandancestor v1"),
            visible_node(grandancestor_recipe_id, 2, "Grandancestor v2"),
            visible_node(ancestor_recipe_id, 1, "Ancestor v1"),
            visible_node(ancestor_recipe_id, 2, "Ancestor v2"),
            visible_node_with_selection(selected_recipe_id, 1, "Selected v1", true),
            visible_node_with_selection(selected_recipe_id, 2, "Selected v2", true),
            visible_node(child_recipe_id, 1, "Child v1"),
            visible_node(child_recipe_id, 2, "Child v2"),
            visible_node(grandchild_recipe_id, 1, "Grandchild v1"),
            visible_node(unrelated_recipe_id, 1, "Unrelated v1"),
        ];
        let edges = vec![
            VersionGraphEdge::fork(grandancestor_recipe_id, 1, ancestor_recipe_id).unwrap(),
            VersionGraphEdge::fork(ancestor_recipe_id, 2, selected_recipe_id).unwrap(),
            VersionGraphEdge::fork(selected_recipe_id, 1, child_recipe_id).unwrap(),
            VersionGraphEdge::fork(child_recipe_id, 2, grandchild_recipe_id).unwrap(),
        ];

        let relationships = super::version_graph_recipe_relationships(&nodes, &edges);

        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: grandancestor_recipe_id,
                version_number: 1,
            }),
            Some(&super::VersionGraphRecipeRelationship::Ancestor)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: grandancestor_recipe_id,
                version_number: 2,
            }),
            Some(&super::VersionGraphRecipeRelationship::Unrelated)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: ancestor_recipe_id,
                version_number: 1,
            }),
            Some(&super::VersionGraphRecipeRelationship::Unrelated)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: ancestor_recipe_id,
                version_number: 2,
            }),
            Some(&super::VersionGraphRecipeRelationship::Ancestor)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: selected_recipe_id,
                version_number: 1,
            }),
            Some(&super::VersionGraphRecipeRelationship::MainRecipe)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: selected_recipe_id,
                version_number: 2,
            }),
            Some(&super::VersionGraphRecipeRelationship::MainRecipe)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: child_recipe_id,
                version_number: 1,
            }),
            Some(&super::VersionGraphRecipeRelationship::ImmediateDescendant)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: child_recipe_id,
                version_number: 2,
            }),
            Some(&super::VersionGraphRecipeRelationship::Unrelated)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: grandchild_recipe_id,
                version_number: 1,
            }),
            Some(&super::VersionGraphRecipeRelationship::Unrelated)
        );
        assert_eq!(
            relationships.get(&VersionGraphNodeId {
                recipe_id: unrelated_recipe_id,
                version_number: 1,
            }),
            Some(&super::VersionGraphRecipeRelationship::Unrelated)
        );
    }

    #[test]
    fn lineage_only_graph_keeps_ancestors_and_descendants_but_removes_side_branches() {
        let ancestor_recipe_id = Uuid::from_u128(101);
        let main_recipe_id = Uuid::from_u128(102);
        let descendant_recipe_id = Uuid::from_u128(103);
        let sibling_recipe_id = Uuid::from_u128(104);
        let cousin_recipe_id = Uuid::from_u128(105);
        let graph = VersionGraph {
            nodes: vec![
                visible_node(ancestor_recipe_id, 1, "Ancestor"),
                visible_node_with_selection(main_recipe_id, 1, "Main", true),
                visible_node(descendant_recipe_id, 1, "Descendant"),
                visible_node(sibling_recipe_id, 1, "Sibling"),
                visible_node(cousin_recipe_id, 1, "Cousin"),
            ],
            edges: vec![
                VersionGraphEdge::fork(ancestor_recipe_id, 1, main_recipe_id).unwrap(),
                VersionGraphEdge::fork(main_recipe_id, 1, descendant_recipe_id).unwrap(),
                VersionGraphEdge::fork(ancestor_recipe_id, 1, sibling_recipe_id).unwrap(),
                VersionGraphEdge::fork(sibling_recipe_id, 1, cousin_recipe_id).unwrap(),
            ],
            frontiers: vec![],
            truncation: crate::types::VersionGraphTruncation {
                is_truncated: true,
                depth_limit_reached: true,
                ..Default::default()
            },
            lineage_counts: Default::default(),
        };

        let lineage = super::version_graph_lineage_only(&graph);
        let recipe_ids = lineage
            .nodes
            .iter()
            .map(|node| node.id().recipe_id)
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(
            recipe_ids,
            [ancestor_recipe_id, main_recipe_id, descendant_recipe_id]
                .into_iter()
                .collect()
        );
        assert!(lineage.edges.iter().all(|edge| {
            recipe_ids.contains(&edge.source().recipe_id)
                && recipe_ids.contains(&edge.target().recipe_id)
        }));
        assert_eq!(lineage.truncation, graph.truncation);
        assert_eq!(lineage.lineage_counts, graph.lineage_counts);
    }

    #[test]
    fn graph_background_menu_toggles_the_lineage_only_view() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("Show ancestor and descendant lineage"));
        assert!(page.contains("Show full graph"));
        assert!(page.contains("role: \"menuitemcheckbox\""));
        assert!(page.contains("aria_checked: lineage_only"));
    }

    #[test]
    fn ancestor_and_descendant_cards_have_distinct_accessible_treatments() {
        let ancestor = visible_node(Uuid::from_u128(1), 1, "Ancestor");
        let descendant = visible_node(Uuid::from_u128(2), 1, "Descendant");

        let ancestor_class = super::version_graph_node_class(
            &ancestor,
            super::VersionGraphRecipeRelationship::Ancestor,
            false,
            false,
        );
        let descendant_class = super::version_graph_node_class(
            &descendant,
            super::VersionGraphRecipeRelationship::ImmediateDescendant,
            false,
            false,
        );

        assert!(ancestor_class.contains("recipe-version-graph__node--ancestor"));
        assert!(descendant_class.contains("recipe-version-graph__node--immediate-descendant"));

        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");
        let css = include_str!("../../assets/main.css");

        assert!(page.contains("Ancestor"));
        assert!(page.contains("Immediate descendant"));
        assert!(page.contains(", ancestor"));
        assert!(page.contains(", immediate descendant"));
        assert!(css.contains(".recipe-version-graph__node--ancestor"));
        assert!(css.contains(".recipe-version-graph__node--immediate-descendant"));
        assert!(css.contains(".recipe-version-graph__relationship-badge--ancestor"));
        assert!(css.contains(".recipe-version-graph__relationship-badge--descendant"));
        assert!(css.contains("@keyframes recipe-version-graph-ancestor-breathe"));
        assert!(css.contains("@keyframes recipe-version-graph-immediate-descendant-breathe"));
        assert!(css
            .contains("animation: recipe-version-graph-ancestor-breathe 2s ease-in-out infinite;"));
        assert!(css.contains(
            "animation: recipe-version-graph-immediate-descendant-breathe 2s ease-in-out infinite;"
        ));
        assert!(css.contains("var(--warning) 10%, var(--surface)"));
        assert!(css.contains("var(--warning) 34%, var(--surface)"));
        assert!(css.contains("#5573a8 8%, var(--surface)"));
        assert!(css.contains("#5573a8 32%, var(--surface)"));
        assert!(css.contains(".recipe-version-graph__node--ancestor,"));
        assert!(css.contains(".recipe-version-graph__node--immediate-descendant"));
        assert!(css.contains("animation: none;"));
    }

    #[test]
    fn graph_legend_explains_relationship_highlights_and_connections() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");
        let css = include_str!("../../assets/main.css");

        assert!(page.contains("aria_label: \"Version graph legend\""));
        assert!(page.contains("Main recipe"));
        assert!(page.contains("Ancestor snapshot"));
        assert!(page.contains("Immediate descendant snapshot"));
        assert!(page.contains("Version history"));
        assert!(page.contains("Fork"));
        assert!(css.contains(".recipe-version-graph__legend-swatch--main-recipe"));
        assert!(css.contains(".recipe-version-graph__legend-swatch--ancestor"));
        assert!(css.contains(".recipe-version-graph__legend-swatch--descendant"));
    }

    #[test]
    fn graph_header_displays_exact_and_capped_lineage_counts() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");
        let css = include_str!("../../assets/main.css");

        assert_eq!(
            super::format_version_graph_lineage_count(crate::types::VersionGraphLineageCount {
                count: 3,
                is_capped: false,
            }),
            "3"
        );
        assert_eq!(
            super::format_version_graph_lineage_count(crate::types::VersionGraphLineageCount {
                count: 50,
                is_capped: true,
            }),
            "50+"
        );
        assert!(page.contains("Ancestors"));
        assert!(page.contains("Forks"));
        assert!(page.contains("Descendants"));
        assert!(page.contains("recipe-version-graph__lineage-counts"));
        assert!(css.contains(".recipe-version-graph__lineage-counts"));
        assert!(css.contains(".recipe-version-graph__lineage-count"));
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

        let root_v1 = layout
            .node(VersionGraphNodeId {
                recipe_id: Uuid::from_u128(1),
                version_number: 1,
            })
            .unwrap();
        let root_v2 = layout
            .node(VersionGraphNodeId {
                recipe_id: Uuid::from_u128(1),
                version_number: 2,
            })
            .unwrap();
        assert!(
            root_v2.x - root_v1.x - super::GRAPH_NODE_WIDTH_PX >= 216.0,
            "cards in adjacent columns need expanded horizontal separation"
        );

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
        let all_snapshot_actions = [true; 8];
        let background_actions = [true; 6];
        let private_snapshot_actions = [false, false, false, false, false, false, true, true];
        let loading_snapshot_actions = [true, true, true, false, false, false, true, true];

        assert_eq!(
            super::next_context_menu_focus_index(0, 1, all_snapshot_actions),
            1
        );
        assert_eq!(
            super::next_context_menu_focus_index(7, 1, all_snapshot_actions),
            0
        );
        assert_eq!(
            super::next_context_menu_focus_index(0, -1, all_snapshot_actions),
            7
        );
        assert_eq!(
            super::next_context_menu_focus_index(5, 1, background_actions),
            0
        );
        assert_eq!(
            super::next_context_menu_focus_index(0, -1, background_actions),
            5
        );
        assert_eq!(
            super::first_context_menu_focus_index(private_snapshot_actions),
            6
        );
        assert_eq!(
            super::next_context_menu_focus_index(6, 1, private_snapshot_actions),
            7
        );
        assert_eq!(
            super::next_context_menu_focus_index(7, 1, private_snapshot_actions),
            6
        );
        assert_eq!(
            super::next_context_menu_focus_index(2, 1, loading_snapshot_actions),
            6
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
        // P2's wider shell and 44px targets reserve more space than the old menu.
        assert_eq!(menu.position, (636.0, 92.0));
        assert_eq!(menu.position.0 + super::GRAPH_CONTEXT_MENU_WIDTH_PX, 892.0);
        assert_eq!(
            menu.position.1 + super::GRAPH_SNAPSHOT_CONTEXT_MENU_HEIGHT_PX,
            552.0
        );
        let narrow =
            super::VersionGraphContextMenu::at((180.0, 150.0), (200.0, 180.0), menu.target);
        assert_eq!(narrow.position, (8.0, 8.0));
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
        assert!(super::snapshot_context_menu_route(node_id, 2).is_none());
        assert!(matches!(
            super::set_main_recipe_route(node_id),
            crate::Route::RecipeVersionGraph { id } if id == recipe_id.to_string()
        ));
    }

    #[test]
    fn setting_a_card_as_main_reloads_the_graph_from_that_recipe() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("Set as main recipe"));
        assert!(!page.contains("Focus recipe graph"));
        assert!(page.contains("set_main_recipe_route(snapshot_id)"));
        assert!(page.contains("navigator.push(route)"));
        assert!(page.contains("use_resource(use_reactive((&id,)"));
        assert!(matches!(
            super::set_main_recipe_route(VersionGraphNodeId {
                recipe_id: Uuid::from_u128(73),
                version_number: 4,
            }),
            crate::Route::RecipeVersionGraph { id } if id == Uuid::from_u128(73).to_string()
        ));
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
            frontiers: vec![],
            truncation: Default::default(),
            lineage_counts: Default::default(),
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
        let css = include_str!("../../assets/main.css");

        assert!(page.contains("onwheel:"));
        assert!(page.contains("onmousedown:"));
        assert!(page.contains("onmousemove:"));
        assert!(page.contains("onmouseup:"));
        assert!(page.contains("MouseButton::Auxiliary"));
        assert!(page.contains("Mouse controls"));
        assert!(page.contains("Scroll wheel"));
        assert!(page.contains("Zoom in and out"));
        assert!(page.contains("Middle-click + drag"));
        assert!(page.contains("Pan around the graph"));
        assert!(page.contains("Right-click"));
        assert!(page.contains("Open more options"));
        assert!(!page.contains("Press and hold a card"));
        assert!(page.contains("recipe-version-graph__mouse-controls"));
        assert!(page.contains("recipe-version-graph__mouse-control"));
        let manual_controls_position = page
            .find("class: \"recipe-version-graph__controls\"")
            .expect("manual graph controls");
        let mouse_guide_position = page
            .find("class: \"recipe-version-graph__mouse-controls\"")
            .expect("mouse controls guide");
        assert!(manual_controls_position < mouse_guide_position);
        assert!(css.contains(".recipe-version-graph__controls-group"));
        assert!(css.contains("font-size: 0.76rem"));
        assert!(css.contains("padding: 0.08rem 0.3rem"));
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
        assert!(page.contains("Set as main recipe"));
        assert!(page.contains("Center snapshot"));
        assert!(page.contains("Zoom to snapshot"));
        assert!(page.contains("Snapshot navigation unavailable"));
        assert!(page.contains("event.prevent_default()"));
        assert!(page.contains("event.stop_propagation()"));
    }

    #[test]
    fn graph_cards_expose_touch_and_keyboard_menus_without_inline_controls() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");
        assert!(!page.contains("recipe-version-graph__snapshot-actions"));
        assert!(!page.contains("Snapshot actions for"));
        assert!(!page.contains("Press and hold a card"));
        assert!(page.contains("Key::ContextMenu"));
        assert!(page.contains("Key::F10"));
        assert!(page.contains("modifiers().shift()"));
        assert!(page.contains("ontouchend:"));
        assert!(page.contains("ontouchstart:"));
        assert!(page.contains("ontouchmove:"));
        assert!(page.contains("ontouchcancel:"));
        assert!(page.contains("GRAPH_TOUCH_CONTEXT_MENU_DELAY_MS"));
        assert!(page.contains("GRAPH_TOUCH_CONTEXT_MENU_MOVE_TOLERANCE_PX"));
        assert!(!page.contains("GRAPH_SNAPSHOT_ACTION_MIN_VISIBLE_SCALE"));
    }

    #[test]
    fn graph_snapshots_support_directional_cross_recipe_comparisons() {
        let source = include_str!("recipe_version_graph.rs");
        let page = source
            .split("#[cfg(test)]")
            .next()
            .expect("version graph page implementation");

        assert!(page.contains("compare_version_graph_snapshots_api"));
        assert!(page.contains("Select as Before"));
        assert!(page.contains("Select as After"));
        assert!(page.contains("Compare with current"));
        assert!(page.contains("Compare selected snapshots"));
        assert!(page.contains("Swap direction"));
        assert!(page.contains("Clear comparison"));
        assert!(page.contains("role: \"status\""));
        assert!(page.contains("VersionGraphComparisonPanel"));
        assert!(page.contains("No content differences between these snapshots."));
        assert_eq!(
            page.matches("disabled: !snapshot_navigation_available || comparison_is_loading()")
                .count(),
            2
        );
        assert!(page.contains("disabled: comparison_is_loading()"));
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

    #[test]
    fn graph_expansion_batches_merge_without_duplicate_nodes_or_edges() {
        let recipe_id = Uuid::from_u128(700);
        let first = visible_node(recipe_id, 1, "First");
        let second = visible_node(recipe_id, 2, "Second");
        let edge = VersionGraphEdge::version(first.id(), second.id()).unwrap();
        let initial = VersionGraph {
            nodes: vec![first.clone()],
            frontiers: vec![],
            ..Default::default()
        };
        let batch = VersionGraph {
            nodes: vec![first, second.clone(), second],
            edges: vec![edge.clone(), edge],
            frontiers: vec![],
            ..Default::default()
        };

        let merged = super::merge_version_graph_batches(&initial, [&batch]);

        assert_eq!(merged.nodes.len(), 2);
        assert_eq!(merged.edges.len(), 1);
    }

    #[test]
    fn a_later_batch_replaces_stale_metadata_for_the_same_frontier() {
        let anchor = VersionGraphNodeId {
            recipe_id: Uuid::from_u128(705),
            version_number: 1,
        };
        let initial = VersionGraph {
            frontiers: vec![crate::types::VersionGraphFrontier {
                anchor,
                kind: crate::types::VersionGraphFrontierKind::ChildForks,
                hidden_count: 12,
                fork_cursor: None,
            }],
            ..Default::default()
        };
        let first_page = VersionGraph {
            frontiers: vec![crate::types::VersionGraphFrontier {
                anchor,
                kind: crate::types::VersionGraphFrontierKind::ChildForks,
                hidden_count: 7,
                fork_cursor: None,
            }],
            ..Default::default()
        };

        let merged = super::merge_version_graph_batches(&initial, [&first_page]);

        assert_eq!(merged.frontiers.len(), 1);
        assert_eq!(merged.frontiers[0].hidden_count, 7);
    }

    #[test]
    fn frontier_controls_use_distinct_directional_ports_around_their_anchor() {
        let anchor = super::VersionGraphLayoutNode {
            id: VersionGraphNodeId {
                recipe_id: Uuid::from_u128(706),
                version_number: 2,
            },
            x: 200.0,
            y: 150.0,
            rank: 1,
            row: 1,
        };
        let transform = super::VersionGraphTransform {
            scale: 0.25,
            offset_x: 20.0,
            offset_y: 30.0,
        };
        let older = super::version_graph_frontier_control_position(
            anchor,
            transform,
            crate::types::VersionGraphFrontierKind::OlderVersions,
        );
        let newer = super::version_graph_frontier_control_position(
            anchor,
            transform,
            crate::types::VersionGraphFrontierKind::NewerVersions,
        );
        let parent = super::version_graph_frontier_control_position(
            anchor,
            transform,
            crate::types::VersionGraphFrontierKind::ParentFork,
        );
        let children = super::version_graph_frontier_control_position(
            anchor,
            transform,
            crate::types::VersionGraphFrontierKind::ChildForks,
        );

        assert!(older.1 < parent.1 && older.1 < children.1);
        assert!(children.1 > parent.1 && children.1 > newer.1);
        assert!(parent.0 < older.0 && parent.0 < newer.0);
        assert!(newer.0 > older.0 && newer.0 > children.0);
    }

    #[test]
    fn frontier_controls_shrink_with_canvas_zoom_without_growing_past_normal_size() {
        let zoomed_out = super::VersionGraphTransform {
            scale: 0.08,
            offset_x: 0.0,
            offset_y: 0.0,
        };
        let zoomed_in = super::VersionGraphTransform {
            scale: 1.4,
            offset_x: 0.0,
            offset_y: 0.0,
        };

        assert_eq!(
            super::version_graph_frontier_control_scale(zoomed_out),
            0.08
        );
        assert_eq!(super::version_graph_frontier_control_scale(zoomed_in), 1.0);
    }

    #[test]
    fn frontier_controls_are_hidden_when_their_anchor_is_fully_outside_the_viewport() {
        let viewport = (800.0, 600.0);
        let scale = 0.5;
        let anchor_x = 100.0;
        let anchor_y = 100.0;
        for (index, (offset_x, offset_y)) in [
            (-(anchor_x + super::GRAPH_NODE_WIDTH_PX) * scale, 0.0),
            (viewport.0 - anchor_x * scale, 0.0),
            (0.0, -(anchor_y + super::GRAPH_NODE_HEIGHT_PX) * scale),
            (0.0, viewport.1 - anchor_y * scale),
        ]
        .into_iter()
        .enumerate()
        {
            let fully_outside = super::VersionGraphLayoutNode {
                id: VersionGraphNodeId {
                    recipe_id: Uuid::from_u128(707 + index as u128),
                    version_number: 1,
                },
                x: anchor_x,
                y: anchor_y,
                rank: 0,
                row: 0,
            };
            let transform = super::VersionGraphTransform {
                scale,
                offset_x,
                offset_y,
            };

            assert!(!super::version_graph_anchor_intersects_viewport(
                fully_outside,
                transform,
                viewport,
            ));
        }
    }

    #[test]
    fn frontier_controls_remain_available_when_their_anchor_is_partially_visible() {
        let viewport = (800.0, 600.0);
        let scale = 0.5;
        let anchor_x = 100.0;
        let transform = super::VersionGraphTransform {
            scale,
            offset_x: -(anchor_x + super::GRAPH_NODE_WIDTH_PX) * scale + 0.5,
            offset_y: 0.0,
        };
        let partially_left = super::VersionGraphLayoutNode {
            id: VersionGraphNodeId {
                recipe_id: Uuid::from_u128(708),
                version_number: 1,
            },
            x: anchor_x,
            y: 100.0,
            rank: 0,
            row: 0,
        };

        assert!(super::version_graph_anchor_intersects_viewport(
            partially_left,
            transform,
            viewport,
        ));
    }

    #[test]
    fn frontier_control_markup_applies_the_zoom_scale_to_the_overlay_control() {
        let source = include_str!("recipe_version_graph.rs");

        assert!(source.contains("--recipe-version-graph-frontier-scale"));
        assert!(source.contains("version_graph_frontier_control_scale(transform)"));
    }

    #[test]
    fn collapsing_one_expansion_keeps_an_independent_branch_visible() {
        let root_id = Uuid::from_u128(710);
        let left_id = Uuid::from_u128(711);
        let right_id = Uuid::from_u128(712);
        let root = visible_node(root_id, 1, "Root");
        let left = visible_node(left_id, 1, "Left");
        let right = visible_node(right_id, 1, "Right");
        let initial = VersionGraph {
            nodes: vec![root],
            ..Default::default()
        };
        let left_batch = VersionGraph {
            nodes: vec![left],
            edges: vec![VersionGraphEdge::fork(root_id, 1, left_id).unwrap()],
            ..Default::default()
        };
        let right_batch = VersionGraph {
            nodes: vec![right],
            edges: vec![VersionGraphEdge::fork(root_id, 1, right_id).unwrap()],
            ..Default::default()
        };

        let expanded = super::merge_version_graph_batches(&initial, [&left_batch, &right_batch]);
        let collapsed_left = super::merge_version_graph_batches(&initial, [&right_batch]);

        assert_eq!(expanded.nodes.len(), 3);
        assert_eq!(collapsed_left.nodes.len(), 2);
        assert!(collapsed_left
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == right_id));
        assert!(!collapsed_left
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == left_id));
    }

    #[test]
    fn nested_expansions_are_hidden_with_their_parent_without_losing_cached_data() {
        let anchor = VersionGraphNodeId {
            recipe_id: Uuid::from_u128(720),
            version_number: 2,
        };
        let parent_frontier = VersionGraphFrontier {
            anchor,
            kind: VersionGraphFrontierKind::OlderVersions,
            hidden_count: 2,
            fork_cursor: None,
        };
        let child_frontier = VersionGraphFrontier {
            anchor: VersionGraphNodeId {
                recipe_id: anchor.recipe_id,
                version_number: 1,
            },
            kind: VersionGraphFrontierKind::ParentFork,
            hidden_count: 1,
            fork_cursor: None,
        };
        let parent_id = super::VersionGraphFrontierId::from(parent_frontier);
        let child_id = super::VersionGraphFrontierId::from(child_frontier);
        let mut records = vec![
            super::VersionGraphExpansionRecord {
                frontier: parent_frontier,
                parent: None,
                batch: Some(VersionGraph {
                    frontiers: vec![child_frontier],
                    ..Default::default()
                }),
                is_expanded: true,
                is_loading: false,
                error: None,
            },
            super::VersionGraphExpansionRecord {
                frontier: child_frontier,
                parent: Some(parent_id),
                batch: Some(VersionGraph::default()),
                is_expanded: true,
                is_loading: false,
                error: None,
            },
        ];

        assert!(super::expansion_record_is_visible(child_id, &records));
        records[0].is_expanded = false;
        assert!(!super::expansion_record_is_visible(child_id, &records));
        assert!(records[1].batch.is_some());
        assert!(records[1].is_expanded);
        records[0].is_expanded = true;
        assert!(super::expansion_record_is_visible(child_id, &records));
    }

    #[test]
    fn active_expansion_retains_cached_nodes_referenced_by_its_edges() {
        let root_id = Uuid::from_u128(730);
        let shared_id = Uuid::from_u128(731);
        let collapsed_only_id = Uuid::from_u128(732);
        let active_id = Uuid::from_u128(733);
        let initial = VersionGraph {
            nodes: vec![visible_node(root_id, 1, "Root")],
            ..Default::default()
        };
        let collapsed_batch = VersionGraph {
            nodes: vec![
                visible_node(shared_id, 1, "Shared"),
                visible_node(collapsed_only_id, 1, "Collapsed only"),
            ],
            ..Default::default()
        };
        let active_batch = VersionGraph {
            nodes: vec![visible_node(active_id, 1, "Active")],
            edges: vec![VersionGraphEdge::fork(shared_id, 1, active_id).unwrap()],
            ..Default::default()
        };
        let cached =
            super::merge_version_graph_batches(&initial, [&collapsed_batch, &active_batch]);
        let visible = super::retain_cached_version_graph_dependencies(
            super::merge_version_graph_batches(&initial, [&active_batch]),
            &cached,
        );
        let visible_ids = visible
            .nodes
            .iter()
            .map(VersionGraphNode::id)
            .collect::<std::collections::HashSet<_>>();

        assert!(visible_ids.contains(&VersionGraphNodeId {
            recipe_id: shared_id,
            version_number: 1,
        }));
        assert!(!visible_ids.contains(&VersionGraphNodeId {
            recipe_id: collapsed_only_id,
            version_number: 1,
        }));
    }

    #[test]
    fn expansion_requests_exclude_cached_dependency_nodes_from_the_loaded_set() {
        let root_id = Uuid::from_u128(740);
        let cached_dependency_id = Uuid::from_u128(741);
        let active_id = Uuid::from_u128(742);
        let initial = VersionGraph {
            nodes: vec![visible_node(root_id, 1, "Root")],
            ..Default::default()
        };
        let active_batch = VersionGraph {
            nodes: vec![visible_node(active_id, 1, "Active")],
            edges: vec![VersionGraphEdge::fork(cached_dependency_id, 1, active_id).unwrap()],
            ..Default::default()
        };
        let visible = super::merge_version_graph_batches(&initial, [&active_batch]);
        let cached = VersionGraph {
            nodes: vec![visible_node(cached_dependency_id, 1, "Cached dependency")],
            ..Default::default()
        };
        let rendered = super::retain_cached_version_graph_dependencies(visible.clone(), &cached);

        assert!(rendered
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == cached_dependency_id));
        assert!(!super::version_graph_expansion_loaded_node_ids(&visible)
            .iter()
            .any(|node_id| node_id.recipe_id == cached_dependency_id));
    }
}
