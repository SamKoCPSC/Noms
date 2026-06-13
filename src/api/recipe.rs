//! Recipe API handlers.
//!
//! Provides endpoints for recipe operations including versioned edits.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth::session,
    db::{self, diff, Recipe},
};

/// Application state for recipe API handlers.
#[derive(Clone)]
pub struct RecipeState {
    pub pool: PgPool,
}

/// Request body for updating a recipe.
#[derive(Debug, Deserialize)]
pub struct UpdateRecipeRequest {
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
}

/// Response body for a successful recipe update.
#[derive(Debug, Serialize)]
pub struct UpdateRecipeResponse {
    pub recipe: Recipe,
    pub new_version_number: i32,
}

// ── Version history types ───────────────────────────────────────────────────

/// Summary of a single version for the timeline view.
#[derive(Debug, Serialize)]
pub struct VersionSummary {
    pub version_number: i32,
    pub title: Option<String>,
    pub created_at: NaiveDateTime,
    pub is_latest: bool,
    pub notes: Option<String>,
}

/// Full reconstructed version for display in the diff viewer.
#[derive(Debug, Serialize)]
pub struct ReconstructedVersion {
    pub version_number: i32,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RestoreVersionResponse {
    pub recipe: db::Recipe,
    pub new_version_number: i32,
    pub restored_from_version: i32,
}

// ── Fork types ──────────────────────────────────────────────────────────────

/// Request body for forking a recipe.
#[derive(Debug, Deserialize)]
pub struct ForkRecipeRequest {
    pub message: Option<String>,
}

/// Response body for a successful fork.
#[derive(Debug, Serialize)]
pub struct ForkRecipeResponse {
    pub new_recipe_id: Uuid,
    pub original_recipe_id: Uuid,
}

/// GET /api/recipes/{recipe_id}/fork_info — get fork attribution for a recipe.
pub async fn get_fork_info_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path(recipe_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let _user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    match db::get_fork_info(&state.pool, recipe_id).await {
        Ok(Some((original_recipe_id, original_owner_id, message))) => {
            // Fetch the original owner's display name
            let owner_name = match db::get_user_by_id(&state.pool, original_owner_id).await {
                Ok(Some(user)) => user.display_name,
                _ => "Unknown".to_string(),
            };
            Json((original_recipe_id, owner_name, message)).into_response()
        }
        Ok(None) => {
            (StatusCode::NO_CONTENT).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// POST /api/recipes/{recipe_id}/fork — fork a recipe into a new draft.
pub async fn fork_recipe_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path(recipe_id): Path<Uuid>,
    Json(body): Json<ForkRecipeRequest>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    match db::fork_recipe(&state.pool, recipe_id, user_id, body.message).await {
        Ok((new_recipe_id, original_recipe_id, _original_title)) => {
            Json(ForkRecipeResponse {
                new_recipe_id,
                original_recipe_id,
            })
            .into_response()
        }
        Err(db::DbError::ForkError) => {
            (StatusCode::FORBIDDEN, "Cannot fork this recipe").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Draft types ──────────────────────────────────────────────────────────────

/// Summary of a recipe for the dashboard list view.
#[derive(Debug, Serialize)]
pub struct RecipeSummary {
    pub id: Uuid,
    pub title: String,
    pub is_draft: bool,
    pub description: Option<String>,
    pub updated_at: NaiveDateTime,
}

/// Response for saving (creating or updating) a draft.
#[derive(Debug, Serialize)]
pub struct SaveDraftResponse {
    pub recipe_id: Uuid,
    pub is_draft: bool,
}

/// Response for publishing a draft.
#[derive(Debug, Serialize)]
pub struct PublishRecipeResponse {
    pub recipe_id: Uuid,
}

/// Response for listing a user's recipes.
#[derive(Debug, Serialize)]
pub struct ListRecipesResponse {
    pub recipes: Vec<RecipeSummary>,
    pub draft_count: i32,
}

/// Request body for saving a draft.
#[derive(Debug, Deserialize)]
pub struct SaveDraftRequest {
    /// Existing recipe ID for updates, omitted for new drafts.
    pub recipe_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
}

/// GET /api/recipes/{id}/versions — list all versions of a recipe.
pub async fn get_recipe_versions_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path(recipe_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    // Verify ownership
    match db::get_recipe_by_id_and_owner(&state.pool, recipe_id, user_id).await {
        Ok(_) => {}
        Err(db::DbError::RecipeNotFound) => {
            return (StatusCode::NOT_FOUND, "Recipe not found").into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    let versions = match db::get_recipe_versions(&state.pool, recipe_id).await {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let summaries: Vec<VersionSummary> = versions
        .into_iter()
        .map(|v| VersionSummary {
            version_number: v.version_number,
            title: v.title,
            created_at: v.created_at.naive_utc(),
            is_latest: v.is_latest,
            notes: v.notes,
        })
        .collect();

    Json(summaries).into_response()
}

/// GET /api/recipes/{id}/versions/{version_number}/reconstruct
/// — reconstruct a specific version from the diff chain.
pub async fn reconstruct_version_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path((recipe_id, version_number)): Path<(Uuid, i32)>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    // Verify ownership
    match db::get_recipe_by_id_and_owner(&state.pool, recipe_id, user_id).await {
        Ok(_) => {}
        Err(db::DbError::RecipeNotFound) => {
            return (StatusCode::NOT_FOUND, "Recipe not found").into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    // Get all versions (ordered ASC for chain reconstruction)
    let versions = match db::get_recipe_versions(&state.pool, recipe_id).await {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Find the target version
    let target_version = match versions.iter().find(|v| v.version_number == version_number) {
        Some(v) => v,
        None => {
            return (StatusCode::NOT_FOUND, "Version not found").into_response();
        }
    };

    // If the target is the latest version, return it directly (no reconstruction needed)
    if target_version.is_latest {
        let summary = ReconstructedVersion {
            version_number: target_version.version_number,
            title: target_version
                .title
                .clone()
                .unwrap_or_else(|| "Untitled".to_string()),
            description: target_version.description.clone(),
            prep_time_min: target_version.prep_time_min,
            cook_time_min: target_version.cook_time_min,
            total_time_min: target_version.total_time_min,
            servings: target_version.servings,
            ingredients: target_version.ingredients.clone(),
            steps: target_version.steps.clone(),
            created_at: target_version.created_at.naive_utc(),
            notes: target_version.notes.clone(),
        };
        return Json(summary).into_response();
    }

    // Get the latest version's full snapshot
    let latest = match versions.iter().find(|v| v.is_latest) {
        Some(v) => v,
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "No latest version found").into_response();
        }
    };

    // Serialize latest version to JSON
    let latest_json = match diff::recipe_to_json(latest) {
        Ok(j) => j,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // Collect reverse diffs from latest-1 down to target version (ordered newest to oldest)
    let reverse_diffs: Vec<serde_json::Value> = versions
        .iter()
        .filter(|v| v.version_number >= target_version.version_number && v.reverse_diff.is_some())
        .map(|v| v.reverse_diff.clone().unwrap())
        // Already in ASC order (oldest to newest), but we need newest to oldest
        .rev()
        .collect();

    // Reconstruct the target version
    let reconstructed_json =
        match diff::reconstruct_from_chain(&latest_json, &reverse_diffs) {
            Ok(j) => j,
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        };

    // Deserialize back to a snapshot-like structure
    let snapshot = match diff::json_to_recipe(&reconstructed_json) {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let result = ReconstructedVersion {
        version_number: target_version.version_number,
        title: snapshot.title,
        description: snapshot.description,
        prep_time_min: snapshot.prep_time_min,
        cook_time_min: snapshot.cook_time_min,
        total_time_min: snapshot.total_time_min,
        servings: snapshot.servings,
        ingredients: Some(serde_json::Value::Array(snapshot.ingredients)),
        steps: Some(serde_json::Value::Array(snapshot.steps)),
        created_at: target_version.created_at.naive_utc(),
        notes: target_version.notes.clone(),
    };

    Json(result).into_response()
}

/// POST /api/recipes/{recipe_id}/versions/{version_number}/restore
/// — restore a historical version by creating a new version with its data.
pub async fn restore_version_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path((recipe_id, version_number)): Path<(Uuid, i32)>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    // Verify ownership
    match db::get_recipe_by_id_and_owner(&state.pool, recipe_id, user_id).await {
        Ok(_) => {}
        Err(db::DbError::RecipeNotFound) => {
            return (StatusCode::NOT_FOUND, "Recipe not found").into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }

    match db::restore_version(&state.pool, &recipe_id, &user_id, version_number).await {
        Ok((recipe, new_version_number)) => Json(RestoreVersionResponse {
            recipe,
            new_version_number,
            restored_from_version: version_number,
        })
        .into_response(),
        Err(db::DbError::VersionNotFound) => {
            (StatusCode::NOT_FOUND, format!("Version {} not found", version_number)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// POST /api/recipes/drafts — create or update a draft recipe.
pub async fn save_draft_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Json(body): Json<SaveDraftRequest>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    let result = if let Some(recipe_id) = body.recipe_id {
        // Update existing draft
        match db::update_draft(
            &state.pool,
            recipe_id,
            user_id,
            &body.title,
            body.description.as_deref(),
            body.prep_time_min,
            body.cook_time_min,
            body.total_time_min,
            body.servings,
            body.ingredients.as_ref(),
            body.steps.as_ref(),
        )
        .await
        {
            Ok(recipe) => SaveDraftResponse {
                recipe_id: recipe.id,
                is_draft: recipe.is_draft,
            },
            Err(db::DbError::RecipeNotFound) => {
                return (StatusCode::NOT_FOUND, "Recipe not found").into_response();
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        }
    } else {
        // Create new draft
        match db::create_draft_recipe(
            &state.pool,
            user_id,
            &body.title,
            body.description.as_deref(),
            body.prep_time_min,
            body.cook_time_min,
            body.total_time_min,
            body.servings,
            body.ingredients.as_ref(),
            body.steps.as_ref(),
        )
        .await
        {
            Ok(recipe) => SaveDraftResponse {
                recipe_id: recipe.id,
                is_draft: recipe.is_draft,
            },
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
            }
        }
    };

    Json(result).into_response()
}

/// POST /api/recipes/{recipe_id}/publish — publish a draft recipe.
pub async fn publish_recipe_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path(recipe_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    match db::publish_recipe(&state.pool, recipe_id, user_id).await {
        Ok(_) => Json(PublishRecipeResponse { recipe_id }).into_response(),
        Err(db::DbError::RecipeNotFound) => {
            (StatusCode::NOT_FOUND, "Recipe not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Query parameters for listing recipes.
#[derive(Debug, Deserialize)]
pub struct ListRecipesQuery {
    #[serde(default)]
    pub include_drafts: bool,
}

/// GET /api/recipes?include_drafts=true — list the current user's recipes.
pub async fn list_my_recipes_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    axum::extract::Query(query): axum::extract::Query<ListRecipesQuery>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    let include_drafts = query.include_drafts;

    let recipes = match db::get_recipes_by_owner_with_draft_filter(
        &state.pool,
        user_id,
        include_drafts,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    let summaries: Vec<RecipeSummary> = recipes
        .iter()
        .map(|r| RecipeSummary {
            id: r.id,
            title: r.title.clone(),
            is_draft: r.is_draft,
            description: r.description.clone(),
            updated_at: r.updated_at.naive_utc(),
        })
        .collect();

    let draft_count = recipes.iter().filter(|r| r.is_draft).count() as i32;

    Json(ListRecipesResponse {
        recipes: summaries,
        draft_count,
    })
    .into_response()
}

/// GET /api/recipes/{recipe_id} — get a single recipe by ID.
pub async fn get_recipe_api(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path(recipe_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    match db::get_recipe_by_id_and_owner(&state.pool, recipe_id, user_id).await {
        Ok(recipe) => Json(&recipe).into_response(),
        Err(db::DbError::RecipeNotFound) => {
            (StatusCode::NOT_FOUND, "Recipe not found").into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Update a recipe with versioning (PUT /api/recipes/{recipe_id}/update).
///
/// Flow: verify session -> verify ownership -> transaction:
///   1. Get current latest version
///   2. Serialize old/new JSON via diff helpers
///   3. Compute forward + reverse diff
///   4. Mark current latest as historical (store reverse_diff)
///   5. Insert new version as latest full snapshot (version_number + 1)
///   6. Update recipe metadata (title, updated_at)
pub async fn update_recipe(
    State(state): State<RecipeState>,
    jar: CookieJar,
    Path(recipe_id): Path<Uuid>,
    Json(body): Json<UpdateRecipeRequest>,
) -> axum::response::Response {
    // Check for valid session
    let Some(cookie) = jar.get(session::COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "Missing session cookie").into_response();
    };

    let user_id = match session::verify_session(&state.pool, cookie.value()).await {
        Ok(user_id) => user_id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid session").into_response(),
    };

    match db::update_recipe_versioned(
        &state.pool,
        &recipe_id,
        &user_id,
        &body.title,
        body.description.as_deref(),
        body.prep_time_min,
        body.cook_time_min,
        body.total_time_min,
        body.servings,
        &body.ingredients,
        &body.steps,
        None,
    )
    .await
    {
        Ok((recipe, version)) => Json(UpdateRecipeResponse {
            recipe,
            new_version_number: version,
        })
        .into_response(),
        Err(e) => match e {
            db::DbError::RecipeNotFound => {
                (StatusCode::NOT_FOUND, "Recipe not found").into_response()
            }
            db::DbError::VersionNotFound => {
                (StatusCode::NOT_FOUND, "No version found").into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
    }
}
