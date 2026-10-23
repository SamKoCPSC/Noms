# Wikidata → Noms seed adapter

The offline adapter reads a local, checksum-pinned entity snapshot. It extracts
candidates and builds complete canonical seed/source files from approved,
revision-bound decisions. It does not query Wikidata during builds or server
requests, write a database, or infer exact identity from names.

## Current review

The pinned capture contains **9,197 items**: 9,157 from the food-ingredient
instance/subclass hierarchy plus 40 benchmark items outside that discovery set.
The initial query also returned four lexeme senses; these are excluded because
they are not item entities. This is a bounded discovery set, not all foods in
Wikidata. Query-service data and entity revisions are not one atomic global dump.

All captured labels and all **10,148 active baseline Noms names**, including their
aliases, were mechanically screened. The saved policy includes 129 explicit
editorial cases. The resulting batch contains:

- **45 new pantry ingredients**, including garam masala, ssamjang, XO sauce,
  mirin, ponzu, kecap manis and Urfa biber.
- **616 mappings across 607 existing ingredients**. The 548 associations from
  unambiguous normalized labels use `related`; the 68 explicit mappings retain
  their reviewed relationship, including narrower/broader/related scope.
- **8,536 held source entities**, including missing English text, taxonomy,
  non-food objects, brands, chemicals, complete dishes and unresolved identities.
  Of these, 4,267 need identity/deduplication review before any new import.

Every item receives a disposition. This is policy-assisted screening with
explicit case review, not individual semantic certification of all 9,197 items.
A held or unmatched name is not proof that Wikidata lacks the ingredient. The
fixed coverage benchmark is unchanged; its discovery grades are not imported
as exact mappings automatically.

The combined seed has **10,193 active entries and 60 retired entries**. All prior
10,208 UUIDs, authored fields, origins and FoodOn mappings are preserved. New
aliases are explicit editorial choices; Wikidata aliases are never bulk copied.
Taxa/disambiguation/category/list pages and unresolved redirects cannot become
culinary identities. For example, Scotch bonnet's pinned taxon record remains
held, tahini's toasted/hulled scope is narrower, and generic bagoong is broader
than shrimp-specific bagoong alamang. Museum eggs, bleach, turpentine, fictional
syrups and a percussion instrument found in the hierarchy are excluded.

## Maintained inputs

- `data/vocabulary/adapters/wikidata.json`: snapshot checksum/bytes, capture
  context, discovery membership, every entity revision, and the baseline commit.
- `wikidata-cooking-policy.json`: cooking holds and explicit editorial decisions.
- `wikidata-cooking-review.json`: approved revision/UUID/mapping decisions and
  checksums binding them to the descriptor and baseline seed/source files.
- `wikidata-ingredients.rq`: repeatable item discovery query for future captures.

The capture keeps English labels/descriptions/aliases and selected semantic
statements, including their ranks, qualifiers and references. Deprecated
statements do not define scope; preferred values supersede normal values.
Structured Wikidata content is CC0; Wikipedia article prose and images are not
imported. See [Wikidata data access](https://www.wikidata.org/wiki/Wikidata:Data_access).

Raw revision responses, the 12.6 MB projected snapshot, generated name-review
CSVs, summaries and selected-entity reports stay outside Git. The revision
registry allows restoring the snapshot without committing those downloads.

## Reproduce the reviewed batch

Requires Python 3.11+ and the standard library. Capture requires network access;
all subsequent extraction, review, tests and builds work offline. Use fresh
output directories. A dedicated external cache resumes successful revision
batches, uses two workers and backs off on errors/load limits.

Recover the baseline from history rather than adding another seed copy to Git:

```bash
mkdir -p /tmp/noms-wikidata-base
git show 891825d4c4b0aba053db132e5baa9c31c2f8dd16:data/vocabulary/ingredients.json > /tmp/noms-wikidata-base/ingredients.json
git show 891825d4c4b0aba053db132e5baa9c31c2f8dd16:data/vocabulary/sources.json > /tmp/noms-wikidata-base/sources.json
python3 scripts/vocabulary/fetch_wikidata.py \
  --restore data/vocabulary/adapters/wikidata.json \
  --cache /tmp/noms-wikidata-cache --output /tmp/noms-wikidata-snapshot
```

Restore requests the exact entity revisions through the MediaWiki revisions API,
projects the retained fields deterministically, then verifies full-file bytes and
SHA-256. Missing/suppressed revisions and checksum mismatches fail the restore.
The saved revisions are authoritative; rerunning a live discovery query may yield
a different set. Omit `--restore` only to prepare a new capture for review; it
must not silently replace an approved descriptor or reuse a source ID with
different metadata.

Extract an unapproved candidate queue, then reproduce the policy-screened review:

```bash
python3 scripts/vocabulary/wikidata.py extract \
  --snapshot /tmp/noms-wikidata-snapshot/entities.json \
  --seed /tmp/noms-wikidata-base/ingredients.json \
  --sources /tmp/noms-wikidata-base/sources.json \
  --output /tmp/noms-wikidata-candidates
python3 scripts/vocabulary/review_wikidata.py \
  --snapshot /tmp/noms-wikidata-snapshot/entities.json \
  --seed /tmp/noms-wikidata-base/ingredients.json \
  --sources /tmp/noms-wikidata-base/sources.json \
  --output /tmp/noms-wikidata-review
cmp /tmp/noms-wikidata-review/review.json data/vocabulary/adapters/wikidata-cooking-review.json
```

The review directory includes `source-names.csv` (every captured item),
`seed-names.csv` (every active baseline ingredient), full JSON evidence and a
summary with policy checksum. Unmatched, ambiguous and held records stay visible.
The command reuses persisted import UUIDs and refuses to silently drop them.
Changing the policy requires inspecting the new decisions; never regenerate
random identities to get a clean build.

```bash
python3 scripts/vocabulary/wikidata.py build \
  --snapshot /tmp/noms-wikidata-snapshot/entities.json \
  --seed /tmp/noms-wikidata-base/ingredients.json \
  --sources /tmp/noms-wikidata-base/sources.json \
  --review data/vocabulary/adapters/wikidata-cooking-review.json \
  --output /tmp/noms-wikidata-seed
cmp /tmp/noms-wikidata-seed/ingredients.json data/vocabulary/ingredients.json
cmp /tmp/noms-wikidata-seed/sources.json data/vocabulary/sources.json
just seed-vocabulary --validate-only \
  --file /tmp/noms-wikidata-seed/ingredients.json \
  --sources /tmp/noms-wikidata-seed/sources.json
```

Builds reject stale approvals, changed descriptors/baselines, conflicting exact
mappings, invalid IDs/text, imported name collisions and unknown review fields.
Mapping decisions cannot replace authored labels/aliases/descriptions. New imports
require explicit wording, an eligible food entity and a saved UUIDv4. Selected
entity/revision evidence accompanies the full output bundle outside Git.

After validation, use the existing deployment-role loader's dry-run and rollout
procedure in the [seed guide](../../data/vocabulary/README.md). This batch has not
been loaded into a test or production database. UI loading is the same canonical
seed path as FoodOn; no separate Wikidata API call runs in the recipe editor.

## Verification

```bash
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py' -v
```

Offline fixtures cover revision/checksum binding, rank semantics, unresolved
redirects/taxa, explicit approvals, collision rejection, preservation of authored
fields and retirements, stable UUID reruns, policy holds and the maintained ledger.
The full capture, source/name audits, review and seed/source files were reproduced
with zero alias collisions. Raw capture is a separate integration check, not a
network dependency in CI.
