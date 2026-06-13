//! Database layer: connection pool, types, and query functions.
//!
//! Only compiled when the `server` feature is enabled.
#![cfg(feature = "server")]
// Public API items are used by other modules at runtime, but the compiler
// can't see that from this crate's binary target alone.
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

pub mod diff;

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
    /// The requested recipe was not found or the user lacks access.
    RecipeNotFound,
    /// The requested version was not found.
    VersionNotFound,
    /// A fork operation failed (e.g., source recipe inaccessible).
    ForkError,
    /// A diff computation or application failed.
    DiffError(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::MissingUrl => write!(f, "DATABASE_URL not set"),
            DbError::Connection(e) => write!(f, "database connection failed: {e}"),
            DbError::Query(e) => write!(f, "database query failed: {e}"),
            DbError::UsernameTaken => write!(f, "username is already taken"),
            DbError::SessionInvalid => write!(f, "session is invalid, revoked, or expired"),
            DbError::RecipeNotFound => write!(f, "recipe not found"),
            DbError::VersionNotFound => write!(f, "version not found"),
            DbError::ForkError => write!(f, "fork operation failed"),
            DbError::DiffError(msg) => write!(f, "diff error: {msg}"),
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
            DbError::RecipeNotFound => None,
            DbError::VersionNotFound => None,
            DbError::ForkError => None,
            DbError::DiffError(_) => None,
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

/// A versioned snapshot of a recipe's data.
///
/// Latest versions store a full snapshot; historical versions store only
/// `reverse_diff` (JSON Patch to reconstruct from the next version).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecipeVersion {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub version_number: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
    pub reverse_diff: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub is_latest: bool,
    pub created_at: DateTime<Utc>,
}

/// A fork relationship linking a forked recipe to its source.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ForkRelationship {
    pub id: Uuid,
    pub original_recipe_id: Uuid,
    pub forked_recipe_id: Uuid,
    pub forked_by: Uuid,
    pub forked_version_number: Option<i32>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Full recipe record from the `recipes` table.
///
/// Maps to the core recipe table. Used for ownership verification
/// and metadata updates in the versioned edit flow.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Recipe {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub is_public: bool,
    pub is_draft: bool,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    sqlx::query("INSERT INTO auth_states (id, provider, redirect_uri, code_verifier, user_id) VALUES ($1, $2, $3, $4, $5)")
        .bind(id)
        .bind(provider)
        .bind(redirect_uri)
        .bind(code_verifier)
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
        "DELETE FROM auth_states WHERE id = $1 RETURNING id, redirect_uri, provider, code_verifier, user_id, created_at",
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
pub async fn get_user_by_username(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    username: &str,
) -> Result<bool, DbError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
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

/// Delete a user by ID. OAuth accounts cascade automatically via `ON DELETE CASCADE`.
///
/// Revokes all OAuth tokens with providers before deletion. Revocation
/// failures are logged but never block deletion.
pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> Result<(), DbError> {
    // Revoke all OAuth tokens before deletion (non-blocking — failures are logged)
    crate::auth::revoke::revoke_all_user_tokens(pool, user_id).await;

    // Now delete the user (oauth_accounts cascade automatically)
    let rows = sqlx::query("DELETE FROM users WHERE id = $1")
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
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND id != $2)")
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

// ── Recipe versioning query functions ────────────────────────────────────────

/// Get the maximum version number for a recipe.
/// Returns 0 if no versions exist.
pub async fn get_max_version_number(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<i32, DbError> {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(MAX(version_number), 0) FROM recipe_versions WHERE recipe_id = $1"#,
        recipe_id
    )
    .fetch_one(executor)
    .await
    .map(|v| v.unwrap_or(0))
    .map_err(DbError::Query)
}

/// Get the latest version of a recipe (full snapshot).
pub async fn get_latest_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<RecipeVersion, DbError> {
    sqlx::query_as!(
        RecipeVersion,
        r#"SELECT id, recipe_id, version_number, title, description, prep_time_min,
                  cook_time_min, total_time_min, servings, ingredients, steps,
                  reverse_diff, notes, is_latest, created_at
           FROM recipe_versions WHERE recipe_id = $1 AND is_latest = TRUE"#,
        recipe_id
    )
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::VersionNotFound,
        other => DbError::Query(other),
    })
}

/// Get a specific version by version number.
pub async fn get_version_by_number(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
) -> Result<RecipeVersion, DbError> {
    sqlx::query_as!(
        RecipeVersion,
        r#"SELECT id, recipe_id, version_number, title, description, prep_time_min,
                  cook_time_min, total_time_min, servings, ingredients, steps,
                  reverse_diff, notes, is_latest, created_at
           FROM recipe_versions WHERE recipe_id = $1 AND version_number = $2"#,
        recipe_id,
        version_number
    )
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::VersionNotFound,
        other => DbError::Query(other),
    })
}

