# NOMS-017: Recipe Taxonomy System — Implementation Plan

**Status:** Discovery — taxonomy system scope and contracts under refinement
**Created:** 2026-08-15
**Updated:** 2026-08-20
**Depends on:** Current recipe schema, recipe editor, versioning, and search contracts
**Coordination:** NOMS-016 imports and exports taxonomy assignments and source values; NOMS-018 consumes canonical ingredient identity while owning quantities, units, pantry, and grocery semantics; NOMS-019 consumes cooking-method, equipment, and timing metadata.
**Approach:** Construct the complete recipe-taxonomy foundation: canonical concepts, stable identities, aliases, governance, lifecycle, recipe assignments, import mappings, authoring, and discovery. Preserve expressive user tags alongside the controlled system.

---

## Intended outcome

Noms has a durable taxonomy system that can define, relate, evolve, localize, assign, import, export, search, and filter culinary concepts without collapsing uncommon culinary identities into a rigid or English-only classification scheme.

Recipes use stable canonical concepts where reliable structure is valuable and retain user-authored freeform tags where personal or uncommon expression is more important than global consistency.

## System scope

### Taxonomy platform

- Define stable, opaque identifiers for canonical concepts; labels and slugs are editable presentation data and never serve as identity.
- Model taxonomy dimensions, concepts, localized labels, aliases, descriptions, and mapping provenance.
- Support concept lifecycle states including proposed, active, deprecated, merged, and retired, with redirect/history behavior that does not break existing recipe assignments.
- Support both platform-curated canonical values and, where permitted by the dimension, user-defined miscellaneous values. User-defined values are immediately usable by their creator without becoming global canonical values.
- Define ownership, visibility, curation permissions, proposal and review workflows, duplicate detection, promotion and mapping rules, merge rules, audit history, and rollback behavior.
- Keep controlled taxonomy assignments separate from user-owned freeform tags.
- Define validation rules for allowed dimensions, cardinality, conflicting assignments, mappings, and deprecated concepts.

### Taxonomy dimensions

- Ingredients and ingredient aliases: canonical identity, common synonyms, and culturally or regionally specific names.
- Cuisines and culinary traditions: independent multi-valued concepts without forcing cultural identities into a geographic or culinary hierarchy.
- Difficulty: a small, understandable controlled scale with room for future refinement.
- Time classifications derived from canonical numeric recipe times rather than independently authored labels.
- Dish types: forms such as soup, stew, salad, bread, cake, or sandwich, independent of cuisine or occasion.
- Equipment: canonical equipment identities and aliases for the existing structured recipe-equipment entries.
- Cooking methods and techniques: multi-valued concepts such as baking, braising, grilling, steaming, fermenting, or sous vide, independent of dish type and equipment.

These are the only taxonomy dimensions in scope for NOMS-017. The underlying system must nevertheless allow values within them to grow without schema changes or application releases. NOMS maintains a large extensible canonical dataset while users can define miscellaneous values wherever the dimension permits them and the canonical dataset is insufficient.

Dietary suitability, allergens, meal types, occasions, courses, seasonality, and cooking context are not taxonomy dimensions in this ticket. Authors may still express these ideas through freeform tags where appropriate.

### Dimension cardinality

Cardinality is defined per dimension according to whether a recipe can logically belong to more than one value. Every dimension remains optional so existing, incomplete, and imported recipes remain valid.

| Dimension | Recipe cardinality | Assignment rule |
| --- | --- | --- |
| Ingredients | Many | A recipe contains many ingredient entries; each entry maps to at most one canonical or miscellaneous ingredient identity. |
| Cuisines and culinary traditions | Many | A recipe may belong to multiple overlapping traditions; no primary cuisine is required. |
| Difficulty | Zero or one | A recipe has at most one difficulty classification. |
| Time | Zero or one | A recipe has at most one taxonomy time classification, derived from its numeric time data rather than selected independently. |
| Dish types | Many | A recipe may legitimately have multiple forms or dish types. |
| Equipment | Many | A recipe contains many equipment entries; each entry maps to at most one canonical or miscellaneous equipment identity. |
| Cooking methods and techniques | Many | A recipe may use multiple methods or techniques. |

Adding a new value to a repeatable dimension does not replace existing assignments. Assigning a value to a singular dimension replaces its previous assignment through normal draft and version history.

### Flat taxonomy model

