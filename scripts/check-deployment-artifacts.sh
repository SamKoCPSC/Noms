#!/usr/bin/env bash
set -euo pipefail

script_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
deployment_root="${NOMS_DEPLOYMENT_ROOT:-$script_root}"
failed=false

while IFS= read -r dockerfile; do
    relative_path="${dockerfile#"$deployment_root"/}"
    while IFS= read -r from_line; do
        image="$(awk '{print $2}' <<< "$from_line")"
        case "$image" in
            scratch|builder|planner|build-check|pgschema-builder)
                ;;
            *@sha256:*)
                ;;
            *)
                echo "unpinned container base in $relative_path" >&2
                failed=true
                ;;
        esac
    done < <(grep -E '^FROM[[:space:]]+' "$dockerfile" || true)

    while IFS= read -r install_line; do
        if [[ "$install_line" != *"--locked"* ]] \
            || { [[ "$install_line" != *"--version"* ]] && [[ ! "$install_line" =~ cargo[[:space:]]+install[[:space:]]+[^[:space:]]+@[^[:space:]]+ ]]; }; then
            echo "unpinned cargo install in $relative_path" >&2
            failed=true
        fi
    done < <(grep -E 'cargo[[:space:]]+install' "$dockerfile" || true)
done < <(find "$deployment_root" -path '*/target' -prune -o -name Dockerfile -type f -print)

compose_file="$deployment_root/docker-compose.yml"
if [ -f "$compose_file" ]; then
    while IFS= read -r image_line; do
        image="$(sed -E 's/^[[:space:]]*image:[[:space:]]*//' <<< "$image_line" | tr -d '"')"
        case "$image" in
            noms-development:*)
                ;;
            *@sha256:*)
                ;;
            *)
                echo "unpinned Compose image in docker-compose.yml" >&2
                failed=true
                ;;
        esac
    done < <(grep -E '^[[:space:]]*image:' "$compose_file" || true)
fi

production_dockerfile="$deployment_root/Dockerfile"
if [ -f "$production_dockerfile" ]; then
    grep -qE '^USER[[:space:]]+noms$' "$production_dockerfile" || {
        echo "production Dockerfile must run as USER noms" >&2
        failed=true
    }
    if grep -qE 'chown.*noms.*/usr/local/app|COPY[[:space:]]+--chown=noms' \
        "$production_dockerfile"; then
        echo "production startup artifacts must not be writable by USER noms" >&2
        failed=true
    fi
    grep -qE 'chmod[[:space:]]+0555[[:space:]]+/usr/local/app' "$production_dockerfile" || {
        echo "production application directory must be read-only to USER noms" >&2
        failed=true
    }
    for ignored_path in '.git' '.env.local' 'target'; do
        grep -Fxq "$ignored_path" "$deployment_root/.dockerignore" 2>/dev/null || {
            echo ".dockerignore must exclude $ignored_path" >&2
            failed=true
        }
    done
    for deployment_file in entrypoint.sh migrations/runtime-grants.sql; do
        [ -f "$deployment_root/$deployment_file" ] || {
            echo "production database privilege file is missing: $deployment_file" >&2
            failed=true
        }
    done
    grep -q 'MIGRATION_DATABASE_URL' "$deployment_root/entrypoint.sh" 2>/dev/null || {
        echo "production entrypoint must separate migration credentials" >&2
        failed=true
    }
    for role_guard in rolsuper pg_auth_members datdba relowner; do
        grep -q "$role_guard" "$deployment_root/migrations/runtime-grants.sql" 2>/dev/null || {
            echo "runtime database role guard is missing: $role_guard" >&2
            failed=true
        }
    done
fi

postgres_dockerfile="$deployment_root/docker/postgres/Dockerfile"
if [ -f "$postgres_dockerfile" ]; then
    grep -q "95b4ef14a43c158e1b18cedc2c59024beeabcb8ab0520f55529cc1932781a495" \
        "$postgres_dockerfile" || {
        echo "pg_search package checksum is not pinned" >&2
        failed=true
    }
    grep -q 'sha256sum.*--check' "$postgres_dockerfile" || {
        echo "pg_search package checksum is not verified" >&2
        failed=true
    }
    for extension_package in postgresql-17-cron postgresql-17-pgvector \
        timescaledb-2-loader-postgresql-17 timescaledb-2-postgresql-17; do
        grep -qE "${extension_package}=[^[:space:]\\]+" "$postgres_dockerfile" || {
            echo "PostgreSQL extension package is not version-pinned: $extension_package" >&2
            failed=true
        }
    done
fi

railway_config="$deployment_root/railway.toml"
if [ -f "$railway_config" ]; then
    grep -q 'builder[[:space:]]*=[[:space:]]*"DOCKERFILE"' "$railway_config" || {
        echo "Railway must deploy the reviewed Dockerfile" >&2
        failed=true
    }
    grep -q 'healthcheckPath[[:space:]]*=[[:space:]]*"/"' "$railway_config" || {
        echo "Railway healthcheck path is missing" >&2
        failed=true
    }
    if grep -q 'buildCommand' "$railway_config"; then
        echo "Railway buildCommand bypasses the Dockerfile build" >&2
        failed=true
    fi
fi

workflow_directory="$deployment_root/.github/workflows"
if [ -d "$workflow_directory" ]; then
    if grep -R -n -E 'uses:[[:space:]]+[^[:space:]]+@(stable|main|master|v[0-9]+)([[:space:]]|$)' \
        "$workflow_directory" >/dev/null; then
        echo "GitHub Actions must be pinned to immutable revisions" >&2
        failed=true
    fi
    if ! grep -R -q 'cargo install cargo-audit --version 0.22.2 --locked' \
        "$workflow_directory"; then
        echo "CI must install the pinned cargo-audit release" >&2
        failed=true
    fi
    if grep -R -A3 'dtolnay/rust-toolchain@a5f673d0ba8626c3977bb416a1612774bc82181b' \
        "$workflow_directory" | grep -q 'toolchain:'; then
        echo "version-specific dtolnay action must not receive an unsupported toolchain input" >&2
        failed=true
    fi
fi

if $failed; then
    exit 1
fi

echo "deployment artifact checks passed"
