//! Per-owner names for saved recipes. Drafts and immutable version snapshots do
//! not reserve names; the unique index is the final guard for concurrent writes.
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use super::DbError;

pub(super) async fn ensure_available(
    executor: impl Executor<'_, Database = Postgres>,
    user_id: Uuid,
    title: &str,
    except_id: Option<Uuid>,
) -> Result<(), DbError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM recipes WHERE user_id = $1 \
         AND lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g'))) = \
             lower(btrim(regexp_replace($2::text, '[[:space:]]+', ' ', 'g'))) \
         AND ($3::uuid IS NULL OR id <> $3))",
    )
    .bind(user_id)
    .bind(title)
    .bind(except_id)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)?;
    if exists {
        return Err(DbError::RecipeNameTaken);
    }
    Ok(())
}

pub(super) fn map_write_error(error: sqlx::Error) -> DbError {
    if error.as_database_error().is_some_and(|error| {
        error.is_unique_violation() && error.constraint() == Some("idx_recipes_owner_title_unique")
    }) {
        DbError::RecipeNameTaken
    } else {
        DbError::Query(error)
    }
}

#[cfg(test)]
#[path = "recipe_names_tests.rs"]
mod tests;
