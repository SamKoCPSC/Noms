use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use super::diff::{
    compute_diff, json_to_snapshot, recipe_to_snapshot, snapshot_to_json, RecipeSnapshot,
    SnapshotChange,
};
use super::{DbError, Recipe};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RecipeVersion {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub version_number: i32,
    pub snapshot: serde_json::Value,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl RecipeVersion {
    pub fn decoded_snapshot(&self) -> Result<RecipeSnapshot, DbError> {
        json_to_snapshot(&self.snapshot)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RecipeDraft {
    pub id: Uuid,
    pub recipe_id: Option<Uuid>,
    pub user_id: Uuid,
    pub base_version_number: Option<i32>,
    pub snapshot: serde_json::Value,
    pub revision: i64,
    pub forked_from_recipe_id: Option<Uuid>,
    pub forked_from_version_number: Option<i32>,
    pub forked_from_title: Option<String>,
    pub fork_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RecipeDraft {
    pub fn decoded_snapshot(&self) -> Result<RecipeSnapshot, DbError> {
        json_to_snapshot(&self.snapshot)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ForkRelationship {
    pub id: Uuid,
    pub original_recipe_id: Option<Uuid>,
    pub forked_recipe_id: Uuid,
    pub forked_by: Uuid,
    pub forked_version_number: Option<i32>,
    pub original_title: String,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

const VERSION_COLUMNS: &str = "id, recipe_id, version_number, snapshot, notes, created_at";
const DRAFT_COLUMNS: &str = "id, recipe_id, user_id, base_version_number, snapshot, revision, \
     forked_from_recipe_id, forked_from_version_number, forked_from_title, fork_message, \
     created_at, updated_at";

pub async fn get_recipe_versions(
    pool: &PgPool,
    recipe_id: Uuid,
) -> Result<Vec<RecipeVersion>, DbError> {
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = $1 ORDER BY version_number DESC"
    ))
    .bind(recipe_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)
}

pub async fn get_latest_recipe_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<RecipeVersion, DbError> {
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = $1 ORDER BY version_number DESC LIMIT 1"
    ))
    .bind(recipe_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::VersionNotFound {
        recipe_id,
        version_number: 0,
    })
}

pub async fn get_recipe_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
) -> Result<RecipeVersion, DbError> {
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "SELECT {VERSION_COLUMNS} FROM recipe_versions \
         WHERE recipe_id = $1 AND version_number = $2"
    ))
    .bind(recipe_id)
    .bind(version_number)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::VersionNotFound {
        recipe_id,
        version_number,
    })
}

pub async fn compare_recipe_versions(
    pool: &PgPool,
    recipe_id: Uuid,
    from_version_number: i32,
    to_version_number: i32,
) -> Result<Vec<SnapshotChange>, DbError> {
    let from = get_recipe_version(pool, recipe_id, from_version_number)
        .await?
        .decoded_snapshot()?;
    let to = get_recipe_version(pool, recipe_id, to_version_number)
        .await?
        .decoded_snapshot()?;
    compute_diff(&from, &to)
}

pub async fn get_next_version_number(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<i32, DbError> {
    let latest = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(version_number) FROM recipe_versions WHERE recipe_id = $1",
    )
    .bind(recipe_id)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(latest.unwrap_or(0) + 1)
}

