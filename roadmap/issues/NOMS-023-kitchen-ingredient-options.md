# NOMS-023: Optional and substitute ingredients in Kitchen checks

**Status:** 🟡 In Progress — optional-only slice authorized and implemented; substitution behavior remains backlog
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

## Authorized optional-only slice — 2026-11-04

The user requested nonblocking optional ingredients with a result distinct from
a full match. This slice applies the structured optional flag to both checks,
keeps required duplicates authoritative, labels optional shortages separately,
and leaves alternatives and grocery generation unchanged. Amounts/units stay
outside presence matching. Substitution questions above remain unapproved;
this partial delivery does not complete the broader proposed item.

## Validation of optional-only delivery

- Local server/WebAssembly development build passed; strict WebAssembly Clippy,
  formatting and whitespace checks passed.
- All 30 focused pantry/navigation/recipe-check tests passed, including library
  database tests with mixed optional/full matches across three cursor pages,
  ownership/revision/visibility rules, malformed flags, legacy data and required
  duplicates. Database tests used memory-backed temporary clusters because the
  workspace disk is full; initial disk-backed setup failures were environmental.
- The real-app browser suite passed nine scenario groups with no JavaScript
  errors: mobile/desktop and light/dark, pantry switching and stock changes,
  pagination/retry/access rules, optional-shortage badges, required exclusions,
  detail/list agreement and upgrade to a full match when stock becomes available.
- EXPLAIN (ANALYZE, BUFFERS) was captured for 326 candidates including 300
  unmatched recipes; execution was approximately 26 ms in the local fixture.
  This is supporting local evidence, not a production performance guarantee.
