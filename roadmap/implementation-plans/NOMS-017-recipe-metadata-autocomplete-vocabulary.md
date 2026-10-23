# NOMS-017 Recipe Metadata and Autocomplete Vocabulary Implementation Plan

**Status:** Discovery — flexible metadata and autocomplete contracts under refinement
**Created:** 2026-08-15
**Updated:** 2026-09-30
**Depends on:** Current recipe schema, recipe editor, versioning, and search contracts
**Coordination:** NOMS-016 imports and exports authored metadata and optional vocabulary references; NOMS-018 may consume recognized ingredient references but owns quantities, units, pantry, ingredient compatibility, and grocery semantics; NOMS-019 consumes cooking-method, equipment, and timing metadata.
**Approach:** Keep recipe fields open to author-entered values. Maintain a large, extensible vocabulary to provide convenient autocomplete, aliases, and optional recognition without enforcing a controlled taxonomy.

## Intended outcome

Authors can describe recipes in their own terms while autocomplete helps them discover and reuse familiar values. Suggestions help authors enter values; they never constrain saving. Normal field validation, such as length limits and cardinality, still applies.

Recipes retain the actual text their authors entered. Selecting a suggestion may also attach a stable vocabulary reference, but an unrecognized value is equally valid. Updating the vocabulary must not silently rewrite recipe text or historical snapshots.

For example, an ingredient field may suggest `chickpea` when an author types `garbanzo`, while still accepting `black chickpeas` or `grandma's spice mix` without requiring the author to create a canonical concept or choose a miscellaneous-value mode.

## System scope

### Recipe metadata fields

- Ingredients and ingredient aliases: suggestions for existing ingredient-name entries, preserving quantities, units, preparation details, and the author's wording.
- Cuisines and culinary traditions: multiple independent labels without imposing a geographic or culinary hierarchy.
- Difficulty: one author-entered label, with a small suggested scale rather than an enforced enumeration.
- Time: one classification derived from numeric recipe times rather than an independently authored label. Numeric prep and cook times remain the source of truth.
- Dish types: forms such as soup, stew, salad, bread, cake, or sandwich, independent of cuisine or occasion.
- Equipment: suggestions for existing structured recipe-equipment names.
- Cooking methods and techniques: multiple labels such as baking, braising, grilling, steaming, fermenting, or sous vide, independent of dish type and equipment.

These are the only dimensions in scope. Vocabulary values can grow without schema changes or application releases. Dietary suitability, allergens, meal types, occasions, courses, seasonality, and cooking context remain out of scope; authors may express these through existing freeform tags where appropriate.

### Dimension cardinality

Cardinality follows whether a recipe can logically have multiple simultaneous values. Every dimension remains optional so existing, incomplete, and imported recipes remain valid.

| Dimension | Recipe cardinality | Storage rule |
| --- | --- | --- |
| Ingredients | Many | Each ingredient entry retains its name and may have at most one recognized vocabulary reference. |
| Cuisines and culinary traditions | Many | Each label retains its text and may have one reference; no primary cuisine is required. |
| Difficulty | Zero or one | One authored label with an optional reference; unfamiliar labels are accepted. |
| Time | Zero or one | At most one category derived from numeric time data, not selected independently. |
| Dish types | Many | Multiple authored labels, each with an optional reference. |
| Equipment | Many | Each equipment entry retains its name and may have at most one recognized vocabulary reference. |
| Cooking methods and techniques | Many | Multiple authored labels, each with an optional reference. |

Adding a value to a repeatable dimension does not replace existing values. Editing a singular dimension replaces its current value through normal draft and version history. Time classification boundaries and handling of incomplete time data remain to be specified; missing time data must not imply a zero-duration recipe.

### Flat vocabulary model

- Values within a dimension are independent. This ticket does not model parent/child trees or general related-value graphs.
- Broad and specific labels may coexist, such as `Chinese` and `Sichuan` or `soup` and `noodle dish`. Search does not infer ancestor or descendant membership.
- Vocabulary entries have stable, opaque identifiers and editable labels, aliases, descriptions, and locale data. Labels and slugs are not identity.
- Aliases provide reviewed alternative names for suggestions and recognition; shared or similar text does not automatically prove equivalence.
- A vocabulary reference means that a value was recognized or selected, not that ingredients are interchangeable or equipment is compatible.
- Retirements and merge redirects preserve reference compatibility without introducing a browsing hierarchy or changing authored recipe text.

### Authoring and optional recognition

