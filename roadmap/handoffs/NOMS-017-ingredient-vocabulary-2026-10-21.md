# NOMS-017 ingredient vocabulary: session handoff

Date: 2026-10-21 (Asia/Shanghai). Remote repository: `SamKoCPSC/Noms`.
Branch: `codex/noms-017-ingredient-vocabulary`.
Implementation base: `36b9e62` (`feat(dev): add reproducible Codex cloud setup and browser checks (#4)`).

## Resume here

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

## Source coverage: current evidence

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

### Remaining USDA procedure

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

## Managed environment blocker and startup

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
