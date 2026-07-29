-- NOMS Schema
-- Applied declaratively by pgschema using an extension-enabled plan database.
-- Additive-only: never DROP or ALTER existing columns.
-- All statements are idempotent (IF NOT EXISTS) for safe repeated application.
--
-- PREREQUISITES: Run migrations/extensions.sql first to install required extensions
-- (pgcrypto, pg_cron, pg_search).

-- Core user table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(30) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    avatar_url TEXT,
    avatar_original_url TEXT,
    avatar_edit JSONB,
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_search
ON users USING bm25 (
    id,
    (username::pdb.simple('alias=user_username_words')),
    (username::pdb.ngram(2, 20, 'alias=user_username_partial')),
    (display_name::pdb.simple('alias=user_display_name_words')),
    (display_name::pdb.ngram(2, 20, 'alias=user_display_name_partial')),
    (bio::pdb.simple('alias=user_bio_words'))
)
WITH (key_field = 'id');

-- OAuth provider accounts linked to users
CREATE TABLE IF NOT EXISTS oauth_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(20) NOT NULL CONSTRAINT valid_oauth_provider CHECK (provider IN ('google', 'apple', 'github')),
    provider_user_id VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    profile_data JSONB,
    refresh_token TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(provider, provider_user_id)
);

CREATE INDEX IF NOT EXISTS idx_oauth_accounts_email ON oauth_accounts(email);

-- Foreign key lookup + CASCADE delete performance
CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user_id ON oauth_accounts(user_id);

-- Short-lived auth state for OAuth CSRF protection (~10 min TTL)
-- Expiry is enforced application-side; pg_cron handles periodic cleanup.
CREATE TABLE IF NOT EXISTS auth_states (
    id VARCHAR(64) PRIMARY KEY,
    redirect_uri TEXT NOT NULL,
    provider TEXT NOT NULL,
    code_verifier TEXT,
    purpose TEXT CHECK (purpose IN ('login', 'link')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id UUID
);

-- Index for periodic cleanup of expired auth states (pg_cron DELETE WHERE created_at < ...)
CREATE INDEX IF NOT EXISTS idx_auth_states_created_at ON auth_states(created_at);

-- Server-side sessions: JWT token is a reference to this table row.
-- The JWT `sub` claim is the session `id`; `verify_session` looks up the row
-- to get the `user_id` and check `revoked` / `expires_at`.
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '15 minutes'),
    refreshed_at TIMESTAMPTZ,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);

-- Lookup by session id (from JWT `sub` claim)
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);

-- Cleanup of expired + revoked sessions (pg_cron)
-- Note: partial index with NOW() is not allowed (NOW() is STABLE, not IMMUTABLE).
-- The pg_cron cleanup query uses: WHERE (revoked = TRUE OR expires_at < NOW()) AND created_at < NOW() - INTERVAL '24 hours'
-- This index covers the revoked=true case efficiently.
CREATE INDEX IF NOT EXISTS idx_sessions_cleanup ON sessions(expires_at, revoked)
    WHERE revoked = TRUE;

