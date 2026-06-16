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
    visibility VARCHAR(20) NOT NULL DEFAULT 'private'
        CONSTRAINT valid_recipe_visibility CHECK (visibility IN ('private', 'unlisted', 'public')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS recipe_tags (
    recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (recipe_id, tag)
);

CREATE INDEX IF NOT EXISTS idx_recipes_user_id ON recipes(user_id);
CREATE INDEX IF NOT EXISTS idx_recipes_created_at ON recipes(created_at DESC);

-- Partial index for public recipe discovery (explore page)
CREATE INDEX IF NOT EXISTS idx_recipes_visibility_created ON recipes(visibility, created_at DESC)
    WHERE visibility = 'public';