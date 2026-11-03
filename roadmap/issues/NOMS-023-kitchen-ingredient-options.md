# NOMS-023: Optional and substitute ingredients in Kitchen checks

**Status:** ⚪ Backlog — proposed; behavior/implementation plan not yet approved
**Recorded:** 2026-11-03 current-state review
**Depends on:** NOMS-017 ingredient options; NOMS-018 presence checks
**Review:** [Application current state](../reviews/2026-11-03-current-state.md)

## Problem

The editor stores optional ingredients and alternatives with independent names,
amounts, units and modifiers. Kitchen still requires every primary name and
ignores alternatives. Missing garnishes block cookability; available authored
alternatives cannot satisfy a missing primary.

## Proposed first slice

- Optional primaries do not block cookability.
- Available authored alternatives can satisfy required primaries; explain which
  alternative is used.
- Use identical rules for recipe details and owned/collected pantry discovery,
  preserving access, deduplication and pagination.
- Keep explicit availability and presence-only checks. Ignore amounts, units,
  servings and equipment; keep modifiers outside identity.
- Retain case-insensitive names unless a new identity policy is approved.
- Keep grocery generation unchanged until its separate behavior is agreed.

## Decisions before Ready

1. How a substitute's optional flag affects a required primary. An optional
   alternative must not erase a requirement without an available match.
2. Automatic acceptance versus user selection, and display of independent
   alternative quantities.
3. Repeated requirements, optional/required duplicates and multiple available
   alternatives across both check surfaces.
4. Labels for missing optional items and alternatives, retaining concise
   missing-only presentation.

## Proposed acceptance

Both surfaces agree for primary/alternative present, required missing and
optional-only recipes. Cover repeats, out-of-stock entries, legacy metadata,
case, empty recipes, access and pagination. Preserve authoring, exports and
saved groceries. Missing-only lists, near-cookable ranking, conversion and
automatic pantry amounts are later slices.