/// Get all versions of a recipe, ordered newest to oldest.
pub async fn get_all_versions(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<Vec<RecipeVersion>, DbError> {
    sqlx::query_as!(
        RecipeVersion,
        r#"SELECT id, recipe_id, version_number, title, description, prep_time_min,
                   cook_time_min, total_time_min, servings, ingredients, steps,
                   reverse_diff, notes, is_latest, created_at
            FROM recipe_versions WHERE recipe_id = $1 ORDER BY version_number DESC"#,
        recipe_id
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get all versions of a recipe, ordered oldest to newest (ascending version_number).
///
/// Used by the version history UI to display a chronological timeline.
pub async fn get_recipe_versions(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<Vec<RecipeVersion>, DbError> {
    sqlx::query_as!(
        RecipeVersion,
        r#"SELECT id, recipe_id, version_number, title, description, prep_time_min,
                   cook_time_min, total_time_min, servings, ingredients, steps,
                   reverse_diff, notes, is_latest, created_at
            FROM recipe_versions WHERE recipe_id = $1 ORDER BY version_number ASC"#,
        recipe_id
    )
    .fetch_all(executor)
    .await
    .map_err(DbError::Query)
}

/// Get reverse diffs for versions up to and including target_version.
/// Returns diffs ordered from newest to oldest for chain reconstruction.
pub async fn get_reverse_diffs(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    target_version: i32,
) -> Result<Vec<serde_json::Value>, DbError> {
    sqlx::query_scalar!(
        r#"SELECT reverse_diff FROM recipe_versions
           WHERE recipe_id = $1 AND version_number <= $2 AND reverse_diff IS NOT NULL
           ORDER BY version_number DESC"#,
        recipe_id,
        target_version
    )
    .fetch_all(executor)
    .await
    .map(|v| v.into_iter().flatten().collect())
    .map_err(DbError::Query)
}

/// Revoke `is_latest` flag from all versions of a recipe.
pub async fn revoke_latest_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
) -> Result<(), DbError> {
    sqlx::query!(
        r#"UPDATE recipe_versions SET is_latest = FALSE WHERE recipe_id = $1 AND is_latest = TRUE"#,
        recipe_id
    )
    .execute(executor)
    .await
    .map(|_| ())
    .map_err(DbError::Query)
}

/// Set a specific version as the latest.
/// Call `revoke_latest_version` first to ensure single latest.
pub async fn set_latest_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
) -> Result<(), DbError> {
    sqlx::query!(
        r#"UPDATE recipe_versions SET is_latest = TRUE
           WHERE recipe_id = $1 AND version_number = $2"#,
        recipe_id,
        version_number
    )
    .execute(executor)
    .await
    .map(|_| ())
    .map_err(DbError::Query)
}

/// Insert a new version row (historical, with reverse_diff).
/// Not marked as latest — call `set_latest_version` separately if needed.
pub async fn insert_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
    reverse_diff: Option<serde_json::Value>,
    notes: Option<&str>,
) -> Result<uuid::Uuid, DbError> {
    sqlx::query_scalar!(
        r#"INSERT INTO recipe_versions (recipe_id, version_number, reverse_diff, notes, is_latest)
           VALUES ($1, $2, $3, $4, FALSE) RETURNING id"#,
        recipe_id,
        version_number,
        reverse_diff,
        notes
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

// ── CP3: Versioned edit flow query functions ────────────────────────────────

/// Insert a new recipe and return it.
///
/// Used by tests and the recipe creation endpoint.
#[allow(clippy::too_many_arguments)]
pub async fn insert_recipe(
    pool: &PgPool,
    owner_id: Uuid,
    title: &str,
    description: Option<&str>,
    is_public: bool,
    is_draft: bool,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<serde_json::Value>,
    steps: Option<serde_json::Value>,
) -> Result<Recipe, DbError> {
    sqlx::query_as!(
        Recipe,
        r#"INSERT INTO recipes
           (owner_id, title, description, is_public, is_draft,
            prep_time_min, cook_time_min, total_time_min, servings,
            ingredients, steps)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, owner_id, title, description, is_public, is_draft,
              prep_time_min, cook_time_min, total_time_min, servings,
              ingredients, steps, created_at, updated_at"#,
        owner_id,
        title,
        description,
        is_public,
        is_draft,
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients,
        steps,
    )
    .fetch_one(pool)
    .await
    .map_err(DbError::Query)
}

/// Get a recipe by ID and verify ownership.
///
/// Returns `DbError::RecipeNotFound` if the recipe doesn't exist or
/// the user is not the owner.
pub async fn get_recipe_by_id_and_owner(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    owner_id: Uuid,
) -> Result<Recipe, DbError> {
    sqlx::query_as!(
        Recipe,
        r#"SELECT id, owner_id, title, description, is_public, is_draft,
               prep_time_min, cook_time_min, total_time_min, servings,
               ingredients, steps, created_at, updated_at
           FROM recipes
           WHERE id = $1 AND owner_id = $2"#,
        recipe_id,
        owner_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::RecipeNotFound,
        other => DbError::Query(other),
    })
}

/// Update recipe metadata, syncing all mutable fields from the new version
/// to the denormalized `recipes` table.
///
/// Note: `ingredients` and `steps` columns are `NOT NULL DEFAULT '[]'::jsonb`,
/// so `None` values are converted to empty arrays.
#[allow(clippy::too_many_arguments)]
pub async fn update_recipe_metadata(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    title: &str,
    description: Option<&str>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<&serde_json::Value>,
    steps: Option<&serde_json::Value>,
) -> Result<(), DbError> {
    // ingredients and steps are NOT NULL columns defaulting to '[]'::jsonb
    let ingredients_val = ingredients.cloned().unwrap_or_else(|| serde_json::json!([]));
    let steps_val = steps.cloned().unwrap_or_else(|| serde_json::json!([]));

    sqlx::query!(
        r#"UPDATE recipes SET title = $2, description = $3, prep_time_min = $4,
           cook_time_min = $5, total_time_min = $6, servings = $7,
           ingredients = $8, steps = $9, updated_at = NOW()
           WHERE id = $1"#,
        recipe_id,
        title,
        description,
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients_val,
        steps_val,
    )
    .execute(executor)
    .await
    .map(|_| ())
    .map_err(DbError::Query)
}

/// Insert a new version as the latest full snapshot.
#[allow(clippy::too_many_arguments)]
pub async fn insert_latest_version(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    version_number: i32,
    title: &str,
    description: Option<&str>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<&serde_json::Value>,
    steps: Option<&serde_json::Value>,
    notes: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query!(
        r#"INSERT INTO recipe_versions
           (recipe_id, version_number, title, description, prep_time_min,
            cook_time_min, total_time_min, servings, ingredients, steps,
            reverse_diff, notes, is_latest)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, $11, TRUE)"#,
        recipe_id,
        version_number,
        title,
        description,
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients,
        steps,
        notes,
    )
    .execute(executor)
    .await
    .map(|_| ())
    .map_err(DbError::Query)
}

/// Update a recipe with versioning.
///
/// Transactional 8-step flow:
/// 1. Verify ownership via `get_recipe_by_id_and_owner`
/// 2. Get current latest version via `get_latest_version`
/// 3. Serialize current version to JSON via `recipe_to_json`
/// 4. Serialize new fields to JSON via `recipe_to_json_from_fields`
/// 5. Compute forward diff (old -> new) via `compute_diff`
/// 6. Compute reverse diff (new -> old) via `reverse_patch`
/// 7. Mark current latest as historical: store reverse_diff, set is_latest = false
/// 8. Insert new version as latest full snapshot (version_number + 1)
/// 9. Update recipe metadata (title, updated_at)
#[allow(clippy::too_many_arguments)]
pub async fn update_recipe_versioned(
    pool: &PgPool,
    recipe_id: &Uuid,
    owner_id: &Uuid,
    title: &str,
    description: Option<&str>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: &Option<serde_json::Value>,
    steps: &Option<serde_json::Value>,
    notes: Option<&str>,
) -> Result<(Recipe, i32), DbError> {
    // Step 1: Verify ownership
    let _recipe = get_recipe_by_id_and_owner(pool, *recipe_id, *owner_id).await?;

    // Step 2: Get current latest version
    let latest = get_latest_version(pool, *recipe_id).await?;

    // Step 3: Serialize current version to JSON
    let old_json = diff::recipe_to_json(&latest)?;

    // Step 4: Serialize new fields to JSON
    let new_json = diff::recipe_to_json_from_fields(
        title, description, prep_time_min, cook_time_min,
        total_time_min, servings, ingredients, steps,
    )?;

    // Step 5: Compute forward diff (old -> new)
    let forward_diff = diff::compute_diff(&old_json, &new_json)?;

    // Step 6: Compute reverse diff (new -> old)
    let reverse_diff_patch = diff::reverse_patch(&forward_diff, &old_json)?;
    let reverse_diff: serde_json::Value =
        serde_json::to_value(reverse_diff_patch).map_err(|e| DbError::DiffError(e.to_string()))?;

    // Steps 7-9: Transactional update
    let mut tx = pool.begin().await.map_err(DbError::Query)?;

    // Step 7a: Revoke latest from current version
    revoke_latest_version(&mut *tx, *recipe_id).await?;

    // Step 7b: Store reverse_diff on current version
    sqlx::query!(
        r#"UPDATE recipe_versions SET reverse_diff = $3
           WHERE recipe_id = $1 AND version_number = $2"#,
        recipe_id,
        latest.version_number,
        reverse_diff,
    )
    .execute(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    // Step 8: Insert new version as latest full snapshot
    let new_version_number = latest.version_number + 1;
    insert_latest_version(
        &mut *tx, *recipe_id, new_version_number, title, description,
        prep_time_min, cook_time_min, total_time_min, servings,
        ingredients.as_ref(), steps.as_ref(), notes,
    )
    .await?;

    // Step 9: Update recipe metadata (sync all fields to denormalized recipes table)
    update_recipe_metadata(
        &mut *tx, *recipe_id, title, description,
        prep_time_min, cook_time_min, total_time_min, servings,
        ingredients.as_ref(), steps.as_ref(),
    )
    .await?;

    // Fetch updated recipe
    let updated_recipe = sqlx::query_as!(
        Recipe,
        r#"SELECT id, owner_id, title, description, is_public, is_draft,
               prep_time_min, cook_time_min, total_time_min, servings,
               ingredients, steps, created_at, updated_at
           FROM recipes WHERE id = $1"#,
        recipe_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    tx.commit().await.map_err(DbError::Query)?;

    Ok((updated_recipe, new_version_number))
}

// ── Restore version ──────────────────────────────────────────────────────────

/// Restore a historical version by reconstructing its data and saving as a new version.
///
/// The target version's data is reconstructed (either directly if it's the latest,
/// or via reverse-diff chain), then saved as a new version with auto-notes.
pub async fn restore_version(
    pool: &PgPool,
    recipe_id: &Uuid,
    owner_id: &Uuid,
    target_version_number: i32,
) -> Result<(Recipe, i32), DbError> {
    // 1. Verify ownership
    let _recipe = get_recipe_by_id_and_owner(pool, *recipe_id, *owner_id).await?;

    // 2. Get all versions (ASC order for chain reconstruction)
    let versions = get_recipe_versions(pool, *recipe_id).await?;

    // 3. Find target version
    let target = versions
        .iter()
        .find(|v| v.version_number == target_version_number)
        .ok_or(DbError::VersionNotFound)?;

    // 4. Reconstruct target version's data
    let (title, description, prep_time_min, cook_time_min, total_time_min, servings, ingredients, steps) =
        if target.is_latest {
            // Target IS latest — use data directly
            (
                target
                    .title
                    .clone()
                    .ok_or_else(|| DbError::DiffError("version title is NULL".to_string()))?,
                target.description.clone(),
                target.prep_time_min,
                target.cook_time_min,
                target.total_time_min,
                target.servings,
                target.ingredients.clone(),
                target.steps.clone(),
            )
        } else {
            // Historical — reconstruct from reverse diff chain
            let latest = versions.iter().find(|v| v.is_latest).ok_or(DbError::VersionNotFound)?;
            let latest_json = diff::recipe_to_json(latest)?;

            let reverse_diffs: Vec<serde_json::Value> = versions
                .iter()
                .filter(|v| v.version_number >= target_version_number && v.reverse_diff.is_some())
                .map(|v| v.reverse_diff.clone().unwrap())
                .rev()
                .collect();

            let reconstructed_json = diff::reconstruct_from_chain(&latest_json, &reverse_diffs)?;
            let snapshot = diff::json_to_recipe(&reconstructed_json)?;

            (
                snapshot.title,
                snapshot.description,
                snapshot.prep_time_min,
                snapshot.cook_time_min,
                snapshot.total_time_min,
                snapshot.servings,
                Some(serde_json::Value::Array(snapshot.ingredients)),
                Some(serde_json::Value::Array(snapshot.steps)),
            )
        };

    // 5. Create new version from restored data (with auto-notes)
    let notes = format!("Restored from v{target_version_number}");
    update_recipe_versioned(
        pool,
        recipe_id,
        owner_id,
        &title,
        description.as_deref(),
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        &ingredients,
        &steps,
        Some(&notes),
    )
    .await
}

// ── Draft operations ────────────────────────────────────────────────────────

/// Publish a draft recipe: set `is_draft = FALSE` and verify ownership.
pub async fn publish_recipe(
    pool: &PgPool,
    recipe_id: Uuid,
    owner_id: Uuid,
) -> Result<Recipe, DbError> {
    // Verify ownership first
    get_recipe_by_id_and_owner(pool, recipe_id, owner_id).await?;

    sqlx::query_as!(
        Recipe,
        r#"UPDATE recipes
           SET is_draft = FALSE, updated_at = NOW()
           WHERE id = $1 AND owner_id = $2
           RETURNING id, owner_id, title, description, is_public, is_draft,
              prep_time_min, cook_time_min, total_time_min, servings,
              ingredients, steps, created_at, updated_at"#,
        recipe_id,
        owner_id,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::RecipeNotFound,
        other => DbError::Query(other),
    })
}

/// Get recipes owned by a user, optionally filtering by draft status.
///
/// When `include_drafts` is `true`, returns all recipes (drafts + published).
/// When `false`, returns only published recipes (`is_draft = FALSE`).
pub async fn get_recipes_by_owner_with_draft_filter(
    pool: &PgPool,
    owner_id: Uuid,
    include_drafts: bool,
) -> Result<Vec<Recipe>, DbError> {
    let recipes = if include_drafts {
        sqlx::query_as!(
            Recipe,
            r#"SELECT id, owner_id, title, description, is_public, is_draft,
                   prep_time_min, cook_time_min, total_time_min, servings,
                   ingredients, steps, created_at, updated_at
               FROM recipes
               WHERE owner_id = $1
               ORDER BY updated_at DESC"#,
            owner_id,
        )
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
    } else {
        sqlx::query_as!(
            Recipe,
            r#"SELECT id, owner_id, title, description, is_public, is_draft,
                   prep_time_min, cook_time_min, total_time_min, servings,
                   ingredients, steps, created_at, updated_at
               FROM recipes
               WHERE owner_id = $1 AND is_draft = FALSE
               ORDER BY updated_at DESC"#,
            owner_id,
        )
        .fetch_all(pool)
        .await
        .map_err(DbError::Query)?
    };

    Ok(recipes)
}

/// Create a new draft recipe with `is_draft = TRUE` and a v1 version entry.
#[allow(clippy::too_many_arguments)]
pub async fn create_draft_recipe(
    pool: &PgPool,
    owner_id: Uuid,
    title: &str,
    description: Option<&str>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<&serde_json::Value>,
    steps: Option<&serde_json::Value>,
) -> Result<Recipe, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;

    // Insert recipe row with is_draft = TRUE
    let recipe = sqlx::query_as!(
        Recipe,
        r#"INSERT INTO recipes
           (owner_id, title, description, is_public, is_draft,
            prep_time_min, cook_time_min, total_time_min, servings,
            ingredients, steps)
           VALUES ($1, $2, $3, FALSE, TRUE, $4, $5, $6, $7, $8, $9)
           RETURNING id, owner_id, title, description, is_public, is_draft,
              prep_time_min, cook_time_min, total_time_min, servings,
              ingredients, steps, created_at, updated_at"#,
        owner_id,
        title,
        description,
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients.cloned().unwrap_or_else(|| serde_json::json!([])),
        steps.cloned().unwrap_or_else(|| serde_json::json!([])),
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    // Backfill v1 version entry
    insert_latest_version(
        &mut *tx,
        recipe.id,
        1,
        title,
        description,
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients,
        steps,
        None,
    )
    .await?;

    tx.commit().await.map_err(DbError::Query)?;

    Ok(recipe)
}

/// Update an existing draft recipe's metadata without creating a new version.
///
/// Used for auto-save during draft editing. Only updates the denormalized
/// `recipes` table — does NOT create a new version entry.
/// Both the metadata update and version snapshot update are wrapped in a
/// transaction for atomicity.
#[allow(clippy::too_many_arguments)]
pub async fn update_draft(
    pool: &PgPool,
    recipe_id: Uuid,
    owner_id: Uuid,
    title: &str,
    description: Option<&str>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<&serde_json::Value>,
    steps: Option<&serde_json::Value>,
) -> Result<Recipe, DbError> {
    let mut tx = pool.begin().await.map_err(DbError::Query)?;

    // Verify ownership first
    get_recipe_by_id_and_owner(&mut *tx, recipe_id, owner_id).await?;

    // Update metadata (ingredients and steps are NOT NULL columns)
    let ingredients_val = ingredients.cloned().unwrap_or_else(|| serde_json::json!([]));
    let steps_val = steps.cloned().unwrap_or_else(|| serde_json::json!([]));

    let recipe = sqlx::query_as!(
        Recipe,
        r#"UPDATE recipes
           SET title = $2, description = $3, prep_time_min = $4,
               cook_time_min = $5, total_time_min = $6, servings = $7,
               ingredients = $8, steps = $9, updated_at = NOW()
           WHERE id = $1 AND owner_id = $10
           RETURNING id, owner_id, title, description, is_public, is_draft,
              prep_time_min, cook_time_min, total_time_min, servings,
              ingredients, steps, created_at, updated_at"#,
        recipe_id,
        title,
        description,
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients_val,
        steps_val,
        owner_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::RecipeNotFound,
        other => DbError::Query(other),
    })?;

    // Also update the latest version's snapshot to keep it in sync
    let latest = get_latest_version(&mut *tx, recipe_id).await.ok();
    if let Some(latest) = latest {
        sqlx::query!(
            r#"UPDATE recipe_versions
               SET title = $2, description = $3, prep_time_min = $4,
                   cook_time_min = $5, total_time_min = $6, servings = $7,
                   ingredients = $8, steps = $9
               WHERE recipe_id = $1 AND version_number = $10"#,
            recipe_id,
            title,
            description,
            prep_time_min,
            cook_time_min,
            total_time_min,
            servings,
            ingredients.cloned().unwrap_or_else(|| serde_json::json!([])),
            steps.cloned().unwrap_or_else(|| serde_json::json!([])),
            latest.version_number,
        )
        .execute(&mut *tx)
        .await
        .map_err(DbError::Query)?;
    }

    tx.commit().await.map_err(DbError::Query)?;

    Ok(recipe)
}

// ── Fork operations ─────────────────────────────────────────────────────────

/// Get a recipe if it's public OR owned by the requesting user.
///
/// Returns `(recipe, owner_id)`. Returns `DbError::ForkError` if the recipe
/// is not found or the user lacks access.
pub async fn get_recipe_accessible(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    recipe_id: Uuid,
    requesting_user_id: Uuid,
) -> Result<Recipe, DbError> {
    sqlx::query_as!(
        Recipe,
        r#"SELECT id, owner_id, title, description, is_public, is_draft,
               prep_time_min, cook_time_min, total_time_min, servings,
               ingredients, steps, created_at, updated_at
           FROM recipes
           WHERE id = $1 AND (is_public = TRUE OR owner_id = $2)"#,
        recipe_id,
        requesting_user_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::ForkError,
        other => DbError::Query(other),
    })
}

