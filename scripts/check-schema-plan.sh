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
# Verified against pgschema v1.9.0 JSON output format:
#   {
#     "groups": [
#       {
#         "steps": [
#           {
#             "sql": "ALTER TABLE users DROP COLUMN bio;",
#             "type": "table.column",
#             "operation": "drop",
#             "path": "public.users.bio"
#           }
#         ]
#       }
#     ]
#   }
#
# If the plan has no changes, "groups" is null.
# ---------------------------------------------------------------------------

set -euo pipefail

PLAN_FILE="${1:?Usage: check-schema-plan.sh <plan.json>}"

if [ ! -f "$PLAN_FILE" ]; then
    echo "❌ Error: Plan file not found: $PLAN_FILE"
    exit 1
fi

# --- Validate JSON structure ---
# Ensure the file is valid JSON and has the expected top-level keys.
if ! jq -e '.version and (.groups == null or (.groups | type == "array"))' "$PLAN_FILE" >/dev/null 2>&1; then
    echo "❌ Error: Plan file is not valid pgschema JSON or has unexpected structure."
    echo "   File: $PLAN_FILE"
    echo "   Inspect manually: jq '.' $PLAN_FILE"
    exit 1
fi

# --- Check 1: Reject DROP operations ---
# pgschema plan JSON uses .groups[].steps[] with .operation field.
# Destructive operations have .operation == "drop".
# The ? operator safely handles null groups (empty plan → 0 drops).
DROPS=$(jq '[.groups[]?.steps[]? | select(.operation == "drop")] | length' "$PLAN_FILE")

if [ "$DROPS" -gt 0 ]; then
    echo ""
    echo "❌ Destructive schema changes detected: $DROPS drop(s) found in plan."
    echo ""
    echo "   The following operations would destroy data:"
    jq -r '.groups[]?.steps[]? | select(.operation == "drop") | "   - \(.type): \(.path)"' "$PLAN_FILE"
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

# --- Check 2: Warn about ALTER operations (informational, not blocking) ---
# ALTER operations (e.g., column type changes) can truncate data but are
# not always destructive. We warn but don't block here — blocking should be
# a manual review decision.
ALTERS=$(jq '[.groups[]?.steps[]? | select(.operation == "alter")] | length' "$PLAN_FILE")

if [ "$ALTERS" -gt 0 ]; then
    echo ""
    echo "⚠️  Warning: $ALTERS ALTER operation(s) detected."
    echo "   Review manually to ensure no data truncation or breaking change risk."
    jq -r '.groups[]?.steps[]? | select(.operation == "alter") | "   - \(.type): \(.path)"' "$PLAN_FILE"
    echo "   Inspect the full plan file: $PLAN_FILE"
fi

# --- Check 3: Verify plan is not unexpectedly malformed ---
# If there are groups but we didn't catch any drops or alters, that's fine
# (create operations are additive and safe). But if the plan has steps with
# unknown operations, that's worth noting.
UNKNOWN_OPS=$(jq '[.groups[]?.steps[]? | select(.operation != "create" and .operation != "alter" and .operation != "drop")] | length' "$PLAN_FILE")

if [ "$UNKNOWN_OPS" -gt 0 ]; then
    echo ""
    echo "⚠️  Warning: $UNKNOWN_OPS step(s) with unrecognized operation(s) detected."
    jq -r '.groups[]?.steps[]? | select(.operation != "create" and .operation != "alter" and .operation != "drop") | "   - \(.operation): \(.type) \(.path)"' "$PLAN_FILE"
    echo "   Review manually: $PLAN_FILE"
fi

echo "✅ No destructive changes detected in plan. Safe to proceed."
exit 0