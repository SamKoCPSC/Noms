//! Shared test infrastructure for server-side integration tests.
//!
//! Provides temporary PostgreSQL databases (via `pgtemp`) with the full
//! NOMS schema applied. Only compiled in test builds with the `server` feature.

use sqlx::PgPool;
use uuid::Uuid;

/// Spawn a fresh temporary PostgreSQL database and apply the NOMS schema.
///
/// Each call creates an isolated database — no shared state, no cleanup needed.
/// The returned `PgTempDB` handle must stay in scope for the duration of the test;
/// dropping it cleans up the temporary database.
pub async fn setup_test_db() -> (pgtemp::PgTempDB, PgPool) {
    let db = pgtemp::PgTempDB::async_new().await;
    let pool = PgPool::connect(&db.connection_uri())
        .await
        .expect("failed to connect to temp database");
    apply_test_schema(&pool).await;
    (db, pool)
}

/// Create tables and extensions needed for tests.
///
/// Uses raw (non-macro) queries so this works without compile-time checking.
/// Optional extensions are created with `IF NOT EXISTS` and failures are
/// silently ignored (they may not be installed in the test environment).
pub async fn apply_test_schema(pool: &PgPool) {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .execute(pool)
        .await
        .expect("failed to create pgcrypto extension");

    // Optional extensions — skip silently if not installed.
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_cron")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_trgm")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search")
        .execute(pool)
        .await;
    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS timescaledb")
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
         refresh_token TEXT,\
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

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user_id ON oauth_accounts(user_id)")
        .execute(pool)
        .await
        .expect("failed to create user_id index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auth_states (\
         id VARCHAR(64) PRIMARY KEY,\
         redirect_uri TEXT NOT NULL,\
         provider TEXT NOT NULL,\
         code_verifier TEXT,\
         user_id UUID,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create auth_states table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_auth_states_created_at ON auth_states(created_at)")
        .execute(pool)
        .await
        .expect("failed to create auth_states created_at index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '15 minutes'),\
         refreshed_at TIMESTAMPTZ,\
         revoked BOOLEAN NOT NULL DEFAULT FALSE\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create sessions table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)")
        .execute(pool)
        .await
        .expect("failed to create sessions user_id index");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_cleanup ON sessions(expires_at, revoked) WHERE revoked = TRUE")
        .execute(pool)
        .await
        .expect("failed to create sessions cleanup index");
}

/// Generate a unique 8-character suffix for test data to avoid duplicate key conflicts.
pub fn uid() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}
