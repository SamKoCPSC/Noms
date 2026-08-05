//! Authenticated search server functions.

use dioxus::prelude::*;

/// Search the authenticated user's visible recipe corpus.
#[server]
pub async fn search_recipes(
    request: crate::types::RecipeSearchRequest,
) -> Result<crate::types::RecipeSearchResponse, ServerFnError> {
    crate::search::validate_recipe_search_request(&request)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::search_recipes(&crate::db::get_pool(), user_id, &request)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "recipe search failed");
            ServerFnError::new("Failed to search recipes")
        })
}

/// Search the public user directory as an authenticated caller.
#[server]
pub async fn search_users(
    request: crate::types::UserSearchRequest,
) -> Result<crate::types::UserSearchResponse, ServerFnError> {
    crate::search::validate_user_search_request(&request)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::db::search_users(&crate::db::get_pool(), &request)
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "user search failed");
            ServerFnError::new("Failed to search users")
        })
}
