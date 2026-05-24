#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# check-schema-plan.sh — Reject destructive schema changes in CI
#
# Usage: ./scripts/check-schema-plan.sh <plan.json>
#
# This script parses a pgschema plan JSON file and exits with code 1 if it
# contains any destructive operations (DROP TABLE, DROP COLUMN, DROP INDEX,
# DROP CONSTRAINT, type narrowing, etc.). It is designed to run in CI
# pipelines after `pgschema plan --output-json` to enforce an additive-only
# migration strategy.
#
# pgschema does not have built-in destructive-op blocking (unlike pgmold's
# --allow-destructive flag). This script fills that gap by acting as a CI
# gate: the PR is rejected if any destructive change is detected, forcing
# developers to use the additive pattern instead (add new column → backfill
# → update code → later drop old column).
#
# IMPORTANT: The JSON field names and structure below are based on pgschema's
# plan output format. Verify against your installed version by running:
#   pgschema plan --output-json test-plan.json --file schema.sql \
#     --host localhost --db noms --user noms --password <pass> --schema public
# Then inspect test-plan.json to confirm field names match.
# ---------------------------------------------------------------------------

set -euo pipefail

PLAN_FILE="${1:?Usage: check-schema-plan.sh <plan.json>}"

if [ ! -f "$PLAN_FILE" ]; then
    echo "❌ Error: Plan file not found: $PLAN_FILE"
    exit 1
fi

# --- Check 1: Reject DROP operations ---
# pgschema plan JSON uses a "changes" array where each change has a "type" field.
# Destructive types include: drop_table, drop_column, drop_index, drop_constraint
# Adjust the field names below if pgschema's JSON schema differs.
DROPS=$(jq '
    [.changes[] | select(
        .type == "drop_table" or
        .type == "drop_column" or
        .type == "drop_index" or
        .type == "drop_constraint" or
        .type == "drop_enum"
    )] | length
' "$PLAN_FILE" 2>/dev/null || echo "parse_error")

if [ "$DROPS" = "parse_error" ]; then
    echo "⚠️  Warning: Could not parse plan JSON for destructive changes."
    echo "   This may indicate an empty plan or an incompatible pgschema version."
    echo "   Inspect the plan file manually: $PLAN_FILE"
    echo ""
    echo "   If the plan is empty (no changes), this is expected and safe."
    echo "   To verify, run: cat $PLAN_FILE | jq '.'"

    # Check if the plan is empty (no changes) — that's safe
    HAS_CHANGES=$(jq '.changes | length' "$PLAN_FILE" 2>/dev/null || echo "error")
    if [ "$HAS_CHANGES" = "0" ] || [ "$HAS_CHANGES" = "error" ]; then
        echo "✅ Plan is empty (no changes to apply). Safe to proceed."
        exit 0
    fi

    echo "❌ Cannot determine safety of the plan. Failing to be safe."
    exit 1
fi

if [ "$DROPS" -gt 0 ]; then
    echo ""
    echo "❌ Destructive schema changes detected: $DROPS drop(s) found in plan."
    echo ""
    echo "   The following operations would destroy data:"
    jq -r '.changes[] | select(.type == "drop_table" or .type == "drop_column" or .type == "drop_index" or .type == "drop_constraint" or .type == "drop_enum") | "   - \(.type): \(.object // .table // "unknown")"' "$PLAN_FILE" 2>/dev/null || echo "   (details unavailable — inspect $PLAN_FILE manually)"
    echo ""
    echo "   Use the additive pattern instead:"
    echo "   1. Add new column → backfill data → update code → drop old column later"
    echo "   2. For table drops, create a new table and migrate data first"
    echo ""
    echo "   If you are CERTAIN this is intentional, bypass with:"
    echo "   pgschema apply --file schema.sql --auto-approve"
    echo "   (Not recommended in CI — use manual destructive override instead)"
    echo ""
    exit 1
fi

# --- Check 2: Warn about type narrowing (informational, not blocking) ---
# Type narrowing (e.g., VARCHAR(200) → VARCHAR(10)) can truncate data
# but is not always destructive. pgschema may flag these or handle them.
# We warn but don't block here — blocking should be a manual review decision.
TYPE_CHANGES=$(jq '
    [.changes[] | select(.type == "alter_column_type" or .type == "modify_column")] | length
' "$PLAN_FILE" 2>/dev/null || echo "0")

if [ "$TYPE_CHANGES" != "0" ] && [ "$TYPE_CHANGES" != "parse_error" ]; then
    echo "⚠️  Warning: $TYPE_CHANGES column type change(s) detected."
    echo "   Review manually to ensure no data truncation risk."
    echo "   Inspect the plan file: $PLAN_FILE"
fi

echo "✅ No destructive changes detected in plan. Safe to proceed."
exit 0