- Use the same input workflow for suggested and custom values. Do not introduce separate canonical and miscellaneous authoring paths.
- Store authored text independently of any optional vocabulary reference. Selecting a suggestion can populate its displayed label; typing an alias must not trigger a silent replacement of the author's text.
- Accept values absent from the vocabulary. Users do not need moderation approval, vocabulary connectivity, or a successful lookup to save a recipe.
- Initially suggest maintained entries only. Personal-history suggestions are deferred; custom values remain valid without recognition.
- Editing linked text must re-evaluate or clear its reference; a stale reference must not remain attached to a value with a different meaning.
- Do not automatically link ambiguous matches. Any automatic exact or alias recognition must use an explicit, tested matching policy and preserve the entered text.
- Keep freeform recipe tags separate from these named metadata fields; do not silently reclassify existing tags.
- Preserve metadata text and optional references in drafts, snapshots, versions, forks, comparisons, restores, and exports.

### User values and vocabulary maintenance

- Custom values are ordinary recipe metadata, not a separately governed taxonomy class. They work immediately. Personal suggestions are deferred from the initial delivery.
- Private recipe values may inform only that author's suggestions; they must not leak into global suggestions, public usage counts, or public curation candidates.
- Public recipe values may be searched and displayed with their recipes without first becoming maintained vocabulary entries or global autocomplete suggestions.
- Common values from public recipes can become candidates for maintained suggestions. Frequency alone must not automatically publish a new suggestion or establish an alias relationship.
- Reviewed additions may create a new vocabulary entry or add an alias to an existing entry, with source and review provenance.
- Adding, renaming, merging, or retiring a suggestion does not rewrite recipe text, bulk-link existing values, or mutate historical snapshots. Any later backfill of references must be explicit, reviewable, and reversible.
- Initial maintenance can use reviewed repository changes and administrative tooling; a dedicated proposal or promotion interface is not required to make author-entered values usable.
- Define abuse prevention, access control, audit history, and rollback for maintenance operations. Avoid building a moderation workflow for every new recipe label.

### Vocabulary seed data and source ingestion

- Maintain vocabulary seed values as declarative, version-controlled files in the repository, with one independently validated file per dimension. Time category labels and rules will instead be small constants and a pure function in shared Rust code, outside the seed dataset; boundaries and incomplete-time behavior remain to be decided.
- Each suggestion entry includes its committed stable identifier, dimension, label, aliases, optional locale data and description, lifecycle state, and source and attribution metadata.
- FoodOn is the leading candidate source for ingredient suggestions and selected culinary terms from the preceding research. Confirm coverage for each dimension rather than assuming one dataset supplies all seven; difficulty suggestions and time rules can be defined by Noms.
- Treat external datasets as inputs to a suggestion vocabulary, not as a taxonomy to enforce on recipes. Import only terms useful for recipe authoring, keeping scientific or industrial terms out of default suggestions where inappropriate.
- Use a reproducible pipeline: pin a source release or commit and checksum, extract candidates, normalize for matching, review useful terms and aliases, generate per-dimension seed files, validate, and load.
- Retain upstream identifiers and source provenance separately from Noms vocabulary identifiers. Verify licensing and attribution for selected terms, including terms imported by an upstream dataset from other sources, before distribution.
- Do not treat broader, narrower, or merely related upstream terms as interchangeable aliases. Ambiguous labels may have multiple suggestions rather than being merged by text alone.
- Production autocomplete reads the local vocabulary and does not depend on live third-party APIs. Source updates are reviewed diffs, not automatic changes to recipes or global suggestions.
- Validate identifiers, dimensions, cardinality rules, references, duplicate entries, alias ambiguity, lifecycle transitions, and required attribution. Apply seeds with an idempotent loader that never overwrites recipe text or personal values.
- Removing an entry from a seed file must not silently delete referenced data. Use explicit retirement or replacement behavior and retain safe fallback display from recipe text.
- The initial ingredient slice uses versioned JSON in `data/vocabulary/ingredients.json`, with a separate `sources.json` manifest and PostgreSQL runtime tables. Initial vocabulary size and source selections for further expansion remain to be finalized.

### Search and filtering

- Index authored metadata so unfamiliar values remain discoverable under the recipe's existing visibility rules.
- Recognized references and reviewed aliases can support more consistent matching; unrecognized values still support text matching without implying semantic equivalence.
- Define exact-text and recognized-value filter behavior and facet counts explicitly. Do not imply that a vocabulary filter covers every synonymous custom label, or count the same recipe twice through both text and reference matches.
- Keep search normalization separate from the stored display text. Do not infer ingredient substitutions, cooking-method equivalence, or cuisine relationships from spelling similarity.
- Vocabulary maintenance must not silently change authored meaning. Document whether maintained aliases and redirects affect current search results while historical recipe text remains unchanged.

