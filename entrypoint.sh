#!/usr/bin/env bash
set -euo pipefail

# Apply database schema if DATABASE_URL is configured.
# Skip silently for environments without a database.
if [ -n "${DATABASE_URL:-}" ]; then
    echo "Applying database schema..."
    pgmold apply --schema sql:/usr/local/app/migrations/schema.sql --database "$DATABASE_URL"
    echo "Database schema applied."
fi

# Launch the server
exec /usr/local/app/server
