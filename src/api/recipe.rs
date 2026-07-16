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
fn editor_to_snapshot(editor: crate::types::RecipeEditorState) -> crate::db::diff::RecipeSnapshot {
    crate::db::diff::RecipeSnapshot {
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
        visibility: editor.visibility,
        tags: editor.tags,
    }
}

#[cfg(feature = "server")]
fn snapshot_to_editor(
    snapshot: crate::db::diff::RecipeSnapshot,
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
        visibility: snapshot.visibility,
        tags: snapshot.tags,
    }
}

#[cfg(feature = "server")]
fn draft_to_view(
    draft: crate::db::RecipeDraft,
) -> Result<crate::types::RecipeDraftView, crate::db::DbError> {
    let editor = snapshot_to_editor(draft.decoded_snapshot()?);
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
    let pool = crate::db::get_pool();

    let snapshot = crate::db::diff::RecipeSnapshot {
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
        visibility,
        tags,
    };
    crate::db::create_published_recipe(&pool, user_id, &snapshot, None)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
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
    visibility: Option<String>,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // Capture the base before reading the denormalized current state. If a
    // concurrent publish happens during the reads below, `publish_snapshot`
    // rejects this update instead of labeling stale content with the newer base.
    let latest = crate::db::get_latest_recipe_version(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let current = crate::db::get_recipe_by_id_and_owner(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let current_tags = crate::db::get_recipe_tags(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(|tag| tag.tag)
        .collect();
    let mut snapshot = crate::db::diff::recipe_to_snapshot(&current, current_tags);
    if let Some(title) = title {
        snapshot.title = title;
    }
    snapshot.description = description;
    snapshot.commentary = commentary;
    snapshot.prep_time_minutes = prep_time_minutes;
    snapshot.cook_time_minutes = cook_time_minutes;
    snapshot.servings = servings;
    if let Some(ingredients) = ingredients {
        snapshot.ingredients = ingredients;
    }
    if let Some(instructions) = instructions {
        snapshot.instructions = instructions;
    }
    if let Some(equipment) = equipment {
        snapshot.equipment = equipment;
    }
    if let Some(images) = images {
        snapshot.images = images;
    }
    if let Some(tags) = tags {
        snapshot.tags = tags;
    }
    if let Some(visibility) = visibility {
        snapshot.visibility = visibility;
    }
    crate::db::publish_snapshot(
        &pool,
        recipe_id,
        user_id,
        latest.version_number,
        &snapshot,
        None,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Start an incomplete, owner-only draft for a new recipe.
#[server]
pub async fn create_recipe_draft(
    editor: crate::types::RecipeEditorState,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let draft = crate::db::create_new_recipe_draft(
        &crate::db::get_pool(),
        user_id,
        &editor_to_snapshot(editor),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
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
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let draft = crate::db::create_edit_draft(&crate::db::get_pool(), recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
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
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    let draft = crate::db::update_recipe_draft(
        &crate::db::get_pool(),
        draft_id,
        user_id,
        expected_revision,
        &editor_to_snapshot(editor),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn discard_recipe_draft(draft_id: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    crate::db::delete_recipe_draft(&crate::db::get_pool(), draft_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn publish_recipe_draft(
    draft_id: String,
    notes: Option<String>,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let draft_id = uuid::Uuid::parse_str(&draft_id)
        .map_err(|error| ServerFnError::new(format!("Invalid draft ID: {error}")))?;
    crate::db::publish_recipe_draft(&crate::db::get_pool(), draft_id, user_id, notes.as_deref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn list_recipe_drafts() -> Result<Vec<crate::types::RecipeDraftView>, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    crate::db::list_recipe_drafts(&crate::db::get_pool(), user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .into_iter()
        .map(|draft| draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string())))
        .collect()
}

#[server]
pub async fn get_recipe_versions_api(
    recipe_id: String,
) -> Result<Vec<crate::types::RecipeVersionSummary>, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let pool = crate::db::get_pool();
    crate::db::get_recipe_by_id_and_owner(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let versions = crate::db::get_recipe_versions(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
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
                notes: version.notes.clone(),
                created_at: version.created_at,
                is_current: index == 0,
                change_summary,
            })
        })
        .collect()
}

#[server]
pub async fn get_recipe_version_api(
    recipe_id: String,
    version_number: i32,
) -> Result<crate::types::RecipeVersionView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let pool = crate::db::get_pool();
    crate::db::get_recipe_by_id_and_owner(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let version = crate::db::get_recipe_version(&pool, recipe_id, version_number)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let snapshot = version
        .decoded_snapshot()
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let changes_from_previous = if version_number > 1 {
        let previous = crate::db::get_recipe_version(&pool, recipe_id, version_number - 1)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .decoded_snapshot()
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        crate::db::diff::compute_diff(&previous, &snapshot)
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .into_iter()
            .map(|change| crate::types::RecipeVersionChange {
                op: change.op,
                path: change.path,
                old_value: change.old_value,
                value: change.value,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(crate::types::RecipeVersionView {
        version_number,
        editor: snapshot_to_editor(snapshot),
        notes: version.notes,
        created_at: version.created_at,
        changes_from_previous,
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
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    crate::db::restore_recipe_version(
        &crate::db::get_pool(),
        recipe_id,
        user_id,
        version_number,
        expected_latest_version,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn fork_recipe(
    source_recipe_id: String,
    message: Option<String>,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let source_recipe_id = uuid::Uuid::parse_str(&source_recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let draft = crate::db::create_fork_draft(
        &crate::db::get_pool(),
        source_recipe_id,
        user_id,
        message.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    draft_to_view(draft).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn get_fork_attribution(
    recipe_id: String,
) -> Result<Option<crate::types::ForkAttribution>, ServerFnError> {
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|error| ServerFnError::new(format!("Invalid recipe ID: {error}")))?;
    let pool = crate::db::get_pool();
    let recipe = crate::db::get_recipe_by_id(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let Some(recipe) = recipe else {
        return Ok(None);
    };
    let viewer_id = crate::auth::session::extract_user_id_from_fullstack().await;
    if recipe.visibility == "private" && viewer_id != Some(recipe.user_id) {
        return Ok(None);
    }
    Ok(crate::db::get_fork_relationship(&pool, recipe_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map(|relationship| crate::types::ForkAttribution {
            original_recipe_id: relationship.original_recipe_id,
            original_title: relationship.original_title,
            message: relationship.message,
        }))
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
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // Fetch recipe first to get images (ownership-gated)
    let recipe = crate::db::get_recipe_by_id_and_owner(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Delete from DB (enforces ownership via WHERE user_id = $2)
    crate::db::delete_recipe(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Clean up only objects which are no longer referenced by another recipe,
    // draft, or immutable version. Forks and historical snapshots deliberately
    // share object URLs with the source recipe.
    let step_photos = collect_step_photos(&recipe.instructions);
    if !recipe.images.is_empty() || !step_photos.is_empty() {
        let retained_urls: HashSet<String> = match crate::db::get_all_image_urls(&pool).await {
            Ok(urls) => urls.into_iter().collect(),
            Err(error) => {
                tracing::warn!(%error, "skipping recipe image cleanup because references could not be scanned");
                return Ok(());
            }
        };
        let storage = crate::storage::get_storage_client();
        let public_url = storage.public_url().trim_end_matches('/');

        for image in &recipe.images {
            delete_recipe_image_objects(&storage, public_url, image, &retained_urls).await;
        }

        for image in step_photos {
            delete_recipe_image_objects(&storage, public_url, &image, &retained_urls).await;
        }
    }

    Ok(())
}

#[cfg(feature = "server")]
async fn delete_recipe_image_objects(
    storage: &crate::storage::StorageClient,
    public_url: &str,
    image: &RecipeImage,
    retained_urls: &HashSet<String>,
) {
    if retained_urls.contains(&image.url) {
        return;
    }

    let Some(full_key) = image
        .url
        .strip_prefix(public_url)
        .map(|s| s.trim_start_matches('/'))
    else {
        return;
    };

    let _ = storage.delete_object(full_key).await;

    let thumb_key = if let Some(thumb_url) = image.thumb_url.as_deref() {
        (!retained_urls.contains(thumb_url))
            .then(|| thumb_url.strip_prefix(public_url))
            .flatten()
            .map(|s| s.trim_start_matches('/').to_string())
    } else {
        full_key
            .strip_suffix("_full.webp")
            .map(|base| format!("{base}_thumb.webp"))
    };

    if let Some(thumb_key) = thumb_key {
        let _ = storage.delete_object(&thumb_key).await;
    }
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
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    let recipes = crate::db::get_recipes_by_owner_paginated(&pool, user_id, limit, offset)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let total_count = crate::db::count_recipes_by_owner(&pool, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

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
    let pool = crate::db::get_pool();

    let recipes = crate::db::get_public_recipes_paginated(&pool, limit, offset)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let total_count = crate::db::count_public_recipes(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

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
    let pool = crate::db::get_pool();

    let user = crate::db::get_user_by_username(&pool, &username)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("User not found"))?;

    let recipes = crate::db::get_user_public_recipes(&pool, user.id, limit, offset)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let total_count = crate::db::count_user_public_recipes(&pool, user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

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
    let tags = crate::db::get_distinct_public_tags(&pool)
        .await
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
