# USDA → Noms seed adapter

This offline adapter reads checksum-pinned USDA FoodData Central CSV ZIPs and
builds complete canonical ingredient/source files from an approved UUID ledger.
It imports food names and provenance, not nutrient measurements. No API key,
live USDA request, database write or automatic recipe recognition runs during
extraction or builds.

## Reviewed releases

The descriptor pins Foundation Foods **2026-04-30** (469 food records) and
SR Legacy **2018-04** (7,793 records): **8,262 eligible records** in total.
The Foundation archive also contains 87,521 sample/acquisition/subsample rows;
those never become candidates. Archive byte lengths and SHA-256, plus the
names/lengths/CRCs of the three consumed tables, are verified before parsing.
Original FDC ID, data type, food category, publication date, optional NDB number
and description are retained. Foundation metadata omits some food records, so a
missing NDB number is valid; the FDC ID remains authoritative.

The all-name screening produces:

- **104 new ingredients**, each with explicit reviewed wording and a persisted
  UUIDv4, including almond flour, 00 flour, cremini and lion's mane mushrooms,
  hulled hemp seed, water spinach, vital wheat gluten and barbecue sauce.
- **1,142 related source associations**, including 1,140 to 706 maintained
  ingredient identities and two to new name families. They do not assert exact
  equivalence or link nutrients to recipes.
- **7,016 held records**, including commercial products, complete meals/soups,
  desserts, infant/medical formulations, industrial materials and 4,319 complex
  or unmatched descriptions needing a specific editorial definition.

Every eligible record receives a status and rationale, and all 12,698 active
baseline names/aliases appear in the name audit. This is complete mechanical
screening with explicit editorial exceptions, not individual semantic
certification of all source records. Held names are not source-absence claims.
FNDDS and the separate Branded release are deferred. SR Legacy itself contains
named products and mixed dishes, so a generic data-type name does not grant
ingredient eligibility.

The combined seed has **12,802 active and 60 retired entries**, four pinned
sources and 1,246 USDA mappings (169 Foundation, 1,077 SR Legacy). All 12,758
prior UUIDs, authored fields, origins, references and mappings remain intact.

## Identity and wording rules

Names suggest related associations through scoped punctuation, accent, plural
and word-order normalization. Reviewed spelling/synonym exceptions handle
portabella/portobello, catsup/ketchup and ground pork/pork mince. Food parts,
species, dried/canned/frozen forms and salted/unsalted distinctions are retained
in the evidence. Stronger lexical matches take precedence over simplifications.

Some analytical, production and preparation qualifiers can be omitted **only
to propose related evidence for a maintained ingredient**. For example, a cooked
or graded USDA record can be related to a generic maintained material; this
does not establish interchangeability, nutrient equivalence or a new aggregate
identity. Such omissions never supply wording for an automatically approved
import. New identities require explicit policy decisions. Unknown and complex
names remain held. Existing duplicate UUIDs are retained; one representative can
receive related evidence. No hard merges or redirects occur.

