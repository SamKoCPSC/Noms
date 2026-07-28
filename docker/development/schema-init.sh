#!/usr/bin/env bash
set -euo pipefail

: "${NOMS_INSTANCE_ID:?Missing isolated development environment}"
: "${PGHOST:?Missing isolated database host}"
: "${PGDATABASE:?Missing isolated database name}"

echo "Applying database extensions for ${NOMS_INSTANCE_ID}..."
PGPASSWORD="$PGPASSWORD" psql \
    --host "$PGHOST" \
    --port "$PGPORT" \
    --username "$PGUSER" \
    --dbname "$PGDATABASE" \
    --no-password \
    --set ON_ERROR_STOP=1 \
    --file /workspace/migrations/extensions.sql

echo "Applying database schema for ${NOMS_INSTANCE_ID}..."
pgschema apply \
    --host "$PGHOST" \
    --port "$PGPORT" \
    --db "$PGDATABASE" \
    --user "$PGUSER" \
    --password "$PGPASSWORD" \
    --file /workspace/migrations/schema.sql \
    --schema public \
    --auto-approve
