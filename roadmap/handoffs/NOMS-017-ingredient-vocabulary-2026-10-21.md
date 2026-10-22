# NOMS-017 ingredient vocabulary: session handoff

Date: 2026-10-21 (Asia/Shanghai). Remote repository: `SamKoCPSC/Noms`.
Branch: `codex/noms-017-ingredient-vocabulary`.
Implementation base: `36b9e62` (`feat(dev): add reproducible Codex cloud setup and browser checks (#4)`).

## Latest: cooking-focused FoodOn seed and test deployment

The user authorized selecting the full catalog's cooking-relevant labels, adding
them to the canonical seed and testing the deployment. This section supersedes
the historical statements below that the maintained seed was unchanged or that
restarting the pilot DB required a separate startup bundle.

The maintained `data/vocabulary/ingredients.json` now contains **10,148 active
entries** (100 curated plus 10,048 imported), and **60 retired staging identities**
retained for UUID/history stability: 10,208 total identities. There are 12,016
FoodOn mappings, 11,956 belonging to active entries. All original 100 IDs and
authored labels/aliases/descriptions/locales/statuses are unchanged. The tested
cumin UUID `1745729e-d12c-454d-92cd-1c448f08dd23` and alias `cumin` are retained.
Original pilot mapping relationships remain intact. The source manifest now
contains the pinned FoodOn release and attribution.

Selection is a policy-screened batch with explicit source exceptions, **not an
individual semantic certification of 20,152 source terms**. It includes regional
ingredients, edible parts, named varieties, pantry products and useful forms.
Generic taxonomy/physical-state/dosage categories, feed, medical formulations,
industrial ingredients, complete meals, unclear wording and incidental variants
are held out. Source exact synonyms are not bulk copied. Additional overlapping
source records use conservative related mappings; unresolved lexical/species
overlaps may still warrant editorial consolidation. The 60 retirements resulted
from the test-deployment review finding residual categories such as microbial
refined, rice-based dishes, industrial manufacturing variants and dosage forms.
Retirement hides them without deleting IDs or rewriting authored recipes.

Durable artifacts under `data/vocabulary/adapters/`:

- `foodon-cooking-policy.json`: source/catalog pins, scope, wording/alias overrides
  and explicit holds.
- `foodon-cooking-review.json`: persisted UUIDv4 decisions, preserved pilot maps,
  related source associations and retained retired identities.
- `foodon-cooking-base-ingredients.json` / `foodon-cooking-base-sources.json`:
  immutable original baseline from `aee6b6ae2ad4e3f5eed7e59e8b82d63971d2139d`.
- `foodon-cooking-summary.json`: selection/disposition counts and policy checksum.
- `foodon-cooking-validation.json`: exact input hashes, counts and verification.

`scripts/vocabulary/prepare_foodon_cooking.py` reproduces the selection using the
saved registry. The existing adapter builds it into complete canonical files.
`canonical.status: retired` is supported for retained staged imports and requires
an editorial reason. See `scripts/vocabulary/README.md` for reproducible commands.
Do not allocate new IDs on reruns or replace the preserved baseline with the
expanded seed. The coverage generator now checks its fixed 100 benchmark IDs
within the growing seed; original source-coverage totals remain unchanged.
Database ranking/transaction tests use the immutable small baseline, while the
Rust validator and content regressions validate the expanded maintained seed.

Verification completed:

- 20 Python adapter/content regressions, six Rust vocabulary tests, the actual
  Rust seed validator (zero warnings), formatting, CI/deployment regression and
  `git diff --check` passed. The first sandboxed DB-test attempt could not bind a
  local port; rerunning with network permission passed all six tests.
- Saved UUID decisions, selection summary and maintained seed/source files
  reproduced byte-for-byte. Every prior development ID and authored field was
  preserved; unsafe text/ambiguous source aliases were not silently adopted.
- Loaded into `noms-noms017` after a verified dry-run rollback. The initial bulk
  load added 10,107 IDs, updated 26 existing definitions and took about 4.1 s.
  Later editorial updates retained all those IDs and retired the 60 unsuitable
  staging entries. All stored canonical definitions match the maintained seed.
  Repeat loading changed no rows, timestamps or seed-run records, and seeding
  did not change recipes/drafts.
