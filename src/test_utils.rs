//! Shared test infrastructure for server-side integration tests.
//!
//! Provides temporary PostgreSQL databases (via `pgtemp`) with the full
//! NOMS schema applied. Only compiled in test builds with the `server` feature.

use sqlx::PgPool;
use uuid::Uuid;

/// Connect to the dedicated ParadeDB test database and prove BM25 indexes work.
///
/// Unlike [`setup_test_db`], this deliberately does not fall back to the host
/// PostgreSQL installation. Search integration tests must set
/// `SEARCH_TEST_DATABASE_URL` to the repository's pinned PostgreSQL image.
pub async fn setup_search_test_db() -> PgPool {
    let database_url = std::env::var("SEARCH_TEST_DATABASE_URL")
        .expect("SEARCH_TEST_DATABASE_URL must point to the pinned ParadeDB test database");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to SEARCH_TEST_DATABASE_URL");

    let extension_version = sqlx::query_scalar::<_, String>(
        "SELECT extversion FROM pg_extension WHERE extname = 'pg_search'",
    )
    .fetch_optional(&pool)
    .await
    .expect("failed to inspect installed PostgreSQL extensions")
    .expect("pg_search is required for search integration tests");
    assert!(
        !extension_version.trim().is_empty(),
        "pg_search reported an empty extension version"
    );

    verify_bm25_index_creation(&pool).await;
    pool
}

