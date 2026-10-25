# Preferred ingredient labels (2026-10-25)

The explicit [review ledger](../../data/vocabulary/adapters/ingredient-label-review.json)
changes ordinary parenthetical cooking labels into natural word order. Examples:

| Prior label | Preferred label |
| --- | --- |
| tomato (canned) | canned tomatoes |
| chicken breast (skinless, boneless) | boneless skinless chicken breast |
| almond (shell off) | shelled almonds |
| beef (ground, extra lean) | extra lean ground beef |
| tomato (pieces, canned) | canned tomato pieces |
| light whipping cream (30–36% milk fat) | light whipping cream with 30–36% milk fat |

All **2,868 active labels containing parentheses** receive a recorded decision:
**2,624 renames**, **202 contextual holds**, **12 nested/nonterminal holds**, and
**30 name-ownership holds**. Holds retain scientific names, unclear relations or
qualifiers, and identities whose proposed natural name belongs to another UUID.
For example, `tomato (sun-dried)` remains separate from the existing owner of
`sun-dried tomatoes`. This review does not decide duplicate-identity merges.
Retired entries remain byte-for-byte equivalent as JSON objects.

No ingredient IDs, statuses, descriptions, provenance, mappings, source manifests
or recipe text change. Stock/broth, species, parts, canned/dried/frozen states,
fat/sodium formulations and packing media remain distinct. Ordinary English
ordering and a bounded set of idiomatic forms/plurals change the display names.
Technical or scientific information is retained when a natural replacement is
uncertain. This is a review of display wording, not certification of upstream
semantics or a blanket rule for future imports.

Every exact prior label/alias remains a searchable name of its original UUID.
For **1,597** renames an existing alias becomes the preferred label; its alias
slot is removed to avoid repeating the same normalized name within one entry.
All renamed entries retain the old label as an alias. The other **1,027** new
preferred names raise active aliases from 7,522 to **8,549**. Entry counts remain
**13,055 active and 81 retired**. Selection still requires an explicit author
choice; vocabulary labels do not overwrite authored ingredient names.

## Reproduction

The review follows commit `1d015efd6f9409817056a3d62283d970a68e907a` and its
checksum-pinned gap-follow-up output:

- Input SHA-256: `373d0b7b4c19f6c8e664655d1ebafedbae97e8028503cca63733c62e9609dd80`
- Output SHA-256: `f6f5d4021ac18e7a4897752aa4c31aa9d2957ebf17608d4c6e5353ca0d280e4a`
- The four-source manifest is unchanged and separately pinned in the ledger.

Obtain `ingredients.json` and `sources.json` from that input commit into a fresh
external directory. Then run from the repository root:

```sh
python3 scripts/vocabulary/rename_labels.py \
  --seed /tmp/label-base/ingredients.json \
  --sources /tmp/label-base/sources.json \
  --output /tmp/label-review
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
```

The builder refuses a stale baseline, missing review decisions, mismatched alias
promotions, name collisions or an unapproved output checksum. It emits complete
canonical files and an old/new/held CSV audit. `restore_baseline` reconstructs
the exact prior stage and verifies its checksum so earlier source/gap tests keep
their immutable expectations. No upstream downloads are needed.

## Verification and limits

All **94 Python tests** pass. The authoritative Rust validator accepts all
13,136 entries and four sources with no warnings. The local test deployment
passes transactional dry-run rollback, **2,624 updates**, unchanged source
metadata, database/JSON definition equality and a zero-change repeat load.
All existing draft, recipe and immutable-version row hashes remain unchanged.

All **5,248 old/new full-label SQL lookups** return the original UUID first using
the production query's normalization, escaping, rank order and eight-result
limit. A bounded browser run passes **192 authenticated lookup checks**, plus
keyboard/mouse selection, UUID save/reload, editing clears the ID and free-text
fallback on lookup failure. A pre-update private draft reloads with the original
`tomato (canned)` authored name and UUID; a new selection saves `canned tomatoes`
with that same UUID. The app remains healthy; no production rollout occurs.

The frozen 1,000-profile review retains **10,349/10,768 covered material lines**
and **667 fully covered profiles**. Expected candidate retrieval for full recipe
wording improves **9,359 → 9,370**: 11 gained lines and zero lost lines. Considering
full wording or three/four-character prefixes, results shift **9,531 → 9,520**:
19 gained lines and 30 lost lines across 12 recipe names. Renaming changes the
alphabetical tie-breaker among broad prefix matches. Those candidate identities
and all their prior names remain available; every old/new full label resolves
first correctly. No ranking, popularity, parser or identity rules were changed
to mask this tradeoff. The external comparison lists every affected name.

Review artifacts and projected recipe checks are in `/workspace/vocabulary-labels`.
The downloadable `/workspace/noms-vocabulary-readable-labels.zip` contains full
vocabulary JSON/CSV, the old/new/held audit, replay tools, source pins and the
projected recipe analysis. Raw catalogs, recipe instructions, browser sessions
and private draft data are excluded. Earlier downloadable reports stay frozen.


## Later curation

The [second-sample gap layer](GAPS.md#second-sample-curation-2026-10-25) follows
this immutable output. Its own ledger adds ingredients/alias suffixes without
changing these preferred labels. `maintained_label_stage` verifies and removes
that later layer before historical label tests; `restore_maintained_baseline`
then recovers the original pre-label seed. The label review's pins and results
above remain frozen; current counts are in the maintained vocabulary README.