pub async fn insert_recipe_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
    snapshot: &RecipeSnapshot,
    notes: Option<&str>,
) -> Result<RecipeVersion, DbError> {
    let snapshot = snapshot_to_json(snapshot)?;
    sqlx::query_as::<_, RecipeVersion>(&format!(
        "INSERT INTO recipe_versions (recipe_id, version_number, snapshot, notes) \
         VALUES ($1, $2, $3, $4) RETURNING {VERSION_COLUMNS}"
    ))
    .bind(recipe_id)
    .bind(version_number)
    .bind(snapshot)
    .bind(notes)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

#[allow(clippy::too_many_arguments)]
async fn insert_recipe_draft_with_source(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
    base_version_number: Option<i32>,
    snapshot: &RecipeSnapshot,
    forked_from_recipe_id: Option<Uuid>,
    forked_from_version_number: Option<i32>,
    forked_from_title: Option<&str>,
    fork_message: Option<&str>,
) -> Result<RecipeDraft, DbError> {
    let snapshot = snapshot_to_json(snapshot)?;
    let inserted = sqlx::query_as::<_, RecipeDraft>(&format!(
        "INSERT INTO recipe_drafts (recipe_id, user_id, base_version_number, snapshot, \
             forked_from_recipe_id, forked_from_version_number, forked_from_title, fork_message) \
         SELECT $1, $2, $3, $4, $5, $6, $7, $8 \
         WHERE $1::UUID IS NULL OR EXISTS (\
             SELECT 1 FROM recipes WHERE id = $1 AND user_id = $2\
         ) RETURNING {DRAFT_COLUMNS}"
    ))
    .bind(recipe_id)
    .bind(user_id)
    .bind(base_version_number)
    .bind(snapshot)
    .bind(forked_from_recipe_id)
    .bind(forked_from_version_number)
    .bind(forked_from_title)
    .bind(fork_message)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?;
    inserted.ok_or_else(|| {
        recipe_id
            .map(DbError::RecipeNotFound)
            .unwrap_or(DbError::DraftNotFound(Uuid::nil()))
    })
}

pub async fn insert_recipe_draft(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
    base_version_number: Option<i32>,
    snapshot: &RecipeSnapshot,
) -> Result<RecipeDraft, DbError> {
    insert_recipe_draft_with_source(
        executor,
        user_id,
        recipe_id,
        base_version_number,
        snapshot,
        None,
        None,
        None,
        None,
    )
    .await
}

pub async fn create_new_recipe_draft(
    pool: &PgPool,
    user_id: Uuid,
    snapshot: &RecipeSnapshot,
) -> Result<RecipeDraft, DbError> {
    insert_recipe_draft(pool, user_id, None, None, snapshot).await
}

pub async fn get_recipe_draft(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    draft_id: Uuid,
    user_id: Uuid,
) -> Result<RecipeDraft, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts WHERE id = $1 AND user_id = $2"
    ))
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::DraftNotFound(draft_id))
}

pub async fn get_recipe_draft_for_recipe(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
) -> Result<Option<RecipeDraft>, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts \
         WHERE recipe_id = $1 AND user_id = $2"
    ))
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

pub async fn list_recipe_drafts(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<RecipeDraft>, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts \
         WHERE user_id = $1 ORDER BY updated_at DESC"
    ))
    .bind(user_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

pub async fn update_recipe_draft(
    pool: &PgPool,
    draft_id: Uuid,
    user_id: Uuid,
    expected_revision: i64,
    snapshot: &RecipeSnapshot,
) -> Result<RecipeDraft, DbError> {
    let snapshot = snapshot_to_json(snapshot)?;
    let updated = sqlx::query_as::<_, RecipeDraft>(&format!(
        "UPDATE recipe_drafts SET snapshot = $4, revision = revision + 1, updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 AND revision = $3 RETURNING {DRAFT_COLUMNS}"
    ))
    .bind(draft_id)
    .bind(user_id)
    .bind(expected_revision)
    .bind(snapshot)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;
    if let Some(updated) = updated {
        return Ok(updated);
    }

    let actual = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM recipe_drafts WHERE id = $1 AND user_id = $2",
    )
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::DraftNotFound(draft_id))?;
    Err(DbError::DraftConflict {
        draft_id,
        expected: expected_revision,
        actual,
    })
}

pub async fn delete_recipe_draft(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    draft_id: Uuid,
    user_id: Uuid,
) -> Result<(), DbError> {
    let affected = sqlx::query("DELETE FROM recipe_drafts WHERE id = $1 AND user_id = $2")
        .bind(draft_id)
        .bind(user_id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    if affected == 0 {
        return Err(DbError::DraftNotFound(draft_id));
    }
    Ok(())
}

pub async fn create_edit_draft(
    pool: &PgPool,
    recipe_id: Uuid,
    user_id: Uuid,
) -> Result<RecipeDraft, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let locked_recipe = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM recipes WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if locked_recipe.is_none() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    if let Some(draft) = get_recipe_draft_for_recipe(&mut *transaction, recipe_id, user_id).await? {
        transaction.commit().await.map_err(DbError::Query)?;
        return Ok(draft);
    }
    let recipe = super::get_recipe_by_id_and_owner(&mut *transaction, recipe_id, user_id).await?;
    let tags = super::get_recipe_tags(&mut *transaction, recipe_id)
        .await?
        .into_iter()
        .map(|tag| tag.tag)
        .collect();
    let snapshot = recipe_to_snapshot(&recipe, tags);
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;

    let draft = insert_recipe_draft(
        &mut *transaction,
        user_id,
        Some(recipe_id),
        Some(latest.version_number),
        &snapshot,
    )
    .await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(draft)
}