- Values within a dimension are independent; NOMS-017 does not model parent/child trees or general related-value graphs.
- Broad and specific concepts may both exist as first-class values. A recipe can carry each applicable value in a repeatable dimension, such as both `Chinese` and `Sichuan` or both `soup` and `noodle dish`.
- Aliases, spelling variants, transliterations, and localized labels resolve to one concept and are not separate taxonomy values.
- Lifecycle redirects such as `replaced_by` and explicit miscellaneous-to-canonical mappings remain supported because they preserve compatibility rather than define a browsing hierarchy.
- Search and facets match explicit assignments and aliases; they do not perform implicit ancestor or descendant expansion.
- A future ticket may add richer relationships if demonstrated search or discovery needs justify the additional model and curation cost.

### Value ownership and extensibility

- Platform values are globally discoverable, curated, localizable, and stable across recipes and users.
- Each dimension defines whether it permits user-defined values, along with their validation, cardinality, and assignment rules. Ingredients, cuisines and traditions, dish types, equipment, and cooking methods are expected to permit them; time remains derived, and the difficulty policy remains to be finalized.
- User-defined values belong to a specific taxonomy dimension and retain their authored label, owner, and provenance.
- A user-defined value can be assigned immediately; creating it does not require moderation or mutate the global taxonomy.
- The editor should suggest canonical values first, then offer an explicit miscellaneous-value path when no suitable match exists.
- A miscellaneous value is reusable by its creator across their own recipes and remains owner-scoped until promoted.
- A miscellaneous value assigned to a public recipe may be displayed with that recipe, but it is not suggested to other authors or exposed as a global facet before promotion.
- Exact-text search may find public recipes through their miscellaneous values without treating those values as canonical concepts.
- Frequently used or proposed miscellaneous values may be reviewed and promoted into the canonical dataset.
- Promotion creates or selects a canonical concept and records an explicit mapping. Existing recipes participate in canonical search and filtering through the mapping, but promotion must not silently rewrite the original value or erase its provenance.
- Future edits suggest the promoted canonical value while historical snapshots retain the original assignment and label.
- Owners can rename their miscellaneous values subject to recipe-history rules. Other users cannot edit or implicitly adopt another user's private value as canonical.
- Search and filtering must define how canonical values, aliases, mapped miscellaneous values, and unmapped miscellaneous values participate in results and facet counts.

### Canonical seed dataset

- Canonical seed values are maintained as declarative, version-controlled data files in the repository rather than hardcoded application values or ad hoc production database inserts.
- Each taxonomy dimension has its own seed file so changes, validation, attribution, and large-dataset updates remain independently reviewable.
- Each entry declares its stable identifier, dimension, canonical label, aliases, locale data, lifecycle state, source, and any required attribution.
- Stable identifiers are committed with the data and are never regenerated from mutable labels or file order.
- Seed changes use the normal review history and must pass automated schema, uniqueness, alias-collision, dimension-policy, and lifecycle validation.
- An idempotent loader applies additions and lifecycle changes consistently in new and existing environments without overwriting user-defined values.
- Removing a committed entry requires an explicit deprecation, replacement, merge, or retirement operation; deleting a line must not silently orphan recipe assignments.
- The exact serialization format and repository directory remain to be selected based on reviewability, localization support, dataset size, and streaming-import needs.

### Recipe integration

- Add controlled taxonomy assignments to canonical recipe snapshots so drafts, versions, forks, comparisons, and restores retain taxonomy history.
- Add authoring controls that distinguish canonical selections, user-defined miscellaneous values, and freeform tags.
- Add validation, display, accessible labels, search, faceted filtering, and empty/unknown states.
- Preserve existing `recipe_tags` behavior as freeform author expression and provide an explicit migration/backfill path rather than silently reclassifying tags.
- Define whether assignments are author-created, imported, curator-reviewed, mapped, or system-derived and retain that provenance for later review.

### Import, export, and interoperability

- Define lossless serialization for taxonomy identifiers, source labels, aliases, dimension, lifecycle state, and mapping provenance.
- Map imported values to canonical concepts only when the match is unambiguous.
- Preserve unmatched or ambiguous source text for author review; never silently change ingredient, culinary-tradition, dish-type, equipment, difficulty, or time meaning.
- Define compatibility behavior when an exported concept is unknown, deprecated, merged, or unavailable in another Noms deployment.

### Ingredient boundary with NOMS-018

NOMS-017 owns ingredient identity as taxonomy: canonical ingredient concepts, aliases, distinctions, and source mappings.

NOMS-018 owns quantitative food semantics: parsing amounts, unit identity and conversion, density or substitution rules, pantry inventory, compatible aggregation, and grocery-list arithmetic. An ingredient that cannot be canonically mapped must remain usable as authored text throughout both systems.

## Working decisions

