#!/bin/bash
# Configure pg_cron for the application database.
# Runs once during first database initialization (/docker-entrypoint-initdb.d/).
#
# pg_cron defaults to storing job metadata in the "postgres" database, but we
# need it in our application database so that CREATE EXTENSION pg_cron works.
# ALTER SYSTEM writes to postgresql.auto.conf which persists across restarts.
#
# shared_preload_libraries is set via ALTER SYSTEM (overwrite, not append).
# This script is the sole authority on which libraries are loaded.
# If you need additional libraries, add them here explicitly.
# The Dockerfile also bakes this setting into postgresql.conf.sample, but the
# init script's ALTER SYSTEM takes precedence on first init.
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