/// Insert a row into the fork_relationships table.
pub async fn insert_fork_relationship(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    original_recipe_id: Uuid,
    forked_recipe_id: Uuid,
    forked_by: Uuid,
    message: Option<String>,
) -> Result<Uuid, DbError> {
    sqlx::query_scalar!(
        r#"INSERT INTO fork_relationships (original_recipe_id, forked_recipe_id, forked_by, message)
           VALUES ($1, $2, $3, $4) RETURNING id"#,
        original_recipe_id,
        forked_recipe_id,
        forked_by,
        message,
    )
    .fetch_one(executor)
    .await
    .map_err(DbError::Query)
}

/// Get fork attribution for a recipe.
///
/// Returns `(original_recipe_id, original_owner_id, message)` if this recipe
/// is a fork of another recipe.
pub async fn get_fork_info(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    forked_recipe_id: Uuid,
) -> Result<Option<(Uuid, Uuid, Option<String>)>, DbError> {
    #[derive(sqlx::FromRow)]
    struct ForkInfoRow {
        original_recipe_id: Uuid,
        owner_id: Uuid,
        message: Option<String>,
    }

    let row: Option<ForkInfoRow> = sqlx::query_as!(
        ForkInfoRow,
        r#"SELECT fr.original_recipe_id, r.owner_id, fr.message
           FROM fork_relationships fr
           JOIN recipes r ON fr.original_recipe_id = r.id
           WHERE fr.forked_recipe_id = $1"#,
        forked_recipe_id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)?;

    Ok(row.map(|r| (r.original_recipe_id, r.owner_id, r.message)))
}

