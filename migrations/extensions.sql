-- NOMS Extensions
-- Applied via psql BEFORE pgschema applies schema.sql.
-- Extensions must exist before tables reference them (e.g., gen_random_uuid uses pgcrypto).
-- Safe to re-run: all statements use IF NOT EXISTS guards.
--
-- NOTE: pg_cron requires shared_preload_libraries = 'pg_cron' in postgresql.conf.
-- This is baked into the custom Docker image (see docker/postgres/Dockerfile).
-- If the extension is missing from the image, this will fail — that's intentional.

-- Core UUID generation (used by all primary keys)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Periodic job scheduler (used for auth_states cleanup)
CREATE EXTENSION IF NOT EXISTS "pg_cron";

-- Schedule cleanup of expired auth states (every 6 hours)
-- Use cron.schedule() rather than INSERT INTO cron.job directly.
-- The function fills in nodeport/nodename defaults that have NOT NULL constraints,
-- and it handles upsert (same jobname for the same user replaces the existing job).
SELECT cron.schedule(
    'cleanup-auth-states',
    '0 */6 * * *',
    'DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL ''10 minutes'''
);