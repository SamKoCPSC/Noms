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