async fn verify_bm25_index_creation(pool: &PgPool) {
    let suffix = Uuid::new_v4().simple().to_string();
    let table_name = format!("noms_search_smoke_{suffix}");
    let index_name = format!("idx_noms_search_smoke_{suffix}");
    let field_alias = format!("noms_search_smoke_body_{suffix}");

    sqlx::query(&format!(
        "CREATE TABLE {table_name} (id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY, body TEXT NOT NULL)"
    ))
    .execute(pool)
    .await
    .expect("failed to create ParadeDB smoke-test table");

    let create_index_result = sqlx::query(&format!(
        "CREATE INDEX {index_name} ON {table_name} USING bm25 (id, (body::pdb.simple('alias={field_alias}'))) WITH (key_field = 'id')"
    ))
    .execute(pool)
    .await;

    let cleanup_result = sqlx::query(&format!("DROP TABLE {table_name}"))
        .execute(pool)
        .await;
    cleanup_result.expect("failed to clean up ParadeDB smoke-test table");
    create_index_result.expect("pg_search could not create a BM25 index");
}

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
         user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
         title VARCHAR(255) NOT NULL,\
         description TEXT,\
         commentary TEXT,\
         visibility VARCHAR(20) NOT NULL DEFAULT 'private',\
         prep_time_minutes INT,\
         cook_time_minutes INT,\
         servings INT,\
         ingredients JSONB DEFAULT '[]'::jsonb,\
         instructions JSONB DEFAULT '[]'::jsonb,\
    equipment JSONB DEFAULT '[]'::jsonb,\
          images JSONB NOT NULL DEFAULT '[]'::jsonb,\
          created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         CONSTRAINT valid_recipe_visibility CHECK (visibility IN ('private', 'unlisted', 'public'))\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create recipes table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS recipe_tags (\
         recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,\
         tag VARCHAR(100) NOT NULL,\
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
         version_number INTEGER NOT NULL CHECK (version_number > 0),\
         snapshot JSONB NOT NULL,\
         notes TEXT,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         UNIQUE (recipe_id, version_number)\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create recipe_versions table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS recipe_drafts (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,\
         user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
         base_version_number INTEGER CHECK (base_version_number IS NULL OR base_version_number > 0),\
         snapshot JSONB NOT NULL,\
         revision BIGINT NOT NULL DEFAULT 0,\
         forked_from_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,\
         forked_from_version_number INTEGER CHECK (forked_from_version_number IS NULL OR forked_from_version_number > 0),\
         forked_from_title VARCHAR(255),\
         fork_message TEXT,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create recipe_drafts table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS fork_relationships (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         original_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,\
         forked_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,\
         forked_by UUID REFERENCES users(id) ON DELETE SET NULL,\
         parent_relationship_id UUID REFERENCES fork_relationships(id) ON DELETE SET NULL,\
         original_recipe_key UUID,\
         forked_recipe_key UUID,\
         forked_version_number INTEGER CHECK (forked_version_number IS NULL OR forked_version_number > 0),\
         original_title VARCHAR(255) NOT NULL,\
         forked_title VARCHAR(255),\
         original_owner_id UUID,\
         original_owner_username VARCHAR(30),\
         original_visibility VARCHAR(20),\
         forked_owner_id UUID,\
         forked_owner_username VARCHAR(30),\
         forked_visibility VARCHAR(20),\
         message TEXT,\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create fork_relationships table");

    for statement in [
        "CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe ON recipe_versions(recipe_id, version_number DESC)",
        "CREATE INDEX IF NOT EXISTS idx_recipe_drafts_user ON recipe_drafts(user_id, updated_at DESC)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_recipe_drafts_existing_recipe ON recipe_drafts(user_id, recipe_id) WHERE recipe_id IS NOT NULL",
        "CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id)",
        "CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id)",
        "CREATE INDEX IF NOT EXISTS idx_forks_original_key ON fork_relationships(original_recipe_key)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_forks_result_key ON fork_relationships(forked_recipe_key) WHERE forked_recipe_key IS NOT NULL",
        "CREATE INDEX IF NOT EXISTS idx_forks_parent ON fork_relationships(parent_relationship_id)",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("failed to create NOMS-010 index");
    }

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipes_user_id ON recipes(user_id)")
        .execute(pool)
        .await
        .expect("failed to create recipes user_id index");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_recipes_created_at ON recipes(created_at DESC)")
        .execute(pool)
        .await
        .expect("failed to create recipes created_at index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collections (\
         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\
         owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,\
         parent_collection_id UUID,\
         name VARCHAR(100) NOT NULL,\
         description TEXT,\
         image JSONB CHECK (image IS NULL OR jsonb_typeof(image) = 'object'),\
         visibility VARCHAR(20) NOT NULL DEFAULT 'private'\
             CHECK (visibility IN ('private', 'unlisted', 'public')),\
         sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         UNIQUE (id, owner_id),\
         CHECK (parent_collection_id IS NULL OR parent_collection_id <> id),\
         FOREIGN KEY (parent_collection_id, owner_id)\
             REFERENCES collections(id, owner_id) ON DELETE CASCADE,\
         CHECK (btrim(name) <> '')\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create collections table");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_collections_owner_updated
         ON collections(owner_id, updated_at DESC)",
    )
    .execute(pool)
    .await
    .expect("failed to create collections owner index");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_collections_public_updated
         ON collections(updated_at DESC, id)
         WHERE visibility = 'public'",
    )
    .execute(pool)
    .await
    .expect("failed to create collections public discovery index");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_collections_parent_order
         ON collections(owner_id, parent_collection_id, sort_order, lower(name))",
    )
    .execute(pool)
    .await
    .expect("failed to create collections parent index");

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_sibling_name
         ON collections(owner_id, parent_collection_id, lower(name))
         NULLS NOT DISTINCT",
    )
    .execute(pool)
    .await
    .expect("failed to create collections name index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS collection_recipes (\
         collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,\
         recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,\
         sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),\
         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
         PRIMARY KEY (collection_id, recipe_id)\
         )",
    )
    .execute(pool)
    .await
    .expect("failed to create collection_recipes table");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_collection_recipes_order
         ON collection_recipes(collection_id, sort_order, created_at)",
    )
    .execute(pool)
    .await
    .expect("failed to create collection_recipes order index");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_collection_recipes_recipe_id
         ON collection_recipes(recipe_id)",
    )
    .execute(pool)
    .await
    .expect("failed to create collection_recipes recipe index");
}

/// Generate a unique 8-character suffix for test data to avoid duplicate key conflicts.
pub fn uid() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

