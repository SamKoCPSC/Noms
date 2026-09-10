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

# Project credentials such as OAuth client secrets and session signing keys do
# not have reliable global prefixes. Inspect assignment-shaped lines and allow
# only the exact development placeholders documented in .env.local.example or
# environment/reference substitutions. Report filenames, never values.
assignment_pattern='^[[:space:]]*(-[[:space:]]*)?(export[[:space:]]+)?"?(SESSION_SECRET|GOOGLE_CLIENT_SECRET|GITHUB_CLIENT_SECRET|R2_ACCESS_KEY_ID|R2_SECRET_ACCESS_KEY)"?[[:space:]]*[:=]'
mapfile -t assignment_files < <(
    git -C "$scan_root" grep -I -l -E "$assignment_pattern" -- . 2>/dev/null || true
)
for assignment_file in "${assignment_files[@]}"; do
    if perl -ne '
        if (/^\s*(?:-\s*)?(?:export\s+)?"?(SESSION_SECRET|GOOGLE_CLIENT_SECRET|GITHUB_CLIENT_SECRET|R2_ACCESS_KEY_ID|R2_SECRET_ACCESS_KEY)"?\s*[:=]\s*(.*)$/) {
            my ($name, $value) = ($1, $2);
            $value =~ s/\s+#.*$//;
            $value =~ s/^\s+|\s+$//g;
            $value =~ s/[,;]$//;
            $value =~ s/^["\x27]|["\x27]$//g;
            my $development =
                ($name eq "SESSION_SECRET" && $value eq "change-me-in-production-use-rust-random-uuid")
                || ($name eq "GOOGLE_CLIENT_SECRET" && $value eq "secret")
                || ($name eq "GITHUB_CLIENT_SECRET" && $value eq "secret")
                || ($name eq "R2_ACCESS_KEY_ID" && $value eq "noms-local-key")
                || ($name eq "R2_SECRET_ACCESS_KEY" && $value eq "noms-local-secret");
            next if $value eq "" || $value =~ /^\$/ || $development;
            $found = 1;
        }
        END { exit($found ? 0 : 1) }
    ' "$scan_root/$assignment_file"; then
        echo "potential project credential assignment in tracked file: $assignment_file" >&2
        failed=true
    fi
done

if $failed; then
    exit 1
fi

echo "tracked-secret scan passed"