- Chromium 154 passed mock OAuth, hydration, alias lookup, label-only options,
  keyboard/mouse selection, canonical UUID autosave, reload, UUID clearing on
  edits, and free-text saving while lookup failed. All four WASM responses were
  200 and there were no page errors. Its 61 authenticated lookup cases passed,
  including common aliases, Unicode normalization, the eight-result cap, literal
  wildcards and excluded/retired categories. Local HTTP median was 7.102 ms,
  p95 47.21 ms and maximum 49.751 ms, excluding UI debounce; no load-test claim.
- Restarted only the verified owned app container through normal startup. It
  became HTTP 200 / Docker healthy in 36.06 s; vocabulary, seed-run and authored
  recipe/draft rows were unchanged. The previous startup-seed mismatch is resolved.

Machine-local evidence: `/workspace/foodon-adapter/cooking-review` (selection and
deployment results), `/workspace/foodon-adapter/cooking-seed-publish` (full adapter
report including source annotations), and
`/workspace/noms-browser-artifacts/foodon-cooking-final` (browser JSON/screenshots).
The inspected private draft is `83ba8355-847a-43e2-ab98-1e0b9dcf9287`, containing
all-purpose flour UUID `3bf07877-efe4-4ae4-af3c-0649e6c9895a`. No recipe was
published. The app was left running at `http://127.0.0.1:32768`; inspect current
metadata before assuming those ports survive an environment replacement.

## Resumption update: 2026-10-22

The USDA coverage review below is now completed for the scoped benchmark.
The original 2026-10-21 resume instructions, source totals and network blockers
are retained below as historical context and are superseded by this update.

Current artifacts:

- [Coverage report](../../data/vocabulary/coverage/README.md): methods, source
  totals, sampling strata, caveats and all 100 linked ingredient judgments.
- [USDA review](../../data/vocabulary/coverage/usda-review.json): generic and
  additional-product decisions, exact FDC descriptions, ingredient evidence,
  source releases, URLs, reasons and reproducible description queries.
- [Source snapshots](../../data/vocabulary/coverage/source-snapshots.json): four
  official USDA CSV ZIPs, byte lengths, SHA-256 values and ZIP member identities.
- [Archive inspector](../../data/vocabulary/coverage/inspect_usda.py): streams
  the pinned archives, reruns description searches and verifies retained evidence.

USDA's generic datasets supply **27/100** usable matches: Foundation 10,
SR Legacy 26 and FNDDS 10, with overlap. The targeted branded pass adds **34**
usable products among the 73 generic gaps, giving **61/100** observed USDA
coverage when product evidence is allowed. Branded records for the 27 already
generic-covered ingredients were not hand-graded; do not report a standalone
Branded coverage percentage for the entire benchmark.

The earlier Wikidata/FoodOn union remains 83/100. Generic USDA evidence adds
**masa harina**, taking the concept/generic union to **84/100**. Branded-only
evidence adds seven further gaps: **smoked paprika, gochugaru, white miso,
Chinese black vinegar, belacan, bagoong alamang and ackee**. The union including
products is **91/100**. Belacan is supported by a named `BLACHEN DRIED SHRIMP
PASTE` product, independently from generic fermented shrimp sauce and Filipino
bagoong. These are observed candidate-discovery lower bounds, not exhaustive
semantic absence claims, formal exact mappings or import approval.

The four archives contain 469 eligible Foundation foods, 7,793 SR Legacy foods,
5,432 FNDDS foods and 1,999,950 Branded records. Sample/acquisition rows in the
Foundation archive are excluded. Official downloads succeeded over the inherited
proxy with TLS verification on 2026-10-21; current runtime observations report
unrestricted, enforced HTTP policy. No personal USDA API key was configured,
and API/Wikidata reachability was not reverified in this USDA pass. The existing
Wikidata/FoodOn review judgments remain unchanged.

Validation on 2026-10-22: the archive inspector verified all four SHA-256 values,
ZIP member identities, eligible food-row counts, **119 distinct selected FDC
records** and **50 distinct selected product ingredient/metadata records**.
The report/CSV generator validates all 100 benchmark identities and source
judgments. No application-wide test run is claimed for this research update.
No canonical seed IDs, aliases, source mappings or application code changed.

```bash
python3 data/vocabulary/coverage/summarize.py
python3 data/vocabulary/coverage/inspect_usda.py --archives /workspace/usda-analysis
```

The archive directory is machine-local; redownload the manifest URLs into their
`local_filename` names if it is absent. Do not commit ZIPs, generated candidate
dumps or runtime state. The previous app-start task also rebuilt and started
`noms-noms017` and loaded the existing 100-entry seed. It had healthy HTTP checks;
the Chromium check was interrupted. Inspect current runtime state before relying
on that process or its old URL.

