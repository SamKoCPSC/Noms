# Maintained ingredient vocabulary

`ingredients.json` is the reviewed source of truth for ingredient suggestions.
The original fifteen entries retain their IDs. The seed now contains 100 curated
ingredients across 19 benchmark strata, including regional staples, fermented
condiments, grains, herbs, and spice blends. The sampling strata describe culinary
usage contexts, not exclusive origins. See `coverage/benchmark.json` and the
coverage review for source-matching evidence. These remain curated entries;
research candidates have not been automatically promoted to source mappings.
Personal suggestions are deferred. Time classification
is outside this dataset and belongs in shared application code.

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
(as 64 hexadecimal characters), licenses, and attribution. The current entries
are manually authored, so their external mappings/references and the manifest are
empty; no upstream review or attribution is implied.

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