pub async fn create_published_recipe(
    pool: &PgPool,
    user_id: Uuid,
    snapshot: &RecipeSnapshot,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let snapshot = normalized_snapshot(snapshot);
    validate_snapshot_for_publish(&snapshot)?;
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let recipe = insert_recipe_from_snapshot(&mut transaction, user_id, &snapshot).await?;
    insert_recipe_version(&mut *transaction, recipe.id, 1, &snapshot, notes).await?;
    super::insert_recipe_tags(&mut transaction, recipe.id, &snapshot.tags).await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

async fn insert_recipe_from_snapshot(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    snapshot: &RecipeSnapshot,
) -> Result<Recipe, DbError> {
    super::insert_recipe(
        &mut **transaction,
        user_id,
        &snapshot.title,
        snapshot.description.as_deref(),
        snapshot.commentary.as_deref(),
        snapshot.prep_time_minutes,
        snapshot.cook_time_minutes,
        snapshot.servings,
        &snapshot.ingredients,
        &snapshot.instructions,
        &snapshot.equipment,
        &snapshot.images,
        &snapshot.visibility,
    )
    .await
}

async fn update_recipe_from_snapshot(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    recipe_id: Uuid,
    user_id: Uuid,
    snapshot: &RecipeSnapshot,
) -> Result<Recipe, DbError> {
    let recipe = super::update_recipe(
        &mut **transaction,
        recipe_id,
        user_id,
        &snapshot.title,
        snapshot.description.as_deref(),
        snapshot.commentary.as_deref(),
        snapshot.prep_time_minutes,
        snapshot.cook_time_minutes,
        snapshot.servings,
        &snapshot.ingredients,
        &snapshot.instructions,
        &snapshot.equipment,
        &snapshot.images,
        Some(&snapshot.visibility),
    )
    .await?;
    super::insert_recipe_tags(transaction, recipe_id, &snapshot.tags).await?;
    Ok(recipe)
}

pub async fn publish_recipe_draft(
    pool: &PgPool,
    draft_id: Uuid,
    user_id: Uuid,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let draft = get_recipe_draft(pool, draft_id, user_id).await?;
    match draft.recipe_id {
        Some(recipe_id) => publish_existing_draft(pool, draft_id, recipe_id, user_id, notes).await,
        None => publish_new_draft(pool, draft_id, user_id, notes).await,
    }
}

async fn locked_draft(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    draft_id: Uuid,
    user_id: Uuid,
) -> Result<RecipeDraft, DbError> {
    sqlx::query_as::<_, RecipeDraft>(&format!(
        "SELECT {DRAFT_COLUMNS} FROM recipe_drafts \
         WHERE id = $1 AND user_id = $2 FOR UPDATE"
    ))
    .bind(draft_id)
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(DbError::Query)?
    .ok_or(DbError::DraftNotFound(draft_id))
}

async fn publish_new_draft(
    pool: &PgPool,
    draft_id: Uuid,
    user_id: Uuid,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let draft = locked_draft(&mut transaction, draft_id, user_id).await?;
    if draft.recipe_id.is_some() {
        return Err(DbError::DraftNotFound(draft_id));
    }
    let snapshot = normalized_snapshot(&draft.decoded_snapshot()?);
    validate_snapshot_for_publish(&snapshot)?;
    let recipe = insert_recipe_from_snapshot(&mut transaction, user_id, &snapshot).await?;
    insert_recipe_version(&mut *transaction, recipe.id, 1, &snapshot, notes).await?;
    super::insert_recipe_tags(&mut transaction, recipe.id, &snapshot.tags).await?;

    if let (Some(original_title), Some(version_number)) = (
        draft.forked_from_title.as_deref(),
        draft.forked_from_version_number,
    ) {
        sqlx::query(
            "INSERT INTO fork_relationships (original_recipe_id, forked_recipe_id, forked_by, \
                 forked_version_number, original_title, message) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(draft.forked_from_recipe_id)
        .bind(recipe.id)
        .bind(user_id)
        .bind(version_number)
        .bind(original_title)
        .bind(draft.fork_message.as_deref())
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    }

    sqlx::query("DELETE FROM recipe_drafts WHERE id = $1")
        .bind(draft_id)
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

async fn publish_existing_draft(
    pool: &PgPool,
    draft_id: Uuid,
    recipe_id: Uuid,
    user_id: Uuid,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let owner = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM recipes WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if owner.is_none() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }

    let draft = locked_draft(&mut transaction, draft_id, user_id).await?;
    if draft.recipe_id != Some(recipe_id) {
        return Err(DbError::DraftNotFound(draft_id));
    }
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;
    let expected = draft.base_version_number.unwrap_or(0);
    if expected != latest.version_number {
        return Err(DbError::VersionConflict {
            recipe_id,
            expected,
            actual: latest.version_number,
        });
    }

    let snapshot = normalized_snapshot(&draft.decoded_snapshot()?);
    validate_snapshot_for_publish(&snapshot)?;
    insert_recipe_version(
        &mut *transaction,
        recipe_id,
        latest.version_number + 1,
        &snapshot,
        notes,
    )
    .await?;
    let recipe =
        update_recipe_from_snapshot(&mut transaction, recipe_id, user_id, &snapshot).await?;
    sqlx::query("DELETE FROM recipe_drafts WHERE id = $1")
        .bind(draft_id)
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

pub async fn publish_snapshot(
    pool: &PgPool,
    recipe_id: Uuid,
    user_id: Uuid,
    expected_latest_version: i32,
    snapshot: &RecipeSnapshot,
    notes: Option<&str>,
) -> Result<Recipe, DbError> {
    let snapshot = normalized_snapshot(snapshot);
    validate_snapshot_for_publish(&snapshot)?;
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let owner = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM recipes WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(recipe_id)
    .bind(user_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(DbError::Query)?;
    if owner.is_none() {
        return Err(DbError::RecipeNotFound(recipe_id));
    }
    let latest = get_latest_recipe_version(&mut *transaction, recipe_id).await?;
    if latest.version_number != expected_latest_version {
        return Err(DbError::VersionConflict {
            recipe_id,
            expected: expected_latest_version,
            actual: latest.version_number,
        });
    }
    insert_recipe_version(
        &mut *transaction,
        recipe_id,
        latest.version_number + 1,
        &snapshot,
        notes,
    )
    .await?;
    let recipe =
        update_recipe_from_snapshot(&mut transaction, recipe_id, user_id, &snapshot).await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(recipe)
}

pub async fn restore_recipe_version(
    pool: &PgPool,
    recipe_id: Uuid,
    user_id: Uuid,
    version_number: i32,
    expected_latest_version: i32,
) -> Result<Recipe, DbError> {
    let target = get_recipe_version(pool, recipe_id, version_number).await?;
    let snapshot = target.decoded_snapshot()?;
    publish_snapshot(
        pool,
        recipe_id,
        user_id,
        expected_latest_version,
        &snapshot,
        Some(&format!("Restored from v{version_number}")),
    )
    .await
}

pub async fn create_fork_draft(
    pool: &PgPool,
    source_recipe_id: Uuid,
    user_id: Uuid,
    message: Option<&str>,
) -> Result<RecipeDraft, DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    let locked_source =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM recipes WHERE id = $1 FOR SHARE")
            .bind(source_recipe_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DbError::Query)?;
    if locked_source.is_none() {
        return Err(DbError::ForkError("source recipe not found".to_string()));
    }
    let source = super::get_recipe_by_id(&mut *transaction, source_recipe_id)
        .await?
        .ok_or_else(|| DbError::ForkError("source recipe not found".to_string()))?;
    if source.user_id != user_id && !matches!(source.visibility.as_str(), "public" | "unlisted") {
        return Err(DbError::ForkError(
            "private recipes can only be forked by their owner".to_string(),
        ));
    }
    let latest = get_latest_recipe_version(&mut *transaction, source_recipe_id).await?;
    let snapshot = latest.decoded_snapshot()?;
    let draft = insert_recipe_draft_with_source(
        &mut *transaction,
        user_id,
        None,
        None,
        &snapshot,
        Some(source_recipe_id),
        Some(latest.version_number),
        Some(&source.title),
        message,
    )
    .await?;
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(draft)
}

pub async fn get_fork_relationship(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    forked_recipe_id: Uuid,
) -> Result<Option<ForkRelationship>, DbError> {
    sqlx::query_as::<_, ForkRelationship>(
        "SELECT id, original_recipe_id, forked_recipe_id, forked_by, forked_version_number, \
         original_title, message, created_at FROM fork_relationships WHERE forked_recipe_id = $1",
    )
    .bind(forked_recipe_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

pub async fn count_recipe_versions_for_draft(
    pool: &PgPool,
    draft_id: Uuid,
) -> Result<i64, DbError> {
    sqlx::query_scalar(
        "SELECT COUNT(recipe_versions.id) FROM recipe_drafts \
         LEFT JOIN recipe_versions ON recipe_versions.recipe_id = recipe_drafts.recipe_id \
         WHERE recipe_drafts.id = $1",
    )
    .bind(draft_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Query)
}

pub fn validate_snapshot_for_publish(snapshot: &RecipeSnapshot) -> Result<(), DbError> {
    if snapshot.title.trim().is_empty() {
        return Err(DbError::InvalidRecipeSnapshot(
            "title is required".to_string(),
        ));
    }
    if !matches!(
        snapshot.visibility.as_str(),
        "private" | "unlisted" | "public"
    ) {
        return Err(DbError::InvalidRecipeSnapshot(
            "visibility must be private, unlisted, or public".to_string(),
        ));
    }
    if snapshot.ingredients.is_empty()
        || snapshot
            .ingredients
            .iter()
            .all(|ingredient| ingredient.name.trim().is_empty())
    {
        return Err(DbError::InvalidRecipeSnapshot(
            "at least one ingredient is required".to_string(),
        ));
    }
    if snapshot.instructions.is_empty() || !contains_step_text(&snapshot.instructions) {
        return Err(DbError::InvalidRecipeSnapshot(
            "at least one instruction is required".to_string(),
        ));
    }
    if snapshot.prep_time_minutes.is_some_and(|value| value < 0)
        || snapshot.cook_time_minutes.is_some_and(|value| value < 0)
    {
        return Err(DbError::InvalidRecipeSnapshot(
            "recipe times cannot be negative".to_string(),
        ));
    }
    if snapshot.servings.is_some_and(|value| value <= 0) {
        return Err(DbError::InvalidRecipeSnapshot(
            "servings must be positive".to_string(),
        ));
    }
    Ok(())
}

fn contains_step_text(steps: &[crate::types::RecipeStep]) -> bool {
    steps
        .iter()
        .any(|step| !step.text.trim().is_empty() || contains_step_text(&step.sub_steps))
}

fn normalized_snapshot(snapshot: &RecipeSnapshot) -> RecipeSnapshot {
    let mut snapshot = snapshot.clone();
    snapshot.title = snapshot.title.trim().to_string();
    snapshot.tags = snapshot
        .tags
        .into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    snapshot.tags.sort();
    snapshot.tags.dedup();
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diff::snapshot_to_json;
    use crate::db::{insert_user, DbError};
    use crate::test_utils;

    async fn owner(pool: &sqlx::PgPool, prefix: &str) -> crate::db::User {
        let suffix = test_utils::uid();
        insert_user(
            pool,
            &format!("{prefix}_{suffix}"),
            "Version Owner",
            &format!("{prefix}_{suffix}@example.test"),
            None,
        )
        .await
        .unwrap()
    }

    fn snapshot(title: &str, visibility: &str) -> crate::db::diff::RecipeSnapshot {
        crate::db::diff::RecipeSnapshot {
            title: title.to_string(),
            description: Some("description".to_string()),
            commentary: Some("commentary".to_string()),
            prep_time_minutes: Some(5),
            cook_time_minutes: Some(15),
            servings: Some(2),
            ingredients: vec![crate::types::RecipeIngredient {
                amount: "1".into(),
                unit: "cup".into(),
                name: "beans".into(),
            }],
            instructions: vec![crate::types::RecipeStep {
                text: "Cook".into(),
                sub_steps: vec![],
                photos: vec![],
            }],
            equipment: vec![crate::types::RecipeEquipment { name: "Pot".into() }],
            images: vec![],
            visibility: visibility.to_string(),
            tags: vec!["quick".into(), "vegan".into()],
        }
    }

    #[tokio::test]
    async fn publishing_new_draft_creates_recipe_v1_tags_and_deletes_draft() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "newdraft").await;
        let draft = create_new_recipe_draft(&pool, user.id, &snapshot("First", "public"))
            .await
            .unwrap();

        assert_eq!(
            count_recipe_versions_for_draft(&pool, draft.id)
                .await
                .unwrap(),
            0
        );

        let published = publish_recipe_draft(&pool, draft.id, user.id, Some("Initial"))
            .await
            .unwrap();
        assert_eq!(published.title, "First");
        let versions = get_recipe_versions(&pool, published.id).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version_number, 1);
        assert_eq!(
            versions[0].snapshot,
            snapshot_to_json(&snapshot("First", "public")).unwrap()
        );
        assert_eq!(
            crate::db::get_recipe_tags(&pool, published.id)
                .await
                .unwrap()
                .len(),
            2
        );
        assert!(matches!(
            get_recipe_draft(&pool, draft.id, user.id).await,
            Err(DbError::DraftNotFound(id)) if id == draft.id
        ));
    }

    #[tokio::test]
    async fn autosave_is_owner_only_revision_guarded_and_never_versions_or_publishes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "autosave").await;
        let intruder = owner(&pool, "intruder").await;
        let published =
            create_published_recipe(&pool, user.id, &snapshot("Published", "public"), None)
                .await
                .unwrap();
        assert!(matches!(
            insert_recipe_draft(
                &pool,
                intruder.id,
                Some(published.id),
                Some(1),
                &snapshot("Unauthorized", "private"),
            )
            .await,
            Err(DbError::RecipeNotFound(id)) if id == published.id
        ));
        let draft = create_edit_draft(&pool, published.id, user.id)
            .await
            .unwrap();
        let changed = snapshot("Draft only", "private");

        let saved = update_recipe_draft(&pool, draft.id, user.id, 0, &changed)
            .await
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert!(matches!(
            update_recipe_draft(&pool, draft.id, user.id, 0, &changed).await,
            Err(DbError::DraftConflict {
                expected: 0,
                actual: 1,
                ..
            })
        ));
        assert!(matches!(
            get_recipe_draft(&pool, draft.id, intruder.id).await,
            Err(DbError::DraftNotFound(_))
        ));
        assert_eq!(
            get_recipe_versions(&pool, published.id)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            crate::db::get_recipe_by_id(&pool, published.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "Published"
        );
    }

    #[tokio::test]
    async fn publishing_existing_draft_creates_exactly_one_next_full_version() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "existing").await;
        let published = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, published.id, user.id)
            .await
            .unwrap();
        update_recipe_draft(
            &pool,
            draft.id,
            user.id,
            draft.revision,
            &snapshot("V2", "public"),
        )
        .await
        .unwrap();

        let result = publish_recipe_draft(&pool, draft.id, user.id, Some("Published update"))
            .await
            .unwrap();
        assert_eq!(result.title, "V2");
        let versions = get_recipe_versions(&pool, published.id).await.unwrap();
        assert_eq!(
            versions
                .iter()
                .map(|v| v.version_number)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
        assert_eq!(
            versions[0].snapshot,
            snapshot_to_json(&snapshot("V2", "public")).unwrap()
        );
        assert_eq!(
            versions[1].snapshot,
            snapshot_to_json(&snapshot("V1", "private")).unwrap()
        );
    }

    #[tokio::test]
    async fn stale_base_rejects_publish_and_preserves_draft_and_published_recipe() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "stale").await;
        let published = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let stale = create_edit_draft(&pool, published.id, user.id)
            .await
            .unwrap();
        sqlx::query("UPDATE recipe_drafts SET base_version_number = 99 WHERE id = $1")
            .bind(stale.id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(matches!(
            publish_recipe_draft(&pool, stale.id, user.id, None).await,
            Err(DbError::VersionConflict {
                expected: 99,
                actual: 1,
                ..
            })
        ));
        assert_eq!(
            get_recipe_draft(&pool, stale.id, user.id).await.unwrap().id,
            stale.id
        );
        assert_eq!(
            crate::db::get_recipe_by_id(&pool, published.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "V1"
        );
        assert_eq!(
            get_recipe_versions(&pool, published.id)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn restore_publishes_target_snapshot_as_new_version_without_rewriting_history() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "restore").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            1,
            &snapshot("V2", "public"),
            None,
        )
        .await
        .unwrap();

        let restored = restore_recipe_version(&pool, recipe.id, user.id, 1, 2)
            .await
            .unwrap();
        assert_eq!(restored.title, "V1");
        let versions = get_recipe_versions(&pool, recipe.id).await.unwrap();
        assert_eq!(
            versions
                .iter()
                .map(|v| v.version_number)
                .collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert_eq!(versions[0].snapshot, versions[2].snapshot);
        assert_eq!(versions[0].notes.as_deref(), Some("Restored from v1"));
    }

    #[tokio::test]
    async fn compares_any_two_immutable_recipe_versions_in_requested_direction() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "compare_versions").await;
        let recipe =
            create_published_recipe(&pool, user.id, &snapshot("Original title", "private"), None)
                .await
                .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            1,
            &snapshot("Middle title", "private"),
            None,
        )
        .await
        .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            2,
            &snapshot("Current title", "public"),
            None,
        )
        .await
        .unwrap();

        let forward = compare_recipe_versions(&pool, recipe.id, 1, 3)
            .await
            .unwrap();
        let forward_title = forward
            .iter()
            .find(|change| change.path == "/title")
            .expect("forward title change");
        assert_eq!(forward_title.old_value.as_ref().unwrap(), "Original title");
        assert_eq!(forward_title.value.as_ref().unwrap(), "Current title");

        let reverse = compare_recipe_versions(&pool, recipe.id, 3, 1)
            .await
            .unwrap();
        let reverse_title = reverse
            .iter()
            .find(|change| change.path == "/title")
            .expect("reverse title change");
        assert_eq!(reverse_title.old_value.as_ref().unwrap(), "Current title");
        assert_eq!(reverse_title.value.as_ref().unwrap(), "Original title");
    }

    #[tokio::test]
    async fn fork_policy_and_lineage_preserve_attribution_after_source_delete() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "source").await;
        let forker = owner(&pool, "forker").await;
        let source =
            create_published_recipe(&pool, source_owner.id, &snapshot("Source", "public"), None)
                .await
                .unwrap();
        let draft = create_fork_draft(&pool, source.id, forker.id, Some("My variation"))
            .await
            .unwrap();
        let fork = publish_recipe_draft(&pool, draft.id, forker.id, None)
            .await
            .unwrap();
        let relationship = get_fork_relationship(&pool, fork.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(relationship.original_recipe_id, Some(source.id));
        assert_eq!(relationship.original_title, "Source");
        assert_eq!(relationship.forked_version_number, Some(1));

        crate::db::delete_recipe(&pool, source.id, source_owner.id)
            .await
            .unwrap();
        let relationship = get_fork_relationship(&pool, fork.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(relationship.original_recipe_id, None);
        assert_eq!(relationship.original_title, "Source");
    }

    #[tokio::test]
    async fn another_users_private_recipe_cannot_be_forked() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let source_owner = owner(&pool, "private_source").await;
        let forker = owner(&pool, "private_forker").await;
        let source = create_published_recipe(
            &pool,
            source_owner.id,
            &snapshot("Private", "private"),
            None,
        )
        .await
        .unwrap();

        assert!(matches!(
            create_fork_draft(&pool, source.id, forker.id, None).await,
            Err(DbError::ForkError(_))
        ));
    }

    #[tokio::test]
    async fn draft_schema_allows_multiple_new_drafts_but_reuses_existing_recipe_draft() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "draft_index").await;
        let first_new = create_new_recipe_draft(&pool, user.id, &snapshot("One", "private"))
            .await
            .unwrap();
        let second_new = create_new_recipe_draft(&pool, user.id, &snapshot("Two", "private"))
            .await
            .unwrap();
        assert_ne!(first_new.id, second_new.id);

        let recipe =
            create_published_recipe(&pool, user.id, &snapshot("Published", "private"), None)
                .await
                .unwrap();
        let first_edit = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        let second_edit = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        assert_eq!(first_edit.id, second_edit.id);
    }

    #[tokio::test]
    async fn publish_rolls_back_version_recipe_and_tags_when_a_late_write_fails() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "rollback").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        let mut invalid_tag_snapshot = snapshot("Must roll back", "public");
        invalid_tag_snapshot.tags = vec!["x".repeat(101)];
        update_recipe_draft(&pool, draft.id, user.id, 0, &invalid_tag_snapshot)
            .await
            .unwrap();

        assert!(publish_recipe_draft(&pool, draft.id, user.id, None)
            .await
            .is_err());
        assert_eq!(
            get_recipe_versions(&pool, recipe.id).await.unwrap().len(),
            1
        );
        assert_eq!(
            crate::db::get_recipe_by_id(&pool, recipe.id)
                .await
                .unwrap()
                .unwrap()
                .title,
            "V1"
        );
        assert!(get_recipe_draft(&pool, draft.id, user.id).await.is_ok());
    }

    #[tokio::test]
    async fn concurrent_publish_of_one_draft_has_one_winner_and_one_consistent_latest_version() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "concurrent").await;
        let recipe = create_published_recipe(&pool, user.id, &snapshot("V1", "private"), None)
            .await
            .unwrap();
        let draft = create_edit_draft(&pool, recipe.id, user.id).await.unwrap();
        update_recipe_draft(&pool, draft.id, user.id, 0, &snapshot("V2", "public"))
            .await
            .unwrap();

        let first_pool = pool.clone();
        let second_pool = pool.clone();
        let first = tokio::spawn(async move {
            publish_recipe_draft(&first_pool, draft.id, user.id, None).await
        });
        let second = tokio::spawn(async move {
            publish_recipe_draft(&second_pool, draft.id, user.id, None).await
        });
        let results = [first.await.unwrap(), second.await.unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            get_recipe_versions(&pool, recipe.id).await.unwrap().len(),
            2
        );
        assert_eq!(
            get_latest_recipe_version(&pool, recipe.id)
                .await
                .unwrap()
                .version_number,
            2
        );
    }

    #[tokio::test]
    async fn orphan_reference_scan_keeps_draft_and_historical_version_images() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let user = owner(&pool, "image_refs").await;
        let mut v1 = snapshot("Images", "private");
        v1.images.push(crate::types::RecipeImage::new(
            "https://cdn.test/historical_full.webp".into(),
            Some("https://cdn.test/historical_thumb.webp".into()),
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        let recipe = create_published_recipe(&pool, user.id, &v1, None)
            .await
            .unwrap();
        publish_snapshot(
            &pool,
            recipe.id,
            user.id,
            1,
            &snapshot("V2", "private"),
            None,
        )
        .await
        .unwrap();
        let mut draft_snapshot = snapshot("Draft image", "private");
        draft_snapshot.images.push(crate::types::RecipeImage::new(
            "https://cdn.test/draft_full.webp".into(),
            None,
            crate::types::RecipeImageEdit::default_rect_16x9(),
        ));
        create_new_recipe_draft(&pool, user.id, &draft_snapshot)
            .await
            .unwrap();

        let urls = crate::db::get_all_image_urls(&pool).await.unwrap();
        assert!(urls.contains(&"https://cdn.test/historical_full.webp".to_string()));
        assert!(urls.contains(&"https://cdn.test/historical_thumb.webp".to_string()));
        assert!(urls.contains(&"https://cdn.test/draft_full.webp".to_string()));
    }
}
