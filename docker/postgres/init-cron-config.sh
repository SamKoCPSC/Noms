#!/bin/bash
# Configure pg_cron for the application database.
# Runs once during first database initialization (/docker-entrypoint-initdb.d/).
#
# pg_cron defaults to storing job metadata in the "postgres" database, but we
# need it in our application database so that CREATE EXTENSION pg_cron works.
# ALTER SYSTEM writes to postgresql.auto.conf which persists across restarts.
#
# Also sets shared_preload_libraries as a safety net for existing data volumes
# where the postgresql.conf.sample copy didn't include pg_cron. This requires
# a server restart, which the entrypoint handles automatically after init.
set -e

# Wait for the temporary server to be ready
until pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null 2>&1; do
    echo "Waiting for PostgreSQL to start..."
    sleep 1
done

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    ALTER SYSTEM SET cron.database_name = '${POSTGRES_DB}';
    ALTER SYSTEM SET shared_preload_libraries = 'pg_cron';
    SELECT pg_reload_conf();
EOSQL