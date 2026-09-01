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
    browser_binding_digest BYTEA NOT NULL
        CHECK (octet_length(browser_binding_digest) = 32),
    purpose TEXT NOT NULL CHECK (purpose IN ('login', 'link')),
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

-- Paired object families uploaded by the generic media API. The browser keeps
-- the raw discard capability; the database stores only its digest.
CREATE TABLE IF NOT EXISTS media_upload_families (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('recipe', 'collection_cover')),
    full_url TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    full_key TEXT NOT NULL UNIQUE,
    thumbnail_key TEXT NOT NULL UNIQUE,
    capability_digest BYTEA NOT NULL UNIQUE CHECK (octet_length(capability_digest) = 32),
    lifecycle TEXT NOT NULL DEFAULT 'uncommitted'
        CHECK (lifecycle IN ('uncommitted', 'retained', 'discard_claimed', 'discarded')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retained_at TIMESTAMPTZ,
    discard_claimed_at TIMESTAMPTZ,
    discarded_at TIMESTAMPTZ,
    cleanup_checked_at TIMESTAMPTZ,
    delete_failure TEXT,
    CHECK (full_url <> thumbnail_url),
    CHECK (full_key <> thumbnail_key)
);

CREATE INDEX IF NOT EXISTS idx_media_upload_families_owner_lifecycle
    ON media_upload_families(owner_id, lifecycle);
CREATE INDEX IF NOT EXISTS idx_media_upload_families_expiry
    ON media_upload_families(expires_at) WHERE lifecycle = 'uncommitted';

-- Durable object-deletion authority created before an account row is removed.
-- `former_owner_id` intentionally has no foreign key: these rows must survive
-- the users/media_upload_families cascades and remain retryable afterwards.
CREATE TABLE IF NOT EXISTS account_media_cleanup_tombstones (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_family_id UUID UNIQUE,
    former_owner_id UUID NOT NULL,
    purpose TEXT NOT NULL CHECK (purpose IN ('recipe', 'collection_cover', 'avatar')),
    full_url TEXT,
    thumbnail_url TEXT,
    original_url TEXT,
    full_key TEXT,
    thumbnail_key TEXT,
    original_key TEXT,
    lifecycle TEXT NOT NULL DEFAULT 'pending'
        CHECK (lifecycle IN ('pending', 'claimed', 'retained', 'deleted')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claim_started_at TIMESTAMPTZ,
    cleanup_checked_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    delete_failure TEXT,
    CHECK (
        (purpose IN ('recipe', 'collection_cover')
            AND source_family_id IS NOT NULL
            AND full_url IS NOT NULL
            AND thumbnail_url IS NOT NULL
            AND full_key IS NOT NULL
            AND thumbnail_key IS NOT NULL)
        OR
        (purpose = 'avatar'
            AND source_family_id IS NULL
            AND (full_key IS NOT NULL OR thumbnail_key IS NOT NULL OR original_key IS NOT NULL))
    ),
    CHECK (full_key IS NULL OR thumbnail_key IS NULL OR full_key <> thumbnail_key)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_account_media_cleanup_full_key
    ON account_media_cleanup_tombstones(full_key) WHERE full_key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_account_media_cleanup_thumbnail_key
    ON account_media_cleanup_tombstones(thumbnail_key) WHERE thumbnail_key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_account_media_cleanup_original_key
    ON account_media_cleanup_tombstones(original_key) WHERE original_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_account_media_cleanup_candidates
    ON account_media_cleanup_tombstones(lifecycle, cleanup_checked_at, created_at)
    WHERE lifecycle <> 'deleted';
CREATE INDEX IF NOT EXISTS idx_account_media_cleanup_former_owner
    ON account_media_cleanup_tombstones(former_owner_id);

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
CREATE INDEX IF NOT EXISTS idx_forks_original_version_cursor
    ON fork_relationships(
        (COALESCE(original_recipe_key, original_recipe_id)),
        forked_version_number,
        id
    );
CREATE INDEX IF NOT EXISTS idx_forks_result_cursor
    ON fork_relationships(
        (COALESCE(forked_recipe_key, forked_recipe_id)),
        id
    );
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
