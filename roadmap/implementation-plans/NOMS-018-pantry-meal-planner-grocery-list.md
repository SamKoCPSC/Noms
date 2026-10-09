# NOMS-018: Kitchen Profiles, Recipe Feasibility & Shopping Checklists — Implementation Plan

**Status:** Discovery — product scope agreed; implementation contracts under refinement
**Created:** 2026-08-15
**Updated:** 2026-09-12
**Depends on:** Current recipe scaling, ingredient, and equipment models. Initial matching can proceed with exact names before NOMS-017 identity integration.
**Coordination:** NOMS-017 owns canonical ingredient/equipment identities and aliases for later integration; NOMS-018 initially treats distinct exact names as distinct identities and owns availability, gram quantities, feasibility, and shopping semantics. NOMS-016 imported recipes must preserve requirements whose quantities cannot be evaluated in grams.
**Approach:** Persist named kitchen profiles, match recipes against their ingredients and equipment, and combine selected recipes into understandable requirements and a mobile shopping checklist.

---

## Intended outcome

Users can answer two complementary questions:

1. Given the ingredients and equipment available in a selected kitchen, what recipes can I make, and what is missing for near matches?
2. Given a set of recipes and serving counts, what ingredients and equipment do I need, and what must I obtain?

Saved kitchen profiles make these workflows reusable. A shopping checklist carries the resulting requirements through an actual shopping trip.

## Agreed product scope

### Persistent kitchen profiles

- Support multiple private, named profiles, each containing its own ingredients and equipment, such as Home or Cabin.
- Allow creating, renaming, editing, duplicating, and deleting profiles, and quickly marking ingredients as out.
- Evaluate against one selected profile at a time and remember the last selected profile. Do not implicitly combine supplies across locations.
- Require explicit availability for every ingredient, including salt, oil, and water; there are no assumed staples.
- Allow ingredient presence without a quantity. Optional numeric quantities are in grams only and enable sufficiency checks; unknown quantities must remain distinguishable from sufficient quantities.
- Keep inventory changes deliberate. Selecting or making a recipe and checking off a shopping item do not automatically deduct or replenish inventory.

### Initial matching and quantity rules

- Identify ingredients by exact name equality. Distinct names are distinct ingredients; do not infer aliases, spelling/case equivalence, varieties, or substitutions. Canonical identity resolution belongs to later NOMS-017 integration.
- Identify equipment by exact name equality in the same way. Each distinct equipment name represents a unique type checked for presence; do not model physical instances, counts, size, capacity, or compatibility initially.
- Support grams only for numeric ingredient quantities, scaling, aggregation, and availability subtraction. Unit conversions and count-based calculations are deferred.
- Preserve existing non-gram or textual recipe quantities for display. They can participate in ingredient-presence matching but cannot establish quantity sufficiency or a numeric shortage; show quantity uncertainty instead of inventing gram values.

### Recipe feasibility

- Match both ingredient and equipment requirements against the selected profile.
- Explain results with missing ingredients, missing equipment, insufficient quantities, and unknown quantities. Having every ingredient is distinct from having enough to make the recipe.
- Use the categories Ready; Ready, quantity uncertain; Missing ingredients; Missing equipment; and Missing both.
- Known ingredient shortages count under Missing ingredients, including when the ingredient is present but its gram quantity is insufficient. Missing both means ingredient shortages and missing equipment coexist.
- Use Ready only when required ingredients have sufficient known quantities and required equipment is present. Use Ready, quantity uncertain when there are no known missing requirements but sufficiency cannot be established. Preserve uncertainty details alongside any missing-requirement category.
- Include near matches with explicit missing requirements. Feasibility ranking is deferred.
- Evaluate recipes individually while browsing and collectively when selected together: 200 g of rice may cover either of two recipes individually while the pair requires 400 g.
- Missing explicitly optional ingredients must not disqualify a recipe. Accept alternatives only where the recipe explicitly permits them.
- Explicitly declared alternatives use the same exact-name checks; no equipment compatibility inference is performed.

### Combined requirements

- Select multiple recipes and serving counts without requiring a calendar.
- Aggregate compatible ingredient quantities and subtract known availability once across the selection. Expose remaining shortages and unknown quantities.
- Deduplicate equipment by exact name across recipes. Counts, size/capacity compatibility, and scheduling simultaneous equipment use are outside the initial scope.
- Retain source-recipe links for every generated requirement and show which recipes contribute to it.
- Recalculate requirements when serving counts or selected recipes change.
- Merge gram quantities only for ingredients with the same exact name. Keep unsupported quantities visible and separate, including when other requirements for that ingredient have known gram quantities.

