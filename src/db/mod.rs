//! Database layer: connection pool, types, and query functions.
//!
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]
// Public API items are used by other modules at runtime, but the compiler
// can't see that from this crate's binary target alone.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, Row};
use std::collections::HashSet;
use uuid::Uuid;

pub mod diff;
mod versioning;

#[allow(unused_imports)]
pub use versioning::*;

use crate::types::{
    Collection, CollectionContents, CollectionRecipeList, CollectionSharePreview,
    CollectionSummary, CollectionVisibility, PublicCollectionListResponse,
};

mod recipe_search;
pub use recipe_search::search_recipes;
mod user_search;
pub use user_search::search_users;
mod collection_search;
pub use collection_search::search_collections;
mod all_search;
pub use all_search::search_all;
mod autocomplete;
pub use autocomplete::autocomplete_entities;

// Re-export typed recipe sub-structs for use in DB layer
pub use crate::types::RecipeEquipment;
pub use crate::types::RecipeIngredient;
pub use crate::types::RecipeStep;

// ── Error type ──────────────────────────────────────────────────────────────

/// Errors from database operations.
#[derive(Debug)]
pub enum DbError {
    /// The `DATABASE_URL` environment variable is not set.
    MissingUrl,
    /// Failed to connect to the database.
    Connection(sqlx::Error),
    /// A query failed.
    Query(sqlx::Error),
    /// The requested username is already taken by another user.
    UsernameTaken,
    /// The session was not found, revoked, or expired.
    SessionInvalid,
    /// The requested recipe was not found.
    RecipeNotFound(uuid::Uuid),
    /// The requested immutable published version was not found.
    VersionNotFound {
        recipe_id: Uuid,
        version_number: i32,
    },
    /// A publish/restore was based on a stale published version.
    VersionConflict {
        recipe_id: Uuid,
        expected: i32,
        actual: i32,
    },
    /// The requested owner-gated draft was not found.
    DraftNotFound(Uuid),
    /// A draft autosave used a stale revision counter.
    DraftConflict {
        draft_id: Uuid,
        expected: i64,
        actual: i64,
    },
    /// A recipe cannot be forked under the current access policy.
    ForkError(String),
    /// A snapshot did not satisfy publication validation.
    InvalidRecipeSnapshot(String),
    /// Failed to serialize or deserialize JSONB data.
    SerdeJson(serde_json::Error),
    /// The requested collection was not found for the current owner.
    CollectionNotFound(Uuid),
    /// The requested parent collection was not found for the current owner.
    CollectionParentNotFound(Uuid),
    /// A sibling collection already has this name, ignoring case.
    CollectionNameTaken,
    /// A collection cannot be moved into itself or a descendant.
    CollectionCycle,
    /// The recipe is not accessible to the collection owner.
    RecipeNotAccessible(Uuid),
    /// The recipe is already a member of the collection.
    RecipeAlreadyInCollection,
    /// The submitted order does not exactly match the current direct members.
    InvalidCollectionOrder,
    /// A search request failed bounded server-side validation.
    InvalidSearchRequest(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::MissingUrl => write!(f, "DATABASE_URL not set"),
            DbError::Connection(e) => write!(f, "database connection failed: {e}"),
            DbError::Query(e) => write!(f, "database query failed: {e}"),
            DbError::UsernameTaken => write!(f, "username is already taken"),
            DbError::SessionInvalid => write!(f, "session is invalid, revoked, or expired"),
            DbError::RecipeNotFound(id) => write!(f, "recipe not found: {id}"),
            DbError::VersionNotFound {
                recipe_id,
                version_number,
            } => write!(f, "recipe version not found: {recipe_id} v{version_number}"),
            DbError::VersionConflict {
                recipe_id,
                expected,
                actual,
            } => write!(
                f,
                "recipe version conflict for {recipe_id}: expected v{expected}, current v{actual}"
            ),
            DbError::DraftNotFound(id) => write!(f, "recipe draft not found: {id}"),
            DbError::DraftConflict {
                draft_id,
                expected,
                actual,
            } => write!(
                f,
                "recipe draft conflict for {draft_id}: expected revision {expected}, current revision {actual}"
            ),
            DbError::ForkError(message) => write!(f, "recipe fork failed: {message}"),
            DbError::InvalidRecipeSnapshot(message) => {
                write!(f, "invalid recipe snapshot: {message}")
            }
            DbError::SerdeJson(e) => write!(f, "JSON serialization/deserialization failed: {e}"),
            DbError::CollectionNotFound(id) => write!(f, "collection not found: {id}"),
            DbError::CollectionParentNotFound(id) => write!(f, "collection parent not found: {id}"),
            DbError::CollectionNameTaken => {
                write!(f, "collection name is already used by a sibling")
            }
            DbError::CollectionCycle => write!(f, "collection move would create a cycle"),
            DbError::RecipeNotAccessible(id) => write!(f, "recipe is not accessible: {id}"),
            DbError::RecipeAlreadyInCollection => write!(f, "recipe is already in the collection"),
            DbError::InvalidCollectionOrder => {
                write!(f, "collection order does not match membership")
            }
            DbError::InvalidSearchRequest(message) => {
                write!(f, "invalid search request: {message}")
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::MissingUrl => None,
            DbError::Connection(e) | DbError::Query(e) => Some(e),
            DbError::UsernameTaken => None,
            DbError::SessionInvalid => None,
            DbError::RecipeNotFound(_) => None,
            DbError::VersionNotFound { .. }
            | DbError::VersionConflict { .. }
            | DbError::DraftNotFound(_)
            | DbError::DraftConflict { .. }
            | DbError::ForkError(_)
            | DbError::InvalidRecipeSnapshot(_) => None,
            DbError::SerdeJson(e) => Some(e),
            DbError::CollectionNotFound(_)
            | DbError::CollectionParentNotFound(_)
            | DbError::CollectionNameTaken
            | DbError::CollectionCycle
            | DbError::RecipeNotAccessible(_)
            | DbError::RecipeAlreadyInCollection
            | DbError::InvalidCollectionOrder => None,
            DbError::InvalidSearchRequest(_) => None,
        }
    }
}

// ── Connection pool ─────────────────────────────────────────────────────────

/// Create a new connection pool from the `DATABASE_URL` environment variable.
///
/// Validates connectivity — returns an error if the database is unreachable.
/// The caller is responsible for storing the returned pool (e.g., in Axum state).
///
/// Pool sizing defaults are tuned for a 2-CPU database:
/// - `max_connections`: 5 (formula: cores × 2 + 1; more connections increase
///   context-switching overhead without improving throughput on few cores)
/// - `min_connections`: 1 (avoids cold-start latency on first request)
/// - `idle_timeout`: 5min (releases unused connections promptly)
///
/// Override `max_connections` via the `DB_MAX_CONNECTIONS` env var if needed.
pub async fn create_pool() -> Result<PgPool, DbError> {
    let url = std::env::var("DATABASE_URL").map_err(|_| DbError::MissingUrl)?;
    let max_connections: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .connect(&url)
        .await
        .map_err(DbError::Connection)
}

/// Global pool instance, initialized lazily on first access.
///
/// Used by server functions to access the database without relying on
/// axum request extensions (which don't propagate to FullstackContext).
static POOL: tokio::sync::OnceCell<PgPool> = tokio::sync::OnceCell::const_new();

/// Initialize the global pool. Call once during application startup.
pub async fn init_pool() {
    POOL.set(create_pool().await.expect("Failed to create database pool"))
        .expect("Pool already initialized");
    eprintln!("Database pool initialized");
}

/// Get a clone of the global database pool.
///
/// Panics if the pool has not been initialized via [`init_pool`].
pub fn get_pool() -> PgPool {
    POOL.get()
        .expect("Database pool not initialized. Call db::init_pool() first.")
        .clone()
}

// ── Rust types ──────────────────────────────────────────────────────────────

/// A user of the application.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An OAuth provider account linked to a user.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OauthAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub profile_data: Option<serde_json::Value>,
    pub refresh_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

/// Short-lived state for OAuth CSRF protection.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthState {
    pub id: String,
    pub redirect_uri: String,
    pub provider: String,
    pub code_verifier: Option<String>,
    pub purpose: Option<String>,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// A server-side session row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub refreshed_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

/// Display-oriented row for listing OAuth accounts on the settings page.
/// Omits `email_verified` and `profile_data` which are not needed for display.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OauthAccountRow {
    pub id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

/// A recipe saved by a user.
///
/// Re-exported from `crate::types` for use in database queries.
pub use crate::types::{Recipe, RecipeImage};

/// Implement `sqlx::FromRow` for `Recipe` so it can be used with raw queries.
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for Recipe {
    fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            title: row.try_get("title")?,
            description: row.try_get("description")?,
            commentary: row.try_get("commentary")?,
            prep_time_minutes: row.try_get("prep_time_minutes")?,
            cook_time_minutes: row.try_get("cook_time_minutes")?,
            servings: row.try_get("servings")?,
            ingredients: serde_json::from_value(row.try_get("ingredients")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            instructions: serde_json::from_value(row.try_get("instructions")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            equipment: serde_json::from_value(row.try_get("equipment")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            images: serde_json::from_value(row.try_get("images")?)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            visibility: row.try_get("visibility")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            author_username: row.try_get("author_username")?,
            author_avatar_url: row.try_get("author_avatar_url")?,
        })
    }
}

/// A tag associated with a recipe.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecipeTag {
    pub recipe_id: Uuid,
    pub tag: String,
}

/// Paginated recipe list response for server functions.
///
/// Re-exported from `crate::types`.
#[allow(unused_imports)]
pub use crate::types::RecipeListResponse;

// ── Collection queries ─────────────────────────────────────────────────────

/// A database-only row for a recipe membership junction.
#[derive(Debug, Clone)]
pub struct CollectionRecipe {
    pub collection_id: Uuid,
    pub recipe_id: Uuid,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

fn collection_visibility_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<CollectionVisibility, sqlx::Error> {
    let visibility: String = row.try_get("visibility")?;
    match visibility.as_str() {
        "private" => Ok(CollectionVisibility::Private),
        "unlisted" => Ok(CollectionVisibility::Unlisted),
        "public" => Ok(CollectionVisibility::Public),
        invalid => Err(sqlx::Error::Decode(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid collection visibility: {invalid}"),
        )))),
    }
}

