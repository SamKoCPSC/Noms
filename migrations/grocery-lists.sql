-- Shared by the declarative schema and isolated database fixtures.
CREATE TABLE IF NOT EXISTS grocery_lists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL CHECK (btrim(name) <> ''),
    note VARCHAR(500) NOT NULL DEFAULT '',
    request_id UUID NOT NULL,
    request_hash VARCHAR(64) NOT NULL CHECK (length(request_hash) = 64),
    sources JSONB NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(sources) = 'array' AND jsonb_array_length(sources) <= 20
               AND octet_length(sources::text) <= 2097152),
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_grocery_lists_owner_order
    ON grocery_lists(owner_id, created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS grocery_list_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_id UUID NOT NULL REFERENCES grocery_lists(id) ON DELETE CASCADE,
    name VARCHAR(500) NOT NULL CHECK (btrim(name) <> ''),
    amount VARCHAR(64) NOT NULL DEFAULT '',
    unit VARCHAR(64) NOT NULL DEFAULT '',
    original JSONB CHECK (original IS NULL OR jsonb_typeof(original) = 'object'),
    contributions JSONB NOT NULL DEFAULT '[]'::jsonb
        CHECK (jsonb_typeof(contributions) = 'array' AND jsonb_array_length(contributions) <= 4000
               AND octet_length(contributions::text) <= 2097152),
    checked BOOLEAN NOT NULL DEFAULT FALSE,
    position INTEGER NOT NULL CHECK (position >= 0),
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    add_key UUID,
    add_hash VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (list_id, position),
    UNIQUE (list_id, add_key),
    CHECK ((add_key IS NULL AND add_hash IS NULL) OR
           (add_key IS NOT NULL AND add_hash IS NOT NULL AND length(add_hash) = 64))
);
