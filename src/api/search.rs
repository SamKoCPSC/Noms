//! Authenticated search server functions.

use dioxus::prelude::*;

#[cfg(feature = "server")]
fn enforce_budget(
    user_id: uuid::Uuid,
    request_class: crate::security::RequestClass,
) -> Result<(), ServerFnError> {
    crate::security::enforce_user_request_budget(user_id, request_class)
        .map_err(|_| ServerFnError::new("Request rate limit exceeded"))
}

#[cfg(feature = "server")]
async fn database_budget<T>(
    operation: impl std::future::Future<Output = Result<T, crate::db::DbError>>,
) -> Result<T, ServerFnError> {
    crate::security::run_with_database_budget(crate::security::DATABASE_EXECUTION_BUDGET, operation)
        .await
        .map_err(|_| ServerFnError::new("Database operation timed out"))?
        .map_err(|error| {
            tracing::error!(%error, "bounded search operation failed");
            ServerFnError::new("Search operation failed")
        })
}

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
    enforce_budget(user_id, crate::security::RequestClass::Search)?;
    let pool = crate::db::get_pool();
    database_budget(crate::db::search_recipes(&pool, user_id, &request)).await
}

/// Search the public user directory as an authenticated caller.
#[server]
pub async fn search_users(
    request: crate::types::UserSearchRequest,
) -> Result<crate::types::UserSearchResponse, ServerFnError> {
    crate::search::validate_user_search_request(&request)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    enforce_budget(user_id, crate::security::RequestClass::Search)?;
    let pool = crate::db::get_pool();
    database_budget(crate::db::search_users(&pool, &request)).await
}

/// Search collections owned by the authenticated user.
#[server]
pub async fn search_collections(
    request: crate::types::CollectionSearchRequest,
) -> Result<crate::types::CollectionSearchResponse, ServerFnError> {
    crate::search::validate_collection_search_request(&request)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    enforce_budget(user_id, crate::security::RequestClass::Search)?;
    let pool = crate::db::get_pool();
    database_budget(crate::db::search_collections(&pool, user_id, &request)).await
}

/// Search recipes, owned collections, and public users in independent sections.
#[server]
pub async fn search_all(
    request: crate::types::AllSearchRequest,
) -> Result<crate::types::SearchResponse, ServerFnError> {
    crate::search::validate_all_search_request(&request)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    enforce_budget(user_id, crate::security::RequestClass::Search)?;
    let pool = crate::db::get_pool();
    database_budget(crate::db::search_all(&pool, user_id, &request)).await
}

/// Return bounded typed suggestions for the authenticated caller.
#[server]
pub async fn autocomplete_entities(
    scope: crate::types::SearchScope,
    text: String,
) -> Result<Vec<crate::types::SearchSuggestion>, ServerFnError> {
    crate::search::validate_autocomplete_text(&text)
        .map_err(|error| ServerFnError::new(error.to_string()))?;
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    enforce_budget(user_id, crate::security::RequestClass::Autocomplete)?;
    let pool = crate::db::get_pool();
    database_budget(crate::db::autocomplete_entities(
        &pool, user_id, scope, &text,
    ))
    .await
}
