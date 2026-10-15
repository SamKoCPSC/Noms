#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

valid_allowlist="$test_root/valid-allowlist.txt"
expired_allowlist="$test_root/expired-allowlist.txt"
printf '%s\n' 'RUSTSEC-2099-0001|2099-12-31|Test-only non-reachable advisory.' > "$valid_allowlist"
printf '%s\n' 'RUSTSEC-2000-0001|2000-01-01|Expired test exception.' > "$expired_allowlist"

NOMS_AUDIT_ALLOWLIST_FILE="$valid_allowlist" \
    "$repo_root/scripts/security-audit.sh" --validate-only --today 2026-08-20

if NOMS_AUDIT_ALLOWLIST_FILE="$expired_allowlist" \
    "$repo_root/scripts/security-audit.sh" --validate-only --today 2026-08-20 \
    >"$test_root/expired.out" 2>&1; then
    echo "expired RustSec exceptions must fail validation" >&2
    exit 1
fi
if grep -q 'Expired test exception' "$test_root/expired.out"; then
    echo "audit validation should identify advisory IDs without echoing rationale" >&2
    exit 1
fi

fixture_repo="$test_root/fixture-repo"
mkdir -p "$fixture_repo"
git -C "$fixture_repo" init --quiet
printf '%s\n' 'safe fixture' > "$fixture_repo/safe.txt"
git -C "$fixture_repo" add safe.txt
NOMS_SECRET_SCAN_ROOT="$fixture_repo" "$repo_root/scripts/check-tracked-secrets.sh"

fixture_secret='AKIAIOSFODNN7EX'"AMPLE"
printf '%s\n' "$fixture_secret" > "$fixture_repo/credential.txt"
git -C "$fixture_repo" add credential.txt
if NOMS_SECRET_SCAN_ROOT="$fixture_repo" "$repo_root/scripts/check-tracked-secrets.sh" \
    >"$test_root/secret.out" 2>&1; then
    echo "tracked credential patterns must fail validation" >&2
    exit 1
fi
grep -q 'credential.txt' "$test_root/secret.out"
if grep -Fq "$fixture_secret" "$test_root/secret.out"; then
    echo "secret scan output must not print discovered values" >&2
    exit 1
fi
git -C "$fixture_repo" rm --quiet --cached credential.txt

project_secret_fixture='cp6-production-credential-value-with-entropy'
{
    printf 'SESSION_SECRET=%s\n' "$project_secret_fixture"
    printf 'export GOOGLE_CLIENT_SECRET=%s\n' "$project_secret_fixture"
    printf '"GITHUB_CLIENT_SECRET": "%s"\n' "$project_secret_fixture"
    printf -- '- R2_ACCESS_KEY_ID=%s\n' "$project_secret_fixture"
    printf 'R2_SECRET_ACCESS_KEY=%s\n' "$project_secret_fixture"
} > "$fixture_repo/production.env"
git -C "$fixture_repo" add production.env
if NOMS_SECRET_SCAN_ROOT="$fixture_repo" "$repo_root/scripts/check-tracked-secrets.sh" \
    >"$test_root/project-secret.out" 2>&1; then
    echo "project credential assignments must fail validation" >&2
    exit 1
fi
grep -q 'production.env' "$test_root/project-secret.out"
if grep -Fq "$project_secret_fixture" "$test_root/project-secret.out"; then
    echo "project secret scan output must not print discovered values" >&2
    exit 1
fi

"$repo_root/scripts/check-deployment-artifacts.sh"
deployment_fixture="$test_root/deployment-fixture"
mkdir -p "$deployment_fixture"
printf '%s\n' 'FROM rust:latest' > "$deployment_fixture/Dockerfile"
if NOMS_DEPLOYMENT_ROOT="$deployment_fixture" \
    "$repo_root/scripts/check-deployment-artifacts.sh" \
    >"$test_root/deployment.out" 2>&1; then
    echo "moving container tags must fail deployment validation" >&2
    exit 1
fi
grep -q 'Dockerfile' "$test_root/deployment.out"

# Native builder selection must not hide a moving image tag from validation.
mkdir -p "$deployment_fixture/tools"
mv "$deployment_fixture/Dockerfile" "$deployment_fixture/tools/Dockerfile"
printf '%s\n' 'FROM --platform=$BUILDPLATFORM golang:latest AS builder' \
    > "$deployment_fixture/tools/Dockerfile"
if NOMS_DEPLOYMENT_ROOT="$deployment_fixture" \
    "$repo_root/scripts/check-deployment-artifacts.sh" \
    >"$test_root/platform-deployment.out" 2>&1; then
    echo "moving native builder tags must fail deployment validation" >&2
    exit 1
fi
grep -q 'unpinned container base' "$test_root/platform-deployment.out"
printf '%s\n' 'FROM --platform=$BUILDPLATFORM golang:1.25.3-bookworm@sha256:4f43b271f9673eb7bd0cb3a49cc17b08d8d6ee110277e26dbacc93c43a5a7793 AS builder' \
    > "$deployment_fixture/tools/Dockerfile"
NOMS_DEPLOYMENT_ROOT="$deployment_fixture" \
    "$repo_root/scripts/check-deployment-artifacts.sh"

echo "security policy script tests passed"
