use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

use super::diff::{
    compute_diff, json_to_snapshot, recipe_to_snapshot, snapshot_to_json, RecipeDraftPayload,
    RecipeSnapshot, SnapshotChange,
};
use super::{DbError, Recipe};
use crate::security::{
    RecipeValidationMode, MAX_ACTIVE_DRAFTS_PER_USER, MAX_FORK_MESSAGE_CHARS,
    MAX_VERSIONS_PER_RECIPE, MAX_VERSIONS_PER_USER, MAX_VERSION_NOTE_CHARS,
};

fn recipe_snapshot_media_urls(snapshot: &RecipeSnapshot) -> Vec<String> {
    fn collect_steps(steps: &[crate::types::RecipeStep], urls: &mut Vec<String>) {
        for step in steps {
            for image in &step.photos {
                urls.push(image.url.clone());
                urls.extend(image.thumb_url.iter().cloned());
            }
            collect_steps(&step.sub_steps, urls);
        }
    }
    let mut urls = Vec::new();
    for image in &snapshot.images {
        urls.push(image.url.clone());
        urls.extend(image.thumb_url.iter().cloned());
    }
    collect_steps(&snapshot.instructions, &mut urls);
    urls
}

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

#[derive(sqlx::FromRow)]
struct ForkAttributionRow {
    original_title: String,
    original_owner_id: Option<Uuid>,
    original_visibility: Option<String>,
    message: Option<String>,
    live_recipe_id: Option<Uuid>,
    live_owner_id: Option<Uuid>,
    live_visibility: Option<String>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VersionSummaryReadWork {
    /// Rows returned by the size-preflight query. This is not a PostgreSQL
    /// storage-engine scan count.
    pub snapshot_metadata_rows_returned: usize,
    /// Rows returned by the bounded snapshot payload query.
    pub snapshot_payload_rows_returned: usize,
    pub encoded_snapshot_bytes: usize,
    pub decoded_snapshot_bytes: usize,
    pub snapshots_decoded: usize,
    pub diff_operations: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedRecipeVersionSummaryPage {
    pub page: crate::types::RecipeVersionSummaryPage,
    pub work: VersionSummaryReadWork,
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
struct RecipeVersionSnapshotSize {
    version_number: i32,
    snapshot_bytes: i64,
    current_version_number: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct BoundedRecipeVersionPayloadRow {
    recipe_id: Uuid,
    version_number: i32,
    snapshot_text: String,
    notes: Option<String>,
    created_at: DateTime<Utc>,
}

impl BoundedRecipeVersionPayloadRow {
    fn decoded_snapshot(&self) -> Result<RecipeSnapshot, DbError> {
        serde_json::from_str(&self.snapshot_text).map_err(DbError::SerdeJson)
    }
}

fn checked_decoded_heap_add(
    total: &mut usize,
    bytes: usize,
    label: &'static str,
) -> Result<(), DbError> {
    *total = total
        .checked_add(bytes)
        .ok_or(DbError::ResourceLimitExceeded(label))?;
    Ok(())
}

fn recipe_image_decoded_heap_bytes(
    image: &crate::types::RecipeImage,
    total: &mut usize,
    label: &'static str,
) -> Result<(), DbError> {
    checked_decoded_heap_add(total, image.url.capacity(), label)?;
    if let Some(value) = &image.thumb_url {
        checked_decoded_heap_add(total, value.capacity(), label)?;
    }
    if let Some(value) = &image.file_name {
        checked_decoded_heap_add(total, value.capacity(), label)?;
    }
    checked_decoded_heap_add(
        total,
        image
            .edits
            .capacity()
            .checked_mul(std::mem::size_of::<crate::types::RecipeImageEdit>())
            .ok_or(DbError::ResourceLimitExceeded(label))?,
        label,
    )?;
    for edit in &image.edits {
        checked_decoded_heap_add(total, edit.name.capacity(), label)?;
    }
    Ok(())
}

fn recipe_steps_decoded_heap_bytes(
    steps: &[crate::types::RecipeStep],
    total: &mut usize,
    label: &'static str,
) -> Result<(), DbError> {
    for step in steps {
        checked_decoded_heap_add(total, step.text.capacity(), label)?;
        checked_decoded_heap_add(
            total,
            step.sub_steps
                .capacity()
                .checked_mul(std::mem::size_of::<crate::types::RecipeStep>())
                .ok_or(DbError::ResourceLimitExceeded(label))?,
            label,
        )?;
        recipe_steps_decoded_heap_bytes(&step.sub_steps, total, label)?;
        checked_decoded_heap_add(
            total,
            step.photos
                .capacity()
                .checked_mul(std::mem::size_of::<crate::types::RecipeImage>())
                .ok_or(DbError::ResourceLimitExceeded(label))?,
            label,
        )?;
        for image in &step.photos {
            recipe_image_decoded_heap_bytes(image, total, label)?;
        }
    }
    Ok(())
}

/// Return the bytes owned by a decoded snapshot: the root value, all vector
/// allocations, and all string allocations. Unlike JSON reserialization,
/// this follows the allocations retained by the typed Rust value.
fn recipe_snapshot_decoded_heap_bytes(snapshot: &RecipeSnapshot) -> Result<usize, DbError> {
    let label = "decoded recipe snapshot heap bytes";
    let mut total = std::mem::size_of::<RecipeSnapshot>();
    checked_decoded_heap_add(&mut total, snapshot.title.capacity(), label)?;
    for value in [&snapshot.description, &snapshot.commentary]
        .into_iter()
        .flatten()
    {
        checked_decoded_heap_add(&mut total, value.capacity(), label)?;
    }
    checked_decoded_heap_add(
        &mut total,
        snapshot
            .ingredients
            .capacity()
            .checked_mul(std::mem::size_of::<crate::types::RecipeIngredient>())
            .ok_or(DbError::ResourceLimitExceeded(label))?,
        label,
    )?;
    for ingredient in &snapshot.ingredients {
        for value in [&ingredient.amount, &ingredient.unit, &ingredient.name] {
            checked_decoded_heap_add(&mut total, value.capacity(), label)?;
        }
    }
    checked_decoded_heap_add(
        &mut total,
        snapshot
            .instructions
            .capacity()
            .checked_mul(std::mem::size_of::<crate::types::RecipeStep>())
            .ok_or(DbError::ResourceLimitExceeded(label))?,
        label,
    )?;
    recipe_steps_decoded_heap_bytes(&snapshot.instructions, &mut total, label)?;
    checked_decoded_heap_add(
        &mut total,
        snapshot
            .equipment
            .capacity()
            .checked_mul(std::mem::size_of::<crate::types::RecipeEquipment>())
            .ok_or(DbError::ResourceLimitExceeded(label))?,
        label,
    )?;
    for equipment in &snapshot.equipment {
        checked_decoded_heap_add(&mut total, equipment.name.capacity(), label)?;
    }
    checked_decoded_heap_add(
        &mut total,
        snapshot
            .images
            .capacity()
            .checked_mul(std::mem::size_of::<crate::types::RecipeImage>())
            .ok_or(DbError::ResourceLimitExceeded(label))?,
        label,
    )?;
    for image in &snapshot.images {
        recipe_image_decoded_heap_bytes(image, &mut total, label)?;
    }
    checked_decoded_heap_add(
        &mut total,
        snapshot
            .tags
            .capacity()
            .checked_mul(std::mem::size_of::<String>())
            .ok_or(DbError::ResourceLimitExceeded(label))?,
        label,
    )?;
    for tag in &snapshot.tags {
        checked_decoded_heap_add(&mut total, tag.capacity(), label)?;
    }
    Ok(total)
}

fn bounded_snapshot_change_summary(older: &RecipeSnapshot, newer: &RecipeSnapshot) -> String {
    let mut fields = Vec::with_capacity(11);
    if older.title != newer.title {
        fields.push("title");
    }
    if older.description != newer.description {
        fields.push("description");
    }
    if older.commentary != newer.commentary {
        fields.push("commentary");
    }
    if older.prep_time_minutes != newer.prep_time_minutes {
        fields.push("prep_time_minutes");
    }
    if older.cook_time_minutes != newer.cook_time_minutes {
        fields.push("cook_time_minutes");
    }
    if older.servings != newer.servings {
        fields.push("servings");
    }
    if older.ingredients != newer.ingredients {
        fields.push("ingredients");
    }
    if older.instructions != newer.instructions {
        fields.push("instructions");
    }
    if older.equipment != newer.equipment {
        fields.push("equipment");
    }
    if older.images != newer.images {
        fields.push("images");
    }
    if older.tags != newer.tags {
        fields.push("tags");
    }
    fields.sort_unstable();
    match fields.as_slice() {
        [] => "No content changes".to_string(),
        [field] => format!("Updated {field}"),
        [first, second] => format!("Updated {first} and {second}"),
        _ => {
            let last = fields.pop().expect("non-empty fields checked above");
            format!("Updated {}, and {last}", fields.join(", "))
        }
    }
}

fn checked_snapshot_byte_total(
    sizes: impl IntoIterator<Item = usize>,
    maximum: usize,
) -> Result<usize, DbError> {
    let mut total = 0usize;
    for size in sizes {
        if size > crate::security::MAX_RECIPE_SNAPSHOT_BYTES {
            return Err(DbError::ResourceLimitExceeded(
                "version history encoded snapshot bytes",
            ));
        }
        total = total
            .checked_add(size)
            .ok_or(DbError::ResourceLimitExceeded(
                "version history encoded snapshot bytes",
            ))?;
        if total > maximum {
            return Err(DbError::ResourceLimitExceeded(
                "version history encoded snapshot bytes",
            ));
        }
    }
    Ok(total)
}

/// Fetch one descending summary page plus the single older snapshot required
/// to describe the last visible row. Size preflight happens before JSONB is
/// transferred into the application process.
pub async fn get_recipe_version_summary_page(
    pool: &PgPool,
    recipe_id: Uuid,
    before_version_number: Option<i32>,
    page_size: usize,
) -> Result<BoundedRecipeVersionSummaryPage, DbError> {
    crate::security::validate_version_history_page(before_version_number, page_size)
        .map_err(|_| DbError::ResourceLimitExceeded("version history page"))?;
    let row_limit = page_size.saturating_add(1);
    let size_rows = sqlx::query_as::<_, RecipeVersionSnapshotSize>(
        "SELECT version.version_number, \
                octet_length(version.snapshot::text)::BIGINT AS snapshot_bytes, \
                (SELECT MAX(current.version_number) \
                   FROM recipe_versions AS current \
                  WHERE current.recipe_id = $1)::INTEGER AS current_version_number \
           FROM recipe_versions AS version \
          WHERE version.recipe_id = $1 \
            AND ($2::INTEGER IS NULL OR version.version_number < $2) \
          ORDER BY version.version_number DESC \
          LIMIT $3",
    )
    .bind(recipe_id)
    .bind(before_version_number)
    .bind(graph_query_limit(row_limit))
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;

    let encoded_snapshot_bytes = checked_snapshot_byte_total(
        size_rows
            .iter()
            .map(|row| usize::try_from(row.snapshot_bytes).unwrap_or(usize::MAX)),
        crate::security::MAX_VERSION_HISTORY_ENCODED_SNAPSHOT_BYTES,
    )?;
    let version_numbers = size_rows
        .iter()
        .map(|row| row.version_number)
        .collect::<Vec<_>>();
    let versions = if version_numbers.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, BoundedRecipeVersionPayloadRow>(
            "SELECT recipe_id, version_number, snapshot::text AS snapshot_text, notes, created_at \
               FROM recipe_versions \
             WHERE recipe_id = $1 AND version_number = ANY($2) \
             ORDER BY version_number DESC",
        )
        .bind(recipe_id)
        .bind(&version_numbers)
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
    };
    if versions.len() != size_rows.len() {
        return Err(DbError::ResourceLimitExceeded(
            "version history changed during page read",
        ));
    }

    let mut decoded_snapshot_bytes = 0usize;
    let mut snapshots = Vec::with_capacity(versions.len());
    for version in &versions {
        let snapshot = version.decoded_snapshot()?;
        let snapshot_bytes = recipe_snapshot_decoded_heap_bytes(&snapshot)?;
        decoded_snapshot_bytes = decoded_snapshot_bytes.checked_add(snapshot_bytes).ok_or(
            DbError::ResourceLimitExceeded("version history decoded snapshot bytes"),
        )?;
        if decoded_snapshot_bytes > crate::security::MAX_VERSION_HISTORY_DECODED_SNAPSHOT_BYTES {
            return Err(DbError::ResourceLimitExceeded(
                "version history decoded snapshot bytes",
            ));
        }
        snapshots.push(snapshot);
    }

    let visible_count = versions.len().min(page_size);
    let current_version_number = size_rows.first().map(|row| row.current_version_number);
    let mut diff_operations = 0usize;
    let items = versions
        .iter()
        .take(visible_count)
        .zip(snapshots.iter())
        .enumerate()
        .map(|(index, (version, snapshot))| {
            let change_summary = snapshots
                .get(index + 1)
                .map(|older| {
                    diff_operations = diff_operations.saturating_add(1);
                    Ok::<_, DbError>(bounded_snapshot_change_summary(older, snapshot))
                })
                .transpose()?
                .unwrap_or_else(|| "Initial version".to_string());
            Ok(crate::types::RecipeVersionSummary {
                version_number: version.version_number,
                title: snapshot.title.clone(),
                image_url: snapshot
                    .images
                    .first()
                    .map(|image| image.display_url(true).to_string()),
                notes: version.notes.clone(),
                created_at: version.created_at,
                is_current: current_version_number == Some(version.version_number),
                change_summary,
            })
        })
        .collect::<Result<Vec<_>, DbError>>()?;
    let next_before_version_number = (versions.len() > page_size)
        .then(|| items.last().map(|summary| summary.version_number))
        .flatten();

    Ok(BoundedRecipeVersionSummaryPage {
        page: crate::types::RecipeVersionSummaryPage {
            items,
            next_before_version_number,
        },
        work: VersionSummaryReadWork {
            snapshot_metadata_rows_returned: size_rows.len(),
            snapshot_payload_rows_returned: versions.len(),
            encoded_snapshot_bytes,
            decoded_snapshot_bytes,
            snapshots_decoded: snapshots.len(),
            diff_operations,
        },
    })
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
) {
    let state = graph_node_state(false, false, is_deleted);
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

const VERSION_GRAPH_LINEAGE_COUNT_CAP: usize = 50;

#[cfg(test)]
fn version_graph_reachable_recipe_ids(
    selected_recipe_id: Uuid,
    adjacent_recipe_ids: &HashMap<Uuid, HashSet<Uuid>>,
) -> HashSet<Uuid> {
    let mut reachable_recipe_ids = HashSet::new();
    let mut recipes_to_visit = vec![selected_recipe_id];

    while let Some(recipe_id) = recipes_to_visit.pop() {
        let Some(adjacent_ids) = adjacent_recipe_ids.get(&recipe_id) else {
            continue;
        };
        for adjacent_id in adjacent_ids {
            if *adjacent_id != selected_recipe_id && reachable_recipe_ids.insert(*adjacent_id) {
                recipes_to_visit.push(*adjacent_id);
            }
        }
    }

    reachable_recipe_ids
}

fn capped_version_graph_lineage_count(count: usize) -> crate::types::VersionGraphLineageCount {
    crate::types::VersionGraphLineageCount {
        count: count.min(VERSION_GRAPH_LINEAGE_COUNT_CAP),
        is_capped: count > VERSION_GRAPH_LINEAGE_COUNT_CAP,
    }
}

#[cfg(test)]
fn version_graph_lineage_counts(
    selected_recipe_id: Uuid,
    edges: &[crate::types::VersionGraphEdge],
) -> crate::types::VersionGraphLineageCounts {
    let mut parent_recipe_ids = HashMap::<Uuid, HashSet<Uuid>>::new();
    let mut child_recipe_ids = HashMap::<Uuid, HashSet<Uuid>>::new();

    for edge in edges
        .iter()
        .filter(|edge| edge.kind() == crate::types::VersionGraphEdgeKind::Fork)
    {
        let source_recipe_id = edge.source().recipe_id;
        let target_recipe_id = edge.target().recipe_id;
        parent_recipe_ids
            .entry(target_recipe_id)
            .or_default()
            .insert(source_recipe_id);
        child_recipe_ids
            .entry(source_recipe_id)
            .or_default()
            .insert(target_recipe_id);
    }

    let ancestors = version_graph_reachable_recipe_ids(selected_recipe_id, &parent_recipe_ids);
    let descendants = version_graph_reachable_recipe_ids(selected_recipe_id, &child_recipe_ids);
    let immediate_descendant_count = child_recipe_ids
        .get(&selected_recipe_id)
        .map_or(0, HashSet::len);

    crate::types::VersionGraphLineageCounts {
        ancestors: capped_version_graph_lineage_count(ancestors.len()),
        immediate_descendants: capped_version_graph_lineage_count(immediate_descendant_count),
        descendants: capped_version_graph_lineage_count(descendants.len()),
    }
}

async fn load_version_graph_lineage_counts(
    pool: &PgPool,
    selected_recipe_id: Uuid,
) -> Result<crate::types::VersionGraphLineageCounts, DbError> {
    let count_limit = graph_query_limit(VERSION_GRAPH_LINEAGE_COUNT_CAP.saturating_add(1));
    let (ancestor_count, immediate_descendant_count, descendant_count) =
        sqlx::query_as::<_, (i64, i64, i64)>(
            "WITH RECURSIVE ancestors(recipe_id) AS (\
                 SELECT COALESCE(original_recipe_key, original_recipe_id) \
                 FROM fork_relationships \
                 WHERE COALESCE(forked_recipe_key, forked_recipe_id) = $1 \
                   AND COALESCE(original_recipe_key, original_recipe_id) IS NOT NULL \
                   AND forked_version_number > 0 \
                   AND COALESCE(original_recipe_key, original_recipe_id) \
                       <> COALESCE(forked_recipe_key, forked_recipe_id) \
                 UNION \
                 SELECT COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) \
                 FROM ancestors \
                 JOIN fork_relationships AS relationship \
                   ON COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) = ancestors.recipe_id \
                 WHERE COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) IS NOT NULL \
                   AND relationship.forked_version_number > 0 \
                   AND COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) \
                       <> COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) \
             ), descendants(recipe_id) AS (\
                 SELECT COALESCE(forked_recipe_key, forked_recipe_id) \
                 FROM fork_relationships \
                 WHERE COALESCE(original_recipe_key, original_recipe_id) = $1 \
                   AND COALESCE(forked_recipe_key, forked_recipe_id) IS NOT NULL \
                   AND forked_version_number > 0 \
                   AND COALESCE(original_recipe_key, original_recipe_id) \
                       <> COALESCE(forked_recipe_key, forked_recipe_id) \
                 UNION \
                 SELECT COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) \
                 FROM descendants \
                 JOIN fork_relationships AS relationship \
                   ON COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) = descendants.recipe_id \
                 WHERE COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) IS NOT NULL \
                   AND relationship.forked_version_number > 0 \
                   AND COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) \
                       <> COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) \
             ) \
             SELECT \
                 (SELECT COUNT(*) FROM (\
                     SELECT recipe_id FROM ancestors WHERE recipe_id <> $1 LIMIT $2\
                 ) AS bounded_ancestors), \
                 (SELECT COUNT(*) FROM (\
                     SELECT DISTINCT COALESCE(forked_recipe_key, forked_recipe_id) AS recipe_id \
                     FROM fork_relationships \
                     WHERE COALESCE(original_recipe_key, original_recipe_id) = $1 \
                       AND COALESCE(forked_recipe_key, forked_recipe_id) IS NOT NULL \
                       AND COALESCE(forked_recipe_key, forked_recipe_id) <> $1 \
                       AND forked_version_number > 0 \
                       AND COALESCE(original_recipe_key, original_recipe_id) \
                           <> COALESCE(forked_recipe_key, forked_recipe_id) \
                     LIMIT $2\
                 ) AS bounded_immediate_descendants), \
                 (SELECT COUNT(*) FROM (\
                     SELECT recipe_id FROM descendants WHERE recipe_id <> $1 LIMIT $2\
                 ) AS bounded_descendants)",
        )
        .bind(selected_recipe_id)
        .bind(count_limit)
        .fetch_one(pool)
        .await
        .map_err(DbError::Query)?;

    Ok(crate::types::VersionGraphLineageCounts {
        ancestors: capped_version_graph_lineage_count(
            usize::try_from(ancestor_count).expect("bounded ancestor count fits usize"),
        ),
        immediate_descendants: capped_version_graph_lineage_count(
            usize::try_from(immediate_descendant_count)
                .expect("bounded immediate descendant count fits usize"),
        ),
        descendants: capped_version_graph_lineage_count(
            usize::try_from(descendant_count).expect("bounded descendant count fits usize"),
        ),
    })
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

fn relationship_endpoint_is_visible(
    live_recipes: &HashMap<Uuid, Recipe>,
    visible_recipe_ids: &HashSet<Uuid>,
    id: crate::types::VersionGraphNodeId,
    metadata: VersionGraphEndpointMetadata<'_>,
    viewer_id: Option<Uuid>,
) -> bool {
    match live_recipes.get(&id.recipe_id) {
        Some(_) => visible_recipe_ids.contains(&id.recipe_id),
        None => {
            deleted_recipe_metadata_is_visible(metadata.visibility, metadata.owner_id, viewer_id)
        }
    }
}

fn version_graph_relationship_edge(
    relationship: &VersionGraphRelationshipRow,
) -> Option<crate::types::VersionGraphEdge> {
    crate::types::VersionGraphEdge::fork(
        relationship.original_recipe_key()?,
        relationship.forked_version_number?,
        relationship.forked_recipe_key()?,
    )
}

/// Return the visibility-preserving connected component containing the recipe
/// the viewer explicitly requested. A visible recipe beyond an inaccessible
/// intermediate branch is intentionally omitted because returning it in this
/// graph would disclose that hidden lineage relationship.
fn visible_version_graph_component(
    selected_recipe_id: Uuid,
    relationships: &[VersionGraphRelationshipRow],
    live_recipes: &HashMap<Uuid, Recipe>,
    discoverable_live_recipe_ids: &HashSet<Uuid>,
    viewer_id: Option<Uuid>,
) -> HashSet<Uuid> {
    let mut adjacency = HashMap::<Uuid, Vec<Uuid>>::new();
    for relationship in relationships {
        let Some(edge) = version_graph_relationship_edge(relationship) else {
            continue;
        };
        if !relationship_endpoint_is_visible(
            live_recipes,
            discoverable_live_recipe_ids,
            edge.source(),
            relationship.original_endpoint_metadata(),
            viewer_id,
        ) || !relationship_endpoint_is_visible(
            live_recipes,
            discoverable_live_recipe_ids,
            edge.target(),
            relationship.forked_endpoint_metadata(),
            viewer_id,
        ) {
            continue;
        }
        adjacency
            .entry(edge.source().recipe_id)
            .or_default()
            .push(edge.target().recipe_id);
        adjacency
            .entry(edge.target().recipe_id)
            .or_default()
            .push(edge.source().recipe_id);
    }

    let mut connected = HashSet::from([selected_recipe_id]);
    let mut pending = vec![selected_recipe_id];
    while let Some(recipe_id) = pending.pop() {
        for adjacent_recipe_id in adjacency.get(&recipe_id).into_iter().flatten() {
            if connected.insert(*adjacent_recipe_id) {
                pending.push(*adjacent_recipe_id);
            }
        }
    }
    connected
}

fn append_visible_relationship_endpoint(
    graph: &mut crate::types::VersionGraph,
    live_recipes: &HashMap<Uuid, Recipe>,
    id: crate::types::VersionGraphNodeId,
    metadata: VersionGraphEndpointMetadata<'_>,
) {
    if !live_recipes.contains_key(&id.recipe_id) {
        append_synthetic_graph_node(
            graph,
            id,
            metadata.title,
            metadata.owner_username,
            metadata.created_at,
            true,
        );
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

    let lineage_counts = load_version_graph_lineage_counts(pool, selected_recipe_id).await?;
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
    let discoverable_live_recipe_ids = visible_recipe_ids.iter().copied().collect::<HashSet<_>>();
    let connected_recipe_ids = visible_version_graph_component(
        selected_recipe_id,
        &relationships,
        &live_recipes,
        &discoverable_live_recipe_ids,
        viewer_id,
    );
    visible_recipe_ids.retain(|recipe_id| connected_recipe_ids.contains(recipe_id));
    visible_recipe_ids.sort_by_key(|recipe_id| (*recipe_id != selected_recipe_id, *recipe_id));
    let mut versions_by_recipe = load_version_graph_versions(
        pool,
        &visible_recipe_ids,
        selected_recipe_id,
        limits.max_nodes,
    )
    .await?;

    let mut graph = crate::types::VersionGraph {
        lineage_counts,
        ..Default::default()
    };
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
        if let Some(edge) = version_graph_relationship_edge(relationship) {
            if !connected_recipe_ids.contains(&edge.source().recipe_id)
                || !connected_recipe_ids.contains(&edge.target().recipe_id)
            {
                continue;
            }
            let source_metadata = relationship.original_endpoint_metadata();
            let target_metadata = relationship.forked_endpoint_metadata();
            let source_is_visible = relationship_endpoint_is_visible(
                &live_recipes,
                &discoverable_live_recipe_ids,
                edge.source(),
                source_metadata,
                viewer_id,
            );
            let target_is_visible = relationship_endpoint_is_visible(
                &live_recipes,
                &discoverable_live_recipe_ids,
                edge.target(),
                target_metadata,
                viewer_id,
            );
            if !source_is_visible || !target_is_visible {
                continue;
            }
            append_visible_relationship_endpoint(
                &mut graph,
                &live_recipes,
                edge.source(),
                source_metadata,
            );
            append_visible_relationship_endpoint(
                &mut graph,
                &live_recipes,
                edge.target(),
                target_metadata,
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

pub const INITIAL_VERSION_GRAPH_NODE_LIMIT: usize = 25;
pub const VERSION_GRAPH_EXPANSION_NODE_LIMIT: usize = 5;

fn version_graph_frontier_rank(kind: crate::types::VersionGraphFrontierKind) -> u8 {
    match kind {
        crate::types::VersionGraphFrontierKind::OlderVersions => 0,
        crate::types::VersionGraphFrontierKind::NewerVersions => 1,
        crate::types::VersionGraphFrontierKind::ParentFork => 2,
        crate::types::VersionGraphFrontierKind::ChildForks => 3,
    }
}

fn version_graph_neighbor(
    anchor: crate::types::VersionGraphNodeId,
    edge: &crate::types::VersionGraphEdge,
) -> Option<(
    crate::types::VersionGraphFrontierKind,
    crate::types::VersionGraphNodeId,
)> {
    match (
        edge.kind(),
        edge.source() == anchor,
        edge.target() == anchor,
    ) {
        (crate::types::VersionGraphEdgeKind::Version, true, _) => Some((
            crate::types::VersionGraphFrontierKind::NewerVersions,
            edge.target(),
        )),
        (crate::types::VersionGraphEdgeKind::Version, _, true) => Some((
            crate::types::VersionGraphFrontierKind::OlderVersions,
            edge.source(),
        )),
        (crate::types::VersionGraphEdgeKind::Fork, true, _) => Some((
            crate::types::VersionGraphFrontierKind::ChildForks,
            edge.target(),
        )),
        (crate::types::VersionGraphEdgeKind::Fork, _, true) => Some((
            crate::types::VersionGraphFrontierKind::ParentFork,
            edge.source(),
        )),
        _ => None,
    }
}

fn version_graph_neighbors(
    graph: &crate::types::VersionGraph,
    anchor: crate::types::VersionGraphNodeId,
) -> Vec<(
    crate::types::VersionGraphFrontierKind,
    crate::types::VersionGraphNodeId,
)> {
    let mut neighbors = graph
        .edges
        .iter()
        .filter_map(|edge| version_graph_neighbor(anchor, edge))
        .collect::<Vec<_>>();
    neighbors.sort_by_key(|(kind, id)| {
        (
            version_graph_frontier_rank(*kind),
            id.recipe_id,
            id.version_number,
        )
    });
    neighbors.dedup();
    neighbors
}

fn version_graph_frontiers(
    graph: &crate::types::VersionGraph,
    loaded_node_ids: &HashSet<crate::types::VersionGraphNodeId>,
) -> Vec<crate::types::VersionGraphFrontier> {
    let mut anchors = loaded_node_ids.iter().copied().collect::<Vec<_>>();
    anchors.sort_by_key(|id| (id.recipe_id, id.version_number));
    let mut frontiers = Vec::new();

    for anchor in anchors {
        let mut hidden_by_kind = HashMap::<crate::types::VersionGraphFrontierKind, Vec<_>>::new();
        for (kind, neighbor) in version_graph_neighbors(graph, anchor) {
            if !loaded_node_ids.contains(&neighbor) {
                hidden_by_kind.entry(kind).or_default().push(neighbor);
            }
        }
        for kind in [
            crate::types::VersionGraphFrontierKind::OlderVersions,
            crate::types::VersionGraphFrontierKind::NewerVersions,
            crate::types::VersionGraphFrontierKind::ParentFork,
            crate::types::VersionGraphFrontierKind::ChildForks,
        ] {
            if let Some(seeds) = hidden_by_kind.remove(&kind) {
                let hidden_count =
                    version_graph_bfs(graph, seeds, loaded_node_ids, usize::MAX).len();
                frontiers.push(crate::types::VersionGraphFrontier {
                    anchor,
                    kind,
                    hidden_count,
                    fork_cursor: None,
                });
            }
        }
    }
    frontiers
}

fn version_graph_bfs(
    graph: &crate::types::VersionGraph,
    seeds: impl IntoIterator<Item = crate::types::VersionGraphNodeId>,
    excluded_node_ids: &HashSet<crate::types::VersionGraphNodeId>,
    max_new_nodes: usize,
) -> Vec<crate::types::VersionGraphNodeId> {
    let graph_node_ids = graph
        .nodes
        .iter()
        .map(crate::types::VersionGraphNode::id)
        .collect::<HashSet<_>>();
    let mut queued = excluded_node_ids.clone();
    let mut queue = VecDeque::new();
    for seed in seeds {
        if graph_node_ids.contains(&seed) && queued.insert(seed) {
            queue.push_back(seed);
        }
    }

    let mut selected = Vec::new();
    while let Some(node_id) = queue.pop_front() {
        if selected.len() == max_new_nodes {
            break;
        }
        selected.push(node_id);
        for (_, neighbor) in version_graph_neighbors(graph, node_id) {
            if graph_node_ids.contains(&neighbor) && queued.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }
    selected
}

fn version_graph_batch(
    complete_graph: &crate::types::VersionGraph,
    previously_loaded_node_ids: &HashSet<crate::types::VersionGraphNodeId>,
    new_node_ids: Vec<crate::types::VersionGraphNodeId>,
) -> crate::types::VersionGraph {
    let new_node_id_set = new_node_ids.iter().copied().collect::<HashSet<_>>();
    let combined_node_ids = previously_loaded_node_ids
        .union(&new_node_id_set)
        .copied()
        .collect::<HashSet<_>>();
    let node_by_id = complete_graph
        .nodes
        .iter()
        .map(|node| (node.id(), node))
        .collect::<HashMap<_, _>>();
    let nodes = new_node_ids
        .into_iter()
        .filter_map(|id| node_by_id.get(&id).map(|node| (*node).clone()))
        .collect();
    let edges = complete_graph
        .edges
        .iter()
        .filter(|edge| {
            combined_node_ids.contains(&edge.source())
                && combined_node_ids.contains(&edge.target())
                && (new_node_id_set.contains(&edge.source())
                    || new_node_id_set.contains(&edge.target()))
        })
        .cloned()
        .collect();

    crate::types::VersionGraph {
        nodes,
        edges,
        frontiers: version_graph_frontiers(complete_graph, &combined_node_ids),
        truncation: complete_graph.truncation,
        lineage_counts: complete_graph.lineage_counts,
    }
}

fn initial_version_graph_batch(
    complete_graph: &crate::types::VersionGraph,
) -> crate::types::VersionGraph {
    let seed = complete_graph
        .nodes
        .iter()
        .filter(|node| node.is_selected_recipe())
        .max_by_key(|node| (node.is_current_version(), node.id().version_number))
        .map(crate::types::VersionGraphNode::id);
    let new_node_ids = seed.map_or_else(Vec::new, |seed| {
        version_graph_bfs(
            complete_graph,
            [seed],
            &HashSet::new(),
            INITIAL_VERSION_GRAPH_NODE_LIMIT,
        )
    });
    version_graph_batch(complete_graph, &HashSet::new(), new_node_ids)
}

fn invalid_version_graph_expansion(message: &str) -> DbError {
    DbError::InvalidRecipeSnapshot(format!("invalid version graph expansion: {message}"))
}

fn expanded_version_graph_batch(
    complete_graph: &crate::types::VersionGraph,
    frontier: crate::types::VersionGraphFrontier,
    loaded_node_ids: Vec<crate::types::VersionGraphNodeId>,
) -> Result<crate::types::VersionGraph, DbError> {
    let loaded_node_id_set = loaded_node_ids.iter().copied().collect::<HashSet<_>>();
    if loaded_node_id_set.len() != loaded_node_ids.len() {
        return Err(invalid_version_graph_expansion(
            "loaded node IDs must be unique",
        ));
    }
    if !loaded_node_id_set.contains(&frontier.anchor) {
        return Err(invalid_version_graph_expansion(
            "frontier anchor is not loaded",
        ));
    }
    let complete_node_ids = complete_graph
        .nodes
        .iter()
        .map(crate::types::VersionGraphNode::id)
        .collect::<HashSet<_>>();
    if !loaded_node_id_set.is_subset(&complete_node_ids) {
        return Err(invalid_version_graph_expansion(
            "loaded node ID is not in this graph",
        ));
    }

    let current_frontier = version_graph_frontiers(complete_graph, &loaded_node_id_set)
        .into_iter()
        .find(|candidate| candidate.anchor == frontier.anchor && candidate.kind == frontier.kind)
        .ok_or_else(|| invalid_version_graph_expansion("frontier direction is not available"))?;
    if current_frontier.hidden_count != frontier.hidden_count {
        return Err(invalid_version_graph_expansion("frontier is stale"));
    }

    let seeds = version_graph_neighbors(complete_graph, frontier.anchor)
        .into_iter()
        .filter(|(kind, id)| *kind == frontier.kind && !loaded_node_id_set.contains(id))
        .map(|(_, id)| id)
        .collect::<Vec<_>>();
    let new_node_ids = version_graph_bfs(
        complete_graph,
        seeds,
        &loaded_node_id_set,
        VERSION_GRAPH_EXPANSION_NODE_LIMIT,
    );
    Ok(version_graph_batch(
        complete_graph,
        &loaded_node_id_set,
        new_node_ids,
    ))
}

const VERSION_GRAPH_PAGE_EDGE_LIMIT: usize = 64;
const VERSION_GRAPH_VALIDATION_RELATIONSHIP_LIMIT: usize = 500;
const VERSION_GRAPH_FRONTIER_COUNT_CAP: usize = VERSION_GRAPH_EXPANSION_NODE_LIMIT + 1;

fn graph_frontier_probe_limit(node_limit: usize) -> usize {
    node_limit
        .max(VERSION_GRAPH_FRONTIER_COUNT_CAP)
        .saturating_add(1)
}

fn graph_fork_raw_page_limit(node_limit: usize) -> usize {
    node_limit.clamp(
        VERSION_GRAPH_EXPANSION_NODE_LIMIT,
        INITIAL_VERSION_GRAPH_NODE_LIMIT,
    )
}

fn graph_fork_query_limit(node_limit: usize) -> usize {
    graph_fork_raw_page_limit(node_limit).saturating_add(1)
}

fn bounded_graph_frontier_hidden_count(probed_rows: usize) -> usize {
    probed_rows.min(VERSION_GRAPH_FRONTIER_COUNT_CAP)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VersionGraphPageReadWork {
    /// SQL result rows returned to the application, not PostgreSQL
    /// storage-engine scan counts.
    pub recipe_rows_returned: usize,
    pub validation_rows_returned: usize,
    pub relationship_rows_returned: usize,
    pub relationship_queries: usize,
    pub snapshot_metadata_rows_returned: usize,
    pub snapshot_payload_rows_returned: usize,
    pub encoded_snapshot_bytes: usize,
    pub decoded_snapshot_bytes: usize,
    pub snapshots_decoded: usize,
    pub diff_operations: usize,
}

impl VersionGraphPageReadWork {
    fn absorb(&mut self, other: Self) {
        self.recipe_rows_returned = self
            .recipe_rows_returned
            .saturating_add(other.recipe_rows_returned);
        self.validation_rows_returned = self
            .validation_rows_returned
            .saturating_add(other.validation_rows_returned);
        self.relationship_rows_returned = self
            .relationship_rows_returned
            .saturating_add(other.relationship_rows_returned);
        self.relationship_queries = self
            .relationship_queries
            .saturating_add(other.relationship_queries);
        self.snapshot_metadata_rows_returned = self
            .snapshot_metadata_rows_returned
            .saturating_add(other.snapshot_metadata_rows_returned);
        self.snapshot_payload_rows_returned = self
            .snapshot_payload_rows_returned
            .saturating_add(other.snapshot_payload_rows_returned);
        self.encoded_snapshot_bytes = self
            .encoded_snapshot_bytes
            .saturating_add(other.encoded_snapshot_bytes);
        self.decoded_snapshot_bytes = self
            .decoded_snapshot_bytes
            .saturating_add(other.decoded_snapshot_bytes);
        self.snapshots_decoded = self
            .snapshots_decoded
            .saturating_add(other.snapshots_decoded);
        self.diff_operations = self.diff_operations.saturating_add(other.diff_operations);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedVersionGraphPage {
    pub graph: crate::types::VersionGraph,
    pub work: VersionGraphPageReadWork,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct GraphRecipeAccessRow {
    id: Uuid,
    user_id: Uuid,
    visibility: String,
    author_username: String,
    current_version_number: Option<i32>,
}

#[derive(Debug, Clone)]
struct OwnedVersionGraphEndpointMetadata {
    title: String,
    owner_username: Option<String>,
    created_at: DateTime<Utc>,
}

impl OwnedVersionGraphEndpointMetadata {
    fn from_borrowed(metadata: VersionGraphEndpointMetadata<'_>) -> Self {
        Self {
            title: metadata.title.to_string(),
            owner_username: metadata.owner_username.map(str::to_string),
            created_at: metadata.created_at,
        }
    }
}

#[derive(Debug, Default)]
struct VersionGraphContext {
    live_recipes: HashMap<Uuid, GraphRecipeAccessRow>,
    synthetic_nodes: HashMap<crate::types::VersionGraphNodeId, OwnedVersionGraphEndpointMetadata>,
    edges: Vec<crate::types::VersionGraphEdge>,
    work: VersionGraphPageReadWork,
}

#[derive(Debug, Default)]
struct VersionGraphDirectionPage {
    node_ids: Vec<crate::types::VersionGraphNodeId>,
    hidden_count: usize,
    next_fork_cursor: Option<Uuid>,
    work: VersionGraphPageReadWork,
}

#[derive(Debug, Clone, Copy)]
struct VersionGraphDirectionRequest<'a> {
    anchor: crate::types::VersionGraphNodeId,
    kind: crate::types::VersionGraphFrontierKind,
    excluded_node_ids: &'a HashSet<crate::types::VersionGraphNodeId>,
    fork_cursor: Option<Uuid>,
    limit: usize,
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
struct GraphSnapshotSize {
    recipe_id: Uuid,
    version_number: i32,
    snapshot_bytes: i64,
}

fn graph_live_recipe_is_visible(
    recipe: &GraphRecipeAccessRow,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
) -> bool {
    if recipe.id == selected_recipe_id {
        recipe.visibility != "private" || viewer_id == Some(recipe.user_id)
    } else {
        recipe.visibility == "public" || viewer_id == Some(recipe.user_id)
    }
}

fn graph_relationship_endpoint_is_visible(
    live_recipes: &HashMap<Uuid, GraphRecipeAccessRow>,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    recipe_id: Uuid,
    metadata: VersionGraphEndpointMetadata<'_>,
) -> bool {
    live_recipes.get(&recipe_id).map_or_else(
        || deleted_recipe_metadata_is_visible(metadata.visibility, metadata.owner_id, viewer_id),
        |recipe| graph_live_recipe_is_visible(recipe, selected_recipe_id, viewer_id),
    )
}

async fn load_graph_recipe_access_rows(
    pool: &PgPool,
    recipe_ids: &[Uuid],
) -> Result<Vec<GraphRecipeAccessRow>, DbError> {
    if recipe_ids.is_empty() {
        return Ok(Vec::new());
    }
    sqlx::query_as::<_, GraphRecipeAccessRow>(
        "SELECT recipe.id, recipe.user_id, recipe.visibility, \
                owner.username AS author_username, \
                (SELECT MAX(version.version_number) \
                   FROM recipe_versions AS version \
                  WHERE version.recipe_id = recipe.id) AS current_version_number \
           FROM recipes AS recipe \
           JOIN users AS owner ON owner.id = recipe.user_id \
          WHERE recipe.id = ANY($1) \
          ORDER BY recipe.id",
    )
    .bind(recipe_ids)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
}

fn visible_graph_relationship(
    relationship: &VersionGraphRelationshipRow,
    live_recipes: &HashMap<Uuid, GraphRecipeAccessRow>,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
) -> bool {
    let Some(source_recipe_id) = relationship.original_recipe_key() else {
        return false;
    };
    let Some(target_recipe_id) = relationship.forked_recipe_key() else {
        return false;
    };
    graph_relationship_endpoint_is_visible(
        live_recipes,
        selected_recipe_id,
        viewer_id,
        source_recipe_id,
        relationship.original_endpoint_metadata(),
    ) && graph_relationship_endpoint_is_visible(
        live_recipes,
        selected_recipe_id,
        viewer_id,
        target_recipe_id,
        relationship.forked_endpoint_metadata(),
    )
}

async fn load_version_graph_context(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    node_ids: &HashSet<crate::types::VersionGraphNodeId>,
) -> Result<VersionGraphContext, DbError> {
    if node_ids.is_empty() {
        return Err(invalid_version_graph_expansion("node set is empty"));
    }
    let mut recipe_ids = node_ids
        .iter()
        .map(|node_id| node_id.recipe_id)
        .collect::<Vec<_>>();
    recipe_ids.sort_unstable();
    recipe_ids.dedup();
    let live_recipe_rows = load_graph_recipe_access_rows(pool, &recipe_ids).await?;
    let mut context = VersionGraphContext {
        work: VersionGraphPageReadWork {
            recipe_rows_returned: live_recipe_rows.len(),
            ..Default::default()
        },
        live_recipes: live_recipe_rows
            .into_iter()
            .map(|recipe| (recipe.id, recipe))
            .collect(),
        ..Default::default()
    };
    if context
        .live_recipes
        .values()
        .any(|recipe| !graph_live_recipe_is_visible(recipe, selected_recipe_id, viewer_id))
    {
        return Err(invalid_version_graph_expansion(
            "loaded node is not visible to this viewer",
        ));
    }

    let live_node_ids = node_ids
        .iter()
        .filter(|node_id| context.live_recipes.contains_key(&node_id.recipe_id))
        .copied()
        .collect::<Vec<_>>();
    let live_recipe_keys = live_node_ids
        .iter()
        .map(|node_id| node_id.recipe_id)
        .collect::<Vec<_>>();
    let live_version_numbers = live_node_ids
        .iter()
        .map(|node_id| node_id.version_number)
        .collect::<Vec<_>>();
    let persisted_live_node_ids = if live_node_ids.is_empty() {
        HashSet::new()
    } else {
        sqlx::query_as::<_, (Uuid, i32)>(
            "WITH requested(recipe_id, version_number) AS (\
                 SELECT * FROM UNNEST($1::UUID[], $2::INTEGER[])\
             ) \
             SELECT version.recipe_id, version.version_number \
               FROM recipe_versions AS version \
               JOIN requested USING (recipe_id, version_number)",
        )
        .bind(&live_recipe_keys)
        .bind(&live_version_numbers)
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
        .into_iter()
        .map(
            |(recipe_id, version_number)| crate::types::VersionGraphNodeId {
                recipe_id,
                version_number,
            },
        )
        .collect::<HashSet<_>>()
    };
    context.work.validation_rows_returned = persisted_live_node_ids.len();

    let relationship_limit = VERSION_GRAPH_VALIDATION_RELATIONSHIP_LIMIT.saturating_add(1);
    let relationships = sqlx::query_as::<_, VersionGraphRelationshipRow>(
        "SELECT relationship.id, relationship.original_recipe_id, \
                relationship.original_recipe_key, relationship.forked_recipe_id, \
                relationship.forked_recipe_key, relationship.forked_version_number, \
                relationship.original_title, relationship.forked_title, \
                relationship.original_owner_id, relationship.original_owner_username, \
                relationship.original_visibility, relationship.forked_owner_id, \
                relationship.forked_owner_username, relationship.forked_visibility, \
                relationship.created_at \
           FROM fork_relationships AS relationship \
          WHERE COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) = ANY($1) \
            AND COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) = ANY($1) \
          ORDER BY relationship.created_at, relationship.id \
          LIMIT $2",
    )
    .bind(&recipe_ids)
    .bind(graph_query_limit(relationship_limit))
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    if relationships.len() > VERSION_GRAPH_VALIDATION_RELATIONSHIP_LIMIT {
        return Err(DbError::ResourceLimitExceeded(
            "version graph validation relationships",
        ));
    }
    context.work.relationship_rows_returned = relationships.len();
    context.work.relationship_queries = 1;

    for relationship in relationships.iter().filter(|relationship| {
        visible_graph_relationship(
            relationship,
            &context.live_recipes,
            selected_recipe_id,
            viewer_id,
        )
    }) {
        let Some(edge) = version_graph_relationship_edge(relationship) else {
            continue;
        };
        if !node_ids.contains(&edge.source()) || !node_ids.contains(&edge.target()) {
            continue;
        }
        if !context.live_recipes.contains_key(&edge.source().recipe_id) {
            context
                .synthetic_nodes
                .entry(edge.source())
                .or_insert_with(|| {
                    OwnedVersionGraphEndpointMetadata::from_borrowed(
                        relationship.original_endpoint_metadata(),
                    )
                });
        }
        if !context.live_recipes.contains_key(&edge.target().recipe_id) {
            context
                .synthetic_nodes
                .entry(edge.target())
                .or_insert_with(|| {
                    OwnedVersionGraphEndpointMetadata::from_borrowed(
                        relationship.forked_endpoint_metadata(),
                    )
                });
        }
        context.edges.push(edge);
    }

    let mut versions_by_recipe = HashMap::<Uuid, Vec<i32>>::new();
    for node_id in node_ids
        .iter()
        .filter(|node_id| context.live_recipes.contains_key(&node_id.recipe_id))
    {
        versions_by_recipe
            .entry(node_id.recipe_id)
            .or_default()
            .push(node_id.version_number);
    }
    for (recipe_id, versions) in &mut versions_by_recipe {
        versions.sort_unstable();
        versions.dedup();
        for pair in versions.windows(2).filter(|pair| pair[1] == pair[0] + 1) {
            if let Some(edge) = crate::types::VersionGraphEdge::version(
                crate::types::VersionGraphNodeId {
                    recipe_id: *recipe_id,
                    version_number: pair[0],
                },
                crate::types::VersionGraphNodeId {
                    recipe_id: *recipe_id,
                    version_number: pair[1],
                },
            ) {
                context.edges.push(edge);
            }
        }
    }
    context.edges.sort_by_key(|edge| {
        (
            edge.source().recipe_id,
            edge.source().version_number,
            edge.target().recipe_id,
            edge.target().version_number,
            match edge.kind() {
                crate::types::VersionGraphEdgeKind::Version => 0_u8,
                crate::types::VersionGraphEdgeKind::Fork => 1_u8,
            },
        )
    });
    context.edges.dedup();

    for node_id in node_ids {
        if context.live_recipes.contains_key(&node_id.recipe_id) {
            if !persisted_live_node_ids.contains(node_id) {
                return Err(invalid_version_graph_expansion(
                    "loaded version does not exist",
                ));
            }
        } else if !context.synthetic_nodes.contains_key(node_id) {
            return Err(invalid_version_graph_expansion(
                "loaded deleted node is not visible in this graph",
            ));
        }
    }

    let selected_current_version = context
        .live_recipes
        .get(&selected_recipe_id)
        .and_then(|recipe| recipe.current_version_number)
        .ok_or(DbError::RecipeNotFound(selected_recipe_id))?;
    let selected_node_id = crate::types::VersionGraphNodeId {
        recipe_id: selected_recipe_id,
        version_number: selected_current_version,
    };
    if !node_ids.contains(&selected_node_id) {
        return Err(invalid_version_graph_expansion(
            "selected current version is not loaded",
        ));
    }
    let mut adjacent_nodes =
        HashMap::<crate::types::VersionGraphNodeId, Vec<crate::types::VersionGraphNodeId>>::new();
    for edge in &context.edges {
        adjacent_nodes
            .entry(edge.source())
            .or_default()
            .push(edge.target());
        adjacent_nodes
            .entry(edge.target())
            .or_default()
            .push(edge.source());
    }
    let mut connected = HashSet::from([selected_node_id]);
    let mut pending = vec![selected_node_id];
    while let Some(node_id) = pending.pop() {
        if let Some(adjacent) = adjacent_nodes.get(&node_id) {
            for adjacent in adjacent {
                if node_ids.contains(adjacent) && connected.insert(*adjacent) {
                    pending.push(*adjacent);
                }
            }
        }
    }
    if connected.len() != node_ids.len() {
        return Err(invalid_version_graph_expansion(
            "loaded nodes are not connected to the selected recipe",
        ));
    }
    Ok(context)
}

async fn load_version_direction_page(
    pool: &PgPool,
    anchor: crate::types::VersionGraphNodeId,
    kind: crate::types::VersionGraphFrontierKind,
    excluded_node_ids: &HashSet<crate::types::VersionGraphNodeId>,
    limit: usize,
) -> Result<VersionGraphDirectionPage, DbError> {
    let excluded_versions = excluded_node_ids
        .iter()
        .filter(|node_id| node_id.recipe_id == anchor.recipe_id)
        .map(|node_id| node_id.version_number)
        .collect::<Vec<_>>();
    let (comparison, ordering) = match kind {
        crate::types::VersionGraphFrontierKind::OlderVersions => ("<", "DESC"),
        crate::types::VersionGraphFrontierKind::NewerVersions => (">", "ASC"),
        _ => return Ok(VersionGraphDirectionPage::default()),
    };
    let mut version_numbers = sqlx::query_scalar::<_, i32>(&format!(
        "SELECT version_number FROM recipe_versions \
         WHERE recipe_id = $1 AND version_number {comparison} $2 \
           AND NOT (version_number = ANY($3)) \
         ORDER BY version_number {ordering} LIMIT $4"
    ))
    .bind(anchor.recipe_id)
    .bind(anchor.version_number)
    .bind(&excluded_versions)
    .bind(graph_query_limit(graph_frontier_probe_limit(limit)))
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    let probed_rows = version_numbers.len();
    let hidden_count = bounded_graph_frontier_hidden_count(probed_rows);
    version_numbers.truncate(limit);
    Ok(VersionGraphDirectionPage {
        node_ids: version_numbers
            .into_iter()
            .map(|version_number| crate::types::VersionGraphNodeId {
                recipe_id: anchor.recipe_id,
                version_number,
            })
            .collect(),
        hidden_count,
        next_fork_cursor: None,
        work: VersionGraphPageReadWork {
            validation_rows_returned: probed_rows,
            ..Default::default()
        },
    })
}

async fn load_fork_direction_page(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    request: VersionGraphDirectionRequest<'_>,
) -> Result<VersionGraphDirectionPage, DbError> {
    let VersionGraphDirectionRequest {
        anchor,
        kind,
        excluded_node_ids,
        fork_cursor: cursor,
        limit,
    } = request;
    let excluded_recipe_ids = excluded_node_ids
        .iter()
        .map(|node_id| node_id.recipe_id)
        .collect::<Vec<_>>();
    let (direction_clause, neighbor_is_source) = match kind {
        crate::types::VersionGraphFrontierKind::ParentFork => (
            "$2 = 1 \
             AND COALESCE(relationship.forked_recipe_key, relationship.forked_recipe_id) = $1",
            true,
        ),
        crate::types::VersionGraphFrontierKind::ChildForks => (
            "COALESCE(relationship.original_recipe_key, relationship.original_recipe_id) = $1 \
             AND relationship.forked_version_number = $2",
            false,
        ),
        _ => return Ok(VersionGraphDirectionPage::default()),
    };
    let select = "SELECT relationship.id, relationship.original_recipe_id, \
        relationship.original_recipe_key, relationship.forked_recipe_id, \
        relationship.forked_recipe_key, relationship.forked_version_number, \
        relationship.original_title, relationship.forked_title, \
        relationship.original_owner_id, relationship.original_owner_username, \
        relationship.original_visibility, relationship.forked_owner_id, \
        relationship.forked_owner_username, relationship.forked_visibility, \
        relationship.created_at";
    // The matching schema indexes order raw branch rows by the random UUID
    // primary key. LIMIT therefore bounds the branch read before visibility,
    // validity, exclusion, or deduplication work happens in the application.
    let mut relationships = if let Some(cursor) = cursor {
        let row_sql = format!(
            "{select} FROM fork_relationships AS relationship \
              WHERE {direction_clause} AND relationship.id > $3 \
              ORDER BY relationship.id LIMIT $4"
        );
        sqlx::query_as::<_, VersionGraphRelationshipRow>(&row_sql)
            .bind(anchor.recipe_id)
            .bind(anchor.version_number)
            .bind(cursor)
            .bind(graph_query_limit(graph_fork_query_limit(limit)))
            .fetch_all(pool)
            .await
            .map_err(DbError::Query)?
    } else {
        let row_sql = format!(
            "{select} FROM fork_relationships AS relationship \
              WHERE {direction_clause} \
              ORDER BY relationship.id LIMIT $3"
        );
        sqlx::query_as::<_, VersionGraphRelationshipRow>(&row_sql)
            .bind(anchor.recipe_id)
            .bind(anchor.version_number)
            .bind(graph_query_limit(graph_fork_query_limit(limit)))
            .fetch_all(pool)
            .await
            .map_err(DbError::Query)?
    };
    let relationship_rows_returned = relationships.len();
    let raw_page_limit = graph_fork_raw_page_limit(limit);
    let has_more_raw_rows = relationships.len() > raw_page_limit;
    relationships.truncate(raw_page_limit);
    // Retain the last consumed key even for a terminal slice. Frontier
    // rebuilding must probe strictly after it; dropping the key would restart
    // at the branch beginning and manufacture a false continuation from rows
    // that are already loaded.
    let next_fork_cursor = relationships.last().map(|relationship| relationship.id);

    let mut endpoint_recipe_ids = relationships
        .iter()
        .flat_map(|relationship| {
            [
                relationship.original_recipe_key(),
                relationship.forked_recipe_key(),
            ]
        })
        .flatten()
        .collect::<Vec<_>>();
    endpoint_recipe_ids.sort_unstable();
    endpoint_recipe_ids.dedup();
    let live_recipe_rows = load_graph_recipe_access_rows(pool, &endpoint_recipe_ids).await?;
    let recipe_rows_returned = live_recipe_rows.len();
    let live_recipes = live_recipe_rows
        .into_iter()
        .map(|recipe| (recipe.id, recipe))
        .collect::<HashMap<_, _>>();

    let mut seen_node_ids = HashSet::new();
    let mut node_ids = relationships
        .iter()
        .filter(|relationship| {
            let Some(source) = relationship.original_recipe_key() else {
                return false;
            };
            let Some(target) = relationship.forked_recipe_key() else {
                return false;
            };
            relationship
                .forked_version_number
                .is_some_and(|version| version > 0)
                && source != target
                && visible_graph_relationship(
                    relationship,
                    &live_recipes,
                    selected_recipe_id,
                    viewer_id,
                )
        })
        .filter_map(|relationship| {
            if neighbor_is_source {
                Some(crate::types::VersionGraphNodeId {
                    recipe_id: relationship.original_recipe_key()?,
                    version_number: relationship.forked_version_number?,
                })
            } else {
                Some(crate::types::VersionGraphNodeId {
                    recipe_id: relationship.forked_recipe_key()?,
                    version_number: 1,
                })
            }
        })
        .filter(|node_id| {
            !excluded_recipe_ids.contains(&node_id.recipe_id) && seen_node_ids.insert(*node_id)
        })
        .collect::<Vec<_>>();
    let hidden_count = bounded_graph_frontier_hidden_count(
        node_ids
            .len()
            .saturating_add(usize::from(has_more_raw_rows)),
    );
    node_ids.truncate(limit);
    Ok(VersionGraphDirectionPage {
        node_ids,
        hidden_count,
        next_fork_cursor,
        work: VersionGraphPageReadWork {
            recipe_rows_returned,
            relationship_rows_returned,
            relationship_queries: 1,
            ..Default::default()
        },
    })
}

async fn load_graph_direction_page(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    request: VersionGraphDirectionRequest<'_>,
) -> Result<VersionGraphDirectionPage, DbError> {
    match request.kind {
        crate::types::VersionGraphFrontierKind::OlderVersions
        | crate::types::VersionGraphFrontierKind::NewerVersions => {
            if request.fork_cursor.is_some() {
                return Err(invalid_version_graph_expansion(
                    "version frontier contains a fork cursor",
                ));
            }
            load_version_direction_page(
                pool,
                request.anchor,
                request.kind,
                request.excluded_node_ids,
                request.limit,
            )
            .await
        }
        crate::types::VersionGraphFrontierKind::ParentFork
        | crate::types::VersionGraphFrontierKind::ChildForks => {
            load_fork_direction_page(pool, selected_recipe_id, viewer_id, request).await
        }
    }
}

async fn discover_initial_version_graph_node_ids(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    seed: crate::types::VersionGraphNodeId,
) -> Result<
    (
        Vec<crate::types::VersionGraphNodeId>,
        VersionGraphPageReadWork,
        HashMap<
            (
                crate::types::VersionGraphNodeId,
                crate::types::VersionGraphFrontierKind,
            ),
            Uuid,
        >,
    ),
    DbError,
> {
    let mut selected = vec![seed];
    let mut selected_set = HashSet::from([seed]);
    let mut queue = VecDeque::from([seed]);
    let mut work = VersionGraphPageReadWork::default();
    let mut fork_continuations = HashMap::new();
    while let Some(anchor) = queue.pop_front() {
        if selected.len() >= INITIAL_VERSION_GRAPH_NODE_LIMIT {
            break;
        }
        for kind in [
            crate::types::VersionGraphFrontierKind::OlderVersions,
            crate::types::VersionGraphFrontierKind::NewerVersions,
            crate::types::VersionGraphFrontierKind::ParentFork,
            crate::types::VersionGraphFrontierKind::ChildForks,
        ] {
            let remaining = INITIAL_VERSION_GRAPH_NODE_LIMIT.saturating_sub(selected.len());
            if remaining == 0 {
                break;
            }
            let page = load_graph_direction_page(
                pool,
                selected_recipe_id,
                viewer_id,
                VersionGraphDirectionRequest {
                    anchor,
                    kind,
                    excluded_node_ids: &selected_set,
                    fork_cursor: None,
                    limit: remaining,
                },
            )
            .await?;
            work.absorb(page.work);
            if let Some(cursor) = page.next_fork_cursor {
                fork_continuations.insert((anchor, kind), cursor);
            }
            for node_id in page.node_ids {
                if selected_set.insert(node_id) {
                    selected.push(node_id);
                    queue.push_back(node_id);
                }
            }
        }
    }
    Ok((selected, work, fork_continuations))
}

async fn build_bounded_version_graph_frontiers(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    loaded_node_ids: &HashSet<crate::types::VersionGraphNodeId>,
    allowed_anchor_ids: &HashSet<crate::types::VersionGraphNodeId>,
    fork_continuations: &HashMap<
        (
            crate::types::VersionGraphNodeId,
            crate::types::VersionGraphFrontierKind,
        ),
        Uuid,
    >,
) -> Result<
    (
        Vec<crate::types::VersionGraphFrontier>,
        VersionGraphPageReadWork,
    ),
    DbError,
> {
    let mut boundary_anchors = Vec::new();
    let mut versions_by_recipe = HashMap::<Uuid, Vec<i32>>::new();
    for node_id in loaded_node_ids {
        versions_by_recipe
            .entry(node_id.recipe_id)
            .or_default()
            .push(node_id.version_number);
    }
    for (recipe_id, versions) in &mut versions_by_recipe {
        versions.sort_unstable();
        versions.dedup();
        if let Some(oldest) = versions.first() {
            boundary_anchors.push((
                crate::types::VersionGraphNodeId {
                    recipe_id: *recipe_id,
                    version_number: *oldest,
                },
                crate::types::VersionGraphFrontierKind::OlderVersions,
            ));
        }
        if let Some(newest) = versions.last() {
            boundary_anchors.push((
                crate::types::VersionGraphNodeId {
                    recipe_id: *recipe_id,
                    version_number: *newest,
                },
                crate::types::VersionGraphFrontierKind::NewerVersions,
            ));
        }
    }
    for node_id in loaded_node_ids {
        boundary_anchors.push((*node_id, crate::types::VersionGraphFrontierKind::ChildForks));
        if node_id.version_number == 1 {
            boundary_anchors.push((*node_id, crate::types::VersionGraphFrontierKind::ParentFork));
        }
    }
    boundary_anchors.sort_by_key(|(anchor, kind)| {
        (
            anchor.recipe_id,
            anchor.version_number,
            version_graph_frontier_rank(*kind),
        )
    });
    boundary_anchors.dedup();

    let mut frontiers = Vec::new();
    let mut work = VersionGraphPageReadWork::default();
    for (anchor, kind) in boundary_anchors {
        if !allowed_anchor_ids.contains(&anchor) {
            continue;
        }
        let fork_cursor = fork_continuations.get(&(anchor, kind)).copied();
        let page = load_graph_direction_page(
            pool,
            selected_recipe_id,
            viewer_id,
            VersionGraphDirectionRequest {
                anchor,
                kind,
                excluded_node_ids: loaded_node_ids,
                fork_cursor,
                limit: 0,
            },
        )
        .await?;
        work.absorb(page.work);
        if page.hidden_count > 0 {
            frontiers.push(crate::types::VersionGraphFrontier {
                anchor,
                kind,
                hidden_count: page.hidden_count,
                fork_cursor,
            });
        }
    }
    Ok((frontiers, work))
}

fn checked_graph_snapshot_byte_total(
    sizes: impl IntoIterator<Item = usize>,
    maximum: usize,
    label: &'static str,
) -> Result<usize, DbError> {
    let mut total = 0usize;
    for size in sizes {
        if size > crate::security::MAX_RECIPE_SNAPSHOT_BYTES {
            return Err(DbError::ResourceLimitExceeded(label));
        }
        total = total
            .checked_add(size)
            .ok_or(DbError::ResourceLimitExceeded(label))?;
        if total > maximum {
            return Err(DbError::ResourceLimitExceeded(label));
        }
    }
    Ok(total)
}

async fn load_bounded_version_graph_nodes(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    node_ids: &[crate::types::VersionGraphNodeId],
    context: &VersionGraphContext,
    maximum_encoded_bytes: usize,
    maximum_decoded_bytes: usize,
) -> Result<
    (
        Vec<crate::types::VersionGraphNode>,
        VersionGraphPageReadWork,
    ),
    DbError,
> {
    let mut requested_snapshot_ids = HashSet::new();
    for node_id in node_ids {
        if context.live_recipes.contains_key(&node_id.recipe_id) {
            requested_snapshot_ids.insert(*node_id);
            if node_id.version_number > 1 {
                requested_snapshot_ids.insert(crate::types::VersionGraphNodeId {
                    recipe_id: node_id.recipe_id,
                    version_number: node_id.version_number - 1,
                });
            }
        }
    }
    let mut requested_snapshot_ids = requested_snapshot_ids.into_iter().collect::<Vec<_>>();
    requested_snapshot_ids.sort_by_key(|node_id| (node_id.recipe_id, node_id.version_number));
    let recipe_ids = requested_snapshot_ids
        .iter()
        .map(|node_id| node_id.recipe_id)
        .collect::<Vec<_>>();
    let version_numbers = requested_snapshot_ids
        .iter()
        .map(|node_id| node_id.version_number)
        .collect::<Vec<_>>();
    let size_rows = if requested_snapshot_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, GraphSnapshotSize>(
            "WITH requested(recipe_id, version_number) AS (\
                 SELECT * FROM UNNEST($1::UUID[], $2::INTEGER[])\
             ) \
             SELECT version.recipe_id, version.version_number, \
                    octet_length(version.snapshot::text)::BIGINT AS snapshot_bytes \
               FROM recipe_versions AS version \
               JOIN requested USING (recipe_id, version_number) \
              ORDER BY version.recipe_id, version.version_number",
        )
        .bind(&recipe_ids)
        .bind(&version_numbers)
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
    };
    if size_rows.len() != requested_snapshot_ids.len() {
        return Err(DbError::ResourceLimitExceeded(
            "version graph snapshot lookahead",
        ));
    }
    let encoded_snapshot_bytes = checked_graph_snapshot_byte_total(
        size_rows
            .iter()
            .map(|row| usize::try_from(row.snapshot_bytes).unwrap_or(usize::MAX)),
        maximum_encoded_bytes,
        "version graph encoded snapshot bytes",
    )?;
    let versions = if requested_snapshot_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as::<_, BoundedRecipeVersionPayloadRow>(
            "WITH requested(recipe_id, version_number) AS (\
                 SELECT * FROM UNNEST($1::UUID[], $2::INTEGER[])\
             ) \
             SELECT version.recipe_id, version.version_number, \
                    version.snapshot::text AS snapshot_text, version.notes, version.created_at \
               FROM recipe_versions AS version \
               JOIN requested USING (recipe_id, version_number) \
              ORDER BY version.recipe_id, version.version_number",
        )
        .bind(&recipe_ids)
        .bind(&version_numbers)
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
    };
    let mut decoded_snapshot_bytes = 0usize;
    let mut snapshots = HashMap::new();
    for version in &versions {
        let snapshot = version.decoded_snapshot()?;
        let decoded_bytes = recipe_snapshot_decoded_heap_bytes(&snapshot)?;
        decoded_snapshot_bytes = decoded_snapshot_bytes.checked_add(decoded_bytes).ok_or(
            DbError::ResourceLimitExceeded("version graph decoded snapshot bytes"),
        )?;
        if decoded_snapshot_bytes > maximum_decoded_bytes {
            return Err(DbError::ResourceLimitExceeded(
                "version graph decoded snapshot bytes",
            ));
        }
        snapshots.insert(
            crate::types::VersionGraphNodeId {
                recipe_id: version.recipe_id,
                version_number: version.version_number,
            },
            (version, snapshot),
        );
    }

    let mut diff_operations = 0usize;
    let mut nodes = Vec::with_capacity(node_ids.len());
    for node_id in node_ids {
        if let Some(recipe) = context.live_recipes.get(&node_id.recipe_id) {
            let (version, snapshot) = snapshots.get(node_id).ok_or(DbError::VersionNotFound {
                recipe_id: node_id.recipe_id,
                version_number: node_id.version_number,
            })?;
            let change_summary = if node_id.version_number == 1 {
                "Initial version".to_string()
            } else {
                let previous_id = crate::types::VersionGraphNodeId {
                    recipe_id: node_id.recipe_id,
                    version_number: node_id.version_number - 1,
                };
                let (_, previous_snapshot) =
                    snapshots
                        .get(&previous_id)
                        .ok_or(DbError::VersionNotFound {
                            recipe_id: previous_id.recipe_id,
                            version_number: previous_id.version_number,
                        })?;
                diff_operations = diff_operations.saturating_add(1);
                bounded_snapshot_change_summary(previous_snapshot, snapshot)
            };
            nodes.push(crate::types::VersionGraphNode::visible(
                *node_id,
                crate::types::VersionGraphNodeContent {
                    title: snapshot.title.clone(),
                    owner_username: Some(recipe.author_username.clone()),
                    image: graph_image(&snapshot.images),
                    created_at: version.created_at,
                    change_summary,
                },
                graph_node_state(
                    node_id.recipe_id == selected_recipe_id,
                    recipe.current_version_number == Some(node_id.version_number),
                    false,
                ),
            ));
        } else {
            let metadata = context.synthetic_nodes.get(node_id).ok_or_else(|| {
                invalid_version_graph_expansion("deleted node metadata is unavailable")
            })?;
            nodes.push(crate::types::VersionGraphNode::visible(
                *node_id,
                crate::types::VersionGraphNodeContent {
                    title: metadata.title.clone(),
                    owner_username: metadata.owner_username.clone(),
                    image: None,
                    created_at: metadata.created_at,
                    change_summary: "Deleted recipe".to_string(),
                },
                graph_node_state(false, false, true),
            ));
        }
    }
    Ok((
        nodes,
        VersionGraphPageReadWork {
            snapshot_metadata_rows_returned: size_rows.len(),
            snapshot_payload_rows_returned: versions.len(),
            encoded_snapshot_bytes,
            decoded_snapshot_bytes,
            snapshots_decoded: snapshots.len(),
            diff_operations,
            ..Default::default()
        },
    ))
}

fn bounded_graph_page(
    mut graph: crate::types::VersionGraph,
    node_limit_reached: bool,
) -> crate::types::VersionGraph {
    deduplicate_and_sort_graph(&mut graph);
    let edge_limit_reached = graph.edges.len() > VERSION_GRAPH_PAGE_EDGE_LIMIT;
    graph.edges.truncate(VERSION_GRAPH_PAGE_EDGE_LIMIT);
    graph.truncation = crate::types::VersionGraphTruncation {
        is_truncated: node_limit_reached || edge_limit_reached,
        node_limit_reached,
        edge_limit_reached,
        depth_limit_reached: false,
        traversal_limit_reached: false,
    };
    graph
}

pub async fn get_initial_recipe_version_graph_with_work(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
) -> Result<BoundedVersionGraphPage, DbError> {
    let selected_rows = load_graph_recipe_access_rows(pool, &[selected_recipe_id]).await?;
    let selected = selected_rows
        .into_iter()
        .next()
        .filter(|recipe| graph_live_recipe_is_visible(recipe, selected_recipe_id, viewer_id))
        .ok_or(DbError::RecipeNotFound(selected_recipe_id))?;
    let current_version_number = selected
        .current_version_number
        .ok_or(DbError::RecipeNotFound(selected_recipe_id))?;
    let seed = crate::types::VersionGraphNodeId {
        recipe_id: selected_recipe_id,
        version_number: current_version_number,
    };
    let lineage_counts = load_version_graph_lineage_counts(pool, selected_recipe_id).await?;
    let (node_ids, discovery_work, fork_continuations) =
        discover_initial_version_graph_node_ids(pool, selected_recipe_id, viewer_id, seed).await?;
    let node_id_set = node_ids.iter().copied().collect::<HashSet<_>>();
    let context =
        load_version_graph_context(pool, selected_recipe_id, viewer_id, &node_id_set).await?;
    let (nodes, node_work) = load_bounded_version_graph_nodes(
        pool,
        selected_recipe_id,
        &node_ids,
        &context,
        crate::security::MAX_INITIAL_GRAPH_ENCODED_SNAPSHOT_BYTES,
        crate::security::MAX_INITIAL_GRAPH_DECODED_SNAPSHOT_BYTES,
    )
    .await?;
    let (frontiers, frontier_work) = build_bounded_version_graph_frontiers(
        pool,
        selected_recipe_id,
        viewer_id,
        &node_id_set,
        &node_id_set,
        &fork_continuations,
    )
    .await?;
    let node_limit_reached =
        node_ids.len() == INITIAL_VERSION_GRAPH_NODE_LIMIT && !frontiers.is_empty();
    let graph = bounded_graph_page(
        crate::types::VersionGraph {
            nodes,
            edges: context.edges.clone(),
            frontiers,
            truncation: Default::default(),
            lineage_counts,
        },
        node_limit_reached,
    );
    let mut work = VersionGraphPageReadWork {
        recipe_rows_returned: 1,
        ..Default::default()
    };
    work.absorb(discovery_work);
    work.absorb(context.work);
    work.absorb(node_work);
    work.absorb(frontier_work);
    Ok(BoundedVersionGraphPage { graph, work })
}

pub async fn expand_recipe_version_graph_with_work(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    frontier: crate::types::VersionGraphFrontier,
    loaded_node_ids: Vec<crate::types::VersionGraphNodeId>,
) -> Result<BoundedVersionGraphPage, DbError> {
    crate::security::validate_graph_expansion(&frontier, &loaded_node_ids)
        .map_err(|_| invalid_version_graph_expansion("request exceeds graph limits"))?;
    let loaded_node_id_set = loaded_node_ids.iter().copied().collect::<HashSet<_>>();
    let loaded_context =
        load_version_graph_context(pool, selected_recipe_id, viewer_id, &loaded_node_id_set)
            .await?;
    let direction_page = load_graph_direction_page(
        pool,
        selected_recipe_id,
        viewer_id,
        VersionGraphDirectionRequest {
            anchor: frontier.anchor,
            kind: frontier.kind,
            excluded_node_ids: &loaded_node_id_set,
            fork_cursor: frontier.fork_cursor,
            limit: VERSION_GRAPH_EXPANSION_NODE_LIMIT,
        },
    )
    .await?;
    if direction_page.hidden_count == 0 {
        return Err(invalid_version_graph_expansion(
            "frontier direction is not available",
        ));
    }
    if direction_page.hidden_count != frontier.hidden_count {
        return Err(invalid_version_graph_expansion("frontier is stale"));
    }
    let new_node_ids = direction_page.node_ids;
    let fork_continuations = direction_page
        .next_fork_cursor
        .map(|cursor| HashMap::from([((frontier.anchor, frontier.kind), cursor)]))
        .unwrap_or_default();
    let new_node_id_set = new_node_ids.iter().copied().collect::<HashSet<_>>();
    let combined_node_ids = loaded_node_id_set
        .union(&new_node_id_set)
        .copied()
        .collect::<HashSet<_>>();
    let context =
        load_version_graph_context(pool, selected_recipe_id, viewer_id, &combined_node_ids).await?;
    let (nodes, node_work) = load_bounded_version_graph_nodes(
        pool,
        selected_recipe_id,
        &new_node_ids,
        &context,
        crate::security::MAX_EXPANSION_GRAPH_ENCODED_SNAPSHOT_BYTES,
        crate::security::MAX_EXPANSION_GRAPH_DECODED_SNAPSHOT_BYTES,
    )
    .await?;
    let mut allowed_frontier_anchors = new_node_id_set.clone();
    allowed_frontier_anchors.insert(frontier.anchor);
    let (frontiers, frontier_work) = build_bounded_version_graph_frontiers(
        pool,
        selected_recipe_id,
        viewer_id,
        &combined_node_ids,
        &allowed_frontier_anchors,
        &fork_continuations,
    )
    .await?;
    let edges = context
        .edges
        .iter()
        .filter(|edge| {
            new_node_id_set.contains(&edge.source()) || new_node_id_set.contains(&edge.target())
        })
        .cloned()
        .collect::<Vec<_>>();
    let lineage_counts = load_version_graph_lineage_counts(pool, selected_recipe_id).await?;
    let node_limit_reached = new_node_ids.len() == VERSION_GRAPH_EXPANSION_NODE_LIMIT
        && direction_page.hidden_count > VERSION_GRAPH_EXPANSION_NODE_LIMIT;
    let graph = bounded_graph_page(
        crate::types::VersionGraph {
            nodes,
            edges,
            frontiers,
            truncation: Default::default(),
            lineage_counts,
        },
        node_limit_reached,
    );
    let mut work = loaded_context.work;
    work.absorb(direction_page.work);
    work.absorb(context.work);
    work.absorb(node_work);
    work.absorb(frontier_work);
    Ok(BoundedVersionGraphPage { graph, work })
}

/// Load the privacy-filtered graph and return its deterministic initial batch.
pub async fn get_initial_recipe_version_graph(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
) -> Result<crate::types::VersionGraph, DbError> {
    Ok(
        get_initial_recipe_version_graph_with_work(pool, selected_recipe_id, viewer_id)
            .await?
            .graph,
    )
}

/// Expand exactly one validated frontier without returning already loaded nodes.
pub async fn expand_recipe_version_graph(
    pool: &PgPool,
    selected_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
    frontier: crate::types::VersionGraphFrontier,
    loaded_node_ids: Vec<crate::types::VersionGraphNodeId>,
) -> Result<crate::types::VersionGraph, DbError> {
    Ok(expand_recipe_version_graph_with_work(
        pool,
        selected_recipe_id,
        viewer_id,
        frontier,
        loaded_node_ids,
    )
    .await?
    .graph)
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

/// Compare immutable snapshots without assuming they belong to the same recipe.
pub async fn compare_recipe_version_snapshots(
    pool: &PgPool,
    from: crate::types::VersionGraphNodeId,
    to: crate::types::VersionGraphNodeId,
) -> Result<Vec<SnapshotChange>, DbError> {
    let from_snapshot = get_recipe_version(pool, from.recipe_id, from.version_number)
        .await?
        .decoded_snapshot()?;
    let to_snapshot = get_recipe_version(pool, to.recipe_id, to.version_number)
        .await?
        .decoded_snapshot()?;
    compute_diff(&from_snapshot, &to_snapshot)
}

fn resource_limit_as_i64(limit: usize) -> i64 {
    i64::try_from(limit).expect("recipe security limits fit in PostgreSQL bigint")
}

async fn lock_resource_owner(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), DbError> {
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(DbError::Query)?;
    if owner.is_none() {
        return Err(DbError::Query(sqlx::Error::RowNotFound));
    }
    Ok(())
}

async fn lock_owned_recipe(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
) -> Result<String, DbError> {
    sqlx::query_scalar(
        "SELECT visibility FROM recipes
         WHERE id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::RecipeNotFound(recipe_id))
}

async fn enforce_active_draft_quota(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), DbError> {
    let draft_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recipe_drafts WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(DbError::Query)?;
    if draft_count >= resource_limit_as_i64(MAX_ACTIVE_DRAFTS_PER_USER) {
        return Err(DbError::ResourceLimitExceeded("active drafts per user"));
    }
    Ok(())
}

async fn enforce_version_quotas(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
) -> Result<(), DbError> {
    let recipe_version_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM recipe_versions WHERE recipe_id = $1")
            .bind(recipe_id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(DbError::Query)?;
    if recipe_version_count >= resource_limit_as_i64(MAX_VERSIONS_PER_RECIPE) {
        return Err(DbError::ResourceLimitExceeded(
            "immutable versions per recipe",
        ));
    }

    let user_version_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM recipe_versions version
         INNER JOIN recipes recipe ON recipe.id = version.recipe_id
         WHERE recipe.user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(DbError::Query)?;
    if user_version_count >= resource_limit_as_i64(MAX_VERSIONS_PER_USER) {
        return Err(DbError::ResourceLimitExceeded(
            "immutable versions per user",
        ));
    }
    Ok(())
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
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
    version_number: i32,
    snapshot: &RecipeSnapshot,
    notes: Option<&str>,
) -> Result<RecipeVersion, DbError> {
    if notes.is_some_and(|value| value.chars().count() > MAX_VERSION_NOTE_CHARS) {
        return Err(DbError::InvalidRecipeSnapshot(
            "version note exceeds an application limit".to_string(),
        ));
    }
    lock_resource_owner(transaction, user_id).await?;
    let visibility = lock_owned_recipe(transaction, recipe_id, user_id).await?;
    validate_snapshot_for_publish(snapshot, &visibility)?;
    enforce_version_quotas(transaction, recipe_id, user_id).await?;
    super::retain_media_urls(
        transaction,
        user_id,
        super::MediaPurpose::Recipe,
        &recipe_snapshot_media_urls(snapshot),
    )
    .await?;

    let snapshot = snapshot_to_json(snapshot)?;
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes) \
         VALUES ($1, $2, $3, $4) RETURNING {VERSION_COLUMNS}"
    ))
    .bind(recipe_id)
    .bind(version_number)
    .bind(snapshot)
    .bind(notes)
    .fetch_one(&mut **transaction)
    .await
    .map_err(DbError::Query)
}

#[allow(clippy::too_many_arguments)]
async fn insert_recipe_draft_with_source(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
    base_version_number: Option<i32>,
    snapshot: &RecipeDraftPayload,
    forked_from_recipe_id: Option<Uuid>,
    forked_from_version_number: Option<i32>,
    forked_from_title: Option<&str>,
    fork_message: Option<&str>,
) -> Result<RecipeDraft, DbError> {
    if fork_message.is_some_and(|value| value.chars().count() > MAX_FORK_MESSAGE_CHARS) {
        return Err(DbError::InvalidRecipeSnapshot(
            "fork message exceeds an application limit".to_string(),
        ));
    }
    validate_draft_payload(snapshot)?;
    lock_resource_owner(transaction, user_id).await?;
    enforce_active_draft_quota(transaction, user_id).await?;
    super::retain_media_urls(
        transaction,
        user_id,
        super::MediaPurpose::Recipe,
        &recipe_snapshot_media_urls(&snapshot.content),
    )
    .await?;
    if forked_from_version_number.is_some() {
        super::retain_inherited_recipe_media_urls(
            transaction,
            &recipe_snapshot_media_urls(&snapshot.content),
        )
        .await?;
    }

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
    .fetch_optional(&mut **transaction)
    .await
    .map_err(DbError::Query)?;
    inserted.ok_or_else(|| {
        recipe_id
            .map(DbError::RecipeNotFound)
            .unwrap_or(DbError::DraftNotFound(Uuid::nil()))
    })
}

pub async fn insert_recipe_draft(
    pool: &PgPool,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
    base_version_number: Option<i32>,
    payload: &RecipeDraftPayload,
) -> Result<RecipeDraft, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let draft = insert_recipe_draft_with_source(
        &mut transaction,
        user_id,
        recipe_id,
        base_version_number,
        payload,
        None,
        None,
        None,
        None,
    )
    .await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(draft)
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
         WHERE user_id = $1 ORDER BY updated_at DESC LIMIT $2"
    ))
    .bind(user_id)
    .bind(resource_limit_as_i64(MAX_ACTIVE_DRAFTS_PER_USER))
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
    validate_draft_payload(payload)?;
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    super::retain_media_urls(
        &mut transaction,
        user_id,
        super::MediaPurpose::Recipe,
        &recipe_snapshot_media_urls(&payload.content),
    )
    .await?;
    let snapshot = serde_json::to_value(payload).map_err(DbError::SerdeJson)?;
    let updated = sqlx::query_as::<_, RecipeDraft>(&format!(
        "UPDATE recipe_drafts SET snapshot = $4, revision = revision + 1, updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND revision = $3 RETURNING {DRAFT_COLUMNS}"
    ))
    .bind(draft_id)
    .bind(user_id)
    .bind(expected_revision)
    .bind(snapshot)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if let Some(updated) = updated {
        transaction.commit().await.map_err(DbError::Query)?;
        return Ok(updated);
    }

    let actual = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM recipe_drafts WHERE id = $1 AND user_id = $2",
    )
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
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
    lock_resource_owner(&mut transaction, user_id).await?;
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

    let draft = insert_recipe_draft_with_source(
        &mut transaction,
        user_id,
        Some(recipe_id),
        Some(latest.version_number),
        &snapshot,
        None,
        None,
        None,
        None,
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
    lock_resource_owner(&mut transaction, user_id).await?;
    let recipe =
        insert_recipe_from_snapshot(&mut transaction, user_id, &snapshot, visibility).await?;
    insert_recipe_version(&mut transaction, recipe.id, user_id, 1, &snapshot, notes).await?;
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
    lock_resource_owner(&mut transaction, user_id).await?;
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
    insert_recipe_version(&mut transaction, recipe.id, user_id, 1, &snapshot, notes).await?;
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
    lock_resource_owner(&mut transaction, user_id).await?;
    let visibility = lock_owned_recipe(&mut transaction, recipe_id, user_id).await?;

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
    validate_snapshot_for_publish(&snapshot, &visibility)?;
    insert_recipe_version(
        &mut transaction,
        recipe_id,
        user_id,
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
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    lock_resource_owner(&mut transaction, user_id).await?;
    let visibility = lock_owned_recipe(&mut transaction, recipe_id, user_id).await?;
    validate_snapshot_for_publish(&snapshot, &visibility)?;
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;
    if latest.version_number != expected_latest_version {
        return Err(DbError::VersionConflict {
            recipe_id,
            expected: expected_latest_version,
            actual: latest.version_number,
        });
    }
    insert_recipe_version(
        &mut transaction,
        recipe_id,
        user_id,
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
    lock_resource_owner(&mut transaction, user_id).await?;
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
        &mut transaction,
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

/// Project fork attribution through the source's current visibility.
///
/// Deleted sources use their captured deletion-time visibility. Inaccessible
/// live or deleted sources retain only the existence of the attribution and
/// the fork author's message; neither title nor reusable recipe ID crosses the
/// response boundary.
pub async fn get_fork_attribution_for_viewer(
    pool: &PgPool,
    forked_recipe_id: Uuid,
    viewer_id: Option<Uuid>,
) -> Result<Option<crate::types::ForkAttribution>, DbError> {
    let row = sqlx::query_as::<_, ForkAttributionRow>(
        "SELECT relationship.original_title, relationship.original_owner_id, \
                relationship.original_visibility, relationship.message, \
                source.id AS live_recipe_id, source.user_id AS live_owner_id, \
                source.visibility AS live_visibility \
         FROM fork_relationships AS relationship \
         LEFT JOIN recipes AS source \
           ON source.id = COALESCE(relationship.original_recipe_key, \
                                   relationship.original_recipe_id) \
         WHERE relationship.forked_recipe_id = $1 OR relationship.forked_recipe_key = $1 \
         ORDER BY relationship.created_at DESC LIMIT 1",
    )
    .bind(forked_recipe_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;

    Ok(row.map(|row| {
        let (original_recipe_id, original_title) = if let Some(live_recipe_id) = row.live_recipe_id
        {
            let source_is_visible = row.live_visibility.as_deref() == Some("public")
                || viewer_id.is_some_and(|viewer| row.live_owner_id == Some(viewer));
            if source_is_visible {
                (Some(live_recipe_id), Some(row.original_title))
            } else {
                (None, None)
            }
        } else {
            let source_is_visible = row.original_visibility.as_deref() == Some("public")
                || viewer_id.is_some_and(|viewer| row.original_owner_id == Some(viewer));
            if source_is_visible {
                (None, Some(row.original_title))
            } else {
                (None, None)
            }
        };
        crate::types::ForkAttribution {
            original_recipe_id,
            original_title,
            message: row.message,
        }
    }))
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
    validate_recipe_visibility(visibility)?;
    crate::security::validate_recipe_snapshot(snapshot, visibility, RecipeValidationMode::Publish)
        .map_err(|_| {
            DbError::InvalidRecipeSnapshot(
                "recipe snapshot exceeds an application limit".to_string(),
            )
        })?;
    validate_snapshot_content_for_publish(snapshot)
}

fn validate_draft_payload(payload: &RecipeDraftPayload) -> Result<(), DbError> {
    let visibility = payload.initial_visibility.as_deref().unwrap_or("private");
    crate::security::validate_recipe_snapshot(
        &payload.content,
        visibility,
        RecipeValidationMode::Draft,
    )
    .map_err(|_| {
        DbError::InvalidRecipeSnapshot("recipe draft exceeds an application limit".to_string())
    })
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

    fn graph_node(
        recipe_id: Uuid,
        version_number: i32,
        is_selected_recipe: bool,
        is_current_version: bool,
    ) -> crate::types::VersionGraphNode {
        crate::types::VersionGraphNode::redacted(
            crate::types::VersionGraphNodeId {
                recipe_id,
                version_number,
            },
            crate::types::VersionGraphNodeState {
                is_selected_recipe,
                is_current_version,
                is_deleted: false,
            },
        )
    }

    fn version_chain_graph(recipe_id: Uuid, version_count: i32) -> crate::types::VersionGraph {
        let nodes = (1..=version_count)
            .map(|version_number| {
                graph_node(
                    recipe_id,
                    version_number,
                    true,
                    version_number == version_count,
                )
            })
            .collect::<Vec<_>>();
        let edges = nodes
            .windows(2)
            .map(|pair| {
                crate::types::VersionGraphEdge::version(pair[0].id(), pair[1].id()).unwrap()
            })
            .collect();
        crate::types::VersionGraph {
            nodes,
            edges,
            ..Default::default()
        }
    }

    #[test]
    fn initial_version_graph_batch_returns_at_most_twenty_five_nodes() {
        let graph = version_chain_graph(Uuid::from_u128(100), 31);

        let initial = initial_version_graph_batch(&graph);

        assert_eq!(initial.nodes.len(), INITIAL_VERSION_GRAPH_NODE_LIMIT);
        assert_eq!(initial.nodes[0].id().version_number, 31);
        assert_eq!(initial.nodes[24].id().version_number, 7);
        assert_eq!(
            initial.frontiers,
            vec![crate::types::VersionGraphFrontier {
                anchor: initial.nodes[24].id(),
                kind: crate::types::VersionGraphFrontierKind::OlderVersions,
                hidden_count: 6,
                fork_cursor: None,
            }]
        );
    }

    #[test]
    fn version_graph_expansion_returns_at_most_five_new_nodes_without_loaded_duplicates() {
        let graph = version_chain_graph(Uuid::from_u128(101), 31);
        let initial = initial_version_graph_batch(&graph);
        let loaded_node_ids = initial
            .nodes
            .iter()
            .map(crate::types::VersionGraphNode::id)
            .collect::<Vec<_>>();

        let expanded =
            expanded_version_graph_batch(&graph, initial.frontiers[0], loaded_node_ids.clone())
                .unwrap();

        assert_eq!(expanded.nodes.len(), VERSION_GRAPH_EXPANSION_NODE_LIMIT);
        assert!(expanded
            .nodes
            .iter()
            .all(|node| !loaded_node_ids.contains(&node.id())));
        assert_eq!(
            expanded
                .nodes
                .iter()
                .map(|node| node.id().version_number)
                .collect::<Vec<_>>(),
            vec![6, 5, 4, 3, 2]
        );
    }

    #[tokio::test]
    async fn graph_pages_decode_only_new_nodes_and_predecessor_lookahead() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "bounded_graph").await;
        let recipe =
            create_published_recipe(&pool, user.id, &snapshot("Version 1", "public"), None)
                .await
                .unwrap();
        for version_number in 2..=60 {
            sqlx::query(
                "INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes) \
                 VALUES ($1, $2, $3, NULL)",
            )
            .bind(recipe.id)
            .bind(version_number)
            .bind(
                snapshot_to_json(&snapshot(&format!("Version {version_number}"), "public"))
                    .unwrap(),
            )
            .execute(&pool)
            .await
            .unwrap();
        }

        let initial = get_initial_recipe_version_graph_with_work(&pool, recipe.id, None)
            .await
            .unwrap();
        assert_eq!(initial.graph.nodes.len(), INITIAL_VERSION_GRAPH_NODE_LIMIT);
        assert_eq!(initial.work.snapshot_metadata_rows_returned, 26);
        assert_eq!(initial.work.snapshot_payload_rows_returned, 26);
        assert_eq!(initial.work.snapshots_decoded, 26);
        assert_eq!(initial.work.diff_operations, 25);
        assert!(initial.work.relationship_rows_returned <= 2 * INITIAL_VERSION_GRAPH_NODE_LIMIT);
        assert!(
            initial.work.encoded_snapshot_bytes
                <= crate::security::MAX_INITIAL_GRAPH_ENCODED_SNAPSHOT_BYTES
        );
        assert!(
            initial.work.decoded_snapshot_bytes
                <= crate::security::MAX_INITIAL_GRAPH_DECODED_SNAPSHOT_BYTES
        );
        let loaded = initial
            .graph
            .nodes
            .iter()
            .map(crate::types::VersionGraphNode::id)
            .collect::<Vec<_>>();
        let older_frontier = initial
            .graph
            .frontiers
            .iter()
            .copied()
            .find(|frontier| frontier.kind == crate::types::VersionGraphFrontierKind::OlderVersions)
            .expect("older-version frontier");

        let mut stale_frontier = older_frontier;
        stale_frontier.hidden_count += 1;
        assert!(expand_recipe_version_graph_with_work(
            &pool,
            recipe.id,
            None,
            stale_frontier,
            loaded.clone(),
        )
        .await
        .is_err());

        let expansion = expand_recipe_version_graph_with_work(
            &pool,
            recipe.id,
            None,
            older_frontier,
            loaded.clone(),
        )
        .await
        .unwrap();
        assert_eq!(
            expansion.graph.nodes.len(),
            VERSION_GRAPH_EXPANSION_NODE_LIMIT
        );
        assert!(expansion
            .graph
            .nodes
            .iter()
            .all(|node| !loaded.contains(&node.id())));
        assert_eq!(expansion.work.snapshot_metadata_rows_returned, 6);
        assert_eq!(expansion.work.snapshot_payload_rows_returned, 6);
        assert_eq!(expansion.work.snapshots_decoded, 6);
        assert_eq!(expansion.work.diff_operations, 5);
        assert!(
            expansion.work.encoded_snapshot_bytes
                <= crate::security::MAX_EXPANSION_GRAPH_ENCODED_SNAPSHOT_BYTES
        );
        assert!(
            expansion.work.decoded_snapshot_bytes
                <= crate::security::MAX_EXPANSION_GRAPH_DECODED_SNAPSHOT_BYTES
        );
    }

    #[tokio::test]
    async fn forged_cross_recipe_loaded_state_fails_before_snapshot_page_loading() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "forged_graph_state").await;
        let selected =
            create_published_recipe(&pool, user.id, &snapshot("Selected", "public"), None)
                .await
                .unwrap();
        let unrelated =
            create_published_recipe(&pool, user.id, &snapshot("Unrelated", "public"), None)
                .await
                .unwrap();
        publish_snapshot(
            &pool,
            selected.id,
            user.id,
            1,
            &snapshot("Selected v2", "public").content,
            None,
        )
        .await
        .unwrap();
        for version_number in 3..=30 {
            sqlx::query(
                "INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes) \
                 VALUES ($1, $2, $3, NULL)",
            )
            .bind(selected.id)
            .bind(version_number)
            .bind(
                snapshot_to_json(&snapshot(&format!("Selected v{version_number}"), "public"))
                    .unwrap(),
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let initial = get_initial_recipe_version_graph(&pool, selected.id, None)
            .await
            .unwrap();
        let frontier = initial.frontiers[0];
        let mut loaded = initial
            .nodes
            .iter()
            .map(crate::types::VersionGraphNode::id)
            .collect::<Vec<_>>();
        loaded.push(crate::types::VersionGraphNodeId {
            recipe_id: unrelated.id,
            version_number: 1,
        });

        assert!(
            expand_recipe_version_graph_with_work(&pool, selected.id, None, frontier, loaded,)
                .await
                .is_err()
        );
    }

    #[test]
    fn version_graph_expansion_is_specific_to_the_requested_frontier() {
        let root_recipe_id = Uuid::from_u128(110);
        let first_child_id = Uuid::from_u128(111);
        let second_child_id = Uuid::from_u128(112);
        let root = graph_node(root_recipe_id, 1, true, true);
        let newer = graph_node(root_recipe_id, 2, true, false);
        let first_child = graph_node(first_child_id, 1, false, true);
        let second_child = graph_node(second_child_id, 1, false, true);
        let graph = crate::types::VersionGraph {
            nodes: vec![
                root.clone(),
                newer.clone(),
                first_child.clone(),
                second_child.clone(),
            ],
            edges: vec![
                crate::types::VersionGraphEdge::version(root.id(), newer.id()).unwrap(),
                crate::types::VersionGraphEdge::fork(root_recipe_id, 1, first_child_id).unwrap(),
                crate::types::VersionGraphEdge::fork(root_recipe_id, 1, second_child_id).unwrap(),
            ],
            ..Default::default()
        };
        let frontier = crate::types::VersionGraphFrontier {
            anchor: root.id(),
            kind: crate::types::VersionGraphFrontierKind::ChildForks,
            hidden_count: 2,
            fork_cursor: None,
        };

        let expanded = expanded_version_graph_batch(&graph, frontier, vec![root.id()]).unwrap();

        assert_eq!(
            expanded
                .nodes
                .iter()
                .map(crate::types::VersionGraphNode::id)
                .collect::<HashSet<_>>(),
            HashSet::from([first_child.id(), second_child.id()])
        );
        assert!(!expanded.nodes.iter().any(|node| node.id() == newer.id()));
        assert!(expanded.frontiers.iter().any(|candidate| {
            candidate.anchor == root.id()
                && candidate.kind == crate::types::VersionGraphFrontierKind::NewerVersions
        }));
    }

    #[test]
    fn version_graph_expansion_rejects_duplicate_loaded_ids_and_wrong_directions() {
        let graph = version_chain_graph(Uuid::from_u128(120), 2);
        let current = graph.nodes[1].id();
        let older_frontier = crate::types::VersionGraphFrontier {
            anchor: current,
            kind: crate::types::VersionGraphFrontierKind::OlderVersions,
            hidden_count: 1,
            fork_cursor: None,
        };
        assert!(
            expanded_version_graph_batch(&graph, older_frontier, vec![current, current]).is_err()
        );

        let wrong_direction = crate::types::VersionGraphFrontier {
            kind: crate::types::VersionGraphFrontierKind::ChildForks,
            ..older_frontier
        };
        assert!(expanded_version_graph_batch(&graph, wrong_direction, vec![current]).is_err());
    }

    #[test]
    fn version_graph_lineage_counts_distinct_recipes_in_each_direction() {
        let selected_recipe_id = Uuid::from_u128(1);
        let parent_recipe_id = Uuid::from_u128(2);
        let root_recipe_id = Uuid::from_u128(3);
        let first_fork_id = Uuid::from_u128(4);
        let second_fork_id = Uuid::from_u128(5);
        let grandchild_id = Uuid::from_u128(6);
        let edges = vec![
            crate::types::VersionGraphEdge::fork(root_recipe_id, 1, parent_recipe_id).unwrap(),
            crate::types::VersionGraphEdge::fork(parent_recipe_id, 1, selected_recipe_id).unwrap(),
            crate::types::VersionGraphEdge::fork(selected_recipe_id, 1, first_fork_id).unwrap(),
            crate::types::VersionGraphEdge::fork(selected_recipe_id, 2, second_fork_id).unwrap(),
            crate::types::VersionGraphEdge::fork(first_fork_id, 1, grandchild_id).unwrap(),
        ];

        let counts = version_graph_lineage_counts(selected_recipe_id, &edges);

        assert_eq!(counts.ancestors.count, 2);
        assert!(!counts.ancestors.is_capped);
        assert_eq!(counts.immediate_descendants.count, 2);
        assert!(!counts.immediate_descendants.is_capped);
        assert_eq!(counts.descendants.count, 3);
        assert!(!counts.descendants.is_capped);
    }

    #[test]
    fn version_graph_lineage_counts_cap_at_fifty() {
        let selected_recipe_id = Uuid::from_u128(1);
        let edges = (2..=52)
            .map(|recipe_number| {
                crate::types::VersionGraphEdge::fork(
                    selected_recipe_id,
                    1,
                    Uuid::from_u128(recipe_number),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        let counts = version_graph_lineage_counts(selected_recipe_id, &edges);

        assert_eq!(counts.ancestors.count, 0);
        assert_eq!(counts.immediate_descendants.count, 50);
        assert!(counts.immediate_descendants.is_capped);
        assert_eq!(counts.descendants.count, 50);
        assert!(counts.descendants.is_capped);
    }

    #[test]
    fn version_graph_lineage_counts_stop_cycles_without_counting_the_selected_recipe() {
        let selected_recipe_id = Uuid::from_u128(1);
        let child_recipe_id = Uuid::from_u128(2);
        let edges = vec![
            crate::types::VersionGraphEdge::fork(selected_recipe_id, 1, child_recipe_id).unwrap(),
            crate::types::VersionGraphEdge::fork(child_recipe_id, 1, selected_recipe_id).unwrap(),
        ];

        let counts = version_graph_lineage_counts(selected_recipe_id, &edges);

        assert_eq!(counts.ancestors.count, 1);
        assert_eq!(counts.immediate_descendants.count, 1);
        assert_eq!(counts.descendants.count, 1);
    }

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
    async fn version_summary_pages_read_only_one_page_and_adjacent_lookahead() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "bounded_history").await;
        let recipe =
            create_published_recipe(&pool, user.id, &snapshot("Version 1", "public"), None)
                .await
                .unwrap();
        for version_number in 2..=60 {
            sqlx::query(
                "INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(recipe.id)
            .bind(version_number)
            .bind(
                snapshot_to_json(&snapshot(&format!("Version {version_number}"), "public"))
                    .unwrap(),
            )
            .bind(format!("Notes {version_number}"))
            .execute(&pool)
            .await
            .unwrap();
        }

        let first = get_recipe_version_summary_page(
            &pool,
            recipe.id,
            None,
            crate::security::MAX_VERSION_HISTORY_PAGE_SIZE,
        )
        .await
        .unwrap();
        assert_eq!(first.page.items.len(), 25);
        assert_eq!(first.page.items[0].version_number, 60);
        assert!(first.page.items[0].is_current);
        assert_eq!(first.page.items[24].version_number, 36);
        assert_eq!(first.page.next_before_version_number, Some(36));
        assert_eq!(first.work.snapshot_metadata_rows_returned, 26);
        assert_eq!(first.work.snapshot_payload_rows_returned, 26);
        assert_eq!(first.work.snapshots_decoded, 26);
        assert_eq!(first.work.diff_operations, 25);
        assert!(first.work.encoded_snapshot_bytes > 0);
        assert!(
            first.work.encoded_snapshot_bytes
                <= crate::security::MAX_VERSION_HISTORY_ENCODED_SNAPSHOT_BYTES
        );
        assert!(
            first.work.decoded_snapshot_bytes
                <= crate::security::MAX_VERSION_HISTORY_DECODED_SNAPSHOT_BYTES
        );

        let second = get_recipe_version_summary_page(
            &pool,
            recipe.id,
            first.page.next_before_version_number,
            crate::security::MAX_VERSION_HISTORY_PAGE_SIZE,
        )
        .await
        .unwrap();
        assert_eq!(second.page.items[0].version_number, 35);
        assert_eq!(second.page.items[24].version_number, 11);
        assert_eq!(second.page.next_before_version_number, Some(11));
        assert!(second.page.items.iter().all(|summary| !summary.is_current));

        let final_page = get_recipe_version_summary_page(
            &pool,
            recipe.id,
            second.page.next_before_version_number,
            crate::security::MAX_VERSION_HISTORY_PAGE_SIZE,
        )
        .await
        .unwrap();
        assert_eq!(final_page.page.items.len(), 10);
        assert_eq!(final_page.page.items[0].version_number, 10);
        assert_eq!(final_page.page.items[9].version_number, 1);
        assert_eq!(final_page.page.items[9].change_summary, "Initial version");
        assert_eq!(final_page.page.next_before_version_number, None);
        assert_eq!(final_page.work.snapshot_metadata_rows_returned, 10);
        assert_eq!(final_page.work.snapshot_payload_rows_returned, 10);
        assert_eq!(final_page.work.snapshots_decoded, 10);
        assert_eq!(final_page.work.diff_operations, 9);
    }

    #[test]
    fn version_snapshot_byte_accounting_accepts_exact_limit_and_rejects_one_over() {
        assert_eq!(checked_snapshot_byte_total([12, 20], 32).unwrap(), 32);
        assert!(matches!(
            checked_snapshot_byte_total([12, 21], 32),
            Err(DbError::ResourceLimitExceeded(
                "version history encoded snapshot bytes"
            ))
        ));
        assert!(matches!(
            checked_snapshot_byte_total([usize::MAX, 1], usize::MAX),
            Err(DbError::ResourceLimitExceeded(
                "version history encoded snapshot bytes"
            ))
        ));
        assert!(matches!(
            checked_snapshot_byte_total(
                [crate::security::MAX_RECIPE_SNAPSHOT_BYTES + 1],
                crate::security::MAX_VERSION_HISTORY_ENCODED_SNAPSHOT_BYTES,
            ),
            Err(DbError::ResourceLimitExceeded(
                "version history encoded snapshot bytes"
            ))
        ));
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
        let fork_draft =
            create_fork_draft(&pool, source.id, fork_owner.id, Some("Private fork note"))
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
    async fn version_graph_omits_non_public_direct_fork_identifiers_for_non_owners() {
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
        let unlisted_draft =
            create_fork_draft(&pool, source.id, fork_owner.id, Some("Unlisted fork note"))
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

        let public_graph = get_initial_recipe_version_graph(&pool, source.id, None)
            .await
            .unwrap();
        let serialized = serde_json::to_string(&public_graph).unwrap();
        for inaccessible_recipe_id in [private_fork.id, unlisted_fork.id] {
            assert!(public_graph
                .nodes
                .iter()
                .all(|node| node.id().recipe_id != inaccessible_recipe_id));
            assert!(public_graph.edges.iter().all(|edge| {
                edge.source().recipe_id != inaccessible_recipe_id
                    && edge.target().recipe_id != inaccessible_recipe_id
            }));
            assert!(!serialized.contains(&inaccessible_recipe_id.to_string()));
        }
        assert!(!serialized.contains("Secret fork"));
        assert!(!serialized.contains("Unlisted fork"));
        assert!(!serialized.contains("Private fork note"));
        assert!(!serialized.contains("Unlisted fork note"));
        assert_eq!(public_graph.lineage_counts.immediate_descendants.count, 2);

        for hidden_recipe_id in [private_fork.id, unlisted_fork.id] {
            let hidden_node_id = crate::types::VersionGraphNodeId {
                recipe_id: hidden_recipe_id,
                version_number: 1,
            };
            let forged_frontier = crate::types::VersionGraphFrontier {
                anchor: hidden_node_id,
                kind: crate::types::VersionGraphFrontierKind::ParentFork,
                hidden_count: 1,
                fork_cursor: None,
            };
            assert!(expand_recipe_version_graph(
                &pool,
                source.id,
                None,
                forged_frontier,
                vec![hidden_node_id],
            )
            .await
            .is_err());
        }

        let known_unlisted_graph = get_initial_recipe_version_graph(&pool, unlisted_fork.id, None)
            .await
            .unwrap();
        assert!(known_unlisted_graph
            .nodes
            .iter()
            .any(|node| node.id().recipe_id == unlisted_fork.id));
        assert!(serde_json::to_string(&known_unlisted_graph)
            .unwrap()
            .contains(&unlisted_fork.id.to_string()));

        assert!(
            get_initial_recipe_version_graph(&pool, private_fork.id, None)
                .await
                .is_err()
        );

        let owner_graph = get_initial_recipe_version_graph(&pool, source.id, Some(fork_owner.id))
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
    async fn child_fork_expansion_reads_only_five_new_snapshot_rows() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "bounded_fork_source").await;
        let fork_owner = owner(&pool, "bounded_fork_children").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Bounded fork source", "public"),
            None,
        )
        .await
        .unwrap();
        for child_number in 1..=30 {
            publish_fork(
                &pool,
                source.id,
                &fork_owner,
                &format!("Child {child_number}"),
            )
            .await;
        }

        let initial = get_initial_recipe_version_graph_with_work(&pool, source.id, None)
            .await
            .unwrap();
        assert_eq!(initial.graph.nodes.len(), INITIAL_VERSION_GRAPH_NODE_LIMIT);
        let child_frontier = initial
            .graph
            .frontiers
            .iter()
            .copied()
            .find(|frontier| {
                frontier.anchor.recipe_id == source.id
                    && frontier.kind == crate::types::VersionGraphFrontierKind::ChildForks
            })
            .expect("remaining child forks");
        assert_eq!(child_frontier.hidden_count, 6);
        let loaded = initial
            .graph
            .nodes
            .iter()
            .map(crate::types::VersionGraphNode::id)
            .collect::<Vec<_>>();

        let expansion = expand_recipe_version_graph_with_work(
            &pool,
            source.id,
            None,
            child_frontier,
            loaded.clone(),
        )
        .await
        .unwrap();
        assert_eq!(expansion.graph.nodes.len(), 5);
        assert_eq!(expansion.work.snapshot_metadata_rows_returned, 5);
        assert_eq!(expansion.work.snapshot_payload_rows_returned, 5);
        assert_eq!(expansion.work.snapshots_decoded, 5);
        assert!(expansion
            .graph
            .nodes
            .iter()
            .all(|node| !loaded.contains(&node.id())));
        assert_eq!(
            expansion
                .graph
                .frontiers
                .iter()
                .find(|frontier| {
                    frontier.anchor.recipe_id == source.id
                        && frontier.kind == crate::types::VersionGraphFrontierKind::ChildForks
                })
                .map(|frontier| frontier.hidden_count),
            Some(1)
        );

        let final_frontier = expansion
            .graph
            .frontiers
            .iter()
            .copied()
            .find(|frontier| {
                frontier.anchor.recipe_id == source.id
                    && frontier.kind == crate::types::VersionGraphFrontierKind::ChildForks
            })
            .expect("final child fork");
        let mut final_loaded = loaded;
        final_loaded.extend(
            expansion
                .graph
                .nodes
                .iter()
                .map(crate::types::VersionGraphNode::id),
        );
        let final_expansion = expand_recipe_version_graph_with_work(
            &pool,
            source.id,
            None,
            final_frontier,
            final_loaded,
        )
        .await
        .unwrap();
        assert_eq!(final_expansion.graph.nodes.len(), 1);
        assert!(final_expansion.graph.frontiers.iter().all(|frontier| {
            frontier.anchor.recipe_id != source.id
                || frontier.kind != crate::types::VersionGraphFrontierKind::ChildForks
        }));
    }

    #[tokio::test]
    async fn fork_cursor_crosses_hidden_raw_pages_without_unbounded_reads() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "cursor_source").await;
        let fork_owner = owner(&pool, "cursor_children").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Cursor source", "public"),
            None,
        )
        .await
        .unwrap();

        for child_number in 1_u128..=30 {
            let hidden = publish_fork_with_visibility(
                &pool,
                source.id,
                &fork_owner,
                &format!("Hidden {child_number}"),
                "private",
            )
            .await;
            sqlx::query("UPDATE fork_relationships SET id = $1 WHERE forked_recipe_key = $2")
                .bind(Uuid::from_u128(child_number))
                .bind(hidden.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        let visible = publish_fork(&pool, source.id, &fork_owner, "Visible after hidden").await;
        sqlx::query("UPDATE fork_relationships SET id = $1 WHERE forked_recipe_key = $2")
            .bind(Uuid::from_u128(u128::MAX - 1))
            .bind(visible.id)
            .execute(&pool)
            .await
            .unwrap();

        let initial = get_initial_recipe_version_graph_with_work(&pool, source.id, None)
            .await
            .unwrap();
        let mut loaded = initial
            .graph
            .nodes
            .iter()
            .map(crate::types::VersionGraphNode::id)
            .collect::<Vec<_>>();
        assert!(initial.work.relationship_rows_returned <= 4 * INITIAL_VERSION_GRAPH_NODE_LIMIT);
        let mut graph = initial.graph;
        let mut expansions = 0;
        loop {
            let Some(frontier) = graph.frontiers.iter().copied().find(|frontier| {
                frontier.anchor.recipe_id == source.id
                    && frontier.kind == crate::types::VersionGraphFrontierKind::ChildForks
            }) else {
                break;
            };
            let expansion = expand_recipe_version_graph_with_work(
                &pool,
                source.id,
                None,
                frontier,
                loaded.clone(),
            )
            .await
            .unwrap();
            assert!(
                expansion.work.relationship_rows_returned <= 4 * INITIAL_VERSION_GRAPH_NODE_LIMIT
            );
            loaded.extend(
                expansion
                    .graph
                    .nodes
                    .iter()
                    .map(crate::types::VersionGraphNode::id),
            );
            graph = expansion.graph;
            expansions += 1;
            assert!(expansions <= 3, "frontier must make bounded progress");
        }

        assert_eq!(expansions, 2);
        assert!(loaded.iter().any(|node_id| node_id.recipe_id == visible.id));
    }

    #[test]
    fn graph_snapshot_byte_accounting_accepts_exact_limit_and_rejects_one_over() {
        assert_eq!(
            checked_graph_snapshot_byte_total([12, 20], 32, "graph bytes").unwrap(),
            32
        );
        assert!(matches!(
            checked_graph_snapshot_byte_total([12, 21], 32, "graph bytes"),
            Err(DbError::ResourceLimitExceeded("graph bytes"))
        ));
        assert!(matches!(
            checked_graph_snapshot_byte_total([usize::MAX, 1], usize::MAX, "graph bytes"),
            Err(DbError::ResourceLimitExceeded("graph bytes"))
        ));
        assert!(matches!(
            checked_graph_snapshot_byte_total(
                [crate::security::MAX_RECIPE_SNAPSHOT_BYTES + 1],
                crate::security::MAX_INITIAL_GRAPH_ENCODED_SNAPSHOT_BYTES,
                "graph bytes",
            ),
            Err(DbError::ResourceLimitExceeded("graph bytes"))
        ));
    }

    #[test]
    fn decoded_snapshot_heap_accounting_measures_allocations_not_json_length() {
        let mut decoded = snapshot("x", "public").content;
        decoded.title.reserve(4_096);

        let heap_bytes = recipe_snapshot_decoded_heap_bytes(&decoded).unwrap();
        let json_bytes = serde_json::to_vec(&decoded).unwrap().len();

        assert!(heap_bytes >= decoded.title.capacity());
        assert!(heap_bytes > json_bytes);
    }

    #[test]
    fn bounded_snapshot_change_summary_matches_the_full_diff_summary() {
        let older = snapshot("Old title", "public").content;
        let mut newer = older.clone();
        newer.title = "New title".to_string();
        newer.tags = vec!["dinner".to_string()];

        assert_eq!(
            bounded_snapshot_change_summary(&older, &newer),
            crate::db::diff::change_summary(&compute_diff(&older, &newer).unwrap())
        );
    }

    #[test]
    fn graph_frontier_counts_use_only_a_bounded_probe() {
        assert_eq!(graph_fork_raw_page_limit(0), 5);
        assert_eq!(graph_fork_raw_page_limit(5), 5);
        assert_eq!(graph_fork_raw_page_limit(25), 25);
        assert_eq!(graph_fork_query_limit(0), 6);
        assert_eq!(graph_fork_query_limit(5), 6);
        assert_eq!(graph_fork_query_limit(25), 26);
        assert_eq!(bounded_graph_frontier_hidden_count(0), 0);
        assert_eq!(bounded_graph_frontier_hidden_count(6), 6);
        assert_eq!(bounded_graph_frontier_hidden_count(7), 6);

        let source = include_str!("versioning.rs");
        let fork_page = source
            .split("async fn load_fork_direction_page")
            .nth(1)
            .expect("fork direction page")
            .split("async fn load_graph_direction_page")
            .next()
            .expect("fork direction page body");
        assert!(!fork_page.contains("COUNT(*)"));
        assert!(!fork_page.contains("OR relationship.id"));

        let schema = include_str!("../../migrations/schema.sql");
        assert!(schema.contains("idx_forks_original_version_cursor"));
        assert!(schema.contains("idx_forks_result_cursor"));
    }

    #[tokio::test]
    async fn version_graph_omits_hidden_branches_and_disconnected_descendants() {
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
        let serialized = serde_json::to_string(&graph).unwrap();
        assert!(!serialized.contains(&hidden_branch.id.to_string()));
        assert!(!serialized.contains("Private intermediate"));
        assert!(!serialized.contains(&public_descendant.id.to_string()));
        assert!(!serialized.contains("Public descendant"));
        assert_eq!(graph.lineage_counts.descendants.count, 2);
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
        assert_eq!(depth_limited.lineage_counts.immediate_descendants.count, 2);
        assert_eq!(depth_limited.lineage_counts.descendants.count, 3);

        let grandchild_graph = get_recipe_version_graph(&pool, grandchild.id, None)
            .await
            .unwrap();
        assert_eq!(grandchild_graph.lineage_counts.ancestors.count, 2);
        assert_eq!(
            grandchild_graph.lineage_counts.immediate_descendants.count,
            0
        );
        assert_eq!(grandchild_graph.lineage_counts.descendants.count, 0);

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
        assert_eq!(
            traversal_limited.lineage_counts.immediate_descendants.count,
            2
        );
        assert_eq!(traversal_limited.lineage_counts.descendants.count, 3);
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
        assert_eq!(node_limited.lineage_counts.immediate_descendants.count, 1);
        assert_eq!(node_limited.lineage_counts.descendants.count, 1);

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
        assert_eq!(edge_limited.lineage_counts.immediate_descendants.count, 1);
        assert_eq!(edge_limited.lineage_counts.descendants.count, 1);
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
    async fn compares_immutable_snapshots_across_recipes_in_requested_direction() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "cross_recipe_compare").await;
        let source =
            create_published_recipe(&pool, user.id, &snapshot("Source recipe", "public"), None)
                .await
                .unwrap();
        let fork =
            create_published_recipe(&pool, user.id, &snapshot("Fork recipe", "public"), None)
                .await
                .unwrap();

        let changes = compare_recipe_version_snapshots(
            &pool,
            crate::types::VersionGraphNodeId {
                recipe_id: source.id,
                version_number: 1,
            },
            crate::types::VersionGraphNodeId {
                recipe_id: fork.id,
                version_number: 1,
            },
        )
        .await
        .expect("cross-recipe snapshots compare");

        assert!(changes.iter().any(|change| {
            change.path == "/title"
                && change.old_value == Some(serde_json::json!("Source recipe"))
                && change.value == Some(serde_json::json!("Fork recipe"))
        }));
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
    async fn fork_attribution_redacts_live_and_deleted_private_sources_for_non_owners() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "attribution_source").await;
        let fork_owner = owner(&pool, "attribution_fork").await;
        let source =
            create_published_recipe(&pool, source_owner.id, &snapshot("Source", "public"), None)
                .await
                .unwrap();
        let draft = create_fork_draft(&pool, source.id, fork_owner.id, Some("My variation"))
            .await
            .unwrap();
        let fork = publish_recipe_draft(&pool, draft.id, fork_owner.id, None)
            .await
            .unwrap();

        let public = get_fork_attribution_for_viewer(&pool, fork.id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(public.original_recipe_id, Some(source.id));
        assert_eq!(public.original_title.as_deref(), Some("Source"));

        crate::db::update_recipe_visibility(&pool, source.id, source_owner.id, "unlisted")
            .await
            .unwrap();
        let unlisted_redacted = get_fork_attribution_for_viewer(&pool, fork.id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unlisted_redacted.original_recipe_id, None);
        assert_eq!(unlisted_redacted.original_title, None);

        crate::db::update_recipe_visibility(&pool, source.id, source_owner.id, "private")
            .await
            .unwrap();
        let redacted = get_fork_attribution_for_viewer(&pool, fork.id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(redacted.original_recipe_id, None);
        assert_eq!(redacted.original_title, None);
        assert_eq!(redacted.message.as_deref(), Some("My variation"));

        let owner_view = get_fork_attribution_for_viewer(&pool, fork.id, Some(source_owner.id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(owner_view.original_recipe_id, Some(source.id));
        assert_eq!(owner_view.original_title.as_deref(), Some("Source"));

        crate::db::delete_recipe(&pool, source.id, source_owner.id)
            .await
            .unwrap();
        let deleted_redacted = get_fork_attribution_for_viewer(&pool, fork.id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(deleted_redacted.original_recipe_id, None);
        assert_eq!(deleted_redacted.original_title, None);

        let deleted_owner = get_fork_attribution_for_viewer(&pool, fork.id, Some(source_owner.id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(deleted_owner.original_recipe_id, None);
        assert_eq!(deleted_owner.original_title.as_deref(), Some("Source"));

        sqlx::query(
            "UPDATE fork_relationships SET original_owner_id = NULL \
             WHERE forked_recipe_key = $1 OR forked_recipe_id = $1",
        )
        .bind(fork.id)
        .execute(&pool)
        .await
        .unwrap();
        let missing_owner_snapshot = get_fork_attribution_for_viewer(&pool, fork.id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(missing_owner_snapshot.original_recipe_id, None);
        assert_eq!(missing_owner_snapshot.original_title, None);

        let public_deleted_source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Public deleted source", "public"),
            None,
        )
        .await
        .unwrap();
        let public_deleted_fork =
            publish_fork(&pool, public_deleted_source.id, &fork_owner, "Public fork").await;
        crate::db::delete_recipe(&pool, public_deleted_source.id, source_owner.id)
            .await
            .unwrap();
        let deleted_public = get_fork_attribution_for_viewer(&pool, public_deleted_fork.id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(deleted_public.original_recipe_id, None);
        assert_eq!(
            deleted_public.original_title.as_deref(),
            Some("Public deleted source")
        );
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
        let private_draft = create_fork_draft(
            &pool,
            source.id,
            fork_owner.id,
            Some("Sensitive lineage note"),
        )
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
        assert_eq!(public_view.descendants[0].node.owner_username, None);
        assert_eq!(public_view.descendants[0].node.image_url, None);
        assert_eq!(public_view.descendants[0].node.fork_message, None);
        let serialized_public_view = serde_json::to_string(&public_view).unwrap();
        assert!(!serialized_public_view.contains(&private_recipe.id.to_string()));
        assert!(!serialized_public_view.contains("Secret branch"));
        assert!(!serialized_public_view.contains("Sensitive lineage note"));

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
        // Keep the draft valid under CP5's early payload bounds, then force a
        // database failure at the final tag-write stage to retain coverage of
        // transaction rollback after the version and recipe writes begin.
        sqlx::query(
            "ALTER TABLE recipe_tags ADD CONSTRAINT reject_late_failure_tag \
             CHECK (tag <> 'late-failure')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut late_failure_snapshot = snapshot("Must roll back", "public");
        late_failure_snapshot.tags = vec!["late-failure".to_string()];
        update_recipe_draft(&pool, draft.id, user.id, 0, &late_failure_snapshot)
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

        let references = crate::db::get_all_owned_image_references(&pool)
            .await
            .unwrap();
        for expected_url in [
            "https://cdn.test/historical_full.webp",
            "https://cdn.test/historical_thumb.webp",
            "https://cdn.test/draft_full.webp",
        ] {
            assert!(references.iter().any(|reference| {
                reference.owner_id == user.id
                    && reference.image_url == expected_url
                    && !reference.allows_inherited_owner
            }));
        }
    }

    #[tokio::test]
    async fn orphan_reference_scan_preserves_fork_draft_media_after_source_is_deleted() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "draft_image_source").await;
        let fork_owner = owner(&pool, "draft_image_fork").await;
        let object_id = Uuid::new_v4();
        let storage = crate::storage::StorageClient::for_test();
        let inherited_url = storage.object_url(&format!(
            "recipes/{}/{object_id}_full.webp",
            source_owner.id
        ));

        let mut source_snapshot = snapshot("Draft source", "public");
        source_snapshot.images.push(crate::types::RecipeImage::new(
            inherited_url.clone(),
            None,
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        let source = create_published_recipe(&pool, source_owner.id, &source_snapshot, None)
            .await
            .unwrap();
        create_fork_draft(&pool, source.id, fork_owner.id, None)
            .await
            .unwrap();

        sqlx::query("DELETE FROM recipes WHERE id = $1")
            .bind(source.id)
            .execute(&pool)
            .await
            .unwrap();

        let references = crate::db::get_all_owned_image_references(&pool)
            .await
            .unwrap();
        assert!(references.iter().any(|reference| {
            reference.owner_id == fork_owner.id
                && reference.image_url == inherited_url
                && reference.allows_inherited_owner
        }));

        let retained =
            crate::worker::orphan_cleanup::collect_retained_media_keys(&storage, &references);
        assert!(retained.contains(&format!(
            "recipes/{}/{object_id}_full.webp",
            source_owner.id
        )));
        assert!(retained.contains(&format!(
            "recipes/{}/{object_id}_thumb.webp",
            source_owner.id
        )));
    }

    #[tokio::test]
    async fn orphan_reference_scan_preserves_inherited_media_after_sources_are_deleted() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let root_owner = owner(&pool, "image_root").await;
        let first_fork_owner = owner(&pool, "image_fork_one").await;
        let second_fork_owner = owner(&pool, "image_fork_two").await;
        let inherited_url = "https://cdn.test/inherited_full.webp".to_string();

        let mut root_snapshot = snapshot("Root", "public");
        root_snapshot.images.push(crate::types::RecipeImage::new(
            inherited_url.clone(),
            None,
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        let root = create_published_recipe(&pool, root_owner.id, &root_snapshot, None)
            .await
            .unwrap();

        let first_draft = create_fork_draft(&pool, root.id, first_fork_owner.id, None)
            .await
            .unwrap();
        let mut first_payload = first_draft.decoded_snapshot().unwrap();
        first_payload.title = "First fork".to_string();
        update_recipe_draft(
            &pool,
            first_draft.id,
            first_fork_owner.id,
            first_draft.revision,
            &first_payload,
        )
        .await
        .unwrap();
        let first_fork = publish_recipe_draft(&pool, first_draft.id, first_fork_owner.id, None)
            .await
            .unwrap();

        let second_draft = create_fork_draft(&pool, first_fork.id, second_fork_owner.id, None)
            .await
            .unwrap();
        let mut second_payload = second_draft.decoded_snapshot().unwrap();
        second_payload.title = "Second fork".to_string();
        update_recipe_draft(
            &pool,
            second_draft.id,
            second_fork_owner.id,
            second_draft.revision,
            &second_payload,
        )
        .await
        .unwrap();
        let second_fork = publish_recipe_draft(&pool, second_draft.id, second_fork_owner.id, None)
            .await
            .unwrap();

        sqlx::query("DELETE FROM recipes WHERE id = $1 OR id = $2")
            .bind(root.id)
            .bind(first_fork.id)
            .execute(&pool)
            .await
            .unwrap();

        let references = crate::db::get_all_owned_image_references(&pool)
            .await
            .unwrap();
        for expected_owner in [root_owner.id, first_fork_owner.id, second_fork_owner.id] {
            assert!(references.iter().any(|reference| {
                reference.owner_id == expected_owner
                    && reference.image_url == inherited_url
                    && !reference.allows_inherited_owner
            }));
        }
        assert!(crate::db::get_recipe_by_id(&pool, second_fork.id)
            .await
            .unwrap()
            .is_some());
    }
}
