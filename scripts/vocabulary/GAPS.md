# Reviewed cooking-material gaps

The first curation layer follows the immutable FoodOn, Wikidata, USDA and alias/scope
stages. It adds **258 curated ingredients**, **246 aliases on those additions**
and **25 aliases on 15 existing entries**. Its output contains **13,039
active and 81 retired entries**, with **7,408 active aliases**. All 12,862 prior
UUIDs, original wording/alias order, definitions, statuses, source associations
and provenance are preserved; only the explicit alias suffixes are appended.
The four-source manifest is unchanged.

The subsequent [October 25 follow-up](#source-follow-up-2026-10-25) adds 16 more
materials and 114 names. After the follow-up, totals are **13,055 active, 81 retired and 7,522
active aliases**. Both curation stages retain their own immutable input/output
pins and explicit review ledgers. See [preferred-label review](LABELS.md) for
the later naming layer.

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

## First-stage verification

At the October 24 stage, 82 offline Python tests checked bundle reproduction,
prior-definition preservation, alias-stage restoration, source contracts and
identity boundaries/ambiguity holds. Rust validation accepted that stage's 13,120
entries/four sources with zero warnings. Generated reports remain external.

The frozen 1,000-profile sample contains 10,768 ingredient lines/1,769 names,
split equally between TheMealDB and Yummly/Kaggle. Material availability rises
from **9,706 to 10,339 covered lines (90.1% to 96.0%)**, including verified reuse
and scoped context review, not solely new entries. Fully covered profiles rise
from **369 to 663**. This is a purposive breadth sample, not a population estimate
or an automatic recipe parser benchmark. Authored ingredients stay unchanged.

## Source follow-up (2026-10-25)

`ingredient-gap-followup.json` applies after commit
`f1ed36568801bfe1b77af10ffcc163246031954b`, whose seed checksum is
`bb4171b4892450ac5e61103f4cc61219b8c9e432fbd16388d35b95a0b2717e46`.
It uses the same offline builder with an explicit `--policy`; the original
`ingredient-gap-review.json` is unchanged. The new output checksum is
`373d0b7b4c19f6c8e664655d1ebafedbae97e8028503cca63733c62e9609dd80`.

The follow-up reviews all 19 prior holds plus selected specific-form and naming
gaps: **53 names**, with eight new-identity uses, five source-context decisions,
31 existing-identity uses and nine remaining holds. It adds **16 identities**,
**55 aliases on them**, and **59 aliases appended to existing entries**. Explicit
alternatives such as dried daylily buds, powdered pectin and imitation vanilla
flavouring are represented without forcing an ambiguous source name into them.

Government, university, original-recipe and maker references identify fat choy as
Nostoc, daylily buds as a separate culinary plant part, and the recipe's dried
sorrel/jam as roselle. Original sources identify Colombian aliños and Lao som moo.
Descriptions are authored review facts; raw pages and recipe instructions stay
outside Git. URLs, capture hashes and source limits are recorded in the ledger.
Provider country labels do not establish the identity: the sorrel recipe's
Dominica Gourmet source resolves its meaning despite different provider metadata.

Whole coriander seed and its ground form retain separate IDs. Generic pectin is
explicitly unspecified; powdered and existing liquid pectin remain separate.
Golden raisins do not imply seedlessness, olive-oil-packed tuna does not imply
solid-pack canned presentation, and red vermouth does not specify a brand or
precise sweetness. Unspecified information remains unspecified; a shared generic
reference is not a dosage or substitution rule. Pul biber remains separate from
specifically named Urfa and Aleppo products.

Qualified `pepper (black)` and `red pepper (bell)` names offer explicit choices
for broad prefix searches; bare pepper/red pepper are not exact aliases.
This improves candidate recall without resolving an ambiguous recipe automatically.
Common names include Dijon mustard, beansprouts, purple onion, clove powder,
apple cider vinegar and qualifier-preserving chicken-cut word order. Reserved
names keep their existing owners: `swedes` and `apple vinegar` are not shadowed
or merged. Bare sorrel, lily buds, vanilla essence, ground red pepper and brands
do not become unconditional aliases of a more specific material. Qualified
aliases can offer explicit choices, such as `dried sorrel (Caribbean roselle)`.

All **88 Python tests** pass. Rust validation accepts **13,136 entries/four
sources with zero warnings**. Test deployment checks dry-run rollback, exact
definition equality, no-op reload and unchanged existing recipes/drafts/versions.
Chromium verifies ground-coriander selection, UUID autosave/reload, ID clearing
and free-text fallback. All **130 new label/alias lookups** and nine additional
qualified-prefix, ambiguity and wildcard-escaping checks pass through the live API.

In the unchanged 1,000-profile sample, covered lines increase from **10,339 to
10,349 of 10,768 (96.1%)**, with **667 fully covered profiles**. The two previously
missing botanical names are represented; nine source names remain held.
Full-wording target hits rise from **8,978 to 9,359**, and full-or-three/four-character
prefix hits from **9,174 to 9,531**, under the updated review. Whole/ground
coriander and generic golden-raisin targets are refined to the new identities.
On the frozen previous 10,339 covered lines with their original expected IDs,
full-wording hits rise to 9,319 and full-or-prefix hits to 9,496, with zero
regressions. Availability and lookup metrics remain separate from parsing,
automatic assignment and compatibility. No details field or runtime matching
change is introduced.


## Second held-out 1,000-profile study (2026-10-25)

A separate sample selects **200 additional TheMealDB recipes and 800 additional
Yummly/Kaggle ingredient profiles** (40 per 20 cuisine labels) from the pinned
captures. It excludes all first-sample IDs and exact normalized ingredient sets,
without selecting on vocabulary outcomes. There are **10,891 lines, 1,850 names
and 732 names new to the first sample**. Yummly records have no titles, measures,
instructions or individual recipe URLs; those fields remain blank.

Against preferred-label output
`f6f5d4021ac18e7a4897752aa4c31aa9d2957ebf17608d4c6e5353ca0d280e4a`,
**10,185/10,891 lines (93.5%)** have represented materials and **516 profiles**
have every line represented. **150 candidate gap phrases** affect **182 lines
across 154 profiles**: 27 missing named-material phrases and 123 specific forms,
formulations or products with only related current definitions. These are not
150 proposed UUIDs; synonym, scope and definition review must precede additions.
All 150 phrases are new to the first sample's wording, rather than proof that
all corresponding concepts are new. Another 430 ambiguous, 51 malformed,
37 branded and six component lines are tracked separately.

Useful candidate areas include farfalle/bowtie, cavatappi, ditalini and ziti;
fresh mozzarella, low-fat/nonfat Greek yogurt and reduced-fat cream cheese;
low/reduced/no-added-salt stock and broth types; dried fruits, roasted nuts and
frozen produce; MSG, muesli, baked brioche and named seasoning/curry pastes.
Cross-checking removes false gaps: chicken bones already exist, FoodOn lists
rotini as an exact fusilli synonym, and fresh goat cheese is represented by
chevre frais. These remain alias/discoverability leads. Captured obsolete MSG
and taxon-only records must not be imported as current edible-material evidence.

The production SQL query is replayed read-only for **3,091 query forms**; full
wording retrieves expected candidates for **8,790 covered lines**, or **9,019**
with three/four-character prefixes. A bounded **100 live lookups plus two
wildcard guards** agree with the capture. Selection and analysis reproduce
byte-for-byte; the seed and all four source descriptors remain unchanged.
No recipe or draft is created, imported, published or rewritten by the study.

Reports, every name-level decision, source leads and CSVs are external at
`/workspace/recipe-vocabulary-second-1000`; the download is
`/workspace/noms-second-1000-recipe-gap-review.zip`. The archive includes standard
library offline verification and prior-sample fingerprints. Full catalogs and
raw recipe instructions remain excluded. Its 200/800 provider mix differs from
the first sample's 500/500 mix: these are breadth studies, not population estimates
or a before/after vocabulary score. Earlier studies and archives stay frozen.
