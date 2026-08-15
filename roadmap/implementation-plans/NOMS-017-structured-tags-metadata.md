# NOMS-017: Structured Recipe Tags & Culinary Metadata — Implementation Plan

**Status:** Placeholder — discovery and implementation planning required
**Created:** 2026-08-15
**Depends on:** Current recipe schema, recipe editor, and search contracts
**Coordination:** NOMS-016 imports and exports this metadata; NOMS-018 consumes ingredient and dietary structure for planning; NOMS-019 may consume cooking-method and timing metadata.
**Approach:** To be finalized after the taxonomy, ownership model, normalization rules, migration path, and search behavior are tested against existing recipe data.

---

## Intended outcome

Recipes carry consistent, searchable culinary metadata without preventing authors from expressing uncommon cuisines, techniques, or dietary needs.

## Proposed planning scope

- Standardize dietary labels, allergens, cuisine, meal type, difficulty, cooking methods, and useful time categories.
- Distinguish controlled platform metadata from user-authored freeform tags.
- Define normalized ingredient identity only to the extent required by filtering and downstream planning.
- Add authoring, validation, display, search, and filtering behavior.
- Preserve existing recipes through an explicit migration and backfill strategy.
- Define accessibility, localization, moderation, and taxonomy-evolution rules.

## Questions to resolve

- Which metadata fields are controlled vocabularies, hierarchical values, or freeform tags?
- Who may create, rename, merge, or deprecate taxonomy values?
- How are dietary claims distinguished from allergen guarantees?
- How much ingredient normalization belongs here versus NOMS-018?
- How do imported values map to canonical values without silently changing meaning?

## Required implementation-plan deliverables

- Taxonomy and data-ownership contract.
- Current-state schema and search baseline.
- Migration, backfill, indexing, and compatibility plan.
- Test-first checkpoints for editor, display, import/export, and search integration.
- Accessibility, privacy, performance, and release verification matrix.

## Definition of ready

- [ ] The initial taxonomy and extension rules are approved.
- [ ] Dietary and allergen semantics are explicitly bounded.
- [ ] Migration and import-mapping behavior are documented.
- [ ] Checkpoints, acceptance evidence, and release gates are defined.

