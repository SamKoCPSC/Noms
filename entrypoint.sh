#!/usr/bin/env bash
set -euo pipefail

# Railway can invoke preparation as a one-shot pre-deploy command. Keep the
# same preparation at startup for image-sourced services without that hook.
if [ "$#" -gt 1 ] || { [ "$#" -eq 1 ] && [ "$1" != '--prepare-database' ]; }; then
    echo 'Usage: entrypoint.sh [--prepare-database]' >&2
    exit 2
fi

# Guard: perl must be available for the URL parser
if ! command -v perl >/dev/null 2>&1; then
    echo "❌ Error: perl is required but not installed in this container."
    echo "   The DATABASE_URL parser depends on perl for correct handling of"
    echo "   URL-encoded special characters (@, #, :, =, etc.)."
    exit 1
fi

# Parse DATABASE_URL into PG* connection parameters for pgschema.
# Railway provides DATABASE_URL; pgschema needs individual flags.
#
# Uses a perl one-liner instead of bash string manipulation because:
#   1. Railway passwords may contain @, #, :, = and other URL-reserved chars
#   2. Railway URL-encodes these (%40, %23, %3A, %3D) in DATABASE_URL
#   3. Bash parameter expansion can't split on the LAST @ (it splits on the FIRST)
#   4. Bash has no built-in percent-decoding
#
# The perl parser handles all of this correctly: splits at the LAST @, splits
# host:port at the LAST :, splits user:password at the FIRST :, and decodes %XX
# sequences. It also handles postgresql:// and postgres:// schemes, missing ports,
# and query params.
#
# NOTE: Passwords with raw (unencoded) ? or # characters are invalid URLs because
# ? starts the query string and # starts the fragment. In practice, Railway always
# URL-encodes these. If you set DATABASE_URL manually, encode ? as %3F and # as %23.
parse_database_url() {
    local database_url="${1:?database URL is required}"

    NOMS_DATABASE_URL_TO_PARSE="$database_url" perl -e '
        use strict;
        use warnings;

        my $url = $ENV{"NOMS_DATABASE_URL_TO_PARSE"};

        # Remove scheme (postgres:// or postgresql://)
        $url =~ s|^postgres(?:ql)?://||;

        # Extract query string (after ?)
        my $query = "";
        $url =~ s/\?(.*)// and $query = $1;

        # Extract database name (after last /)
        my $dbname = "";
        $url =~ s/\/([^\/]*)$// and $dbname = $1;

        # Split credentials and host at the LAST @
        my ($creds, $hostport) = ("", $url);
        if ($url =~ /@/) {
            $url =~ /^(.*)@([^@]+)$/;
            $creds = $1;
            $hostport = $2;
        }

        # Split host and port at the LAST :
        my ($host, $port) = ($hostport, "5432");
        if ($hostport =~ /:([^:]+)$/) {
            $port = $1;
            $host = $hostport;
            $host =~ s/:\Q$port\E$//;
        }

        # Split user and password at the FIRST : in credentials
        my ($user, $pass) = ($creds, "");
        if ($creds =~ /:/) {
            $user = $creds;
            $user =~ s/:.*//;
            $pass = $creds;
            $pass =~ s/^[^:]*://;
        }

        # Percent-decode (%XX → character)
        sub decode {
            my $s = shift;
            $s =~ s/%([0-9A-Fa-f]{2})/chr(hex($1))/ge;
            return $s;
        }

        print decode($user) . "\n";
        print decode($pass) . "\n";
        print decode($host) . "\n";
        print decode($port) . "\n";
        print decode($dbname) . "\n";
    '
}

# Apply database extensions (pgcrypto, pg_cron) before schema.
# We use the parsed PG* variables instead of the raw DATABASE_URL because:
#   1. Railway passwords often contain special characters (@, #, :, =)
#   2. psql with a URL can fail to extract the password correctly, prompting
#      interactively and crashing the container
#   3. The PG* variables are already percent-decoded by parse_database_url
apply_extensions() {
    echo "Applying database extensions..."
    PGPASSWORD="$MIGRATION_PGPASSWORD" psql \
        -h "$MIGRATION_PGHOST" \
        -p "$MIGRATION_PGPORT" \
        -U "$MIGRATION_PGUSER" \
        -d "$MIGRATION_PGDATABASE" \
        -w \
        --set=ON_ERROR_STOP=1 \
        -f /usr/local/app/migrations/extensions.sql
    echo "Extensions applied."
}

