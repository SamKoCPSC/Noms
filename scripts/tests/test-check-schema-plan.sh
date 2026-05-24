#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Test suite for check-schema-plan.sh
#
# Run with: bash scripts/tests/test-check-schema-plan.sh
# ---------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECK_SCRIPT="$(dirname "$SCRIPT_DIR")/check-schema-plan.sh"
TESTS_DIR="$SCRIPT_DIR"

PASS=0
FAIL=0

run_test() {
    local name="$1"
    local fixture="$2"
    local expected_exit="$3"
    local description="$4"

    echo "TEST: $name"
    echo "  Fixture: $fixture"
    echo "  Expected exit: $expected_exit"
    echo "  Description: $description"

    local actual_exit=0
    output=$("$CHECK_SCRIPT" "$fixture" 2>&1) || actual_exit=$?

    if [ "$actual_exit" -eq "$expected_exit" ]; then
        echo "  ✅ PASS (exit $actual_exit)"
        ((PASS++)) || true
    else
        echo "  ❌ FAIL (expected exit $expected_exit, got $actual_exit)"
        echo "  Output:"
        echo "$output" | sed 's/^/    /'
        ((FAIL++)) || true
    fi
    echo ""
}

echo "Running check-schema-plan.sh tests..."
echo "Script: $CHECK_SCRIPT"
echo ""

# Test 1: Empty plan should pass
run_test \
    "empty_plan_passes" \
    "$TESTS_DIR/fixture-empty-plan.json" \
    0 \
    "A plan with no changes (groups: null) should exit 0 — safe to proceed."

# Test 2: Safe plan (only creates) should pass
run_test \
    "safe_plan_passes" \
    "$TESTS_DIR/fixture-safe-plan.json" \
    0 \
    "A plan with only CREATE operations should exit 0 — additive changes are safe."

# Test 3: Destructive plan (drops) should fail
run_test \
    "destructive_plan_fails" \
    "$TESTS_DIR/fixture-destructive-plan.json" \
    1 \
    "A plan with DROP operations should exit 1 — destructive changes blocked."

# Test 4: Alter plan should pass with warning
run_test \
    "alter_plan_passes_with_warning" \
    "$TESTS_DIR/fixture-alter-plan.json" \
    0 \
    "A plan with ALTER operations should exit 0 but emit a warning."

# Test 5: Mixed plan (create + drop) should fail
run_test \
    "mixed_plan_fails" \
    "$TESTS_DIR/fixture-mixed-plan.json" \
    1 \
    "A plan with both CREATE and DROP should exit 1 — any drop is rejected."

# Test 6: Unknown operation plan should pass with warning
run_test \
    "unknown_op_plan_passes_with_warning" \
    "$TESTS_DIR/fixture-unknown-op-plan.json" \
    0 \
    "A plan with an unrecognized operation should exit 0 but emit a warning."

echo "========================================"
echo "Results: $PASS passed, $FAIL failed"
echo "========================================"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
exit 0