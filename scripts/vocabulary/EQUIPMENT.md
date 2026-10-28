# Equipment vocabulary research

This stage extracts and evaluates candidates. It does not create an equipment
seed, assign Noms vocabulary UUIDs, approve aliases, or write to a database.
See the [dated evaluation](../../docs/equipment-vocabulary-evaluation-2026-10-28.md).

## Maintained review inputs

`data/vocabulary/research/equipment/` contains:

- `wikidata-capture.json`: roots, exact discovery queries, raw-result hashes,
  selected IDs/revisions, source bytes/checksum, scope and CC0 attribution.
- `review.json`: explicit proposed labels, families, tiers and rationale, bound
  to the source checksum. All other records remain excluded or held by the
  evaluator's conservative scope rules. This is not a seed approval ledger.
- `benchmark.json`: independent, overlapping Wikibooks, PRISM and Noms common
  kitchen checklists. Noms' 141-item checklist is editorial, not random sampling.
- `gap-review.json`: classification and evidence for every common-checklist row
  without a shortlist name match, plus specific identity/alias cautions.
- `supplemental-entities.json` / `supplemental-capture.json`: separately pinned
  targeted evidence. It does not change hierarchy capture or coverage counts.
- `reference-manifest.json`: captured reference hashes and attribution. Category
  membership cannot be restored from a category-page revision alone; exact raw
  reference responses are preserved in the research archive.
- `summary.json`: deterministic evaluation counts, input checksums and limits.

Large hierarchy snapshots, API revision caches, raw query results, generated
CSVs, search results and the downloadable archive remain outside Git.

## Restore the pinned capture

From the repository root, using Python 3.11+ and the standard library:

```sh
python3 scripts/vocabulary/fetch_equipment.py \
  --restore data/vocabulary/research/equipment/wikidata-capture.json \
  --discovery /workspace/equipment-replay/discovery \
  --cache /workspace/equipment-replay/revision-cache \
  --output /workspace/equipment-replay/snapshot
```

An empty revision cache needs network access to Wikimedia's API. An existing
complete immutable cache restores without network access. Snapshot bytes must
match the pinned checksum. Suppressed or unavailable upstream revisions fail
restoration rather than substituting current versions. The downloaded archive
also includes the exact snapshot and can be evaluated offline directly.

## Evaluate offline

```sh
python3 scripts/vocabulary/equipment_research.py \
  --snapshot /workspace/equipment-replay/snapshot/entities.json \
  --descriptor data/vocabulary/research/equipment/wikidata-capture.json \
  --policy data/vocabulary/research/equipment/review.json \
  --benchmark data/vocabulary/research/equipment/benchmark.json \
  --output /workspace/equipment-replay/evaluation
python3 scripts/vocabulary/test_equipment_research.py
```

Outputs are all 2,312 candidate rows, the 249-row proposed shortlist (including
13 supplies/safety items), duplicate-label and shared-alias queues, coverage rows
and summary counts. Every candidate retains its source ID and revision URL.
No matching operation merges identities or approves an upstream alias. CSV
formula-like source cells are escaped for spreadsheet viewing; source JSON
remains unchanged. Proposed labels have their own match provenance and are never
presented as original upstream wording.

## Discover a future capture

Omit `--restore` and use fresh external discovery/cache/output directories. The
capture runs twelve separate SPARQL queries: classes, instances and
English-Wikipedia-linked instances for each of four roots. Detailed capture
selects all class IDs plus those article-linked instances; the full discovery
registry and result files retain the records outside detailed review.

An existing discovery directory is a resumable cache, not a freshness request.
Use a fresh directory for a new discovery date. Root results are not an atomic
global dump; if an item has different revision observations, the latest observed
revision is selected and raw results retain the observations. A new capture
cannot use the old review policy unless its source checksum matches.

## Before a seed

Resolve potential duplicate identities and narrower equipment distinctions,
review aliases explicitly, incorporate appropriate targeted discoveries, and
decide supply/safety scope. Keep historical recipe text intact. In particular,
generic mixers, heating vessels and grouped catalogue terms are not permission
to collapse stand/hand mixers, double boilers/water baths or unrelated products.
