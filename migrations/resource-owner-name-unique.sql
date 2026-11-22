-- Apply after all resource tables exist. Resolve scripts/resource-name-conflicts.sql
-- first; no names or contents are changed automatically.
BEGIN;
CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_owner_name_unique
ON collections (owner_id, (lower(btrim(regexp_replace(name, '[[:space:]]+', ' ', 'g')))));
CREATE UNIQUE INDEX IF NOT EXISTS idx_grocery_lists_owner_name_unique
ON grocery_lists (owner_id, (lower(btrim(regexp_replace(name, '[[:space:]]+', ' ', 'g')))));
CREATE UNIQUE INDEX IF NOT EXISTS idx_kitchen_profiles_owner_name_unique
ON kitchen_profiles (owner_id, (lower(btrim(regexp_replace(name, '[[:space:]]+', ' ', 'g')))));
COMMIT;
