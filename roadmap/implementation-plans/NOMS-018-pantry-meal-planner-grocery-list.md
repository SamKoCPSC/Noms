# NOMS-018: Pantry, Meal Planner & Grocery Lists — Implementation Plan

**Status:** Placeholder — discovery and implementation planning required
**Created:** 2026-08-15
**Depends on:** Current recipe scaling and ingredient models; NOMS-017 metadata and normalization decisions
**Coordination:** NOMS-016 imported recipes must produce data that degrades safely when ingredients cannot be normalized.
**Approach:** To be finalized after ingredient identity, unit conversion, ownership, calendar, aggregation, and offline interaction contracts are defined.

---

## Intended outcome

Users can track what they have, plan scaled recipes for specific dates, and turn the plan into a practical grocery list with understandable quantities and provenance.

## Proposed planning scope

- Maintain a private pantry with quantities, units, optional expiration dates, and staple status.
- Schedule recipes and serving counts on a meal-planning calendar.
- Aggregate recipe ingredients into a grocery list while retaining source-recipe links.
- Merge only compatible quantities and clearly preserve items that cannot be normalized.
- Subtract pantry availability without silently hiding shortages or ambiguous ingredients.
- Support manual grocery items, check-off state, and deliberate pantry updates.
- Define responsive and offline-tolerant behavior for in-store use.

## Questions to resolve

- What ingredient and unit model is sufficiently accurate for an initial release?
- When should quantities merge, convert, remain separate, or require confirmation?
- Does checking off a grocery item update pantry inventory automatically?
- What calendar recurrence, timezone, household, and sharing behavior is in scope?
- Which planner and list states must remain available offline?

## Required implementation-plan deliverables

- Ingredient normalization and unit-conversion contract.
- Pantry, plan, grocery-list, and provenance data model.
- Authorization, privacy, migration, and offline behavior.
- Test-first checkpoints for each vertical slice and their integration.
- Automated and manual verification covering scaling, aggregation, ambiguity, and mobile use.

## Definition of ready

- [ ] Ingredient identity and quantity semantics are approved.
- [ ] Pantry subtraction and grocery aggregation rules are specified.
- [ ] Calendar, privacy, sharing, and offline scope are bounded.
- [ ] Checkpoints, acceptance evidence, and release gates are defined.