Remaining NOMS-017 work is still the broader metadata/ingestion scope described
at the end of this handoff. Stronger source-specific evidence can refine the
conservative related-only judgments; the USDA review itself is no longer blocked.

### FoodOn adapter implementation: 2026-10-22

The next ingestion slice is implemented in
[`scripts/vocabulary/foodon.py`](../../scripts/vocabulary/foodon.py), with
[commands and review semantics](../../scripts/vocabulary/README.md) and a
checksum/version-pinned descriptor under `data/vocabulary/adapters/foodon.json`.
`extract` reads the local RDF/XML snapshot and prepares source candidates plus
unapproved mapping proposals for the 50 usable benchmark matches. `build`
converts approved mappings/imports into complete canonical seed/source files in
a fresh output directory, preserving all existing IDs, wording and retirements.
New imported identities come from persisted UUIDv4 review decisions. No database
schema change or automatic runtime ingestion was added. The maintained seed and
source manifest remain unchanged.

The adapter preserves synonym scopes, English versus other source literals,
definitions, source notices and axiom annotations. It follows named subclass and
intersection paths from the food-material root, never treating existential
restriction fillers as ingredient parents. It rejects stale review inputs,
obsolete/out-of-scope terms, conflicting exact identities and output overwrites.
Source-derived wording is proposed explicitly; map decisions never overwrite
curated wording. Alias collisions remain separate and are reported.

Full validation is now complete: the pinned OWL was downloaded with verified
TLS and its exact SHA-256 checked. Extraction found 29,239 named FoodOn classes
and 20,152 eligible terms, including three explicitly reviewed material-scope
exceptions for dried konbu, annatto seed and teff kernel. Their named ancestry
does not lead to the food-material root in this snapshot. Imported parent IRIs,
outside-hierarchy flags and exception reasons remain visible; whole organism
and anatomy branches were not admitted. Review approval is also bound to the
complete descriptor checksum, so scope changes invalidate stale approvals.

The durable `data/vocabulary/adapters/foodon-pilot-review.json` contains 50
reviewed mapping decisions: 28 exact, 8 broader, 12 narrower and 2 related. This
is more conservative than treating all discovery E grades as formal exact maps.
In particular, saffron's source definition says anther rather than stigma, and
taro's says root rather than corm; both retain related evidence. The FoodEx2
soy-sauce term includes tamari and is broader than the canonical ingredient.

The full server seed CLI built successfully after fetching the missing
dependency, and rustfmt is now available. Its actual `--validate-only` mode
accepted the generated 100-entry, one-source pilot with no warnings and no
`DATABASE_URL`. Identical rebuilds produced byte-identical seed/source/report
files and preserved all 100 existing IDs, labels, aliases, descriptions, locales
and statuses. Invalid aliases and mutually exclusive validation/dry-run flags
were rejected before database access. A separate full-source cumin-seed import
probe also verified the new-entry path with the actual validator.

Validation also includes 14 offline adapter regressions, the CI/deployment
workflow regression, formatting and `git diff --check`. The earlier isolated
Rust module harness passed its two validator tests. No application-wide test
run is claimed. Raw sources, catalogs, generated pilot
files, the new-entry probe and the earlier harness live outside the checkout
under `/workspace/foodon-adapter`. The reproducible pilot output is in
`/workspace/foodon-adapter/pilot-seed`; the maintained seed is unchanged.

### FoodOn development UI verification: 2026-10-22

The user authorized loading the pilot into the development app and verifying
autocomplete. The environment restart had left stopped containers and retained
volumes but no `/tmp` lifecycle metadata. After checking all seven containers'
instance/project/token labels and the app's worktree mount, only those stopped
containers were removed (no volumes). The documented cloud `up noms017` command
then restored the stack from retained images and volumes; no cold rebuild was
needed. Its owning tool session was left running. Inspect metadata for current
ports rather than assuming the retained URL remains valid.

The actual seed CLI loaded `/workspace/foodon-adapter/pilot-seed`: 50 updated,
50 unchanged, one source, no warnings. A second load made no changes, including
timestamps/audit rows. Dry-runs of both this pilot and the extra import were
verified to leave entries, aliases, sources, mappings and seed-run tables
unchanged. `/workspace/foodon-adapter/new-entry-probe` then added cumin seed
(`FOODON_00003396`, UUID `1745729e-d12c-454d-92cd-1c448f08dd23`, alias `cumin`).
The development DB now contains **101 entries and 51 FoodOn mappings**: 29 exact,
8 broader, 12 narrower and 2 related. The maintained seed/source files remain
unchanged; this is a development integration probe, not seed promotion.

