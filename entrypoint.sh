#!/usr/bin/env bash
set -euo pipefail

# Parse DATABASE_URL into PG* connection parameters for pgschema.
# Railway provides DATABASE_URL; pgschema needs individual flags.
# This script extracts host, port, database, user, and password from a
# standard postgres:// URL and exports them as PGHOST, PGPORT, PGDATABASE,
# PGUSER, PGPASSWORD so that pgschema can connect.

parse_database_url() {
    local url="${DATABASE_URL:?DATABASE_URL environment variable is not set}"
    # Strip the postgres:// prefix
    local rest="${url#postgres://}"
    # Extract user:password
    local userpass="${rest%%@*}"
    PGUSER="${userpass%%:*}"
    PGPASSWORD="${userpass#*:}"
    # Extract host:port/database
    local hostdb="${rest#*@}"
    # Extract host:port
    local hostport="${hostdb%%/*}"
    PGHOST="${hostport%%:*}"
    PGPORT="${hostport#*:}"
    # Port might be just the host if no colon, or include /dbname after
    PGPORT="${PGPORT%%/*}"
    if [ "$PGPORT" = "$PGHOST" ]; then
        PGPORT="5432"
    fi
    # Extract database name (strip query params if present)
    PGDATABASE="${hostdb#*/}"
    PGDATABASE="${PGDATABASE%%\?*}"
    export PGUSER PGPASSWORD PGHOST PGPORT PGDATABASE
}

# Apply database extensions (pgcrypto, pg_cron) before schema
apply_extensions() {
    echo "Applying database extensions..."
    PGPASSWORD="$PGPASSWORD" psql \
        -h "$PGHOST" -p "$PGPORT" -d "$PGDATABASE" -U "$PGUSER" \
        -f /usr/local/app/migrations/extensions.sql
    echo "Extensions applied."
}

# Apply database schema using pgschema
apply_schema() {
    echo "Applying database schema..."
    pgschema apply \
        --host "$PGHOST" \
        --port "$PGPORT" \
        --db "$PGDATABASE" \
        --user "$PGUSER" \
        --password "$PGPASSWORD" \
        --file /usr/local/app/migrations/schema.sql \
        --schema public \
        --auto-approve
    echo "Database schema applied."
}

# Main entrypoint
if [ -n "${DATABASE_URL:-}" ]; then
    parse_database_url
    apply_extensions
    apply_schema
else
    echo "DATABASE_URL not set — skipping database migration."
fi

# Launch the server
exec /usr/local/app/server