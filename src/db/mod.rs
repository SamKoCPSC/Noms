//! Database layer: connection pool, types, and query functions.
//!
//! Only compiled when the `server` feature is enabled.
// TODO: Remove after checkpoint 4 wires up callers (account linking, OAuth handlers).
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
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::MissingUrl => write!(f, "DATABASE_URL not set"),
            DbError::Connection(e) => write!(f, "database connection failed: {e}"),
            DbError::Query(e) => write!(f, "database query failed: {e}"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::MissingUrl => None,
            DbError::Connection(e) | DbError::Query(e) => Some(e),
        }
    }
}

// ── Connection pool ─────────────────────────────────────────────────────────

/// Create a new connection pool from the `DATABASE_URL` environment variable.
///
/// Validates connectivity — returns an error if the database is unreachable.
/// The caller is responsible for storing the returned pool (e.g., in Axum state).
pub async fn create_pool() -> Result<PgPool, DbError> {
    let url = std::env::var("DATABASE_URL").map_err(|_| DbError::MissingUrl)?;
    PgPool::connect(&url).await.map_err(DbError::Connection)
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
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

/// Short-lived state for OAuth CSRF protection.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthState {
    pub id: String,
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
}

// ── Auth state queries ──────────────────────────────────────────────────────

/// Insert an auth state for OAuth CSRF protection.
pub async fn insert_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
    redirect_uri: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT INTO auth_states (id, redirect_uri) VALUES ($1, $2)")
        .bind(id)
        .bind(redirect_uri)
        .execute(executor)
        .await
        .map_err(DbError::Query)?;
    Ok(())
}

/// Get an auth state by ID.
pub async fn get_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
) -> Result<Option<AuthState>, DbError> {
    sqlx::query_as!(
        AuthState,
        "SELECT id, redirect_uri, created_at FROM auth_states WHERE id = $1",
        id,
    )
    .fetch_optional(executor)
    .await
    .map_err(DbError::Query)
}

/// Delete an auth state by ID. Returns `true` if a row was deleted.
pub async fn delete_auth_state(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM auth_states WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await
        .map_err(DbError::Query)?
        .rows_affected();
    Ok(rows > 0)
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
         profile_data, created_at, last_used_at \
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
         profile_data, created_at, last_used_at \
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
) -> Result<OauthAccount, DbError> {
    sqlx::query_as!(
        OauthAccount,
        "INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email, profile_data) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, user_id, provider, provider_user_id, email, email_verified, \
         profile_data, created_at, last_used_at",
        user_id,
        provider,
        provider_user_id,
        email,
        profile_data,
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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pgtemp::PgTempDB;

    /// Spawn a fresh temporary PostgreSQL database and apply the NOMS schema.
    /// Each test gets its own isolated database — no shared state, no cleanup needed.
    async fn setup_test_db() -> (PgTempDB, PgPool) {
        let db = PgTempDB::async_new().await;
        let pool = PgPool::connect(&db.connection_uri())
            .await
            .expect("failed to connect to temp database");
        apply_test_schema(&pool).await;
        (db, pool)
    }

    /// Create tables and extensions needed for tests.
    /// Uses raw (non-macro) queries so this works without compile-time checking.
    async fn apply_test_schema(pool: &PgPool) {
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
            .execute(pool)
            .await
            .expect("failed to create pgcrypto extension");

        // pg_cron is optional — skip silently if the extension binary isn't installed.
        let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_cron")
            .execute(pool)
            .await;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (\
             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
             username VARCHAR(30) UNIQUE NOT NULL,\
             display_name VARCHAR(100) NOT NULL,\
             email VARCHAR(255) UNIQUE NOT NULL,\
             avatar_url TEXT,\
             bio TEXT,\
             created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
             updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
             )",
        )
        .execute(pool)
        .await
        .expect("failed to create users table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS oauth_accounts (\
             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
             user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
             provider VARCHAR(20) NOT NULL,\
             provider_user_id VARCHAR(255) NOT NULL,\
             email VARCHAR(255),\
             email_verified BOOLEAN NOT NULL DEFAULT FALSE,\
             profile_data JSONB,\
             created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
             last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
             UNIQUE(provider, provider_user_id),\
             CONSTRAINT valid_oauth_provider CHECK (provider IN ('google', 'apple', 'github'))\
             )",
        )
        .execute(pool)
        .await
        .expect("failed to create oauth_accounts table");

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_accounts_email ON oauth_accounts(email)")
            .execute(pool)
            .await
            .expect("failed to create email index");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS auth_states (\
             id VARCHAR(64) PRIMARY KEY,\
             redirect_uri TEXT NOT NULL,\
             created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
             )",
        )
        .execute(pool)
        .await
        .expect("failed to create auth_states table");
    }

    /// Generate a unique suffix for test data to avoid duplicate key conflicts.
    fn uid() -> String {
        Uuid::new_v4().to_string()[..8].to_string()
    }

    #[tokio::test]
    async fn test_insert_and_get_user() {
        let (_db, pool) = setup_test_db().await;
        let u = uid();
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
        let (_db, pool) = setup_test_db().await;
        let fake_id = Uuid::nil();
        let result = get_user_by_id(&pool, fake_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_insert_and_get_auth_state() {
        let (_db, pool) = setup_test_db().await;
        let state_id = format!("test-state-{}", uid());
        insert_auth_state(&pool, &state_id, "/dashboard")
            .await
            .unwrap();

        let state = get_auth_state(&pool, &state_id).await.unwrap();
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state.id, state_id);
        assert_eq!(state.redirect_uri, "/dashboard");
    }

    #[tokio::test]
    async fn test_delete_auth_state() {
        let (_db, pool) = setup_test_db().await;
        let state_id = format!("test-state-del-{}", uid());
        insert_auth_state(&pool, &state_id, "/login").await.unwrap();

        let deleted = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(deleted);

        // Should be gone
        let state = get_auth_state(&pool, &state_id).await.unwrap();
        assert!(state.is_none());

        // Delete again should return false
        let deleted_again = delete_auth_state(&pool, &state_id).await.unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_insert_and_get_oauth_account() {
        let (_db, pool) = setup_test_db().await;
        let u = uid();

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
        let (_db, pool) = setup_test_db().await;
        let u = uid();

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
}
