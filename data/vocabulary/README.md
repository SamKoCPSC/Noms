# Maintained ingredient vocabulary

`ingredients.json` is the reviewed source of truth for ingredient suggestions.
The seed contains **12,781 active ingredients**: the original 100 curated entries,
10,027 FoodOn-derived cooking/pantry entries, 2,550 Wikidata additions and 104
USDA additions.
Another 81 identities are retained as retired: 60 from the original deployment
review and 21 from the subsequent ingredient-scope cleanup.
Every original ID and authored
label/alias/description is preserved, including the original fifteen IDs. The
previously tested cumin-seed UUID is also retained. There are 12,016 FoodOn source
mappings (11,935 on active entries); overlapping source records use conservative `related` relationships
rather than inferred exact equivalence from matching names.

The expansion uses a recorded cooking-scope policy and explicit source-specific
exceptions. This is batch screening, not individual semantic certification of
every source annotation. It includes regional foods, edible parts, named varieties,
pantry products and useful forms; organizational/industrial/medical categories,
complete meals and unclear terms are held out. Source synonyms are not copied
wholesale. See `adapters/foodon-cooking-policy.json` and the persisted UUID/review
ledger in `adapters/foodon-cooking-review.json`.

The [Wikidata adapter](../../scripts/vocabulary/WIKIDATA.md) adds 2,550 ingredients
and 1,120 additional source associations, totaling 3,670 Wikidata mappings.
Its identity review gives all 4,449 initially unresolved candidates an explicit
outcome; 1,440 stay held for specific scope/evidence reasons. The combined seed
also retains the same 60 retired identities. Existing IDs and authored wording
are preserved; related evidence does not hard-merge duplicate legacy UUIDs.
The source snapshot is pinned by entity revisions and checksum, and full case
audits remain outside Git.

The [USDA adapter](../../scripts/vocabulary/USDA.md) screens all 8,262 food records
in pinned Foundation Foods and SR Legacy releases, adds 104 explicitly reviewed
identities and 1,142 related associations, and holds 7,016 records for scope or
further definition. Its 1,246 mappings retain FDC IDs and archive provenance.
Analytical/production qualifiers can propose related evidence for an existing
ingredient; they do not establish exact equivalence or link nutrient values.
All 12,758 pre-USDA IDs and authored fields are preserved. Raw archives and full
audits stay outside Git. FNDDS, Branded and nutrient ingestion remain deferred.

The subsequent [alias and scope review](../../scripts/vocabulary/ALIASES.md) screens every
active name and adds 7,009 reviewed plurals, regional spellings, alternative names
and qualified word-order variants to 4,823 entries. There are now 7,137 active
aliases, plus two retained on newly retired entries. Source/definition review of
200 flagged labels retires 21 unsuitable entries, including livestock salt blocks,
clinical formulations, manufacturing terms, catch-all categories and finished
meals. Their UUIDs, labels, aliases and source evidence remain available for
history; only active suggestion status changes. The recorded policy holds
conflicting aliases and reproduces from the immutable combined pre-alias seed.
It does not add ingredients or change runtime matching semantics.

The source coverage research still measures the original **100-ingredient**
benchmark across 19 strata. Those percentages do not describe the expanded seed.
See `coverage/benchmark.json` and the coverage review for source-matching evidence.
Personal suggestions are deferred. Time classification
is outside this dataset and belongs in shared application code.

The [FoodOn adapter](../../scripts/vocabulary/README.md) extracts a checksum-pinned
OWL snapshot into review candidates and converts approved decisions into complete
canonical seed/source files. It preserves existing IDs and authored wording.
Use `just seed-vocabulary --validate-only --file PATH --sources PATH` to run the
Rust validator without connecting to a database before a normal loader dry-run.

Apply the declarative schema with the existing `just migrate` workflow first.
With the deployment/seed role's `DATABASE_URL` exported, run:

```bash
just seed-vocabulary --dry-run
just seed-vocabulary
```

The recipe editor searches maintained English labels and aliases by prefix after
two characters. Results prefer exact labels, then exact aliases, then prefixes;
prefix matches sort by public recipe count descending, then label and ID. Label
and alias prefixes share the same popularity tier. Results show labels only,
are deduplicated by entry ID and capped at eight. Selecting a result uses
its label and optional ID. Typing always accepts custom text and clears an old ID.
Missing suggestions never prevent saving. Existing ingredient JSON without IDs
remains readable; draft and published snapshots retain optional IDs independently
of subsequent vocabulary changes. A reference does not imply interchangeability.

## Seed contract (version 1)

- `dimension`: currently `ingredient`; `locale`: currently `en`.
- `id`: a non-nil UUID assigned once and retained across renames and retirement.
- `label`, `aliases`: display wording, 1–200 characters; comparison keys use NFKC,
  lowercase, and collapsed whitespace. No stemming or inferred equivalence.