/// Fork a recipe: create a new draft recipe based on an existing one.
///
/// The source recipe must be public or owned by the forking user.
/// Returns `(new_recipe_id, original_recipe_id, original_title)`.
pub async fn fork_recipe(
    pool: &PgPool,
    source_recipe_id: Uuid,
    forking_user_id: Uuid,
    message: Option<String>,
) -> Result<(Uuid, Uuid, String), DbError> {
    // Step 1: Verify access to source recipe
    let source_recipe = get_recipe_accessible(pool, source_recipe_id, forking_user_id).await?;
    let original_title = source_recipe.title.clone();

    // Step 2: Get the source recipe's latest version snapshot
    let latest = get_latest_version(pool, source_recipe_id).await?;

    // Step 3: Parse snapshot fields
    let title = latest
        .title
        .clone()
        .ok_or_else(|| DbError::DiffError("source version title is NULL".to_string()))?;
    let description = latest.description.clone();
    let prep_time_min = latest.prep_time_min;
    let cook_time_min = latest.cook_time_min;
    let total_time_min = latest.total_time_min;
    let servings = latest.servings;
    let ingredients = latest.ingredients.clone();
    let steps = latest.steps.clone();

    // Step 4-7: Transactional creation
    let mut tx = pool.begin().await.map_err(DbError::Query)?;

    // Step 4: Create new recipe (draft, owned by forking user)
    let new_recipe_id = sqlx::query_scalar!(
        r#"INSERT INTO recipes
           (owner_id, title, description, is_public, is_draft,
            prep_time_min, cook_time_min, total_time_min, servings,
            ingredients, steps)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id"#,
        forking_user_id,
        title,
        description.as_deref(),
        false, // is_public = false
        true,  // is_draft = true
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients.clone(),
        steps.clone(),
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::Query)?;

    // Step 5: Create v1 with full snapshot
    insert_latest_version(
        &mut *tx,
        new_recipe_id,
        1,
        &title,
        description.as_deref(),
        prep_time_min,
        cook_time_min,
        total_time_min,
        servings,
        ingredients.as_ref(),
        steps.as_ref(),
        None,
    )
    .await?;

    // Step 6: Record fork relationship
    insert_fork_relationship(
        &mut *tx,
        source_recipe_id,
        new_recipe_id,
        forking_user_id,
        message,
    )
    .await?;

    tx.commit().await.map_err(DbError::Query)?;

    Ok((new_recipe_id, source_recipe_id, original_title))
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

    // ── CP3: Versioned edit flow tests ──────────────────────────────────────

    #[tokio::test]
    async fn test_update_recipe_versioned() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (user, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Initial state: v1 exists
        let v1 = get_latest_version(&pool, recipe.id).await.unwrap();
        assert_eq!(v1.version_number, 1);
        assert_eq!(v1.title.as_deref(), Some("Test Recipe"));

        // Update the recipe with different content
        let (updated, new_version) = update_recipe_versioned(
            &pool,
            &recipe.id,
            &user.id,
            "Updated Recipe",
            Some("Updated description"),
            Some(20),
            Some(40),
            Some(60),
            Some(8),
            &Some(serde_json::json!(["flour", "sugar", "eggs", "butter"])),
            &Some(serde_json::json!(["Mix", "Bake", "Cool", "Serve"])),
            None,
        )
        .await
        .unwrap();

        // Verify returned data
        assert_eq!(new_version, 2);
        assert_eq!(updated.title, "Updated Recipe");

        // Verify recipes table is fully synced (denormalized latest)
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.prep_time_min, Some(20));
        assert_eq!(updated.cook_time_min, Some(40));
        assert_eq!(updated.total_time_min, Some(60));
        assert_eq!(updated.servings, Some(8));
        assert_eq!(
            updated.ingredients.as_ref().unwrap(),
            &serde_json::json!(["flour", "sugar", "eggs", "butter"])
        );
        assert_eq!(
            updated.steps.as_ref().unwrap(),
            &serde_json::json!(["Mix", "Bake", "Cool", "Serve"])
        );

        // Verify v2 is now latest
        let v2 = get_latest_version(&pool, recipe.id).await.unwrap();
        assert_eq!(v2.version_number, 2);
        assert_eq!(v2.title.as_deref(), Some("Updated Recipe"));
        assert!(v2.is_latest);

        // Verify v1 is no longer latest and has reverse_diff
        let all_versions = get_all_versions(&pool, recipe.id).await.unwrap();
        assert_eq!(all_versions.len(), 2);

        let v1_after = all_versions
            .iter()
            .find(|v| v.version_number == 1)
            .unwrap();
        assert!(!v1_after.is_latest);
        assert!(v1_after.reverse_diff.is_some());
    }

    #[tokio::test]
    async fn test_version_chain() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (user, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Update to v2
        let (_, v2_num) = update_recipe_versioned(
            &pool,
            &recipe.id,
            &user.id,
            "Version 2",
            None,
            None,
            None,
            None,
            None,
            &Some(serde_json::json!(["bread", "butter"])),
            &None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(v2_num, 2);

        // Update to v3
        let (_, v3_num) = update_recipe_versioned(
            &pool,
            &recipe.id,
            &user.id,
            "Version 3",
            None,
            None,
            None,
            None,
            None,
            &Some(serde_json::json!(["cheese", "tomato"])),
            &None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(v3_num, 3);

        // Verify version chain
        let all_versions = get_all_versions(&pool, recipe.id).await.unwrap();
        assert_eq!(all_versions.len(), 3);

        // v1: original, not latest, has reverse_diff
        let v1 = all_versions.iter().find(|v| v.version_number == 1).unwrap();
        assert!(!v1.is_latest);
        assert!(v1.reverse_diff.is_some());

        // v2: intermediate, not latest, has reverse_diff
        let v2 = all_versions.iter().find(|v| v.version_number == 2).unwrap();
        assert!(!v2.is_latest);
        assert!(v2.reverse_diff.is_some());

        // v3: latest, no reverse_diff
        let v3 = all_versions.iter().find(|v| v.version_number == 3).unwrap();
        assert!(v3.is_latest);
        assert!(v3.reverse_diff.is_none());

        // Verify chain reconstruction: v3 -> v2 -> v1
        let v3_json = diff::recipe_to_json(v3).unwrap();
        let v2_reverse: serde_json::Value = v2.reverse_diff.as_ref().unwrap().clone();
        let v1_reverse: serde_json::Value = v1.reverse_diff.as_ref().unwrap().clone();

        // Reconstruct v2 from v3
        let v2_reconstructed =
            diff::reconstruct_from_chain(&v3_json, std::slice::from_ref(&v2_reverse)).unwrap();
        assert_eq!(
            v2_reconstructed.get("title").unwrap().as_str().unwrap(),
            "Version 2"
        );

        // Reconstruct v1 from v3 through v2
        let v1_reconstructed =
            diff::reconstruct_from_chain(&v3_json, &[v2_reverse, v1_reverse]).unwrap();
        assert_eq!(
            v1_reconstructed.get("title").unwrap().as_str().unwrap(),
            "Test Recipe"
        );
    }

    #[tokio::test]
    async fn test_update_recipe_wrong_owner() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (_owner, recipe) = test_utils::create_test_user_and_recipe(&pool).await;
        let u = test_utils::uid();
        let imposter = insert_user(
            &pool,
            &format!("imposter_{u}"),
            "Imposter",
            &format!("imposter{u}@test.com"),
            None,
        )
        .await
        .unwrap();
        let result = update_recipe_versioned(
            &pool,
            &recipe.id,
            &imposter.id,
            "Hacked",
            None,
            None,
            None,
            None,
            None,
            &None,
            &None,
            None,
        )
        .await;
        assert!(matches!(result, Err(DbError::RecipeNotFound)));
    }

    // ── CP5: Restore version tests ──────────────────────────────────────────

    #[tokio::test]
    async fn test_restore_version_creates_new_version() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (user, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // v1: "Test Recipe" (created by create_test_user_and_recipe)
        let v1 = get_latest_version(&pool, recipe.id).await.unwrap();
        assert_eq!(v1.version_number, 1);
        assert_eq!(v1.title.as_deref(), Some("Test Recipe"));

        // Create v2 with different title
        update_recipe_versioned(
            &pool,
            &recipe.id,
            &user.id,
            "Modified Recipe",
            None,
            None,
            None,
            None,
            None,
            &Some(serde_json::json!(["bread", "butter"])),
            &None,
            None,
        )
        .await
        .unwrap();

        // Create v3 with yet another title
        update_recipe_versioned(
            &pool,
            &recipe.id,
            &user.id,
            "Latest Recipe",
            None,
            None,
            None,
            None,
            None,
            &Some(serde_json::json!(["cheese", "tomato"])),
            &None,
            None,
        )
        .await
        .unwrap();

        // Verify v3 is latest
        let v3 = get_latest_version(&pool, recipe.id).await.unwrap();
        assert_eq!(v3.version_number, 3);
        assert_eq!(v3.title.as_deref(), Some("Latest Recipe"));

        // Restore v1 — should create v4 with v1's data
        let (restored_recipe, new_version) = restore_version(
            &pool,
            &recipe.id,
            &user.id,
            1,
        )
        .await
        .unwrap();

        // v4 is created
        assert_eq!(new_version, 4);
        assert_eq!(restored_recipe.title, "Test Recipe");

        // Verify v4 is latest with correct data
        let v4 = get_latest_version(&pool, recipe.id).await.unwrap();
        assert_eq!(v4.version_number, 4);
        assert_eq!(v4.title.as_deref(), Some("Test Recipe"));
        assert!(v4.is_latest);
        assert_eq!(
            v4.notes.as_deref(),
            Some("Restored from v1")
        );

        // Verify original versions still exist unchanged
        let all_versions = get_all_versions(&pool, recipe.id).await.unwrap();
        assert_eq!(all_versions.len(), 4);

        let v1_after = all_versions.iter().find(|v| v.version_number == 1).unwrap();
        assert!(!v1_after.is_latest);
        assert_eq!(v1_after.title.as_deref(), Some("Test Recipe"));

        let v2_after = all_versions.iter().find(|v| v.version_number == 2).unwrap();
        assert!(!v2_after.is_latest);
        assert_eq!(v2_after.title.as_deref(), Some("Modified Recipe"));

        let v3_after = all_versions.iter().find(|v| v.version_number == 3).unwrap();
        assert!(!v3_after.is_latest);
        assert_eq!(v3_after.title.as_deref(), Some("Latest Recipe"));
    }

    #[tokio::test]
    async fn test_restore_version_unauthorized() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (_owner, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Create a different user
        let u = test_utils::uid();
        let imposter = insert_user(
            &pool,
            &format!("imposter_{u}"),
            "Imposter",
            &format!("imposter{u}@test.com"),
            None,
        )
        .await
        .unwrap();

        // Imposter tries to restore v1
        let result = restore_version(&pool, &recipe.id, &imposter.id, 1).await;
        assert!(matches!(result, Err(DbError::RecipeNotFound)));
    }

    #[tokio::test]
    async fn test_restore_version_not_found() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (user, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Try to restore a version that doesn't exist
        let result = restore_version(&pool, &recipe.id, &user.id, 99).await;
        assert!(matches!(result, Err(DbError::VersionNotFound)));
    }

  // ── CP7: Fork tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_fork_creates_new_draft() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (_owner, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Make recipe public so the forker can access it
        sqlx::query("UPDATE recipes SET is_public = TRUE WHERE id = $1")
            .bind(recipe.id)
            .execute(&pool)
            .await
            .unwrap();

        let forker = insert_user(
            &pool,
            "forker_user",
            "Forker",
            "forker@test.com",
            None,
        )
        .await
        .unwrap();

        let (new_id, orig_id, orig_title) = fork_recipe(&pool, recipe.id, forker.id, None)
            .await
            .unwrap();

        // New recipe is different from original
        assert_ne!(new_id, recipe.id);
        assert_eq!(orig_id, recipe.id);
        assert_eq!(orig_title, "Test Recipe");

        // New recipe is owned by forker, is draft, not public
        let new_recipe: Recipe = sqlx::query_as!(
            Recipe,
            r#"SELECT id, owner_id, title, description, is_public, is_draft,
               prep_time_min, cook_time_min, total_time_min, servings,
               ingredients, steps, created_at, updated_at
               FROM recipes WHERE id = $1"#,
            new_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(new_recipe.owner_id, forker.id);
        assert!(new_recipe.is_draft);
        assert!(!new_recipe.is_public);
        assert_eq!(new_recipe.title, "Test Recipe");

        // New recipe has v1
        let new_versions = get_all_versions(&pool, new_id).await.unwrap();
        assert_eq!(new_versions.len(), 1);
        assert_eq!(new_versions[0].version_number, 1);
    }

    #[tokio::test]
    async fn test_fork_records_relationship() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (owner, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Make recipe public so the forker can access it
        sqlx::query("UPDATE recipes SET is_public = TRUE WHERE id = $1")
            .bind(recipe.id)
            .execute(&pool)
            .await
            .unwrap();

        let forker = insert_user(
            &pool,
            "forker_rel",
            "Forker Rel",
            "forkerrel@test.com",
            None,
        )
        .await
        .unwrap();

        let message = "Love this recipe!".to_string();
        let (new_id, _orig_id, _orig_title) =
            fork_recipe(&pool, recipe.id, forker.id, Some(message.clone()))
                .await
                .unwrap();

        let fork_info = get_fork_info(&pool, new_id).await.unwrap();
        assert!(fork_info.is_some());
        let (orig_id, orig_owner, fork_message) = fork_info.unwrap();
        assert_eq!(orig_id, recipe.id);
        assert_eq!(orig_owner, owner.id);
        assert_eq!(fork_message, Some(message));
    }

    #[tokio::test]
    async fn test_fork_private_recipe_fails() {
        let (_db, pool) = test_utils::setup_test_db().await;
        let (_owner, recipe) = test_utils::create_test_user_and_recipe(&pool).await;

        // Set is_public = false via raw SQL (the test fixture creates a public recipe)
        sqlx::query("UPDATE recipes SET is_public = FALSE WHERE id = $1")
            .bind(recipe.id)
            .execute(&pool)
            .await
            .unwrap();

        let forker = insert_user(
            &pool,
            "forker_private",
            "Forker Private",
            "forkerprivate@test.com",
            None,
        )
        .await
        .unwrap();

        let result = fork_recipe(&pool, recipe.id, forker.id, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DbError::ForkError));
    }
}
