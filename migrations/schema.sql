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

-- First-class managed media identity and object-deletion authority. Holder
-- references below, rather than uploader lifetime or document URL scans,
-- determine whether an asset is live.
CREATE TABLE IF NOT EXISTS media_assets (
    id UUID PRIMARY KEY,
    uploader_id UUID REFERENCES users(id) ON DELETE SET NULL,
    purpose TEXT NOT NULL CHECK (purpose IN ('recipe', 'collection_cover', 'avatar')),
    full_url TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    original_url TEXT,
    full_key TEXT NOT NULL UNIQUE,
    thumbnail_key TEXT NOT NULL UNIQUE,
    original_key TEXT UNIQUE,
    capability_digest BYTEA UNIQUE CHECK (
        capability_digest IS NULL OR octet_length(capability_digest) = 32
    ),
    lifecycle TEXT NOT NULL DEFAULT 'uncommitted'
        CHECK (lifecycle IN ('uncommitted', 'retained', 'delete_claimed', 'deleted')),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retained_at TIMESTAMPTZ,
    delete_claimed_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    cleanup_checked_at TIMESTAMPTZ,
    delete_failure TEXT,
    CHECK (full_url <> thumbnail_url),
    CHECK (full_key <> thumbnail_key),
    CHECK (original_url IS NULL OR original_url <> full_url),
    CHECK (original_url IS NULL OR original_url <> thumbnail_url),
    CHECK (original_key IS NULL OR original_key <> full_key),
    CHECK (original_key IS NULL OR original_key <> thumbnail_key),
    CHECK (
        (purpose IN ('recipe', 'collection_cover')
            AND original_url IS NULL
            AND original_key IS NULL
            AND capability_digest IS NOT NULL
            AND expires_at IS NOT NULL)
        OR
        (purpose = 'avatar'
            AND original_url IS NOT NULL
            AND original_key IS NOT NULL
            AND capability_digest IS NULL
            AND expires_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_media_assets_uploader_lifecycle
    ON media_assets(uploader_id, lifecycle);
CREATE INDEX IF NOT EXISTS idx_media_assets_expiry
    ON media_assets(expires_at, created_at, id) WHERE lifecycle = 'uncommitted';
CREATE INDEX IF NOT EXISTS idx_media_assets_cleanup_claimed
    ON media_assets((cleanup_checked_at IS NOT NULL), cleanup_checked_at, created_at, id)
    WHERE lifecycle = 'delete_claimed';
CREATE INDEX IF NOT EXISTS idx_media_assets_cleanup_retained
    ON media_assets((cleanup_checked_at IS NOT NULL), cleanup_checked_at, created_at, id)
    WHERE lifecycle = 'retained';

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

-- Holder-specific foreign keys make media liveness an indexed relational
-- invariant. Cascading holder deletion removes only that holder's grant.
CREATE TABLE IF NOT EXISTS recipe_media_refs (
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES media_assets(id) ON DELETE RESTRICT,
    PRIMARY KEY (recipe_id, asset_id)
);
CREATE INDEX IF NOT EXISTS idx_recipe_media_refs_asset
    ON recipe_media_refs(asset_id);

CREATE TABLE IF NOT EXISTS recipe_version_media_refs (
    recipe_version_id UUID NOT NULL REFERENCES recipe_versions(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES media_assets(id) ON DELETE RESTRICT,
    PRIMARY KEY (recipe_version_id, asset_id)
);
CREATE INDEX IF NOT EXISTS idx_recipe_version_media_refs_asset
    ON recipe_version_media_refs(asset_id);

CREATE TABLE IF NOT EXISTS recipe_draft_media_refs (
    recipe_draft_id UUID NOT NULL REFERENCES recipe_drafts(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES media_assets(id) ON DELETE RESTRICT,
    PRIMARY KEY (recipe_draft_id, asset_id)
);
CREATE INDEX IF NOT EXISTS idx_recipe_draft_media_refs_asset
    ON recipe_draft_media_refs(asset_id);

CREATE TABLE IF NOT EXISTS collection_media_refs (
    collection_id UUID PRIMARY KEY REFERENCES collections(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES media_assets(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_collection_media_refs_asset
    ON collection_media_refs(asset_id);

CREATE TABLE IF NOT EXISTS avatar_media_refs (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES media_assets(id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_avatar_media_refs_asset
    ON avatar_media_refs(asset_id);

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

-- NOMS-017: maintained ingredient suggestions. The seed CLI owns these rows;
-- authored recipe text and optional IDs remain in existing JSON snapshots.
CREATE TABLE IF NOT EXISTS vocabulary_entries (
    id UUID PRIMARY KEY,
    dimension TEXT NOT NULL CHECK (dimension IN ('ingredient')),
    locale TEXT NOT NULL CHECK (locale IN ('en')),
    label TEXT NOT NULL CHECK (length(btrim(label)) BETWEEN 1 AND 200),
    normalized_label TEXT NOT NULL CHECK (length(normalized_label) > 0),
    description TEXT NOT NULL CHECK (length(description) <= 2000),
    status TEXT NOT NULL CHECK (status IN ('active', 'retired')),
    definition JSONB NOT NULL CHECK (jsonb_typeof(definition) = 'object'),
    recipe_count BIGINT NOT NULL DEFAULT 0 CHECK (recipe_count >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_vocabulary_active_label
    ON vocabulary_entries (dimension, locale, normalized_label text_pattern_ops)
    WHERE status = 'active';

CREATE TABLE IF NOT EXISTS vocabulary_aliases (
    entry_id UUID NOT NULL REFERENCES vocabulary_entries(id) ON DELETE RESTRICT,
    alias TEXT NOT NULL CHECK (length(btrim(alias)) BETWEEN 1 AND 200),
    normalized_alias TEXT NOT NULL CHECK (length(normalized_alias) > 0),
    PRIMARY KEY (entry_id, normalized_alias)
);
-- Aliases may be ambiguous across entries; do not enforce global uniqueness.
CREATE INDEX IF NOT EXISTS idx_vocabulary_alias_prefix
    ON vocabulary_aliases (normalized_alias text_pattern_ops, entry_id);

CREATE TABLE IF NOT EXISTS vocabulary_sources (
    id TEXT PRIMARY KEY,
    definition JSONB NOT NULL CHECK (jsonb_typeof(definition) = 'object')
);
CREATE TABLE IF NOT EXISTS vocabulary_source_mappings (
    entry_id UUID NOT NULL REFERENCES vocabulary_entries(id) ON DELETE RESTRICT,
    source_id TEXT NOT NULL REFERENCES vocabulary_sources(id) ON DELETE RESTRICT,
    external_id TEXT NOT NULL,
    relationship TEXT NOT NULL CHECK (relationship IN ('exact', 'broader', 'narrower', 'related')),
    PRIMARY KEY (entry_id, source_id, external_id, relationship)
);
CREATE TABLE IF NOT EXISTS vocabulary_seed_runs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    checksum TEXT NOT NULL CHECK (length(checksum) = 64),
    entry_count INTEGER NOT NULL CHECK (entry_count > 0),
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Only current public published recipes contribute; private/unlisted recipes,
-- drafts and immutable versions never influence global popularity.
-- Unknown IDs remain valid recipe history and are ignored until seeded.
CREATE OR REPLACE FUNCTION ingredient_vocabulary_ids(ingredients JSONB)
RETURNS UUID[] LANGUAGE sql IMMUTABLE PARALLEL SAFE
SET search_path = pg_catalog
AS $$
    SELECT COALESCE(array_agg(DISTINCT CASE
        WHEN item->>'vocabulary_id' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        THEN (item->>'vocabulary_id')::uuid END), '{}'::uuid[])
    FROM jsonb_array_elements(CASE WHEN jsonb_typeof(ingredients) = 'array'
        THEN ingredients ELSE '[]'::jsonb END) AS item
$$;

CREATE OR REPLACE FUNCTION maintain_ingredient_recipe_counts()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    old_ids UUID[] := '{}';
    new_ids UUID[] := '{}';
    change RECORD;
BEGIN
    -- Shared by recipe writers, exclusive for reconciliation/seed loading.
    -- Take this before vocabulary row locks. Updates use a stable UUID order.
    PERFORM pg_advisory_xact_lock_shared(170018);
    IF TG_OP <> 'INSERT' THEN
        IF OLD.visibility = 'public' THEN
            old_ids := public.ingredient_vocabulary_ids(OLD.ingredients);
        END IF;
    END IF;
    IF TG_OP <> 'DELETE' THEN
        IF NEW.visibility = 'public' THEN
            new_ids := public.ingredient_vocabulary_ids(NEW.ingredients);
        END IF;
    END IF;
    FOR change IN
        SELECT id, sum(delta) AS delta FROM (
            SELECT unnest(new_ids) AS id, 1 AS delta
            UNION ALL
            SELECT unnest(old_ids) AS id, -1 AS delta
        ) AS changes
        WHERE id IS NOT NULL
        GROUP BY id HAVING sum(delta) <> 0 ORDER BY id
    LOOP
        UPDATE public.vocabulary_entries
        SET recipe_count = recipe_count + change.delta
        WHERE id = change.id AND dimension = 'ingredient';
    END LOOP;
    RETURN NULL;
END
$$;

CREATE TRIGGER ingredient_recipe_counts
AFTER INSERT OR UPDATE OF ingredients, visibility OR DELETE ON recipes
FOR EACH ROW EXECUTE FUNCTION maintain_ingredient_recipe_counts();
