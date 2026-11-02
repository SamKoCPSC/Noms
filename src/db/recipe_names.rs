//! Translate only the recipe-name index collision into actionable editor feedback.
use super::DbError;

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