The repeatable [browser check](../../scripts/vocabulary/browser-smoke.py) passed
in Chromium 154 after mock GitHub OAuth: WASM/theme hydration, alias lookup,
label-only options, keyboard and mouse selection with canonical UUID autosave,
reload persistence, UUID clearing on edits, and free text while suggestion
requests fail. All four WASM responses were 200 and there were no page errors.
An initial harness attempt clicked before the post-OAuth document hydrated;
the check now waits for that document and verifies its theme interaction first.
No application change was required for the ingredient data to appear.

The final private draft is `fffefde7-ba23-4081-8d1c-c570a0fe9137`, titled
`FoodOn UI smoke 2b26c4863339`; a direct DB read confirmed revision 3, the cumin
UUID, private initial visibility and no published recipe. Browser screenshots
and result JSON are under `/workspace/noms-browser-artifacts/foodon-pilot`.
Seed logs, rollback/idempotency results and the saved-row check are under
`/workspace/foodon-adapter/ui-verification`. No credentials/cookies are saved in
those reports. The current app URL is `http://127.0.0.1:32768`.

**Restart constraint:** development startup still loads the maintained 100-entry
seed. The loader intentionally rejects dropping the extra probe UUID. Before
restarting against this DB, use a complete startup seed retaining that UUID, or
reset this disposable instance using the documented cloud down/up lifecycle.
The source mappings are data-only and have no dedicated provenance UI yet.

### Full-catalog FoodOn label triage: 2026-10-22

The user requested candidate review before a full-dataset seed trial. The
[label audit](../../data/vocabulary/adapters/foodon-label-audit.md) screens all
20,152 structurally eligible terms and records 54 specific editorial findings
in `data/vocabulary/adapters/foodon-label-review.json`. Reproduce with
`scripts/vocabulary/audit_foodon_labels.py`; the full 20,152-row review queue,
346 shared-name groups, summary and input hashes are outside the checkout in
`/workspace/foodon-adapter/label-audit-v2`.

Signals overlap: 5,999 classification labels, 1,154 ontology-worded labels,
81 terms in the named animal-feed branch, 13 hazard-category terms, 4,351
preparation/storage matches, and seven proposals violating canonical text
constraints. There are 346 normalized label/alias strings shared across 587
terms, despite no duplicate normalized labels by themselves. Ambiguities include
peppercorn/Capsicum `pepper`, allspice/pimiento `pimento`, caper buds/fruit/plant
aliases, and regional biscuit/cookie/cracker wording. Source exact synonyms are
not automatically safe culinary aliases: one soft-shell-crab synonym is a
326-character explanatory paragraph. Five labels contain control/mojibake
characters, and one proposed definition exceeds 2,000 characters.

This is mechanical full-catalog triage plus selected semantic checks, not
20,152 individual import approvals. Even labels without flags are unapproved.
Do not blanket-strip parentheses or exclude all feed/soup/baby words: yeast
activity, dried/canned tomato forms, bee pollen, canned soup and baby spinach
demonstrate why scope/identity checks matter. Cosmetic cleanup also does not
repair the saffron-anther or taro-root source-definition mismatches. The
maintained seed, source mappings, existing pilot review and development DB were
not changed by this audit.

## Original resume instructions (2026-10-21, superseded)

The ingredient vocabulary slice is implemented and was exercised end to end.
The immediate unfinished task is the **three-source coverage assessment for the
100-ingredient international benchmark**, particularly USDA FoodData Central.
The user published an environment configuration allowing USDA and Wikidata, but
this session remained bound to an older configuration. Do not repeat the full
implementation or regenerate canonical IDs. Check out this branch, read the
coverage report, verify actual network access, and finish the source review.

```bash
git fetch origin
git switch --track origin/codex/noms-017-ingredient-vocabulary
python3 data/vocabulary/coverage/summarize.py
```

If the local branch already exists, switch to it normally instead of recreating
it. The summary command validates and regenerates saved results; it does not
query the sources or perform the missing USDA review.

## User decisions and authorization

- Maintained suggestions live in PostgreSQL, sourced from version-controlled
  canonical seed files. Third-party APIs are ingestion/research inputs, never a
  runtime dependency for autocomplete.
