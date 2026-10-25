# Reviewed ingredient aliases and scope

The alias overlay follows FoodOn, Wikidata and USDA ingestion. It screens all
**12,802 active entries**, retains 60 retired entries unchanged, and adds **7,009
aliases to 4,823 entries**. Scope review then retires 21 unsuitable entries,
leaving **12,781 active and 81 retired identities**, with 7,137 active aliases
and two aliases retained on newly retired entries. UUIDs, labels, original
aliases/order, definitions, source mappings, provenance and the source manifest
are preserved; only the 21 explicitly approved statuses change. It does not create ingredients, merge identities,
infer substitutes, ingest nutrients or change the runtime search algorithm.

These are the immutable alias-stage counts. The later [gap curation](GAPS.md)
adds reviewed cooking materials and appended names without changing this policy
or its historical input/output pins.

## Review scope

The maintained `data/vocabulary/adapters/ingredient-alias-policy.json` records
approved count-noun inflections, regional spellings, accent/hyphen alternatives,
qualified/inverted word order, 227 explicit ingredient-name decisions, ambiguity
holds and suggestion representatives. The complete pre-alias seed and source
checksums, base commit and approved output checksum bind the review. The Python
builder refuses stale inputs or output that differs from the approved result.
Scope retirements additionally require the original label, an exact source
mapping already attached to the UUID, a category and a specific rationale.

Every active entry receives a generated review row, including entries with no
safe addition. This is comprehensive lexical screening plus explicit culinary
exceptions, **not individual semantic certification of every imported concept**
or a claim to enumerate every possible synonym. FoodOn's 1,678 exact-synonym
proposals for active exact-mapped terms were also screened for useful alternatives
and traps. Source synonym scopes are evidence, not automatic alias approval;
scientific names, broader/narrower forms, unclear regional names and malformed
text are not bulk imported. Generated source evidence stays outside Git.

Examples include `eggs`, `plain flour`, `ground cumin`, `creme fraiche`,
`canned tomatoes`, `bay leaves`, `garbanzo beans`, `borlotti beans`, `phyllo`,
`linseed`, `molokhia`, `panang curry paste` and regional chili spellings.
Count nouns use an explicit lexicon rather than blind stemming: rice, molasses,
bass, milk and cream are not singularized/pluralized. All food-form qualifiers
remain present during generated word-order changes. `walnut (shell off)` can
gain `shelled walnut`; `lamb chop (with bone)` can gain `bone-in lamb chop`.

**216 proposals stay held**: 135 collide with existing names, 80 have multiple
candidate identities without a representative for that target, and one is
explicitly ambiguous. Counts are proposal/target pairs, not unique alias strings.
Existing names are reserved. A separately reviewed representative can receive a
new shared cooking-material name without changing any UUID or redirecting old
references. Neither source order nor fuzzy similarity chooses ownership.

Do not infer `double cream` = `heavy cream`, `pimento` = `allspice`, raw = cooked,
leaf = seed, generic flour = corn starch, or meatless stock base = chicken stock.
`chili (canned)` is a prepared dish: a recorded per-entry exception prevents
pepper plurals such as `canned chilies`. Names such as pepper, corn flour,
coriander, lime leaves and king prawns need context or more precise identities.
Preparation instructions and quality claims are not stripped to inflate lookup
success. Missing ingredients are a separate curation task.

## Reviewed label retirements

Screening flags 200 active labels for source/definition review. Keywords never
automatically retire an entry. The policy records these 21 explicit retirements:

| Reason | Retired labels |
|---|---|
| Animal feed | salt block for livestock; calf milk replacer |
| Manufacturing or unclear technical identity | starch molding; flour dough moulding; oil grill; edible coating; nut coating |
| Clinical/supplement formulation | nutritive preparation for clinical use; nutritional preparation |
| Undefined or catch-all category | natural extractive; coffee ingredients; cocoa ingredients; gravy ingredients; loaf (nonspecific); various pastry |
| Finished meal | eggs benedict; breakfast (frozen); breakfast, ready; pizza (sausage with cheese, mix); pizza with meat topping |
| Undefined breakfast formula | breakfast beverage (dry, instant) |

Soup mixes, canned mushroom soup, breakfast cereals/sausages, salad dressings,
pizza dough/sauce/toppings, mold-ripened cheeses and edible blood are retained.
Bee pollen remains despite an animal-food ancestry path: its human ingredient
use was explicitly reviewed. Unfamiliar or regional names, missing definitions,
and preparation words alone are not exclusion grounds. Undefined technical
labels are retired pending clarification, never guessed into a new ingredient.

All 12,862 UUIDs and their complete definitions remain in the seed/database.
Retirement hides the label and every alias from suggestions without rewriting
authored recipes or drafts. Existing retired identities are unchanged. Audit
rows record scope flags, action and rationale alongside alias outcomes. A
future return to active use requires a separately reviewed policy change.

## Reproduction

Requires Python 3.11+ and the standard library. The upstream adapters reproduce
the immutable pre-alias seed at `c60162b0ad827963be7541ec12f21fd29764781d`.
Apply this overlay **after** that stage, then apply gap curation for the maintained
complete bundle:

```bash
mkdir -p /tmp/noms-alias-base
git show c60162b0ad827963be7541ec12f21fd29764781d:data/vocabulary/ingredients.json > /tmp/noms-alias-base/ingredients.json
git show c60162b0ad827963be7541ec12f21fd29764781d:data/vocabulary/sources.json > /tmp/noms-alias-base/sources.json
python3 scripts/vocabulary/review_aliases.py review \
  --seed /tmp/noms-alias-base/ingredients.json \
  --sources /tmp/noms-alias-base/sources.json \
  --output /tmp/noms-alias-review
python3 scripts/vocabulary/review_aliases.py build \
  --seed /tmp/noms-alias-base/ingredients.json \
  --sources /tmp/noms-alias-base/sources.json \
  --output /tmp/noms-alias-seed
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/noms-alias-seed/ingredients.json \
  --sources /tmp/noms-alias-seed/sources.json \
  --output /tmp/noms-gap-seed
python3 scripts/vocabulary/fill_gaps.py \
  --policy data/vocabulary/adapters/ingredient-gap-followup.json \
  --seed /tmp/noms-gap-seed/ingredients.json \
  --sources /tmp/noms-gap-seed/sources.json \
  --output /tmp/noms-gap-followup
cmp /tmp/noms-gap-followup/ingredients.json data/vocabulary/ingredients.json
cmp /tmp/noms-gap-followup/sources.json data/vocabulary/sources.json
just seed-vocabulary --validate-only \
  --file /tmp/noms-gap-followup/ingredients.json \
  --sources /tmp/noms-gap-followup/sources.json
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py' -v
```

Use fresh external output directories. Outputs include every entry and every
alias proposal with disposition/rationale, CSVs, summary and canonical files.
Only the policy, builder/tests, guide and final seed are maintained; generated
audits and upstream catalogs stay outside Git. Original source ingestion ledgers
continue to reproduce their own stage; they are not rewritten to conceal the
later alias edits. After future ingestion, rebase the alias review explicitly.

For editorial changes, run `review` to inspect candidates, conflicts and the
output checksum. Update approved policy inputs and the output pin only after
review; `build` requires `approved: true` and the pinned result. Promote the
validated complete bundle with the existing transactional loader/dry-run workflow
in [the seed guide](../../data/vocabulary/README.md). Custom text and historical
recipes retain their existing behavior. No automatic recognition/backfill runs.
