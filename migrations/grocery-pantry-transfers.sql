-- Receipts recover transfers after a lost response, including removed source rows.
CREATE TABLE IF NOT EXISTS grocery_pantry_transfers (
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id UUID NOT NULL,
    list_id UUID NOT NULL REFERENCES grocery_lists(id) ON DELETE CASCADE,
    pantry_id UUID NOT NULL REFERENCES kitchen_profiles(id) ON DELETE CASCADE,
    request_hash VARCHAR(64) NOT NULL CHECK (length(request_hash) = 64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (owner_id, request_id)
);
CREATE INDEX IF NOT EXISTS idx_grocery_pantry_transfers_list ON grocery_pantry_transfers(list_id);
CREATE INDEX IF NOT EXISTS idx_grocery_pantry_transfers_pantry ON grocery_pantry_transfers(pantry_id);