- Suggestions remain optional. Custom ingredient text must save immediately,
  even when autocomplete fails. No separate miscellaneous-value workflow.
- Personal-history suggestions are deferred. The small time classification
  dataset should eventually be constants/shared client code; time rules were
  not implemented in this ingredient slice.
- The dropdown displays **labels only**, without descriptions. Descriptions
  remain available in the canonical data for editorial context.
- Ingredient popularity is a counter directly on the vocabulary row. Following
  the roadmap's privacy contract, only current **public published** recipes
  contribute. Private/unlisted recipes and drafts do not affect global ranking.
- International ingredient coverage is a primary goal. The user requested a
  100-ingredient sample spanning familiar and less familiar cuisines and a
  comparison of Wikidata, FoodOn, and USDA FoodData Central.
- The user explicitly authorized committing/pushing these changes to a new
  remote branch and writing this handoff. No production deployment, merge, or
  automatic promotion of research mappings has been requested.

## Implemented behavior

### Canonical data and seeding

`data/vocabulary/ingredients.json` contains **100 curated entries**. The original
15 entries, including IDs, labels, aliases and descriptions, were compared with
the retained earlier seed and are unchanged. The 85 additions have UUIDv4 IDs
assigned once. All external source mappings and the source manifest remain empty:
research candidates have not been silently approved as imported vocabulary.

`src/vocabulary/mod.rs` defines the version-1 seed contract, normalization and
validation. Comparison keys use Unicode NFKC, lowercase and collapsed whitespace;
display text is preserved. UUIDs identify entries, not normalized names. Duplicate
IDs are rejected. Label/alias collisions within one entry are rejected; collisions
between entries generate warnings and do not merge identities. The database
deduplicates aliases per entry, not globally. Different forms/preparations can
therefore remain distinct even where upstream naming overlaps.

`src/vocabulary/store.rs` provides transactional loading, autocomplete and count
reconciliation. Loading validates the whole file and manifest, compares existing
definitions, updates seed-owned aliases/mappings, and records changed runs by
checksum. Dry-run uses the same transaction and rolls back. An unchanged rerun
does not update timestamps or add audit records. Omitted previously maintained
IDs fail rather than deleting rows; use explicit retirement.

Two deployment-role commands are available:

```bash
just seed-vocabulary --dry-run
just seed-vocabulary
just reconcile-vocabulary-counts --dry-run
just reconcile-vocabulary-counts
```

The seed CLI also accepts `--file PATH` and `--sources PATH`. Run from the
repository root with the deployment role's `DATABASE_URL` configured normally.
Do not print credentials. Production server startup does not seed automatically;
local development's app entrypoint seeds after schema initialization.

### Schema, recipe compatibility and API/UI

The repository uses a declarative `migrations/schema.sql` and `pgschema`, not a
new numbered migration file. New tables hold entries, aliases, source manifests,
source mappings and seed runs. Prefix indexes use `text_pattern_ops`.

`RecipeIngredient.vocabulary_id` is an optional UUID with serde default/omission
behavior for old JSON. Authored ingredient names remain independent. References
travel through drafts, publication, forks, snapshots, comparisons and restores.
Unknown references do not prevent ordinary recipe text from remaining readable.
Existing fixture changes mostly add `vocabulary_id: None`.

`src/api/vocabulary.rs` exposes the Dioxus server function
`ingredient_suggestions`. It requires authentication, validates input length,
uses the existing per-user autocomplete request budget and database/search work
budgets, and returns maintained active English ingredient suggestions.

`src/components/ingredient_autocomplete.rs` is used in new/edit recipe forms.
Lookup starts at two characters, with a 200 ms browser debounce. It rejects stale
responses, supports arrows/Enter/Escape/Tab, and uses combobox/listbox semantics.
Selecting a label stores its ID; every text edit clears the old reference.
Results are deduplicated by entry ID and capped at eight. Ranking is exact label,
then exact alias, then prefix matches ordered by public recipe count descending,
normalized label and ID. Alias and label prefixes share the same popularity tier.
Failures leave the input usable and display a small availability hint.

### Popularity and permissions

`vocabulary_entries.recipe_count` is a nonnegative BIGINT. The database trigger
`maintain_ingredient_recipe_counts` maintains distinct canonical ingredient IDs
per current public recipe, including publication, ingredient edits, visibility
transitions, restores, deletions and account-deletion cascades. Multiple rows for
one ingredient in one recipe count once. A failed transaction rolls back counts.
Unknown IDs are ignored until seeded; seeds reconcile references to new entries.
Counts are never supplied by the browser or seed definition.

