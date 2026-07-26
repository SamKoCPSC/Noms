use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::diff::{
    compute_diff, json_to_snapshot, recipe_to_snapshot, snapshot_to_json, RecipeDraftPayload,
    RecipeSnapshot, SnapshotChange,
};
use super::{DbError, Recipe};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RecipeVersion {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub version_number: i32,
    pub snapshot: serde_json::Value,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl RecipeVersion {
    pub fn decoded_snapshot(&self) -> Result<RecipeSnapshot, DbError> {
        json_to_snapshot(&self.snapshot)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RecipeDraft {
    pub id: Uuid,
    pub recipe_id: Option<Uuid>,
    pub user_id: Uuid,
    pub base_version_number: Option<i32>,
    pub snapshot: serde_json::Value,
    pub revision: i64,
    pub forked_from_recipe_id: Option<Uuid>,
    pub forked_from_version_number: Option<i32>,
    pub forked_from_title: Option<String>,
    pub fork_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RecipeDraft {
    pub fn decoded_snapshot(&self) -> Result<RecipeDraftPayload, DbError> {
        serde_json::from_value(self.snapshot.clone()).map_err(DbError::SerdeJson)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ForkRelationship {
    pub id: Uuid,
    pub original_recipe_id: Option<Uuid>,
    pub forked_recipe_id: Option<Uuid>,
    pub forked_by: Option<Uuid>,
    pub forked_version_number: Option<i32>,
    pub original_title: String,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LineageEdgeRow {
    id: Uuid,
    parent_relationship_id: Option<Uuid>,
    original_recipe_id: Option<Uuid>,
    original_recipe_key: Option<Uuid>,
    forked_recipe_id: Option<Uuid>,
    forked_recipe_key: Option<Uuid>,
    forked_version_number: Option<i32>,
    original_title: String,
    forked_title: Option<String>,
    original_owner_id: Option<Uuid>,
    original_owner_username: Option<String>,
    original_visibility: Option<String>,
    forked_owner_id: Option<Uuid>,
    forked_owner_username: Option<String>,
    forked_visibility: Option<String>,
    message: Option<String>,
    created_at: DateTime<Utc>,
    depth: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LiveLineageNode {
    id: Uuid,
    user_id: Uuid,
    title: String,
    visibility: String,
    owner_username: String,
    image_url: Option<String>,
}

#[derive(Debug, Clone)]
struct LineageNodeSnapshot {
    recipe_key: Option<Uuid>,
    title: String,
    owner_id: Option<Uuid>,
    owner_username: Option<String>,
    visibility: Option<String>,
    forked_from_version_number: Option<i32>,
    fork_message: Option<String>,
}

const VERSION_COLUMNS: &str = "id, recipe_id, version_number, snapshot, notes, created_at";
const DRAFT_COLUMNS: &str = "id, recipe_id, user_id, base_version_number, snapshot, revision, \
     forked_from_recipe_id, forked_from_version_number, forked_from_title, fork_message, \
     created_at, updated_at";

pub async fn get_recipe_versions(
    pool: &PgPool,
    recipe_id: Uuid,
) -> Result<Vec<RecipeVersion>, DbError> {
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = $1 ORDER BY version_number DESC"
    ))
    .bind(recipe_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
}

pub async fn get_latest_recipe_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<RecipeVersion, DbError> {
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = $1 ORDER BY version_number DESC LIMIT 1"
    ))
    .bind(recipe_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::VersionNotFound {
        recipe_id,
        version_number: 0,
    })
}

pub async fn get_recipe_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
) -> Result<RecipeVersion, DbError> {
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = $1 AND version_number = $2"
    ))
    .bind(recipe_id)
    .bind(version_number)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::VersionNotFound {
        recipe_id,
        version_number,
    })
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct VersionGraphRelationshipRow {
    id: Uuid,
    original_recipe_id: Option<Uuid>,
    original_recipe_key: Option<Uuid>,
    forked_recipe_id: Option<Uuid>,
    forked_recipe_key: Option<Uuid>,
    forked_version_number: Option<i32>,
    original_title: String,
    forked_title: Option<String>,
    original_owner_id: Option<Uuid>,
    original_owner_username: Option<String>,
    original_visibility: Option<String>,
    forked_owner_id: Option<Uuid>,
    forked_owner_username: Option<String>,
    forked_visibility: Option<String>,
    created_at: DateTime<Utc>,
}

impl VersionGraphRelationshipRow {
    fn original_recipe_key(&self) -> Option<Uuid> {
        self.original_recipe_key.or(self.original_recipe_id)
    }

    fn forked_recipe_key(&self) -> Option<Uuid> {
        self.forked_recipe_key.or(self.forked_recipe_id)
    }

    fn original_endpoint_metadata(&self) -> VersionGraphEndpointMetadata<'_> {
        VersionGraphEndpointMetadata {
            title: &self.original_title,
            owner_username: self.original_owner_username.as_deref(),
            owner_id: self.original_owner_id,
            visibility: self.original_visibility.as_deref(),
            created_at: self.created_at,
        }
    }

    fn forked_endpoint_metadata(&self) -> VersionGraphEndpointMetadata<'_> {
        VersionGraphEndpointMetadata {
            title: self.forked_title.as_deref().unwrap_or("Deleted recipe"),
            owner_username: self.forked_owner_username.as_deref(),
            owner_id: self.forked_owner_id,
            visibility: self.forked_visibility.as_deref(),
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct VersionGraphEndpointMetadata<'a> {
    title: &'a str,
    owner_username: Option<&'a str>,
    owner_id: Option<Uuid>,
    visibility: Option<&'a str>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
struct VersionGraphLimits {
    max_nodes: usize,
    max_edges: usize,
    max_depth: usize,
    max_relationships: usize,
}

impl Default for VersionGraphLimits {
    fn default() -> Self {
        Self {
            max_nodes: 1_000,
            max_edges: 1_500,
            max_depth: 32,
            max_relationships: 500,
        }
    }
}

fn graph_image(images: &[crate::types::RecipeImage]) -> Option<crate::types::RecipeImage> {
    images.first().cloned()
}

fn graph_node_state(
    is_selected_recipe: bool,
    is_current_version: bool,
    is_deleted: bool,
) -> crate::types::VersionGraphNodeState {
    crate::types::VersionGraphNodeState {
        is_selected_recipe,
        is_current_version,
        is_deleted,
    }
}

fn append_live_recipe_chain(
    graph: &mut crate::types::VersionGraph,
    recipe: &Recipe,
    mut versions: Vec<RecipeVersion>,
    is_selected_recipe: bool,
) -> Result<(), DbError> {
    versions.sort_by_key(|version| version.version_number);
    let snapshots = versions
        .iter()
        .map(RecipeVersion::decoded_snapshot)
        .collect::<Result<Vec<_>, _>>()?;
    let current_version_number = versions.last().map(|version| version.version_number);

    for (index, (version, snapshot)) in versions.iter().zip(&snapshots).enumerate() {
        let id = crate::types::VersionGraphNodeId {
            recipe_id: recipe.id,
            version_number: version.version_number,
        };
        let change_summary = match index.checked_sub(1) {
            Some(previous_index) => {
                super::diff::change_summary(&compute_diff(&snapshots[previous_index], snapshot)?)
            }
            None => "Initial version".to_string(),
        };
        graph.nodes.push(crate::types::VersionGraphNode::visible(
            id,
            crate::types::VersionGraphNodeContent {
                title: snapshot.title.clone(),
                owner_username: Some(recipe.author_username.clone()),
                image: graph_image(&snapshot.images),
                created_at: version.created_at,
                change_summary,
            },
            graph_node_state(
                is_selected_recipe,
                current_version_number == Some(version.version_number),
                false,
            ),
        ));
    }

    for pair in versions.windows(2) {
        if let Some(edge) = crate::types::VersionGraphEdge::version(
            crate::types::VersionGraphNodeId {
                recipe_id: recipe.id,
                version_number: pair[0].version_number,
            },
            crate::types::VersionGraphNodeId {
                recipe_id: recipe.id,
                version_number: pair[1].version_number,
            },
        ) {
            graph.edges.push(edge);
        }
    }
    Ok(())
}

fn adjacent_live_recipe_is_visible(recipe: &Recipe, viewer_id: Option<Uuid>) -> bool {
    recipe.visibility == "public" || viewer_id == Some(recipe.user_id)
}

fn deleted_recipe_metadata_is_visible(
    visibility: Option<&str>,
    owner_id: Option<Uuid>,
    viewer_id: Option<Uuid>,
) -> bool {
    visibility == Some("public") || owner_id.is_some_and(|owner| viewer_id == Some(owner))
}

fn append_synthetic_graph_node(
    graph: &mut crate::types::VersionGraph,
    id: crate::types::VersionGraphNodeId,
    title: &str,
    owner_username: Option<&str>,
    created_at: DateTime<Utc>,
    is_deleted: bool,
    metadata_is_visible: bool,
) {
    let state = graph_node_state(false, false, is_deleted);
    if metadata_is_visible {
        graph.nodes.push(crate::types::VersionGraphNode::visible(
            id,
            crate::types::VersionGraphNodeContent {
                title: title.to_string(),
                owner_username: owner_username.map(str::to_string),
                image: None,
                created_at,
                change_summary: if is_deleted {
                    "Deleted recipe".to_string()
                } else {
                    "Forked recipe".to_string()
                },
            },
            state,
        ));
    } else {
        // A topology-only endpoint proves the fork edge without exposing the
        // adjacent recipe's current or deletion-snapshot metadata.
        graph
            .nodes
            .push(crate::types::VersionGraphNode::redacted(id, state));
    }
}

fn deduplicate_and_sort_graph(graph: &mut crate::types::VersionGraph) {
    graph.nodes.sort_by_key(|node| {
        (
            !node.is_selected_recipe(),
            node.id().recipe_id,
            node.id().version_number,
            node.is_redacted(),
        )
    });
    graph.nodes.dedup_by_key(|node| node.id());
    graph.edges.sort_by_key(|edge| {
        (
            edge.source().recipe_id,
            edge.source().version_number,
            edge.target().recipe_id,
            edge.target().version_number,
            match edge.kind() {
                crate::types::VersionGraphEdgeKind::Version => 0,
                crate::types::VersionGraphEdgeKind::Fork => 1,
            },
        )
    });
    graph.edges.dedup_by(|left, right| {
        left.source() == right.source()
            && left.target() == right.target()
            && left.kind() == right.kind()
    });
}

fn graph_query_limit(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}

async fn traverse_version_graph_relationships(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    limits: VersionGraphLimits,
) -> Result<(Vec<VersionGraphRelationshipRow>, HashSet<Uuid>, bool, bool), DbError> {
    let mut relationships = Vec::new();
    let mut discovered_recipe_keys = HashSet::from([selected_recipe_id]);
    let mut traversed_relationship_ids = HashSet::new();
    let mut frontier = vec![selected_recipe_id];
    let mut depth = 0usize;
    let mut depth_limit_reached = false;
    let mut traversal_limit_reached = false;

    while !frontier.is_empty() {
        frontier.sort_unstable();
        let remaining_relationships = limits
            .max_relationships
            .saturating_sub(traversed_relationship_ids.len());
        let query_limit = remaining_relationships.saturating_add(1);
        let mut frontier_relationships = sqlx::query_as::<_, VersionGraphRelationshipRow>(
            "SELECT id, original_recipe_id, original_recipe_key, forked_recipe_id, \
                     forked_recipe_key, forked_version_number, original_title, forked_title, \
                     original_owner_id, original_owner_username, original_visibility, \
                     forked_owner_id, forked_owner_username, forked_visibility, created_at \
                 FROM fork_relationships \
                 WHERE (COALESCE(original_recipe_key, original_recipe_id) = ANY($1) \
                    OR COALESCE(forked_recipe_key, forked_recipe_id) = ANY($1)) \
                   AND NOT (id = ANY($2)) \
                 ORDER BY created_at, id \
                 LIMIT $3",
        )
        .bind(&frontier)
        .bind(
            traversed_relationship_ids
                .iter()
                .copied()
                .collect::<Vec<_>>(),
        )
        .bind(graph_query_limit(query_limit))
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?;

        if frontier_relationships.len() > remaining_relationships {
            traversal_limit_reached = true;
            frontier_relationships.truncate(remaining_relationships);
        }

        let mut next_frontier = HashSet::new();
        for relationship in frontier_relationships {
            traversed_relationship_ids.insert(relationship.id);
            let Some(source_recipe_key) = relationship.original_recipe_key() else {
                continue;
            };
            let Some(forked_recipe_key) = relationship.forked_recipe_key() else {
                continue;
            };

            let source_was_discovered = discovered_recipe_keys.contains(&source_recipe_key);
            let fork_was_discovered = discovered_recipe_keys.contains(&forked_recipe_key);
            if depth >= limits.max_depth {
                if !source_was_discovered || !fork_was_discovered {
                    depth_limit_reached = true;
                    continue;
                }
            } else {
                if discovered_recipe_keys.insert(source_recipe_key) {
                    next_frontier.insert(source_recipe_key);
                }
                if discovered_recipe_keys.insert(forked_recipe_key) {
                    next_frontier.insert(forked_recipe_key);
                }
            }

            relationships.push(relationship);
        }

        if traversal_limit_reached {
            break;
        }
        frontier = next_frontier.into_iter().collect();
        depth = depth.saturating_add(1);
    }

    Ok((
        relationships,
        discovered_recipe_keys,
        depth_limit_reached,
        traversal_limit_reached,
    ))
}

async fn load_version_graph_recipes(
    pool: &PgPool,
    recipe_keys: &[Uuid],
) -> Result<HashMap<Uuid, Recipe>, DbError> {
    sqlx::query_as::<_, Recipe>(
        "SELECT recipes.id, recipes.user_id, recipes.title, recipes.description, \
             recipes.commentary, recipes.prep_time_minutes, recipes.cook_time_minutes, \
             recipes.servings, recipes.ingredients, recipes.instructions, recipes.equipment, \
             recipes.images, recipes.visibility, recipes.created_at, recipes.updated_at, \
             users.username AS author_username, users.avatar_url AS author_avatar_url \
         FROM recipes JOIN users ON users.id = recipes.user_id \
         WHERE recipes.id = ANY($1) \
         ORDER BY recipes.id",
    )
    .bind(recipe_keys)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
    .map(|recipes| {
        recipes
            .into_iter()
            .map(|recipe| (recipe.id, recipe))
            .collect()
    })
}

async fn load_version_graph_versions(
    pool: &PgPool,
    recipe_ids: &[Uuid],
    selected_recipe_id: Uuid,
    max_nodes: usize,
) -> Result<HashMap<Uuid, Vec<RecipeVersion>>, DbError> {
    if recipe_ids.is_empty() {
        return Ok(HashMap::new());
    }

    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = ANY($1) \
         ORDER BY (recipe_id = $2) DESC, recipe_id, version_number \
         LIMIT $3"
    ))
    .bind(recipe_ids)
    .bind(selected_recipe_id)
    .bind(graph_query_limit(max_nodes.saturating_add(1)))
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
    .map(|versions| {
        let mut versions_by_recipe = HashMap::<Uuid, Vec<RecipeVersion>>::new();
        for version in versions {
            versions_by_recipe
                .entry(version.recipe_id)
                .or_default()
                .push(version);
        }
        versions_by_recipe
    })
}

fn append_relationship_endpoint(
    graph: &mut crate::types::VersionGraph,
    live_recipes: &HashMap<Uuid, Recipe>,
    visible_recipe_ids: &HashSet<Uuid>,
    id: crate::types::VersionGraphNodeId,
    metadata: VersionGraphEndpointMetadata<'_>,
    viewer_id: Option<Uuid>,
) {
    match live_recipes.get(&id.recipe_id) {
        Some(_) if visible_recipe_ids.contains(&id.recipe_id) => {}
        Some(_) => {
            append_synthetic_graph_node(graph, id, "", None, metadata.created_at, false, false)
        }
        None => append_synthetic_graph_node(
            graph,
            id,
            metadata.title,
            metadata.owner_username,
            metadata.created_at,
            true,
            deleted_recipe_metadata_is_visible(metadata.visibility, metadata.owner_id, viewer_id),
        ),
    }
}

fn apply_version_graph_limits(
    graph: &mut crate::types::VersionGraph,
    limits: VersionGraphLimits,
    depth_limit_reached: bool,
    traversal_limit_reached: bool,
) {
    deduplicate_and_sort_graph(graph);

    let node_limit_reached = graph.nodes.len() > limits.max_nodes;
    graph.nodes.truncate(limits.max_nodes);
    let retained_node_ids = graph
        .nodes
        .iter()
        .map(crate::types::VersionGraphNode::id)
        .collect::<HashSet<_>>();
    graph.edges.retain(|edge| {
        retained_node_ids.contains(&edge.source()) && retained_node_ids.contains(&edge.target())
    });

    let edge_limit_reached = graph.edges.len() > limits.max_edges;
    graph.edges.truncate(limits.max_edges);
    graph.truncation = crate::types::VersionGraphTruncation {
        is_truncated: node_limit_reached
            || edge_limit_reached
            || depth_limit_reached
            || traversal_limit_reached,
        node_limit_reached,
        edge_limit_reached,
        depth_limit_reached,
        traversal_limit_reached,
    };
}

async fn get_recipe_version_graph_with_limits(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    limits: VersionGraphLimits,
) -> Result<crate::types::VersionGraph, DbError> {
    super::get_recipe_by_id(pool, selected_recipe_id)
        .await?
        .filter(|recipe| recipe.visibility != "private" || viewer_id == Some(recipe.user_id))
        .ok_or(DbError::RecipeNotFound(selected_recipe_id))?;

    let (relationships, discovered_recipe_keys, depth_limit_reached, traversal_limit_reached) =
        traverse_version_graph_relationships(pool, selected_recipe_id, limits).await?;
    let mut recipe_keys = discovered_recipe_keys.into_iter().collect::<Vec<_>>();
    recipe_keys.sort_unstable();
    let live_recipes = load_version_graph_recipes(pool, &recipe_keys).await?;
    let mut visible_recipe_ids = live_recipes
        .values()
        .filter(|recipe| {
            recipe.id == selected_recipe_id || adjacent_live_recipe_is_visible(recipe, viewer_id)
        })
        .map(|recipe| recipe.id)
        .collect::<Vec<_>>();
    visible_recipe_ids.sort_by_key(|recipe_id| (*recipe_id != selected_recipe_id, *recipe_id));
    let visible_recipe_id_set = visible_recipe_ids.iter().copied().collect::<HashSet<_>>();
    let mut versions_by_recipe = load_version_graph_versions(
        pool,
        &visible_recipe_ids,
        selected_recipe_id,
        limits.max_nodes,
    )
    .await?;

    let mut graph = crate::types::VersionGraph::default();
    for recipe_id in &visible_recipe_ids {
        let Some(recipe) = live_recipes.get(recipe_id) else {
            continue;
        };
        append_live_recipe_chain(
            &mut graph,
            recipe,
            versions_by_recipe.remove(recipe_id).unwrap_or_default(),
            *recipe_id == selected_recipe_id,
        )?;
    }

    for relationship in &relationships {
        let Some(source_recipe_id) = relationship.original_recipe_key() else {
            continue;
        };
        let Some(forked_recipe_id) = relationship.forked_recipe_key() else {
            continue;
        };
        let Some(recorded_version) = relationship.forked_version_number else {
            continue;
        };
        if let Some(edge) = crate::types::VersionGraphEdge::fork(
            source_recipe_id,
            recorded_version,
            forked_recipe_id,
        ) {
            append_relationship_endpoint(
                &mut graph,
                &live_recipes,
                &visible_recipe_id_set,
                edge.source(),
                relationship.original_endpoint_metadata(),
                viewer_id,
            );
            append_relationship_endpoint(
                &mut graph,
                &live_recipes,
                &visible_recipe_id_set,
                edge.target(),
                relationship.forked_endpoint_metadata(),
                viewer_id,
            );
            graph.edges.push(edge);
        }
    }

    apply_version_graph_limits(
        &mut graph,
        limits,
        depth_limit_reached,
        traversal_limit_reached,
    );
    Ok(graph)
}

/// Build the bounded connected version graph containing the selected recipe.
pub async fn get_recipe_version_graph(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
) -> Result<crate::types::VersionGraph, DbError> {
    get_recipe_version_graph_with_limits(
        pool,
        selected_recipe_id,
        viewer_id,
        VersionGraphLimits::default(),
    )
    .await
}

pub async fn compare_recipe_versions(
    pool: &PgPool,
    recipe_id: Uuid,
    from_version_number: i32,
    to_version_number: i32,
) -> Result<Vec<SnapshotChange>, DbError> {
    let from = get_recipe_version(pool, recipe_id, from_version_number)
        .await?
        .decoded_snapshot()?;
    let to = get_recipe_version(pool, recipe_id, to_version_number)
        .await?
        .decoded_snapshot()?;
    compute_diff(&from, &to)
}

pub async fn get_next_version_number(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<i32, DbError> {
    let latest = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(version_number) FROM recipe_versions WHERE recipe_id = $1",
    )
    .bind(recipe_id)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(latest.unwrap_or(0) + 1)
}

pub async fn insert_recipe_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
    snapshot: &RecipeSnapshot,
    notes: Option<&str>,
) -> Result<RecipeVersion, DbError> {
    let snapshot = snapshot_to_json(snapshot)?;
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes) \
         VALUES ($1, $2, $3, $4) RETURNING {VERSION_COLUMNS}"
    ))
    .bind(recipe_id)
    .bind(version_number)
    .bind(snapshot)
    .bind(notes)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

