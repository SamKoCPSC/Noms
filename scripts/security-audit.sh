#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
allowlist_file="${NOMS_AUDIT_ALLOWLIST_FILE:-$repo_root/security/rustsec-allowlist.txt}"
expected_auditor_version="0.22.2"
validate_only=false
today="$(date -u +%F)"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --validate-only)
            validate_only=true
            shift
            ;;
        --today)
            [ "$#" -ge 2 ] || { echo "--today requires YYYY-MM-DD" >&2; exit 2; }
            today="$2"
            shift 2
            ;;
        *)
            echo "unknown security-audit option: $1" >&2
            exit 2
            ;;
    esac
done

if [ ! -f "$allowlist_file" ]; then
    echo "RustSec allowlist file is missing: $allowlist_file" >&2
    exit 1
fi
if ! date -u -d "$today" +%F >/dev/null 2>&1; then
    echo "audit validation date must use YYYY-MM-DD" >&2
    exit 2
fi

audit_arguments=()
declare -A seen_advisories=()
while IFS='|' read -r advisory_id expires_on rationale extra; do
    advisory_id="${advisory_id//[[:space:]]/}"
    expires_on="${expires_on//[[:space:]]/}"
    if [ -z "$advisory_id" ] || [[ "$advisory_id" == \#* ]]; then
        continue
    fi
    if [ -n "${extra:-}" ] || [[ ! "$advisory_id" =~ ^RUSTSEC-[0-9]{4}-[0-9]{4}$ ]]; then
        echo "invalid RustSec allowlist entry: $advisory_id" >&2
        exit 1
    fi
    if ! date -u -d "$expires_on" +%F >/dev/null 2>&1; then
        echo "invalid expiry for RustSec advisory $advisory_id" >&2
        exit 1
    fi
    if [ "$expires_on" \< "$today" ]; then
        echo "RustSec advisory exception expired: $advisory_id" >&2
        exit 1
    fi
    if [ -z "${rationale//[[:space:]]/}" ]; then
        echo "RustSec advisory exception lacks rationale: $advisory_id" >&2
        exit 1
    fi
    if [ -n "${seen_advisories[$advisory_id]:-}" ]; then
        echo "duplicate RustSec advisory exception: $advisory_id" >&2
        exit 1
    fi
    seen_advisories[$advisory_id]=1
    audit_arguments+=(--ignore "$advisory_id")
done < "$allowlist_file"

if $validate_only; then
    exit 0
fi

if ! command -v cargo-audit >/dev/null 2>&1; then
    echo "cargo-audit $expected_auditor_version is required" >&2
    exit 1
fi
installed_version="$(cargo audit --version)"
if [[ ! "$installed_version" =~ (^|[[:space:]])$expected_auditor_version$ ]]; then
    echo "cargo-audit $expected_auditor_version is required; found an incompatible version" >&2
    exit 1
fi

cd "$repo_root"
exec cargo audit "${audit_arguments[@]}"
