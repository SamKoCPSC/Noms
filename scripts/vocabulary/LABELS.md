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

## Readable-label follow-up (2026-10-27)

After the user identified confusing names such as `hen egg white, dried`, a full
lexical audit screens **13,617 records: 13,532 active and 85 retired**. It flags
1,492 active labels across catalogue commas, parenthetical/technical wording,
trailing modifiers, packaging/use language and malformed punctuation. The
external audit records 608 draft names; 123 collide with names owned by other
active IDs. A lexical flag is a review lead, not proof that a label is wrong.

The user-approved follow-up changes **488 preferred labels**. **139 existing
aliases** become preferred names; all previous preferred labels become aliases.
There are **349 additional active aliases**, yielding **13,532 active, 85 retired
and 10,094 active aliases**. No ingredients are added, merged or retired. Every
UUID, description, source association, status and authored recipe field is
preserved. The explicit ledger is
[`ingredient-readable-label-review.json`](../../data/vocabulary/adapters/ingredient-readable-label-review.json).

| Prior label | Preferred label |
| --- | --- |
| hen egg mixed whole, dried | whole chicken egg powder |
| hen egg white, dried | dried chicken egg whites |
| hen egg yolk, dried | dried chicken egg yolks |
| coffee ground, roasted | roasted ground coffee |
| milk powder, full fat | full-fat milk powder |
| cocoa beverage-preparation, powder | cocoa drink powder |
| pate, chicken liver | chicken liver pate |
| yoghurt, goat milk | goat milk yogurt |
| mussel shell off, raw) | raw shelled mussels |
| spelt flour, medium | semi-refined spelt flour |
| berry (impregnated with sugar) | sweetened berries |

These are per-ID decisions, not a blanket comma inversion. The egg white/yolk
definitions specify dried forms without limiting every product to powder, so
their labels retain that broader physical-form scope. The whole-egg definition
explicitly describes powder. Cocoa drink powder stays distinct from cocoa
powder; semi-refined spelt and the leafy part of black nightshade are supported
by the existing definitions. Names already present as aliases are promoted with
their exact wording so no prior searchable string is removed.

**1,009 reviewed labels remain unchanged.** They include conflicting names such
as `cheese, brie`, `juice, apple` and `milk powder, skimmed`; unclear source or
classification terms; and useful established names such as cocoa mass, soy
protein concentrate, liquid coffee concentrate, refined olive oil, passion fruit
pulp, reconstituted milk and transglutaminase preparation. Two bovine labels are
held because their definitions include yak/zebu/buffalo as well as cattle. No
duplicate identities or source-scope problems are resolved by silently renaming
them. Named species, edible parts, maturity, processing, concentration, fat and
other formulation distinctions are retained.

### Readable-label replay and verification

The separate review follows commit `ec68a2b3d00c3cfe7ba518d73e1f09766375d930`:

- Input seed: `4dbdc76d56668cc99f8ec57d941fe463a5861bc3c53f16202e179617b15b2571`
- Output seed: `3de52f891231718606912b69dd80eaf4dd304d86ac5ea9b9eb2051599618b483`
- Unchanged sources: `1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`

Obtain the input seed/sources from that commit in a fresh external directory:

```sh
python3 scripts/vocabulary/rename_labels.py \
  --seed /tmp/readable-base/ingredients.json \
  --sources /tmp/readable-base/sources.json \
  --policy data/vocabulary/adapters/ingredient-readable-label-review.json \
  --output /tmp/readable-replay
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
```

The label builder accepts an explicit, unique active-ID review scope for this
later ledger; the original parenthetical-only review retains its old behavior.
Every reviewed ID has exactly one change/hold. After the user approved the
12 wording candidates in [the awkward-language review](AWKWARD_LABELS.md), five
additional IDs extend the original 1,492-ID scope to 1,497 reviewed IDs. The
other seven approved IDs move from hold to change. Every previous name stays
searchable, including the singular `wonton wrapper` alias. The original audit's
flag counts remain historical evidence, not the amended scope count.
Stale seeds, incomplete decisions,
retired/missing/duplicate scope IDs, name collisions, stale alias promotions and
unreviewed field changes fail validation. `maintained_next_gap_stage` removes
this layer before the next-sample tests; all earlier ledgers and checksums stay
unchanged. Only compact decisions, maintained data, tests and documentation are
committed; raw audits and downloads remain external.

The initial 475-rename iteration produced seed checksum
`7eee963a2fc28ffd6c56065d8022f6eab99341344158c1b25171c830de2a175f`.
The deployment, Rust and browser evidence below belongs to that frozen iteration;
the subsequent berry and 12-wording amendments are separately described in
[the awkward-language review](AWKWARD_LABELS.md). It updates this current PR's
readable-label ledger while preserving every earlier curation stage and pin.

For that initial iteration, all **135 vocabulary Python tests** pass.
Replay is byte-for-byte identical, and
the Rust validator accepts all **13,617 records/four sources without warnings**.
The restarted local mock-auth app's normal startup seed applies **475 updates**,
with zero additions/retirements/source updates. A subsequent dry run makes no
changes and rolls back completely; `--if-changed` then skips without writes.
Recipe/draft/version and source/mapping hashes match the prior verified database
snapshot, before and after browser checks.

All **1,003 preferred/legacy labels and aliases on renamed IDs** return their
original UUID first through the production SQL lookup. Chromium passes **950
old/new full-label checks**, without per-query server calls, page errors or a new
draft. Its cache refreshes despite the unchanged 13,532-entry count. A test-only
route supplies the prior catalogue, reconstructed byte-for-byte with its known
SHA-256 because restarting discarded the server's in-memory old snapshots; the
override is removed before the real manifest/catalogue refresh. Search/ranking
code is unchanged. Broad-prefix ordering can change with a new preferred label;
full old names continue to identify the same ingredients.

Audit artifacts remain at `/workspace/vocabulary-readable-label-audit`, with the
original frozen download `/workspace/noms-vocabulary-readability-audit.zip`.
Update decisions, full seed, replay, old/new CSV and verification artifacts are
external at `/workspace/vocabulary-readable-label-update`.

## Kitchen suitability follow-up

The later [kitchen-suitability review](KITCHEN_REVIEW.md) resolves all 24 remaining
awkward-language candidates with three natural-name changes and 21 retirements.
It uses a separate ledger after this frozen 488-rename stage.
`maintained_readable_stage` verifies and reverses that layer before the tests in
this section. Current totals are 13,511 active, 106 retired and 10,088 active
aliases; the readable-label counts and checksums above remain historical pins.