Recipe writers acquire shared advisory lock `170018`; seeding/reconciliation
acquire it exclusively. Seed runs also serialize on `170017`. Counter updates
take vocabulary rows in stable UUID order. The trigger is SECURITY DEFINER with
a fixed `pg_catalog` search path and qualified application relations. Runtime
credentials cannot write vocabulary tables or directly execute the definer
function. They can read entries and aliases. Seeds and reconciliation require
deployment credentials.

For an existing deployment, pause recipe writes, apply the declarative schema,
reapply `migrations/runtime-grants.sql` through the established deployment-role
workflow, reconcile counts, then resume writes. `just migrate` reads local
connection settings from `.env.local`; do not improvise a production connection.
The vocabulary README has the rollout details. No production rollout occurred.

## Source coverage: 2026-10-21 baseline

Read these committed artifacts first:

- `data/vocabulary/coverage/README.md`: method, caveats, regional table, findings
  and the full 100-row linked matrix.
- `benchmark.json`: stable ID, label, aliases, culinary scope and sampling stratum.
- `wikidata-review.json` and `foodon-review.json`: each reviewed candidate ID,
  URL, status and rationale.
- `coverage.csv`: flat matrix suitable for analysis.
- `source-snapshots.json`: precise FoodOn artifact identity and access limitations.
- `summarize.py`: validates alignment and regenerates CSV/report from reviewed JSON.

The benchmark retains 15 existing controls and adds 85 ingredients across 18
regional usage groups (19 strata total). Examples include doenjang, shio koji,
padaek, buah keluak, goraka, tklapi, kashk, korarima, ogiri, Bambara groundnut,
huacatay, tucupi, and Pacific staples. These are culinary usage contexts, not
exclusive geographical origins. This is a purposive long-tail stress test,
not a random or frequency-weighted sample of all international ingredients.
Individual strata contain only 2–15 items; do not generalize their percentages.

| Source | Usable E | Related R | No specific match confirmed N | Unassessed |
| --- | ---: | ---: | ---: | ---: |
| Wikidata | 68 | 32 | 0 | 0 |
| FoodOn | 50 | 30 | 20 | 0 |
| USDA FoodData Central | — | — | — | 100 |

The usable union is **83/100**: 35 overlap, 33 Wikidata-only, 15 FoodOn-only, and
17 with neither usable match confirmed. Usable discovery coverage is not formal
exact ontology equivalence and is not permission to import a source unchanged.
Plant/taxon matches alone do not count as edible ingredient coverage. Required
part and transformation matter; compatible presentations can qualify where
identity is preserved, such as whole/ground culinary sumac fruit spice.

Wikidata is particularly helpful for named regional preparations and blends;
FoodOn contributes useful edible-part and processed-material distinctions.
The current evidence supports complementary candidate discovery with manual
curation. USDA's incremental value has not yet been measured.

### Wikidata limitations and traps

The API was blocked. These are observed lower bounds from indexed search,
public entity pages, Wikipedia sitelinks, redirects and plant-to-food relation
traversal. Cached pages are not one atomic snapshot. Related-only rows may hide
more specific records. Do not report them as proven absent from Wikidata.

Examples of identity checks that must survive future ingestion:

- Tomato food is `Q20638126`; `Q23501` is the plant.
- Garlic food is `Q21546392`; `Q23400` is the plant.
- Kombu food is `Q11261547`; reject a same-name disambiguation page.
- Gochugaru needs processed Korean chili powder, not Korean chili pepper
  `Q48189338` by itself.
- Follow plant-to-food relations where appropriate: perilla leaf `Q27650144`,
  pili nut `Q104188819`, teff seed `Q103205493`, taro root `Q134452914`.
- Iru and dawadawa overlap regionally; the benchmark uses distinct condiment
  ogiri for entry 73 instead of counting both as independent identities.

### FoodOn snapshot and traps

The full retained OWL snapshot was reviewed, including definitions and synonyms.
Pin this artifact when reproducing the result:

```text
commit: cd73540243a84bcd511a500d9a497d12d7dc02f6
URL: https://raw.githubusercontent.com/FoodOntology/foodon/cd73540243a84bcd511a500d9a497d12d7dc02f6/foodon.owl
SHA-256: b897bf64c1b265c422db9ee01e1235c10c0b809e7f02326ef810a52068347edf
bytes: 40779019
version IRI: http://purl.obolibrary.org/obo/foodon/releases/2026-09-20/foodon.owl
```