fn collection_from_row(row: &sqlx::postgres::PgRow) -> Result<Collection, sqlx::Error> {
    let image = row
        .try_get::<Option<serde_json::Value>, _>("image")?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    Ok(Collection {
        id: row.try_get("id")?,
        owner_id: row.try_get("owner_id")?,
        parent_collection_id: row.try_get("parent_collection_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        image,
        visibility: collection_visibility_from_row(row)?,
        sort_order: row.try_get("sort_order")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn collection_summary_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<CollectionSummary, sqlx::Error> {
    let image = row
        .try_get::<Option<serde_json::Value>, _>("image")?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    // Recipe image JSON predates collections and is not schema-constrained.
    // A malformed legacy entry must not make the entire collection unreadable.
    let preview_images = row
        .try_get::<serde_json::Value, _>("preview_images")?
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|image| serde_json::from_value(image.clone()).ok())
        .take(4)
        .collect();
    Ok(CollectionSummary {
        id: row.try_get("id")?,
        parent_collection_id: row.try_get("parent_collection_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        image,
        preview_images,
        visibility: collection_visibility_from_row(row)?,
        sort_order: row.try_get("sort_order")?,
        child_count: row.try_get("child_count")?,
        recipe_count: row.try_get("recipe_count")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn collection_query_error(error: sqlx::Error) -> DbError {
    if error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .as_deref()
        == Some("23505")
    {
        DbError::CollectionNameTaken
    } else {
        DbError::Query(error)
    }
}

fn membership_query_error(error: sqlx::Error) -> DbError {
    if error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .as_deref()
        == Some("23505")
    {
        DbError::RecipeAlreadyInCollection
    } else {
        DbError::Query(error)
    }
}

fn collection_preview_images_select(recipe_visibility_predicate: &str) -> String {
    format!(
        "COALESCE((
             SELECT jsonb_agg(preview.image ORDER BY preview.sort_order, preview.created_at, preview.recipe_id)
             FROM (
                 SELECT r.images -> 0 AS image, cr.sort_order, cr.created_at, cr.recipe_id
                 FROM collection_recipes cr
                 INNER JOIN recipes r ON r.id = cr.recipe_id
                 WHERE cr.collection_id = c.id
                   AND {recipe_visibility_predicate}
                   AND jsonb_typeof(r.images) = 'array'
                   AND r.images <> '[]'::jsonb
                   AND (
                       jsonb_typeof(r.images -> 0) = 'string'
                       OR (
                           jsonb_typeof(r.images -> 0) = 'object'
                           AND jsonb_typeof(r.images -> 0 -> 'url') = 'string'
                       )
                   )
                 ORDER BY cr.sort_order, cr.created_at, cr.recipe_id
                 LIMIT 4
             ) preview
         ), '[]'::jsonb)"
    )
}

fn collection_summary_select() -> String {
    format!(
        "SELECT c.id, c.parent_collection_id, c.name, c.description, c.image, c.visibility,
            c.sort_order, c.updated_at,
            {} AS preview_images,
            (SELECT COUNT(*) FROM collections child
             WHERE child.owner_id = c.owner_id AND child.parent_collection_id = c.id) AS child_count,
            (SELECT COUNT(*)
             FROM collection_recipes cr
             INNER JOIN recipes r ON r.id = cr.recipe_id
             WHERE cr.collection_id = c.id
               AND (r.user_id = c.owner_id OR r.visibility IN ('public', 'unlisted'))) AS recipe_count
     FROM collections c",
        collection_preview_images_select(
            "(r.user_id = c.owner_id OR r.visibility IN ('public', 'unlisted'))"
        )
    )
}

async fn lock_owner(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    owner_id: Uuid,
) -> Result<(), DbError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(DbError::Query)?;
    if found.is_none() {
        return Err(DbError::Query(sqlx::Error::RowNotFound));
    }
    Ok(())
}

/// Insert a root or nested collection with explicit visibility.
pub async fn insert_collection(
    pool: &PgPool,
    owner_id: Uuid,
    parent_collection_id: Option<Uuid>,
    name: &str,
    description: Option<&str>,
    visibility: CollectionVisibility,
    image: Option<&RecipeImage>,
) -> Result<Collection, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    lock_owner(&mut tx, owner_id).await?;

    if let Some(parent_id) = parent_collection_id {
        let parent_exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM collections
             WHERE id = $1 AND owner_id = $2
             FOR UPDATE",
        )
        .bind(parent_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DbError::Query)?;
        if parent_exists.is_none() {
            return Err(DbError::CollectionParentNotFound(parent_id));
        }
    }

    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1
         FROM collections
         WHERE owner_id = $1 AND parent_collection_id IS NOT DISTINCT FROM $2",
    )
    .bind(owner_id)
    .bind(parent_collection_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    let row = sqlx::query(
        "INSERT INTO collections
            (owner_id, parent_collection_id, name, description, sort_order, visibility, image)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, owner_id, parent_collection_id, name, description, image,
                   visibility, sort_order, created_at, updated_at",
    )
    .bind(owner_id)
    .bind(parent_collection_id)
    .bind(name)
    .bind(description)
    .bind(next_order)
    .bind(visibility.as_str())
    .bind(
        image
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| DbError::Query(sqlx::Error::Encode(Box::new(error))))?,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(collection_query_error)?;

    let collection = collection_from_row(&row).map_err(DbError::Query)?;
    tx.commit().await.map_err(DbError::Query)?;
    Ok(collection)
}

/// Get a collection only when it belongs to the supplied owner.
pub async fn get_collection_by_id_and_owner(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    collection_id: Uuid,
    owner_id: Uuid,
) -> Result<Option<Collection>, DbError> {
    sqlx::query(
        "SELECT id, owner_id, parent_collection_id, name, description, image,
                visibility, sort_order, created_at, updated_at
         FROM collections
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?
    .map(|row| collection_from_row(&row))
    .transpose()
    .map_err(DbError::Query)
}

/// List an owner's root collections in normalized sibling order.
pub async fn get_root_collections(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    owner_id: Uuid,
) -> Result<Vec<CollectionSummary>, DbError> {
    let query = format!(
        "{} WHERE c.owner_id = $1 AND c.parent_collection_id IS NULL
         ORDER BY c.sort_order, lower(c.name), c.id",
        collection_summary_select()
    );
    let rows = sqlx::query(&query)
        .bind(owner_id)
        .fetch_all(executor)
        .await
        .map_err(DbError::Query)?;
    rows.iter()
        .map(collection_summary_from_row)
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// List direct children only when the parent belongs to the owner.
pub async fn get_child_collections(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    parent_collection_id: Uuid,
    owner_id: Uuid,
) -> Result<Vec<CollectionSummary>, DbError> {
    let query = format!(
        "{} WHERE c.owner_id = $1 AND c.parent_collection_id = $2
         ORDER BY c.sort_order, lower(c.name), c.id",
        collection_summary_select()
    );
    let rows = sqlx::query(&query)
        .bind(owner_id)
        .bind(parent_collection_id)
        .fetch_all(executor)
        .await
        .map_err(DbError::Query)?;
    rows.iter()
        .map(collection_summary_from_row)
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// List every collection summary owned by a user.
pub async fn get_all_collections_by_owner(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    owner_id: Uuid,
) -> Result<Vec<CollectionSummary>, DbError> {
    let query = format!(
        "{} WHERE c.owner_id = $1
         ORDER BY c.parent_collection_id NULLS FIRST, c.sort_order, lower(c.name), c.id",
        collection_summary_select()
    );
    let rows = sqlx::query(&query)
        .bind(owner_id)
        .fetch_all(executor)
        .await
        .map_err(DbError::Query)?;
    rows.iter()
        .map(collection_summary_from_row)
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// List public-safe collection summaries for a public user profile.
pub async fn get_public_collections_by_username(
    pool: &PgPool,
    username: &str,
    offset: i64,
    limit: i64,
) -> Result<PublicCollectionListResponse, DbError> {
    let offset = offset.max(0);
    let limit = limit.max(0);
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;

    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    let Some(owner_id) = owner_id else {
        tx.commit().await.map_err(DbError::Query)?;
        return Ok(PublicCollectionListResponse {
            collections: Vec::new(),
            total_count: 0,
            has_more: false,
        });
    };

    let total_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM collections
         WHERE owner_id = $1 AND visibility = 'public'",
    )
    .bind(owner_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    let query = format!(
        "SELECT c.id,
                CASE
                    WHEN parent.owner_id = c.owner_id AND parent.visibility = 'public'
                    THEN c.parent_collection_id
                    ELSE NULL
                END AS parent_collection_id,
                c.name, c.description, c.image, c.visibility,
                ((ROW_NUMBER() OVER (ORDER BY c.updated_at DESC, c.id) - 1)::INTEGER)
                    AS sort_order,
                c.updated_at,
                {} AS preview_images,
                (SELECT COUNT(*)
                 FROM collections child
                 WHERE child.owner_id = c.owner_id
                   AND child.parent_collection_id = c.id
                   AND child.visibility = 'public') AS child_count,
                (SELECT COUNT(*)
                 FROM collection_recipes cr
                 INNER JOIN recipes r ON r.id = cr.recipe_id
                 WHERE cr.collection_id = c.id
                   AND r.visibility = 'public') AS recipe_count
         FROM collections c
         LEFT JOIN collections parent ON parent.id = c.parent_collection_id
         WHERE c.owner_id = $1 AND c.visibility = 'public'
         ORDER BY c.updated_at DESC, c.id
         LIMIT $2 OFFSET $3",
        collection_preview_images_select("r.visibility = 'public'")
    );
    let rows = sqlx::query(&query)
        .bind(owner_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    let collections = rows
        .iter()
        .map(collection_summary_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::Query)?;
    let has_more = offset.saturating_add(collections.len() as i64) < total_count;

    tx.commit().await.map_err(DbError::Query)?;
    Ok(PublicCollectionListResponse {
        collections,
        total_count,
        has_more,
    })
}

/// Return an owner-checked root-to-leaf breadcrumb path using a recursive CTE.
pub async fn get_collection_breadcrumbs(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    collection_id: Uuid,
    owner_id: Uuid,
) -> Result<Vec<CollectionSummary>, DbError> {
    let query = format!(
        "WITH RECURSIVE ancestors AS (
             SELECT c.id, c.parent_collection_id, ARRAY[c.id] AS path, 0 AS depth
             FROM collections c
             WHERE c.id = $1 AND c.owner_id = $2
             UNION ALL
             SELECT parent.id, parent.parent_collection_id,
                    ancestors.path || parent.id, ancestors.depth + 1
             FROM collections parent
             INNER JOIN ancestors ON parent.id = ancestors.parent_collection_id
             WHERE parent.owner_id = $2
               AND NOT parent.id = ANY(ancestors.path)
         )
         SELECT c.id, c.parent_collection_id, c.name, c.description, c.image, c.visibility,
                c.sort_order, c.updated_at,
                {} AS preview_images,
                (SELECT COUNT(*) FROM collections child
                 WHERE child.owner_id = c.owner_id AND child.parent_collection_id = c.id) AS child_count,
                (SELECT COUNT(*) FROM collection_recipes cr
                 INNER JOIN recipes r ON r.id = cr.recipe_id
                 WHERE cr.collection_id = c.id
                   AND (r.user_id = c.owner_id OR r.visibility IN ('public', 'unlisted'))) AS recipe_count
         FROM ancestors
         INNER JOIN collections c ON c.id = ancestors.id
         ORDER BY ancestors.depth DESC",
        collection_preview_images_select(
            "(r.user_id = c.owner_id OR r.visibility IN ('public', 'unlisted'))"
        )
    );
    let rows = sqlx::query(&query)
        .bind(collection_id)
        .bind(owner_id)
        .fetch_all(executor)
        .await
        .map_err(DbError::Query)?;
    rows.iter()
        .map(collection_summary_from_row)
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// Return all strict descendants of an owner-scoped collection using a recursive CTE.
pub async fn get_collection_descendants(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    collection_id: Uuid,
    owner_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    sqlx::query_scalar(
        "WITH RECURSIVE descendants AS (
             SELECT c.id, ARRAY[c.id] AS path
             FROM collections c
             WHERE c.parent_collection_id = $1 AND c.owner_id = $2
             UNION ALL
             SELECT child.id, descendants.path || child.id
             FROM collections child
             INNER JOIN descendants ON child.parent_collection_id = descendants.id
             WHERE child.owner_id = $2
               AND NOT child.id = ANY(descendants.path)
         )
         SELECT id FROM descendants ORDER BY path",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Rename or update the description of an owner-scoped collection.
pub async fn update_collection(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    name: &str,
    description: Option<&str>,
    image: Option<&RecipeImage>,
) -> Result<Collection, DbError> {
    let row = sqlx::query(
        "UPDATE collections
         SET name = $3, description = $4, image = $5, updated_at = NOW()
         WHERE id = $1 AND owner_id = $2
         RETURNING id, owner_id, parent_collection_id, name, description, image,
                   visibility, sort_order, created_at, updated_at",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(name)
    .bind(description)
    .bind(
        image
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| DbError::Query(sqlx::Error::Encode(Box::new(error))))?,
    )
    .fetch_one(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => DbError::CollectionNotFound(collection_id),
        other => collection_query_error(other),
    })?;
    collection_from_row(&row).map_err(DbError::Query)
}

/// Change visibility only when the collection belongs to the supplied owner.
pub async fn update_collection_visibility(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    visibility: CollectionVisibility,
) -> Result<Collection, DbError> {
    let row = sqlx::query(
        "UPDATE collections
         SET visibility = $3,
             updated_at = GREATEST(NOW(), updated_at + INTERVAL '1 microsecond')
         WHERE id = $1 AND owner_id = $2
         RETURNING id, owner_id, parent_collection_id, name, description, image,
                   visibility, sort_order, created_at, updated_at",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(visibility.as_str())
    .fetch_one(pool)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => DbError::CollectionNotFound(collection_id),
        other => DbError::Query(other),
    })?;
    collection_from_row(&row).map_err(DbError::Query)
}

async fn lock_siblings(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    owner_id: Uuid,
    parent_collection_id: Option<Uuid>,
) -> Result<(), DbError> {
    sqlx::query(
        "SELECT id FROM collections
         WHERE owner_id = $1 AND parent_collection_id IS NOT DISTINCT FROM $2
         ORDER BY id
         FOR UPDATE",
    )
    .bind(owner_id)
    .bind(parent_collection_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

async fn normalize_sibling_order(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    owner_id: Uuid,
    parent_collection_id: Option<Uuid>,
) -> Result<(), DbError> {
    sqlx::query(
        "WITH ordered AS (
             SELECT id,
                    ROW_NUMBER() OVER (ORDER BY sort_order, lower(name), id) - 1 AS new_order
             FROM collections
             WHERE owner_id = $1 AND parent_collection_id IS NOT DISTINCT FROM $2
         )
         UPDATE collections c
         SET sort_order = ordered.new_order
         FROM ordered
         WHERE c.id = ordered.id",
    )
    .bind(owner_id)
    .bind(parent_collection_id)
    .execute(&mut **tx)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

async fn normalize_recipe_order(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    collection_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query(
        "WITH ordered AS (
             SELECT recipe_id,
                    ROW_NUMBER() OVER (ORDER BY sort_order, created_at, recipe_id) - 1 AS new_order
             FROM collection_recipes
             WHERE collection_id = $1
         )
         UPDATE collection_recipes cr
         SET sort_order = ordered.new_order
         FROM ordered
         WHERE cr.collection_id = $1 AND cr.recipe_id = ordered.recipe_id",
    )
    .bind(collection_id)
    .execute(&mut **tx)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

/// Move a collection to another owned parent or to the root, rejecting cycles.
pub async fn move_collection(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    new_parent_collection_id: Option<Uuid>,
) -> Result<Collection, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    lock_owner(&mut tx, owner_id).await?;

    let current = sqlx::query(
        "SELECT parent_collection_id FROM collections
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let current_parent: Option<Uuid> = current
        .as_ref()
        .map(|row| row.try_get("parent_collection_id"))
        .transpose()
        .map_err(DbError::Query)?;
    if current.is_none() {
        return Err(DbError::CollectionNotFound(collection_id));
    }

    let mut ids_to_lock = vec![collection_id];
    if let Some(parent_id) = current_parent {
        ids_to_lock.push(parent_id);
    }
    if let Some(parent_id) = new_parent_collection_id {
        ids_to_lock.push(parent_id);
    }
    ids_to_lock.sort();
    ids_to_lock.dedup();
    sqlx::query(
        "SELECT id FROM collections
         WHERE owner_id = $1 AND id = ANY($2)
         ORDER BY id
         FOR UPDATE",
    )
    .bind(owner_id)
    .bind(&ids_to_lock)
    .fetch_all(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    if let Some(parent_id) = new_parent_collection_id {
        let parent_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM collections WHERE id = $1 AND owner_id = $2")
                .bind(parent_id)
                .bind(owner_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(DbError::Query)?;
        if parent_exists.is_none() {
            return Err(DbError::CollectionParentNotFound(parent_id));
        }

        if parent_id == collection_id
            || get_collection_descendants(&mut *tx, collection_id, owner_id)
                .await?
                .contains(&parent_id)
        {
            return Err(DbError::CollectionCycle);
        }
    }

    let mut parents_to_lock: Vec<Uuid> = [current_parent, new_parent_collection_id]
        .into_iter()
        .flatten()
        .collect();
    parents_to_lock.sort();
    parents_to_lock.dedup();
    for parent_id in parents_to_lock {
        lock_siblings(&mut tx, owner_id, Some(parent_id)).await?;
    }
    if current_parent.is_none() || new_parent_collection_id.is_none() {
        lock_siblings(&mut tx, owner_id, None).await?;
    }

    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1
         FROM collections
         WHERE owner_id = $1 AND parent_collection_id IS NOT DISTINCT FROM $2",
    )
    .bind(owner_id)
    .bind(new_parent_collection_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    sqlx::query(
        "UPDATE collections
         SET parent_collection_id = $3, sort_order = $4, updated_at = NOW()
         WHERE id = $1 AND owner_id = $2",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(new_parent_collection_id)
    .bind(next_order)
    .execute(&mut *tx)
    .await
    .map_err(collection_query_error)?;

    if let Some(old_parent) = current_parent {
        normalize_sibling_order(&mut tx, owner_id, Some(old_parent)).await?;
        sqlx::query("UPDATE collections SET updated_at = NOW() WHERE id = $1 AND owner_id = $2")
            .bind(old_parent)
            .bind(owner_id)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    }
    if new_parent_collection_id != current_parent {
        normalize_sibling_order(&mut tx, owner_id, new_parent_collection_id).await?;
    }
    if let Some(new_parent) = new_parent_collection_id {
        sqlx::query("UPDATE collections SET updated_at = NOW() WHERE id = $1 AND owner_id = $2")
            .bind(new_parent)
            .bind(owner_id)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    }

    let row = sqlx::query(
        "SELECT id, owner_id, parent_collection_id, name, description, image,
                visibility, sort_order, created_at, updated_at
         FROM collections WHERE id = $1 AND owner_id = $2",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let moved = collection_from_row(&row).map_err(DbError::Query)?;
    tx.commit().await.map_err(DbError::Query)?;
    Ok(moved)
}

/// Delete an owner-scoped collection; descendants and memberships cascade in the database.
pub async fn delete_collection(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
) -> Result<(), DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    lock_owner(&mut tx, owner_id).await?;
    let rows = sqlx::query("DELETE FROM collections WHERE id = $1 AND owner_id = $2")
        .bind(collection_id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    if rows == 0 {
        return Err(DbError::CollectionNotFound(collection_id));
    }
    tx.commit().await.map_err(DbError::Query)
}

/// Add an accessible published recipe to an owned collection, appending atomically.
pub async fn add_recipe_to_collection(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    recipe_id: Uuid,
) -> Result<CollectionRecipe, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    let collection_exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM collections WHERE id = $1 AND owner_id = $2 FOR UPDATE")
            .bind(collection_id)
            .bind(owner_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    if collection_exists.is_none() {
        return Err(DbError::CollectionNotFound(collection_id));
    }

    let recipe_exists: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM recipes
         WHERE id = $1 AND (user_id = $2 OR visibility IN ('public', 'unlisted'))",
    )
    .bind(recipe_id)
    .bind(owner_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    if recipe_exists.is_none() {
        return Err(DbError::RecipeNotAccessible(recipe_id));
    }

    let next_order: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sort_order), -1) + 1
         FROM collection_recipes WHERE collection_id = $1",
    )
    .bind(collection_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let row = sqlx::query(
        "INSERT INTO collection_recipes (collection_id, recipe_id, sort_order)
         VALUES ($1, $2, $3)
         RETURNING collection_id, recipe_id, sort_order, created_at",
    )
    .bind(collection_id)
    .bind(recipe_id)
    .bind(next_order)
    .fetch_one(&mut *tx)
    .await
    .map_err(membership_query_error)?;
    sqlx::query("UPDATE collections SET updated_at = NOW() WHERE id = $1")
        .bind(collection_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;

    let membership = CollectionRecipe {
        collection_id: row.try_get("collection_id").map_err(DbError::Query)?,
        recipe_id: row.try_get("recipe_id").map_err(DbError::Query)?,
        sort_order: row.try_get("sort_order").map_err(DbError::Query)?,
        created_at: row.try_get("created_at").map_err(DbError::Query)?,
    };
    tx.commit().await.map_err(DbError::Query)?;
    Ok(membership)
}

/// Remove a recipe membership from an owner-scoped collection.
pub async fn remove_recipe_from_collection(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    recipe_id: Uuid,
) -> Result<(), DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    let collection_exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM collections WHERE id = $1 AND owner_id = $2 FOR UPDATE")
            .bind(collection_id)
            .bind(owner_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    if collection_exists.is_none() {
        return Err(DbError::CollectionNotFound(collection_id));
    }
    let removed =
        sqlx::query("DELETE FROM collection_recipes WHERE collection_id = $1 AND recipe_id = $2")
            .bind(collection_id)
            .bind(recipe_id)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?
            .rows_affected();
    if removed > 0 {
        normalize_recipe_order(&mut tx, collection_id).await?;
        sqlx::query("UPDATE collections SET updated_at = NOW() WHERE id = $1")
            .bind(collection_id)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    }
    tx.commit().await.map_err(DbError::Query)
}

/// List direct visible recipe members of an owner-scoped collection.
pub async fn get_collection_recipes(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    offset: i64,
    limit: i64,
) -> Result<CollectionRecipeList, DbError> {
    if get_collection_by_id_and_owner(pool, collection_id, owner_id)
        .await?
        .is_none()
    {
        return Err(DbError::CollectionNotFound(collection_id));
    }
    let total_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM collection_recipes cr
         INNER JOIN collections c ON c.id = cr.collection_id AND c.owner_id = $2
         INNER JOIN recipes r ON r.id = cr.recipe_id
         WHERE cr.collection_id = $1
           AND (r.user_id = $2 OR r.visibility IN ('public', 'unlisted'))",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Query)?;
    let rows = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary,
                r.prep_time_minutes, r.cook_time_minutes, r.servings,
                r.ingredients, r.instructions, r.equipment, r.images,
                r.visibility, r.created_at, r.updated_at,
                u.username AS author_username, u.avatar_url AS author_avatar_url
         FROM collection_recipes cr
         INNER JOIN collections c ON c.id = cr.collection_id AND c.owner_id = $2
         INNER JOIN recipes r ON r.id = cr.recipe_id
         LEFT JOIN users u ON u.id = r.user_id
         WHERE cr.collection_id = $1
           AND (r.user_id = $2 OR r.visibility IN ('public', 'unlisted'))
         ORDER BY cr.sort_order, cr.created_at, cr.recipe_id
         LIMIT $3 OFFSET $4",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(limit.max(0))
    .bind(offset.max(0))
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    let recipes = rows
        .iter()
        .map(Recipe::from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::Query)?;
    Ok(CollectionRecipeList {
        has_more: offset.max(0) + (recipes.len() as i64) < total_count,
        recipes,
        total_count,
    })
}

/// Preview how many owner-visible memberships remain visible at a target level.
pub async fn get_collection_share_preview(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    target_visibility: CollectionVisibility,
) -> Result<CollectionSharePreview, DbError> {
    let row = sqlx::query(
        "SELECT
             COUNT(r.id) FILTER (
                 WHERE r.user_id = c.owner_id
                    OR r.visibility IN ('public', 'unlisted')
             ) AS total_recipe_count,
             COUNT(r.id) FILTER (
                 WHERE (r.user_id = c.owner_id
                        OR r.visibility IN ('public', 'unlisted'))
                   AND ($3 = 'private'
                        OR r.visibility = 'public'
                        OR ($3 = 'unlisted' AND r.visibility = 'unlisted'))
             ) AS visible_recipe_count
         FROM collections c
         LEFT JOIN collection_recipes cr ON cr.collection_id = c.id
         LEFT JOIN recipes r ON r.id = cr.recipe_id
         WHERE c.id = $1 AND c.owner_id = $2
         GROUP BY c.id",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(target_visibility.as_str())
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;
    let Some(row) = row else {
        return Err(DbError::CollectionNotFound(collection_id));
    };
    let total_recipe_count: i64 = row.try_get("total_recipe_count").map_err(DbError::Query)?;
    let visible_recipe_count: i64 = row
        .try_get("visible_recipe_count")
        .map_err(DbError::Query)?;
    Ok(CollectionSharePreview {
        total_recipe_count,
        visible_recipe_count,
        hidden_recipe_count: total_recipe_count - visible_recipe_count,
    })
}

/// Return collection contents when the optional viewer may access the collection.
///
/// Owner reads retain the established owner-safe membership predicate. Shared
/// reads use the requested collection's visibility as the context for every
/// hierarchy count, recipe count, and returned row.
pub async fn get_collection_contents_for_viewer(
    pool: &PgPool,
    collection_id: Uuid,
    viewer_id: Option<Uuid>,
    offset: i64,
    limit: i64,
) -> Result<Option<CollectionContents>, DbError> {
    let access_row = sqlx::query(
        "SELECT c.owner_id, c.visibility, u.username AS owner_username
         FROM collections c
         INNER JOIN users u ON u.id = c.owner_id
         WHERE c.id = $1
           AND (c.owner_id = $2 OR c.visibility IN ('public', 'unlisted'))",
    )
    .bind(collection_id)
    .bind(viewer_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?;
    let Some(access_row) = access_row else {
        return Ok(None);
    };

    let owner_id: Uuid = access_row.try_get("owner_id").map_err(DbError::Query)?;
    let visibility = collection_visibility_from_row(&access_row).map_err(DbError::Query)?;
    let owner_username: String = access_row
        .try_get("owner_username")
        .map_err(DbError::Query)?;
    let can_edit = viewer_id == Some(owner_id);

    if can_edit {
        let breadcrumbs = get_collection_breadcrumbs(pool, collection_id, owner_id).await?;
        let Some(collection) = breadcrumbs.last().cloned() else {
            return Ok(None);
        };
        let child_collections = get_child_collections(pool, collection_id, owner_id).await?;
        let recipes = get_collection_recipes(pool, collection_id, owner_id, offset, limit).await?;
        return Ok(Some(CollectionContents {
            owner_username,
            can_edit: true,
            collection,
            breadcrumbs,
            child_collections,
            recipes,
        }));
    }

    let include_unlisted = match visibility {
        CollectionVisibility::Private => return Ok(None),
        CollectionVisibility::Unlisted => true,
        CollectionVisibility::Public => false,
    };

    // One snapshot prevents concurrent visibility changes from making shared
    // metadata, counts, and rows disagree within a response.
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;

    let owner_username: Option<String> = sqlx::query_scalar(
        "SELECT u.username
         FROM collections c
         INNER JOIN users u ON u.id = c.owner_id
         WHERE c.id = $1 AND c.owner_id = $2 AND c.visibility = $3",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(visibility.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let Some(owner_username) = owner_username else {
        return Ok(None);
    };

    let breadcrumb_query = format!(
        "WITH RECURSIVE ancestors AS (
             SELECT c.id, c.parent_collection_id, ARRAY[c.id] AS path, 0 AS depth
             FROM collections c
             WHERE c.id = $1 AND c.owner_id = $2
               AND (c.visibility = 'public' OR ($3 AND c.visibility = 'unlisted'))
             UNION ALL
             SELECT parent.id, parent.parent_collection_id,
                    ancestors.path || parent.id, ancestors.depth + 1
             FROM collections parent
             INNER JOIN ancestors ON parent.id = ancestors.parent_collection_id
             WHERE parent.owner_id = $2
               AND (parent.visibility = 'public'
                    OR ($3 AND parent.visibility = 'unlisted'))
               AND NOT parent.id = ANY(ancestors.path)
         )
         SELECT c.id,
                CASE
                    WHEN EXISTS (
                        SELECT 1 FROM ancestors visible_parent
                        WHERE visible_parent.id = c.parent_collection_id
                    )
                    THEN c.parent_collection_id
                    ELSE NULL
                END AS parent_collection_id,
                c.name, c.description, c.image, c.visibility,
                0::INTEGER AS sort_order, c.updated_at,
                {} AS preview_images,
                (SELECT COUNT(*)
                 FROM collections child
                 WHERE child.owner_id = c.owner_id
                   AND child.parent_collection_id = c.id
                   AND (child.visibility = 'public'
                        OR ($3 AND child.visibility = 'unlisted'))) AS child_count,
                (SELECT COUNT(*)
                 FROM collection_recipes cr
                 INNER JOIN recipes r ON r.id = cr.recipe_id
                 WHERE cr.collection_id = c.id
                   AND (r.visibility = 'public'
                        OR ($3 AND r.visibility = 'unlisted'))) AS recipe_count
         FROM ancestors
         INNER JOIN collections c ON c.id = ancestors.id
         ORDER BY ancestors.depth DESC",
        collection_preview_images_select(
            "(r.visibility = 'public' OR ($3 AND r.visibility = 'unlisted'))"
        )
    );
    let breadcrumb_rows = sqlx::query(&breadcrumb_query)
        .bind(collection_id)
        .bind(owner_id)
        .bind(include_unlisted)
        .fetch_all(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    let breadcrumbs = breadcrumb_rows
        .iter()
        .map(collection_summary_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::Query)?;
    let Some(collection) = breadcrumbs.last().cloned() else {
        return Ok(None);
    };

    let child_query = format!(
        "SELECT c.id, c.parent_collection_id, c.name, c.description, c.image, c.visibility,
                ((ROW_NUMBER() OVER (
                    ORDER BY c.sort_order, lower(c.name), c.id
                 ) - 1)::INTEGER) AS sort_order,
                c.updated_at,
                {} AS preview_images,
                (SELECT COUNT(*)
                 FROM collections child
                 WHERE child.owner_id = c.owner_id
                   AND child.parent_collection_id = c.id
                   AND (child.visibility = 'public'
                        OR ($3 AND child.visibility = 'unlisted'))) AS child_count,
                (SELECT COUNT(*)
                 FROM collection_recipes cr
                 INNER JOIN recipes r ON r.id = cr.recipe_id
                 WHERE cr.collection_id = c.id
                   AND (r.visibility = 'public'
                        OR ($3 AND r.visibility = 'unlisted'))) AS recipe_count
         FROM collections c
         WHERE c.parent_collection_id = $1 AND c.owner_id = $2
           AND (c.visibility = 'public' OR ($3 AND c.visibility = 'unlisted'))
         ORDER BY c.sort_order, lower(c.name), c.id",
        collection_preview_images_select(
            "(r.visibility = 'public' OR ($3 AND r.visibility = 'unlisted'))"
        )
    );
    let child_rows = sqlx::query(&child_query)
        .bind(collection_id)
        .bind(owner_id)
        .bind(include_unlisted)
        .fetch_all(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    let child_collections = child_rows
        .iter()
        .map(collection_summary_from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::Query)?;

    let total_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM collection_recipes cr
         INNER JOIN collections c ON c.id = cr.collection_id AND c.owner_id = $2
         INNER JOIN recipes r ON r.id = cr.recipe_id
         WHERE cr.collection_id = $1
           AND (c.visibility = 'public' OR ($3 AND c.visibility = 'unlisted'))
           AND (r.visibility = 'public' OR ($3 AND r.visibility = 'unlisted'))",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(include_unlisted)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let recipe_rows = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary,
                r.prep_time_minutes, r.cook_time_minutes, r.servings,
                r.ingredients, r.instructions, r.equipment, r.images,
                r.visibility, r.created_at, r.updated_at,
                u.username AS author_username, u.avatar_url AS author_avatar_url
         FROM collection_recipes cr
         INNER JOIN collections c ON c.id = cr.collection_id AND c.owner_id = $2
         INNER JOIN recipes r ON r.id = cr.recipe_id
         LEFT JOIN users u ON u.id = r.user_id
         WHERE cr.collection_id = $1
           AND (c.visibility = 'public' OR ($3 AND c.visibility = 'unlisted'))
           AND (r.visibility = 'public' OR ($3 AND r.visibility = 'unlisted'))
         ORDER BY cr.sort_order, cr.created_at, cr.recipe_id
         LIMIT $4 OFFSET $5",
    )
    .bind(collection_id)
    .bind(owner_id)
    .bind(include_unlisted)
    .bind(limit.max(0))
    .bind(offset.max(0))
    .fetch_all(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let returned_recipes = recipe_rows
        .iter()
        .map(Recipe::from_row)
        .collect::<Result<Vec<_>, _>>()
        .map_err(DbError::Query)?;
    let recipes = CollectionRecipeList {
        has_more: offset.max(0) + (returned_recipes.len() as i64) < total_count,
        recipes: returned_recipes,
        total_count,
    };

    tx.commit().await.map_err(DbError::Query)?;
    Ok(Some(CollectionContents {
        owner_username,
        can_edit: false,
        collection,
        breadcrumbs,
        child_collections,
        recipes,
    }))
}

/// List collections owned by a user that contain an accessible recipe.
pub async fn get_recipe_collections(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    owner_id: Uuid,
) -> Result<Vec<Uuid>, DbError> {
    sqlx::query_scalar(
        "SELECT cr.collection_id
         FROM collection_recipes cr
         INNER JOIN collections c ON c.id = cr.collection_id AND c.owner_id = $2
         INNER JOIN recipes r ON r.id = cr.recipe_id
         WHERE cr.recipe_id = $1
           AND (r.user_id = $2 OR r.visibility IN ('public', 'unlisted'))
         ORDER BY cr.collection_id",
    )
    .bind(recipe_id)
    .bind(owner_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Count visible direct recipe members in an owner-scoped collection.
pub async fn get_collection_recipe_count(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
) -> Result<i64, DbError> {
    if get_collection_by_id_and_owner(pool, collection_id, owner_id)
        .await?
        .is_none()
    {
        return Err(DbError::CollectionNotFound(collection_id));
    }
    sqlx::query_scalar(
        "SELECT COUNT(*)
         FROM collection_recipes cr
         INNER JOIN collections c ON c.id = cr.collection_id AND c.owner_id = $2
         INNER JOIN recipes r ON r.id = cr.recipe_id
         WHERE cr.collection_id = $1
           AND (r.user_id = $2 OR r.visibility IN ('public', 'unlisted'))",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Query)
}

/// Reorder visible recipe members and retain inaccessible members after them.
///
/// A recipe saved while public or unlisted can later become private. Such a membership must stay
/// hidden without making it impossible for the collection owner to reorder the recipes they can
/// still see. The submitted IDs must exactly match the currently visible membership; hidden rows
/// retain their relative order after the visible rows and are never returned to the caller.
pub async fn reorder_collection_recipes(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    ordered_recipe_ids: &[Uuid],
) -> Result<(), DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    let collection_exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM collections WHERE id = $1 AND owner_id = $2 FOR UPDATE")
            .bind(collection_id)
            .bind(owner_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    if collection_exists.is_none() {
        return Err(DbError::CollectionNotFound(collection_id));
    }
    let membership_rows = sqlx::query(
        "SELECT cr.recipe_id,
                (r.user_id = $2 OR r.visibility IN ('public', 'unlisted')) AS is_visible
         FROM collection_recipes cr
         INNER JOIN recipes r ON r.id = cr.recipe_id
         WHERE cr.collection_id = $1
         ORDER BY cr.sort_order, cr.created_at, cr.recipe_id
         FOR UPDATE OF cr",
    )
    .bind(collection_id)
    .bind(owner_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    let mut visible_ids = Vec::new();
    let mut hidden_ids = Vec::new();
    for row in membership_rows {
        let recipe_id = row.try_get("recipe_id").map_err(DbError::Query)?;
        if row.try_get("is_visible").map_err(DbError::Query)? {
            visible_ids.push(recipe_id);
        } else {
            hidden_ids.push(recipe_id);
        }
    }
    if visible_ids.len() != ordered_recipe_ids.len()
        || visible_ids.iter().collect::<HashSet<_>>()
            != ordered_recipe_ids.iter().collect::<HashSet<_>>()
    {
        return Err(DbError::InvalidCollectionOrder);
    }
    for (sort_order, recipe_id) in ordered_recipe_ids.iter().chain(&hidden_ids).enumerate() {
        sqlx::query(
            "UPDATE collection_recipes SET sort_order = $3
             WHERE collection_id = $1 AND recipe_id = $2",
        )
        .bind(collection_id)
        .bind(recipe_id)
        .bind(sort_order as i32)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    }
    sqlx::query("UPDATE collections SET updated_at = NOW() WHERE id = $1")
        .bind(collection_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    tx.commit().await.map_err(DbError::Query)
}

/// Reorder all direct child collections after verifying exact membership.
pub async fn reorder_child_collections(
    pool: &PgPool,
    owner_id: Uuid,
    parent_collection_id: Option<Uuid>,
    ordered_collection_ids: &[Uuid],
) -> Result<(), DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;
    lock_owner(&mut tx, owner_id).await?;
    if let Some(parent_id) = parent_collection_id {
        let parent_exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM collections WHERE id = $1 AND owner_id = $2 FOR UPDATE",
        )
        .bind(parent_id)
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(DbError::Query)?;
        if parent_exists.is_none() {
            return Err(DbError::CollectionParentNotFound(parent_id));
        }
    }
    let current_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM collections
         WHERE owner_id = $1 AND parent_collection_id IS NOT DISTINCT FROM $2
         ORDER BY id FOR UPDATE",
    )
    .bind(owner_id)
    .bind(parent_collection_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(DbError::Query)?;
    if current_ids.len() != ordered_collection_ids.len()
        || current_ids.iter().collect::<HashSet<_>>()
            != ordered_collection_ids.iter().collect::<HashSet<_>>()
    {
        return Err(DbError::InvalidCollectionOrder);
    }
    for (sort_order, child_id) in ordered_collection_ids.iter().enumerate() {
        sqlx::query(
            "UPDATE collections
             SET sort_order = $2, updated_at = NOW()
             WHERE id = $1 AND owner_id = $3",
        )
        .bind(child_id)
        .bind(sort_order as i32)
        .bind(owner_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    }
    if let Some(parent_id) = parent_collection_id {
        sqlx::query("UPDATE collections SET updated_at = NOW() WHERE id = $1 AND owner_id = $2")
            .bind(parent_id)
            .bind(owner_id)
            .execute(&mut *tx)
            .await
            .map_err(DbError::Query)?;
    }
    tx.commit().await.map_err(DbError::Query)
}

// ── Auth state queries ──────────────────────────────────────────────────────

/// Insert an auth state for OAuth CSRF protection.
pub async fn insert_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
    code_verifier: &str,
    user_id: Option<Uuid>,
) -> Result<(), DbError> {
    insert_auth_state_for(
        executor,
        id,
        provider,
        redirect_uri,
        code_verifier,
        "login",
        user_id,
    )
    .await
}

/// Insert an auth state with an explicit immutable flow purpose.
pub async fn insert_auth_state_for(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
    code_verifier: &str,
    purpose: &str,
    user_id: Option<Uuid>,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO auth_states (id, provider, redirect_uri, code_verifier, purpose, user_id) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(id)
        .bind(provider)
        .bind(redirect_uri)
        .bind(code_verifier)
        .bind(purpose)
        .bind(user_id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

/// Atomically delete an auth state by ID and return the deleted row.
///
/// Uses `DELETE ... RETURNING *` so the first caller consumes the state.
/// Concurrent callers with the same ID will get `None` (row already deleted).
pub async fn delete_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
) -> Result<Option<AuthState>, DbError> {
    sqlx::query_as!(
        AuthState,
        "DELETE FROM auth_states WHERE id = $1 RETURNING id, redirect_uri, provider, code_verifier, purpose, user_id, created_at",
        id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete all auth states older than 15 minutes.
///
/// Used by the application-level fallback cleanup task when pg_cron is
/// unavailable. Also callable directly for testing.
pub async fn cleanup_expired_auth_states(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
) -> Result<u64, DbError> {
    let result =
        sqlx::query("DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL '15 minutes'")
            .execute(executor)
            .await
            .map_err(DbError::Query)?;
    Ok(result.rows_affected())
}

// ── Session queries ─────────────────────────────────────────────────────────

/// Insert a new session row and return it.
pub async fn insert_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
) -> Result<Session, DbError> {
    sqlx::query_as!(
        Session,
        "INSERT INTO sessions (user_id, expires_at) VALUES ($1, $2)
         RETURNING id, user_id, created_at, expires_at, refreshed_at, revoked",
        user_id,
        expires_at,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Get an active (non-revoked, non-expired) session by its ID.
///
/// Used by `verify_session` to validate the JWT's `sub` claim against the DB.
/// Returns `None` if the session doesn't exist, is revoked, or is expired.
pub async fn get_active_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    session_id: Uuid,
) -> Result<Option<Session>, DbError> {
    sqlx::query_as!(
        Session,
        "SELECT id, user_id, created_at, expires_at, refreshed_at, revoked
         FROM sessions
         WHERE id = $1 AND revoked = FALSE AND expires_at > NOW()",
        session_id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Revoke a session by ID. Returns `true` if a row was updated.
pub async fn revoke_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    session_id: Uuid,
) -> Result<bool, DbError> {
    let rows = sqlx::query("UPDATE sessions SET revoked = TRUE WHERE id = $1")
        .bind(session_id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    Ok(rows > 0)
}

/// Refresh a session: extend `expires_at` and set `refreshed_at`.
///
/// Only refreshes if the session is active (not revoked, not expired).
/// Returns the updated session on success, or `None` if the session is gone.
pub async fn refresh_session(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    session_id: Uuid,
    new_expires_at: DateTime<Utc>,
) -> Result<Option<Session>, DbError> {
    sqlx::query_as!(
        Session,
        "UPDATE sessions
         SET expires_at = $2, refreshed_at = NOW()
         WHERE id = $1 AND revoked = FALSE AND expires_at > NOW()
         RETURNING id, user_id, created_at, expires_at, refreshed_at, revoked",
        session_id,
        new_expires_at,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete expired + revoked sessions older than a given age.
///
/// Used by the pg_cron cleanup job (and testable directly).
/// Returns the number of rows deleted.
pub async fn cleanup_expired_sessions(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    older_than: &str, // e.g. "24 hours"
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "DELETE FROM sessions WHERE (revoked = TRUE OR expires_at < NOW()) AND created_at < NOW() - INTERVAL $1",
    )
    .bind(older_than)
    .execute(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(result.rows_affected())
}

// ── OAuth account queries ───────────────────────────────────────────────────

/// Get an OAuth account by provider and provider user ID.
pub async fn get_oauth_account_by_provider(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<OauthAccount>, DbError> {
    sqlx::query_as!(
        OauthAccount,
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE provider = $1 AND provider_user_id = $2",
        provider,
        provider_user_id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Get an OAuth account by email.
pub async fn get_oauth_account_by_email(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    email: &str,
) -> Result<Option<OauthAccount>, DbError> {
    sqlx::query_as!(
        OauthAccount,
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE email = $1 \
         LIMIT 1",
        email,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Update the `last_used_at` timestamp for an OAuth account.
pub async fn update_oauth_last_used(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
) -> Result<(), DbError> {
    sqlx::query("UPDATE oauth_accounts SET last_used_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

/// Get all OAuth accounts linked to a user, for display on the settings page.
pub async fn get_oauth_accounts_by_user(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<OauthAccountRow>, DbError> {
    sqlx::query_as!(
        OauthAccountRow,
        "SELECT id, provider, provider_user_id, email, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE user_id = $1 \
         ORDER BY provider",
        user_id,
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get all OAuth accounts for a user (full records including refresh tokens).
/// Used by the revocation flow before account deletion.
pub async fn get_oauth_accounts_by_user_id(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<OauthAccount>, DbError> {
    sqlx::query_as!(
        OauthAccount,
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts \
         WHERE user_id = $1",
        user_id,
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete a single OAuth account, guarded by user_id to prevent cross-user deletion.
///
/// Revokes the OAuth token with the provider before deletion. Revocation
/// failures are logged but never block deletion.
pub async fn delete_oauth_account(
    pool: &PgPool,
    account_id: Uuid,
    user_id: Uuid,
) -> Result<(), DbError> {
    // Fetch the account first to get the refresh token for revocation
    let account = sqlx::query_as!(
        OauthAccount,
        "SELECT id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at \
         FROM oauth_accounts WHERE id = $1 AND user_id = $2",
        account_id,
        user_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(DbError::Query)?
    .ok_or_else(|| DbError::Query(sqlx::Error::RowNotFound))?;

    // Revoke the token before deletion (non-blocking — failures are logged)
    crate::auth::revoke::revoke_account(&account).await;

    // Now delete the account
    let rows = sqlx::query("DELETE FROM oauth_accounts WHERE id = $1 AND user_id = $2")
        .bind(account_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Query)?
        .rows_affected();

    if rows == 0 {
        return Err(DbError::Query(sqlx::Error::RowNotFound));
    }
    Ok(())
}

/// Count the number of OAuth accounts linked to a user.
pub async fn count_oauth_accounts(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(count)
}

// ── User queries ────────────────────────────────────────────────────────────

/// Insert a new user. Returns the created user with the generated ID.
pub async fn insert_user(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    username: &str,
    display_name: &str,
    email: &str,
    avatar_url: Option<&str>,
) -> Result<User, DbError> {
    sqlx::query_as!(
        User,
        "INSERT INTO users (username, display_name, email, avatar_url) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
        username,
        display_name,
        email,
        avatar_url,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Insert a new OAuth account linked to a user.
pub async fn insert_oauth_account(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    provider: &str,
    provider_user_id: &str,
    email: Option<&str>,
    profile_data: Option<&serde_json::Value>,
    refresh_token: Option<&str>,
) -> Result<OauthAccount, DbError> {
    sqlx::query_as!(
        OauthAccount,
        "INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, profile_data, refresh_token) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, refresh_token, created_at, last_used_at",
        user_id,
        provider,
        provider_user_id,
        email,
        profile_data,
        refresh_token,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Get a user by ID.
pub async fn get_user_by_id(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
) -> Result<Option<User>, DbError> {
    sqlx::query_as!(
        User,
        "SELECT id, username, display_name, email, avatar_url, bio, created_at, updated_at \
         FROM users WHERE id = $1",
        id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Check whether a username already exists.
pub async fn username_exists(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    username: &str,
) -> Result<bool, DbError> {
    // Concatenating the empty string keeps identity lookup on PostgreSQL's
    // native text equality path instead of matching either ParadeDB expression
    // index. This remains portable to the plain PostgreSQL unit-test database.
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE (username || '') = $1)")
            .bind(username)
            .fetch_one(executor)
            .await
            .map_err(DbError::Query)?;
    Ok(exists)
}

/// Get a user by email address.
pub async fn get_user_by_email(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    email: &str,
) -> Result<Option<User>, DbError> {
    sqlx::query_as!(
        User,
        "SELECT id, username, display_name, email, avatar_url, bio, \
          created_at, updated_at FROM users WHERE email = $1",
        email,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

async fn snapshot_owned_recipe_lineage(
    transaction: &mut sqlx::Transaction<'_, Postgres>,
    user_id: Uuid,
    recipe_id: Option<Uuid>,
) -> Result<(), DbError> {
    let _locked_recipe_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM recipes \
         WHERE user_id = $1 AND ($2::UUID IS NULL OR id = $2) \
         ORDER BY id FOR UPDATE",
    )
    .bind(user_id)
    .bind(recipe_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(DbError::Query)?;

    sqlx::query(
        "UPDATE fork_relationships AS relationship SET \
             original_recipe_key = CASE \
                 WHEN relationship.original_recipe_key = recipe.id \
                   OR relationship.original_recipe_id = recipe.id \
                 THEN recipe.id ELSE relationship.original_recipe_key END, \
             original_title = CASE \
                 WHEN relationship.original_recipe_key = recipe.id \
                   OR relationship.original_recipe_id = recipe.id \
                 THEN recipe.title ELSE relationship.original_title END, \
             original_owner_id = CASE \
                 WHEN relationship.original_recipe_key = recipe.id \
                   OR relationship.original_recipe_id = recipe.id \
                 THEN recipe.user_id ELSE relationship.original_owner_id END, \
             original_owner_username = CASE \
                 WHEN relationship.original_recipe_key = recipe.id \
                   OR relationship.original_recipe_id = recipe.id \
                 THEN owner.username ELSE relationship.original_owner_username END, \
             original_visibility = CASE \
                 WHEN relationship.original_recipe_key = recipe.id \
                   OR relationship.original_recipe_id = recipe.id \
                 THEN recipe.visibility ELSE relationship.original_visibility END, \
             forked_recipe_key = CASE \
                 WHEN relationship.forked_recipe_key = recipe.id \
                   OR relationship.forked_recipe_id = recipe.id \
                 THEN recipe.id ELSE relationship.forked_recipe_key END, \
             forked_title = CASE \
                 WHEN relationship.forked_recipe_key = recipe.id \
                   OR relationship.forked_recipe_id = recipe.id \
                 THEN recipe.title ELSE relationship.forked_title END, \
             forked_owner_id = CASE \
                 WHEN relationship.forked_recipe_key = recipe.id \
                   OR relationship.forked_recipe_id = recipe.id \
                 THEN recipe.user_id ELSE relationship.forked_owner_id END, \
             forked_owner_username = CASE \
                 WHEN relationship.forked_recipe_key = recipe.id \
                   OR relationship.forked_recipe_id = recipe.id \
                 THEN owner.username ELSE relationship.forked_owner_username END, \
             forked_visibility = CASE \
                 WHEN relationship.forked_recipe_key = recipe.id \
                   OR relationship.forked_recipe_id = recipe.id \
                 THEN recipe.visibility ELSE relationship.forked_visibility END \
         FROM recipes AS recipe \
         JOIN users AS owner ON owner.id = recipe.user_id \
         WHERE recipe.user_id = $1 \
           AND ($2::UUID IS NULL OR recipe.id = $2) \
           AND (relationship.original_recipe_key = recipe.id \
                OR relationship.original_recipe_id = recipe.id \
                OR relationship.forked_recipe_key = recipe.id \
                OR relationship.forked_recipe_id = recipe.id)",
    )
    .bind(user_id)
    .bind(recipe_id)
    .execute(&mut **transaction)
    .await
    .map_err(DbError::Query)?;
    Ok(())
}

/// Delete a user by ID. OAuth accounts cascade automatically via `ON DELETE CASCADE`.
///
/// Revokes all OAuth tokens with providers before deletion. Revocation
/// failures are logged but never block deletion.
pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), DbError> {
    // Revoke all OAuth tokens before deletion (non-blocking — failures are logged)
    crate::auth::revoke::revoke_all_user_tokens(pool, user_id).await;

    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(DbError::Query)?;
    snapshot_owned_recipe_lineage(&mut transaction, user_id, None).await?;

    // Now delete the user (oauth_accounts and recipes cascade automatically)
    let rows = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    if rows == 0 {
        return Err(DbError::Query(sqlx::Error::RowNotFound));
    }
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(())
}

/// Update a user's display name and bio. Returns the updated user record.
pub async fn update_user_profile(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    display_name: &str,
    bio: Option<&str>,
) -> Result<User, DbError> {
    sqlx::query_as!(
        User,
        "UPDATE users SET display_name = $2, bio = $3, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
        user_id,
        display_name,
        bio,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Update a user's username. Checks uniqueness first (excluding the current user).
///
/// Returns the updated user record on success.
/// Returns `DbError::UsernameTaken` if the username is already taken by another user.
/// Returns `DbError::Query` if the user doesn't exist or another DB error occurs.
pub async fn update_username(
    executor: impl sqlx::Executor<'_, Database = Postgres> + Clone,
    user_id: Uuid,
    new_username: &str,
) -> Result<User, DbError> {
    // Check uniqueness (exclude the current user's own username)
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users \
         WHERE (username || '') = $1 AND id != $2)",
    )
    .bind(new_username)
    .bind(user_id)
    .fetch_one(executor.clone())
    .await
    .map_err(DbError::Query)?;

    if exists {
        return Err(DbError::UsernameTaken);
    }

    sqlx::query_as!(
        User,
        "UPDATE users SET username = $2, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
        user_id,
        new_username,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Update a user's avatar URL.
///
/// Pass `Some(url)` to set a new avatar, or `None` to remove the avatar.
/// Returns the updated user record on success.
pub async fn update_avatar_url(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    avatar_url: Option<String>,
) -> Result<User, DbError> {
    sqlx::query_as!(
        User,
        "UPDATE users SET avatar_url = $2, updated_at = NOW() \
         WHERE id = $1 \
         RETURNING id, username, display_name, email, avatar_url, bio, created_at, updated_at",
        user_id,
        avatar_url,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

// ── Recipe queries ──────────────────────────────────────────────────────────

/// Insert a new recipe. Returns the created recipe with the generated ID.
#[allow(clippy::too_many_arguments)]
pub async fn insert_recipe(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    title: &str,
    description: Option<&str>,
    commentary: Option<&str>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: &[RecipeIngredient],
    instructions: &[RecipeStep],
    equipment: &[RecipeEquipment],
    images: &[RecipeImage],
    visibility: &str,
) -> Result<Recipe, DbError> {
    let ingredients_json = serde_json::to_value(ingredients).map_err(DbError::SerdeJson)?;
    let instructions_json = serde_json::to_value(instructions).map_err(DbError::SerdeJson)?;
    let equipment_json = serde_json::to_value(equipment).map_err(DbError::SerdeJson)?;
    let images_json = serde_json::to_value(images).map_err(DbError::SerdeJson)?;

    let row = sqlx::query(
        "INSERT INTO recipes (user_id, title, description, commentary, prep_time_minutes, cook_time_minutes, servings, ingredients, instructions, equipment, images, visibility) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING id, user_id, title, description, commentary, prep_time_minutes, cook_time_minutes, servings, ingredients, instructions, equipment, images, visibility, created_at, updated_at, \
            (SELECT username FROM users WHERE users.id = $1) AS author_username, \
            (SELECT avatar_url FROM users WHERE users.id = $1) AS author_avatar_url",
    )
    .bind(user_id)
    .bind(title)
    .bind(description)
    .bind(commentary)
    .bind(prep_time_minutes)
    .bind(cook_time_minutes)
    .bind(servings)
    .bind(ingredients_json)
    .bind(instructions_json)
    .bind(equipment_json)
    .bind(images_json)
    .bind(visibility)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)?;

    Recipe::from_row(&row).map_err(DbError::Query)
}

/// Get a recipe by ID. Returns `None` if not found.
pub async fn get_recipe_by_id(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
) -> Result<Option<Recipe>, DbError> {
    let row = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.id = $1",
    )
    .bind(id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?;

    row.map(|r| Recipe::from_row(&r))
        .transpose()
        .map_err(DbError::Query)
}

/// Get a recipe by ID, guarded by user_id to prevent cross-user access.
/// Returns `DbError::RecipeNotFound` if the recipe doesn't exist or belongs to another user.
pub async fn get_recipe_by_id_and_owner(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
    user_id: Uuid,
) -> Result<Recipe, DbError> {
    let row = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.id = $1 AND r.user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::RecipeNotFound(id),
        other => DbError::Query(other),
    })?;

    Recipe::from_row(&row).map_err(DbError::Query)
}

/// Get all recipes for a user, ordered by most recently created first.
pub async fn get_recipes_by_owner(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<Vec<Recipe>, DbError> {
    let rows = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.user_id = $1 \
         ORDER BY r.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)?;

    rows.into_iter()
        .map(|r| Recipe::from_row(&r))
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// Update a recipe, enforcing ownership via `user_id` in the WHERE clause.
/// Returns the updated recipe.
/// Returns `DbError::RecipeNotFound` if the recipe doesn't exist or doesn't belong to the user.
#[allow(clippy::too_many_arguments)]
pub async fn update_recipe(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
    user_id: Uuid,
    title: &str,
    description: Option<&str>,
    commentary: Option<&str>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: &[RecipeIngredient],
    instructions: &[RecipeStep],
    equipment: &[RecipeEquipment],
    images: &[RecipeImage],
    visibility: Option<&str>,
) -> Result<Recipe, DbError> {
    let ingredients_json = serde_json::to_value(ingredients).map_err(DbError::SerdeJson)?;
    let instructions_json = serde_json::to_value(instructions).map_err(DbError::SerdeJson)?;
    let equipment_json = serde_json::to_value(equipment).map_err(DbError::SerdeJson)?;
    let images_json = serde_json::to_value(images).map_err(DbError::SerdeJson)?;

    let row = sqlx::query(
        "UPDATE recipes \
         SET title = $3, description = $4, commentary = $5, prep_time_minutes = $6, cook_time_minutes = $7, \
             servings = $8, ingredients = $9, instructions = $10, equipment = $11, images = $12, \
             visibility = COALESCE($13::VARCHAR, recipes.visibility), \
             updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 \
         RETURNING id, user_id, title, description, commentary, prep_time_minutes, cook_time_minutes, servings, ingredients, instructions, equipment, images, visibility, created_at, updated_at, \
            (SELECT username FROM users WHERE users.id = $2) AS author_username, \
            (SELECT avatar_url FROM users WHERE users.id = $2) AS author_avatar_url",
    )
    .bind(id)
    .bind(user_id)
    .bind(title)
    .bind(description)
    .bind(commentary)
    .bind(prep_time_minutes)
    .bind(cook_time_minutes)
    .bind(servings)
    .bind(ingredients_json)
    .bind(instructions_json)
    .bind(equipment_json)
    .bind(images_json)
    .bind(visibility)
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::RecipeNotFound(id),
        other => DbError::Query(other),
    })?;

    Recipe::from_row(&row).map_err(DbError::Query)
}

/// Change only recipe-level visibility, enforcing ownership.
pub async fn update_recipe_visibility(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
    user_id: Uuid,
    visibility: &str,
) -> Result<Recipe, DbError> {
    versioning::validate_recipe_visibility(visibility)?;
    let row = sqlx::query(
        "UPDATE recipes SET visibility = $3, updated_at = NOW() \
         WHERE id = $1 AND user_id = $2 \
         RETURNING id, user_id, title, description, commentary, prep_time_minutes, \
             cook_time_minutes, servings, ingredients, instructions, equipment, images, \
             visibility, created_at, updated_at, \
             (SELECT username FROM users WHERE users.id = $2) AS author_username, \
             (SELECT avatar_url FROM users WHERE users.id = $2) AS author_avatar_url",
    )
    .bind(id)
    .bind(user_id)
    .bind(visibility)
    .fetch_one(executor)
    .await
    .map_err(|error| match error {
        sqlx::Error::RowNotFound => DbError::RecipeNotFound(id),
        other => DbError::Query(other),
    })?;

    Recipe::from_row(&row).map_err(DbError::Query)
}

/// Delete a recipe, enforcing ownership via `user_id` in the WHERE clause.
/// Tags cascade automatically via `ON DELETE CASCADE`.
/// Returns `DbError::RecipeNotFound` if the recipe doesn't exist or doesn't belong to the user.
pub async fn delete_recipe(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), DbError> {
    let mut transaction = pool.begin().await.map_err(DbError::Query)?;
    snapshot_owned_recipe_lineage(&mut transaction, user_id, Some(id)).await?;

    let rows = sqlx::query("DELETE FROM recipes WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    if rows == 0 {
        return Err(DbError::RecipeNotFound(id));
    }
    transaction.commit().await.map_err(DbError::Query)?;
    Ok(())
}

/// Insert tags for a recipe. Replaces any existing tags for that recipe.
pub async fn insert_recipe_tags(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipe_id: Uuid,
    tags: &[String],
) -> Result<(), DbError> {
    // Delete existing tags first
    sqlx::query("DELETE FROM recipe_tags WHERE recipe_id = $1")
        .bind(recipe_id)
        .execute(&mut **tx)
        .await
        .map_err(DbError::Query)?;

    // Insert new tags
    if !tags.is_empty() {
        for tag in tags {
            sqlx::query("INSERT INTO recipe_tags (recipe_id, tag) VALUES ($1, $2)")
                .bind(recipe_id)
                .bind(tag)
                .execute(&mut **tx)
                .await
                .map_err(DbError::Query)?;
        }
    }
    Ok(())
}

/// Get all tags for a recipe.
pub async fn get_recipe_tags(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<Vec<RecipeTag>, DbError> {
    sqlx::query_as!(
        RecipeTag,
        "SELECT recipe_id, tag FROM recipe_tags WHERE recipe_id = $1 ORDER BY tag",
        recipe_id,
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get recipes for a user with pagination, ordered by most recently created first.
pub async fn get_recipes_by_owner_paginated(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Recipe>, DbError> {
    let rows = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.user_id = $1 \
         ORDER BY r.created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)?;

    rows.into_iter()
        .map(|r| Recipe::from_row(&r))
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// Count total recipes for a user.
pub async fn count_recipes_by_owner(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recipes WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(count)
}

// ── Public access queries (CP8) ──────────────────────────────────────────────

/// Get a full user record by username. Returns `None` if not found.
pub async fn get_user_by_username(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    username: &str,
) -> Result<Option<User>, DbError> {
    sqlx::query_as::<_, User>(
        "SELECT id, username, display_name, email, avatar_url, bio, created_at, updated_at \
         FROM users WHERE (username || '') = $1",
    )
    .bind(username)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Get public recipes with pagination, ordered by most recently created first.
pub async fn get_public_recipes_paginated(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Recipe>, DbError> {
    let rows = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.visibility = 'public' \
         ORDER BY r.created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)?;

    rows.into_iter()
        .map(|r| Recipe::from_row(&r))
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// Count total public recipes (for pagination has_more calculation).
pub async fn count_public_recipes(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM recipes WHERE visibility = 'public'")
        .fetch_one(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(count)
}

/// Get a recipe by ID for public/unlisted access (no ownership check).
/// Returns `None` if the recipe doesn't exist or is private.
pub async fn get_recipe_by_id_public(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: Uuid,
) -> Result<Option<Recipe>, DbError> {
    let row = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.id = $1 AND r.visibility IN ('public', 'unlisted')",
    )
    .bind(id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?;

    row.map(|r| Recipe::from_row(&r))
        .transpose()
        .map_err(DbError::Query)
}

/// Get a user's public recipes with pagination, ordered by most recently created first.
pub async fn get_user_public_recipes(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Recipe>, DbError> {
    let rows = sqlx::query(
        "SELECT r.id, r.user_id, r.title, r.description, r.commentary, r.prep_time_minutes, r.cook_time_minutes, r.servings, r.ingredients, r.instructions, r.equipment, r.images, r.visibility, r.created_at, r.updated_at, u.username AS author_username, u.avatar_url AS author_avatar_url \
         FROM recipes r LEFT JOIN users u ON u.id = r.user_id WHERE r.user_id = $1 AND r.visibility = 'public' \
         ORDER BY r.created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)?;

    rows.into_iter()
        .map(|r| Recipe::from_row(&r))
        .collect::<Result<_, _>>()
        .map_err(DbError::Query)
}

/// Count a user's public recipes (for profile page pagination).
pub async fn count_user_public_recipes(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    user_id: Uuid,
) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM recipes WHERE user_id = $1 AND visibility = 'public'",
    )
    .bind(user_id)
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)?;
    Ok(count)
}

/// Get distinct tags from all public recipes, ordered alphabetically.
pub async fn get_distinct_public_tags(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
) -> Result<Vec<String>, DbError> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT tag FROM recipe_tags
         INNER JOIN recipes ON recipe_tags.recipe_id = recipes.id
         WHERE recipes.visibility = 'public'
         ORDER BY tag",
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get tags for a specific recipe that is public or unlisted.
/// Joins with recipes table to verify visibility. Returns empty vec if recipe
/// is private or doesn't exist (no existence leakage).
pub async fn get_public_recipe_tags(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<Vec<RecipeTag>, DbError> {
    sqlx::query_as!(
        RecipeTag,
        "SELECT recipe_tags.recipe_id, recipe_tags.tag \
          FROM recipe_tags \
          INNER JOIN recipes ON recipe_tags.recipe_id = recipes.id \
          WHERE recipe_tags.recipe_id = $1 AND recipes.visibility IN ('public', 'unlisted') \
          ORDER BY recipe_tags.tag",
        recipe_id,
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get all distinct image URLs referenced by recipes or collection covers.
///
/// Used by the orphan cleanup worker to determine which storage objects are still needed.
pub async fn get_all_image_urls(pool: &PgPool) -> Result<Vec<String>, DbError> {
    let rows = sqlx::query_scalar::<_, String>(
        "WITH RECURSIVE recipe_documents(images, instructions) AS (
             SELECT images, instructions FROM recipes
             UNION ALL
             SELECT snapshot -> 'images', snapshot -> 'instructions' FROM recipe_drafts
             UNION ALL
             SELECT snapshot -> 'images', snapshot -> 'instructions' FROM recipe_versions
         ),
         step_entries(step_entry) AS (
             SELECT step_entry
             FROM recipe_documents
             CROSS JOIN LATERAL jsonb_array_elements(instructions) AS step_entry
             WHERE instructions IS NOT NULL
                AND jsonb_typeof(instructions) = 'array'
             UNION ALL
             SELECT child_step
             FROM step_entries
             CROSS JOIN LATERAL jsonb_array_elements(
                 CASE
                     WHEN jsonb_typeof(step_entry -> 'sub_steps') = 'array'
                     THEN step_entry -> 'sub_steps'
                     ELSE '[]'::jsonb
                 END
             ) AS child_step
         ),
         image_urls(image_url) AS (
             SELECT image_urls.image_url
             FROM recipe_documents
             CROSS JOIN LATERAL jsonb_array_elements(images) AS image_entry
             CROSS JOIN LATERAL (
                 VALUES (image_entry ->> 'url'), (image_entry ->> 'thumb_url')
             ) AS image_urls(image_url)
             WHERE images IS NOT NULL
                AND jsonb_array_length(images) > 0
                AND jsonb_typeof(image_entry) = 'object'
                UNION ALL
                SELECT step_photo_urls.image_url
                FROM step_entries
                CROSS JOIN LATERAL jsonb_array_elements(
                    CASE
                        WHEN jsonb_typeof(step_entry -> 'photos') = 'array'
                        THEN step_entry -> 'photos'
                        ELSE '[]'::jsonb
                    END
                ) AS step_photo
                CROSS JOIN LATERAL (
                    VALUES (
                        step_photo ->> 'url'
                    ), (
                        step_photo ->> 'thumb_url'
                    )
                ) AS step_photo_urls(image_url)
                WHERE jsonb_typeof(step_photo) = 'object'
                UNION ALL
                SELECT collection_image_urls.image_url
                FROM collections
                CROSS JOIN LATERAL (
                    VALUES (image ->> 'url'), (image ->> 'thumb_url')
                ) AS collection_image_urls(image_url)
                WHERE image IS NOT NULL
                  AND jsonb_typeof(image) = 'object'
            )
         SELECT DISTINCT image_url
         FROM image_urls
         WHERE image_url IS NOT NULL
            AND image_url <> ''",
    )
    .fetch_all(pool)
    .await
    .map_err(DbError::Query)?;
    Ok(rows)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[tokio::test]
    async fn test_insert_and_get_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();
        let user = insert_user(
            &pool,
            &format!("testuser_{u}"),
            "Test User",
            &format!("test{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        assert_eq!(user.username, format!("testuser_{u}"));
        assert_eq!(user.display_name, "Test User");
        assert!(user.bio.is_none());

        // Lookup by ID
        let found = get_user_by_id(&pool, user.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);
    }

    #[tokio::test]
    async fn test_get_nonexistent_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let fake_id = Uuid::nil();
        let result = get_user_by_id(&pool, fake_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_insert_and_get_auth_state() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let state_id = format!("test-state-{}", test_utils::uid());
        let verifier = "test-verifier-that-is-at-least-43-chars-long!";
        insert_auth_state(&pool, &state_id, "google", "/dashboard", verifier, None)
            .await
            .unwrap();

        let state = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.id, state_id);
        assert_eq!(state.redirect_uri, "/dashboard");
        assert_eq!(state.provider, "google");
        assert_eq!(state.code_verifier, Some(verifier.to_string()));
    }

    #[tokio::test]
    async fn test_delete_auth_state() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let state_id = format!("test-state-del-{}", test_utils::uid());
        insert_auth_state(
            &pool,
            &state_id,
            "github",
            "/login",
            "dummy-verifier-minimum-43-chars-long!!",
            None,
        )
        .await
        .unwrap();

        let deleted = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(deleted.is_some());
        let deleted_state = deleted.unwrap();
        assert_eq!(deleted_state.id, state_id);

        // Should be gone (second delete returns None)
        let gone = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(gone.is_none());

        // Delete again should return None
        let deleted_again = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(deleted_again.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_auth_states() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // Insert a fresh state — should NOT be deleted
        let fresh_id = format!("test-state-fresh-{}", test_utils::uid());
        insert_auth_state(
            &pool,
            &fresh_id,
            "google",
            "/dashboard",
            "fresh-verifier-minimum-43-chars-long!!",
            None,
        )
        .await
        .unwrap();

        // Insert a "stale" state by backdating its created_at via raw SQL
        let stale_id = format!("test-state-stale-{}", test_utils::uid());
        insert_auth_state(
            &pool,
            &stale_id,
            "github",
            "/login",
            "stale-verifier-minimum-43-chars-long!!",
            None,
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE auth_states SET created_at = NOW() - INTERVAL '20 minutes' WHERE id = $1",
        )
        .bind(&stale_id)
        .execute(&pool)
        .await
        .unwrap();

        // Run cleanup
        let deleted = cleanup_expired_auth_states(&pool).await.unwrap();
        assert_eq!(deleted, 1);

        // Fresh state should still exist
        let fresh = delete_auth_state(&pool, &fresh_id).await.unwrap();
        assert!(fresh.is_some());

        // Stale state should be gone
        let stale = delete_auth_state(&pool, &stale_id).await.unwrap();
        assert!(stale.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_auth_states_empty_table() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let deleted = cleanup_expired_auth_states(&pool).await.unwrap();
        assert_eq!(deleted, 0);
    }

    #[tokio::test]
    async fn test_insert_and_get_oauth_account() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        // Create a user first
        let user = insert_user(
            &pool,
            &format!("oauthuser_{u}"),
            "OAuth User",
            &format!("oauth{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Link an OAuth account
        let account = insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("oauth{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(account.provider, "google");
        assert_eq!(account.provider_user_id, format!("google-{u}"));
        assert_eq!(account.user_id, user.id);

        // Lookup by provider
        let found = get_oauth_account_by_provider(&pool, "google", &format!("google-{u}"))
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, account.id);

        // Lookup by email
        let found_by_email = get_oauth_account_by_email(&pool, &format!("oauth{u}@example.com"))
            .await
            .unwrap();
        assert!(found_by_email.is_some());
    }

    #[tokio::test]
    async fn test_update_oauth_last_used() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("updateuser_{u}"),
            "Update User",
            &format!("update{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let account = insert_oauth_account(
            &pool,
            user.id,
            "github",
            &format!("github-{u}"),
            Some(&format!("update{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        let before = account.last_used_at;

        // Small delay to ensure timestamp difference
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        update_oauth_last_used(&pool, account.id).await.unwrap();

        let updated = get_oauth_account_by_provider(&pool, "github", &format!("github-{u}"))
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_used_at >= before);
    }

    #[tokio::test]
    async fn test_update_user_profile() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("profileuser_{u}"),
            "Original Name",
            &format!("profile{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let updated = update_user_profile(&pool, user.id, "New Name", Some("Hello world"))
            .await
            .unwrap();

        assert_eq!(updated.id, user.id);
        assert_eq!(updated.display_name, "New Name");
        assert_eq!(updated.bio, Some("Hello world".to_string()));

        // Verify persisted
        let found = get_user_by_id(&pool, user.id).await.unwrap().unwrap();
        assert_eq!(found.display_name, "New Name");
        assert_eq!(found.bio, Some("Hello world".to_string()));
    }

    #[tokio::test]
    async fn test_update_user_profile_clears_bio() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("clearbio_{u}"),
            "Clear Bio User",
            &format!("clearbio{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let updated = update_user_profile(&pool, user.id, "Still Name", None)
            .await
            .unwrap();

        assert_eq!(updated.bio, None);
    }

    #[tokio::test]
    async fn test_update_user_profile_nonexistent() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let fake_id = Uuid::nil();
        let result = update_user_profile(&pool, fake_id, "No One", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_username_success() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("user_{u}"),
            "Test User",
            &format!("test{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let new_username = format!("newuser_{u}");
        let updated = update_username(&pool, user.id, &new_username)
            .await
            .unwrap();

        assert_eq!(updated.username, new_username);
        assert_eq!(updated.id, user.id);

        // Verify persisted
        let found = get_user_by_id(&pool, user.id).await.unwrap().unwrap();
        assert_eq!(found.username, new_username);
    }

    #[tokio::test]
    async fn test_update_username_taken() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user1 = insert_user(
            &pool,
            &format!("user1_{u}"),
            "User One",
            &format!("user1{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let user2 = insert_user(
            &pool,
            &format!("user2_{u}"),
            "User Two",
            &format!("user2{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // user2 tries to take user1's username
        let result = update_username(&pool, user2.id, &user1.username).await;
        assert!(matches!(result, Err(DbError::UsernameTaken)));
    }

    #[tokio::test]
    async fn test_update_username_same_as_current() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("same_{u}"),
            "Same User",
            &format!("same{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Setting username to the same value should succeed (no-op)
        let updated = update_username(&pool, user.id, &user.username)
            .await
            .unwrap();

        assert_eq!(updated.username, user.username);
    }

    #[tokio::test]
    async fn test_update_username_nonexistent_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let fake_id = Uuid::nil();
        let result = update_username(&pool, fake_id, "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_oauth_accounts_by_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("listuser_{u}"),
            "List User",
            &format!("list{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Initially empty
        let accounts = get_oauth_accounts_by_user(&pool, user.id).await.unwrap();
        assert!(accounts.is_empty());

        // Insert two accounts
        insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("list{u}@google.com")),
            None,
            None,
        )
        .await
        .unwrap();
        insert_oauth_account(
            &pool,
            user.id,
            "github",
            &format!("github-{u}"),
            Some(&format!("list{u}@github.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Should return both, ordered by provider
        let accounts = get_oauth_accounts_by_user(&pool, user.id).await.unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].provider, "github");
        assert_eq!(accounts[1].provider, "google");
        assert_eq!(accounts[0].email, Some(format!("list{u}@github.com")));
        assert_eq!(accounts[1].email, Some(format!("list{u}@google.com")));
    }

    #[tokio::test]
    async fn test_delete_oauth_account() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("deluser_{u}"),
            "Del User",
            &format!("del{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let account = insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("del{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Delete succeeds
        delete_oauth_account(&pool, account.id, user.id)
            .await
            .unwrap();

        // Verify it's gone
        let accounts = get_oauth_accounts_by_user(&pool, user.id).await.unwrap();
        assert!(accounts.is_empty());
    }

    #[tokio::test]
    async fn test_delete_oauth_account_wrong_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("wronguser_{u}"),
            "Wrong User",
            &format!("wrong{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let account = insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("wrong{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Try deleting with a different user_id — should fail
        let wrong_user_id = Uuid::new_v4();
        let result = delete_oauth_account(&pool, account.id, wrong_user_id).await;
        assert!(result.is_err());

        // Account should still exist
        let accounts = get_oauth_accounts_by_user(&pool, user.id).await.unwrap();
        assert_eq!(accounts.len(), 1);
    }

    #[tokio::test]
    async fn test_count_oauth_accounts() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("countuser_{u}"),
            "Count User",
            &format!("count{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Initially zero
        let count = count_oauth_accounts(&pool, user.id).await.unwrap();
        assert_eq!(count, 0);

        // Insert one
        insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("count{u}@example.com")),
            None,
            None,
        )
        .await
        .unwrap();

        let count = count_oauth_accounts(&pool, user.id).await.unwrap();
        assert_eq!(count, 1);

        // Insert another
        insert_oauth_account(
            &pool,
            user.id,
            "github",
            &format!("github-{u}"),
            Some(&format!("count{u}@github.com")),
            None,
            None,
        )
        .await
        .unwrap();

        let count = count_oauth_accounts(&pool, user.id).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_delete_user_cascades_oauth_accounts() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("cascade_{u}"),
            "Cascade User",
            &format!("cascade{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Insert two OAuth accounts
        insert_oauth_account(
            &pool,
            user.id,
            "google",
            &format!("google-{u}"),
            Some(&format!("cascade{u}@google.com")),
            None,
            None,
        )
        .await
        .unwrap();
        insert_oauth_account(
            &pool,
            user.id,
            "github",
            &format!("github-{u}"),
            Some(&format!("cascade{u}@github.com")),
            None,
            None,
        )
        .await
        .unwrap();

        // Verify both exist
        let accounts = get_oauth_accounts_by_user(&pool, user.id).await.unwrap();
        assert_eq!(accounts.len(), 2);

        // Delete the user
        delete_user(&pool, user.id).await.unwrap();

        // User should be gone
        let found = get_user_by_id(&pool, user.id).await.unwrap();
        assert!(found.is_none());

        // OAuth accounts should have been cascaded
        let accounts = get_oauth_accounts_by_user(&pool, user.id).await.unwrap();
        assert!(accounts.is_empty());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_user() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let fake_id = Uuid::nil();
        let result = delete_user(&pool, fake_id).await;
        assert!(result.is_err());
    }

    // ── Recipe tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_insert_recipe() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("recipeuser_{u}"),
            "Recipe User",
            &format!("recipe{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Pancakes",
            Some("Fluffy buttermilk pancakes"),
            None,
            Some(10),
            Some(15),
            Some(4),
            &[],
            &[RecipeStep {
                text: "Mix batter, cook on griddle".to_string(),
                sub_steps: vec![],
                photos: Vec::new(),
            }],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        assert_eq!(recipe.title, "Pancakes");
        assert_eq!(recipe.user_id, user.id);
        assert_eq!(
            recipe.description,
            Some("Fluffy buttermilk pancakes".to_string())
        );
        assert_eq!(recipe.prep_time_minutes, Some(10));
        assert_eq!(recipe.cook_time_minutes, Some(15));
        assert_eq!(recipe.servings, Some(4));
        assert_eq!(recipe.ingredients, Vec::<RecipeIngredient>::new());
        assert_eq!(
            recipe.instructions,
            vec![RecipeStep {
                text: "Mix batter, cook on griddle".to_string(),
                sub_steps: vec![],
                photos: Vec::new(),
            }]
        );
    }

    #[tokio::test]
    async fn test_get_recipe_by_id() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("getrecipeuser_{u}"),
            "Get Recipe User",
            &format!("getrecipe{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Toast",
            None,
            None,
            None,
            Some(5),
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let found = get_recipe_by_id(&pool, recipe.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Toast");

        // Nonexistent recipe
        let not_found = get_recipe_by_id(&pool, Uuid::nil()).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_get_recipe_by_id_and_owner() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("owneruser_{u}"),
            "Owner User",
            &format!("owner{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Secret Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Correct owner can access
        let found = get_recipe_by_id_and_owner(&pool, recipe.id, user.id)
            .await
            .unwrap();
        assert_eq!(found.id, recipe.id);

        // Wrong owner cannot access
        let wrong_user_id = Uuid::new_v4();
        let result = get_recipe_by_id_and_owner(&pool, recipe.id, wrong_user_id).await;
        assert!(matches!(result, Err(DbError::RecipeNotFound(_))));

        // Nonexistent recipe
        let fake_id = Uuid::nil();
        let result = get_recipe_by_id_and_owner(&pool, fake_id, user.id).await;
        assert!(matches!(result, Err(DbError::RecipeNotFound(_))));
    }

    #[tokio::test]
    async fn test_get_recipes_by_owner() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("listuser_{u}"),
            "List User",
            &format!("list{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Initially empty
        let recipes = get_recipes_by_owner(&pool, user.id).await.unwrap();
        assert!(recipes.is_empty());

        // Insert two recipes
        insert_recipe(
            &pool,
            user.id,
            "Recipe A",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        insert_recipe(
            &pool,
            user.id,
            "Recipe B",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let recipes = get_recipes_by_owner(&pool, user.id).await.unwrap();
        assert_eq!(recipes.len(), 2);
        // Ordered by created_at DESC, so Recipe B comes first
        assert_eq!(recipes[0].title, "Recipe B");
        assert_eq!(recipes[1].title, "Recipe A");
    }

    #[tokio::test]
    async fn test_update_recipe() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("updateuser_{u}"),
            "Update User",
            &format!("update{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Original Title",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let original_updated_at = recipe.updated_at;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let updated = update_recipe(
            &pool,
            recipe.id,
            user.id,
            "Updated Title",
            Some("New description"),
            None,
            Some(20),
            Some(30),
            Some(6),
            &[],
            &[RecipeStep {
                text: "New instructions".to_string(),
                sub_steps: vec![],
                photos: Vec::new(),
            }],
            &[],
            &[],
            None,
        )
        .await
        .unwrap();

        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.description, Some("New description".to_string()));
        assert_eq!(updated.prep_time_minutes, Some(20));
        assert_eq!(updated.cook_time_minutes, Some(30));
        assert_eq!(updated.servings, Some(6));
        assert_eq!(
            updated.instructions,
            vec![RecipeStep {
                text: "New instructions".to_string(),
                sub_steps: vec![],
                photos: Vec::new(),
            }]
        );
        assert!(updated.updated_at >= original_updated_at);

        // Update nonexistent recipe
        let fake_id = Uuid::nil();
        let result = update_recipe(
            &pool,
            fake_id,
            user.id,
            "X",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .await;
        assert!(matches!(result, Err(DbError::RecipeNotFound(_))));

        // Update another user's recipe — should fail (ownership enforced at DB layer)
        let other_user = insert_user(
            &pool,
            &format!("otheruser_{u}"),
            "Other User",
            &format!("other{u}@example.com"),
            None,
        )
        .await
        .unwrap();
        let result = update_recipe(
            &pool,
            recipe.id,
            other_user.id,
            "Hacked",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .await;
        assert!(matches!(result, Err(DbError::RecipeNotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_recipe() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("deluser_{u}"),
            "Delete User",
            &format!("del{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "To Delete",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Delete another user's recipe — should fail (ownership enforced at DB layer)
        let other_user = insert_user(
            &pool,
            &format!("otherdeluser_{u}"),
            "Other Delete User",
            &format!("otherdel{u}@example.com"),
            None,
        )
        .await
        .unwrap();
        let result = delete_recipe(&pool, recipe.id, other_user.id).await;
        assert!(matches!(result, Err(DbError::RecipeNotFound(_))));

        // Delete succeeds with correct owner
        delete_recipe(&pool, recipe.id, user.id).await.unwrap();

        // Verify it's gone
        let found = get_recipe_by_id(&pool, recipe.id).await.unwrap();
        assert!(found.is_none());

        // Delete nonexistent recipe
        let result = delete_recipe(&pool, recipe.id, user.id).await;
        assert!(matches!(result, Err(DbError::RecipeNotFound(_))));
    }

    #[tokio::test]
    async fn test_insert_and_get_recipe_tags() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("taguser_{u}"),
            "Tag User",
            &format!("tag{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Tagged Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Initially no tags
        let tags = get_recipe_tags(&pool, recipe.id).await.unwrap();
        assert!(tags.is_empty());

        // Insert tags
        let tags_to_insert = vec![
            "breakfast".to_string(),
            "quick".to_string(),
            "easy".to_string(),
        ];
        {
            let mut tx = pool.begin().await.unwrap();
            insert_recipe_tags(&mut tx, recipe.id, &tags_to_insert)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let tags = get_recipe_tags(&pool, recipe.id).await.unwrap();
        assert_eq!(tags.len(), 3);
        // Ordered by tag alphabetically
        assert_eq!(tags[0].tag, "breakfast");
        assert_eq!(tags[1].tag, "easy");
        assert_eq!(tags[2].tag, "quick");

        // Inserting again replaces existing tags
        let new_tags = vec!["dinner".to_string()];
        {
            let mut tx = pool.begin().await.unwrap();
            insert_recipe_tags(&mut tx, recipe.id, &new_tags)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let tags = get_recipe_tags(&pool, recipe.id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag, "dinner");

        // Empty tags clears all
        {
            let mut tx = pool.begin().await.unwrap();
            insert_recipe_tags(&mut tx, recipe.id, &[]).await.unwrap();
            tx.commit().await.unwrap();
        }

        let tags = get_recipe_tags(&pool, recipe.id).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_recipe_tags_cascade_on_delete() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("cascadeuser_{u}"),
            "Cascade User",
            &format!("cascade{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Cascade Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        {
            let mut tx = pool.begin().await.unwrap();
            insert_recipe_tags(&mut tx, recipe.id, &tags).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Tags exist
        let tags = get_recipe_tags(&pool, recipe.id).await.unwrap();
        assert_eq!(tags.len(), 2);

        // Delete recipe — tags should cascade
        delete_recipe(&pool, recipe.id, user.id).await.unwrap();

        // Tags are gone
        let tags = get_recipe_tags(&pool, recipe.id).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_get_recipes_by_owner_paginated() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("paguser_{u}"),
            "Paginated User",
            &format!("pag{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Insert 4 recipes with small delays for ordering
        for i in 1..=4 {
            insert_recipe(
                &pool,
                user.id,
                &format!("Recipe {i}"),
                None,
                None,
                None,
                None,
                None,
                &[],
                &[],
                &[],
                &[],
                "private",
            )
            .await
            .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // Page 1: limit 2, offset 0
        let page1 = get_recipes_by_owner_paginated(&pool, user.id, 2, 0)
            .await
            .unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].title, "Recipe 4");
        assert_eq!(page1[1].title, "Recipe 3");

        // Page 2: limit 2, offset 2
        let page2 = get_recipes_by_owner_paginated(&pool, user.id, 2, 2)
            .await
            .unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].title, "Recipe 2");
        assert_eq!(page2[1].title, "Recipe 1");

        // Page 3: limit 2, offset 4 — empty
        let page3 = get_recipes_by_owner_paginated(&pool, user.id, 2, 4)
            .await
            .unwrap();
        assert!(page3.is_empty());
    }

    #[tokio::test]
    async fn test_count_recipes_by_owner() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("countuser_{u}"),
            "Count User",
            &format!("count{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Initially zero
        let count = count_recipes_by_owner(&pool, user.id).await.unwrap();
        assert_eq!(count, 0);

        // Insert two recipes
        insert_recipe(
            &pool,
            user.id,
            "Count A",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Count B",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let count = count_recipes_by_owner(&pool, user.id).await.unwrap();
        assert_eq!(count, 2);

        // Different user has zero
        let other_user = insert_user(
            &pool,
            &format!("other_{u}"),
            "Other User",
            &format!("other{u}@example.com"),
            None,
        )
        .await
        .unwrap();
        let other_count = count_recipes_by_owner(&pool, other_user.id).await.unwrap();
        assert_eq!(other_count, 0);
    }

    // ── Visibility tests (CP8) ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_insert_recipe_with_visibility() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("visuser_{u}"),
            "Visibility User",
            &format!("vis{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let r_public = insert_recipe(
            &pool,
            user.id,
            "Public",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        assert_eq!(r_public.visibility, "public");

        let r_unlisted = insert_recipe(
            &pool,
            user.id,
            "Unlisted",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "unlisted",
        )
        .await
        .unwrap();
        assert_eq!(r_unlisted.visibility, "unlisted");

        let r_private = insert_recipe(
            &pool,
            user.id,
            "Private",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        assert_eq!(r_private.visibility, "private");
    }

    #[tokio::test]
    async fn test_get_public_recipes_paginated() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user_a = insert_user(
            &pool,
            &format!("puba_{u}"),
            "Public A",
            &format!("puba{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let user_b = insert_user(
            &pool,
            &format!("pubb_{u}"),
            "Public B",
            &format!("pubb{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // User A: 2 public + 1 private
        insert_recipe(
            &pool,
            user_a.id,
            "A Public 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user_a.id,
            "A Public 2",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user_a.id,
            "A Private",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // User B: 1 public
        insert_recipe(
            &pool,
            user_b.id,
            "B Public",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();

        // All public: 3 recipes
        let all = get_public_recipes_paginated(&pool, 10, 0).await.unwrap();
        assert_eq!(all.len(), 3);
        for r in &all {
            assert_eq!(r.visibility, "public");
        }

        // Pagination: limit 2, offset 0
        let page1 = get_public_recipes_paginated(&pool, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        // Pagination: limit 2, offset 2
        let page2 = get_public_recipes_paginated(&pool, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 1);

        // Pagination: limit 2, offset 5 — empty
        let page3 = get_public_recipes_paginated(&pool, 2, 5).await.unwrap();
        assert!(page3.is_empty());
    }

    #[tokio::test]
    async fn test_count_public_recipes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("cpub_{u}"),
            "Count Public",
            &format!("cpub{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        insert_recipe(
            &pool,
            user.id,
            "Pub 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Pub 2",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Priv 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Unlist 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "unlisted",
        )
        .await
        .unwrap();

        let count = count_public_recipes(&pool).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_distinct_public_tags() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("dpt_{u}"),
            "Distinct Tags User",
            &format!("dpt{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Create two public recipes with overlapping and unique tags
        let r1 = insert_recipe(
            &pool,
            user.id,
            "Public Recipe 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        let r2 = insert_recipe(
            &pool,
            user.id,
            "Public Recipe 2",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        // Private recipe — tags should NOT appear
        let r_private = insert_recipe(
            &pool,
            user.id,
            "Private Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Add tags: r1 gets "breakfast" and "quick", r2 gets "quick" and "dinner"
        // Private recipe gets "secret"
        {
            let mut tx = pool.begin().await.unwrap();
            insert_recipe_tags(
                &mut tx,
                r1.id,
                &["breakfast".to_string(), "quick".to_string()],
            )
            .await
            .unwrap();
            insert_recipe_tags(&mut tx, r2.id, &["quick".to_string(), "dinner".to_string()])
                .await
                .unwrap();
            insert_recipe_tags(
                &mut tx,
                r_private.id,
                &["secret".to_string(), "breakfast".to_string()],
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        // Distinct public tags should be: breakfast, dinner, quick (alphabetical)
        let tags = get_distinct_public_tags(&pool).await.unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0], "breakfast");
        assert_eq!(tags[1], "dinner");
        assert_eq!(tags[2], "quick");
        // "secret" from private recipe should NOT appear
        assert!(!tags.contains(&"secret".to_string()));
    }

    #[tokio::test]
    async fn test_get_distinct_public_tags_empty() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("dpte_{u}"),
            "Empty Tags User",
            &format!("dpte{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Only private recipes — no public tags
        insert_recipe(
            &pool,
            user.id,
            "Private Only",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let tags = get_distinct_public_tags(&pool).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_get_recipe_by_id_public() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("gr_{u}"),
            "Get Recipe",
            &format!("gr{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let r_public = insert_recipe(
            &pool,
            user.id,
            "Public",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        let r_unlisted = insert_recipe(
            &pool,
            user.id,
            "Unlisted",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "unlisted",
        )
        .await
        .unwrap();
        let r_private = insert_recipe(
            &pool,
            user.id,
            "Private",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Public is accessible
        let found = get_recipe_by_id_public(&pool, r_public.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Public");

        // Unlisted is accessible
        let found = get_recipe_by_id_public(&pool, r_unlisted.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Unlisted");

        // Private is NOT accessible
        let found = get_recipe_by_id_public(&pool, r_private.id).await.unwrap();
        assert!(found.is_none());

        // Non-existent ID returns None
        let found = get_recipe_by_id_public(&pool, uuid::Uuid::new_v4())
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_get_user_public_recipes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("gupr_{u}"),
            "Get User Public",
            &format!("gupr{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        insert_recipe(
            &pool,
            user.id,
            "Pub 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Pub 2",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Priv 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Only public recipes returned
        let recipes = get_user_public_recipes(&pool, user.id, 10, 0)
            .await
            .unwrap();
        assert_eq!(recipes.len(), 2);

        // Pagination
        let page1 = get_user_public_recipes(&pool, user.id, 1, 0).await.unwrap();
        assert_eq!(page1.len(), 1);

        let page2 = get_user_public_recipes(&pool, user.id, 1, 1).await.unwrap();
        assert_eq!(page2.len(), 1);

        let page3 = get_user_public_recipes(&pool, user.id, 1, 2).await.unwrap();
        assert!(page3.is_empty());
    }

    #[tokio::test]
    async fn test_count_user_public_recipes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("cupr_{u}"),
            "Count User Public",
            &format!("cupr{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        insert_recipe(
            &pool,
            user.id,
            "Pub 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Priv 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        insert_recipe(
            &pool,
            user.id,
            "Unlist 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "unlisted",
        )
        .await
        .unwrap();

        let count = count_user_public_recipes(&pool, user.id).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_get_user_by_username() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("gub_{u}"),
            "Get User By",
            &format!("gub{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Existing user is found
        let found = get_user_by_username(&pool, &format!("gub_{u}"))
            .await
            .unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, user.id);
        assert_eq!(found.display_name, "Get User By");

        // Non-existent user returns None
        let not_found = get_user_by_username(&pool, "nonexistent_user_xyz")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_username_exists() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let username = format!("ue_{u}");
        insert_user(
            &pool,
            &username,
            "Exists User",
            &format!("ue{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        assert!(username_exists(&pool, &username).await.unwrap());
        assert!(!username_exists(&pool, "nonexistent_xyz").await.unwrap());
    }

    #[tokio::test]
    async fn test_update_recipe_visibility() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("urv_{u}"),
            "Update Vis",
            &format!("urv{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Vis Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        assert_eq!(recipe.visibility, "private");

        // Update to public
        let updated = update_recipe(
            &pool,
            recipe.id,
            user.id,
            "Vis Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            Some("public"),
        )
        .await
        .unwrap();
        assert_eq!(updated.visibility, "public");

        // Update to unlisted
        let updated = update_recipe(
            &pool,
            recipe.id,
            user.id,
            "Vis Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            Some("unlisted"),
        )
        .await
        .unwrap();
        assert_eq!(updated.visibility, "unlisted");

        // Update with None — visibility unchanged
        let updated = update_recipe(
            &pool,
            recipe.id,
            user.id,
            "Renamed",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .await
        .unwrap();
        assert_eq!(updated.visibility, "unlisted");
        assert_eq!(updated.title, "Renamed");
    }

    #[tokio::test]
    async fn visibility_only_update_validates_value_and_enforces_ownership() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let suffix = test_utils::uid();
        let owner = insert_user(
            &pool,
            &format!("visibility_owner_{suffix}"),
            "Visibility Owner",
            &format!("visibility_owner_{suffix}@example.test"),
            None,
        )
        .await
        .unwrap();
        let other_user = insert_user(
            &pool,
            &format!("visibility_other_{suffix}"),
            "Visibility Other",
            &format!("visibility_other_{suffix}@example.test"),
            None,
        )
        .await
        .unwrap();
        let recipe = insert_recipe(
            &pool,
            owner.id,
            "Visibility authority",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let updated = update_recipe_visibility(&pool, recipe.id, owner.id, "public")
            .await
            .unwrap();
        assert_eq!(updated.visibility, "public");
        assert!(matches!(
            update_recipe_visibility(&pool, recipe.id, other_user.id, "private").await,
            Err(DbError::RecipeNotFound(id)) if id == recipe.id
        ));
        assert!(matches!(
            update_recipe_visibility(&pool, recipe.id, owner.id, "friends").await,
            Err(DbError::InvalidRecipeSnapshot(_))
        ));
    }

    #[tokio::test]
    async fn test_visibility_check_constraint() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("vcc_{u}"),
            "Constraint Check",
            &format!("vcc{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Invalid visibility should fail
        let result: Result<i64, sqlx::Error> = sqlx::query_scalar(
            "INSERT INTO recipes (user_id, title, visibility) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(user.id)
        .bind("Bad Recipe")
        .bind("invalid_visibility")
        .fetch_one(&pool)
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cross_user_public_access() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user_a = insert_user(
            &pool,
            &format!("cua_{u}"),
            "Cross A",
            &format!("cua{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let _user_b = insert_user(
            &pool,
            &format!("cub_{u}"),
            "Cross B",
            &format!("cub{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // User A creates public recipe — accessible via public lookup
        let r_public = insert_recipe(
            &pool,
            user_a.id,
            "Public",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();
        let found = get_recipe_by_id_public(&pool, r_public.id).await.unwrap();
        assert!(found.is_some());

        // User A creates private recipe — NOT accessible via public lookup
        let r_private = insert_recipe(
            &pool,
            user_a.id,
            "Private",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        let found = get_recipe_by_id_public(&pool, r_private.id).await.unwrap();
        assert!(found.is_none());

        // User A creates unlisted recipe — accessible via public lookup
        let r_unlisted = insert_recipe(
            &pool,
            user_a.id,
            "Unlisted",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "unlisted",
        )
        .await
        .unwrap();
        let found = get_recipe_by_id_public(&pool, r_unlisted.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_get_public_recipe_tags() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("gptr_{u}"),
            "Public Tags User",
            &format!("gptr{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Public recipe with 3 tags
        let r_public = insert_recipe(
            &pool,
            user.id,
            "Public Tags",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "public",
        )
        .await
        .unwrap();

        // Unlisted recipe with 2 tags
        let r_unlisted = insert_recipe(
            &pool,
            user.id,
            "Unlisted Tags",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "unlisted",
        )
        .await
        .unwrap();

        // Private recipe with 1 tag — should NOT be accessible
        let r_private = insert_recipe(
            &pool,
            user.id,
            "Private Tags",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        {
            let mut tx = pool.begin().await.unwrap();
            insert_recipe_tags(
                &mut tx,
                r_public.id,
                &[
                    "breakfast".to_string(),
                    "quick".to_string(),
                    "easy".to_string(),
                ],
            )
            .await
            .unwrap();
            insert_recipe_tags(
                &mut tx,
                r_unlisted.id,
                &["dinner".to_string(), "quick".to_string()],
            )
            .await
            .unwrap();
            insert_recipe_tags(&mut tx, r_private.id, &["secret".to_string()])
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        // Public recipe tags — all 3 returned alphabetically
        let tags = get_public_recipe_tags(&pool, r_public.id).await.unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].tag, "breakfast");
        assert_eq!(tags[1].tag, "easy");
        assert_eq!(tags[2].tag, "quick");

        // Unlisted recipe tags — both returned
        let tags = get_public_recipe_tags(&pool, r_unlisted.id).await.unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].tag, "dinner");
        assert_eq!(tags[1].tag, "quick");

        // Private recipe tags — empty (no existence leakage)
        let tags = get_public_recipe_tags(&pool, r_private.id).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_get_public_recipe_tags_missing() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // Non-existent recipe returns empty vec (no error, no existence leakage)
        let tags = get_public_recipe_tags(&pool, Uuid::nil()).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_insert_recipe_with_commentary() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("commuser_{u}"),
            "Commentary User",
            &format!("comm{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let recipe = insert_recipe(
            &pool,
            user.id,
            "Recipe with Commentary",
            Some("A brief description"),
            Some("This is the author's commentary about the recipe."),
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // Verify commentary is stored
        assert_eq!(
            recipe.commentary,
            Some("This is the author's commentary about the recipe.".to_string())
        );

        // Verify it round-trips via get_recipe_by_id
        let fetched = get_recipe_by_id(&pool, recipe.id).await.unwrap().unwrap();
        assert_eq!(
            fetched.commentary,
            Some("This is the author's commentary about the recipe.".to_string())
        );

        // Verify None commentary when not provided
        let recipe_no_comm = insert_recipe(
            &pool,
            user.id,
            "Recipe without Commentary",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        assert_eq!(recipe_no_comm.commentary, None);
    }

    #[tokio::test]
    async fn test_update_recipe_commentary() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("commup_{u}"),
            "Commentary Update User",
            &format!("commup{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Insert recipe without commentary
        let recipe = insert_recipe(
            &pool,
            user.id,
            "Original Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();
        assert_eq!(recipe.commentary, None);

        // Update to add commentary
        let updated = update_recipe(
            &pool,
            recipe.id,
            user.id,
            "Original Recipe",
            None,
            Some("Added commentary after update."),
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            updated.commentary,
            Some("Added commentary after update.".to_string())
        );

        // Update to clear commentary (set to None)
        let cleared = update_recipe(
            &pool,
            recipe.id,
            user.id,
            "Original Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            None,
        )
        .await
        .unwrap();
        assert_eq!(cleared.commentary, None);

        // Verify cleared commentary persists via get_recipe_by_id
        let fetched = get_recipe_by_id(&pool, recipe.id).await.unwrap().unwrap();
        assert_eq!(fetched.commentary, None);
    }

    // ── get_all_image_urls tests ──────────────────────────────────────────

    #[tokio::test]
    async fn test_get_all_image_urls_empty_recipes() {
        let (_db, pool) = test_utils::setup_test_db().await;

        // No recipes at all — should return empty vec
        let urls = get_all_image_urls(&pool).await.unwrap();
        assert!(urls.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_image_urls_recipe_with_no_images() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("imguser_{u}"),
            "Image User",
            &format!("img{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        // Recipe with empty images array (the default)
        insert_recipe(
            &pool,
            user.id,
            "No Images",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        let urls = get_all_image_urls(&pool).await.unwrap();
        assert!(urls.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_image_urls_single_recipe_multiple_images() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("imguser2_{u}"),
            "Image User 2",
            &format!("img2{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let images = vec![
            RecipeImage {
                url: format!("https://storage.example.com/recipes/{u}_full.webp"),
                thumb_url: Some(format!(
                    "https://storage.example.com/recipes/{u}_thumb.webp"
                )),
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
            RecipeImage {
                url: format!("https://storage.example.com/recipes/{u}_no_thumb.webp"),
                thumb_url: None,
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
            RecipeImage {
                url: format!("https://storage.example.com/recipes/{u}_2_full.webp"),
                thumb_url: Some(format!(
                    "https://storage.example.com/recipes/{u}_2_thumb.webp"
                )),
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
        ];
        let instructions = vec![RecipeStep {
            text: "Prep the filling".to_string(),
            photos: vec![
                RecipeImage {
                    url: format!("https://storage.example.com/recipes/{u}_step_full.webp"),
                    thumb_url: Some(format!(
                        "https://storage.example.com/recipes/{u}_step_thumb.webp"
                    )),
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                },
                RecipeImage {
                    url: format!("https://storage.example.com/recipes/{u}_step_extra_1_full.webp"),
                    thumb_url: Some(format!(
                        "https://storage.example.com/recipes/{u}_step_extra_1_thumb.webp"
                    )),
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                },
                RecipeImage {
                    url: format!("https://storage.example.com/recipes/{u}_step_extra_2_full.webp"),
                    thumb_url: None,
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                },
            ],
            sub_steps: vec![RecipeStep {
                text: "Check the texture".to_string(),
                photos: vec![RecipeImage {
                    url: format!("https://storage.example.com/recipes/{u}_nested_step_full.webp"),
                    thumb_url: None,
                    file_name: None,
                    file_size: None,
                    edits: Vec::new(),
                }],
                sub_steps: Vec::new(),
            }],
        }];

        insert_recipe(
            &pool,
            user.id,
            "Image Recipe",
            None,
            None,
            None,
            None,
            None,
            &[],
            &instructions,
            &[],
            &images,
            "private",
        )
        .await
        .unwrap();

        let urls = get_all_image_urls(&pool).await.unwrap();
        assert_eq!(urls.len(), 11);
        // Results may be in any order, so sort for comparison
        let mut actual = urls;
        actual.sort();
        let mut expected = vec![
            format!("https://storage.example.com/recipes/{u}_2_full.webp"),
            format!("https://storage.example.com/recipes/{u}_2_thumb.webp"),
            format!("https://storage.example.com/recipes/{u}_full.webp"),
            format!("https://storage.example.com/recipes/{u}_nested_step_full.webp"),
            format!("https://storage.example.com/recipes/{u}_step_extra_1_full.webp"),
            format!("https://storage.example.com/recipes/{u}_step_extra_1_thumb.webp"),
            format!("https://storage.example.com/recipes/{u}_step_extra_2_full.webp"),
            format!("https://storage.example.com/recipes/{u}_step_full.webp"),
            format!("https://storage.example.com/recipes/{u}_step_thumb.webp"),
            format!("https://storage.example.com/recipes/{u}_thumb.webp"),
            format!("https://storage.example.com/recipes/{u}_no_thumb.webp"),
        ];
        expected.sort();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_get_all_image_urls_deduplicates_across_recipes() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("imguser3_{u}"),
            "Image User 3",
            &format!("img3{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        let shared_url = format!("https://storage.example.com/recipes/shared_{u}.webp");

        // Recipe 1 has shared_url + unique1
        let images1 = vec![
            RecipeImage {
                url: shared_url.clone(),
                thumb_url: Some(format!(
                    "https://storage.example.com/recipes/shared_{u}_thumb.webp"
                )),
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
            RecipeImage {
                url: format!("https://storage.example.com/recipes/unique1_{u}.webp"),
                thumb_url: None,
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
        ];
        insert_recipe(
            &pool,
            user.id,
            "Recipe 1",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &images1,
            "private",
        )
        .await
        .unwrap();

        // Recipe 2 has shared_url + unique2
        let images2 = vec![
            RecipeImage {
                url: shared_url.clone(),
                thumb_url: Some(format!(
                    "https://storage.example.com/recipes/shared_{u}_thumb.webp"
                )),
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
            RecipeImage {
                url: format!("https://storage.example.com/recipes/unique2_{u}.webp"),
                thumb_url: None,
                file_name: None,
                file_size: None,
                edits: Vec::new(),
            },
        ];
        insert_recipe(
            &pool,
            user.id,
            "Recipe 2",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &images2,
            "private",
        )
        .await
        .unwrap();

        // Recipe 3 has no images
        insert_recipe(
            &pool,
            user.id,
            "Recipe 3 No Images",
            None,
            None,
            None,
            None,
            None,
            &[],
            &[],
            &[],
            &[],
            "private",
        )
        .await
        .unwrap();

        // DISTINCT should deduplicate: 4 unique URLs, not 6
        let urls = get_all_image_urls(&pool).await.unwrap();
        assert_eq!(urls.len(), 4);
        assert!(urls.contains(&shared_url));
        assert!(urls.contains(&format!(
            "https://storage.example.com/recipes/shared_{u}_thumb.webp"
        )));
        assert!(urls.contains(&format!(
            "https://storage.example.com/recipes/unique1_{u}.webp"
        )));
        assert!(urls.contains(&format!(
            "https://storage.example.com/recipes/unique2_{u}.webp"
        )));
    }

    #[tokio::test]
    async fn test_update_avatar_url_sets_url() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let user = insert_user(
            &pool,
            &format!("avatar_{u}"),
            "Avatar User",
            &format!("avatar{u}@example.com"),
            None,
        )
        .await
        .unwrap();

        assert!(user.avatar_url.is_none());

        let new_url = format!("https://storage.example.com/avatars/{u}.webp");
        let updated = update_avatar_url(&pool, user.id, Some(new_url.clone()))
            .await
            .unwrap();

        assert_eq!(updated.avatar_url, Some(new_url));
    }

    #[tokio::test]
    async fn test_update_avatar_url_removes_url() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let u = test_utils::uid();

        let existing_url = format!("https://storage.example.com/avatars/{u}.webp");
        let user = insert_user(
            &pool,
            &format!("avatar2_{u}"),
            "Avatar User 2",
            &format!("avatar2{u}@example.com"),
            Some(&existing_url),
        )
        .await
        .unwrap();

        assert_eq!(user.avatar_url, Some(existing_url));

        let updated = update_avatar_url(&pool, user.id, None).await.unwrap();

        assert!(updated.avatar_url.is_none());
    }

    #[tokio::test]
    async fn test_update_avatar_url_nonexistent_user() {
        let (_db, pool) = test_utils::setup_test_db().await;

        let result = update_avatar_url(
            &pool,
            uuid::Uuid::nil(),
            Some("https://example.com/avatar.webp".into()),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collection_cover_round_trips_and_is_retained_by_orphan_scan() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let cover = RecipeImage::new(
            "https://storage.example.com/collections/cover-full.webp".to_string(),
            Some("https://storage.example.com/collections/cover-thumb.webp".to_string()),
            crate::types::RecipeImageEdit::default_rect_16x9(),
        )
        .with_file_metadata("cover.webp".to_string(), 42);

        let collection = insert_collection(
            &pool,
            owner.id,
            None,
            "Covered collection",
            Some("A visible cover"),
            CollectionVisibility::Public,
            Some(&cover),
        )
        .await
        .unwrap();
        assert_eq!(collection.image.as_ref(), Some(&cover));

        let owned = get_root_collections(&pool, owner.id).await.unwrap();
        assert_eq!(owned[0].image.as_ref(), Some(&cover));
        let breadcrumbs = get_collection_breadcrumbs(&pool, collection.id, owner.id)
            .await
            .unwrap();
        assert_eq!(breadcrumbs[0].image.as_ref(), Some(&cover));

        let child = insert_collection(
            &pool,
            owner.id,
            Some(collection.id),
            "Covered child",
            None,
            CollectionVisibility::Public,
            Some(&cover),
        )
        .await
        .unwrap();
        let public_contents = get_collection_contents_for_viewer(&pool, collection.id, None, 0, 10)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(public_contents.collection.image.as_ref(), Some(&cover));
        assert_eq!(public_contents.child_collections.len(), 1);
        assert_eq!(public_contents.child_collections[0].id, child.id);
        assert_eq!(
            public_contents.child_collections[0].image.as_ref(),
            Some(&cover)
        );

        let unlisted = insert_collection(
            &pool,
            owner.id,
            None,
            "Unlisted cover",
            None,
            CollectionVisibility::Unlisted,
            Some(&cover),
        )
        .await
        .unwrap();
        assert_eq!(
            get_collection_contents_for_viewer(&pool, unlisted.id, None, 0, 10)
                .await
                .unwrap()
                .unwrap()
                .collection
                .image
                .as_ref(),
            Some(&cover)
        );
        let public = get_public_collections_by_username(&pool, &owner.username, 0, 10)
            .await
            .unwrap();
        assert!(public
            .collections
            .iter()
            .any(|summary| summary.id == collection.id && summary.image.as_ref() == Some(&cover)));

        let urls = get_all_image_urls(&pool).await.unwrap();
        assert!(urls.contains(&cover.url));
        assert!(urls.contains(cover.thumb_url.as_ref().unwrap()));

        let updated = update_collection(
            &pool,
            collection.id,
            owner.id,
            "Covered collection",
            Some("Cover removed"),
            None,
        )
        .await
        .unwrap();
        assert!(updated.image.is_none());
        assert!(
            get_collection_by_id_and_owner(&pool, collection.id, owner.id)
                .await
                .unwrap()
                .unwrap()
                .image
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_collection_recipe_cover_previews_preserve_order_and_viewer_privacy() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let author = test_utils::insert_test_user(&pool).await;
        let collection = insert_collection(
            &pool,
            owner.id,
            None,
            "Recipe mosaic",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();

        let mut malformed_recipe_ids = Vec::new();
        for malformed_images in [
            serde_json::json!([null]),
            serde_json::json!([{}]),
            serde_json::json!([42]),
            serde_json::json!([{"url": 42}]),
        ] {
            let recipe = test_utils::insert_test_recipe(&pool, owner.id, "public").await;
            sqlx::query("UPDATE recipes SET images = $2 WHERE id = $1")
                .bind(recipe.id)
                .bind(malformed_images)
                .execute(&pool)
                .await
                .unwrap();
            test_utils::add_test_recipe_to_collection(&pool, collection.id, owner.id, recipe.id)
                .await;
            malformed_recipe_ids.push(recipe.id);
        }

        let recipe_specs = [
            (owner.id, "private", "owner-private"),
            (author.id, "private", "other-private"),
            (author.id, "public", "public-first"),
            (author.id, "unlisted", "unlisted"),
            (author.id, "public", "public-second"),
            (author.id, "public", "public-fifth"),
        ];
        for (recipe_owner, visibility, image_name) in recipe_specs {
            // Add cross-owner private coverage through a valid historical state:
            // the recipe is public when added, then becomes private afterward.
            let initial_visibility = if recipe_owner != owner.id && visibility == "private" {
                "public"
            } else {
                visibility
            };
            let recipe =
                test_utils::insert_test_recipe(&pool, recipe_owner, initial_visibility).await;
            let image = RecipeImage::new(
                format!("https://storage.example.com/{image_name}_full.webp"),
                Some(format!(
                    "https://storage.example.com/{image_name}_thumb.webp"
                )),
                crate::types::RecipeImageEdit::default_rect_16x9(),
            );
            sqlx::query("UPDATE recipes SET images = $2 WHERE id = $1")
                .bind(recipe.id)
                .bind(serde_json::to_value(vec![image]).unwrap())
                .execute(&pool)
                .await
                .unwrap();
            test_utils::add_test_recipe_to_collection(&pool, collection.id, owner.id, recipe.id)
                .await;
            if initial_visibility != visibility {
                sqlx::query("UPDATE recipes SET visibility = $2 WHERE id = $1")
                    .bind(recipe.id)
                    .bind(visibility)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }

        let owner_summary = get_root_collections(&pool, owner.id).await.unwrap();
        let owner_urls: Vec<&str> = owner_summary[0]
            .preview_images
            .iter()
            .map(|image| image.url.as_str())
            .collect();
        assert_eq!(owner_urls.len(), 4);
        assert!(owner_urls[0].contains("owner-private"));
        assert!(owner_urls[1].contains("public-first"));
        assert!(owner_urls[2].contains("unlisted"));
        assert!(owner_urls[3].contains("public-second"));
        assert!(owner_urls.iter().all(|url| !url.contains("other-private")));

        let public_summary = get_public_collections_by_username(&pool, &owner.username, 0, 10)
            .await
            .unwrap();
        let public_urls: Vec<&str> = public_summary.collections[0]
            .preview_images
            .iter()
            .map(|image| image.url.as_str())
            .collect();
        assert_eq!(public_urls.len(), 3);
        assert!(public_urls[0].contains("public-first"));
        assert!(public_urls[1].contains("public-second"));
        assert!(public_urls[2].contains("public-fifth"));

        // These fixtures exercise summary tolerance only; full recipe rows
        // retain their established valid-image contract.
        for recipe_id in malformed_recipe_ids {
            sqlx::query("DELETE FROM recipes WHERE id = $1")
                .bind(recipe_id)
                .execute(&pool)
                .await
                .unwrap();
        }

        let viewer_contents = get_collection_contents_for_viewer(&pool, collection.id, None, 0, 10)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(viewer_contents.collection.preview_images.len(), 3);
        assert_eq!(
            viewer_contents.collection.preview_images,
            public_summary.collections[0].preview_images
        );
    }

    #[tokio::test]
    async fn test_collection_visibility_defaults_and_owner_updates() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other_owner = test_utils::insert_test_user(&pool).await;

        let private = test_utils::insert_test_collection(&pool, owner.id, None, "Private").await;
        assert_eq!(private.visibility, CollectionVisibility::Private);

        let public = insert_collection(
            &pool,
            owner.id,
            None,
            "Public",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();
        assert_eq!(public.visibility, CollectionVisibility::Public);

        let unlisted = update_collection_visibility(
            &pool,
            public.id,
            owner.id,
            CollectionVisibility::Unlisted,
        )
        .await
        .unwrap();
        assert_eq!(unlisted.visibility, CollectionVisibility::Unlisted);

        assert!(matches!(
            update_collection_visibility(
                &pool,
                public.id,
                other_owner.id,
                CollectionVisibility::Private,
            )
            .await,
            Err(DbError::CollectionNotFound(id)) if id == public.id
        ));

        let invalid =
            sqlx::query("UPDATE collections SET visibility = 'friends-only' WHERE id = $1")
                .bind(public.id)
                .execute(&pool)
                .await
                .unwrap_err();
        assert_eq!(
            invalid
                .as_database_error()
                .and_then(|database_error| database_error.code())
                .as_deref(),
            Some("23514")
        );
    }

    #[tokio::test]
    async fn test_collection_viewer_reads_apply_collection_and_recipe_visibility() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let author = test_utils::insert_test_user(&pool).await;
        let viewer = test_utils::insert_test_user(&pool).await;
        let private_collection =
            test_utils::insert_test_collection(&pool, owner.id, None, "Private collection").await;
        let unlisted_collection = insert_collection(
            &pool,
            owner.id,
            None,
            "Unlisted collection",
            None,
            CollectionVisibility::Unlisted,
            None,
        )
        .await
        .unwrap();
        let public_collection = insert_collection(
            &pool,
            owner.id,
            None,
            "Public collection",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();
        let owner_private_recipe = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let public_recipe = test_utils::insert_test_recipe(&pool, author.id, "public").await;
        let unlisted_recipe = test_utils::insert_test_recipe(&pool, author.id, "unlisted").await;

        for collection_id in [
            private_collection.id,
            unlisted_collection.id,
            public_collection.id,
        ] {
            test_utils::add_test_recipe_to_collection(
                &pool,
                collection_id,
                owner.id,
                owner_private_recipe.id,
            )
            .await;
            test_utils::add_test_recipe_to_collection(
                &pool,
                collection_id,
                owner.id,
                public_recipe.id,
            )
            .await;
            test_utils::add_test_recipe_to_collection(
                &pool,
                collection_id,
                owner.id,
                unlisted_recipe.id,
            )
            .await;
        }

        assert!(
            get_collection_contents_for_viewer(&pool, private_collection.id, None, 0, 20,)
                .await
                .unwrap()
                .is_none()
        );
        assert!(get_collection_contents_for_viewer(
            &pool,
            private_collection.id,
            Some(viewer.id),
            0,
            20,
        )
        .await
        .unwrap()
        .is_none());

        let owner_view =
            get_collection_contents_for_viewer(&pool, private_collection.id, Some(owner.id), 0, 20)
                .await
                .unwrap()
                .unwrap();
        assert!(owner_view.can_edit);
        assert_eq!(owner_view.owner_username, owner.username);
        assert_eq!(owner_view.recipes.total_count, 3);

        let unlisted_view =
            get_collection_contents_for_viewer(&pool, unlisted_collection.id, None, 0, 1)
                .await
                .unwrap()
                .unwrap();
        assert!(!unlisted_view.can_edit);
        assert_eq!(unlisted_view.recipes.total_count, 2);
        assert_eq!(unlisted_view.recipes.recipes.len(), 1);
        assert!(unlisted_view.recipes.has_more);

        let public_view =
            get_collection_contents_for_viewer(&pool, public_collection.id, Some(viewer.id), 0, 20)
                .await
                .unwrap()
                .unwrap();
        assert!(!public_view.can_edit);
        assert_eq!(public_view.recipes.total_count, 1);
        assert_eq!(public_view.recipes.recipes[0].id, public_recipe.id);

        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(public_recipe.id)
            .execute(&pool)
            .await
            .unwrap();
        let hidden_after_change =
            get_collection_contents_for_viewer(&pool, public_collection.id, None, 0, 20)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(hidden_after_change.recipes.total_count, 0);
        assert!(hidden_after_change.recipes.recipes.is_empty());
    }

    #[tokio::test]
    async fn test_collection_share_preview_is_owner_scoped_and_matches_target_visibility() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let author = test_utils::insert_test_user(&pool).await;
        let other_user = test_utils::insert_test_user(&pool).await;
        let collection = test_utils::insert_test_collection(&pool, owner.id, None, "Preview").await;
        let own_private = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let public = test_utils::insert_test_recipe(&pool, author.id, "public").await;
        let unlisted = test_utils::insert_test_recipe(&pool, author.id, "unlisted").await;
        for recipe_id in [own_private.id, public.id, unlisted.id] {
            test_utils::add_test_recipe_to_collection(&pool, collection.id, owner.id, recipe_id)
                .await;
        }

        let public_preview = get_collection_share_preview(
            &pool,
            collection.id,
            owner.id,
            CollectionVisibility::Public,
        )
        .await
        .unwrap();
        assert_eq!(public_preview.total_recipe_count, 3);
        assert_eq!(public_preview.visible_recipe_count, 1);
        assert_eq!(public_preview.hidden_recipe_count, 2);

        let unlisted_preview = get_collection_share_preview(
            &pool,
            collection.id,
            owner.id,
            CollectionVisibility::Unlisted,
        )
        .await
        .unwrap();
        assert_eq!(unlisted_preview.visible_recipe_count, 2);
        assert_eq!(unlisted_preview.hidden_recipe_count, 1);

        assert!(matches!(
            get_collection_share_preview(
                &pool,
                collection.id,
                other_user.id,
                CollectionVisibility::Public,
            )
            .await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
    }

    #[tokio::test]
    async fn test_public_collection_discovery_excludes_unlisted_and_sanitizes_metadata() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let author = test_utils::insert_test_user(&pool).await;
        let private_root =
            test_utils::insert_test_collection(&pool, owner.id, None, "Private root").await;
        let public_below_private = insert_collection(
            &pool,
            owner.id,
            Some(private_root.id),
            "Public below private",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();
        let unlisted = insert_collection(
            &pool,
            owner.id,
            None,
            "Unlisted",
            None,
            CollectionVisibility::Unlisted,
            None,
        )
        .await
        .unwrap();
        let public_root = insert_collection(
            &pool,
            owner.id,
            None,
            "Public root",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();
        let private_child = test_utils::insert_test_collection(
            &pool,
            owner.id,
            Some(public_root.id),
            "Private child",
        )
        .await;
        let public_child = insert_collection(
            &pool,
            owner.id,
            Some(public_root.id),
            "Public child",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();
        let unlisted_child = insert_collection(
            &pool,
            owner.id,
            Some(public_root.id),
            "Unlisted child",
            None,
            CollectionVisibility::Unlisted,
            None,
        )
        .await
        .unwrap();

        let own_private = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let public_recipe = test_utils::insert_test_recipe(&pool, author.id, "public").await;
        let unlisted_recipe = test_utils::insert_test_recipe(&pool, author.id, "unlisted").await;
        for recipe_id in [own_private.id, public_recipe.id, unlisted_recipe.id] {
            test_utils::add_test_recipe_to_collection(&pool, public_root.id, owner.id, recipe_id)
                .await;
        }

        let discovered = get_public_collections_by_username(&pool, &owner.username, 0, 100)
            .await
            .unwrap();
        assert_eq!(discovered.total_count, 3);
        assert!(!discovered.has_more);
        assert!(discovered
            .collections
            .iter()
            .all(|collection| collection.visibility == CollectionVisibility::Public));
        assert!(!discovered.collections.iter().any(|collection| {
            [
                private_root.id,
                private_child.id,
                unlisted.id,
                unlisted_child.id,
            ]
            .contains(&collection.id)
        }));

        let nested = discovered
            .collections
            .iter()
            .find(|collection| collection.id == public_below_private.id)
            .unwrap();
        assert_eq!(nested.parent_collection_id, None);

        let root = discovered
            .collections
            .iter()
            .find(|collection| collection.id == public_root.id)
            .unwrap();
        assert_eq!(root.child_count, 1);
        assert_eq!(root.recipe_count, 1);
        assert!(discovered
            .collections
            .iter()
            .any(|collection| collection.id == public_child.id));

        let first_page = get_public_collections_by_username(&pool, &owner.username, 0, 2)
            .await
            .unwrap();
        assert_eq!(first_page.collections.len(), 2);
        assert_eq!(first_page.total_count, 3);
        assert!(first_page.has_more);

        let missing = get_public_collections_by_username(&pool, "missing-user", 0, 20)
            .await
            .unwrap();
        assert!(missing.collections.is_empty());
        assert_eq!(missing.total_count, 0);
    }

    #[tokio::test]
    async fn test_shared_collection_hierarchy_does_not_leak_inaccessible_relatives() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let private_root =
            test_utils::insert_test_collection(&pool, owner.id, None, "Private root").await;
        let unlisted_middle = insert_collection(
            &pool,
            owner.id,
            Some(private_root.id),
            "Unlisted middle",
            None,
            CollectionVisibility::Unlisted,
            None,
        )
        .await
        .unwrap();
        let public_leaf = insert_collection(
            &pool,
            owner.id,
            Some(unlisted_middle.id),
            "Public leaf",
            None,
            CollectionVisibility::Public,
            None,
        )
        .await
        .unwrap();
        let private_child = test_utils::insert_test_collection(
            &pool,
            owner.id,
            Some(unlisted_middle.id),
            "Private child",
        )
        .await;
        let unlisted_child = insert_collection(
            &pool,
            owner.id,
            Some(public_leaf.id),
            "Unlisted child",
            None,
            CollectionVisibility::Unlisted,
            None,
        )
        .await
        .unwrap();

        let public_view = get_collection_contents_for_viewer(&pool, public_leaf.id, None, 0, 20)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            public_view
                .breadcrumbs
                .iter()
                .map(|breadcrumb| breadcrumb.id)
                .collect::<Vec<_>>(),
            vec![public_leaf.id]
        );
        assert!(public_view.child_collections.is_empty());
        assert_eq!(public_view.collection.child_count, 0);

        let unlisted_view =
            get_collection_contents_for_viewer(&pool, unlisted_middle.id, None, 0, 20)
                .await
                .unwrap()
                .unwrap();
        assert_eq!(
            unlisted_view
                .breadcrumbs
                .iter()
                .map(|breadcrumb| breadcrumb.id)
                .collect::<Vec<_>>(),
            vec![unlisted_middle.id]
        );
        assert_eq!(
            unlisted_view
                .child_collections
                .iter()
                .map(|child| child.id)
                .collect::<Vec<_>>(),
            vec![public_leaf.id]
        );
        assert_eq!(unlisted_view.collection.child_count, 1);
        assert!(!unlisted_view
            .child_collections
            .iter()
            .any(|child| child.id == private_child.id));
        assert!(!public_view
            .child_collections
            .iter()
            .any(|child| child.id == unlisted_child.id));
    }

    #[tokio::test]
    async fn test_collection_hierarchy_queries_are_owner_scoped() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other_owner = test_utils::insert_test_user(&pool).await;

        let root = test_utils::insert_test_collection(&pool, owner.id, None, "Root").await;
        let child =
            test_utils::insert_test_collection(&pool, owner.id, Some(root.id), "Child").await;
        let grandchild =
            test_utils::insert_test_collection(&pool, owner.id, Some(child.id), "Grandchild").await;
        let other_root =
            test_utils::insert_test_collection(&pool, other_owner.id, None, "Root").await;

        assert_eq!(
            get_root_collections(&pool, owner.id).await.unwrap().len(),
            1
        );
        assert_eq!(
            get_child_collections(&pool, root.id, owner.id)
                .await
                .unwrap()[0]
                .id,
            child.id
        );
        assert!(
            get_collection_by_id_and_owner(&pool, other_root.id, owner.id)
                .await
                .unwrap()
                .is_none()
        );

        let breadcrumbs = get_collection_breadcrumbs(&pool, grandchild.id, owner.id)
            .await
            .unwrap();
        assert_eq!(
            breadcrumbs.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![root.id, child.id, grandchild.id]
        );

        let descendants = get_collection_descendants(&pool, root.id, owner.id)
            .await
            .unwrap();
        assert_eq!(descendants, vec![child.id, grandchild.id]);

        let all = get_all_collections_by_owner(&pool, owner.id).await.unwrap();
        assert_eq!(all.len(), 3);
        assert!(all.iter().all(|item| item.recipe_count == 0));
    }

    #[tokio::test]
    async fn test_collection_detail_queries_return_only_direct_contents() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let root = test_utils::insert_test_collection(&pool, owner.id, None, "Root").await;
        let child =
            test_utils::insert_test_collection(&pool, owner.id, Some(root.id), "Child").await;
        let grandchild =
            test_utils::insert_test_collection(&pool, owner.id, Some(child.id), "Grandchild").await;
        let nested_recipe = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        test_utils::add_test_recipe_to_collection(&pool, grandchild.id, owner.id, nested_recipe.id)
            .await;

        assert_eq!(
            get_child_collections(&pool, root.id, owner.id)
                .await
                .unwrap()
                .iter()
                .map(|collection| collection.id)
                .collect::<Vec<_>>(),
            vec![child.id]
        );
        assert_eq!(
            get_child_collections(&pool, child.id, owner.id)
                .await
                .unwrap()
                .iter()
                .map(|collection| collection.id)
                .collect::<Vec<_>>(),
            vec![grandchild.id]
        );
        assert_eq!(
            get_collection_recipes(&pool, root.id, owner.id, 0, 20)
                .await
                .unwrap()
                .total_count,
            0
        );
        assert_eq!(
            get_collection_recipes(&pool, grandchild.id, owner.id, 0, 20)
                .await
                .unwrap()
                .recipes[0]
                .id,
            nested_recipe.id
        );
    }

    #[tokio::test]
    async fn test_collection_writes_reject_wrong_owner_and_parent() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other_owner = test_utils::insert_test_user(&pool).await;
        let collection =
            test_utils::insert_test_collection(&pool, owner.id, None, "Private Folder").await;
        let other_collection =
            test_utils::insert_test_collection(&pool, other_owner.id, None, "Other Folder").await;
        let recipe = test_utils::insert_test_recipe(&pool, owner.id, "private").await;

        assert!(matches!(
            insert_collection(
                &pool,
                owner.id,
                Some(other_collection.id),
                "Cross-owner child",
                None,
                CollectionVisibility::Private,
                None,
            )
            .await,
            Err(DbError::CollectionParentNotFound(id)) if id == other_collection.id
        ));
        assert!(matches!(
            update_collection(&pool, collection.id, other_owner.id, "Nope", None, None).await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
        assert!(matches!(
            move_collection(&pool, collection.id, other_owner.id, None).await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
        assert!(matches!(
            add_recipe_to_collection(&pool, collection.id, other_owner.id, recipe.id).await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
        assert!(matches!(
            get_collection_recipes(&pool, collection.id, other_owner.id, 0, 10).await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
        assert!(matches!(
            get_collection_recipe_count(&pool, collection.id, other_owner.id).await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
        assert!(matches!(
            delete_collection(&pool, collection.id, other_owner.id).await,
            Err(DbError::CollectionNotFound(id)) if id == collection.id
        ));
        assert!(
            get_collection_by_id_and_owner(&pool, collection.id, owner.id)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_collection_name_uniqueness_is_case_insensitive_per_parent() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let first = test_utils::insert_test_collection(&pool, owner.id, None, "Recipes").await;

        let duplicate = insert_collection(
            &pool,
            owner.id,
            None,
            "recipes",
            None,
            CollectionVisibility::Private,
            None,
        )
        .await;
        assert!(matches!(duplicate, Err(DbError::CollectionNameTaken)));

        let nested =
            test_utils::insert_test_collection(&pool, owner.id, Some(first.id), "recipes").await;
        assert_eq!(nested.parent_collection_id, Some(first.id));
    }

    #[tokio::test]
    async fn test_collection_membership_applies_recipe_visibility_rules() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let author = test_utils::insert_test_user(&pool).await;
        let collection = test_utils::insert_test_collection(&pool, owner.id, None, "Recipes").await;
        let own_private = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let public = test_utils::insert_test_recipe(&pool, author.id, "public").await;
        let unlisted = test_utils::insert_test_recipe(&pool, author.id, "unlisted").await;
        let private = test_utils::insert_test_recipe(&pool, author.id, "private").await;

        assert_eq!(
            test_utils::add_test_recipe_to_collection(
                &pool,
                collection.id,
                owner.id,
                own_private.id,
            )
            .await
            .sort_order,
            0
        );
        test_utils::add_test_recipe_to_collection(&pool, collection.id, owner.id, public.id).await;
        test_utils::add_test_recipe_to_collection(&pool, collection.id, owner.id, unlisted.id)
            .await;

        let inaccessible =
            add_recipe_to_collection(&pool, collection.id, owner.id, private.id).await;
        assert!(matches!(
            inaccessible,
            Err(DbError::RecipeNotAccessible(id)) if id == private.id
        ));
        assert!(matches!(
            add_recipe_to_collection(&pool, collection.id, owner.id, own_private.id).await,
            Err(DbError::RecipeAlreadyInCollection)
        ));

        let listed = get_collection_recipes(&pool, collection.id, owner.id, 0, 20)
            .await
            .unwrap();
        assert_eq!(listed.total_count, 3);
        assert!(!listed.has_more);

        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(public.id)
            .execute(&pool)
            .await
            .unwrap();
        let after_visibility_change = get_collection_recipes(&pool, collection.id, owner.id, 0, 20)
            .await
            .unwrap();
        assert_eq!(after_visibility_change.total_count, 2);
        assert_eq!(
            get_collection_recipe_count(&pool, collection.id, owner.id)
                .await
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn test_collection_move_rejects_cycles_and_delete_cascades_memberships() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let root = test_utils::insert_test_collection(&pool, owner.id, None, "Root").await;
        let child =
            test_utils::insert_test_collection(&pool, owner.id, Some(root.id), "Child").await;
        let grandchild =
            test_utils::insert_test_collection(&pool, owner.id, Some(child.id), "Grandchild").await;
        let destination =
            test_utils::insert_test_collection(&pool, owner.id, None, "Destination").await;
        let recipe = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        test_utils::add_test_recipe_to_collection(&pool, grandchild.id, owner.id, recipe.id).await;

        let tree_before_cycle_rejections = sqlx::query_as::<_, (Uuid, Option<Uuid>, i32)>(
            "SELECT id, parent_collection_id, sort_order
             FROM collections
             WHERE owner_id = $1
             ORDER BY id",
        )
        .bind(owner.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(matches!(
            move_collection(&pool, child.id, owner.id, Some(child.id)).await,
            Err(DbError::CollectionCycle)
        ));
        assert!(matches!(
            move_collection(&pool, child.id, owner.id, Some(grandchild.id)).await,
            Err(DbError::CollectionCycle)
        ));
        let tree_after_cycle_rejections = sqlx::query_as::<_, (Uuid, Option<Uuid>, i32)>(
            "SELECT id, parent_collection_id, sort_order
             FROM collections
             WHERE owner_id = $1
             ORDER BY id",
        )
        .bind(owner.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(tree_after_cycle_rejections, tree_before_cycle_rejections);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)
                 FROM collection_recipes
                 WHERE collection_id = $1 AND recipe_id = $2",
            )
            .bind(grandchild.id)
            .bind(recipe.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );

        let conflict =
            test_utils::insert_test_collection(&pool, owner.id, Some(destination.id), "Child")
                .await;
        assert!(matches!(
            move_collection(&pool, child.id, owner.id, Some(destination.id)).await,
            Err(DbError::CollectionNameTaken)
        ));
        delete_collection(&pool, conflict.id, owner.id)
            .await
            .unwrap();

        let moved = move_collection(&pool, child.id, owner.id, Some(destination.id))
            .await
            .unwrap();
        assert_eq!(moved.parent_collection_id, Some(destination.id));
        assert!(get_collection_by_id_and_owner(&pool, child.id, owner.id)
            .await
            .unwrap()
            .is_some());

        delete_collection(&pool, root.id, owner.id).await.unwrap();
        assert!(
            get_collection_by_id_and_owner(&pool, grandchild.id, owner.id)
                .await
                .unwrap()
                .is_some()
        );
        assert!(get_collection_by_id_and_owner(&pool, child.id, owner.id)
            .await
            .unwrap()
            .is_some());

        delete_collection(&pool, destination.id, owner.id)
            .await
            .unwrap();
        assert!(get_collection_by_id_and_owner(&pool, child.id, owner.id)
            .await
            .unwrap()
            .is_none());
        assert!(
            get_collection_by_id_and_owner(&pool, grandchild.id, owner.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM recipes WHERE id = $1")
                .bind(recipe.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM collection_recipes WHERE recipe_id = $1"
            )
            .bind(recipe.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn test_collection_reorders_normalize_and_update_timestamp() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let parent = test_utils::insert_test_collection(&pool, owner.id, None, "Parent").await;
        let first =
            test_utils::insert_test_collection(&pool, owner.id, Some(parent.id), "First").await;
        let second =
            test_utils::insert_test_collection(&pool, owner.id, Some(parent.id), "Second").await;
        let third =
            test_utils::insert_test_collection(&pool, owner.id, Some(parent.id), "Third").await;
        let recipe_a = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let recipe_b = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        test_utils::add_test_recipe_to_collection(&pool, parent.id, owner.id, recipe_a.id).await;
        test_utils::add_test_recipe_to_collection(&pool, parent.id, owner.id, recipe_b.id).await;

        let before = get_collection_by_id_and_owner(&pool, parent.id, owner.id)
            .await
            .unwrap()
            .unwrap()
            .updated_at;
        reorder_child_collections(
            &pool,
            owner.id,
            Some(parent.id),
            &[third.id, first.id, second.id],
        )
        .await
        .unwrap();
        reorder_collection_recipes(&pool, parent.id, owner.id, &[recipe_b.id, recipe_a.id])
            .await
            .unwrap();

        let children = get_child_collections(&pool, parent.id, owner.id)
            .await
            .unwrap();
        assert_eq!(
            children.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![third.id, first.id, second.id]
        );
        let recipes = get_collection_recipes(&pool, parent.id, owner.id, 0, 20)
            .await
            .unwrap();
        assert_eq!(recipes.recipes[0].id, recipe_b.id);
        assert!(
            get_collection_by_id_and_owner(&pool, parent.id, owner.id)
                .await
                .unwrap()
                .unwrap()
                .updated_at
                >= before
        );

        let children_before_invalid_reorder = get_child_collections(&pool, parent.id, owner.id)
            .await
            .unwrap();
        let recipes_before_invalid_reorder =
            get_collection_recipes(&pool, parent.id, owner.id, 0, 20)
                .await
                .unwrap();
        assert!(matches!(
            reorder_child_collections(&pool, owner.id, Some(parent.id), &[first.id]).await,
            Err(DbError::InvalidCollectionOrder)
        ));
        assert!(matches!(
            reorder_collection_recipes(&pool, parent.id, owner.id, &[recipe_a.id]).await,
            Err(DbError::InvalidCollectionOrder)
        ));
        assert_eq!(
            get_child_collections(&pool, parent.id, owner.id)
                .await
                .unwrap(),
            children_before_invalid_reorder
        );
        assert_eq!(
            get_collection_recipes(&pool, parent.id, owner.id, 0, 20)
                .await
                .unwrap(),
            recipes_before_invalid_reorder
        );
    }

    #[tokio::test]
    async fn test_concurrent_root_collection_appends_have_unique_orders() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let mut tasks = Vec::new();
        for index in 0..8 {
            let pool = pool.clone();
            tasks.push(tokio::spawn(async move {
                insert_collection(
                    &pool,
                    owner.id,
                    None,
                    &format!("Concurrent {index}"),
                    None,
                    CollectionVisibility::Private,
                    None,
                )
                .await
                .unwrap()
                .sort_order
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }

        let orders = sqlx::query_scalar::<_, i32>(
            "SELECT sort_order FROM collections WHERE owner_id = $1 ORDER BY sort_order",
        )
        .bind(owner.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(orders, (0..8).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn test_concurrent_collection_reorders_keep_sequences_normalized_and_isolated() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let parent = test_utils::insert_test_collection(&pool, owner.id, None, "Parent").await;
        let first =
            test_utils::insert_test_collection(&pool, owner.id, Some(parent.id), "First").await;
        let second =
            test_utils::insert_test_collection(&pool, owner.id, Some(parent.id), "Second").await;
        let third =
            test_utils::insert_test_collection(&pool, owner.id, Some(parent.id), "Third").await;
        let other_parent = test_utils::insert_test_collection(&pool, owner.id, None, "Other").await;
        let other_first = test_utils::insert_test_collection(
            &pool,
            owner.id,
            Some(other_parent.id),
            "Other first",
        )
        .await;
        let other_second = test_utils::insert_test_collection(
            &pool,
            owner.id,
            Some(other_parent.id),
            "Other second",
        )
        .await;

        let recipe_a = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let recipe_b = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let recipe_c = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let other_recipe_a = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        let other_recipe_b = test_utils::insert_test_recipe(&pool, owner.id, "private").await;
        for recipe_id in [recipe_a.id, recipe_b.id, recipe_c.id] {
            test_utils::add_test_recipe_to_collection(&pool, parent.id, owner.id, recipe_id).await;
        }
        for recipe_id in [other_recipe_a.id, other_recipe_b.id] {
            test_utils::add_test_recipe_to_collection(&pool, other_parent.id, owner.id, recipe_id)
                .await;
        }

        let other_children_before = sqlx::query_as::<_, (Uuid, i32)>(
            "SELECT id, sort_order
             FROM collections
             WHERE parent_collection_id = $1
             ORDER BY sort_order, id",
        )
        .bind(other_parent.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            other_children_before,
            vec![(other_first.id, 0), (other_second.id, 1)]
        );
        let other_recipes_before = sqlx::query_as::<_, (Uuid, i32)>(
            "SELECT recipe_id, sort_order
             FROM collection_recipes
             WHERE collection_id = $1
             ORDER BY sort_order, recipe_id",
        )
        .bind(other_parent.id)
        .fetch_all(&pool)
        .await
        .unwrap();

        let child_order_a = vec![third.id, first.id, second.id];
        let child_order_b = vec![second.id, third.id, first.id];
        let (child_result_a, child_result_b) = tokio::join!(
            reorder_child_collections(&pool, owner.id, Some(parent.id), &child_order_a),
            reorder_child_collections(&pool, owner.id, Some(parent.id), &child_order_b),
        );
        child_result_a.unwrap();
        child_result_b.unwrap();

        let recipe_order_a = vec![recipe_c.id, recipe_a.id, recipe_b.id];
        let recipe_order_b = vec![recipe_b.id, recipe_c.id, recipe_a.id];
        let (recipe_result_a, recipe_result_b) = tokio::join!(
            reorder_collection_recipes(&pool, parent.id, owner.id, &recipe_order_a),
            reorder_collection_recipes(&pool, parent.id, owner.id, &recipe_order_b),
        );
        recipe_result_a.unwrap();
        recipe_result_b.unwrap();

        let child_rows = sqlx::query_as::<_, (Uuid, i32)>(
            "SELECT id, sort_order
             FROM collections
             WHERE parent_collection_id = $1
             ORDER BY sort_order, id",
        )
        .bind(parent.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            child_rows
                .iter()
                .map(|(_, order)| *order)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        let final_child_ids = child_rows.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        assert!(final_child_ids == child_order_a || final_child_ids == child_order_b);

        let recipe_rows = sqlx::query_as::<_, (Uuid, i32)>(
            "SELECT recipe_id, sort_order
             FROM collection_recipes
             WHERE collection_id = $1
             ORDER BY sort_order, recipe_id",
        )
        .bind(parent.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            recipe_rows
                .iter()
                .map(|(_, order)| *order)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        let final_recipe_ids = recipe_rows.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        assert!(final_recipe_ids == recipe_order_a || final_recipe_ids == recipe_order_b);

        assert_eq!(
            sqlx::query_as::<_, (Uuid, i32)>(
                "SELECT id, sort_order
                 FROM collections
                 WHERE parent_collection_id = $1
                 ORDER BY sort_order, id",
            )
            .bind(other_parent.id)
            .fetch_all(&pool)
            .await
            .unwrap(),
            other_children_before
        );
        assert_eq!(
            sqlx::query_as::<_, (Uuid, i32)>(
                "SELECT recipe_id, sort_order
                 FROM collection_recipes
                 WHERE collection_id = $1
                 ORDER BY sort_order, recipe_id",
            )
            .bind(other_parent.id)
            .fetch_all(&pool)
            .await
            .unwrap(),
            other_recipes_before
        );
    }

    #[tokio::test]
    async fn test_collection_cascades_are_scoped_and_preserve_unrelated_collections() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let collection_owner = test_utils::insert_test_user(&pool).await;
        let recipe_author = test_utils::insert_test_user(&pool).await;
        let unrelated_owner = test_utils::insert_test_user(&pool).await;

        let saved_sources =
            test_utils::insert_test_collection(&pool, collection_owner.id, None, "Saved sources")
                .await;
        let source_recipe = test_utils::insert_test_recipe(&pool, recipe_author.id, "public").await;
        test_utils::add_test_recipe_to_collection(
            &pool,
            saved_sources.id,
            collection_owner.id,
            source_recipe.id,
        )
        .await;

        let unrelated_collection =
            test_utils::insert_test_collection(&pool, unrelated_owner.id, None, "Unrelated").await;
        let unrelated_recipe =
            test_utils::insert_test_recipe(&pool, unrelated_owner.id, "private").await;
        test_utils::add_test_recipe_to_collection(
            &pool,
            unrelated_collection.id,
            unrelated_owner.id,
            unrelated_recipe.id,
        )
        .await;

        sqlx::query("UPDATE recipes SET visibility = 'private' WHERE id = $1")
            .bind(source_recipe.id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            get_collection_recipes(&pool, saved_sources.id, collection_owner.id, 0, 20)
                .await
                .unwrap()
                .total_count,
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM collection_recipes
                 WHERE collection_id = $1 AND recipe_id = $2",
            )
            .bind(saved_sources.id)
            .bind(source_recipe.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert!(
            get_collection_by_id_and_owner(&pool, saved_sources.id, collection_owner.id)
                .await
                .unwrap()
                .is_some()
        );

        delete_recipe(&pool, source_recipe.id, recipe_author.id)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM collection_recipes
                 WHERE collection_id = $1 AND recipe_id = $2",
            )
            .bind(saved_sources.id)
            .bind(source_recipe.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        assert!(
            get_collection_by_id_and_owner(&pool, saved_sources.id, collection_owner.id)
                .await
                .unwrap()
                .is_some()
        );

        let doomed_collection = test_utils::insert_test_collection(
            &pool,
            collection_owner.id,
            Some(saved_sources.id),
            "Deleted with owner",
        )
        .await;
        let owned_recipe =
            test_utils::insert_test_recipe(&pool, collection_owner.id, "private").await;
        test_utils::add_test_recipe_to_collection(
            &pool,
            doomed_collection.id,
            collection_owner.id,
            owned_recipe.id,
        )
        .await;
        delete_user(&pool, collection_owner.id).await.unwrap();

        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM collections WHERE owner_id = $1")
                .bind(collection_owner.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM collection_recipes WHERE collection_id = $1",
            )
            .bind(doomed_collection.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        assert!(
            get_collection_by_id_and_owner(&pool, unrelated_collection.id, unrelated_owner.id,)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            get_collection_recipes(&pool, unrelated_collection.id, unrelated_owner.id, 0, 20,)
                .await
                .unwrap()
                .recipes[0]
                .id,
            unrelated_recipe.id
        );
    }
}