/// Publish a complete recipe with immutable version 1 for integration tests.
#[allow(dead_code)]
pub async fn create_test_published_recipe(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
) -> crate::db::Recipe {
    crate::db::create_published_recipe(
        pool,
        user_id,
        &crate::db::diff::RecipeDraftPayload {
            content: test_recipe_snapshot(title),
            initial_visibility: Some("private".to_string()),
        },
        None,
    )
    .await
    .expect("failed to create test published recipe")
}

/// Create or load the owner's draft for an existing published recipe.
#[allow(dead_code)]
pub async fn create_test_edit_draft(
    pool: &PgPool,
    user_id: Uuid,
    recipe_id: Uuid,
) -> crate::db::RecipeDraft {
    crate::db::create_edit_draft(pool, recipe_id, user_id)
        .await
        .expect("failed to create test edit draft")
}

/// Create an incomplete, unsaved new-recipe draft without creating a version.
#[allow(dead_code)]
pub async fn create_test_new_draft(pool: &PgPool, user_id: Uuid) -> crate::db::RecipeDraft {
    crate::db::create_new_recipe_draft(
        pool,
        user_id,
        &crate::db::diff::RecipeDraftPayload {
            content: test_recipe_snapshot(""),
            initial_visibility: Some("private".to_string()),
        },
    )
    .await
    .expect("failed to create test new draft")
}

#[allow(dead_code)]
fn test_recipe_snapshot(title: &str) -> crate::db::diff::RecipeSnapshot {
    crate::db::diff::RecipeSnapshot {
        title: title.to_string(),
        description: None,
        commentary: None,
        prep_time_minutes: None,
        cook_time_minutes: None,
        servings: None,
        ingredients: vec![crate::types::RecipeIngredient {
            amount: "1".to_string(),
            unit: "cup".to_string(),
            name: "test ingredient".to_string(),
        }],
        instructions: vec![crate::types::RecipeStep {
            text: "Test instruction".to_string(),
            sub_steps: Vec::new(),
            photos: Vec::new(),
        }],
        equipment: Vec::new(),
        images: Vec::new(),
        tags: Vec::new(),
    }
}

/// Insert a user with unique test-only identity fields.
pub async fn insert_test_user(pool: &PgPool) -> crate::db::User {
    let suffix = uid();
    crate::db::insert_user(
        pool,
        &format!("collections_{suffix}"),
        "Collections Test User",
        &format!("collections_{suffix}@example.com"),
        None,
    )
    .await
    .expect("failed to insert test user")
}

/// Insert a published or private recipe with empty structured recipe fields.
pub async fn insert_test_recipe(
    pool: &PgPool,
    user_id: Uuid,
    visibility: &str,
) -> crate::types::Recipe {
    crate::db::insert_recipe(
        pool,
        user_id,
        "Collection Test Recipe",
        None,
        None,
        None,
        None,
        None,
        &[],
        &[],
        &[],
        &[],
        visibility,
    )
    .await
    .expect("failed to insert test recipe")
}

/// Insert a collection through the production query path.
pub async fn insert_test_collection(
    pool: &PgPool,
    owner_id: Uuid,
    parent_collection_id: Option<Uuid>,
    name: &str,
) -> crate::types::Collection {
    crate::db::insert_collection(
        pool,
        owner_id,
        parent_collection_id,
        name,
        None,
        crate::types::CollectionVisibility::Private,
        None,
    )
    .await
    .expect("failed to insert test collection")
}

/// Add a recipe to a collection through the production query path.
pub async fn add_test_recipe_to_collection(
    pool: &PgPool,
    collection_id: Uuid,
    owner_id: Uuid,
    recipe_id: Uuid,
) -> crate::db::CollectionRecipe {
    crate::db::add_recipe_to_collection(pool, collection_id, owner_id, recipe_id)
        .await
        .expect("failed to add test recipe to collection")
}

#[cfg(test)]
mod search_integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
    async fn search_integration_paradedb_database_is_available() {
        let pool = setup_search_test_db().await;
        pool.close().await;
    }
}
