#!/usr/bin/env bash
set -euo pipefail

script_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scan_root="${NOMS_SECRET_SCAN_ROOT:-$script_root}"

if ! git -C "$scan_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "tracked-secret scan root is not a Git worktree" >&2
    exit 2
fi

failed=false
while IFS= read -r tracked_file; do
    case "$tracked_file" in
        .env.local.example)
            ;;
        .env|.env.*|*/.env|*/.env.*|*.pem|*.key|*.p12|*.pfx|id_rsa|id_ed25519|*/id_rsa|*/id_ed25519)
            echo "sensitive filename is tracked: $tracked_file" >&2
            failed=true
            ;;
    esac
done < <(git -C "$scan_root" ls-files)

secret_pattern='AKIA[0-9A-Z]{16}|-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----|ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9]{20,}'
mapfile -t matching_files < <(
    git -C "$scan_root" grep -I -l -E "$secret_pattern" -- . 2>/dev/null || true
)
for matching_file in "${matching_files[@]}"; do
    echo "potential credential pattern in tracked file: $matching_file" >&2
    failed=true
done

if $failed; then
    exit 1
fi

echo "tracked-secret scan passed"