#[allow(clippy::too_many_arguments)]
async fn insert_recipe_draft_with_source(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
    base_version_number: Option<i32>,
    snapshot: &RecipeDraftPayload,
    forked_from_recipe_id: Option<Uuid>,
    forked_from_version_number: Option<i32>,
    forked_from_title: Option<&str>,
    fork_message: Option<&str>,
) -> Result<RecipeDraft, DbError> {
    let snapshot = serde_json::to_value(snapshot).map_err(DbError::SerdeJson)?;
    let inserted = sqlx::query_as::<_, RecipeDraft>(&format!(
        "INSERT INTO recipe_drafts (recipe_id, user_id, base_version_number, snapshot, \
             forked_from_recipe_id, forked_from_version_number, forked_from_title, fork_message) \
         SELECT $1, $2, $3, $4, $5, $6, $7, $8 \
         WHERE $1::UUID IS NULL OR EXISTS (\
             SELECT 1 FROM recipes WHERE id = $1 AND user_id = $2\
         ) RETURNING {DRAFT_COLUMNS}"
    ))
    .bind(recipe_id)
    .bind(user_id)
    .bind(base_version_number)
    .bind(snapshot)
    .bind(forked_from_recipe_id)
    .bind(forked_from_version_number)
    .bind(forked_from_title)
    .bind(fork_message)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?;
    inserted.ok_or_else(|| {
        recipe_id
            .map(DbError::RecipeNotFound)
            .unwrap_or(DbError::DraftNotFound(Uuid::nil()))
    })
}

