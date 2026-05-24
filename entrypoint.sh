#!/usr/bin/env bash
set -euo pipefail

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
    local url="${DATABASE_URL:?DATABASE_URL environment variable is not set}"

    {
        read -r PGUSER
        read -r PGPASSWORD
        read -r PGHOST
        read -r PGPORT
        read -r PGDATABASE
    } < <(perl -e '
        use strict;
        use warnings;

        my $url = $ENV{"DATABASE_URL"};

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
    ')

    export PGUSER PGPASSWORD PGHOST PGPORT PGDATABASE
}

# Apply database extensions (pgcrypto, pg_cron) before schema.
# We use the raw DATABASE_URL here because psql uses libpq, which has a
# proper URL parser that handles special characters and percent-decoding
# natively. Only pgschema needs the individual PG* flags.
apply_extensions() {
    echo "Applying database extensions..."
    psql "$DATABASE_URL" \
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