### Import and export

- Define lossless serialization for authored metadata text, dimension, optional vocabulary references, and relevant source and recognition provenance.
- Preserve unmatched and ambiguous imported values without requiring resolution before saving.
- Resolve references only when the relevant dimension and identity are known. Unknown, retired, or unavailable references must not make portable recipe text unusable.
- Keep imported wording intact unless an author explicitly edits it. Define how recognition can be added without silently replacing source text.
- Coordinate these fields with the NOMS-016 portable snapshot contract before that schema is frozen.

### Ingredient boundary with NOMS-018

NOMS-017 owns ingredient-name suggestions, reviewed aliases, source mappings, and optional recognition references. It does not require every ingredient to have a canonical identity.

NOMS-018 owns amounts, units and conversion, density or substitution rules, pantry inventory, compatible aggregation, and grocery-list arithmetic. A suggestion match alone is not permission to combine quantities or treat foods as interchangeable. For example, `chili`, `chili powder`, and `chili flakes` must not collapse through loose matching. Unrecognized ingredients remain usable as authored text; the existing exact-name baseline in NOMS-018 remains unchanged unless a separate integration contract explicitly replaces it.

## Working decisions

- Recipe metadata is flexible author-entered text, supported by autocomplete rather than constrained by a strict taxonomy.
- Retain the seven agreed dimensions and their optional singular or repeatable cardinality; time remains derived from numeric fields.
- Keep the vocabulary flat, with stable identifiers and reviewed aliases but no hierarchy.
- Selecting or recognizing a suggestion may attach a reference; that reference is optional and independent of the stored text.
- There is no separate miscellaneous-value mode or approval requirement for custom recipe labels.
- Reuse personal values privately; review common public values before adding them to global suggestions.
- Preserve authored text and recipe history through vocabulary updates, merges, and retirements.
- Maintain independently validated repository seed files per dimension and a reproducible, idempotent ingestion and loading process.
- Treat search recognition and quantitative ingredient compatibility as distinct concerns.

## Questions to resolve

- What is the exact text-plus-optional-reference representation for metadata and existing ingredient/equipment entries, including recognition provenance?
- Does selecting an alias suggestion populate the alias or its preferred label, and what should the UI communicate before selection?
- Which automatic recognition cases, if any, are safe without author confirmation? How are ambiguous matches and edited linked text handled?
- How should maintained suggestions, personal history, locale, and matching aliases be ranked and deduplicated?
- Which initial sources and subsets cover each dimension, with what licensing, attribution, update cadence, and useful vocabulary size?
- Which file format best supports human review, localization, validation, and a large ingredient vocabulary?
- What difficulty labels and time category boundaries should be suggested, and how should incomplete time data be classified?
- What text normalization, localized label fallback, transliteration, exact filters, recognized-value filters, and facet-count rules apply?
- How should public usage generate review candidates without leaking private values or enabling suggestion spam?
- What migration and optional reference-backfill behavior is justified, and what audit and rollback evidence is required?

## Required implementation plan deliverables

- Flexible metadata and vocabulary model, including text preservation, optional references, cardinality, flat semantics, provenance, localization, and lifecycle rules.
- Initial per-dimension seed specification with source selection, licensing, attribution, candidate review, and update strategy.
- Current-state recipe, tag, draft, versioning, import/export, and search baseline.
- Database constraints, indexes, migration, optional backfill, compatibility, and rollback plan.
- Typed API and serialization contracts for autocomplete, recipe metadata, optional recognition, maintenance, and lossless import/export.
- Authoring interaction contract covering custom values, suggestion selection, ambiguous matches, stale references, and unavailable autocomplete.
- Privacy and maintenance contract for personal suggestions, public candidates, reviewed global additions, audit, and abuse prevention.
- Test-first checkpoints covering all seven dimensions, custom and recognized values, recipe history, imports, search, and seed loading.
- Accessibility, localization, security, observability, performance, and release verification matrix.
- Explicit handoff to NOMS-018 that does not infer quantitative compatibility from vocabulary recognition.

## Proposed delivery slices

1. Prove flexible metadata storage, optional references, cardinality, text preservation, and mixed old/new snapshot compatibility with a minimal vocabulary.
2. Add per-dimension seed files, source provenance, validated idempotent loading, and vocabulary read APIs.
3. Integrate text and optional references into drafts, versions, forks, restores, and the portable export contract.
4. Add autocomplete and display for all applicable fields, accepting custom values and preserving saving behavior when suggestions are unavailable.
5. Add safe import recognition, reviewed aliases, personal suggestion reuse, and audited maintenance without requiring a custom-value promotion workflow.
6. Add indexed text search and optional recognized-value filtering with visibility, alias ambiguity, deduplication, and facet-count tests.
7. Validate migration, rollback, privacy, localization, accessibility, performance, and downstream NOMS-018 compatibility.

