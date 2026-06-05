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
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::MissingUrl => write!(f, "DATABASE_URL not set"),
            DbError::Connection(e) => write!(f, "database connection failed: {e}"),
            DbError::Query(e) => write!(f, "database query failed: {e}"),
            DbError::UsernameTaken => write!(f, "username is already taken"),
            DbError::SessionInvalid => write!(f, "session is invalid, revoked, or expired"),
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

// ── Auth state queries ──────────────────────────────────────────────────────

/// Insert an auth state for OAuth CSRF protection.
pub async fn insert_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    provider: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO auth_states (id, provider, redirect_uri, code_verifier) VALUES ($1, $2, $3, $4)")
        .bind(id)
        .bind(provider)
        .bind(redirect_uri)
        .bind(code_verifier)
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
        "DELETE FROM auth_states WHERE id = $1 RETURNING id, redirect_uri, provider, code_verifier, created_at",
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
        insert_auth_state(&pool, &state_id, "google", "/dashboard", verifier)
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
}
