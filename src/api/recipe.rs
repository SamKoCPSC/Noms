//! Recipe CRUD server functions.
//!
//! All functions require authentication via session cookie.
//! Functions accessing individual recipes verify ownership.

#![allow(clippy::too_many_arguments)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::types::{RecipeImage, RecipeStep};

#[cfg(feature = "server")]
fn enforce_graph_budget(viewer_id: Option<uuid::Uuid>) -> Result<(), ServerFnError> {
    crate::security::enforce_request_budget(viewer_id, crate::security::RequestClass::Graph)
        .map_err(|_| ServerFnError::new("Graph request rate limit exceeded"))
}

#[cfg(feature = "server")]
fn enforce_mutation_budget(user_id: uuid::Uuid) -> Result<(), ServerFnError> {
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))
}

#[cfg(feature = "server")]
fn enforce_publication_budget(user_id: uuid::Uuid) -> Result<(), ServerFnError> {
    crate::security::enforce_user_request_budget(
        user_id,
        crate::security::RequestClass::Publication,
    )
    .map_err(|_| ServerFnError::new("Publication rate limit exceeded"))
}

#[cfg(feature = "server")]
async fn bounded_database_operation<T>(
    operation: impl std::future::Future<Output = Result<T, crate::db::DbError>>,
) -> Result<T, ServerFnError> {
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, operation)
        .await
        .map_err(|_| ServerFnError::new("Database operation timed out"))?
        .map_err(|error| ServerFnError::new(error.to_string()))
}

#[cfg(feature = "server")]
fn editor_to_draft_payload(
    editor: crate::types::RecipeEditorState,
) -> crate::db::diff::RecipeDraftPayload {
    crate::db::diff::RecipeDraftPayload {
        initial_visibility: Some(editor.visibility),
        content: crate::db::diff::RecipeSnapshot {
            title: editor.title,
            description: editor.description,
            commentary: editor.commentary,
            prep_time_minutes: editor.prep_time_minutes,
            cook_time_minutes: editor.cook_time_minutes,
            servings: editor.servings,
            ingredients: editor.ingredients,
            instructions: editor.instructions,
            equipment: editor.equipment,
            images: editor.images,
            tags: editor.tags,
        },
    }
}

#[cfg(feature = "server")]
fn snapshot_to_editor(
    snapshot: crate::db::diff::RecipeSnapshot,
    visibility: String,
) -> crate::types::RecipeEditorState {
    crate::types::RecipeEditorState {
        title: snapshot.title,
        description: snapshot.description,
        commentary: snapshot.commentary,
        prep_time_minutes: snapshot.prep_time_minutes,
        cook_time_minutes: snapshot.cook_time_minutes,
        servings: snapshot.servings,
        ingredients: snapshot.ingredients,
        instructions: snapshot.instructions,
        equipment: snapshot.equipment,
        images: snapshot.images,
        visibility,
        tags: snapshot.tags,
    }
}

#[cfg(feature = "server")]
fn draft_to_view(
    draft: crate::db::RecipeDraft,
) -> Result<crate::types::RecipeDraftView, crate::db::DbError> {
    let payload = draft.decoded_snapshot()?;
    let editor = snapshot_to_editor(
        payload.content,
        payload
            .initial_visibility
            .unwrap_or_else(|| "private".to_string()),
    );
    Ok(crate::types::RecipeDraftView {
        id: draft.id,
        recipe_id: draft.recipe_id,
        base_version_number: draft.base_version_number,
        revision: draft.revision,
        editor,
        forked_from_recipe_id: draft.forked_from_recipe_id,
        forked_from_version_number: draft.forked_from_version_number,
        forked_from_title: draft.forked_from_title,
        fork_message: draft.fork_message,
        created_at: draft.created_at,
        updated_at: draft.updated_at,
    })
}

#[cfg(feature = "server")]
struct RecipeCreateInput {
    title: String,
    description: Option<String>,
    commentary: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: Vec<crate::types::RecipeIngredient>,
    instructions: Vec<RecipeStep>,
    equipment: Vec<crate::types::RecipeEquipment>,
    images: Vec<RecipeImage>,
    tags: Vec<String>,
    visibility: String,
}

#[cfg(feature = "server")]
struct RecipeUpdateInput {
    title: Option<String>,
    description: Option<String>,
    commentary: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<Vec<crate::types::RecipeIngredient>>,
    instructions: Option<Vec<RecipeStep>>,
    equipment: Option<Vec<crate::types::RecipeEquipment>>,
    images: Option<Vec<RecipeImage>>,
    tags: Option<Vec<String>>,
    visibility: Option<String>,
}

/// Create a new recipe for the authenticated user.
///
/// Tags are inserted after the recipe is created.
#[allow(clippy::too_many_arguments)]
#[server]
pub async fn create_recipe(
    expected_account_id: Option<uuid::Uuid>,
    title: String,
    description: Option<String>,
    commentary: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: Vec<crate::types::RecipeIngredient>,
    instructions: Vec<crate::types::RecipeStep>,
    equipment: Vec<crate::types::RecipeEquipment>,
    images: Vec<crate::types::RecipeImage>,
    tags: Vec<String>,
    visibility: String,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    enforce_publication_budget(user_id)?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();
    create_recipe_for_user(
        &pool,
        &storage,
        user_id,
        RecipeCreateInput {
            title,
            description,
            commentary,
            prep_time_minutes,
            cook_time_minutes,
            servings,
            ingredients,
            instructions,
            equipment,
            images,
            tags,
            visibility,
        },
    )
    .await
}

/// Get a recipe by ID. Two-phase access: ownership-gated first, then public/unlisted fallback.
#[server]
pub async fn get_recipe(recipe_id: String) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // First try ownership-gated lookup (works for all visibility levels if owner)
    match crate::db::get_recipe_by_id_and_owner(&pool, recipe_id, user_id).await {
        Ok(recipe) => Ok(recipe),
        Err(crate::db::DbError::RecipeNotFound(_)) => {
            // Not the owner — try public/unlisted access
            crate::db::get_recipe_by_id_public(&pool, recipe_id)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .ok_or_else(|| ServerFnError::new("Recipe not found"))
        }
        Err(e) => Err(ServerFnError::new(e.to_string())),
    }
}