The file has two versionInfo values (2026-02-02 and 2026-09-20); the commit and
digest identify the actual reviewed artifact. It is CC-BY-4.0; inspect notices
for selected imported terms before distributing derived seed content. FOODON
FoodEx2 categories count if their definitions identify the scoped food; technical
labels need culinary editorial cleanup.

Reject lexical false positives: FoodOn's `scotch bonnet` can be a mushroom,
`pimento` can mean sweet pepper rather than allspice, and generic shrimp paste
can list both belacan and bagoong alamang as synonyms without preserving their
regional preparation differences. `FOODON_00002529` masa conflates wet dough and
flour in its definition and does not unambiguously establish dry masa harina.
Do not collapse garlic, garlic powder and garlic paste through loose matching.

### Original USDA procedure (completed by the update above)

1. Verify access to the official download page and API through the inherited
   proxy. A sandbox socket denial is distinct from a proxy policy 403.
2. Use the official bulk CSV exports, record release, URL, byte count and SHA-256.
   The API DEMO_KEY has a low quota and is unsuitable for a 100-item search pass.
   No personal USDA API key was configured in this session.
3. Assess Foundation, SR Legacy and FNDDS as generic data separately from
   Branded. Count additional branded-only ingredient products separately. A
   ready meal or mention inside an ingredient list is not an ingredient match.
4. Stream ZIP members (especially `food.csv`) rather than extracting nutrition
   tables. The prior download page advertised approximately 460 MB compressed
   and 3.1 GB expanded for the full CSV archive; recheck the current release.
   This machine had only about 1.7 GB free. Do not extract the full archive.
5. Generate candidates using each benchmark label and aliases, then review
   identity/part/form manually. Record FDC ID, exact description, data type,
   source release, URL and grading reason for every selected positive result.
   Keep ambiguous/broader matches separate. Do not assign a no-match verdict
   when a request failed or the searched release was incomplete.
6. Add the USDA review artifact, extend `summarize.py` (which currently emits
   BLOCKED for every USDA row), regenerate CSV/report and record USDA's
   incremental union coverage. Update `source-snapshots.json` and this handoff.
7. If Wikidata API access now works, strengthen candidate discovery and retrieve
   revision-aware entity evidence, especially for the 32 related-only cases.
   Re-review any changed judgments using the same ingredient scope.

Official entry point: https://fdc.nal.usda.gov/download-datasets/
Previously advertised filenames included
`FoodData_Central_foundation_food_csv_2026-04-30.zip`,
`FoodData_Central_sr_legacy_food_csv_2018-04.zip`, and
`FoodData_Central_branded_food_csv_2026-04-30.zip`; verify them on the live page.
No USDA archive was downloaded and no coverage percentage was fabricated.

## Original managed environment blocker and startup

The user published additions for `fdc.nal.usda.gov`, `api.nal.usda.gov`,
`www.wikidata.org`, and `query.wikidata.org`. Last checks of the bound runtime
still showed only custom hosts `apt.postgresql.org` and `storage.googleapis.com`,
with the package-manager preset. Current observations were true and policy was
enforced, yet normal proxy requests to all four research hosts returned
`Tunnel connection failed: 403 Forbidden`. Later status revisions alone did not
mean that the new published source-configuration version was adopted.

On resumption, read the installed cloud-environment-runtime skill and networking
reference if available, call `environment_status`, inspect
`/etc/codex/network-policy.json`, and test the exact endpoints. Publishing an
editable configuration did not rebind this session. There was no exposed refresh,
restart or rebind tool. The user may need to recreate/select the environment
through the platform UI. Restarting the app or Docker cannot update the sidecar
policy. Do not bypass the proxy, alter routes or disable TLS verification.

For application work, read `docker/codex-cloud/README.md` before any startup.
Inspect containers, images, disk space and provenance first; request Docker
socket access with the approval mechanism when needed and reuse compatible
retained images. Do not cold-build on an almost full disk or blanket-prune
unrelated images/volumes. The research itself does not require starting Noms.
The committed cloud scripts describe how to rebuild a missing environment.

The earlier app/browser flow was validated locally with mock OAuth, PostgreSQL
and MinIO. It was never publicly deployed. The original development DB had the
15-entry seed; **the expanded 100-entry seed has not been applied to the running
application DB**. A future normal development startup will load it through the
entrypoint after schema setup. Inspect actual state rather than assuming the
stack or its processes survived.

