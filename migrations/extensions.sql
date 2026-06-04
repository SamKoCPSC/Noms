-- NOMS Extensions
-- Applied via psql BEFORE pgschema applies schema.sql.
-- Extensions must exist before tables reference them (e.g., gen_random_uuid uses pgcrypto).
-- Safe to re-run: all statements use IF NOT EXISTS guards.
--
-- NOTE: timescaledb, pg_cron, and pg_search require shared_preload_libraries
-- in postgresql.conf. This is baked into the custom Docker image
-- (see docker/postgres/Dockerfile). If an extension is missing from the image,
-- this will fail — that's intentional.

-- Core UUID generation (used by all primary keys)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Time-series database for analytics (recipe views, metrics, future-proofing)
-- Enables hypertables, continuous aggregates, and time_bucket().
-- Requires shared_preload_libraries = 'timescaledb' in postgresql.conf.
CREATE EXTENSION IF NOT EXISTS "timescaledb";

-- Periodic job scheduler (used for auth_states cleanup)
-- Requires shared_preload_libraries = 'pg_cron' in postgresql.conf.
CREATE EXTENSION IF NOT EXISTS "pg_cron";

-- Trigram matching for autocomplete and fuzzy search (Phase 2-3)
-- Enables GIN indexes: CREATE INDEX ... USING GIN (col gin_trgm_ops)
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Vector similarity search for semantic/recipe search (Phase 5+)
-- Enables vector type, HNSW/IVFFlat indexes, and nearest-neighbor operators.
CREATE EXTENSION IF NOT EXISTS "vector";

-- BM25 full-text search via ParadeDB (Phase 5)
-- Enables USING bm25 indexes and the @@@ search operator.
-- Requires shared_preload_libraries = 'pg_search' in postgresql.conf.
CREATE EXTENSION IF NOT EXISTS "pg_search";

-- Schedule cleanup of expired auth states (every 5 minutes).
-- Uses cron.schedule() with a named job — calling again with the same name
-- replaces the existing schedule (upsert behavior). This handles the migration
-- from the previous '0 */6 * * *' schedule transparently.
-- Deletes auth_states older than 15 minutes (5-minute grace after the 10-minute
-- application-side validation TTL in oauth.rs:CSRF_STATE_TTL_SECS).
SELECT cron.schedule(
    'cleanup-auth-states',
    '*/5 * * * *',
    'DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL ''15 minutes'''
);