/// Update an existing recipe (ownership-gated).
#[allow(clippy::too_many_arguments)]
#[server]
pub async fn update_recipe(
    expected_account_id: Option<uuid::Uuid>,
    recipe_id: String,
    title: Option<String>,
    description: Option<String>,
    commentary: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<Vec<crate::types::RecipeIngredient>>,
    instructions: Option<Vec<crate::types::RecipeStep>>,
    equipment: Option<Vec<crate::types::RecipeEquipment>>,
    images: Option<Vec<crate::types::RecipeImage>>,
    tags: Option<Vec<String>>,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    enforce_publication_budget(user_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();
    update_recipe_for_user(
        &pool,
        &storage,
        user_id,
        recipe_id,
        RecipeUpdateInput {
            title,
            description,
            commentary,
            prep_time_minutes,
            cook_time_minutes,
            servings,
            ingredients,
            instructions,
            equipment,
            images,
            tags,
            visibility: None,
        },
    )
    .await
}

/// Change recipe-level visibility without creating or publishing a content version.
#[server]
pub async fn update_recipe_visibility(
    expected_account_id: Option<uuid::Uuid>,
    recipe_id: String,
    visibility: String,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    enforce_mutation_budget(user_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    bounded_database_operation(crate::db::update_recipe_visibility(
        &crate::db::get_pool(),
        recipe_id,
        user_id,
        &visibility,
    ))
    .await
}

/// Start an incomplete, owner-only draft for a new recipe.
#[server]
pub async fn create_recipe_draft(
    expected_account_id: Option<uuid::Uuid>,
    editor: crate::types::RecipeEditorState,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    crate::security::validate_recipe_editor(&editor, crate::security::RecipeValidationMode::Draft)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let storage = crate::storage::get_storage_client();
    validate_recipe_media_references(&storage, &editor.images, &editor.instructions)?;
    let draft = bounded_database_operation(crate::db::create_new_recipe_draft(
        &crate::db::get_pool(),
        user_id,
        &editor_to_draft_payload(editor),
    ))
    .await?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

/// Load the owner's existing edit draft, or copy the latest published snapshot.
#[server]
pub async fn create_edit_draft(
    expected_account_id: Option<uuid::Uuid>,
    recipe_id: String,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    enforce_mutation_budget(user_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let draft = bounded_database_operation(crate::db::create_edit_draft(
        &crate::db::get_pool(),
        recipe_id,
        user_id,
    ))
    .await?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn get_recipe_draft_api(
    expected_account_id: Option<uuid::Uuid>,
    draft_id: String,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    let draft = crate::db::get_recipe_draft(&crate::db::get_pool(), draft_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

/// Revision-guarded autosave. This only updates `recipe_drafts`.
#[server]
pub async fn save_recipe_draft(
    expected_account_id: Option<uuid::Uuid>,
    draft_id: String,
    expected_revision: i64,
    editor: crate::types::RecipeEditorState,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    crate::security::validate_recipe_editor(&editor, crate::security::RecipeValidationMode::Draft)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    let current_draft = crate::db::get_recipe_draft(&crate::db::get_pool(), draft_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let storage = crate::storage::get_storage_client();
    validate_recipe_media_references(&storage, &editor.images, &editor.instructions)?;
    let mut payload = editor_to_draft_payload(editor);
    if current_draft.recipe_id.is_some() {
        payload.initial_visibility = None;
    }
    let draft = bounded_database_operation(crate::db::update_recipe_draft(
        &crate::db::get_pool(),
        draft_id,
        user_id,
        expected_revision,
        &payload,
    ))
    .await?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn discard_recipe_draft(
    expected_account_id: Option<uuid::Uuid>,
    draft_id: String,
) -> Result<(), ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    enforce_mutation_budget(user_id)?;
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    bounded_database_operation(crate::db::delete_recipe_draft(
        &crate::db::get_pool(),
        draft_id,
        user_id,
    ))
    .await
}

#[server]
pub async fn publish_recipe_draft(
    expected_account_id: Option<uuid::Uuid>,
    draft_id: String,
    notes: Option<String>,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    enforce_publication_budget(user_id)?;
    if notes
        .as_deref()
        .is_some_and(|value| value.chars().count() > crate::security::MAX_VERSION_NOTE_CHARS)
    {
        return Err(ServerFnError::new("Version note is too long"));
    }
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    let pool = crate::db::get_pool();
    let draft = crate::db::get_recipe_draft(&pool, draft_id, user_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let payload = draft
        .decoded_snapshot()
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let visibility = payload
        .initial_visibility
        .clone()
        .unwrap_or_else(|| "private".to_string());
    let editor = snapshot_to_editor(payload.content.clone(), visibility);
    crate::security::validate_recipe_editor(
        &editor,
        crate::security::RecipeValidationMode::Publish,
    )
    .map_err(|error| ServerFnError::new(error.to_string()))?;
    let storage = crate::storage::get_storage_client();
    validate_recipe_media_references(
        &storage,
        &payload.content.images,
        &payload.content.instructions,
    )?;
    bounded_database_operation(crate::db::publish_recipe_draft(
        &pool,
        draft_id,
        user_id,
        notes.as_deref(),
    ))
    .await
}

#[server]
pub async fn list_recipe_drafts() -> Result<Vec<crate::types::RecipeDraftView>, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    bounded_database_operation(crate::db::list_recipe_drafts(
        &crate::db::get_pool(),
        user_id,
    ))
    .await?
    .into_iter()
    .map(|draft| draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string())))
    .collect()
}

#[server]
pub async fn get_recipe_versions_api(
    recipe_id: String,
    before_version_number: Option<i32>,
    page_size: usize,
) -> Result<crate::types::RecipeVersionSummaryPage, ServerFnError> {
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    crate::security::validate_version_history_page(before_version_number, page_size)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    crate::security::run_with_graph_history_work_budget(async move {
        let started_at = std::time::Instant::now();
        let recipe_id = uuid::Uuid::parse_str(&recipe_id)
            .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
        let pool = crate::db::get_pool();
        bounded_database_operation(crate::db::get_recipe_by_id(&pool, recipe_id))
            .await?
            .filter(|recipe| viewer_may_read_recipe(&recipe.visibility, recipe.user_id, viewer_id))
            .ok_or_else(|| ServerFnError::new("Recipe not found"))?;
        let bounded_page = bounded_database_operation(crate::db::get_recipe_version_summary_page(
            &pool,
            recipe_id,
            before_version_number,
            page_size,
        ))
        .await?;
        tracing::info!(
            snapshot_metadata_rows_returned = bounded_page.work.snapshot_metadata_rows_returned,
            snapshot_payload_rows_returned = bounded_page.work.snapshot_payload_rows_returned,
            encoded_snapshot_bytes = bounded_page.work.encoded_snapshot_bytes,
            decoded_snapshot_bytes = bounded_page.work.decoded_snapshot_bytes,
            snapshots_decoded = bounded_page.work.snapshots_decoded,
            diff_operations = bounded_page.work.diff_operations,
            returned_items = bounded_page.page.items.len(),
            has_more = bounded_page.page.next_before_version_number.is_some(),
            duration_ms = started_at.elapsed().as_millis() as u64,
            "completed bounded recipe version-summary page"
        );
        Ok(bounded_page.page)
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))?
}

#[cfg(all(feature = "server", test))]
fn version_summary_image_url(images: &[crate::types::RecipeImage]) -> Option<String> {
    images
        .first()
        .map(|image| image.display_url(true).to_string())
}

#[cfg(all(feature = "server", test))]
fn version_graph_image(images: &[crate::types::RecipeImage]) -> Option<crate::types::RecipeImage> {
    images.first().cloned()
}

#[cfg(feature = "server")]
fn viewer_may_read_recipe(
    recipe_visibility: &str,
    recipe_owner_id: uuid::Uuid,
    viewer_id: Option<uuid::Uuid>,
) -> bool {
    recipe_visibility != "private" || viewer_id == Some(recipe_owner_id)
}

#[cfg(all(feature = "server", test))]
fn build_recipe_version_graph(
    recipe_id: uuid::Uuid,
    owner_username: &str,
    mut versions: Vec<crate::db::RecipeVersion>,
) -> Result<crate::types::VersionGraph, crate::db::DbError> {
    versions.sort_by_key(|version| version.version_number);
    let snapshots = versions
        .iter()
        .map(crate::db::RecipeVersion::decoded_snapshot)
        .collect::<Result<Vec<_>, _>>()?;
    let current_version_number = versions.last().map(|version| version.version_number);

    let nodes = versions
        .iter()
        .zip(&snapshots)
        .enumerate()
        .map(|(index, (version, snapshot))| {
            let id = crate::types::VersionGraphNodeId {
                recipe_id,
                version_number: version.version_number,
            };
            let state = crate::types::VersionGraphNodeState {
                is_selected_recipe: true,
                is_current_version: current_version_number == Some(version.version_number),
                is_deleted: false,
            };
            let change_summary = match index.checked_sub(1) {
                Some(previous_index) => crate::db::diff::change_summary(
                    &crate::db::diff::compute_diff(&snapshots[previous_index], snapshot)?,
                ),
                None => "Initial version".to_string(),
            };
            Ok(crate::types::VersionGraphNode::visible(
                id,
                crate::types::VersionGraphNodeContent {
                    title: snapshot.title.clone(),
                    owner_username: Some(owner_username.to_string()),
                    image: version_graph_image(&snapshot.images),
                    created_at: version.created_at,
                    change_summary,
                },
                state,
            ))
        })
        .collect::<Result<Vec<_>, crate::db::DbError>>()?;

    let edges = nodes
        .windows(2)
        .map(|pair| {
            crate::types::VersionGraphEdge::version(pair[0].id(), pair[1].id()).ok_or_else(|| {
                crate::db::DbError::InvalidRecipeSnapshot(
                    "recipe versions are not consecutive".to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(crate::types::VersionGraph {
        nodes,
        edges,
        frontiers: Vec::new(),
        truncation: crate::types::VersionGraphTruncation::default(),
        lineage_counts: crate::types::VersionGraphLineageCounts::default(),
    })
}

/// Return the selected recipe's immutable snapshots as a normalized graph.
#[server]
pub async fn get_recipe_version_graph(
    recipe_id: String,
) -> Result<crate::types::VersionGraph, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let pool = crate::db::get_pool();
    let started_at = std::time::Instant::now();
    let bounded_page = crate::security::run_with_graph_history_work_budget(async move {
        crate::security::run_with_database_budget(
            crate::security::DATABASE_EXECUTION_BUDGET,
            crate::db::get_initial_recipe_version_graph_with_work(&pool, recipe_id, viewer_id),
        )
        .await
        .map_err(|_| ServerFnError::new("Database operation timed out"))?
        .map_err(|error| match error {
            crate::db::DbError::RecipeNotFound(_) => ServerFnError::new("Recipe not found"),
            other => ServerFnError::new(other.to_string()),
        })
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))??;
    tracing::info!(
        recipe_rows_returned = bounded_page.work.recipe_rows_returned,
        validation_rows_returned = bounded_page.work.validation_rows_returned,
        relationship_rows_returned = bounded_page.work.relationship_rows_returned,
        relationship_queries = bounded_page.work.relationship_queries,
        snapshot_metadata_rows_returned = bounded_page.work.snapshot_metadata_rows_returned,
        snapshot_payload_rows_returned = bounded_page.work.snapshot_payload_rows_returned,
        encoded_snapshot_bytes = bounded_page.work.encoded_snapshot_bytes,
        decoded_snapshot_bytes = bounded_page.work.decoded_snapshot_bytes,
        snapshots_decoded = bounded_page.work.snapshots_decoded,
        diff_operations = bounded_page.work.diff_operations,
        returned_nodes = bounded_page.graph.nodes.len(),
        returned_edges = bounded_page.graph.edges.len(),
        duration_ms = started_at.elapsed().as_millis() as u64,
        "completed bounded initial version-graph page"
    );
    Ok(bounded_page.graph)
}

/// Expand one validated boundary of the progressively loaded version graph.
#[server]
pub async fn expand_recipe_version_graph(
    recipe_id: String,
    frontier: crate::types::VersionGraphFrontier,
    loaded_node_ids: Vec<crate::types::VersionGraphNodeId>,
) -> Result<crate::types::VersionGraph, ServerFnError> {
    crate::security::validate_graph_expansion(&frontier, &loaded_node_ids)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let pool = crate::db::get_pool();
    let started_at = std::time::Instant::now();
    let bounded_page = crate::security::run_with_graph_history_work_budget(async move {
        crate::security::run_with_database_budget(
            crate::security::DATABASE_EXECUTION_BUDGET,
            crate::db::expand_recipe_version_graph_with_work(
                &pool,
                recipe_id,
                viewer_id,
                frontier,
                loaded_node_ids,
            ),
        )
        .await
        .map_err(|_| ServerFnError::new("Database operation timed out"))?
        .map_err(|error| match error {
            crate::db::DbError::RecipeNotFound(_) => ServerFnError::new("Recipe not found"),
            other => ServerFnError::new(other.to_string()),
        })
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))??;
    tracing::info!(
        recipe_rows_returned = bounded_page.work.recipe_rows_returned,
        validation_rows_returned = bounded_page.work.validation_rows_returned,
        relationship_rows_returned = bounded_page.work.relationship_rows_returned,
        relationship_queries = bounded_page.work.relationship_queries,
        snapshot_metadata_rows_returned = bounded_page.work.snapshot_metadata_rows_returned,
        snapshot_payload_rows_returned = bounded_page.work.snapshot_payload_rows_returned,
        encoded_snapshot_bytes = bounded_page.work.encoded_snapshot_bytes,
        decoded_snapshot_bytes = bounded_page.work.decoded_snapshot_bytes,
        snapshots_decoded = bounded_page.work.snapshots_decoded,
        diff_operations = bounded_page.work.diff_operations,
        returned_nodes = bounded_page.graph.nodes.len(),
        returned_edges = bounded_page.graph.edges.len(),
        duration_ms = started_at.elapsed().as_millis() as u64,
        "completed bounded version-graph expansion"
    );
    Ok(bounded_page.graph)
}

#[server]
pub async fn get_recipe_version_api(
    recipe_id: String,
    version_number: i32,
) -> Result<crate::types::RecipeVersionView, ServerFnError> {
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let pool = crate::db::get_pool();
    crate::security::run_with_graph_history_work_budget(async move {
        let recipe = bounded_database_operation(crate::db::get_recipe_by_id(&pool, recipe_id))
            .await?
            .filter(|recipe| viewer_may_read_recipe(&recipe.visibility, recipe.user_id, viewer_id))
            .ok_or_else(|| ServerFnError::new("Recipe not found"))?;
        let version = bounded_database_operation(crate::db::get_recipe_version(
            &pool,
            recipe_id,
            version_number,
        ))
        .await?;
        let snapshot = version
            .decoded_snapshot()
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let latest =
            bounded_database_operation(crate::db::get_latest_recipe_version(&pool, recipe_id))
                .await?;
        let changes_from_previous = if version_number > 1 {
            bounded_database_operation(crate::db::compare_recipe_versions(
                &pool,
                recipe_id,
                version_number - 1,
                version_number,
            ))
            .await?
        } else {
            Vec::new()
        };
        let changes_to_current =
            bounded_database_operation(crate::db::compare_recipe_versions(
                &pool,
                recipe_id,
                version_number,
                latest.version_number,
            ))
            .await?;
        let changes_from_original =
            bounded_database_operation(crate::db::compare_recipe_versions(
                &pool,
                recipe_id,
                1,
                version_number,
            ))
            .await?;

        let (comparisons_available, changes_from_previous, changes_to_current, changes_from_original) =
            match bounded_recipe_version_change_sets_to_view([
                changes_from_previous,
                changes_to_current,
                changes_from_original,
            ]) {
                Ok(change_sets) => {
                    let [changes_from_previous, changes_to_current, changes_from_original] =
                        change_sets
                            .try_into()
                            .expect("three comparison inputs produce three outputs");
                    (
                        true,
                        changes_from_previous,
                        changes_to_current,
                        changes_from_original,
                    )
                }
                Err(crate::security::SecurityLimitError::VersionComparisonOutputExceeded) => {
                    tracing::warn!(
                        recipe_id = %recipe_id,
                        version_number,
                        maximum_changes = crate::security::MAX_VERSION_COMPARISON_CHANGES,
                        maximum_output_bytes = crate::security::MAX_VERSION_COMPARISON_OUTPUT_BYTES,
                        "omitted historical version comparisons because their aggregate output limit was exceeded"
                    );
                    (false, Vec::new(), Vec::new(), Vec::new())
                }
                Err(error) => return Err(ServerFnError::new(error.to_string())),
            };

        Ok(crate::types::RecipeVersionView {
            version_number,
            current_version_number: latest.version_number,
            editor: snapshot_to_editor(snapshot, recipe.visibility),
            notes: version.notes,
            created_at: version.created_at,
            comparisons_available,
            changes_from_previous,
            changes_to_current,
            changes_from_original,
        })
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))?
}

#[cfg(feature = "server")]
fn recipe_version_changes_to_view(
    changes: Vec<crate::db::diff::SnapshotChange>,
) -> Vec<crate::types::RecipeVersionChange> {
    changes
        .into_iter()
        .map(|change| crate::types::RecipeVersionChange {
            op: change.op,
            path: change.path,
            old_value: change.old_value.map(snapshot_change_value_to_string),
            value: change.value.map(snapshot_change_value_to_string),
        })
        .collect()
}

#[cfg(feature = "server")]
fn bounded_recipe_version_change_sets_to_view(
    change_sets: impl IntoIterator<Item = Vec<crate::db::diff::SnapshotChange>>,
) -> Result<Vec<Vec<crate::types::RecipeVersionChange>>, crate::security::SecurityLimitError> {
    let mut total_changes = 0usize;
    let mut total_output_bytes = 0usize;
    let mut bounded_change_sets = Vec::new();

    for changes in change_sets {
        total_changes = total_changes
            .checked_add(changes.len())
            .filter(|count| *count <= crate::security::MAX_VERSION_COMPARISON_CHANGES)
            .ok_or(crate::security::SecurityLimitError::VersionComparisonOutputExceeded)?;

        let changes = recipe_version_changes_to_view(changes);
        let encoded_bytes = serde_json::to_vec(&changes)
            .map_err(|_| crate::security::SecurityLimitError::VersionComparisonOutputExceeded)?
            .len();
        total_output_bytes = total_output_bytes
            .checked_add(encoded_bytes)
            .filter(|bytes| *bytes <= crate::security::MAX_VERSION_COMPARISON_OUTPUT_BYTES)
            .ok_or(crate::security::SecurityLimitError::VersionComparisonOutputExceeded)?;
        bounded_change_sets.push(changes);
    }

    Ok(bounded_change_sets)
}

#[cfg(feature = "server")]
fn snapshot_change_value_to_string(value: serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "Not set".to_string(),
        serde_json::Value::String(value) if value.is_empty() => "(empty)".to_string(),
        serde_json::Value::String(value) => value,
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        value => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
    }
}

#[server]
pub async fn compare_recipe_versions_api(
    recipe_id: String,
    from_version_number: i32,
    to_version_number: i32,
) -> Result<crate::types::RecipeVersionComparison, ServerFnError> {
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let pool = crate::db::get_pool();
    crate::security::run_with_graph_history_work_budget(async move {
        bounded_database_operation(crate::db::get_recipe_by_id(&pool, recipe_id))
            .await?
            .filter(|recipe| viewer_may_read_recipe(&recipe.visibility, recipe.user_id, viewer_id))
            .ok_or_else(|| ServerFnError::new("Recipe not found"))?;
        let changes = bounded_database_operation(crate::db::compare_recipe_versions(
            &pool,
            recipe_id,
            from_version_number,
            to_version_number,
        ))
        .await?;
        let change_sets = bounded_recipe_version_change_sets_to_view([changes])
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let [changes] = change_sets
            .try_into()
            .expect("one comparison input produces one output");

        Ok(crate::types::RecipeVersionComparison {
            from_version_number,
            to_version_number,
            changes,
        })
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))?
}

/// Compare two graph snapshots after independently authorizing both recipes.
#[server]
pub async fn compare_version_graph_snapshots_api(
    from: crate::types::VersionGraphNodeId,
    to: crate::types::VersionGraphNodeId,
) -> Result<crate::types::VersionGraphComparison, ServerFnError> {
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let pool = crate::db::get_pool();

    crate::security::run_with_graph_history_work_budget(async move {
        for snapshot in [from, to] {
            bounded_database_operation(crate::db::get_recipe_by_id(&pool, snapshot.recipe_id))
                .await?
                .filter(|recipe| {
                    viewer_may_read_recipe(&recipe.visibility, recipe.user_id, viewer_id)
                })
                .ok_or_else(|| ServerFnError::new("Recipe not found"))?;
        }

        let changes = crate::security::run_with_database_budget(
            crate::security::DATABASE_EXECUTION_BUDGET,
            crate::db::compare_recipe_version_snapshots(&pool, from, to),
        )
        .await
        .map_err(|_| ServerFnError::new("Database operation timed out"))?
        .map_err(|error| match error {
            crate::db::DbError::VersionNotFound { .. } => {
                ServerFnError::new("Recipe snapshot not found")
            }
            other => ServerFnError::new(other.to_string()),
        })?;
        let change_sets = bounded_recipe_version_change_sets_to_view([changes])
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let [changes] = change_sets
            .try_into()
            .expect("one comparison input produces one output");

        Ok(crate::types::VersionGraphComparison { from, to, changes })
    })
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))?
}

#[cfg(all(test, feature = "server"))]
mod version_change_tests {
    use super::{
        bounded_recipe_version_change_sets_to_view, version_graph_image, version_summary_image_url,
    };
    use crate::db::diff::SnapshotChange;
    use crate::security::{
        SecurityLimitError, MAX_VERSION_COMPARISON_CHANGES, MAX_VERSION_COMPARISON_OUTPUT_BYTES,
    };
    use serde_json::json;

    fn scalar_change(
        index: usize,
        old_value: serde_json::Value,
        value: serde_json::Value,
    ) -> SnapshotChange {
        SnapshotChange {
            op: "replace".to_string(),
            path: format!("/ingredients/{index}/name"),
            old_value: Some(old_value),
            value: Some(value),
        }
    }

    #[test]
    fn comparison_values_are_transported_as_display_strings() {
        let mut change_sets = bounded_recipe_version_change_sets_to_view([vec![SnapshotChange {
            op: "replace".to_string(),
            path: "/images/0".to_string(),
            old_value: Some(json!({
                "url": "https://example.test/old.webp",
                "edits": [{"name": "rectangle:16:9", "version": 1}]
            })),
            value: Some(json!({
                "url": "https://example.test/new.webp",
                "edits": [{"name": "rectangle:16:9", "version": 1}]
            })),
        }]])
        .expect("small comparison output is accepted");
        let changes = change_sets.pop().expect("one converted change set");

        let comparison = crate::types::RecipeVersionComparison {
            from_version_number: 1,
            to_version_number: 2,
            changes,
        };
        let encoded = serde_json::to_vec(&comparison).expect("comparison is serializable");
        let decoded: crate::types::RecipeVersionComparison =
            serde_json::from_slice(&encoded).expect("comparison crosses the client boundary");

        let encoded =
            serde_json::to_value(&decoded.changes[0]).expect("comparison change is serializable");
        assert!(encoded["old_value"].is_string());
        assert!(encoded["value"].is_string());
        assert!(encoded["old_value"]
            .as_str()
            .expect("old value string")
            .contains("old.webp"));
    }

    #[test]
    fn comparison_change_count_is_bounded_across_every_returned_change_set() {
        let boundary = (0..MAX_VERSION_COMPARISON_CHANGES)
            .map(|index| scalar_change(index, json!("old"), json!("new")))
            .collect::<Vec<_>>();
        assert!(bounded_recipe_version_change_sets_to_view([boundary.clone()]).is_ok());

        assert_eq!(
            bounded_recipe_version_change_sets_to_view([
                boundary,
                vec![scalar_change(
                    MAX_VERSION_COMPARISON_CHANGES,
                    json!("old"),
                    json!("new"),
                )],
            ]),
            Err(SecurityLimitError::VersionComparisonOutputExceeded)
        );
    }

    #[test]
    fn comparison_encoded_bytes_are_bounded_across_every_returned_change_set() {
        let quarter_limit = "x".repeat(MAX_VERSION_COMPARISON_OUTPUT_BYTES / 4);
        let first = vec![scalar_change(
            0,
            json!(quarter_limit.clone()),
            json!(quarter_limit.clone()),
        )];
        let second = vec![scalar_change(
            1,
            json!(quarter_limit.clone()),
            json!(quarter_limit),
        )];

        assert!(bounded_recipe_version_change_sets_to_view([first.clone()]).is_ok());
        assert!(bounded_recipe_version_change_sets_to_view([second.clone()]).is_ok());
        assert_eq!(
            bounded_recipe_version_change_sets_to_view([first, second]),
            Err(SecurityLimitError::VersionComparisonOutputExceeded)
        );
    }

    #[test]
    fn version_comparison_endpoints_share_the_graph_history_work_budget() {
        let source = include_str!("recipe.rs");
        for (function, next_function) in [
            (
                "pub async fn get_recipe_version_api",
                "fn recipe_version_changes_to_view",
            ),
            (
                "pub async fn compare_recipe_versions_api",
                "pub async fn compare_version_graph_snapshots_api",
            ),
            (
                "pub async fn compare_version_graph_snapshots_api",
                "#[cfg(all(test, feature = \"server\"))]\nmod version_change_tests",
            ),
        ] {
            let start = source.find(function).expect("comparison endpoint exists");
            let end = source[start..]
                .find(next_function)
                .map(|offset| start + offset)
                .expect("comparison endpoint has a bounded source slice");
            let endpoint = &source[start..end];
            assert_eq!(
                endpoint
                    .matches("run_with_graph_history_work_budget")
                    .count(),
                1,
                "{function} must acquire exactly one shared comparison-work permit"
            );
            assert!(
                endpoint.contains("bounded_recipe_version_change_sets_to_view"),
                "{function} must enforce the aggregate comparison-output budget"
            );
        }
    }

    #[test]
    fn version_summary_images_prefer_thumbnails_and_fall_back_to_full_images() {
        let with_thumbnail: crate::types::RecipeImage = serde_json::from_value(json!({
            "url": "https://example.test/full.webp",
            "thumb_url": "https://example.test/thumb.webp",
            "edits": []
        }))
        .unwrap();
        let full_only: crate::types::RecipeImage = serde_json::from_value(json!({
            "url": "https://example.test/only-full.webp",
            "edits": []
        }))
        .unwrap();

        assert_eq!(
            version_summary_image_url(&[with_thumbnail]),
            Some("https://example.test/thumb.webp".to_string())
        );
        assert_eq!(
            version_summary_image_url(&[full_only]),
            Some("https://example.test/only-full.webp".to_string())
        );
        assert_eq!(version_summary_image_url(&[]), None);
    }

    #[test]
    fn version_graph_images_preserve_edits_and_thumbnail_source() {
        let image: crate::types::RecipeImage = serde_json::from_value(json!({
            "url": "https://example.test/full.webp",
            "thumb_url": "https://example.test/thumb.webp",
            "edits": []
        }))
        .unwrap();
        let image_without_thumbnail: crate::types::RecipeImage = serde_json::from_value(json!({
            "url": "https://example.test/full-only.webp",
            "edits": []
        }))
        .unwrap();

        assert_eq!(
            version_graph_image(&[image])
                .as_ref()
                .map(|image| image.display_url(true)),
            Some("https://example.test/thumb.webp")
        );
        assert_eq!(
            version_graph_image(&[image_without_thumbnail])
                .as_ref()
                .map(|image| image.display_url(true)),
            Some("https://example.test/full-only.webp")
        );
        assert_eq!(version_graph_image(&[]), None);
    }
}

#[server]
pub async fn restore_version(
    expected_account_id: Option<uuid::Uuid>,
    recipe_id: String,
    version_number: i32,
    expected_latest_version: i32,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    enforce_publication_budget(user_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    bounded_database_operation(crate::db::restore_recipe_version(
        &crate::db::get_pool(),
        recipe_id,
        user_id,
        version_number,
        expected_latest_version,
    ))
    .await
}

#[server]
pub async fn fork_recipe(
    expected_account_id: Option<uuid::Uuid>,
    source_recipe_id: String,
    message: Option<String>,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    if message
        .as_deref()
        .is_some_and(|value| value.chars().count() > crate::security::MAX_FORK_MESSAGE_CHARS)
    {
        return Err(ServerFnError::new("Fork message is too long"));
    }
    let source_recipe_id = uuid::Uuid::parse_str(&source_recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let draft = bounded_database_operation(crate::db::create_fork_draft(
        &crate::db::get_pool(),
        source_recipe_id,
        user_id,
        message.as_deref(),
    ))
    .await?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn get_fork_attribution(
    recipe_id: String,
) -> Result<Option<crate::types::ForkAttribution>, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let pool = crate::db::get_pool();
    let recipe = bounded_database_operation(crate::db::get_recipe_by_id(&pool, recipe_id)).await?;
    let Some(recipe) = recipe else {
        return Ok(None);
    };
    if recipe.visibility == "private" && viewer_id != Some(recipe.user_id) {
        return Ok(None);
    }
    bounded_database_operation(crate::db::get_fork_attribution_for_viewer(
        &pool, recipe_id, viewer_id,
    ))
    .await
}

/// Load the current recipe's ancestors and a bounded descendant tree.
/// Anonymous viewers are supported; privacy filtering is enforced in the
/// database layer for every returned node.
#[cfg(feature = "server")]
fn normalize_lineage_descendant_depth(descendant_depth: Option<i32>) -> i32 {
    descendant_depth.unwrap_or(2).clamp(1, 8)
}

#[server]
pub async fn get_recipe_lineage(
    recipe_id: String,
    descendant_depth: Option<i32>,
) -> Result<crate::types::RecipeLineageView, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let descendant_depth = normalize_lineage_descendant_depth(descendant_depth);

    bounded_database_operation(crate::db::get_recipe_lineage(
        &crate::db::get_pool(),
        recipe_id,
        viewer_id,
        descendant_depth,
    ))
    .await
}

/// Delete a recipe (ownership-gated).
///
/// Fetches the recipe first to get its images, deletes from DB, then cleans up
/// associated images from storage (best-effort).
#[server]
pub async fn delete_recipe(
    expected_account_id: Option<uuid::Uuid>,
    recipe_id: String,
) -> Result<(), ServerFnError> {
    let user_id = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    enforce_mutation_budget(user_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();
    let storage = crate::storage::get_storage_client();
    delete_recipe_for_user(&pool, &storage, user_id, recipe_id).await
}

#[cfg(feature = "server")]
async fn create_recipe_for_user(
    pool: &sqlx::PgPool,
    storage: &crate::storage::StorageClient,
    user_id: uuid::Uuid,
    input: RecipeCreateInput,
) -> Result<crate::types::Recipe, ServerFnError> {
    let editor = crate::types::RecipeEditorState {
        title: input.title.clone(),
        description: input.description.clone(),
        commentary: input.commentary.clone(),
        prep_time_minutes: input.prep_time_minutes,
        cook_time_minutes: input.cook_time_minutes,
        servings: input.servings,
        ingredients: input.ingredients.clone(),
        instructions: input.instructions.clone(),
        equipment: input.equipment.clone(),
        images: input.images.clone(),
        tags: input.tags.clone(),
        visibility: input.visibility.clone(),
    };
    crate::security::validate_recipe_editor(
        &editor,
        crate::security::RecipeValidationMode::Publish,
    )
    .map_err(|error| ServerFnError::new(error.to_string()))?;
    validate_recipe_media_references(storage, &input.images, &input.instructions)?;
    bounded_database_operation(crate::db::create_published_recipe(
        pool,
        user_id,
        &crate::db::diff::RecipeDraftPayload {
            initial_visibility: Some(input.visibility),
            content: crate::db::diff::RecipeSnapshot {
                title: input.title,
                description: input.description,
                commentary: input.commentary,
                prep_time_minutes: input.prep_time_minutes,
                cook_time_minutes: input.cook_time_minutes,
                servings: input.servings,
                ingredients: input.ingredients,
                instructions: input.instructions,
                equipment: input.equipment,
                images: input.images,
                tags: input.tags,
            },
        },
        None,
    ))
    .await
}

#[cfg(feature = "server")]
async fn update_recipe_for_user(
    pool: &sqlx::PgPool,
    storage: &crate::storage::StorageClient,
    user_id: uuid::Uuid,
    recipe_id: uuid::Uuid,
    input: RecipeUpdateInput,
) -> Result<crate::types::Recipe, ServerFnError> {
    let latest = crate::db::get_latest_recipe_version(pool, recipe_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let current = crate::db::get_recipe_by_id_and_owner(pool, recipe_id, user_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let current_tags = crate::db::get_recipe_tags(pool, recipe_id)
        .await
        .map_err(|error| ServerFnError::new(error.to_string()))?
        .into_iter()
        .map(|tag| tag.tag)
        .collect();
    let mut snapshot = crate::db::diff::recipe_to_snapshot(&current, current_tags);
    if let Some(title) = input.title {
        snapshot.title = title;
    }
    snapshot.description = input.description;
    snapshot.commentary = input.commentary;
    snapshot.prep_time_minutes = input.prep_time_minutes;
    snapshot.cook_time_minutes = input.cook_time_minutes;
    snapshot.servings = input.servings;
    if let Some(ingredients) = input.ingredients {
        snapshot.ingredients = ingredients;
    }
    if let Some(instructions) = input.instructions {
        snapshot.instructions = instructions;
    }
    if let Some(equipment) = input.equipment {
        snapshot.equipment = equipment;
    }
    if let Some(images) = input.images {
        snapshot.images = images;
    }
    if let Some(tags) = input.tags {
        snapshot.tags = tags;
    }
    let editor = snapshot_to_editor(snapshot.clone(), current.visibility.clone());
    crate::security::validate_recipe_editor(
        &editor,
        crate::security::RecipeValidationMode::Publish,
    )
    .map_err(|error| ServerFnError::new(error.to_string()))?;
    validate_recipe_media_references(storage, &snapshot.images, &snapshot.instructions)?;
    let recipe = bounded_database_operation(crate::db::publish_snapshot(
        pool,
        recipe_id,
        user_id,
        latest.version_number,
        &snapshot,
        None,
    ))
    .await?;
    if let Some(visibility) = input.visibility {
        return crate::db::update_recipe_visibility(pool, recipe_id, user_id, &visibility)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()));
    }
    Ok(recipe)
}

#[cfg(feature = "server")]
async fn delete_recipe_for_user(
    pool: &sqlx::PgPool,
    storage: &crate::storage::StorageClient,
    user_id: uuid::Uuid,
    recipe_id: uuid::Uuid,
) -> Result<(), ServerFnError> {
    // Resolve canonical asset identities before the holder rows cascade away.
    let recipe = crate::db::get_recipe_by_id_and_owner(pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let asset_ids = crate::db::recipe_media_asset_ids(&recipe.images, &recipe.instructions);

    bounded_database_operation(crate::db::delete_recipe(pool, recipe_id, user_id)).await?;

    // The asset row, never a submitted URL, authorizes object deletion. A
    // surviving recipe, draft, version, collection, avatar, or fork reference
    // makes the claim retain the asset.
    for asset_id in asset_ids {
        cleanup_unreferenced_media_asset(pool, storage, asset_id).await;
    }

    Ok(())
}

#[cfg(feature = "server")]
async fn cleanup_unreferenced_media_asset(
    pool: &sqlx::PgPool,
    storage: &crate::storage::StorageClient,
    asset_id: uuid::Uuid,
) {
    let decision = match crate::db::claim_media_cleanup(pool, asset_id).await {
        Ok(decision) => decision,
        Err(error) => {
            tracing::warn!(%error, %asset_id, "Skipping media cleanup after claim failure");
            return;
        }
    };
    let crate::db::MediaDiscardDecision::Claimed(claim) = decision else {
        return;
    };
    let mut failures = Vec::new();
    let mut keys = vec![claim.full_key, claim.thumbnail_key];
    keys.extend(claim.original_key);
    for key in keys {
        if let Err(error) = storage.delete_object(&key).await {
            failures.push(format!("{key}: {error}"));
        }
    }
    let failure = (!failures.is_empty()).then(|| failures.join("; "));
    if let Err(error) =
        crate::db::finish_media_asset_delete(pool, claim.id, failure.as_deref()).await
    {
        tracing::warn!(%error, claim_id = %claim.id, "Failed to finish media cleanup");
    }
}

#[cfg(feature = "server")]
fn validate_recipe_media_references(
    storage: &crate::storage::StorageClient,
    images: &[RecipeImage],
    instructions: &[RecipeStep],
) -> Result<(), ServerFnError> {
    for image in images {
        validate_recipe_image(storage, image)?;
    }
    for step in instructions {
        for image in &step.photos {
            validate_recipe_image(storage, image)?;
        }
        validate_recipe_media_references(storage, &[], &step.sub_steps)?;
    }
    Ok(())
}

#[cfg(feature = "server")]
fn validate_recipe_image(
    storage: &crate::storage::StorageClient,
    image: &RecipeImage,
) -> Result<(), ServerFnError> {
    use crate::storage::{MediaObjectKind, MediaUrl};

    match storage
        .classify_media_url(&image.url)
        .map_err(ServerFnError::new)?
    {
        MediaUrl::Internal(full) => {
            if image.asset_id.is_none() || full.kind() != MediaObjectKind::RecipeFull {
                return Err(ServerFnError::new(
                    "Managed recipe media requires an asset identity and full-size recipe object",
                ));
            }
            if let Some(thumb_url) = image.thumb_url.as_deref() {
                let thumb = storage
                    .parse_media_url(thumb_url)
                    .map_err(ServerFnError::new)?;
                if thumb.kind() != MediaObjectKind::RecipeThumbnail
                    || thumb.owner_id() != full.owner_id()
                    || thumb.object_id() != full.object_id()
                {
                    return Err(ServerFnError::new(
                        "Recipe thumbnail must match its owned full-size object",
                    ));
                }
            }
        }
        MediaUrl::ExternalHttps => {
            if image.asset_id.is_some() {
                return Err(ServerFnError::new(
                    "External recipe media cannot claim a managed asset identity",
                ));
            }
            if let Some(thumb_url) = image.thumb_url.as_deref() {
                if storage
                    .classify_media_url(thumb_url)
                    .map_err(ServerFnError::new)?
                    != MediaUrl::ExternalHttps
                {
                    return Err(ServerFnError::new(
                        "External recipe media must use an external HTTPS thumbnail",
                    ));
                }
            }
        }
    }
    Ok(())
}

/// List recipes for the authenticated user with pagination.
#[server]
pub async fn list_my_recipes(
    offset: i64,
    limit: i64,
) -> Result<crate::types::RecipeListResponse, ServerFnError> {
    crate::security::validate_offset_pagination(offset, limit)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    let recipes = bounded_database_operation(crate::db::get_recipes_by_owner_paginated(
        &pool, user_id, limit, offset,
    ))
    .await?;

    let total_count =
        bounded_database_operation(crate::db::count_recipes_by_owner(&pool, user_id)).await?;

    let has_more = (offset + recipes.len() as i64) < total_count;

    Ok(crate::types::RecipeListResponse {
        recipes,
        total_count,
        has_more,
    })
}

/// Get tags for a recipe (ownership-gated).
///
/// Returns just the tag name strings, avoiding the server-only `RecipeTag` type.
#[server]
pub async fn get_recipe_tags(recipe_id: String) -> Result<Vec<String>, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // Verify ownership first
    crate::db::get_recipe_by_id_and_owner(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Fetch tags
    let tags = crate::db::get_recipe_tags(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(tags.into_iter().map(|t| t.tag).collect())
}

// ── Public access server functions (CP8) ──────────────────────────────────

/// Get a public or unlisted recipe by ID. No authentication required.
/// Returns error if recipe is private or doesn't exist.
#[server]
pub async fn get_public_recipe(recipe_id: String) -> Result<crate::types::Recipe, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    crate::db::get_recipe_by_id_public(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Recipe not found"))
}

/// Hold one discovery permit across all queries and the complete response check.
#[cfg(feature = "server")]
async fn with_public_discovery_budget<T>(
    operation: impl std::future::Future<Output = Result<T, ServerFnError>>,
) -> Result<T, ServerFnError> {
    crate::security::enforce_request_budget(None, crate::security::RequestClass::PublicDiscovery)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    crate::security::run_with_public_discovery_work_budget(
        crate::security::run_with_database_budget(
            crate::security::DATABASE_EXECUTION_BUDGET,
            operation,
        ),
    )
    .await
    .map_err(|error| ServerFnError::new(error.to_string()))?
    .map_err(|error| ServerFnError::new(error.to_string()))?
}

/// Get paginated public recipes for the explore page. No authentication required.
#[server]
pub async fn get_public_recipes(
    offset: i64,
    limit: i64,
    query: String,
) -> Result<crate::types::PublicRecipeListResponse, ServerFnError> {
    crate::security::validate_public_recipe_pagination(offset, limit)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    crate::security::validate_public_discovery_query(&query)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    with_public_discovery_budget(async {
        let pool = crate::db::get_pool();

        let recipes =
            crate::db::get_public_recipes_filtered_paginated(&pool, offset, limit, &query)
                .await
                .map_err(|error| ServerFnError::new(error.to_string()))?;

        let total_count = crate::db::count_public_recipes_filtered(&pool, &query)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;

        let has_more = (offset + recipes.len() as i64) < total_count;

        let page = crate::types::PublicRecipeListResponse {
            recipes,
            total_count,
            has_more,
        };
        crate::security::validate_public_recipe_card_page(&page)
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(page)
    })
    .await
}

/// Get user profile info by username. No authentication required.
/// Returns user info and their public recipe count.
#[server]
pub async fn get_user_profile(
    username: String,
) -> Result<crate::types::UserProfile, ServerFnError> {
    let pool = crate::db::get_pool();

    let user = crate::db::get_user_by_username(&pool, &username)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    let public_count = crate::db::count_user_public_recipes(&pool, user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(crate::types::UserProfile {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        bio: user.bio,
        public_recipe_count: public_count,
    })
}

/// Get a user's public recipes by username with pagination. No authentication required.
#[server]
pub async fn get_user_public_recipes(
    username: String,
    offset: i64,
    limit: i64,
) -> Result<crate::types::PublicRecipeListResponse, ServerFnError> {
    crate::security::validate_public_recipe_pagination(offset, limit)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    with_public_discovery_budget(async {
        let pool = crate::db::get_pool();

        let user = crate::db::get_user_by_username(&pool, &username)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .ok_or_else(|| ServerFnError::new("User not found"))?;

        let recipes = crate::db::get_user_public_recipes(&pool, user.id, limit, offset)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;

        let total_count = crate::db::count_user_public_recipes(&pool, user.id)
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?;

        let has_more = (offset + recipes.len() as i64) < total_count;

        let page = crate::types::PublicRecipeListResponse {
            recipes,
            total_count,
            has_more,
        };
        crate::security::validate_public_recipe_card_page(&page)
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        Ok(page)
    })
    .await
}

/// Fetch distinct tags from all public recipes.
/// No authentication required.
#[server]
pub async fn get_public_recipe_tags() -> Result<Vec<String>, ServerFnError> {
    let pool = crate::db::get_pool();
    let tags = crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        crate::db::get_distinct_public_tags(&pool),
    )
    .await
    .map_err(|_| ServerFnError::new("Database operation timed out"))?
    .map_err(|e| {
        tracing::error!(error = ?e, "Failed to fetch distinct public tags");
        ServerFnError::new("Failed to fetch tags")
    })?;
    Ok(tags)
}

/// Get tags for a specific public or unlisted recipe. No authentication required.
/// Returns empty vec if recipe is private or doesn't exist.
#[server]
pub async fn get_public_recipe_tags_by_id(recipe_id: String) -> Result<Vec<String>, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    let tags = crate::db::get_public_recipe_tags(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(tags.into_iter().map(|t| t.tag).collect())
}

/// Get the owner's username for a public or unlisted recipe.
/// No authentication required. Returns `None` if recipe is private or doesn't exist.
#[server]
pub async fn get_recipe_owner_username(recipe_id: String) -> Result<Option<String>, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // First verify the recipe is publicly accessible and get its user_id
    let recipe = crate::db::get_recipe_by_id_public(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let Some(recipe) = recipe else {
        return Ok(None); // Recipe not found or private
    };

    // Look up the owner's username using existing get_user_by_id
    let user = crate::db::get_user_by_id(&pool, recipe.user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(user.map(|u| u.username))
}

#[cfg(all(test, feature = "server"))]
mod lineage_depth_tests {
    use super::normalize_lineage_descendant_depth;

    #[test]
    fn lineage_endpoint_accepts_up_to_eight_descendant_levels() {
        assert_eq!(normalize_lineage_descendant_depth(None), 2);
        assert_eq!(normalize_lineage_descendant_depth(Some(0)), 1);
        assert_eq!(normalize_lineage_descendant_depth(Some(8)), 8);
        assert_eq!(normalize_lineage_descendant_depth(Some(9)), 8);
    }
}

#[cfg(all(test, feature = "server"))]
mod version_graph_tests {
    use super::{build_recipe_version_graph, viewer_may_read_recipe};
    use crate::db::diff::{snapshot_to_json, RecipeSnapshot};
    use crate::db::RecipeVersion;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn snapshot(title: &str, _visibility: &str, image_url: Option<&str>) -> RecipeSnapshot {
        RecipeSnapshot {
            title: title.to_string(),
            description: None,
            commentary: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            servings: None,
            ingredients: Vec::new(),
            instructions: Vec::new(),
            equipment: Vec::new(),
            images: image_url
                .map(|url| {
                    vec![crate::types::RecipeImage {
                        asset_id: None,
                        url: url.to_string(),
                        thumb_url: None,
                        file_name: None,
                        file_size: None,
                        edits: Vec::new(),
                    }]
                })
                .unwrap_or_default(),
            tags: Vec::new(),
        }
    }

    fn version(recipe_id: Uuid, version_number: i32, snapshot: RecipeSnapshot) -> RecipeVersion {
        RecipeVersion {
            id: Uuid::new_v4(),
            recipe_id,
            version_number,
            snapshot: snapshot_to_json(&snapshot).unwrap(),
            notes: None,
            created_at: Utc
                .with_ymd_and_hms(2026, 7, 20, version_number as u32, 0, 0)
                .unwrap(),
            media_asset_urls: serde_json::Value::Null,
        }
    }

    #[test]
    fn selected_recipe_graph_orders_every_snapshot_and_connects_consecutive_versions() {
        let recipe_id = Uuid::new_v4();
        let versions = vec![
            version(recipe_id, 3, snapshot("Soup three", "public", None)),
            version(recipe_id, 1, snapshot("Soup one", "public", None)),
            version(
                recipe_id,
                2,
                snapshot("Soup two", "public", Some("https://example.com/soup.webp")),
            ),
        ];

        let graph = build_recipe_version_graph(recipe_id, "cook", versions).unwrap();

        assert_eq!(
            graph
                .nodes
                .iter()
                .map(|node| node.id().version_number)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].source().version_number, 1);
        assert_eq!(graph.edges[0].target().version_number, 2);
        assert_eq!(graph.edges[1].source().version_number, 2);
        assert_eq!(graph.edges[1].target().version_number, 3);
        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| node.is_current_version())
                .map(|node| node.id().version_number)
                .collect::<Vec<_>>(),
            vec![3]
        );
        assert_eq!(
            graph.nodes[1].thumbnail_url(),
            Some("https://example.com/soup.webp")
        );
    }

    #[test]
    fn accessible_recipes_expose_all_historical_snapshots_to_non_owners() {
        let recipe_id = Uuid::new_v4();
        let versions = vec![
            version(recipe_id, 2, snapshot("Public soup", "public", None)),
            version(
                recipe_id,
                1,
                snapshot(
                    "Private family soup",
                    "private",
                    Some("https://example.com/private.webp"),
                ),
            ),
        ];

        let graph = build_recipe_version_graph(recipe_id, "cook", versions).unwrap();

        assert!(!graph.nodes[0].is_redacted());
        assert_eq!(graph.nodes[0].title(), Some("Private family soup"));
        assert_eq!(graph.nodes[0].owner_username(), Some("cook"));
        assert_eq!(
            graph.nodes[0].thumbnail_url(),
            Some("https://example.com/private.webp")
        );
        assert!(!graph.nodes[1].is_redacted());
        assert_eq!(graph.nodes[1].title(), Some("Public soup"));
        assert_ne!(graph.nodes[1].change_summary(), Some("Published version"));
    }

    #[test]
    fn immutable_snapshots_do_not_persist_recipe_visibility() {
        let serialized =
            snapshot_to_json(&snapshot("Soup", "private", None)).expect("snapshot serializes");

        assert!(serialized.get("visibility").is_none());
    }

    #[test]
    fn current_recipe_visibility_authorizes_the_entire_history() {
        let owner_id = Uuid::new_v4();
        let another_user_id = Uuid::new_v4();

        assert!(viewer_may_read_recipe("public", owner_id, None));
        assert!(viewer_may_read_recipe(
            "unlisted",
            owner_id,
            Some(another_user_id)
        ));
        assert!(!viewer_may_read_recipe("private", owner_id, None));
        assert!(!viewer_may_read_recipe(
            "private",
            owner_id,
            Some(another_user_id)
        ));
        assert!(viewer_may_read_recipe("private", owner_id, Some(owner_id)));
    }
}

#[cfg(all(feature = "server", test))]
mod media_ownership_tests {
    use super::*;

    fn image(url: String, thumb_url: Option<String>) -> RecipeImage {
        RecipeImage {
            asset_id: None,
            url,
            thumb_url,
            file_name: None,
            file_size: None,
            edits: Vec::new(),
        }
    }

    fn test_storage(endpoint: &str) -> crate::storage::StorageClient {
        crate::storage::StorageClient::for_test_with_endpoint_and_public_url(
            endpoint,
            "https://cdn.example.com/media",
        )
    }

    async fn create_test_user(pool: &sqlx::PgPool, label: &str) -> crate::db::User {
        let unique = crate::test_utils::uid();
        crate::db::insert_user(
            pool,
            &format!("{label}_{unique}"),
            label,
            &format!("{label}_{unique}@example.com"),
            None,
        )
        .await
        .expect("test user should be created")
    }

    fn create_input(images: Vec<RecipeImage>, instructions: Vec<RecipeStep>) -> RecipeCreateInput {
        RecipeCreateInput {
            title: "Ownership test".to_string(),
            description: None,
            commentary: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            servings: None,
            ingredients: Vec::new(),
            instructions,
            equipment: Vec::new(),
            images,
            tags: Vec::new(),
            visibility: "private".to_string(),
        }
    }

    fn update_input(images: Vec<RecipeImage>, instructions: Vec<RecipeStep>) -> RecipeUpdateInput {
        RecipeUpdateInput {
            title: Some("Changed title".to_string()),
            description: None,
            commentary: None,
            prep_time_minutes: None,
            cook_time_minutes: None,
            servings: None,
            ingredients: Some(Vec::new()),
            instructions: Some(instructions),
            equipment: Some(Vec::new()),
            images: Some(images),
            tags: None,
            visibility: Some("private".to_string()),
        }
    }

    #[test]
    fn recipe_media_rejects_foreign_recipe_and_avatar_objects_at_any_depth() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let victim_recipe_object = uuid::Uuid::new_v4();
        let victim_avatar_object = uuid::Uuid::new_v4();
        let storage = test_storage("http://127.0.0.1:9");

        let victim_recipe = image(
            storage.object_url(&format!(
                "recipes/{victim_id}/{victim_recipe_object}_full.webp"
            )),
            Some(storage.object_url(&format!(
                "recipes/{victim_id}/{victim_recipe_object}_thumb.webp"
            ))),
        );
        let victim_avatar = image(
            storage.object_url(&format!("avatars/{victim_id}/{victim_avatar_object}.webp")),
            None,
        );
        let owner_recipe_object = uuid::Uuid::new_v4();
        let mismatched_thumbnail_object = uuid::Uuid::new_v4();
        let mismatched_pair = image(
            storage.object_url(&format!(
                "recipes/{owner_id}/{owner_recipe_object}_full.webp"
            )),
            Some(storage.object_url(&format!(
                "recipes/{owner_id}/{mismatched_thumbnail_object}_thumb.webp"
            ))),
        );
        let nested_steps = vec![RecipeStep {
            text: "parent".to_string(),
            photos: Vec::new(),
            sub_steps: vec![RecipeStep {
                text: "child".to_string(),
                photos: vec![victim_avatar],
                sub_steps: Vec::new(),
            }],
        }];

        assert!(validate_recipe_media_references(&storage, &[victim_recipe], &[]).is_err());
        assert!(validate_recipe_media_references(&storage, &[mismatched_pair], &[]).is_err());
        assert!(validate_recipe_media_references(&storage, &[], &nested_steps).is_err());
    }

    #[test]
    fn recipe_media_accepts_owned_pairs_and_external_https_urls() {
        let owner_id = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();
        let storage = test_storage("http://127.0.0.1:9");
        let mut owned = image(
            storage.object_url(&format!("recipes/{owner_id}/{object_id}_full.webp")),
            Some(storage.object_url(&format!("recipes/{owner_id}/{object_id}_thumb.webp"))),
        );
        owned.asset_id = Some(object_id);
        let external = image(
            "https://images.example.net/recipe.webp".to_string(),
            Some("https://images.example.net/recipe-thumb.webp".to_string()),
        );

        validate_recipe_media_references(&storage, &[owned, external], &[])
            .expect("owned and external HTTPS media should remain supported");
    }

    #[tokio::test]
    async fn create_rejects_foreign_media_without_database_change() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let owner = create_test_user(&pool, "media_owner").await;
        let victim = create_test_user(&pool, "media_victim").await;
        let object_id = uuid::Uuid::new_v4();
        let storage = test_storage("http://127.0.0.1:9");
        let foreign = image(
            storage.object_url(&format!("recipes/{}/{object_id}_full.webp", victim.id)),
            None,
        );

        let result = create_recipe_for_user(
            &pool,
            &storage,
            owner.id,
            create_input(vec![foreign], Vec::new()),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            crate::db::count_recipes_by_owner(&pool, owner.id)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn update_rejects_nested_foreign_media_without_database_change() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let owner = create_test_user(&pool, "update_owner").await;
        let victim = create_test_user(&pool, "update_victim").await;
        let storage = test_storage("http://127.0.0.1:9");
        let original = crate::db::insert_recipe(
            &pool,
            owner.id,
            "Original title",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        let victim_object = uuid::Uuid::new_v4();
        let nested_steps = vec![RecipeStep {
            text: "parent".to_string(),
            photos: Vec::new(),
            sub_steps: vec![RecipeStep {
                text: "child".to_string(),
                photos: vec![image(
                    storage.object_url(&format!("recipes/{}/{victim_object}_full.webp", victim.id)),
                    None,
                )],
                sub_steps: Vec::new(),
            }],
        }];

        let result = update_recipe_for_user(
            &pool,
            &storage,
            owner.id,
            original.id,
            update_input(Vec::new(), nested_steps),
        )
        .await;

        assert!(result.is_err());
        let unchanged = crate::db::get_recipe_by_id_and_owner(&pool, original.id, owner.id)
            .await
            .unwrap();
        assert_eq!(unchanged.title, "Original title");
        assert!(unchanged.instructions.is_empty());
    }

    #[tokio::test]
    async fn deleting_legacy_recipe_skips_foreign_objects_but_removes_database_row() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let owner = create_test_user(&pool, "delete_owner").await;
        let victim = create_test_user(&pool, "delete_victim").await;
        let victim_object = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        let storage = test_storage(&mock_server.uri());
        let foreign = image(
            storage.object_url(&format!("recipes/{}/{victim_object}_full.webp", victim.id)),
            Some(storage.object_url(&format!("recipes/{}/{victim_object}_thumb.webp", victim.id))),
        );
        let recipe = crate::db::insert_recipe(
            &pool,
            owner.id,
            "Legacy foreign reference",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[foreign],
            "private",
        )
        .await
        .unwrap();

        delete_recipe_for_user(&pool, &storage, owner.id, recipe.id)
            .await
            .expect("recipe deletion should proceed");

        assert!(
            crate::db::get_recipe_by_id_and_owner(&pool, recipe.id, owner.id)
                .await
                .is_err()
        );
        assert!(mock_server.received_requests().await.unwrap().is_empty());
    }
}

/// Download the current saved content visible to this viewer, never an editor draft.
#[server]
pub async fn export_recipe_json(recipe_id: String) -> Result<String, ServerFnError> {
    let viewer = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer)?;
    let id =
        uuid::Uuid::parse_str(&recipe_id).map_err(|_| ServerFnError::new("Invalid recipe ID"))?;
    crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        export_recipe_json_for_viewer(&crate::db::get_pool(), id, viewer),
    )
    .await
    .map_err(|_| ServerFnError::new("Recipe export timed out"))?
}

#[cfg(feature = "server")]
async fn export_recipe_json_for_viewer(
    pool: &sqlx::PgPool,
    id: uuid::Uuid,
    viewer: Option<uuid::Uuid>,
) -> Result<String, ServerFnError> {
    read_recipe_export(pool, id, viewer)
        .await
        .map(|(_, json)| json)
}

#[cfg(feature = "server")]
pub(super) async fn read_recipe_export(
    pool: &sqlx::PgPool,
    id: uuid::Uuid,
    viewer: Option<uuid::Uuid>,
) -> Result<(crate::types::Recipe, String), ServerFnError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ServerFnError::new("Cannot read recipe"))?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| ServerFnError::new("Cannot read recipe"))?;
    let recipe = match crate::db::get_recipe_by_id_public(&mut *tx, id)
        .await
        .map_err(|_| ServerFnError::new("Cannot read recipe"))?
    {
        Some(recipe) => recipe,
        None => match viewer {
            Some(owner) => crate::db::get_recipe_by_id_and_owner(&mut *tx, id, owner)
                .await
                .map_err(|_| ServerFnError::new("Recipe not found"))?,
            None => return Err(ServerFnError::new("Recipe not found")),
        },
    };
    let tags = crate::db::get_recipe_tags(&mut *tx, id)
        .await
        .map_err(|_| ServerFnError::new("Cannot read recipe tags"))?
        .into_iter()
        .map(|tag| tag.tag)
        .collect();
    let output = crate::recipe_json::export_recipe(&recipe, tags).map_err(ServerFnError::new)?;
    tx.commit()
        .await
        .map_err(|_| ServerFnError::new("Cannot finish recipe export"))?;
    Ok((recipe, output))
}

/// Validate the exact versioned JSON contract before creating one private copy.
#[server]
pub async fn import_recipe_json(
    expected_account_id: Option<uuid::Uuid>,
    json: String,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user = crate::auth::account_binding::require_form_account(expected_account_id).await?;
    enforce_mutation_budget(user)?;
    let (_, editor) = crate::recipe_json::parse_recipe(&json).map_err(ServerFnError::new)?;
    let draft = bounded_database_operation(crate::db::create_new_recipe_draft(
        &crate::db::get_pool(),
        user,
        &editor_to_draft_payload(editor),
    ))
    .await?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

#[cfg(all(test, feature = "server"))]
mod recipe_json_tests {
    use super::*;

    #[tokio::test]
    async fn json_export_enforces_visibility_and_exports_saved_content_not_owner_drafts() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let unique = crate::test_utils::uid();
        let owner = crate::db::insert_user(
            &pool,
            &format!("json_{unique}"),
            "Cook",
            &format!("{unique}@example.test"),
            None,
        )
        .await
        .unwrap();
        let recipe =
            crate::test_utils::create_test_published_recipe(&pool, owner.id, "Saved recipe").await;
        assert!(export_recipe_json_for_viewer(&pool, recipe.id, None)
            .await
            .is_err());
        assert!(
            export_recipe_json_for_viewer(&pool, recipe.id, Some(uuid::Uuid::new_v4()))
                .await
                .is_err()
        );
        let draft = crate::test_utils::create_test_edit_draft(&pool, owner.id, recipe.id).await;
        let mut edited = draft.decoded_snapshot().unwrap();
        edited.content.title = "Unpublished secret".into();
        crate::db::update_recipe_draft(&pool, draft.id, owner.id, draft.revision, &edited)
            .await
            .unwrap();
        let json = export_recipe_json_for_viewer(&pool, recipe.id, Some(owner.id))
            .await
            .unwrap();
        let (_, content) = crate::recipe_json::parse_recipe(&json).unwrap();
        assert_eq!(content.title, "Saved recipe");
        assert!(!json.contains(&draft.id.to_string()));
        for visibility in ["public", "unlisted"] {
            crate::db::update_recipe_visibility(&pool, recipe.id, owner.id, visibility)
                .await
                .unwrap();
            let json = export_recipe_json_for_viewer(&pool, recipe.id, None)
                .await
                .unwrap();
            let (file, content) = crate::recipe_json::parse_recipe(&json).unwrap();
            assert_eq!(content.visibility, "private");
            assert_eq!(file.source_attribution.author_username, owner.username);
        }
        assert!(
            export_recipe_json_for_viewer(&pool, uuid::Uuid::new_v4(), Some(owner.id))
                .await
                .is_err()
        );
    }
}