-- ── Recipe management ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    commentary TEXT,
    prep_time_minutes INT,
    cook_time_minutes INT,
    servings INT,
    ingredients JSONB NOT NULL DEFAULT '[]'::jsonb,
    instructions JSONB NOT NULL DEFAULT '[]'::jsonb,
    equipment JSONB NOT NULL DEFAULT '[]'::jsonb,
    images JSONB NOT NULL DEFAULT '[]'::jsonb,
    visibility VARCHAR(20) NOT NULL DEFAULT 'private'
        CONSTRAINT valid_recipe_visibility CHECK (visibility IN ('private', 'unlisted', 'public')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipes_search
ON recipes USING bm25 (
    id,
    (title::pdb.simple('alias=recipe_title_words')),
    (title::pdb.ngram(2, 15, 'alias=recipe_title_partial')),
    (description::pdb.simple('alias=recipe_description_words')),
    (description::pdb.ngram(2, 15, 'alias=recipe_description_partial')),
    (jsonb_path_query_array(ingredients, 'strict $.** ? (@.type() == "string")')::pdb.simple('alias=recipe_ingredients_words')),
    (jsonb_path_query_array(instructions, 'strict $.**.text')::pdb.simple('alias=recipe_instructions_words'))
)
WITH (key_field = 'id');

CREATE TABLE IF NOT EXISTS recipe_tags (
    recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (recipe_id, tag)
);

-- Immutable full snapshots for every published recipe revision.
CREATE TABLE IF NOT EXISTS recipe_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL CHECK (version_number > 0),
    snapshot JSONB NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (recipe_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe
    ON recipe_versions(recipe_id, version_number DESC);

-- Mutable, owner-only work in progress. Existing recipes have one draft per
-- owner/recipe pair; users may keep multiple new-recipe drafts.
CREATE TABLE IF NOT EXISTS recipe_drafts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    base_version_number INTEGER CHECK (base_version_number IS NULL OR base_version_number > 0),
    snapshot JSONB NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    forked_from_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,
    forked_from_version_number INTEGER CHECK (
        forked_from_version_number IS NULL OR forked_from_version_number > 0
    ),
    forked_from_title VARCHAR(255),
    fork_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipe_drafts_user
    ON recipe_drafts(user_id, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_recipe_drafts_existing_recipe
    ON recipe_drafts(user_id, recipe_id)
    WHERE recipe_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,
    forked_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,
    forked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    parent_relationship_id UUID REFERENCES fork_relationships(id) ON DELETE SET NULL,
    original_recipe_key UUID,
    forked_recipe_key UUID,
    forked_version_number INTEGER CHECK (
        forked_version_number IS NULL OR forked_version_number > 0
    ),
    original_title VARCHAR(255) NOT NULL,
    forked_title VARCHAR(255),
    original_owner_id UUID,
    original_owner_username VARCHAR(30),
    original_visibility VARCHAR(20) CHECK (
        original_visibility IS NULL OR original_visibility IN ('private', 'unlisted', 'public')
    ),
    forked_owner_id UUID,
    forked_owner_username VARCHAR(30),
    forked_visibility VARCHAR(20) CHECK (
        forked_visibility IS NULL OR forked_visibility IN ('private', 'unlisted', 'public')
    ),
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id);
CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id);
CREATE INDEX IF NOT EXISTS idx_forks_original_key ON fork_relationships(original_recipe_key);
CREATE UNIQUE INDEX IF NOT EXISTS idx_forks_result_key
    ON fork_relationships(forked_recipe_key) WHERE forked_recipe_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_forks_parent ON fork_relationships(parent_relationship_id);

CREATE INDEX IF NOT EXISTS idx_recipe_tags_tag ON recipe_tags(tag);

CREATE INDEX IF NOT EXISTS idx_recipes_user_id ON recipes(user_id);
CREATE INDEX IF NOT EXISTS idx_recipes_created_at ON recipes(created_at DESC);

-- Partial index for public recipe discovery (explore page)
CREATE INDEX IF NOT EXISTS idx_recipes_visibility_created ON recipes(visibility, created_at DESC)
    WHERE visibility = 'public';

-- ── Recipe collections ─────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_collection_id UUID,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    image JSONB CHECK (image IS NULL OR jsonb_typeof(image) = 'object'),
    visibility VARCHAR(20) NOT NULL DEFAULT 'private'
        CHECK (visibility IN ('private', 'unlisted', 'public')),
    sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (id, owner_id),
    CHECK (parent_collection_id IS NULL OR parent_collection_id <> id),
    FOREIGN KEY (parent_collection_id, owner_id)
        REFERENCES collections(id, owner_id) ON DELETE CASCADE,
    CHECK (btrim(name) <> '')
);

CREATE INDEX IF NOT EXISTS idx_collections_owner_updated
    ON collections(owner_id, updated_at DESC);

-- Partial index for public collection discovery.
CREATE INDEX IF NOT EXISTS idx_collections_public_updated
    ON collections(updated_at DESC, id)
    WHERE visibility = 'public';

CREATE INDEX IF NOT EXISTS idx_collections_parent_order
    ON collections(owner_id, parent_collection_id, sort_order, lower(name));

CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_sibling_name
    ON collections(owner_id, parent_collection_id, lower(name))
    NULLS NOT DISTINCT;

CREATE INDEX IF NOT EXISTS idx_collections_search
ON collections USING bm25 (
    id,
    (name::pdb.simple('alias=collection_name_words')),
    (name::pdb.ngram(2, 15, 'alias=collection_name_partial')),
    (description::pdb.simple('alias=collection_description_words')),
    (description::pdb.ngram(2, 15, 'alias=collection_description_partial'))
)
WITH (key_field = 'id');

CREATE TABLE IF NOT EXISTS collection_recipes (
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collection_id, recipe_id)
);

CREATE INDEX IF NOT EXISTS idx_collection_recipes_order
    ON collection_recipes(collection_id, sort_order, created_at);

CREATE INDEX IF NOT EXISTS idx_collection_recipes_recipe_id
    ON collection_recipes(recipe_id);
