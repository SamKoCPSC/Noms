#!/usr/bin/env bash
set -euo pipefail

repo_root="${NOMS_CI_DEPLOYMENT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
workflow="$repo_root/.github/workflows/ci.yml"

fail() {
    echo "FAIL: $1" >&2
    exit 1
}

require() {
    grep -Fq -- "$1" "$workflow" || fail "$2"
}

job_steps() {
    awk -v job="$1" '
        $0 == "  " job ":" { active = 1; next }
        active && /^  [a-zA-Z][a-zA-Z-]*:/ { exit }
        active { print }
    ' "$workflow"
}

# Nested branches (including codex/*) must receive the same gates as main.
require 'branches: ["**"]' 'CI must run on pushes to nested branch names'
require 'cargo fmt --check' 'CI must preserve formatting validation'
require 'cargo clippy --locked --no-default-features --features server -- -D warnings' \
    'CI must lint the deployment server feature set'
require 'bash scripts/tests/test-ci-deployment-workflow.sh' \
    'CI must run its workflow regressions'

test_steps="$(job_steps test)"
for expected in 'apt-get install -y postgresql postgresql-client' \
    'pg_config --bindir' 'initdb --version' 'postgres --version' \
    'cargo test --locked --no-default-features --features server -- --test-threads=8'; do
    grep -Fq -- "$expected" <<< "$test_steps" \
        || fail "database test job is missing: $expected"
done

# Go installs tools outside PATH on hosted runners. Both schema consumers need
# the explicit path export, not just the search integration job.
for job in search-integration schema-check; do
    steps="$(job_steps "$job")"
    grep -Fq 'go install github.com/pgplex/pgschema@v1.12.1' <<< "$steps" \
        || fail "$job must preserve the pinned pgschema installation"
    grep -Fq 'echo "$(go env GOPATH)/bin" >> "$GITHUB_PATH"' <<< "$steps" \
        || fail "$job must expose pgschema on PATH"
    grep -Fq '"$(go env GOPATH)/bin/pgschema" --help' <<< "$steps" \
        || fail "$job must verify pgschema installation"
done
grep -Fq 'fetch-depth: 0' <<< "$(job_steps schema-check)" \
    || fail 'schema PR checks need the base commit for their git diff'

echo 'PASS: CI owns validation and exposes its database tools'