Each slice must be deployable with mixed old/new data and provide evidence that existing recipe creation, editing, history, imports, and search have not regressed.

## Definition of ready

- [ ] The flexible metadata model, cardinality, optional references, text-preservation rules, and ingredient boundary are approved.
- [ ] The seven dimensions, initial suggestions, time rules, seed scope, sources, licensing, and localization strategy are approved.
- [ ] Autocomplete selection, ambiguity, personal reuse, public review candidates, and privacy behavior are documented.
- [ ] Vocabulary updates, aliases, reference compatibility, audit, and rollback do not silently change authored text or historical snapshots.
- [ ] Draft/version storage, migration, import/export, and optional reference-backfill behavior are documented.
- [ ] Text search, recognized-value filters, facets, ranking, indexing, and performance targets are specified.
- [ ] Acceptance tests prove custom values save without vocabulary lookup, unknown references remain readable, stale links are cleared, and private values do not leak.
- [ ] Test-first checkpoints, compatibility expectations, release gates, and NOMS-018 handoff requirements are defined.

## Initial ingredient implementation slice

The first slice provides the declarative PostgreSQL schema, an explicit
`seed-vocabulary` CLI with dry-run and transactional updates, and maintained
English ingredient autocomplete in new/edit recipe forms. See
[`data/vocabulary/README.md`](../../data/vocabulary/README.md) for the seed contract
and loading procedure. Optional ingredient IDs travel with authored text in
existing JSON drafts and snapshots; editing the name clears the UI reference.
The maintained seed includes 100 curated ingredients plus the FoodOn expansion
described below, retaining the original fifteen IDs. Source assessments use a
fixed 100-ingredient international benchmark under `data/vocabulary/coverage`.
Other metadata dimensions, additional source adapters, vocabulary-aware portable
export policy and personal suggestions remain future work.

Ingredient suggestions also rank prefix matches by a maintained `recipe_count`
on the canonical row. Only current public published recipes contribute, once per
ingredient per recipe; private/unlisted recipes and drafts do not influence global
popularity. A transactional database trigger maintains counts across publication,
edits, visibility changes, restores, and deletions. The deployment-only
`reconcile-vocabulary-counts` CLI backfills or repairs counts; seeds preserve them.
Exact label and alias matches retain priority over popularity, and the UI remains
label only. See the vocabulary README for rollout and concurrency details.

For current implementation state, verification and the completed USDA
benchmark review, read the
[dated session handoff](../handoffs/NOMS-017-ingredient-vocabulary-2026-10-21.md).
The [coverage report](../../data/vocabulary/coverage/README.md) distinguishes
27/100 generic USDA matches from 34 additional branded-only products. The
three-source union is 84/100 with generic USDA evidence, or 91/100 including
products; these research candidates have not been promoted to seed mappings.

The initial [FoodOn adapter](../../scripts/vocabulary/README.md) now extracts a
pinned local OWL file into review candidates and builds complete canonical
seed/source files from approved decisions. Existing IDs/text remain stable and
source synonyms retain their scopes. Full-snapshot extraction, 20 offline
regressions and the actual Rust seed CLI validation passed. The durable pilot
review records 50 mappings (28 exact, 8 broader, 12 narrower, 2 related) and
preserves all 100 existing identities/texts. The pilot initially left the maintained seed unchanged.
The generated pilot and a cumin-seed import probe were subsequently loaded into
the disposable development DB (101 entries, 51 mappings). Verified dry-run
rollback, idempotency, and the browser flow through mock sign-in, alias lookup,
keyboard/mouse selection, canonical UUID autosave, reload, and free-text fallback
all passed. The [repeatable UI check](../../scripts/vocabulary/browser-smoke.py)
requires a live local stack and loaded bundle; no provenance-management UI is included.

The subsequent cooking-focused expansion is now promoted: **10,148 active entries**
(100 curated plus 10,048 imported), with 60 staged UUIDs retained retired.
All prior development IDs remain in the maintained seed, resolving the pilot's
startup-seed constraint. The selection policy, original baseline, UUID/review
ledger are versioned under `data/vocabulary/adapters/foodon-cooking-*`.
The adapter guide records seed counts; generated summaries, source annotations
and complete hold queues stay outside Git. This is policy-based
screening with explicit exceptions; unresolved semantic overlaps can still need
editorial consolidation. The coverage study remains the fixed 100-entry benchmark.