pub async fn insert_recipe_draft(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
    base_version_number: Option<i32>,
    payload: &RecipeDraftPayload,
) -> Result<RecipeDraft, DbError> {
    insert_recipe_draft_with_source(
        executor,
        user_id,
        recipe_id,
        base_version_number,
        payload,
        None,
        None,
        None,
        None,
    )
    .await
}

pub async fn create_new_recipe_draft(
    pool: &PgPool,
    user_id: Uuid,
    payload: &RecipeDraftPayload,
) -> Result<RecipeDraft, DbError> {
    insert_recipe_draft(pool, user_id, None, None, payload).await
}

pub async fn get_recipe_draft(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    draft_id: Uuid,
    user_id: Uuid,
) -> Result<RecipeDraft, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts WHERE id = $1 AND user_id = $2"
    ))
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::DraftNotFound(draft_id))
}

pub async fn get_recipe_draft_for_recipe(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
) -> Result<Option<RecipeDraft>, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts \
         WHERE recipe_id = $1 AND user_id = $2"
    ))
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

pub async fn list_recipe_drafts(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<RecipeDraft>, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts \
         WHERE user_id = $1 ORDER BY updated_at DESC"
    ))
    .bind(user_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

pub async fn update_recipe_draft(
    pool: &PgPool,
    draft_id: Uuid,
    user_id: Uuid,
    expected_revision: i64,
    payload: &RecipeDraftPayload,
) -> Result<RecipeDraft, DbError> {
    let snapshot = serde_json::to_value(payload).map_err(DbError::SerdeJson)?;
    let updated = sqlx::query_as::<_, RecipeDraft>(&format!(
        "UPDATE recipe_drafts SET snapshot = $4, revision = revision + 1, updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND revision = $3 RETURNING {DRAFT_COLUMNS}"
    ))
    .bind(draft_id)
    .bind(user_id)
    .bind(expected_revision)
    .bind(snapshot)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;
    if let Some(updated) = updated {
        return Ok(updated);
    }

    let actual = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM recipe_drafts WHERE id = $1 AND user_id = $2",
    )
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::DraftNotFound(draft_id))?;
    Err(DbError::DraftConflict {
        draft_id,
        expected: expected_revision,
        actual,
    })
}

pub async fn delete_recipe_draft(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    draft_id: Uuid,
    user_id: Uuid,
) -> Result<(), DbError> {
    let affected = sqlx::query("DELETE FROM recipe_drafts WHERE id = $1 AND user_id = $2")
        .bind(draft_id)
        .bind(user_id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    if affected == 0 {
        return Err(DbError::DraftNotFound(draft_id));
    }
    Ok(())
}

pub async fn create_edit_draft(
    pool: &PgPool,
    recipe_id: Uuid,
    user_id: Uuid,
) -> Result<RecipeDraft, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let locked_recipe = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM recipes WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if locked_recipe.is_none() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    if let Some(draft) = get_recipe_draft_for_recipe(&mut *transaction, recipe_id, user_id).await? {
        transaction.commit().await.map_err(DbError::Query)?;
        return Ok(draft);
    }
    let recipe = super::get_recipe_by_id_and_owner(&mut *transaction, recipe_id, user_id).await?;
    let tags = super::get_recipe_tags(&mut *transaction, recipe_id)
        .await?
        .into_iter()
        .map(|tag| tag.tag)
        .collect();
    let snapshot = RecipeDraftPayload {
        content: recipe_to_snapshot(&recipe, tags),
        initial_visibility: None,
    };
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;

    let draft = insert_recipe_draft(
        &mut *transaction,
        user_id,
        Some(recipe_id),
        Some(latest.version_number),
        &snapshot,
    )
    .await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(draft)
}

pub async fn create_published_recipe(
    pool: &PgPool,
    user_id: Uuid,
    payload: &RecipeDraftPayload,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let snapshot = normalized_snapshot(&payload.content);
    let visibility = payload.initial_visibility.as_deref().ok_or_else(|| {
        DbError::InvalidRecipeSnapshot("initial visibility is required".to_string())
    })?;
    validate_snapshot_for_publish(&snapshot, visibility)?;
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let recipe =
        insert_recipe_from_snapshot(&mut transaction, user_id, &snapshot, visibility).await?;
    insert_recipe_version(&mut *transaction, recipe.id, 1, &snapshot, notes).await?;
    super::insert_recipe_tags(&mut transaction, recipe.id, &snapshot.tags).await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

async fn insert_recipe_from_snapshot(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    snapshot: &RecipeSnapshot,
    visibility: &str,
) -> Result<Recipe, DbError> {
    super::insert_recipe(
        &mut **transaction,
        user_id,
        &snapshot.title,
        snapshot.description.as_deref(),
        snapshot.commentary.as_deref(),
        snapshot.prep_time_minutes,
        snapshot.cook_time_minutes,
        snapshot.servings,
        &snapshot.ingredients,
        &snapshot.instructions,
        &snapshot.equipment,
        &snapshot.images,
        visibility,
    )
    .await
}

async fn update_recipe_from_snapshot(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
    snapshot: &RecipeSnapshot,
) -> Result<Recipe, DbError> {
    let recipe = super::update_recipe(
        &mut **transaction,
        recipe_id,
        user_id,
        &snapshot.title,
        snapshot.description.as_deref(),
        snapshot.commentary.as_deref(),
        snapshot.prep_time_minutes,
        snapshot.cook_time_minutes,
        snapshot.servings,
        &snapshot.ingredients,
        &snapshot.instructions,
        &snapshot.equipment,
        &snapshot.images,
        None,
    )
    .await?;
    super::insert_recipe_tags(transaction, recipe_id, &snapshot.tags).await?;
    Ok(recipe)
}

pub async fn publish_recipe_draft(
    pool: &PgPool,
    draft_id: Uuid,
    user_id: Uuid,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let draft = get_recipe_draft(pool, draft_id, user_id).await?;
    match draft.recipe_id {
        Some(recipe_id) => publish_existing_draft(pool, draft_id, recipe_id, user_id, notes).await,
        None => publish_new_draft(pool, draft_id, user_id, notes).await,
    }
}

async fn locked_draft(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    draft_id: Uuid,
    user_id: Uuid,
) -> Result<RecipeDraft, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts \
         WHERE id = $1 AND user_id = $2 FOR UPDATE"
    ))
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::DraftNotFound(draft_id))
}

async fn publish_new_draft(
    pool: &PgPool,
    draft_id: Uuid,
    user_id: Uuid,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let draft = locked_draft(&mut transaction, draft_id, user_id).await?;
    if draft.recipe_id.is_some() {
        return Err(DbError::DraftNotFound(draft_id));
    }
    let payload = draft.decoded_snapshot()?;
    let snapshot = normalized_snapshot(&payload.content);
    let visibility = payload.initial_visibility.as_deref().ok_or_else(|| {
        DbError::InvalidRecipeSnapshot("initial visibility is required".to_string())
    })?;
    validate_snapshot_for_publish(&snapshot, visibility)?;
    let recipe =
        insert_recipe_from_snapshot(&mut transaction, user_id, &snapshot, visibility).await?;
    insert_recipe_version(&mut *transaction, recipe.id, 1, &snapshot, notes).await?;
    super::insert_recipe_tags(&mut transaction, recipe.id, &snapshot.tags).await?;

    if let (Some(original_title), Some(version_number)) = (
        draft.forked_from_title.as_deref(),
        draft.forked_from_version_number,
    ) {
        let source = if let Some(source_recipe_id) = draft.forked_from_recipe_id {
            sqlx::query_as::<_, LiveLineageNode>(
                "SELECT recipes.id, recipes.user_id, recipes.title, recipes.visibility, \
                     users.username AS owner_username, \
                     COALESCE(recipes.images -> 0 ->> 'thumb_url', \
                              recipes.images -> 0 ->> 'url', \
                              recipes.images ->> 0) AS image_url \
                 FROM recipes JOIN users ON users.id = recipes.user_id \
                 WHERE recipes.id = $1",
            )
            .bind(source_recipe_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DbError::Query)?
        } else {
            None
        };
        let parent_relationship_id = if let Some(source_recipe_id) = draft.forked_from_recipe_id {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM fork_relationships \
                 WHERE forked_recipe_key = $1 OR forked_recipe_id = $1 \
                 ORDER BY created_at DESC LIMIT 1",
            )
            .bind(source_recipe_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DbError::Query)?
        } else {
            None
        };
        let forked_owner_username =
            sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(DbError::Query)?;

        sqlx::query(
            "INSERT INTO fork_relationships (\
                 original_recipe_id, forked_recipe_id, forked_by, parent_relationship_id, \
                 original_recipe_key, forked_recipe_key, forked_version_number, \
                 original_title, forked_title, original_owner_id, original_owner_username, \
                 original_visibility, forked_owner_id, forked_owner_username, \
                 forked_visibility, message\
             ) VALUES (\
                 $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16\
             )",
        )
        .bind(draft.forked_from_recipe_id)
        .bind(recipe.id)
        .bind(user_id)
        .bind(parent_relationship_id)
        .bind(draft.forked_from_recipe_id)
        .bind(recipe.id)
        .bind(version_number)
        .bind(original_title)
        .bind(&snapshot.title)
        .bind(source.as_ref().map(|source| source.user_id))
        .bind(source.as_ref().map(|source| source.owner_username.as_str()))
        .bind(source.as_ref().map(|source| source.visibility.as_str()))
        .bind(user_id)
        .bind(forked_owner_username)
        .bind(visibility)
        .bind(draft.fork_message.as_deref())
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    }

    sqlx::query("DELETE FROM recipe_drafts WHERE id = $1")
        .bind(draft_id)
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

