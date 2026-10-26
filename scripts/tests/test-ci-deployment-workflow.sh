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
# Match literal workflow shell expressions without evaluating them here.
# shellcheck disable=SC2016
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

publish_steps="$(job_steps publish-image)"
for expected in \
    'needs: [check, clippy, fmt, test, search-integration, security]' \
    "if: github.ref == 'refs/heads/main' && github.event_name != 'pull_request'" \
    'packages: write' 'platforms: linux/amd64' \
    'cache-to: type=gha,scope=noms-release,mode=max' \
    ':sha-${{ github.sha }}' ':main'; do
    grep -Fq -- "$expected" <<< "$publish_steps" \
        || fail "release image publishing is missing: $expected"
done

dockerfile="$repo_root/Dockerfile"
if grep -Eq '^RUN cargo (fmt|clippy|test)([[:space:]]|$)' "$dockerfile"; then
    fail 'deployment builds must not duplicate CI validation'
fi
# The runtime still needs postgresql-client for migrations; only inspect
# the builder stage, where server binaries existed solely for pgtemp.
builder_steps="$(awk '/ AS builder$/ { active = 1; next } active && /^FROM / { exit } active { print }' "$dockerfile")"
if grep -Eq 'apt-get install.*(postgresql|sudo)|rustup component add.*(rustfmt|clippy)' <<< "$builder_steps"; then
    fail 'deployment builders must not install validation-only tools'
fi
grep -Fq 'RUN dx bundle --platform web --release --force-sequential true' "$dockerfile" \
    || fail 'deployment builds must still produce the release web bundle'
grep -Fq 'RUN cargo chef cook --release --recipe-path recipe.json' "$dockerfile" \
    || fail 'deployment builds must preserve dependency caching'
for artifact in \
    'RUN cargo build --locked --release --no-default-features --features server --bin seed-vocabulary' \
    '/usr/src/app/target/release/seed-vocabulary /usr/local/app/seed-vocabulary' \
    '/usr/src/app/data/vocabulary/ingredients.json /usr/local/app/data/vocabulary/ingredients.json' \
    '/usr/src/app/data/vocabulary/sources.json /usr/local/app/data/vocabulary/sources.json'; do
    grep -Fq -- "$artifact" "$dockerfile" \
        || fail "release image is missing maintained vocabulary: $artifact"
done

echo 'PASS: CI owns validation and exposes its database tools'
