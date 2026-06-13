//! Shared test infrastructure for server-side integration tests.
//!
//! Provides temporary PostgreSQL databases (via `pgtemp`) with the full
//! NOMS schema applied. Only compiled in test builds with the `server` feature.

use sqlx::PgPool;
use uuid::Uuid;

use crate::db;

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

    // ── Recipe tables ──────────────────────────────────────────────────────

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS recipes (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
         title VARCHAR(200) NOT NULL,\
         description TEXT,\
         is_public BOOLEAN NOT NULL DEFAULT FALSE,\
         is_draft BOOLEAN NOT NULL DEFAULT FALSE,\
         prep_time_min INTEGER,\
         cook_time_min INTEGER,\
         total_time_min INTEGER,\
         servings INTEGER,\
         ingredients JSONB NOT NULL DEFAULT '[]'::jsonb,\
         steps JSONB NOT NULL DEFAULT '[]'::jsonb,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create recipes table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipes_owner_id ON recipes(owner_id)")
        .execute(pool)
        .await
        .expect("failed to create recipes owner_id index");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipes_updated_at ON recipes(updated_at DESC)")
        .execute(pool)
        .await
        .expect("failed to create recipes updated_at index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS recipe_tags (\
         recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,\
         tag TEXT NOT NULL,\
         PRIMARY KEY (recipe_id, tag)\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create recipe_tags table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS recipe_versions (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,\
         version_number INTEGER NOT NULL,\
         title VARCHAR(200),\
         description TEXT,\
         prep_time_min INTEGER,\
         cook_time_min INTEGER,\
         total_time_min INTEGER,\
         servings INTEGER,\
         ingredients JSONB,\
         steps JSONB,\
         reverse_diff JSONB,\
         notes TEXT,\
         is_latest BOOLEAN NOT NULL DEFAULT FALSE,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         UNIQUE(recipe_id, version_number)\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create recipe_versions table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe ON recipe_versions(recipe_id, version_number DESC)")
        .execute(pool)
        .await
        .expect("failed to create recipe_versions index");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipe_versions_latest ON recipe_versions(recipe_id, is_latest) WHERE is_latest = TRUE")
        .execute(pool)
        .await
        .expect("failed to create recipe_versions latest index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS fork_relationships (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         original_recipe_id UUID NOT NULL REFERENCES recipes(id),\
         forked_recipe_id UUID NOT NULL REFERENCES recipes(id),\
         forked_by UUID NOT NULL REFERENCES users(id),\
         forked_version_number INTEGER,\
         message TEXT,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create fork_relationships table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id)")
        .execute(pool)
        .await
        .expect("failed to create forks original index");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id)")
        .execute(pool)
        .await
        .expect("failed to create forks result index");
}

/// Generate a unique 8-character suffix for test data to avoid duplicate key conflicts.
pub fn uid() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

/// Insert a test recipe for a given owner and return its ID.
///
/// Creates the recipe row AND a v1 version entry (full snapshot, is_latest = true).
/// Uses minimal default data so tests can focus on versioning logic.
#[allow(dead_code)]
pub async fn insert_test_recipe(pool: &PgPool, owner_id: Uuid, title: &str) -> Uuid {
    let recipe_id: Uuid = sqlx::query_scalar(
        "INSERT INTO recipes (owner_id, title, ingredients, steps) \
         VALUES ($1, $2, '[]'::jsonb, '[]'::jsonb) \
         RETURNING id",
    )
    .bind(owner_id)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("failed to insert test recipe");

    // Backfill v1 version
    sqlx::query(
        "INSERT INTO recipe_versions (recipe_id, version_number, title, ingredients, steps, is_latest) \
         VALUES ($1, 1, $2, '[]'::jsonb, '[]'::jsonb, TRUE)",
    )
    .bind(recipe_id)
    .bind(title)
    .execute(pool)
    .await
    .expect("failed to insert test recipe v1");

    recipe_id
}

/// Create a test user and recipe (with v1 version) in one call.
///
/// Returns `(User, Recipe)` so that CP3 tests can verify ownership
/// and versioning in a single setup step.
pub async fn create_test_user_and_recipe(
    pool: &PgPool,
) -> (db::User, db::Recipe) {
    let u = uid();
    let user = db::insert_user(
        pool,
        &format!("testuser_{u}"),
        "Test User",
        &format!("test{u}@example.com"),
        None,
    )
    .await
    .expect("failed to insert test user");

    let recipe = db::insert_recipe(
        pool,
        user.id,
        "Test Recipe",
        Some("A test recipe"),
        false,
        false,
        Some(10),
        Some(20),
        Some(30),
        Some(4),
        Some(serde_json::json!(["flour", "sugar", "eggs"])),
        Some(serde_json::json!(["Mix", "Bake", "Cool"])),
    )
    .await
    .expect("failed to insert test recipe");

    // Backfill v1 version as latest
    db::insert_latest_version(
        pool,
        recipe.id,
        1,
        "Test Recipe",
        Some("A test recipe"),
        Some(10),
        Some(20),
        Some(30),
        Some(4),
        Some(serde_json::json!(["flour", "sugar", "eggs"])).as_ref(),
        Some(serde_json::json!(["Mix", "Bake", "Cool"])).as_ref(),
        None,
    )
    .await
    .expect("failed to insert test recipe v1");

    (user, recipe)
}