async fn publish_existing_draft(
    pool: &PgPool,
    draft_id: Uuid,
    recipe_id: Uuid,
    user_id: Uuid,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let owner = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM recipes WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if owner.is_none() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    let draft = locked_draft(&mut transaction, draft_id, user_id).await?;
    if draft.recipe_id != Some(recipe_id) {
        return Err(DbError::DraftNotFound(draft_id));
    }
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;
    let expected = draft.base_version_number.unwrap_or(0);
    if expected != latest.version_number {
        return Err(DbError::VersionConflict {
            recipe_id,
            expected,
            actual: latest.version_number,
        });
    }

    let payload = draft.decoded_snapshot()?;
    let snapshot = normalized_snapshot(&payload.content);
    validate_snapshot_content_for_publish(&snapshot)?;
    insert_recipe_version(
        &mut *transaction,
        recipe_id,
        latest.version_number + 1,
        &snapshot,
        notes,
    )
    .await?;
    let recipe =
        update_recipe_from_snapshot(&mut transaction, recipe_id, user_id, &snapshot).await?;
    sqlx::query("DELETE FROM recipe_drafts WHERE id = $1")
        .bind(draft_id)
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

pub async fn publish_snapshot(
    pool: &PgPool,
    recipe_id: Uuid,
    user_id: Uuid,
    expected_latest_version: i32,
    snapshot: &RecipeSnapshot,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let snapshot = normalized_snapshot(snapshot);
    validate_snapshot_content_for_publish(&snapshot)?;
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let owner = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM recipes WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if owner.is_none() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;
    if latest.version_number != expected_latest_version {
        return Err(DbError::VersionConflict {
            recipe_id,
            expected: expected_latest_version,
            actual: latest.version_number,
        });
    }
    insert_recipe_version(
        &mut *transaction,
        recipe_id,
        latest.version_number + 1,
        &snapshot,
        notes,
    )
    .await?;
    let recipe =
        update_recipe_from_snapshot(&mut transaction, recipe_id, user_id, &snapshot).await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

pub async fn restore_recipe_version(
    pool: &PgPool,
    recipe_id: Uuid,
    user_id: Uuid,
    version_number: i32,
    expected_latest_version: i32,
) -> Result<Recipe, DbError> {
    let target = get_recipe_version(pool, recipe_id, version_number).await?;
    let snapshot = target.decoded_snapshot()?;
    publish_snapshot(
        pool,
        recipe_id,
        user_id,
        expected_latest_version,
        &snapshot,
        Some(&format!("Restored from v{version_number}")),
    )
    .await
}

pub async fn create_fork_draft(
    pool: &PgPool,
    source_recipe_id: Uuid,
    user_id: Uuid,
    message: Option<&str>,
) -> Result<RecipeDraft, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let locked_source =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM recipes WHERE id = $1 FOR SHARE")
            .bind(source_recipe_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DbError::Query)?;
    if locked_source.is_none() {
        return Err(DbError::ForkError("source recipe not found".to_string()));
    }
    let source = super::get_recipe_by_id(&mut *transaction, source_recipe_id)
        .await?
        .ok_or_else(|| DbError::ForkError("source recipe not found".to_string()))?;
    if source.user_id != user_id && !matches!(source.visibility.as_str(), "public" | "unlisted") {
        return Err(DbError::ForkError(
            "private recipes can only be forked by their owner".to_string(),
        ));
    }
    let latest = get_latest_recipe_version(&mut *transaction, source_recipe_id).await?;
    let snapshot = RecipeDraftPayload {
        content: latest.decoded_snapshot()?,
        initial_visibility: Some(source.visibility.clone()),
    };
    let draft = insert_recipe_draft_with_source(
        &mut *transaction,
        user_id,
        None,
        None,
        &snapshot,
        Some(source_recipe_id),
        Some(latest.version_number),
        Some(&source.title),
        message,
    )
    .await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(draft)
}

pub async fn get_fork_relationship(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    forked_recipe_id: Uuid,
) -> Result<Option<ForkRelationship>, DbError> {
    sqlx::query_as::<_, ForkRelationship>(
        "SELECT id, original_recipe_id, forked_recipe_id, forked_by, forked_version_number, \
         original_title, message, created_at FROM fork_relationships \
         WHERE forked_recipe_id = $1 OR forked_recipe_key = $1 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(forked_recipe_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

const LINEAGE_EDGE_COLUMNS: &str = "id, parent_relationship_id, original_recipe_id, \
    original_recipe_key, forked_recipe_id, forked_recipe_key, forked_version_number, \
    original_title, forked_title, original_owner_id, original_owner_username, \
    original_visibility, forked_owner_id, forked_owner_username, forked_visibility, \
    message, created_at, depth";
const MAX_ANCESTOR_DEPTH: i32 = 32;
const MAX_DESCENDANT_DEPTH: i32 = 8;
const MAX_DESCENDANT_EDGES: usize = 250;
const DESCENDANT_EDGE_QUERY_LIMIT: usize = MAX_DESCENDANT_EDGES + 1;

impl LineageEdgeRow {
    fn original_snapshot(&self, incoming_edge: Option<&Self>) -> LineageNodeSnapshot {
        LineageNodeSnapshot {
            recipe_key: self.original_recipe_key.or(self.original_recipe_id),
            title: self.original_title.clone(),
            owner_id: self.original_owner_id,
            owner_username: self.original_owner_username.clone(),
            visibility: self.original_visibility.clone(),
            forked_from_version_number: incoming_edge.and_then(|edge| edge.forked_version_number),
            fork_message: incoming_edge.and_then(|edge| edge.message.clone()),
        }
    }

    fn forked_snapshot(&self) -> LineageNodeSnapshot {
        LineageNodeSnapshot {
            recipe_key: self.forked_recipe_key.or(self.forked_recipe_id),
            title: self
                .forked_title
                .clone()
                .unwrap_or_else(|| "Deleted recipe".to_string()),
            owner_id: self.forked_owner_id,
            owner_username: self.forked_owner_username.clone(),
            visibility: self.forked_visibility.clone(),
            forked_from_version_number: self.forked_version_number,
            fork_message: self.message.clone(),
        }
    }
}

fn lineage_node_from_snapshot(
    snapshot: LineageNodeSnapshot,
    live_nodes: &HashMap<Uuid, LiveLineageNode>,
    viewer_id: Option<Uuid>,
    is_current: bool,
) -> crate::types::RecipeLineageNode {
    let live_node = snapshot
        .recipe_key
        .and_then(|recipe_key| live_nodes.get(&recipe_key));
    let (is_accessible, is_deleted, recipe_id, title, owner_username, image_url) =
        if let Some(live_node) = live_node {
            let is_owner = viewer_id == Some(live_node.user_id);
            let is_accessible = is_owner
                || live_node.visibility == "public"
                || (is_current && live_node.visibility == "unlisted");
            (
                is_accessible,
                false,
                is_accessible.then_some(live_node.id),
                if is_accessible {
                    live_node.title.clone()
                } else {
                    "Private recipe".to_string()
                },
                is_accessible.then(|| live_node.owner_username.clone()),
                is_accessible.then(|| live_node.image_url.clone()).flatten(),
            )
        } else {
            let is_owner = snapshot.owner_id.is_some() && snapshot.owner_id == viewer_id;
            let is_accessible = snapshot.visibility.as_deref() == Some("public") || is_owner;
            (
                is_accessible,
                true,
                None,
                if is_accessible {
                    snapshot.title.clone()
                } else {
                    "Private recipe".to_string()
                },
                is_accessible
                    .then(|| snapshot.owner_username.clone())
                    .flatten(),
                None,
            )
        };

    crate::types::RecipeLineageNode {
        recipe_id,
        title,
        owner_username,
        image_url,
        forked_from_version_number: is_accessible
            .then_some(snapshot.forked_from_version_number)
            .flatten(),
        fork_message: is_accessible.then_some(snapshot.fork_message).flatten(),
        is_deleted,
        is_redacted: !is_accessible,
    }
}

fn build_descendant_branch(
    edge: &LineageEdgeRow,
    children_by_parent: &HashMap<Uuid, Vec<&LineageEdgeRow>>,
    live_nodes: &HashMap<Uuid, LiveLineageNode>,
    viewer_id: Option<Uuid>,
    requested_depth: i32,
    visited: &mut HashSet<Uuid>,
) -> crate::types::RecipeLineageBranch {
    if !visited.insert(edge.id) {
        return crate::types::RecipeLineageBranch {
            node: lineage_node_from_snapshot(edge.forked_snapshot(), live_nodes, viewer_id, false),
            children: Vec::new(),
            truncated: true,
        };
    }

    let child_edges = children_by_parent
        .get(&edge.id)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let (children, truncated) = if edge.depth >= requested_depth {
        (Vec::new(), !child_edges.is_empty())
    } else {
        let children = child_edges
            .iter()
            .filter(|child| child.depth <= requested_depth)
            .map(|child| {
                build_descendant_branch(
                    child,
                    children_by_parent,
                    live_nodes,
                    viewer_id,
                    requested_depth,
                    visited,
                )
            })
            .collect();
        (children, false)
    };
    visited.remove(&edge.id);

    crate::types::RecipeLineageBranch {
        node: lineage_node_from_snapshot(edge.forked_snapshot(), live_nodes, viewer_id, false),
        children,
        truncated,
    }
}

fn branch_is_truncated(branch: &crate::types::RecipeLineageBranch) -> bool {
    branch.truncated || branch.children.iter().any(branch_is_truncated)
}

/// Return the privacy-filtered lineage surrounding a recipe.
///
/// Immutable endpoint keys and relationship-to-relationship parent links keep
/// the tree connected after live recipes or users have been deleted.
pub async fn get_recipe_lineage(
    pool: &PgPool,
    recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    descendant_depth: i32,
) -> Result<crate::types::RecipeLineageView, DbError> {
    let current_live = sqlx::query_as::<_, LiveLineageNode>(
        "SELECT recipes.id, recipes.user_id, recipes.title, recipes.visibility, \
             users.username AS owner_username, \
             COALESCE(recipes.images -> 0 ->> 'thumb_url', \
                      recipes.images -> 0 ->> 'url', \
                      recipes.images ->> 0) AS image_url \
         FROM recipes JOIN users ON users.id = recipes.user_id \
         WHERE recipes.id = $1",
    )
    .bind(recipe_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;

    if current_live
        .as_ref()
        .is_some_and(|recipe| recipe.visibility == "private" && viewer_id != Some(recipe.user_id))
    {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    let ancestor_edges = sqlx::query_as::<_, LineageEdgeRow>(&format!(
        "WITH RECURSIVE ancestors AS (\
             SELECT relationship.*, 1::INTEGER AS depth, ARRAY[relationship.id] AS visited \
             FROM fork_relationships AS relationship \
             WHERE relationship.forked_recipe_key = $1 \
                OR (relationship.forked_recipe_key IS NULL \
                    AND relationship.forked_recipe_id = $1) \
             UNION ALL \
             SELECT parent.*, child.depth + 1, child.visited || parent.id \
             FROM ancestors AS child \
             JOIN fork_relationships AS parent \
               ON parent.id = child.parent_relationship_id \
             WHERE child.depth < {MAX_ANCESTOR_DEPTH} \
               AND NOT parent.id = ANY(child.visited)\
         ) \
         SELECT {LINEAGE_EDGE_COLUMNS} FROM ancestors ORDER BY depth DESC"
    ))
    .bind(recipe_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;

    let requested_depth = descendant_depth.clamp(0, MAX_DESCENDANT_DEPTH);
    let mut descendant_edges = sqlx::query_as::<_, LineageEdgeRow>(&format!(
        "WITH RECURSIVE descendants AS (\
             SELECT relationship.*, 1::INTEGER AS depth, ARRAY[relationship.id] AS visited \
             FROM fork_relationships AS relationship \
             WHERE relationship.original_recipe_key = $1 \
                OR (relationship.original_recipe_key IS NULL \
                    AND relationship.original_recipe_id = $1) \
             UNION ALL \
             SELECT child.*, parent.depth + 1, parent.visited || child.id \
             FROM descendants AS parent \
             JOIN fork_relationships AS child \
               ON child.parent_relationship_id = parent.id \
             WHERE parent.depth < $2 \
               AND NOT child.id = ANY(parent.visited)\
         ) \
         SELECT {LINEAGE_EDGE_COLUMNS} FROM descendants \
         LIMIT {DESCENDANT_EDGE_QUERY_LIMIT}"
    ))
    .bind(recipe_id)
    .bind((requested_depth + 1).max(1))
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    let node_limit_reached = descendant_edges.len() > MAX_DESCENDANT_EDGES;
    descendant_edges.truncate(MAX_DESCENDANT_EDGES);
    descendant_edges.sort_by(|left, right| {
        left.depth
            .cmp(&right.depth)
            .then_with(|| left.created_at.cmp(&right.created_at))
            .then_with(|| left.id.cmp(&right.id))
    });

    if current_live.is_none() && ancestor_edges.is_empty() && descendant_edges.is_empty() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    let mut recipe_keys = HashSet::from([recipe_id]);
    for edge in ancestor_edges.iter().chain(&descendant_edges) {
        recipe_keys.extend(edge.original_recipe_key.or(edge.original_recipe_id));
        recipe_keys.extend(edge.forked_recipe_key.or(edge.forked_recipe_id));
    }
    let recipe_keys: Vec<Uuid> = recipe_keys.into_iter().collect();
    let live_nodes = sqlx::query_as::<_, LiveLineageNode>(
        "SELECT recipes.id, recipes.user_id, recipes.title, recipes.visibility, \
             users.username AS owner_username, \
             COALESCE(recipes.images -> 0 ->> 'thumb_url', \
                      recipes.images -> 0 ->> 'url', \
                      recipes.images ->> 0) AS image_url \
         FROM recipes JOIN users ON users.id = recipes.user_id \
         WHERE recipes.id = ANY($1)",
    )
    .bind(&recipe_keys)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?
    .into_iter()
    .map(|node| (node.id, node))
    .collect::<HashMap<_, _>>();

    let ancestor_by_id = ancestor_edges
        .iter()
        .map(|edge| (edge.id, edge))
        .collect::<HashMap<_, _>>();
    let ancestors = ancestor_edges
        .iter()
        .map(|edge| {
            let incoming_edge = edge
                .parent_relationship_id
                .and_then(|parent_id| ancestor_by_id.get(&parent_id).copied());
            lineage_node_from_snapshot(
                edge.original_snapshot(incoming_edge),
                &live_nodes,
                viewer_id,
                false,
            )
        })
        .collect::<Vec<_>>();

    let current_incoming_edge = ancestor_edges.iter().find(|edge| edge.depth == 1);
    let current_snapshot = current_incoming_edge
        .map(LineageEdgeRow::forked_snapshot)
        .or_else(|| {
            descendant_edges
                .iter()
                .find(|edge| edge.depth == 1)
                .map(|edge| edge.original_snapshot(None))
        })
        .or_else(|| {
            current_live.as_ref().map(|recipe| LineageNodeSnapshot {
                recipe_key: Some(recipe.id),
                title: recipe.title.clone(),
                owner_id: Some(recipe.user_id),
                owner_username: Some(recipe.owner_username.clone()),
                visibility: Some(recipe.visibility.clone()),
                forked_from_version_number: None,
                fork_message: None,
            })
        })
        .ok_or(DbError::RecipeNotFound(recipe_id))?;
    let current = lineage_node_from_snapshot(current_snapshot, &live_nodes, viewer_id, true);
    if current.is_redacted {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    let mut children_by_parent: HashMap<Uuid, Vec<&LineageEdgeRow>> = HashMap::new();
    for edge in &descendant_edges {
        if let Some(parent_relationship_id) = edge.parent_relationship_id {
            children_by_parent
                .entry(parent_relationship_id)
                .or_default()
                .push(edge);
        }
    }
    let descendants = if requested_depth == 0 {
        Vec::new()
    } else {
        descendant_edges
            .iter()
            .filter(|edge| edge.depth == 1)
            .map(|edge| {
                build_descendant_branch(
                    edge,
                    &children_by_parent,
                    &live_nodes,
                    viewer_id,
                    requested_depth,
                    &mut HashSet::new(),
                )
            })
            .collect::<Vec<_>>()
    };
    let truncated = node_limit_reached
        || (requested_depth == 0 && !descendant_edges.is_empty())
        || descendants.iter().any(branch_is_truncated);

    Ok(crate::types::RecipeLineageView {
        ancestors,
        current,
        descendants,
        truncated,
        node_limit_reached,
    })
}

pub async fn count_recipe_versions_for_draft(
    pool: &PgPool,
    draft_id: Uuid,
) -> Result<i64, DbError> {
    sqlx::query_scalar(
        "SELECT COUNT(recipe_versions.id) FROM recipe_drafts \
         LEFT JOIN recipe_versions ON recipe_versions.recipe_id = recipe_drafts.recipe_id \
         WHERE recipe_drafts.id = $1",
    )
    .bind(draft_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Query)
}

pub fn validate_snapshot_for_publish(
    snapshot: &RecipeSnapshot,
    visibility: &str,
) -> Result<(), DbError> {
    validate_snapshot_content_for_publish(snapshot)?;
    validate_recipe_visibility(visibility)
}

pub fn validate_recipe_visibility(visibility: &str) -> Result<(), DbError> {
    if !matches!(visibility, "private" | "unlisted" | "public") {
        return Err(DbError::InvalidRecipeSnapshot(
            "visibility must be private, unlisted, or public".to_string(),
        ));
    }
    Ok(())
}

fn validate_snapshot_content_for_publish(snapshot: &RecipeSnapshot) -> Result<(), DbError> {
    if snapshot.title.trim().is_empty() {
        return Err(DbError::InvalidRecipeSnapshot(
            "title is required".to_string(),
        ));
    }
    if snapshot.ingredients.is_empty()
        || snapshot
            .ingredients
            .iter()
            .all(|ingredient| ingredient.name.trim().is_empty())
    {
        return Err(DbError::InvalidRecipeSnapshot(
            "at least one ingredient is required".to_string(),
        ));
    }
    if snapshot.instructions.is_empty() || !contains_step_text(&snapshot.instructions) {
        return Err(DbError::InvalidRecipeSnapshot(
            "at least one instruction is required".to_string(),
        ));
    }
    if snapshot.prep_time_minutes.is_some_and(|value| value < 0)
        || snapshot.cook_time_minutes.is_some_and(|value| value < 0)
    {
        return Err(DbError::InvalidRecipeSnapshot(
            "recipe times cannot be negative".to_string(),
        ));
    }
    if snapshot.servings.is_some_and(|value| value <= 0) {
        return Err(DbError::InvalidRecipeSnapshot(
            "servings must be positive".to_string(),
        ));
    }
    Ok(())
}

fn contains_step_text(steps: &[crate::types::RecipeStep]) -> bool {
    steps
        .iter()
        .any(|step| !step.text.trim().is_empty() || contains_step_text(&step.sub_steps))
}

fn normalized_snapshot(snapshot: &RecipeSnapshot) -> RecipeSnapshot {
    let mut snapshot = snapshot.clone();
    snapshot.title = snapshot.title.trim().to_string();
    snapshot.tags = snapshot
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    snapshot.tags.sort();
    snapshot.tags.dedup();
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diff::snapshot_to_json;
    use crate::db::{insert_user, DbError};
    use crate::test_utils;

    async fn owner(pool: &sqlx::PgPool, prefix: &str) -> crate::db::User {
        let suffix = test_utils::uid();
        insert_user(
            pool,
            &format!("{prefix}_{suffix}"),
            "Version Owner",
            &format!("{prefix}_{suffix}@example.test"),
            None,
        )
        .await
        .unwrap()
    }

    fn snapshot(title: &str, visibility: &str) -> crate::db::diff::RecipeDraftPayload {
        crate::db::diff::RecipeDraftPayload {
            initial_visibility: Some(visibility.to_string()),
            content: crate::db::diff::RecipeSnapshot {
                title: title.to_string(),
                description: Some("description".to_string()),
                commentary: Some("commentary".to_string()),
                prep_time_minutes: Some(5),
                cook_time_minutes: Some(15),
                servings: Some(2),
                ingredients: vec![crate::types::RecipeIngredient {
                    amount: "1".into(),
                    unit: "cup".into(),
                    name: "beans".into(),
                }],
                instructions: vec![crate::types::RecipeStep {
                    text: "Cook".into(),
                    sub_steps: vec![],
                    photos: vec![],
                }],
                equipment: vec![crate::types::RecipeEquipment { name: "Pot".into() }],
                images: vec![],
                tags: vec!["quick".into(), "vegan".into()],
            },
        }
    }

    async fn publish_fork(
        pool: &sqlx::PgPool,
        source_recipe_id: Uuid,
        fork_owner: &crate::db::User,
        title: &str,
    ) -> Recipe {
        publish_fork_with_visibility(pool, source_recipe_id, fork_owner, title, "public").await
    }

    async fn publish_fork_with_visibility(
        pool: &sqlx::PgPool,
        source_recipe_id: Uuid,
        fork_owner: &crate::db::User,
        title: &str,
        visibility: &str,
    ) -> Recipe {
        let draft = create_fork_draft(pool, source_recipe_id, fork_owner.id, None)
            .await
            .unwrap();
        update_recipe_draft(
            pool,
            draft.id,
            fork_owner.id,
            draft.revision,
            &snapshot(title, visibility),
        )
        .await
        .unwrap();
        publish_recipe_draft(pool, draft.id, fork_owner.id, None)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn publishing_new_draft_creates_recipe_v1_tags_and_deletes_draft() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "newdraft").await;
        let draft = create_new_recipe_draft(&pool, user.id, &snapshot("First", "public"))
            .await
            .unwrap();

        assert_eq!(
            count_recipe_versions_for_draft(&pool, draft.id)
                .await
                .unwrap(),
            0
        );

        let published = publish_recipe_draft(&pool, draft.id, user.id, Some("Initial"))
            .await
            .unwrap();
        assert_eq!(published.title, "First");
        let versions = get_recipe_versions(&pool, published.id).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_number, 1);
        assert_eq!(
            versions[0].snapshot,
            snapshot_to_json(&snapshot("First", "public")).unwrap()
        );
        assert_eq!(
            crate::db::get_recipe_tags(&pool, published.id)
                .await
                .unwrap()
                .len(),
            2
        );
        assert!(matches!(
            get_recipe_draft(&pool, draft.id, user.id).await,
            Err(DbError::DraftNotFound(id)) if id == draft.id
        ));
    }

    #[tokio::test]
    async fn autosave_is_owner_only_revision_guarded_and_never_versions_or_publishes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "autosave").await;
        let intruder = owner(&pool, "intruder").await;
        let published =
            create_published_recipe(&pool, user.id, &snapshot("Published", "public"), None)
                .await
                .unwrap();
        assert!(matches!(
            insert_recipe_draft(
                &pool,
                intruder.id,
                Some(published.id),
                Some(1),
                &snapshot("Unauthorized", "private"),
            )
            .await,
            Err(DbError::RecipeNotFound(id)) if id == published.id
        ));
        let draft = create_edit_draft(&pool, published.id, user.id)
            .await
            .unwrap();
        let changed = snapshot("Draft only", "private");

        let saved = update_recipe_draft(&pool, draft.id, user.id, 0, &changed)
            .await
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert!(matches!(
            update_recipe_draft(&pool, draft.id, user.id, 0, &changed).await,
            Err(DbError::DraftConflict {
                expected: 0,
                actual: 1,
                ..
            })
        ));
        assert!(matches!(
            get_recipe_draft(&pool, draft.id, intruder.id).await,
            Err(DbError::DraftNotFound(_))
        ));
        assert_eq!(
            get_recipe_versions(&pool, published.id)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            crate::db::get_recipe_by_id(&pool, published.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "Published"
        );
    }

    #[tokio::test]
    async fn publishing_existing_draft_creates_exactly_one_next_full_version() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "existing").await;
        let published = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, published.id, user.id)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            draft.id,
            user.id,
            draft.revision,
            &snapshot("V2", "public"),
        )
        .await
        .unwrap();

        let result = publish_recipe_draft(&pool, draft.id, user.id, Some("Published update"))
            .await
            .unwrap();
        assert_eq!(result.title, "V2");
        let versions = get_recipe_versions(&pool, published.id).await.unwrap();
        assert_eq!(
            versions
                .iter()
                .map(|v| v.version_number)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(
            versions[0].snapshot,
            snapshot_to_json(&snapshot("V2", "public")).unwrap()
        );
        assert_eq!(
            versions[1].snapshot,
            snapshot_to_json(&snapshot("V1", "private")).unwrap()
        );
    }

    #[tokio::test]
    async fn publishing_existing_draft_preserves_current_recipe_visibility() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "draft_visibility").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("Original", "public"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();

        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(recipe.id)
            .execute(&pool)
            .await
            .unwrap();

        let published = publish_recipe_draft(&pool, draft.id, user.id, None)
            .await
            .unwrap();

        assert_eq!(published.visibility, "private");
    }

    #[tokio::test]
    async fn version_graph_connects_direct_forks_to_the_recorded_source_snapshot() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "graph_source").await;
        let fork_owner = owner(&pool, "graph_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Source v1", "public"),
            None,
        )
        .await
        .unwrap();
        publish_snapshot(
            &pool,
            source.id,
            source_owner.id,
            1,
            &snapshot("Source v2", "public").content,
            None,
        )
        .await
        .unwrap();
        let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        let fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
            .await
            .unwrap();
        publish_snapshot(
            &pool,
            fork.id,
            fork_owner.id,
            1,
            &snapshot("Fork v2", "public").content,
            None,
        )
        .await
        .unwrap();

        let source_graph = get_recipe_version_graph(&pool, source.id, None)
            .await
            .unwrap();
        let child_graph = get_recipe_version_graph(&pool, fork.id, None)
            .await
            .unwrap();

        for graph in [&source_graph, &child_graph] {
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .filter(|node| node.id().recipe_id == source.id)
                    .count(),
                2
            );
            assert_eq!(
                graph
                    .nodes
                    .iter()
                    .filter(|node| node.id().recipe_id == fork.id)
                    .count(),
                2
            );
            let fork_edge = graph
                .edges
                .iter()
                .find(|edge| edge.kind() == crate::types::VersionGraphEdgeKind::Fork)
                .expect("direct fork edge");
            assert_eq!(
                fork_edge.source(),
                crate::types::VersionGraphNodeId {
                    recipe_id: source.id,
                    version_number: 2,
                }
            );
            assert_eq!(
                fork_edge.target(),
                crate::types::VersionGraphNodeId {
                    recipe_id: fork.id,
                    version_number: 1,
                }
            );
        }
    }

    #[tokio::test]
    async fn version_graph_redacts_non_public_direct_forks_for_non_owners() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "graph_public").await;
        let fork_owner = owner(&pool, "graph_private").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Public source", "public"),
            None,
        )
        .await
        .unwrap();
        let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            fork_draft.id,
            fork_owner.id,
            fork_draft.revision,
            &snapshot("Secret fork", "private"),
        )
        .await
        .unwrap();
        let private_fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
            .await
            .unwrap();
        let unlisted_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            unlisted_draft.id,
            fork_owner.id,
            unlisted_draft.revision,
            &snapshot("Unlisted fork", "unlisted"),
        )
        .await
        .unwrap();
        let unlisted_fork = publish_recipe_draft(&pool, unlisted_draft.id, fork_owner.id, None)
            .await
            .unwrap();

        let public_graph = get_recipe_version_graph(&pool, source.id, None)
            .await
            .unwrap();
        for inaccessible_recipe_id in [private_fork.id, unlisted_fork.id] {
            let inaccessible_node = public_graph
                .nodes
                .iter()
                .find(|node| node.id().recipe_id == inaccessible_recipe_id)
                .expect("redacted non-public fork endpoint");
            assert!(inaccessible_node.is_redacted());
            assert_eq!(inaccessible_node.title(), None);
            assert_eq!(inaccessible_node.owner_username(), None);
            assert_eq!(inaccessible_node.image(), None);
            assert!(public_graph.edges.iter().any(|edge| {
                edge.kind() == crate::types::VersionGraphEdgeKind::Fork
                    && edge.target().recipe_id == inaccessible_recipe_id
            }));
        }

        let owner_graph = get_recipe_version_graph(&pool, source.id, Some(fork_owner.id))
            .await
            .unwrap();
        let owner_node = owner_graph
            .nodes
            .iter()
            .find(|node| node.id().recipe_id == private_fork.id)
            .expect("owner-visible private fork");
        assert!(!owner_node.is_redacted());
        assert_eq!(owner_node.title(), Some("Secret fork"));
        let unlisted_owner_node = owner_graph
            .nodes
            .iter()
            .find(|node| node.id().recipe_id == unlisted_fork.id)
            .expect("owner-visible unlisted fork");
        assert!(!unlisted_owner_node.is_redacted());
        assert_eq!(unlisted_owner_node.title(), Some("Unlisted fork"));
    }

    #[tokio::test]
    async fn version_graph_traverses_hidden_branches_without_exposing_their_metadata() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let root_owner = owner(&pool, "graph_hidden_root").await;
        let branch_owner = owner(&pool, "graph_hidden_branch").await;
        let root = create_published_recipe(
            &pool,
            root_owner.id,
            &snapshot("Public root", "public"),
            None,
        )
        .await
        .unwrap();
        let hidden_branch = publish_fork_with_visibility(
            &pool,
            root.id,
            &branch_owner,
            "Private intermediate",
            "private",
        )
        .await;
        let public_descendant =
            publish_fork(&pool, hidden_branch.id, &branch_owner, "Public descendant").await;

        let graph = get_recipe_version_graph(&pool, root.id, None)
            .await
            .unwrap();
        let hidden_nodes = graph
            .nodes
            .iter()
            .filter(|node| node.id().recipe_id == hidden_branch.id)
            .collect::<Vec<_>>();
        let public_node = graph
            .nodes
            .iter()
            .find(|node| node.id().recipe_id == public_descendant.id)
            .expect("public recipe behind hidden branch");

        assert!(!hidden_nodes.is_empty());
        assert!(hidden_nodes.iter().all(|node| node.is_redacted()));
        assert!(hidden_nodes.iter().all(|node| node.title().is_none()));
        assert!(hidden_nodes
            .iter()
            .all(|node| node.owner_username().is_none()));
        assert!(hidden_nodes.iter().all(|node| node.image().is_none()));
        assert_eq!(public_node.title(), Some("Public descendant"));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind() == crate::types::VersionGraphEdgeKind::Fork
                && edge.source().recipe_id == hidden_branch.id
                && edge.target().recipe_id == public_descendant.id
        }));
    }

    #[tokio::test]
    async fn version_graph_keeps_direct_fork_topology_after_either_recipe_is_deleted() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "graph_del_src").await;
        let fork_owner = owner(&pool, "graph_del_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Source v1", "public"),
            None,
        )
        .await
        .unwrap();
        publish_snapshot(
            &pool,
            source.id,
            source_owner.id,
            1,
            &snapshot("Source v2", "public").content,
            None,
        )
        .await
        .unwrap();
        let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        let fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
            .await
            .unwrap();

        crate::db::delete_recipe(&pool, source.id, source_owner.id)
            .await
            .unwrap();
        let child_graph = get_recipe_version_graph(&pool, fork.id, None)
            .await
            .unwrap();
        let deleted_parent = child_graph
            .nodes
            .iter()
            .find(|node| {
                node.id()
                    == (crate::types::VersionGraphNodeId {
                        recipe_id: source.id,
                        version_number: 2,
                    })
            })
            .expect("deleted parent fork endpoint");
        assert!(deleted_parent.is_deleted());
        assert!(child_graph.edges.iter().any(|edge| {
            edge.kind() == crate::types::VersionGraphEdgeKind::Fork
                && edge.source() == deleted_parent.id()
                && edge.target().recipe_id == fork.id
        }));

        let second_source_owner = owner(&pool, "graph_del2_src").await;
        let second_fork_owner = owner(&pool, "graph_del2_fork").await;
        let second_source = create_published_recipe(
            &pool,
            second_source_owner.id,
            &snapshot("Second source", "public"),
            None,
        )
        .await
        .unwrap();
        let second_fork_draft =
            create_fork_draft(&pool, second_source.id, second_fork_owner.id, None)
                .await
                .unwrap();
        let second_fork =
            publish_recipe_draft(&pool, second_fork_draft.id, second_fork_owner.id, None)
                .await
                .unwrap();
        crate::db::delete_recipe(&pool, second_fork.id, second_fork_owner.id)
            .await
            .unwrap();

        let source_graph = get_recipe_version_graph(&pool, second_source.id, None)
            .await
            .unwrap();
        let deleted_child = source_graph
            .nodes
            .iter()
            .find(|node| {
                node.id()
                    == (crate::types::VersionGraphNodeId {
                        recipe_id: second_fork.id,
                        version_number: 1,
                    })
            })
            .expect("deleted child fork endpoint");
        assert!(deleted_child.is_deleted());
        assert!(source_graph.edges.iter().any(|edge| {
            edge.kind() == crate::types::VersionGraphEdgeKind::Fork
                && edge.source().recipe_id == second_source.id
                && edge.target() == deleted_child.id()
        }));
    }

    #[tokio::test]
    async fn version_graph_includes_sibling_and_cousin_branches() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let graph_owner = owner(&pool, "graph_family").await;
        let root =
            create_published_recipe(&pool, graph_owner.id, &snapshot("Root", "public"), None)
                .await
                .unwrap();
        let left_branch = publish_fork(&pool, root.id, &graph_owner, "Left branch").await;
        let right_branch = publish_fork(&pool, root.id, &graph_owner, "Right branch").await;
        let selected = publish_fork(&pool, left_branch.id, &graph_owner, "Selected leaf").await;
        let cousin = publish_fork(&pool, right_branch.id, &graph_owner, "Cousin leaf").await;

        let graph = get_recipe_version_graph(&pool, selected.id, None)
            .await
            .unwrap();
        let recipe_ids = graph
            .nodes
            .iter()
            .map(|node| node.id().recipe_id)
            .collect::<HashSet<_>>();

        assert_eq!(
            recipe_ids,
            HashSet::from([
                root.id,
                left_branch.id,
                right_branch.id,
                selected.id,
                cousin.id,
            ])
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.kind() == crate::types::VersionGraphEdgeKind::Fork)
                .count(),
            4
        );
        assert!(!graph.truncation.is_truncated);
    }

    #[tokio::test]
    async fn version_graph_stops_malformed_cycles_without_duplicate_topology() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let graph_owner = owner(&pool, "graph_cycle").await;
        let root = create_published_recipe(
            &pool,
            graph_owner.id,
            &snapshot("Cycle root", "public"),
            None,
        )
        .await
        .unwrap();
        let child = publish_fork(&pool, root.id, &graph_owner, "Cycle child").await;
        sqlx::query(
            "INSERT INTO fork_relationships (\
                 original_recipe_id, forked_recipe_id, original_recipe_key, forked_recipe_key, \
                 forked_version_number, original_title, forked_title, original_owner_id, \
                 original_owner_username, original_visibility, forked_owner_id, \
                 forked_owner_username, forked_visibility\
             ) VALUES ($1, $2, $1, $2, 1, $3, $4, $5, $6, 'public', $5, $6, 'public')",
        )
        .bind(child.id)
        .bind(root.id)
        .bind(&child.title)
        .bind(&root.title)
        .bind(graph_owner.id)
        .bind(&graph_owner.username)
        .execute(&pool)
        .await
        .unwrap();

        let graph = get_recipe_version_graph(&pool, root.id, None)
            .await
            .unwrap();
        let node_ids = graph
            .nodes
            .iter()
            .map(|node| node.id())
            .collect::<HashSet<_>>();
        let fork_edges = graph
            .edges
            .iter()
            .filter(|edge| edge.kind() == crate::types::VersionGraphEdgeKind::Fork)
            .collect::<Vec<_>>();

        assert_eq!(node_ids.len(), graph.nodes.len());
        assert_eq!(node_ids.len(), 2);
        assert_eq!(fork_edges.len(), 2);
        assert!(!graph.truncation.is_truncated);
    }

    #[tokio::test]
    async fn version_graph_reports_depth_and_traversal_limits_independently() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let graph_owner = owner(&pool, "graph_walk_limits").await;
        let root = create_published_recipe(
            &pool,
            graph_owner.id,
            &snapshot("Limit root", "public"),
            None,
        )
        .await
        .unwrap();
        let child = publish_fork(&pool, root.id, &graph_owner, "Limit child").await;
        let grandchild = publish_fork(&pool, child.id, &graph_owner, "Limit grandchild").await;
        let sibling = publish_fork(&pool, root.id, &graph_owner, "Limit sibling").await;

        let depth_limited = get_recipe_version_graph_with_limits(
            &pool,
            root.id,
            None,
            VersionGraphLimits {
                max_depth: 1,
                ..VersionGraphLimits::default()
            },
        )
        .await
        .unwrap();
        assert!(depth_limited
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == child.id));
        assert!(!depth_limited
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == grandchild.id));
        assert!(depth_limited.truncation.is_truncated);
        assert!(depth_limited.truncation.depth_limit_reached);
        assert!(!depth_limited.truncation.traversal_limit_reached);

        let traversal_limited = get_recipe_version_graph_with_limits(
            &pool,
            root.id,
            None,
            VersionGraphLimits {
                max_relationships: 1,
                ..VersionGraphLimits::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(
            traversal_limited
                .edges
                .iter()
                .filter(|edge| edge.kind() == crate::types::VersionGraphEdgeKind::Fork)
                .count(),
            1
        );
        assert!(traversal_limited.truncation.is_truncated);
        assert!(traversal_limited.truncation.traversal_limit_reached);
        assert!(!traversal_limited.truncation.depth_limit_reached);
        assert!(traversal_limited
            .nodes
            .iter()
            .any(|node| { [child.id, sibling.id].contains(&node.id().recipe_id) }));
    }

    #[tokio::test]
    async fn version_graph_reports_node_and_edge_limits_independently() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let graph_owner = owner(&pool, "graph_shape_limits").await;
        let root = create_published_recipe(
            &pool,
            graph_owner.id,
            &snapshot("Shape root v1", "public"),
            None,
        )
        .await
        .unwrap();
        publish_snapshot(
            &pool,
            root.id,
            graph_owner.id,
            1,
            &snapshot("Shape root v2", "public").content,
            None,
        )
        .await
        .unwrap();
        let child = publish_fork(&pool, root.id, &graph_owner, "Shape child").await;

        let node_limited = get_recipe_version_graph_with_limits(
            &pool,
            root.id,
            None,
            VersionGraphLimits {
                max_nodes: 2,
                ..VersionGraphLimits::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(node_limited.nodes.len(), 2);
        assert!(node_limited.truncation.is_truncated);
        assert!(node_limited.truncation.node_limit_reached);
        assert!(!node_limited.truncation.edge_limit_reached);

        let edge_limited = get_recipe_version_graph_with_limits(
            &pool,
            root.id,
            None,
            VersionGraphLimits {
                max_edges: 1,
                ..VersionGraphLimits::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(edge_limited.edges.len(), 1);
        assert!(edge_limited
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == child.id));
        assert!(edge_limited.truncation.is_truncated);
        assert!(edge_limited.truncation.edge_limit_reached);
        assert!(!edge_limited.truncation.node_limit_reached);
    }

    #[tokio::test]
    async fn stale_base_rejects_publish_and_preserves_draft_and_published_recipe() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "stale").await;
        let published = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let stale = create_edit_draft(&pool, published.id, user.id)
            .await
            .unwrap();
        sqlx::query("UPDATE recipe_drafts SET base_version_number = 99 WHERE id = $1")
            .bind(stale.id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(matches!(
            publish_recipe_draft(&pool, stale.id, user.id, None).await,
            Err(DbError::VersionConflict {
                expected: 99,
                actual: 1,
                ..
            })
        ));
        assert_eq!(
            get_recipe_draft(&pool, stale.id, user.id).await.unwrap().id,
            stale.id
        );
        assert_eq!(
            crate::db::get_recipe_by_id(&pool, published.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "V1"
        );
        assert_eq!(
            get_recipe_versions(&pool, published.id)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn restore_publishes_target_snapshot_as_new_version_without_rewriting_history() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "restore").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            1,
            &snapshot("V2", "public"),
            None,
        )
        .await
        .unwrap();

        let restored = restore_recipe_version(&pool, recipe.id, user.id, 1, 2)
            .await
            .unwrap();
        assert_eq!(restored.title, "V1");
        let versions = get_recipe_versions(&pool, recipe.id).await.unwrap();
        assert_eq!(
            versions
                .iter()
                .map(|v| v.version_number)
                .collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert_eq!(versions[0].snapshot, versions[2].snapshot);
        assert_eq!(versions[0].notes.as_deref(), Some("Restored from v1"));
    }

    #[tokio::test]
    async fn compares_any_two_immutable_recipe_versions_in_requested_direction() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "compare_versions").await;
        let recipe =
            create_published_recipe(&pool, user.id, &snapshot("Original title", "private"), None)
                .await
                .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            1,
            &snapshot("Middle title", "private"),
            None,
        )
        .await
        .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            2,
            &snapshot("Current title", "public"),
            None,
        )
        .await
        .unwrap();

        let forward = compare_recipe_versions(&pool, recipe.id, 1, 3)
            .await
            .unwrap();
        let forward_title = forward
            .iter()
            .find(|change| change.path == "/title")
            .expect("forward title change");
        assert_eq!(forward_title.old_value.as_ref().unwrap(), "Original title");
        assert_eq!(forward_title.value.as_ref().unwrap(), "Current title");

        let reverse = compare_recipe_versions(&pool, recipe.id, 3, 1)
            .await
            .unwrap();
        let reverse_title = reverse
            .iter()
            .find(|change| change.path == "/title")
            .expect("reverse title change");
        assert_eq!(reverse_title.old_value.as_ref().unwrap(), "Current title");
        assert_eq!(reverse_title.value.as_ref().unwrap(), "Original title");
    }

    #[tokio::test]
    async fn fork_policy_and_lineage_preserve_attribution_after_source_delete() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "source").await;
        let forker = owner(&pool, "forker").await;
        let source =
            create_published_recipe(&pool, source_owner.id, &snapshot("Source", "public"), None)
                .await
                .unwrap();
        let draft = create_fork_draft(&pool, source.id, forker.id, Some("My variation"))
            .await
            .unwrap();
        let fork = publish_recipe_draft(&pool, draft.id, forker.id, None)
            .await
            .unwrap();
        let relationship = get_fork_relationship(&pool, fork.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(relationship.original_recipe_id, Some(source.id));
        assert_eq!(relationship.original_title, "Source");
        assert_eq!(relationship.forked_version_number, Some(1));

        crate::db::delete_recipe(&pool, source.id, source_owner.id)
            .await
            .unwrap();
        let relationship = get_fork_relationship(&pool, fork.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(relationship.original_recipe_id, None);
        assert_eq!(relationship.original_title, "Source");
    }

    #[tokio::test]
    async fn lineage_traverses_both_directions_and_survives_deleted_intermediate_recipes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "lineage_source").await;
        let fork_owner = owner(&pool, "lineage_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Family root", "public"),
            None,
        )
        .await
        .unwrap();
        let branch_draft = create_fork_draft(&pool, source.id, fork_owner.id, Some("Less sugar"))
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            branch_draft.id,
            fork_owner.id,
            branch_draft.revision,
            &snapshot("Branch", "public"),
        )
        .await
        .unwrap();
        let branch = publish_recipe_draft(&pool, branch_draft.id, fork_owner.id, None)
            .await
            .unwrap();

        let leaf_draft = create_fork_draft(&pool, branch.id, fork_owner.id, Some("Spicier"))
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            leaf_draft.id,
            fork_owner.id,
            leaf_draft.revision,
            &snapshot("Leaf", "public"),
        )
        .await
        .unwrap();
        let leaf = publish_recipe_draft(&pool, leaf_draft.id, fork_owner.id, None)
            .await
            .unwrap();

        let from_root = get_recipe_lineage(&pool, source.id, None, 4).await.unwrap();
        assert_eq!(from_root.current.title, "Family root");
        assert_eq!(from_root.descendants[0].node.title, "Branch");
        assert_eq!(from_root.descendants[0].children[0].node.title, "Leaf");

        let from_leaf = get_recipe_lineage(&pool, leaf.id, None, 4).await.unwrap();
        assert_eq!(
            from_leaf
                .ancestors
                .iter()
                .map(|node| node.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Family root", "Branch"]
        );

        crate::db::delete_recipe(&pool, branch.id, fork_owner.id)
            .await
            .unwrap();
        let after_intermediate_delete =
            get_recipe_lineage(&pool, source.id, None, 4).await.unwrap();
        let deleted_branch = &after_intermediate_delete.descendants[0];
        assert!(deleted_branch.node.is_deleted);
        assert_eq!(deleted_branch.node.title, "Branch");
        assert_eq!(deleted_branch.children[0].node.recipe_id, Some(leaf.id));
    }

    #[tokio::test]
    async fn lineage_caps_wide_descendant_trees_and_reports_truncation() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "wide_src").await;
        let fork_owner = owner(&pool, "wide_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Wide family root", "public"),
            None,
        )
        .await
        .unwrap();

        sqlx::query(
            "WITH inserted_forks AS (\
                 INSERT INTO recipes (\
                     id, user_id, title, description, ingredients, instructions, equipment, \
                     images, visibility\
                 ) \
                 SELECT gen_random_uuid(), $1, 'Wide fork ' || fork_number, NULL, \
                        '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, 'public' \
                 FROM generate_series(1, 275) AS fork_number \
                 RETURNING id, user_id, title, visibility\
             ) \
             INSERT INTO fork_relationships (\
                 original_recipe_id, forked_recipe_id, forked_by, original_recipe_key, \
                 forked_recipe_key, forked_version_number, original_title, forked_title, \
                 original_owner_id, original_owner_username, original_visibility, \
                 forked_owner_id, forked_owner_username, forked_visibility\
             ) \
             SELECT $2, inserted.id, $1, $2, inserted.id, 1, 'Wide family root', \
                    inserted.title, $3, $4, 'public', $1, $5, inserted.visibility \
             FROM inserted_forks AS inserted",
        )
        .bind(fork_owner.id)
        .bind(source.id)
        .bind(source_owner.id)
        .bind(&source_owner.username)
        .bind(&fork_owner.username)
        .execute(&pool)
        .await
        .unwrap();

        let lineage = get_recipe_lineage(&pool, source.id, None, 2).await.unwrap();
        assert!(lineage.node_limit_reached);
        assert!(lineage.truncated);
        assert_eq!(lineage.descendants.len(), MAX_DESCENDANT_EDGES);
    }

    #[tokio::test]
    async fn lineage_redacts_non_public_descendants_for_other_viewers() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "lineage_public").await;
        let fork_owner = owner(&pool, "lineage_private").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Visible source", "public"),
            None,
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE recipes SET images = \
             '[{\"url\":\"https://images.test/source-full.webp\",\"thumb_url\":\"https://images.test/source-thumb.webp\",\"edits\":[]}]'::jsonb \
             WHERE id = $1",
        )
        .bind(source.id)
        .execute(&pool)
        .await
        .unwrap();
        let private_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            private_draft.id,
            fork_owner.id,
            private_draft.revision,
            &snapshot("Secret branch", "private"),
        )
        .await
        .unwrap();
        let private_recipe = publish_recipe_draft(&pool, private_draft.id, fork_owner.id, None)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE recipes SET images = \
             '[{\"url\":\"https://images.test/private-full.webp\",\"edits\":[]}]'::jsonb \
             WHERE id = $1",
        )
        .bind(private_recipe.id)
        .execute(&pool)
        .await
        .unwrap();

        let public_view = get_recipe_lineage(&pool, source.id, None, 2).await.unwrap();
        assert_eq!(
            public_view.current.image_url.as_deref(),
            Some("https://images.test/source-thumb.webp")
        );
        assert!(public_view.descendants[0].node.is_redacted);
        assert_eq!(public_view.descendants[0].node.title, "Private recipe");
        assert_eq!(public_view.descendants[0].node.recipe_id, None);
        assert_eq!(public_view.descendants[0].node.image_url, None);

        let owner_view = get_recipe_lineage(&pool, source.id, Some(fork_owner.id), 2)
            .await
            .unwrap();
        assert!(!owner_view.descendants[0].node.is_redacted);
        assert_eq!(owner_view.descendants[0].node.title, "Secret branch");
        assert_eq!(
            owner_view.descendants[0].node.image_url.as_deref(),
            Some("https://images.test/private-full.webp")
        );
    }

    #[tokio::test]
    async fn deleted_lineage_nodes_use_their_latest_visibility_for_privacy() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "linevis_src").await;
        let fork_owner = owner(&pool, "linevis_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Visible source", "public"),
            None,
        )
        .await
        .unwrap();
        let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            fork_draft.id,
            fork_owner.id,
            fork_draft.revision,
            &snapshot("Once-public branch", "public"),
        )
        .await
        .unwrap();
        let fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
            .await
            .unwrap();

        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(fork.id)
            .execute(&pool)
            .await
            .unwrap();
        crate::db::delete_recipe(&pool, fork.id, fork_owner.id)
            .await
            .unwrap();

        let public_view = get_recipe_lineage(&pool, source.id, None, 2).await.unwrap();
        assert!(public_view.descendants[0].node.is_deleted);
        assert!(public_view.descendants[0].node.is_redacted);
        assert_eq!(public_view.descendants[0].node.title, "Private recipe");

        let owner_view = get_recipe_lineage(&pool, source.id, Some(fork_owner.id), 2)
            .await
            .unwrap();
        assert_eq!(owner_view.descendants[0].node.title, "Once-public branch");
    }

    #[tokio::test]
    async fn account_deletion_snapshots_the_latest_lineage_privacy() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "acctdel_src").await;
        let fork_owner = owner(&pool, "acctdel_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Visible source", "public"),
            None,
        )
        .await
        .unwrap();
        let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            fork_draft.id,
            fork_owner.id,
            fork_draft.revision,
            &snapshot("Private before account deletion", "public"),
        )
        .await
        .unwrap();
        let fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
            .await
            .unwrap();
        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(fork.id)
            .execute(&pool)
            .await
            .unwrap();

        crate::db::delete_user(&pool, fork_owner.id).await.unwrap();

        let public_view = get_recipe_lineage(&pool, source.id, None, 2).await.unwrap();
        assert!(public_view.descendants[0].node.is_deleted);
        assert!(public_view.descendants[0].node.is_redacted);
        assert_eq!(public_view.descendants[0].node.title, "Private recipe");
    }

    #[tokio::test]
    async fn deletion_waits_for_concurrent_visibility_updates_before_snapshotting() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "lock_src").await;
        let fork_owner = owner(&pool, "lock_fork").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Lock source", "public"),
            None,
        )
        .await
        .unwrap();
        let fork_draft = create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();
        let fork = publish_recipe_draft(&pool, fork_draft.id, fork_owner.id, None)
            .await
            .unwrap();

        let mut visibility_update = pool.begin().await.unwrap();
        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(fork.id)
            .execute(&mut *visibility_update)
            .await
            .unwrap();

        let delete_pool = pool.clone();
        let delete_task = tokio::spawn(async move {
            crate::db::delete_recipe(&delete_pool, fork.id, fork_owner.id).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!delete_task.is_finished());
        visibility_update.commit().await.unwrap();
        delete_task.await.unwrap().unwrap();

        let public_view = get_recipe_lineage(&pool, source.id, None, 2).await.unwrap();
        assert!(public_view.descendants[0].node.is_redacted);
        assert_eq!(public_view.descendants[0].node.title, "Private recipe");
    }

    #[test]
    fn fresh_development_startup_has_no_lineage_backfill_step() {
        let entrypoint = include_str!("../../entrypoint.sh");
        let justfile = include_str!("../../justfile");

        assert!(!entrypoint.contains("backfill_fork_lineage"));
        assert!(!justfile.contains("backfill_fork_lineage"));
    }

    #[tokio::test]
    async fn another_users_private_recipe_cannot_be_forked() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "private_source").await;
        let forker = owner(&pool, "private_forker").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Private", "private"),
            None,
        )
        .await
        .unwrap();

        assert!(matches!(
            create_fork_draft(&pool, source.id, forker.id, None).await,
            Err(DbError::ForkError(_))
        ));
    }

    #[tokio::test]
    async fn draft_schema_allows_multiple_new_drafts_but_reuses_existing_recipe_draft() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "draft_index").await;
        let first_new = create_new_recipe_draft(&pool, user.id, &snapshot("One", "private"))
            .await
            .unwrap();
        let second_new = create_new_recipe_draft(&pool, user.id, &snapshot("Two", "private"))
            .await
            .unwrap();
        assert_ne!(first_new.id, second_new.id);

        let recipe =
            create_published_recipe(&pool, user.id, &snapshot("Published", "private"), None)
                .await
                .unwrap();
        let first_edit = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        let second_edit = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        assert_eq!(first_edit.id, second_edit.id);
    }

    #[tokio::test]
    async fn publish_rolls_back_version_recipe_and_tags_when_a_late_write_fails() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "rollback").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        let mut invalid_tag_snapshot = snapshot("Must roll back", "public");
        invalid_tag_snapshot.tags = vec!["x".repeat(101)];
        update_recipe_draft(&pool, draft.id, user.id, 0, &invalid_tag_snapshot)
            .await
            .unwrap();

        assert!(publish_recipe_draft(&pool, draft.id, user.id, None)
            .await
            .is_err());
        assert_eq!(
            get_recipe_versions(&pool, recipe.id).await.unwrap().len(),
            1
        );
        assert_eq!(
            crate::db::get_recipe_by_id(&pool, recipe.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "V1"
        );
        assert!(get_recipe_draft(&pool, draft.id, user.id).await.is_ok());
    }

    #[tokio::test]
    async fn concurrent_publish_of_one_draft_has_one_winner_and_one_consistent_latest_version() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "concurrent").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        update_recipe_draft(&pool, draft.id, user.id, 0, &snapshot("V2", "public"))
            .await
            .unwrap();

        let first_pool = pool.clone();
        let second_pool = pool.clone();
        let first = tokio::spawn(async move {
            publish_recipe_draft(&first_pool, draft.id, user.id, None).await
        });
        let second = tokio::spawn(async move {
            publish_recipe_draft(&second_pool, draft.id, user.id, None).await
        });
        let results = [first.await.unwrap(), second.await.unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            get_recipe_versions(&pool, recipe.id).await.unwrap().len(),
            2
        );
        assert_eq!(
            get_latest_recipe_version(&pool, recipe.id)
                .await
                .unwrap()
                .version_number,
            2
        );
    }

    #[tokio::test]
    async fn orphan_reference_scan_keeps_draft_and_historical_version_images() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "image_refs").await;
        let mut v1 = snapshot("Images", "private");
        v1.images.push(crate::types::RecipeImage::new(
            "https://cdn.test/historical_full.webp".into(),
            Some("https://cdn.test/historical_thumb.webp".into()),
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        let recipe = create_published_recipe(&pool, user.id, &v1, None)
            .await
            .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            1,
            &snapshot("V2", "private"),
            None,
        )
        .await
        .unwrap();
        let mut draft_snapshot = snapshot("Draft image", "private");
        draft_snapshot.images.push(crate::types::RecipeImage::new(
            "https://cdn.test/draft_full.webp".into(),
            None,
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        create_new_recipe_draft(&pool, user.id, &draft_snapshot)
            .await
            .unwrap();

        let urls = crate::db::get_all_image_urls(&pool).await.unwrap();
        assert!(urls.contains(&"https://cdn.test/historical_full.webp".to_string()));
        assert!(urls.contains(&"https://cdn.test/historical_thumb.webp".to_string()));
        assert!(urls.contains(&"https://cdn.test/draft_full.webp".to_string()));
    }
}