### Shopping checklist UI

- Turn combined requirements into a saved checklist associated with the selected kitchen profile, showing what is already available and what remains to obtain.
- Present ingredient shortages and missing equipment distinctly, with quantities, units, uncertainty, and source recipes available for inspection.
- Support checking and unchecking items with persisted state and a clear distinction between outstanding and completed items.
- Allow manual items unrelated to recipes, including non-food shopping needs such as dish soap.
- Provide a responsive phone layout with accessible, touch-friendly checkboxes and readable quantities.
- Keep a previously loaded checklist readable and checkable offline, persist local changes, and synchronize when connectivity returns. Show pending synchronization and failures without losing edits.
- Preserve manual items and reconcile completed state when recipe requirements change; the exact reconciliation rules remain to be specified.
- Checking an item off records shopping progress only. Pantry replenishment remains an explicit user action.

## Outside the initial scope

- Meal-planning calendars, recurrence, and simultaneous cooking schedules.
- Household sharing or combining multiple kitchen profiles for a match.
- Expiration tracking, automatic inventory consumption or replenishment, and automatic substitution suggestions.
- Full offline editing of kitchen profiles and recipe selections; the offline commitment here is the shopping checklist.
- Units other than grams, unit conversions, inferred ingredient/equipment equivalence, equipment counts and compatibility, and feasibility ranking. Canonical identity integration is a later step coordinated with NOMS-017.

## Questions to resolve

- What numeric precision and input validation should gram quantities use, and how are missing base serving counts handled?
- How will exact-name entries migrate to NOMS-017 identities when that integration is introduced?
- How are optional requirements and equipment alternatives represented in recipe data?
- When recipes, serving counts, or inventory change, how are existing checklist quantities and completed states reconciled? How are recipe versions retained or refreshed?
- How are offline edits synchronized across devices, conflicts resolved, and profile deletion handled for existing lists?
- What is the explicit workflow for adding purchased quantities to a kitchen profile?

## Required implementation-plan deliverables

- Exact-name ingredient/equipment matching and grams-only quantity contracts, with a later NOMS-017 identity integration path.
- Kitchen profile, recipe selection, combined requirements, checklist, and provenance data models.
- Authorization, privacy, migration, and offline behavior.
- Test-first checkpoints for each vertical slice and their integration.
- Automated and manual verification covering profile isolation, explicit staples, optional requirements, equipment matching, scaling, shared-stock aggregation, ambiguity, and mobile/offline checklist use.

## Acceptance scenarios to carry into implementation

- A user switches between Home and Cabin and sees results based only on the selected profile, including after a reload.
- A recipe requiring salt reports it missing when salt has not been explicitly included.
- A recipe with all ingredient names present but unknown amounts shows quantity uncertainty; missing required equipment remains visible.
- Two selected recipes requiring 200 g of rice each, against a profile containing 200 g of rice, produce a 200 g shortage.
- A profile listing `scallion` does not satisfy `green onion`; distinct equipment names likewise do not match automatically.
- A recipe requiring rice in cups with rice present in the profile reports quantity uncertainty and does not convert cups to grams.
- Feasibility categories distinguish known ingredient shortages, missing equipment, both, full coverage, and coverage with uncertain quantities; uncertainty remains visible even alongside a known shortage.
- Two recipes requiring the same reusable blender produce one equipment requirement with both source recipes retained.
- An absent optional garnish does not block a match; an explicitly permitted equipment alternative can satisfy its requirement.
- Changing serving counts updates requirements without losing manual shopping items or silently misrepresenting completed purchases.
- A user opens a saved checklist on a phone, goes offline, checks and unchecks items, reloads it, and reconnects without losing shopping progress.
- Shopping check-off leaves the kitchen inventory unchanged until the user explicitly updates it.

## Definition of ready

- [x] Product scope includes multiple persistent kitchen profiles, explicit ingredients and equipment, both matching directions, and a shopping checklist.
- [ ] Ingredient/equipment matching and quantity semantics are specified.
- [ ] Availability subtraction, aggregation, and checklist reconciliation rules are specified.
- [ ] Authorization, profile/list lifecycle, and offline synchronization contracts are specified.
- [ ] Checkpoints, acceptance evidence, and release gates are defined.
