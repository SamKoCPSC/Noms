//! Recipe CRUD server functions.
//!
//! All functions require authentication via session cookie.
//! Functions accessing individual recipes verify ownership.

#![allow(clippy::too_many_arguments)]

use dioxus::prelude::*;

/// Create a new recipe for the authenticated user.
///
/// Tags are inserted after the recipe is created.
#[allow(clippy::too_many_arguments)]
#[server]
pub async fn create_recipe(
    title: String,
    description: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    instructions: Option<String>,
    tags: Vec<String>,
    visibility: String,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let pool = crate::db::get_pool();

    // Wrap recipe + tag insertion in a single transaction so that
    // if tag insertion fails the recipe is rolled back as well.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let recipe = crate::db::insert_recipe(
        &mut *tx,
        user_id,
        &title,
        description.as_deref(),
        prep_time_minutes,
        cook_time_minutes,
        servings,
        instructions.as_deref(),
        &visibility,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Insert tags if provided (still inside the transaction)
    if !tags.is_empty() {
        crate::db::insert_recipe_tags(&mut tx, recipe.id, &tags)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("transaction commit failed: {e}")))?;

    Ok(recipe)
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
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    instructions: Option<String>,
    tags: Option<Vec<String>>,
    visibility: Option<String>,
) -> Result<crate::types::Recipe, ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // Wrap recipe update + tag update in a single transaction;
    // ownership is enforced inside `update_recipe` via the WHERE clause.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let recipe = crate::db::update_recipe(
        &mut *tx,
        recipe_id,
        user_id,
        title.as_deref().unwrap_or(""),
        description.as_deref(),
        prep_time_minutes,
        cook_time_minutes,
        servings,
        instructions.as_deref(),
        visibility.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Update tags if provided (still inside the transaction)
    if let Some(tags) = tags {
        crate::db::insert_recipe_tags(&mut tx, recipe.id, &tags)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("transaction commit failed: {e}")))?;

    Ok(recipe)
}

/// Delete a recipe (ownership-gated).
#[server]
pub async fn delete_recipe(recipe_id: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let recipe_id = uuid::Uuid::parse_str(&recipe_id)
        .map_err(|e| ServerFnError::new(format!("Invalid recipe ID: {e}")))?;
    let pool = crate::db::get_pool();

    // Delete enforces ownership inside the DB layer via WHERE user_id = $2
    crate::db::delete_recipe(&pool, recipe_id, user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
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