- Controlled concepts use stable, platform-owned identifiers with editable and localizable labels.
- Freeform recipe tags remain separate and user-owned.
- Recipes may select multiple independent cuisine and culinary-tradition concepts; the taxonomy does not impose parent/child relationships between them.
- Authors may define a miscellaneous value in any dimension whose policy permits it when the canonical taxonomy is insufficient.
- User-defined values are usable immediately but do not become global concepts implicitly.
- Miscellaneous values remain owner-scoped, are reusable by their creator, and do not become global suggestions or facets before promotion.
- New global concepts enter through curated additions or explicit promotion of commonly used miscellaneous values.
- Promotion and mapping preserve the original value and provenance and never silently rewrite recipe meaning.
- Each dimension independently defines optional-singular or repeatable cardinality according to whether multiple simultaneous values are logically valid.
- Imported values retain their source and require an unambiguous mapping or explicit review before becoming canonical assignments.
- Time categories are derived from numeric recipe data so labels cannot drift out of sync.
- Canonical seed data is maintained in version-controlled repository files and applied through a validated, idempotent loader.
- Canonical seed data is split into one independently validated file per taxonomy dimension.

## Questions to resolve

- Does the first release need a curator and promotion interface, or can seed data and audited administrative operations establish the initial governance path?
- Which initial concept sets and authoritative sources are suitable for seeding each dimension, and what licensing or attribution obligations apply?
- Which serialization format best balances human review, localization, schema validation, and a potentially large ingredient dataset?
- Which dimensions allow multiple values, and which assignments require confidence, provenance, mapping state, or an explicit unknown value?
- Which ingredient distinctions require separate canonical identities, and which are aliases, without crossing into NOMS-018 substitution or quantity semantics?
- How are user-defined equipment values reconciled with the existing free-text `RecipeEquipment` model?
- What search ranking and facet-count behavior applies to aliases, deprecated concepts, mapped and unmapped miscellaneous values, and freeform tags?
- How are taxonomy changes represented in recipe history: by stable reference, captured display snapshot, or both?
- What visibility, moderation, abuse-prevention, and privacy rules apply to user-defined values and promotion candidates?
- Which locale owns fallback labels, collation, synonyms, transliteration, and search normalization?

## Required implementation-plan deliverables

- Taxonomy domain model and invariants, including identifiers, dimensions, flat-value semantics, lifecycle, provenance, localization, and audit history.
- Initial seed-taxonomy specification with source, licensing, review, and update strategy for all seven dimensions.
- Governance and data-ownership contract covering user-defined values, proposals, promotion, mapping, curation, rename, merge, deprecation, retirement, and rollback.
- Current-state recipe, tag, draft, versioning, import/export, and search baseline.
- Database schema, constraints, indexes, migration, backfill, and rollback plan.
- Typed API and serialization contracts for taxonomy discovery, canonical and miscellaneous value creation, assignment, curation, promotion, import/export mapping, and compatibility.
- Test-first vertical checkpoints for the taxonomy core, all seven dimensions, canonical and miscellaneous values, recipe authoring, versioning, display, import/export, and faceted search.
- Accessibility, localization, moderation, privacy, security, observability, performance, and release verification matrix.
- Explicit handoff contract for NOMS-018 quantitative ingredient and unit semantics.

## Proposed delivery slices

1. Prove the taxonomy domain model and database invariants with a minimal seed set and lifecycle tests.
2. Add read APIs, localized label resolution, alias matching, and administrative seed/update tooling for the seven dimensions.
3. Integrate assignments into recipe drafts, snapshots, versions, forks, restores, and export contracts.
4. Add recipe authoring and display for canonical concepts, user-defined miscellaneous values, and freeform tags.
5. Add import mapping and miscellaneous-value promotion with provenance, ambiguity review, and safe unmatched-value behavior.
6. Add indexed search and faceted filtering with alias, explicit-assignment, mapping, ownership, and visibility tests.
7. Validate migration/backfill, governance operations, localization, accessibility, performance, rollback, and downstream NOMS-018 compatibility.

Each slice must remain deployable with mixed old/new data and must define evidence that existing recipe creation, editing, history, and search behavior has not regressed.

## Definition of ready

- [ ] The taxonomy domain model, invariants, extension mechanism, and ingredient boundary are approved.
- [ ] The seven taxonomy dimensions, seed scope, data sources, licensing, and localization strategy are approved.
- [ ] Canonical and user-defined value ownership, visibility, creation, proposal, promotion, mapping, merge, deprecation, retirement, audit, and rollback behavior are documented.
- [ ] Recipe assignment, versioning, migration, backfill, and import-mapping behavior are documented.
- [ ] Search aliases, explicit assignments, facets, ranking, indexing, and performance targets are specified.
- [ ] Test-first checkpoints, acceptance evidence, compatibility expectations, and release gates are defined.
