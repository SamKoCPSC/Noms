-- Additive migration for existing databases. Resolve reported conflicts first:
-- psql "$MIGRATION_DATABASE_URL" -f scripts/recipe-name-conflicts.sql
-- Existing recipe titles and immutable history are never renamed automatically.
-- All saved recipes reserve a name, regardless of visibility. Drafts live separately.
CREATE UNIQUE INDEX IF NOT EXISTS idx_recipes_owner_title_unique
ON recipes (user_id, (lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g')))));
