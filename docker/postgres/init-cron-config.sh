#!/bin/bash
# Configure pg_cron database for the application database.
# Runs once during first database initialization (/docker-entrypoint-initdb.d/).
#
# pg_cron defaults to storing job metadata in the "postgres" database, but we
# prefer it in our application database so that jobs run there by default.
# ALTER SYSTEM writes to postgresql.auto.conf which persists across restarts.
#
# NOTE: shared_preload_libraries is set in the Dockerfile
# (postgresql.conf.sample) and must NOT be altered via ALTER SYSTEM here.
# PostgreSQL has a quirk where ALTER SYSTEM on array-type GUCs like
# shared_preload_libraries can wrap the value in extra double quotes
# (e.g. '"pg_cron,pg_search"'), which causes a fatal parse error on restart.
set -e

# Wait for the temporary server to be ready
until pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null 2>&1; do
    echo "Waiting for PostgreSQL to start..."
    sleep 1
done

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
    ALTER SYSTEM SET cron.database_name = '${POSTGRES_DB}';
    SELECT pg_reload_conf();
EOSQL