# Apply the declarative schema using the extension-enabled target as pgschema's
# plan database so extension-provided types and index methods can be inspected.
apply_schema() {
    echo "Applying database schema..."
    PGPASSWORD="$MIGRATION_PGPASSWORD" \
    PGSCHEMA_PLAN_PASSWORD="$MIGRATION_PGPASSWORD" \
    pgschema apply \
        --host "$MIGRATION_PGHOST" \
        --port "$MIGRATION_PGPORT" \
        --db "$MIGRATION_PGDATABASE" \
        --user "$MIGRATION_PGUSER" \
        --plan-host "$MIGRATION_PGHOST" \
        --plan-port "$MIGRATION_PGPORT" \
        --plan-db "$MIGRATION_PGDATABASE" \
        --plan-user "$MIGRATION_PGUSER" \
        --file /usr/local/app/migrations/schema.sql \
        --schema public \
        --lock-timeout 30s \
        --auto-approve
    echo "Database schema applied."
}

grant_runtime_access() {
    echo "Applying least-privilege runtime database grants..."
    PGPASSWORD="$MIGRATION_PGPASSWORD" psql \
        -h "$MIGRATION_PGHOST" \
        -p "$MIGRATION_PGPORT" \
        -U "$MIGRATION_PGUSER" \
        -d "$MIGRATION_PGDATABASE" \
        -w \
        --set=ON_ERROR_STOP=1 \
        --set=runtime_role="$APP_PGUSER" \
        --set=runtime_database="$APP_PGDATABASE" \
        -f /usr/local/app/migrations/runtime-grants.sql
    echo "Runtime database grants applied."
}

# Main entrypoint
runtime_database_url="${DATABASE_URL:?DATABASE_URL environment variable is not set}"
migration_database_url="${MIGRATION_DATABASE_URL:?MIGRATION_DATABASE_URL environment variable is not set}"

{
    read -r APP_PGUSER
    read -r APP_PGPASSWORD
    read -r APP_PGHOST
    read -r APP_PGPORT
    read -r APP_PGDATABASE
} < <(parse_database_url "$runtime_database_url")
{
    read -r MIGRATION_PGUSER
    read -r MIGRATION_PGPASSWORD
    read -r MIGRATION_PGHOST
    read -r MIGRATION_PGPORT
    read -r MIGRATION_PGDATABASE
} < <(parse_database_url "$migration_database_url")

if [ "$APP_PGUSER" = "$MIGRATION_PGUSER" ]; then
    echo "DATABASE_URL and MIGRATION_DATABASE_URL must use separate roles." >&2
    exit 1
fi
if [ "$APP_PGHOST" != "$MIGRATION_PGHOST" ] \
    || [ "$APP_PGPORT" != "$MIGRATION_PGPORT" ] \
    || [ "$APP_PGDATABASE" != "$MIGRATION_PGDATABASE" ]; then
    echo "DATABASE_URL and MIGRATION_DATABASE_URL must target the same database server and database." >&2
    exit 1
fi

apply_extensions
apply_schema
grant_runtime_access

# The runtime role deliberately cannot write maintained vocabulary. Restrict the
# deployment URL to this child process, leaving DATABASE_URL unchanged for the app.
echo 'Checking maintained vocabulary...'
DATABASE_URL="$migration_database_url" /usr/local/app/seed-vocabulary \
    --if-changed \
    --file /usr/local/app/data/vocabulary/ingredients.json \
    --sources /usr/local/app/data/vocabulary/sources.json

# The application process receives only its runtime URL. Migration credentials
# are removed from the inherited environment before exec.
unset MIGRATION_DATABASE_URL migration_database_url MIGRATION_PGPASSWORD APP_PGPASSWORD
export NOMS_MIGRATIONS_APPLIED=1

if [ "${1:-}" = '--prepare-database' ]; then
    exit 0
fi

# Launch the server
exec /usr/local/app/server
