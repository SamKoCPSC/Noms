-- NOMS Initial Schema
-- Applied by `just migrate` (local) or pgmold (CI/prod).
-- Additive-only: never DROP or ALTER existing columns.

-- Enable UUID extension (PostgreSQL only)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Core user table
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(30) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    avatar_url TEXT,
    bio TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- OAuth provider accounts linked to users
CREATE TABLE oauth_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(20) NOT NULL CONSTRAINT valid_oauth_provider CHECK (provider IN ('google', 'apple', 'github')),
    provider_user_id VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    email_verified BOOLEAN DEFAULT FALSE,
    profile_data JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(provider, provider_user_id)
);

CREATE INDEX idx_oauth_accounts_email ON oauth_accounts(email);

-- Short-lived auth state for OAuth CSRF protection (~10 min TTL)
CREATE TABLE auth_states (
    id VARCHAR(64) PRIMARY KEY,
    redirect_uri TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT state_expiry CHECK (NOW() < created_at + INTERVAL '10 minutes')
);
