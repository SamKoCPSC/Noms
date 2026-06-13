-- NOMS Schema
-- Applied by pgschema: `just migrate` (local), entrypoint.sh (Docker/Railway).
-- Additive-only: never DROP or ALTER existing columns.
-- All statements are idempotent (IF NOT EXISTS) for safe repeated application.
--
-- PREREQUISITES: Run migrations/extensions.sql first to install required extensions
-- (pgcrypto, pg_cron). This file only manages schema objects that pgschema tracks.

-- Core user table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(30) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    avatar_url TEXT,
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

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

-- Recipe (single row, denormalized latest version for O(1) reads)
CREATE TABLE IF NOT EXISTS recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    is_draft BOOLEAN NOT NULL DEFAULT FALSE,
    prep_time_min INTEGER,
    cook_time_min INTEGER,
    total_time_min INTEGER,
    servings INTEGER,
    ingredients JSONB NOT NULL DEFAULT '[]'::jsonb,
    steps JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipes_owner_id ON recipes(owner_id);
CREATE INDEX IF NOT EXISTS idx_recipes_updated_at ON recipes(updated_at DESC);

-- Recipe tags (many-to-many, freeform text)
CREATE TABLE IF NOT EXISTS recipe_tags (
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (recipe_id, tag)
);

-- Recipe versions (reverse-diff chain, latest is anchor)
CREATE TABLE IF NOT EXISTS recipe_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,

    -- Full snapshot data (populated for latest version only)
    title VARCHAR(200),
    description TEXT,
    prep_time_min INTEGER,
    cook_time_min INTEGER,
    total_time_min INTEGER,
    servings INTEGER,
    ingredients JSONB,
    steps JSONB,

    -- Reverse diff: JSON Patch (RFC 6902) to apply to NEXT version to get THIS version
    -- NULL for latest version, populated for all historical versions
    reverse_diff JSONB,

    notes TEXT,
    is_latest BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(recipe_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe ON recipe_versions(recipe_id, version_number DESC);
CREATE INDEX IF NOT EXISTS idx_recipe_versions_latest ON recipe_versions(recipe_id, is_latest) WHERE is_latest = TRUE;

-- Fork relationships (DAG of recipe lineage)
CREATE TABLE IF NOT EXISTS fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_recipe_id UUID NOT NULL REFERENCES recipes(id),
    forked_recipe_id UUID NOT NULL REFERENCES recipes(id),
    forked_by UUID NOT NULL REFERENCES users(id),
    forked_version_number INTEGER,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id);
CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id);