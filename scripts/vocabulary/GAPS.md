# Reviewed cooking-material gaps

This curation layer follows the immutable FoodOn, Wikidata, USDA and alias/scope
stages. It adds **258 curated ingredients**, **246 aliases on those additions**
and **25 aliases on 15 existing entries**. The complete seed contains **13,039
active and 81 retired entries**, with **7,408 active aliases**. All 12,862 prior
UUIDs, original wording/alias order, definitions, statuses, source associations
and provenance are preserved; only the explicit alias suffixes are appended.
The four-source manifest is unchanged.

## Identity decisions

The [identity policy](../../data/vocabulary/README.md#ingredient-identity-for-gap-curation)
uses meaningful recipe effects of substitution to distinguish materials. Stock
and broth remain separate, including animal/vegetable base, concentration,
low-sodium versus reduced-sodium, and fat-free versus fat-skimmed formulations.
Single, double, heavy, light-whipping and unspecified whipping cream retain
separate identities. Regional fat ranges are evidence, not universally identical
product standards. The existing heavy/light cream entries are preserved.

Other additions cover named rice types, plain pasta/noodle shapes, corn versus
wheat tortillas, meaningful raw/cooked or preservation states, meat cuts, produce
types, dairy formulations, sauces, spices and useful prepared cooking components.
No new row is generated merely by removing a modifier or expanding all possible
combinations. Chopping, melting, grating, size and pitting stay in authored text.

Definition review reuses existing identities: `spaghetti macaroni` explicitly has
FoodOn synonym `spaghetti`; `date honey` is date syrup; the existing `low-fat milk`
definition specifies 1% milk fat. Shell pasta is already `conchiglie`, and the
first/second molasses entries cover light/dark molasses. New aliases also make
`grainy mustard`, `Nanfia` and calamansi available under their verified cooking
names. Unqualified sweet soy sauce remains distinct from specifically Javanese
`kecap manis`; rice for sushi, cooked short-grain rice and vinegar-seasoned sushi
rice are separate. Recipe context is required where bare names span these forms.

## Review ledger and evidence

`data/vocabulary/adapters/ingredient-gap-review.json` contains:

- The immutable alias-stage commit/checksums and approved output checksum.
- Complete canonical records with UUIDs minted once and retained in the ledger.
- Explicit appended aliases for existing entries, bound to their IDs and labels.
- A decision for all **314 missing/partial names** from the frozen 1,000-profile
  review: 279 use new identities, 12 reuse existing identities, three require
  context, one is a probable source error and 19 remain held. These are name
  counts, not counts of new ingredients. Additional polenta and rice forms support
  ambiguous source wording without forcing that wording to one form.
- Per-addition rationale, observed names and related-definition evidence.
- Related associations for 21 pinned FoodOn/Wikidata records, with revisions or
  source checksums and scope notes. None asserts new exact source or nutrient
  equivalence. All additions have curated origin, including those with source
  evidence; they are not an unreviewed adapter import.
- Primary/industry reference URLs and capture hashes for stock/broth, regional
  cream definitions, sushi preparation and selected specialist components.
- Explicit corpus-scoped row contexts. These are availability evidence, not
  automatic assignment of a recipe's optional vocabulary reference.

Held cases include unclear essence formulations, ground red pepper, pectin form,
light raisins, regional olive/sauce/sausage terms, a commercial pepper product,
and the botanical identity/processing of black moss and lily buds. Bare polenta
can mean dry meal or cooked polenta; both are represented while the source stays
unresolved. Prepared frosting-word-order errors do not become aliases.

Full provider catalogs, source HTML, copyrighted recipe instructions, per-line
audit/search matrices and browser artifacts remain outside Git. The maintained
ledger contains review facts and authored cooking descriptions. Dataset pages
and corpus/review hashes pin the research inputs without committing the full
research corpus or inferring its license covers recipe prose.

## Reproduction

Python 3.11+ and the standard library suffice. Use fresh external output paths.
The earlier adapters and alias builder retain their original stage checksums;
they are not edited to absorb this later curation layer.

```bash
mkdir -p /tmp/noms-gap-base
git show aa53490bedf674643a75ea9a8c71b57dff6da759:data/vocabulary/ingredients.json > /tmp/noms-gap-base/ingredients.json
git show aa53490bedf674643a75ea9a8c71b57dff6da759:data/vocabulary/sources.json > /tmp/noms-gap-base/sources.json
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/noms-gap-base/ingredients.json \
  --sources /tmp/noms-gap-base/sources.json \
  --output /tmp/noms-gap-seed
cmp /tmp/noms-gap-seed/ingredients.json data/vocabulary/ingredients.json
cmp /tmp/noms-gap-seed/sources.json data/vocabulary/sources.json
just seed-vocabulary --validate-only \
  --file /tmp/noms-gap-seed/ingredients.json \
  --sources /tmp/noms-gap-seed/sources.json
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py' -v
```

Alternatively, pass the alias builder's exact output as `--seed` and `--sources`
for complete pipeline replay. The builder rejects stale input, repeated/new-ID
collisions, shadowing an existing label/alias, retired alias targets, incomplete
decisions, missing identity evidence and output that differs from the approved
checksum. It never generates IDs, contacts a provider, loads a database, changes
recipe text or supplies runtime substitution rules. Later upstream changes need
an explicit review rebase; do not run the alias expansion over the final seed.

Promote the validated complete bundle using the existing transactional seed
loader, with dry-run rollback and repeat-load checks. A later editorial correction
must retain published UUIDs and use forward changes or retirement rather than
removing an identity from the complete seed.

## Verification

The 82 offline Python tests check full bundle reproduction, prior-definition
preservation, alias-stage restoration, source contracts and the new identity
boundaries/ambiguity holds. The authoritative Rust validator accepts all 13,120
entries/four sources with zero warnings. Generated reports remain external.

The frozen 1,000-profile sample contains 10,768 ingredient lines/1,769 names,
split equally between TheMealDB and Yummly/Kaggle. Material availability rises
from **9,706 to 10,339 covered lines (90.1% to 96.0%)**, including verified reuse
and scoped context review, not solely new entries. Fully covered profiles rise
from **369 to 663**. This is a purposive breadth sample, not a population estimate
or an automatic recipe parser benchmark. Authored ingredients stay unchanged.
