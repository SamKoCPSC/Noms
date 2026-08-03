#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$ROOT_DIR"

fail() {
    echo "FAIL: $1" >&2
    exit 1
}

assert_contains() {
    local description=$1
    local pattern=$2
    local file=$3

    rg --quiet --fixed-strings -- "$pattern" "$file" \
        || fail "$description ($file)"
}

assert_count() {
    local description=$1
    local pattern=$2
    local expected=$3
    local file=$4
    local actual

    actual=$(rg --count --fixed-strings -- "$pattern" "$file" || true)
    [[ "$actual" == "$expected" ]] \
        || fail "$description: expected $expected occurrence(s), found ${actual:-0} ($file)"
}

PGSCHEMA_MODULE="github.com/pgplex/pgschema@v1.12.1"

assert_contains "Production image pins the verified pgschema version" \
    "$PGSCHEMA_MODULE" Dockerfile
assert_contains "Development image pins the verified pgschema version" \
    "$PGSCHEMA_MODULE" docker/development/Dockerfile
assert_count "CI pins the verified pgschema version in both jobs" \
    "$PGSCHEMA_MODULE" 2 .github/workflows/ci.yml

if rg --quiet --fixed-strings 'github.com/pgplex/pgschema@latest' \
    Dockerfile docker/development/Dockerfile .github/workflows/ci.yml; then
    fail "pgschema installations must be reproducibly pinned"
fi

# Extension setup remains direct SQL, but declarative schema ownership belongs
# exclusively to pgschema. One reference per runtime script is the pgschema file.
assert_count "Production entrypoint applies schema.sql only through pgschema" \
    '/usr/local/app/migrations/schema.sql' 1 entrypoint.sh
assert_count "Development initializer applies schema.sql only through pgschema" \
    '/workspace/migrations/schema.sql' 1 docker/development/schema-init.sh
assert_count "Local commands use schema.sql only for pgschema apply and plan" \
    'migrations/schema.sql' 2 justfile

if rg --quiet --fixed-strings '< migrations/schema.sql' .github/workflows/ci.yml; then
    fail "CI must not bootstrap the declarative schema through psql"
fi

for workflow_file in entrypoint.sh docker/development/schema-init.sh justfile .github/workflows/ci.yml; do
    assert_contains "Extension-aware planning remains configured" \
        '--plan-host' "$workflow_file"
done

echo "PASS: pgschema workflow is pinned and declarative"