- `description`: up to 2,000 characters of cooking context.
- `status`: `active` or `retired`. Retired entries retain their IDs and are hidden
  from autocomplete. Keep them in the file; omission fails rather than deleting.
- `provenance`: `origin` is `curated` or `imported`; `references` identifies source
  manifest entries. Imported entries require at least one reference.
- `source_mappings`: source ID, external ID, and `exact`, `broader`, `narrower`, or
  `related` relationship. These mappings do not merge Noms entries.

`sources.json` records external source IDs, HTTPS URLs, versions, SHA-256 checksums
(as 64 hexadecimal characters), licenses, and attribution. The manifest now pins
FoodOn's 2026-09-20 snapshot with CC-BY-4.0 attribution and a revision-pinned
Wikidata structured-data capture under CC0-1.0, and USDA Foundation Foods
2026-04-30 and SR Legacy 2018-04 public-domain archives under CC0-1.0.
Per-term FoodOn source notices
remain in the adapter's generated report; the manifest does not replace review
of third-party definition attribution. The original entries retain curated origin;
new source-derived entries use imported origin.

A normalized label/alias must be unique within an entry. Collisions between
entries produce warnings and remain separate suggestions for review. IDs are
never deduced from names. The loader does not ingest upstream data directly.

## Loading and maintenance

The CLI validates the complete file and manifest before connecting, then acquires
one transaction-scoped advisory lock. Entry updates, aliases, mappings, source
metadata, and the seed-run checksum record commit together. A failed run rolls
back; `--dry-run` runs the same comparison and SQL constraints but rolls back.
A repeated unchanged input does not update timestamps or add audit records.
Changed aliases and mappings replace the seed-owned sets for that entry.

Use `--file PATH` and `--sources PATH` for an explicit alternative file pair. Each
ingredient file must represent the complete maintained ingredient vocabulary.
Run one loader after schema setup as a deployment step, not per application
replica. Local Docker development does this before `dx serve`. Production runtime
credentials have SELECT access only; seed loading requires deployment credentials.
Seeds own maintained vocabulary rows; direct administrative editing is deferred.

Before an upstream update, review additions, changes, alias ambiguity, and source
notices in the dry-run. Recovery is a reviewed forward seed change with the same
IDs. Preserve entries once published; retirement is safer than deleting an ID
that may occur in recipe history. This initial slice does not auto-link typed
aliases, backfill recipes, or provide cuisine/equipment vocabulary.

## Ingredient popularity

`vocabulary_entries.recipe_count` counts distinct **current public published
recipes** that carry the ingredient's canonical ID. Repeated ingredient rows
count once. Private and unlisted recipes, drafts, immutable versions, custom text,
and unknown IDs do not affect global ranking. Visibility changes, publishing,
ingredient edits, restores, recipe deletion, and account-deletion cascades update
the count in the recipe transaction. A failed publication rolls the delta back.
Retired ingredients retain counts but remain excluded from suggestions.

The database trigger uses atomic increments in UUID order. Its fixed search path
and qualified relations allow runtime recipe writes without granting runtime
write access to vocabulary rows. An exclusive maintenance advisory lock serializes
seed loading and reconciliation with the trigger's shared lock. No popularity
values are accepted from the browser or stored in seed definitions.

For rollout to an existing database, pause recipe writes, apply `just migrate`,
reapply `migrations/runtime-grants.sql` using the deployment role, then backfill
before resuming writes:

```bash
just reconcile-vocabulary-counts --dry-run
just reconcile-vocabulary-counts
```

These commands require the deployment role's `DATABASE_URL`; runtime credentials
cannot rewrite counters. Dry-run reports how many rows differ without modifying
them. Repeating a completed reconciliation reports zero corrections. Existing
recipes are backfilled from their IDs, without changing authored text or history.
New seed entries also reconcile existing references before committing. Do not
delete or zero counts while recipes still reference the entries; use this repair
command instead. The full rebuild scans public recipes, so schedule large rebuilds
during quiet periods; ordinary recipe writes only update changed ingredient IDs.

## Verification

```bash
SQLX_OFFLINE=true cargo test --features server --lib vocabulary
SQLX_OFFLINE=true cargo test --features server --bin noms ingredient_vocabulary
SQLX_OFFLINE=true cargo test --features server --bin noms ingredient_popularity
SQLX_OFFLINE=true cargo test --features server --bin noms runtime_grants
```

Database tests require PostgreSQL tools on PATH and permission to bind a local
port. They use disposable databases, not the configured application database.
The seed test loads the first ten entries, adds the remaining entries, and checks
no-op reruns, alias matching/removal, renames, retirement, rollback, and the prefix
index. History tests cover draft saves, publication, forks, and restored versions.
Popularity tests cover duplicate rows, prefix ranking, exact-match priority,
seed preservation, reconciliation, concurrent writes, visibility transitions,
late-write rollback, ownership failures, deletion cascades, and runtime grants.