The white-button mushroom addition uses its specific culinary name and does not
adopt the conflicting FoodOn white-lepiota alias. Plant parts and ambiguous
beech-mushroom scope are not collapsed. Source aliases are not bulk copied.
USDA descriptions are public-domain structured data; the descriptor cites the
[USDA data guide](https://fdc.nal.usda.gov/api-guide/). Nutrient and portion tables
are not read. Canonical references record source provenance only.

## Reproduction

Requires Python 3.11+ and its standard library. Keep raw ZIPs and generated audits
outside Git, and use fresh output directories. The maintained inputs are
`usda.json`, `usda-cooking-policy.json` and `usda-cooking-review.json` under
`data/vocabulary/adapters/`. The ledger binds approved records to the archive
descriptor, baseline seed/source hashes and individual row-evidence hashes.

Recover the immutable pre-USDA baseline and obtain the two pinned downloads:

```bash
mkdir -p /tmp/noms-usda-base /tmp/noms-usda-archives
git show ab7ab53c32d48ff89f8fbd90833008f0ba292a73:data/vocabulary/ingredients.json > /tmp/noms-usda-base/ingredients.json
git show ab7ab53c32d48ff89f8fbd90833008f0ba292a73:data/vocabulary/sources.json > /tmp/noms-usda-base/sources.json
curl --fail --location https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_foundation_food_csv_2026-04-30.zip --output /tmp/noms-usda-archives/foundation-2026-04-30.zip
curl --fail --location https://fdc.nal.usda.gov/fdc-datasets/FoodData_Central_sr_legacy_food_csv_2018-04.zip --output /tmp/noms-usda-archives/sr-legacy-2018-04.zip
python3 scripts/vocabulary/usda.py extract \
  --archives /tmp/noms-usda-archives \
  --seed /tmp/noms-usda-base/ingredients.json \
  --sources /tmp/noms-usda-base/sources.json \
  --output /tmp/noms-usda-candidates
python3 scripts/vocabulary/review_usda.py \
  --archives /tmp/noms-usda-archives \
  --seed /tmp/noms-usda-base/ingredients.json \
  --sources /tmp/noms-usda-base/sources.json \
  --output /tmp/noms-usda-review
cmp /tmp/noms-usda-review/review.json data/vocabulary/adapters/usda-cooking-review.json
python3 scripts/vocabulary/usda.py build \
  --archives /tmp/noms-usda-archives \
  --seed /tmp/noms-usda-base/ingredients.json \
  --sources /tmp/noms-usda-base/sources.json \
  --review data/vocabulary/adapters/usda-cooking-review.json \
  --output /tmp/noms-usda-seed
cmp /tmp/noms-usda-seed/ingredients.json data/vocabulary/ingredients.json
cmp /tmp/noms-usda-seed/sources.json data/vocabulary/sources.json
just seed-vocabulary --validate-only \
  --file /tmp/noms-usda-seed/ingredients.json \
  --sources /tmp/noms-usda-seed/sources.json
```

Review emits `source-names.csv` and its JSON evidence for all 8,262 records,
`seed-names.csv` for every active baseline identity, and a summary with the
policy checksum. The saved ledger reuses import UUIDs; dropping a previously
approved import fails. Review and canonical output reproduce byte-for-byte.
Builds reject stale baselines/descriptors/row evidence, source metadata changes,
missing approval booleans, duplicate decisions, invalid IDs/text, retired map
targets, name collisions and conflicting source relationships. Maps cannot
overwrite authored wording. ZIP members are streamed without extraction.

Run offline regressions with:

```bash
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py' -v
```

Database loading remains the existing deployment-role operation described in the
[seed guide](../../data/vocabulary/README.md). Pause recipe writes for a rollout,
inspect the loader dry-run and preserve the complete seed. UI suggestions use
the same canonical database path as the other sources, with no live USDA call.

## Verification and local deployment

All **61 Python tests** and **six Rust vocabulary tests** pass. The authoritative
Rust validator accepts 12,862 entries/four sources with no warnings. Policy
review, persisted UUIDs and canonical output reproduce byte-for-byte. The
original 100-ingredient coverage benchmark remains unchanged.

The disposable local stack previously held the FoodOn-only baseline. The combined
Wikidata/USDA load added 2,654 entries and enriched 1,476 existing definitions;
its dry-run rolled back without changing entries, source metadata or seed-run
history. Normal loading preserved existing authored recipes/drafts. Every loaded
definition matches the canonical seed, and repeat loading changes no definitions,
timestamps or seed-run records. No production rollout occurred.

Headless Chromium verified hydration, label-only options, alias/prefix matching,
keyboard/mouse selection, UUID autosave/reload, reference clearing on edits and
free-text saving when lookup fails. **109 authenticated lookup checks** include
all 104 USDA additions and legacy/distinction probes. WASM delivery succeeded
without page errors. Full CSVs, deployment results and browser evidence remain
outside Git; the review scripts regenerate the maintained evidence.