## Validation and known limits

Fresh branch-publication checks on 2026-10-21 passed: formatting, benchmark/report
regeneration, **10 focused tests** (6 vocabulary library, 2 ingredient reference/
history, 1 recipe popularity, 1 runtime-grants), strict server library/binaries
Clippy, and `git diff --check`. The full 1,083-test application binary suite and
live browser flows were not rerun for this push.
The checked-in tests cover seed validation/loading, rollback and unchanged reruns,
incremental additions, reference serialization/history, counts, reconciliation,
concurrency, visibility and runtime permissions. They use disposable PostgreSQL
databases, not the configured application DB.

```bash
cargo fmt --all -- --check
python3 data/vocabulary/coverage/summarize.py
SQLX_OFFLINE=true cargo test --features server --lib vocabulary -- --test-threads=1
SQLX_OFFLINE=true cargo test --features server --bin noms ingredient_vocabulary -- --test-threads=1
SQLX_OFFLINE=true cargo test --features server --bin noms ingredient_popularity -- --test-threads=1
SQLX_OFFLINE=true cargo test --features server --bin noms runtime_grants -- --test-threads=1
SQLX_OFFLINE=true cargo clippy --features server --lib --bins -- -D warnings
git diff --check
```

The previous host helper `/workspace/noms-tools/env.sh` supplied workspace-local
Cargo/Rustup, PostgreSQL 17 tools/libraries, SQLX_OFFLINE=true, build jobs=8 and
test debug=0. It is machine-local and not part of the branch. A replacement host
must provision tools or use the documented development container; do not assume
this helper exists. Database tests may require approval to bind local ports.
The old Rust toolchain identified itself as 1.99 during Clippy checks.

Earlier browser verification exercised suggestions after incremental seed
additions, alias-to-canonical selection, distinct garlic forms, regional
ingredients, free text, failed lookup, keyboard selection, draft reload, edit,
publication and light/dark desktop/mobile layouts. The later label-only check
confirmed descriptions were absent. Popularity browser verification observed
garlic-paste count 0 → 1 after publication → 0 after deletion, popular prefix
ordering, exact-label priority and no page errors. These browser checks were
not repeated just to publish this handoff.

Earlier broad Clippy checks hit unrelated existing warnings: the WASM binary's
`chunks_exact_to_as_chunks` in settings profile, and all-target test lints in
upload, version graph, search, unlink recovery and recipe scaler code. Do not
claim the entire CI suite or every target is green from the focused checks.

## Durable branch contents versus old local artifacts

All feature source, schema/grants, seed files, review matrices, report generator
and this handoff are committed. The branch does not contain Docker layers,
databases, credentials, auth cookies, CA files, compiled binaries, raw browser
logs, screenshots or a 40 MB ontology dump. Those do not belong in source Git.

If the old workspace survives, useful non-durable locations were:

- `/workspace/work/foodon-analysis/foodon.owl` and `classes.json`: retained
  source and parsed classes; redownload and verify the pinned hash if absent.
- `/workspace/work/coverage-100/`: raw indexed-search/page captures and temporary
  review scripts. The old initial search capture for row 73 still says dawadawa;
  the committed benchmark/review correctly uses ogiri, with a later verification.
- `/workspace/work/vocabulary-evidence/`: original feature screenshot bundle
  and test logs; `/workspace/work/ingredient-label-only.png`: later dropdown check.
- `/workspace/work/popularity-validation/results.json`: earlier successful
  browser popularity observation and screenshots in the same directory.
- `/workspace/work/handoff-validation/`: fresh focused-check logs for this push.

Treat these as optional evidence, not required checkout inputs. The user asked
to remove the earlier shared validation Page, and it was removed. Do not recreate
it. Earlier file-sharing links failed with a filename/file-ID error; do not
assume old artifact links will be accessible from a new session. Use the client's
supported attachment mechanism for any newly requested screenshots.

## Remaining scope after the source review

This branch is the ingredient slice, not completion of all NOMS-017 metadata.
Other dimensions, time boundaries, localization policy, source adapters and
deduplication review pipeline, automatic recognition/backfill, portable export
specification, facets, merge redirects and personal suggestions remain future
work. The existing implementation plan contains broader discovery questions;
read its final initial-slice section alongside the actual code and vocabulary
README rather than treating every historical question as still undecided for
this delivered slice. A vocabulary ID never implies substitution or quantity
compatibility; those belong to NOMS-018.
