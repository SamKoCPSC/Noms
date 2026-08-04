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
    use crate::types::{
        AdvancedRecipeSearchCriteria, AdvancedTextField, IntegerRange, RecipeSearchRequest,
        RecipeSearchSort, RecipeSearchVisibility, RecipeVisibilityFacetValue, TextCriterion,
        TextMatchMode, TimestampRange,
    };
    use chrono::{Duration, Utc};

    async fn index_exists(pool: &PgPool, index_name: &str) -> bool {
        sqlx::query_scalar::<_, bool>("SELECT to_regclass($1) IS NOT NULL")
            .bind(format!("public.{index_name}"))
            .fetch_one(pool)
            .await
            .expect("failed to inspect search index")
    }

    async fn recipe_matches(pool: &PgPool, recipe_id: Uuid, predicate: &str, text: &str) -> bool {
        let query = format!("SELECT EXISTS (SELECT 1 FROM recipes WHERE id = $2 AND {predicate})");
        sqlx::query_scalar::<_, bool>(&query)
            .bind(text)
            .bind(recipe_id)
            .fetch_one(pool)
            .await
            .expect("failed to execute recipe BM25 smoke query")
    }

    async fn user_matches(pool: &PgPool, user_id: Uuid, predicate: &str, text: &str) -> bool {
        let query = format!("SELECT EXISTS (SELECT 1 FROM users WHERE id = $2 AND {predicate})");
        sqlx::query_scalar::<_, bool>(&query)
            .bind(text)
            .bind(user_id)
            .fetch_one(pool)
            .await
            .expect("failed to execute user BM25 smoke query")
    }

    struct RecipeFixture<'a> {
        id: Uuid,
        owner_id: Uuid,
        title: &'a str,
        description: &'a str,
        visibility: &'a str,
        prep_time: Option<i32>,
        cook_time: Option<i32>,
        servings: Option<i32>,
        ingredients: serde_json::Value,
        instructions: serde_json::Value,
        created_at: chrono::DateTime<Utc>,
        updated_at: chrono::DateTime<Utc>,
        tags: Vec<String>,
    }

    async fn insert_search_user(pool: &PgPool, user_id: Uuid, suffix: &str, label: &str) {
        sqlx::query(
            "INSERT INTO users (id, username, display_name, email) VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(format!("{label}{suffix}"))
        .bind(format!("{label} {suffix}"))
        .bind(format!("{label}-{suffix}@example.test"))
        .execute(pool)
        .await
        .expect("failed to insert recipe-search user");
    }

    async fn insert_search_recipe(pool: &PgPool, fixture: RecipeFixture<'_>) {
        sqlx::query(
            "INSERT INTO recipes (id, user_id, title, description, prep_time_minutes, \
             cook_time_minutes, servings, ingredients, instructions, visibility, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(fixture.id)
        .bind(fixture.owner_id)
        .bind(fixture.title)
        .bind(fixture.description)
        .bind(fixture.prep_time)
        .bind(fixture.cook_time)
        .bind(fixture.servings)
        .bind(fixture.ingredients)
        .bind(fixture.instructions)
        .bind(fixture.visibility)
        .bind(fixture.created_at)
        .bind(fixture.updated_at)
        .execute(pool)
        .await
        .expect("failed to insert recipe-search fixture");

        for tag in fixture.tags {
            sqlx::query("INSERT INTO recipe_tags (recipe_id, tag) VALUES ($1, $2)")
                .bind(fixture.id)
                .bind(tag)
                .execute(pool)
                .await
                .expect("failed to insert recipe-search tag");
        }
    }

    fn criterion(text: impl Into<String>, mode: TextMatchMode) -> TextCriterion {
        TextCriterion {
            text: text.into(),
            mode,
        }
    }

    #[tokio::test]
    #[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
    async fn search_integration_paradedb_database_is_available() {
        let pool = setup_search_test_db().await;
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
    async fn search_integration_indexes_support_required_queries() {
        let pool = setup_search_test_db().await;

        assert!(index_exists(&pool, "idx_recipes_search").await);
        assert!(index_exists(&pool, "idx_users_search").await);
        assert!(index_exists(&pool, "idx_recipe_tags_tag").await);

        for index_name in ["idx_recipes_search", "idx_users_search"] {
            let has_key_field = sqlx::query_scalar::<_, bool>(
                "SELECT COALESCE(reloptions @> ARRAY['key_field=id'], false) \
                 FROM pg_class WHERE oid = $1::regclass",
            )
            .bind(index_name)
            .fetch_one(&pool)
            .await
            .expect("failed to inspect ParadeDB key_field option");
            assert!(has_key_field, "{index_name} must retain key_field=id");
        }

        let collections_exist =
            sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.collections') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .expect("failed to inspect collections table");
        if collections_exist {
            assert!(index_exists(&pool, "idx_collections_search").await);
        }

        let suffix = uid();
        let user_id = Uuid::new_v4();
        let recipe_id = Uuid::new_v4();
        let username = format!("solarchef{suffix}");
        let title = format!("Solar Saffron Risotto {suffix}");
        let updated_title = format!("Comet Cardamom Pilaf {suffix}");

        sqlx::query(
            "INSERT INTO users (id, username, display_name, email, bio) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(&username)
        .bind(format!("Solar Chef {suffix}"))
        .bind(format!("solar-{suffix}@example.test"))
        .bind("Cooks bright seasonal food")
        .execute(&pool)
        .await
        .expect("failed to insert search smoke-test user");

        let ingredients = serde_json::json!([
            {"name": "saffron", "notes": "golden thread"}
        ]);
        let instructions = serde_json::json!([{
            "text": "Toast the arborio rice gently",
            "photos": [{"url": "photo-secret-metadata"}],
            "sub_steps": [{"text": "Fold in parmesan slowly"}]
        }]);
        sqlx::query(
            "INSERT INTO recipes \
             (id, user_id, title, description, ingredients, instructions, visibility) \
             VALUES ($1, $2, $3, $4, $5, $6, 'public')",
        )
        .bind(recipe_id)
        .bind(user_id)
        .bind(&title)
        .bind("Creamy orchard supper")
        .bind(ingredients)
        .bind(instructions)
        .execute(&pool)
        .await
        .expect("failed to insert search smoke-test recipe");
        sqlx::query("INSERT INTO recipe_tags (recipe_id, tag) VALUES ($1, $2)")
            .bind(recipe_id)
            .bind(format!("risotto-{suffix}"))
            .execute(&pool)
            .await
            .expect("failed to insert search smoke-test tag");

        let recipe_title_words =
            "(title::pdb.simple('alias=recipe_title_words')) @@@ pdb.match_conjunction($1)";
        let recipe_title_partial =
            "(title::pdb.ngram(2, 15, 'alias=recipe_title_partial')) @@@ pdb.match_conjunction($1)";
        let recipe_description_words = "(description::pdb.simple(\
            'alias=recipe_description_words')) @@@ pdb.match_conjunction($1)";
        let recipe_description_partial = "(description::pdb.ngram(2, 15, \
            'alias=recipe_description_partial')) @@@ pdb.match_conjunction($1)";
        let recipe_title_phrase =
            "(title::pdb.simple('alias=recipe_title_words')) @@@ pdb.phrase($1)";
        let recipe_ingredients_words = "(jsonb_path_query_array(ingredients, \
            'strict $.** ? (@.type() == \"string\")')::pdb.simple(\
            'alias=recipe_ingredients_words')) @@@ pdb.match_conjunction($1)";
        let recipe_instructions_words = "(jsonb_path_query_array(instructions, \
            'strict $.**.text')::pdb.simple('alias=recipe_instructions_words')) \
            @@@ pdb.match_conjunction($1)";

        assert!(recipe_matches(&pool, recipe_id, recipe_title_words, "saffron").await);
        assert!(recipe_matches(&pool, recipe_id, recipe_title_partial, "affr").await);
        assert!(recipe_matches(&pool, recipe_id, recipe_description_words, "orchard").await);
        assert!(recipe_matches(&pool, recipe_id, recipe_description_partial, "ream").await);
        assert!(recipe_matches(&pool, recipe_id, recipe_title_phrase, "solar saffron").await);
        assert!(!recipe_matches(&pool, recipe_id, recipe_title_phrase, "saffron solar").await);
        assert!(recipe_matches(&pool, recipe_id, recipe_ingredients_words, "golden").await);
        assert!(recipe_matches(&pool, recipe_id, recipe_instructions_words, "parmesan").await);
        assert!(
            !recipe_matches(
                &pool,
                recipe_id,
                recipe_instructions_words,
                "photo secret metadata"
            )
            .await
        );

        let unboosted_score = sqlx::query_scalar::<_, f32>(
            "SELECT pdb.score(id) FROM recipes \
             WHERE id = $2 AND (title::pdb.simple('alias=recipe_title_words')) \
             @@@ pdb.match_conjunction($1)",
        )
        .bind("saffron")
        .bind(recipe_id)
        .fetch_one(&pool)
        .await
        .expect("failed to execute unboosted prepared query");
        let boosted_score = sqlx::query_scalar::<_, f32>(
            "SELECT pdb.score(id) FROM recipes \
             WHERE id = $2 AND (title::pdb.simple('alias=recipe_title_words')) \
             @@@ $1::pdb.boost(3)",
        )
        .bind("saffron")
        .bind(recipe_id)
        .fetch_one(&pool)
        .await
        .expect("failed to execute boosted prepared query");
        assert!(boosted_score > unboosted_score);

        let user_username_words =
            "(username::pdb.simple('alias=user_username_words')) @@@ pdb.match_conjunction($1)";
        let user_username_partial = "(username::pdb.ngram(2, 20, \
            'alias=user_username_partial')) @@@ pdb.match_conjunction($1)";
        let user_display_name_words = "(display_name::pdb.simple(\
            'alias=user_display_name_words')) @@@ pdb.match_conjunction($1)";
        let user_display_name_partial = "(display_name::pdb.ngram(2, 20, \
            'alias=user_display_name_partial')) @@@ pdb.match_conjunction($1)";
        let user_bio_words =
            "(bio::pdb.simple('alias=user_bio_words')) @@@ pdb.match_conjunction($1)";
        assert!(user_matches(&pool, user_id, user_username_words, &username).await);
        assert!(user_matches(&pool, user_id, user_username_partial, "olar").await);
        assert!(user_matches(&pool, user_id, user_display_name_words, "solar").await);
        assert!(user_matches(&pool, user_id, user_display_name_partial, "olar").await);
        assert!(user_matches(&pool, user_id, user_bio_words, "seasonal").await);

        sqlx::query("UPDATE recipes SET title = $1 WHERE id = $2")
            .bind(&updated_title)
            .bind(recipe_id)
            .execute(&pool)
            .await
            .expect("failed to update indexed recipe title");
        assert!(recipe_matches(&pool, recipe_id, recipe_title_words, "cardamom").await);
        assert!(!recipe_matches(&pool, recipe_id, recipe_title_words, "saffron").await);

        let recipe_plan = sqlx::query_scalar::<_, String>(
            "EXPLAIN (COSTS OFF) SELECT id FROM recipes \
             WHERE (title::pdb.simple('alias=recipe_title_words')) \
             @@@ pdb.match_conjunction('cardamom')",
        )
        .fetch_all(&pool)
        .await
        .expect("failed to explain recipe BM25 query")
        .join("\n");
        assert!(recipe_plan.contains("Index: idx_recipes_search"));

        let user_plan = sqlx::query_scalar::<_, String>(
            "EXPLAIN (COSTS OFF) SELECT id FROM users \
             WHERE (username::pdb.simple('alias=user_username_words')) \
             @@@ pdb.match_conjunction('solarchef')",
        )
        .fetch_all(&pool)
        .await
        .expect("failed to explain user BM25 query")
        .join("\n");
        assert!(user_plan.contains("Index: idx_users_search"));

        let mut connection = pool
            .acquire()
            .await
            .expect("failed to acquire test connection");
        sqlx::query("SET enable_seqscan = off")
            .execute(&mut *connection)
            .await
            .expect("failed to disable sequential scans for tag index proof");
        let tag_plan = sqlx::query_scalar::<_, String>(
            "EXPLAIN (COSTS OFF) SELECT recipe_id FROM recipe_tags WHERE tag = $1",
        )
        .bind(format!("risotto-{suffix}"))
        .fetch_all(&mut *connection)
        .await
        .expect("failed to explain tag lookup")
        .join("\n");
        sqlx::query("RESET enable_seqscan")
            .execute(&mut *connection)
            .await
            .expect("failed to restore sequential scan setting");
        assert!(tag_plan.contains("idx_recipe_tags_tag"));

        let user_index_definition =
            sqlx::query_scalar::<_, String>("SELECT pg_get_indexdef('idx_users_search'::regclass)")
                .fetch_one(&pool)
                .await
                .expect("failed to inspect user search index definition");
        assert!(!user_index_definition.contains("email"));

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("failed to clean up search smoke fixtures");
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
    async fn search_integration_recipe_operation_enforces_authorization_filters_and_facets() {
        let pool = setup_search_test_db().await;
        let suffix = uid();
        let anchor_tag = format!("checkpoint3-{suffix}");
        let owner_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();
        insert_search_user(&pool, owner_id, &suffix, "facetowner").await;
        insert_search_user(&pool, other_id, &suffix, "facetother").await;

        let now = Utc::now();
        let fixtures = [
            (
                owner_id,
                "private",
                Some(10),
                Some(10),
                Some(2),
                "quick",
                -6,
            ),
            (
                owner_id,
                "unlisted",
                Some(20),
                Some(30),
                Some(4),
                "weeknight",
                -5,
            ),
            (
                owner_id,
                "public",
                Some(60),
                Some(70),
                Some(10),
                "project",
                -4,
            ),
            (owner_id, "public", None, None, None, "unknown", -3),
            (
                other_id,
                "public",
                Some(30),
                Some(40),
                Some(6),
                "shared",
                -2,
            ),
            (other_id, "private", Some(5), Some(5), Some(1), "hidden", -1),
        ];
        let mut hidden_other_id = Uuid::nil();
        for (index, (user_id, visibility, prep, cook, servings, tag, day)) in
            fixtures.into_iter().enumerate()
        {
            let recipe_id = Uuid::new_v4();
            if user_id == other_id && visibility == "private" {
                hidden_other_id = recipe_id;
            }
            insert_search_recipe(
                &pool,
                RecipeFixture {
                    id: recipe_id,
                    owner_id: user_id,
                    title: &format!("Facet Recipe {index} {suffix}"),
                    description: "facet coverage",
                    visibility,
                    prep_time: prep,
                    cook_time: cook,
                    servings,
                    ingredients: serde_json::json!([]),
                    instructions: serde_json::json!([]),
                    created_at: now + Duration::days(day),
                    updated_at: now + Duration::days(day),
                    tags: vec![anchor_tag.clone(), format!("{tag}-{suffix}")],
                },
            )
            .await;
        }

        let request = RecipeSearchRequest {
            text: Some("   ".to_string()),
            tags_all: vec![anchor_tag.clone()],
            page_size: 2,
            ..RecipeSearchRequest::default()
        };
        let response = crate::db::search_recipes(&pool, owner_id, &request)
            .await
            .expect("recipe browse search failed");
        assert_eq!(response.total_count, 5);
        assert_eq!(response.results.len(), 2);
        assert!(response
            .results
            .iter()
            .all(|result| result.relevance.is_none()));
        assert!(!response
            .results
            .iter()
            .any(|result| result.recipe.id == hidden_other_id));

        let visibility_counts: Vec<_> = response
            .facets
            .visibility
            .iter()
            .map(|bucket| (bucket.visibility, bucket.count))
            .collect();
        assert_eq!(
            visibility_counts,
            vec![
                (RecipeVisibilityFacetValue::Private, 1),
                (RecipeVisibilityFacetValue::Unlisted, 1),
                (RecipeVisibilityFacetValue::Public, 3),
            ]
        );
        assert_eq!(
            response
                .facets
                .total_time
                .iter()
                .map(|bucket| bucket.count)
                .collect::<Vec<_>>(),
            vec![1, 1, 1, 1]
        );
        assert_eq!(
            response
                .facets
                .servings
                .iter()
                .map(|bucket| bucket.count)
                .collect::<Vec<_>>(),
            vec![1, 1, 1, 1]
        );
        let selected_anchor = response
            .facets
            .tags
            .iter()
            .find(|bucket| bucket.tag == anchor_tag)
            .expect("selected tag facet missing");
        assert_eq!(selected_anchor.count, 5);

        let mine_request = RecipeSearchRequest {
            visibility: RecipeSearchVisibility::MineOnly,
            tags_all: vec![anchor_tag.clone()],
            page_size: 100,
            ..RecipeSearchRequest::default()
        };
        let mine = crate::db::search_recipes(&pool, owner_id, &mine_request)
            .await
            .expect("mine-only recipe search failed");
        assert_eq!(mine.total_count, 4);
        assert!(mine
            .results
            .iter()
            .all(|result| result.recipe.user_id == owner_id));
        assert_eq!(
            mine.facets
                .visibility
                .iter()
                .map(|bucket| bucket.count)
                .collect::<Vec<_>>(),
            vec![1, 1, 3],
            "visibility facet must omit only the mine-only filter"
        );

        let ranged_request = RecipeSearchRequest {
            tags_all: vec![anchor_tag.clone()],
            total_time: Some(IntegerRange {
                lower_inclusive: Some(0),
                upper_exclusive: Some(60),
            }),
            servings: Some(IntegerRange {
                lower_inclusive: Some(1),
                upper_exclusive: Some(5),
            }),
            page_size: 100,
            ..RecipeSearchRequest::default()
        };
        let ranged = crate::db::search_recipes(&pool, owner_id, &ranged_request)
            .await
            .expect("ranged recipe search failed");
        assert_eq!(ranged.total_count, 2);
        assert_eq!(
            ranged
                .facets
                .total_time
                .iter()
                .map(|bucket| bucket.count)
                .collect::<Vec<_>>(),
            vec![1, 1, 0, 0],
            "time facet omits time but retains the servings filter"
        );
        assert_eq!(
            ranged
                .facets
                .servings
                .iter()
                .map(|bucket| bucket.count)
                .collect::<Vec<_>>(),
            vec![1, 1, 0, 0],
            "servings facet omits servings but retains the time filter"
        );

        let public_request = RecipeSearchRequest {
            visibility: RecipeSearchVisibility::PublicOnly,
            tags_all: vec![anchor_tag.clone()],
            page_size: 100,
            ..RecipeSearchRequest::default()
        };
        let public = crate::db::search_recipes(&pool, owner_id, &public_request)
            .await
            .expect("public-only recipe search failed");
        assert_eq!(public.total_count, 3);

        for sort in [
            RecipeSearchSort::Relevance,
            RecipeSearchSort::UpdatedNewest,
            RecipeSearchSort::UpdatedOldest,
            RecipeSearchSort::TitleAscending,
            RecipeSearchSort::TitleDescending,
            RecipeSearchSort::Quickest,
            RecipeSearchSort::Longest,
        ] {
            let sorted_request = RecipeSearchRequest {
                tags_all: vec![anchor_tag.clone()],
                sort,
                page: 2,
                page_size: 2,
                ..RecipeSearchRequest::default()
            };
            let sorted = crate::db::search_recipes(&pool, owner_id, &sorted_request)
                .await
                .expect("recipe sort failed");
            assert_eq!(sorted.total_count, 5);
            assert_eq!(sorted.results.len(), 2);
        }

        sqlx::query("DELETE FROM users WHERE id = ANY($1)")
            .bind(vec![owner_id, other_id])
            .execute(&pool)
            .await
            .expect("failed to clean up facet fixtures");
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "requires SEARCH_TEST_DATABASE_URL backed by the pinned ParadeDB image"]
    async fn search_integration_recipe_operation_handles_text_advanced_tags_and_dates() {
        let pool = setup_search_test_db().await;
        let suffix = uid();
        let anchor_tag = format!("text-{suffix}");
        let owner_id = Uuid::new_v4();
        insert_search_user(&pool, owner_id, &suffix, "textowner").await;
        let now = Utc::now();

        let cross_field_id = Uuid::new_v4();
        insert_search_recipe(
            &pool,
            RecipeFixture {
                id: cross_field_id,
                owner_id,
                title: &format!("Aurora Soup {suffix}"),
                description: "Nebula public OR private supper",
                visibility: "private",
                prep_time: Some(10),
                cook_time: Some(20),
                servings: Some(2),
                ingredients: serde_json::json!([{"amount":"1","unit":"tsp","name":"cardamom"}]),
                instructions: serde_json::json!([{
                    "text":"Toast gently",
                    "sub_steps":[{"text":"Fold in parmesan","sub_steps":[],"photos":[]}],
                    "photos":[{"url":"secret-photo-metadata","edits":[]}]
                }]),
                created_at: now - Duration::days(2),
                updated_at: now - Duration::days(1),
                tags: vec![anchor_tag.clone(), format!("comfort-{suffix}")],
            },
        )
        .await;

        let title_ranked_id = Uuid::new_v4();
        insert_search_recipe(
            &pool,
            RecipeFixture {
                id: title_ranked_id,
                owner_id,
                title: &format!("Quasar Tart {suffix}"),
                description: "ranking fixture",
                visibility: "public",
                prep_time: Some(15),
                cook_time: Some(15),
                servings: Some(4),
                ingredients: serde_json::json!([]),
                instructions: serde_json::json!([]),
                created_at: now - Duration::days(1),
                updated_at: now,
                tags: vec![anchor_tag.clone(), format!("rank-{suffix}")],
            },
        )
        .await;

        let ingredient_ranked_id = Uuid::new_v4();
        insert_search_recipe(
            &pool,
            RecipeFixture {
                id: ingredient_ranked_id,
                owner_id,
                title: &format!("Spiced Tart {suffix}"),
                description: "ranking fixture",
                visibility: "public",
                prep_time: Some(20),
                cook_time: Some(20),
                servings: Some(4),
                ingredients: serde_json::json!([{"amount":"1","unit":"cup","name":"quasar"}]),
                instructions: serde_json::json!([]),
                created_at: now,
                updated_at: now,
                tags: vec![anchor_tag.clone(), format!("rank-{suffix}")],
            },
        )
        .await;

        let search = |text: &str| RecipeSearchRequest {
            text: Some(text.to_string()),
            tags_all: vec![anchor_tag.clone()],
            page_size: 100,
            ..RecipeSearchRequest::default()
        };
        let cross_field = crate::db::search_recipes(&pool, owner_id, &search("aurora nebula"))
            .await
            .expect("cross-field all-term search failed");
        assert_eq!(cross_field.total_count, 1);
        assert_eq!(cross_field.results[0].recipe.id, cross_field_id);

        let literal = crate::db::search_recipes(&pool, owner_id, &search("public OR private"))
            .await
            .expect("literal parser-looking search failed");
        assert_eq!(literal.total_count, 1);
        assert_eq!(literal.results[0].recipe.id, cross_field_id);

        let ranking = crate::db::search_recipes(&pool, owner_id, &search("quasar"))
            .await
            .expect("ranked recipe search failed");
        assert_eq!(ranking.total_count, 2);
        assert_eq!(ranking.results[0].recipe.id, title_ranked_id);
        assert!(ranking.results[0].relevance > ranking.results[1].relevance);

        let advanced = AdvancedRecipeSearchCriteria {
            title: AdvancedTextField {
                include: Some(criterion("aurora soup", TextMatchMode::ExactPhrase)),
                ..AdvancedTextField::default()
            },
            description: AdvancedTextField {
                include: Some(criterion("nebula supper", TextMatchMode::AllTerms)),
                ..AdvancedTextField::default()
            },
            ingredients: AdvancedTextField {
                include: Some(criterion("cardamom", TextMatchMode::AnyTerm)),
                exclude: vec![criterion("cumin", TextMatchMode::AnyTerm)],
            },
            instructions: AdvancedTextField {
                include: Some(criterion("parmesan", TextMatchMode::AllTerms)),
                exclude: vec![criterion("secret photo metadata", TextMatchMode::AllTerms)],
            },
            tags_any: vec![format!("comfort-{suffix}")],
            tags_excluded: vec![format!("forbidden-{suffix}")],
            created_at: Some(TimestampRange {
                lower_inclusive: Some(now - Duration::days(3)),
                upper_exclusive: Some(now),
            }),
            updated_at: Some(TimestampRange {
                lower_inclusive: Some(now - Duration::days(2)),
                upper_exclusive: Some(now),
            }),
        };
        let advanced_response = crate::db::search_recipes(
            &pool,
            owner_id,
            &RecipeSearchRequest {
                advanced: Some(advanced.clone()),
                tags_all: vec![anchor_tag.clone()],
                page_size: 100,
                ..RecipeSearchRequest::default()
            },
        )
        .await
        .expect("advanced recipe search failed");
        assert_eq!(advanced_response.total_count, 1);
        assert_eq!(advanced_response.results[0].recipe.id, cross_field_id);

        let mut excluded = advanced;
        excluded.title.exclude = vec![criterion("aurora", TextMatchMode::AnyTerm)];
        let excluded_response = crate::db::search_recipes(
            &pool,
            owner_id,
            &RecipeSearchRequest {
                advanced: Some(excluded),
                tags_all: vec![anchor_tag.clone()],
                page_size: 100,
                ..RecipeSearchRequest::default()
            },
        )
        .await
        .expect("advanced exclusion search failed");
        assert_eq!(excluded_response.total_count, 0);

        let partial = crate::db::search_recipes(
            &pool,
            owner_id,
            &RecipeSearchRequest {
                advanced: Some(AdvancedRecipeSearchCriteria {
                    title: AdvancedTextField {
                        include: Some(criterion("uror", TextMatchMode::AnyTerm)),
                        ..AdvancedTextField::default()
                    },
                    ..AdvancedRecipeSearchCriteria::default()
                }),
                tags_all: vec![anchor_tag.clone()],
                page_size: 100,
                ..RecipeSearchRequest::default()
            },
        )
        .await
        .expect("partial advanced recipe search failed");
        assert_eq!(partial.total_count, 1);
        assert_eq!(partial.results[0].recipe.id, cross_field_id);

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(owner_id)
            .execute(&pool)
            .await
            .expect("failed to clean up text fixtures");
        pool.close().await;
    }
}
