//! Recipe CRUD server functions.
//!
//! All functions require authentication via session cookie.
//! Functions accessing individual recipes verify ownership.

#![allow(clippy::too_many_arguments)]

use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::types::{RecipeImage, RecipeStep};
#[cfg(feature = "server")]
use std::collections::HashSet;

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
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    recipe_id: String,
    visibility: String,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    editor: crate::types::RecipeEditorState,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    crate::security::validate_recipe_editor(&editor, crate::security::RecipeValidationMode::Draft)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let storage = crate::storage::get_storage_client();
    validate_recipe_media_references(&storage, user_id, &editor.images, &editor.instructions)?;
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
    recipe_id: String,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    draft_id: String,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    draft_id: String,
    expected_revision: i64,
    editor: crate::types::RecipeEditorState,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::security::enforce_user_request_budget(user_id, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Mutation rate limit exceeded"))?;
    crate::security::validate_recipe_editor(&editor, crate::security::RecipeValidationMode::Draft)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    let current_draft = crate::db::get_recipe_draft(&crate::db::get_pool(), draft_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let current_payload = current_draft
        .decoded_snapshot()
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let allowed_urls = collect_recipe_media_urls(
        &current_payload.content.images,
        &current_payload.content.instructions,
    );
    let storage = crate::storage::get_storage_client();
    validate_recipe_media_references_with_allowed(
        &storage,
        user_id,
        &editor.images,
        &editor.instructions,
        &allowed_urls,
    )?;
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
pub async fn discard_recipe_draft(draft_id: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    draft_id: String,
    notes: Option<String>,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    let allowed_urls =
        collect_recipe_media_urls(&payload.content.images, &payload.content.instructions);
    let storage = crate::storage::get_storage_client();
    validate_recipe_media_references_with_allowed(
        &storage,
        user_id,
        &payload.content.images,
        &payload.content.instructions,
        &allowed_urls,
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
) -> Result<Vec<crate::types::RecipeVersionSummary>, ServerFnError> {
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    enforce_graph_budget(viewer_id)?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let pool = crate::db::get_pool();
    bounded_database_operation(crate::db::get_recipe_by_id(&pool, recipe_id))
        .await?
        .filter(|recipe| viewer_may_read_recipe(&recipe.visibility, recipe.user_id, viewer_id))
        .ok_or_else(|| ServerFnError::new("Recipe not found"))?;
    let versions =
        bounded_database_operation(crate::db::get_recipe_versions(&pool, recipe_id)).await?;
    let snapshots = versions
        .iter()
        .map(crate::db::RecipeVersion::decoded_snapshot)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    versions
        .iter()
        .enumerate()
        .map(|(index, version)| {
            let change_summary = snapshots
                .get(index + 1)
                .map(|older| crate::db::diff::compute_diff(older, &snapshots[index]))
                .transpose()
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .map(|diff| crate::db::diff::change_summary(&diff))
                .unwrap_or_else(|| "Initial version".to_string());
            Ok(crate::types::RecipeVersionSummary {
                version_number: version.version_number,
                title: snapshots[index].title.clone(),
                image_url: version_summary_image_url(&snapshots[index].images),
                notes: version.notes.clone(),
                created_at: version.created_at,
                is_current: index == 0,
                change_summary,
            })
        })
        .collect()
}

#[cfg(feature = "server")]
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
    crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        crate::db::get_initial_recipe_version_graph(&pool, recipe_id, viewer_id),
    )
    .await
    .map_err(|_| ServerFnError::new("Database operation timed out"))?
    .map_err(|error| match error {
        crate::db::DbError::RecipeNotFound(_) => ServerFnError::new("Recipe not found"),
        other => ServerFnError::new(other.to_string()),
    })
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
    crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        crate::db::expand_recipe_version_graph(
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
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let latest =
        bounded_database_operation(crate::db::get_latest_recipe_version(&pool, recipe_id)).await?;
    let changes_from_previous = if version_number > 1 {
        recipe_version_changes_to_view(
            bounded_database_operation(crate::db::compare_recipe_versions(
                &pool,
                recipe_id,
                version_number - 1,
                version_number,
            ))
            .await?,
        )
    } else {
        Vec::new()
    };
    let (changes_to_current, changes_from_original) = (
        recipe_version_changes_to_view(
            bounded_database_operation(crate::db::compare_recipe_versions(
                &pool,
                recipe_id,
                version_number,
                latest.version_number,
            ))
            .await?,
        ),
        recipe_version_changes_to_view(
            bounded_database_operation(crate::db::compare_recipe_versions(
                &pool,
                recipe_id,
                1,
                version_number,
            ))
            .await?,
        ),
    );
    Ok(crate::types::RecipeVersionView {
        version_number,
        current_version_number: latest.version_number,
        editor: snapshot_to_editor(snapshot, recipe.visibility),
        notes: version.notes,
        created_at: version.created_at,
        comparisons_available: true,
        changes_from_previous,
        changes_to_current,
        changes_from_original,
    })
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

#[cfg(all(test, feature = "server"))]
mod version_change_tests {
    use super::{recipe_version_changes_to_view, version_graph_image, version_summary_image_url};
    use crate::db::diff::SnapshotChange;
    use serde_json::json;

    #[test]
    fn comparison_values_are_transported_as_display_strings() {
        let changes = recipe_version_changes_to_view(vec![SnapshotChange {
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
        }]);

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

    Ok(crate::types::RecipeVersionComparison {
        from_version_number,
        to_version_number,
        changes: recipe_version_changes_to_view(changes),
    })
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

    for snapshot in [from, to] {
        bounded_database_operation(crate::db::get_recipe_by_id(&pool, snapshot.recipe_id))
            .await?
            .filter(|recipe| viewer_may_read_recipe(&recipe.visibility, recipe.user_id, viewer_id))
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

    Ok(crate::types::VersionGraphComparison {
        from,
        to,
        changes: recipe_version_changes_to_view(changes),
    })
}

#[server]
pub async fn restore_version(
    recipe_id: String,
    version_number: i32,
    expected_latest_version: i32,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    source_recipe_id: String,
    message: Option<String>,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    Ok(
        bounded_database_operation(crate::db::get_fork_relationship(&pool, recipe_id))
            .await?
            .map(|relationship| crate::types::ForkAttribution {
                original_recipe_id: relationship.original_recipe_id,
                original_title: relationship.original_title,
                message: relationship.message,
            }),
    )
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
pub async fn delete_recipe(recipe_id: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
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
    validate_recipe_media_references(storage, user_id, &input.images, &input.instructions)?;
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
    let allowed_urls = collect_recipe_media_urls(&current.images, &current.instructions);
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
    validate_recipe_media_references_with_allowed(
        storage,
        user_id,
        &snapshot.images,
        &snapshot.instructions,
        &allowed_urls,
    )?;
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
    // Fetch recipe first to get images (ownership-gated)
    let recipe = crate::db::get_recipe_by_id_and_owner(pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Delete from DB (enforces ownership via WHERE user_id = $2)
    bounded_database_operation(crate::db::delete_recipe(pool, recipe_id, user_id)).await?;

    // Clean up only objects which are no longer referenced by another recipe,
    // draft, or immutable version. Forks and historical snapshots deliberately
    // share object URLs with the source recipe.
    let step_photos = collect_step_photos(&recipe.instructions);
    if !recipe.images.is_empty() || !step_photos.is_empty() {
        let retained_urls: HashSet<String> = match crate::db::get_all_image_urls(pool).await {
            Ok(urls) => urls.into_iter().collect(),
            Err(error) => {
                tracing::warn!(%error, "skipping recipe image cleanup because references could not be scanned");
                return Ok(());
            }
        };
        for image in &recipe.images {
            delete_recipe_image_objects(storage, user_id, image, &retained_urls).await;
        }

        for image in step_photos {
            delete_recipe_image_objects(storage, user_id, &image, &retained_urls).await;
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
async fn delete_recipe_image_objects(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    image: &RecipeImage,
    retained_urls: &HashSet<String>,
) {
    if retained_urls.contains(&image.url) {
        return;
    }
    let object = match storage.parse_media_url(&image.url) {
        Ok(object) => object,
        Err(error) => {
            tracing::warn!(
                owner_id = %owner_id,
                error = %error,
                "Skipping unverified recipe media during deletion"
            );
            return;
        }
    };
    if object.owner_id() != owner_id || object.kind() != crate::storage::MediaObjectKind::RecipeFull
    {
        tracing::warn!(
            owner_id = %owner_id,
            object_owner_id = %object.owner_id(),
            object_key = %object.key(),
            "Skipping foreign or non-recipe media during recipe deletion"
        );
        return;
    }
    let _ = storage.delete_object(object.key()).await;
    let thumbnail_is_retained = image
        .thumb_url
        .as_ref()
        .is_some_and(|url| retained_urls.contains(url));
    if !thumbnail_is_retained {
        if let Some(thumb_key) = object.paired_thumbnail_key() {
            let _ = storage.delete_object(&thumb_key).await;
        }
    }
}

#[cfg(feature = "server")]
fn validate_recipe_media_references(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    images: &[RecipeImage],
    instructions: &[RecipeStep],
) -> Result<(), ServerFnError> {
    validate_recipe_media_references_with_allowed(
        storage,
        owner_id,
        images,
        instructions,
        &HashSet::new(),
    )
}

#[cfg(feature = "server")]
fn validate_recipe_media_references_with_allowed(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    images: &[RecipeImage],
    instructions: &[RecipeStep],
    allowed_internal_urls: &HashSet<String>,
) -> Result<(), ServerFnError> {
    for image in images {
        validate_recipe_image(storage, owner_id, image, allowed_internal_urls)?;
    }
    for step in instructions {
        for image in &step.photos {
            validate_recipe_image(storage, owner_id, image, allowed_internal_urls)?;
        }
        validate_recipe_media_references_with_allowed(
            storage,
            owner_id,
            &[],
            &step.sub_steps,
            allowed_internal_urls,
        )?;
    }
    Ok(())
}

#[cfg(feature = "server")]
fn validate_recipe_image(
    storage: &crate::storage::StorageClient,
    owner_id: uuid::Uuid,
    image: &RecipeImage,
    allowed_internal_urls: &HashSet<String>,
) -> Result<(), ServerFnError> {
    use crate::storage::{MediaObjectKind, MediaUrl};

    match storage
        .classify_media_url(&image.url)
        .map_err(ServerFnError::new)?
    {
        MediaUrl::Internal(full) => {
            if full.kind() != MediaObjectKind::RecipeFull
                || (full.owner_id() != owner_id && !allowed_internal_urls.contains(&image.url))
            {
                return Err(ServerFnError::new(
                    "Recipe media must be an owned full-size recipe object",
                ));
            }
            if let Some(thumb_url) = image.thumb_url.as_deref() {
                let thumb = storage
                    .parse_media_url(thumb_url)
                    .map_err(ServerFnError::new)?;
                if thumb.owner_id() != owner_id && !allowed_internal_urls.contains(thumb_url)
                    || thumb.kind() != MediaObjectKind::RecipeThumbnail
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

#[cfg(feature = "server")]
fn collect_recipe_media_urls(
    images: &[RecipeImage],
    instructions: &[RecipeStep],
) -> HashSet<String> {
    let mut urls = HashSet::new();
    for image in images {
        urls.insert(image.url.clone());
        if let Some(thumb_url) = &image.thumb_url {
            urls.insert(thumb_url.clone());
        }
    }
    for step in instructions {
        urls.extend(collect_recipe_media_urls(&step.photos, &step.sub_steps));
    }
    urls
}

#[cfg(feature = "server")]
fn collect_step_photos(steps: &[RecipeStep]) -> Vec<RecipeImage> {
    let mut photos = Vec::new();
    for step in steps {
        photos.extend(step.display_photos());
        photos.extend(collect_step_photos(&step.sub_steps));
    }
    photos
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

/// Get paginated public recipes for the explore page. No authentication required.
#[server]
pub async fn get_public_recipes(
    offset: i64,
    limit: i64,
) -> Result<crate::types::RecipeListResponse, ServerFnError> {
    crate::security::validate_offset_pagination(offset, limit)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let pool = crate::db::get_pool();

    let recipes = bounded_database_operation(crate::db::get_public_recipes_paginated(
        &pool, limit, offset,
    ))
    .await?;

    let total_count = bounded_database_operation(crate::db::count_public_recipes(&pool)).await?;

    let has_more = (offset + recipes.len() as i64) < total_count;

    Ok(crate::types::RecipeListResponse {
        recipes,
        total_count,
        has_more,
    })
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
) -> Result<crate::types::RecipeListResponse, ServerFnError> {
    crate::security::validate_offset_pagination(offset, limit)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let pool = crate::db::get_pool();

    let user = bounded_database_operation(crate::db::get_user_by_username(&pool, &username))
        .await?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    let recipes = bounded_database_operation(crate::db::get_user_public_recipes(
        &pool, user.id, limit, offset,
    ))
    .await?;

    let total_count =
        bounded_database_operation(crate::db::count_user_public_recipes(&pool, user.id)).await?;

    let has_more = (offset + recipes.len() as i64) < total_count;

    Ok(crate::types::RecipeListResponse {
        recipes,
        total_count,
        has_more,
    })
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

        assert!(
            validate_recipe_media_references(&storage, owner_id, &[victim_recipe], &[]).is_err()
        );
        assert!(
            validate_recipe_media_references(&storage, owner_id, &[mismatched_pair], &[]).is_err()
        );
        assert!(validate_recipe_media_references(&storage, owner_id, &[], &nested_steps).is_err());
    }

    #[test]
    fn recipe_media_accepts_owned_pairs_and_external_https_urls() {
        let owner_id = uuid::Uuid::new_v4();
        let object_id = uuid::Uuid::new_v4();
        let storage = test_storage("http://127.0.0.1:9");
        let owned = image(
            storage.object_url(&format!("recipes/{owner_id}/{object_id}_full.webp")),
            Some(storage.object_url(&format!("recipes/{owner_id}/{object_id}_thumb.webp"))),
        );
        let external = image(
            "https://images.example.net/recipe.webp".to_string(),
            Some("https://images.example.net/recipe-thumb.webp".to_string()),
        );

        validate_recipe_media_references(&storage, owner_id, &[owned, external], &[])
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
    async fn deletion_sends_only_exact_owned_recipe_keys_to_storage() {
        let owner_id = uuid::Uuid::new_v4();
        let victim_id = uuid::Uuid::new_v4();
        let owned_object = uuid::Uuid::new_v4();
        let victim_object = uuid::Uuid::new_v4();
        let mock_server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .expect(2)
            .mount(&mock_server)
            .await;
        let storage = test_storage(&mock_server.uri());

        let owned = image(
            storage.object_url(&format!("recipes/{owner_id}/{owned_object}_full.webp")),
            Some(storage.object_url(&format!("recipes/{owner_id}/{owned_object}_thumb.webp"))),
        );
        let victim = image(
            storage.object_url(&format!("recipes/{victim_id}/{victim_object}_full.webp")),
            Some(storage.object_url(&format!("recipes/{victim_id}/{victim_object}_thumb.webp"))),
        );
        let external = image("https://images.example.net/recipe.webp".to_string(), None);

        let retained_urls = HashSet::new();
        delete_recipe_image_objects(&storage, owner_id, &victim, &retained_urls).await;
        delete_recipe_image_objects(&storage, owner_id, &external, &retained_urls).await;
        delete_recipe_image_objects(&storage, owner_id, &owned, &retained_urls).await;

        let requests = mock_server.received_requests().await.unwrap();
        let deleted_paths: Vec<_> = requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect();
        assert_eq!(requests.len(), 2);
        assert!(deleted_paths
            .iter()
            .any(|path| path.ends_with(&format!("/{owned_object}_full.webp"))));
        assert!(deleted_paths
            .iter()
            .any(|path| path.ends_with(&format!("/{owned_object}_thumb.webp"))));
        assert!(deleted_paths
            .iter()
            .all(|path| !path.contains(&victim_id.to_string())));
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
