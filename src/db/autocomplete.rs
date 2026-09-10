//! Typed, bounded autocomplete routing across fixed entity queries.

use sqlx::PgPool;
use uuid::Uuid;

use super::{
    collection_search::autocomplete_collections, recipe_search::autocomplete_recipes,
    user_search::autocomplete_users, DbError,
};
use crate::search::{
    validate_autocomplete_text, ALL_AUTOCOMPLETE_COLLECTION_LIMIT, ALL_AUTOCOMPLETE_RECIPE_LIMIT,
    ALL_AUTOCOMPLETE_USER_LIMIT, MAX_AUTOCOMPLETE_SUGGESTIONS,
};
use crate::types::{SearchScope, SearchSuggestion};

/// Return suggestions from only the fixed entity queries selected by `scope`.
pub async fn autocomplete_entities(
    pool: &PgPool,
    resolved_user_id: Uuid,
    scope: SearchScope,
    text: &str,
) -> Result<Vec<SearchSuggestion>, DbError> {
    validate_autocomplete_text(text)
        .map_err(|error| DbError::InvalidSearchRequest(error.to_string()))?;

    match scope {
        SearchScope::Recipes => {
            autocomplete_recipes(
                pool,
                resolved_user_id,
                text,
                MAX_AUTOCOMPLETE_SUGGESTIONS as u32,
            )
            .await
        }
        SearchScope::Collections => {
            autocomplete_collections(
                pool,
                resolved_user_id,
                text,
                MAX_AUTOCOMPLETE_SUGGESTIONS as u32,
            )
            .await
        }
        SearchScope::Users => {
            autocomplete_users(pool, text, MAX_AUTOCOMPLETE_SUGGESTIONS as u32).await
        }
        SearchScope::All => {
            let mut suggestions = autocomplete_recipes(
                pool,
                resolved_user_id,
                text,
                ALL_AUTOCOMPLETE_RECIPE_LIMIT as u32,
            )
            .await?;
            suggestions.extend(
                autocomplete_collections(
                    pool,
                    resolved_user_id,
                    text,
                    ALL_AUTOCOMPLETE_COLLECTION_LIMIT as u32,
                )
                .await?,
            );
            suggestions
                .extend(autocomplete_users(pool, text, ALL_AUTOCOMPLETE_USER_LIMIT as u32).await?);
            Ok(suggestions)
        }
    }
}
