//! Grouped search across recipes, owned collections, and public user profiles.

use sqlx::PgPool;
use uuid::Uuid;

use super::{
    collection_search::search_collection_section, recipe_search::search_recipe_section,
    user_search::search_user_section, DbError,
};
use crate::search::validate_all_search_request;
use crate::types::{AllSearchRequest, SearchResponse, SearchScope};

/// Search each fixed section independently without comparing scores across indexes.
pub async fn search_all(
    pool: &PgPool,
    resolved_user_id: Uuid,
    request: &AllSearchRequest,
) -> Result<SearchResponse, DbError> {
    validate_all_search_request(request)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    let (recipes, recipe_total_count) =
        search_recipe_section(pool, resolved_user_id, &request.text, request.section_limit).await?;
    let (collections, collection_total_count) =
        search_collection_section(pool, resolved_user_id, &request.text, request.section_limit)
            .await?;
    let (users, user_total_count) =
        search_user_section(pool, &request.text, request.section_limit).await?;

    let response = SearchResponse {
        scope: SearchScope::All,
        recipes,
        recipe_total_count,
        collections,
        collection_total_count,
        users,
        user_total_count,
    };
    crate::search::validate_search_response(&response)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;
    Ok(response)
}
