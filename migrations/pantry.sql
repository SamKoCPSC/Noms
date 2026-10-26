-- NOMS-018 pantry inventory; shared with isolated PostgreSQL fixtures.
CREATE TABLE IF NOT EXISTS kitchen_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL CHECK (btrim(name) <> ''),
    note VARCHAR(500) NOT NULL DEFAULT '',
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    request_id UUID NOT NULL,
    request_hash VARCHAR(64) NOT NULL CHECK (length(request_hash) = 64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_kitchen_profiles_owner_order
ON kitchen_profiles (owner_id, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS pantry_ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id UUID NOT NULL REFERENCES kitchen_profiles(id) ON DELETE CASCADE,
    name VARCHAR(500) NOT NULL CHECK (btrim(name) <> ''),
    amount VARCHAR(64) NOT NULL DEFAULT '',
    unit VARCHAR(64) NOT NULL DEFAULT '',
    quantity_zero BOOLEAN NOT NULL DEFAULT FALSE,
    available BOOLEAN NOT NULL DEFAULT TRUE,
    position INTEGER NOT NULL CHECK (position >= 0),
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    add_key UUID,
    add_hash VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (profile_id, position),
    UNIQUE (profile_id, add_key),
    CHECK ((add_key IS NULL AND add_hash IS NULL) OR
           (add_key IS NOT NULL AND add_hash IS NOT NULL AND length(add_hash) = 64))
);

CREATE TABLE IF NOT EXISTS pantry_equipment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id UUID NOT NULL REFERENCES kitchen_profiles(id) ON DELETE CASCADE,
    name VARCHAR(500) NOT NULL CHECK (btrim(name) <> ''),
    available BOOLEAN NOT NULL DEFAULT TRUE,
    position INTEGER NOT NULL CHECK (position >= 0),
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    add_key UUID,
    add_hash VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (profile_id, position),
    UNIQUE (profile_id, add_key),
    CHECK ((add_key IS NULL AND add_hash IS NULL) OR
           (add_key IS NOT NULL AND add_hash IS NOT NULL AND length(add_hash) = 64))
);
