# Reviewed cooking-material gaps

Current maintained totals after the [grocery follow-up](#grocery-material-follow-up-2026-10-25)
are **13,232 active, 81 retired and 8,989 active aliases**. Earlier stage counts,
source studies and checksums below remain frozen.

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


## Second-sample curation (2026-10-25)

The [explicit ledger](../../data/vocabulary/adapters/ingredient-second-sample-gaps.json)
reviews all 150 candidate phrases from the held-out study plus nine existing-name
discoveries. It adds **148 curated definitions**, including qualified alternatives
for unresolved wording, and **298 aliases**: 273 on new entries and 25 appended to
13 existing identities. These counts are not a one-to-one conversion of recipe
phrases into ingredients. Totals at that stage are **13,203 active, 81 retired and
8,847 active aliases**. All four source descriptors and every preexisting field
remain unchanged except the approved alias suffixes.

The additions cover MSG; farfalle/bowtie, cavatappi/cellentani, ditalini and ziti;
fresh mozzarella, low-fat/nonfat Greek-yogurt forms, reduced-fat cream cheese;
stock/broth, sodium and concentration variants; dried fruits, frozen produce,
roasted nuts; baking products, seasonings and named culinary plant/material types.
Plain/flavored, fresh/low-moisture, whole/ground, raw/cooked, stock/broth,
low/reduced/no-added-salt and meaningful edible-part differences retain separate
IDs. Unknown species, flavor, salt, sweetness, processing or brand stays unknown.

Definition review reuses published IDs for ancho powder, low-fat mayonnaise whose
definition explicitly includes light mayonnaise, fusilli/rotini, chicken bone,
TVP, chevre frais, ramson, seltzer water, gelatine, heavy cream and whole-grain
bread, chocolate liquor/unsweetened chocolate and sopressata salami/soppressata. No old identities are merged, retired or relabeled.

Five phrases remain held: **apricot brandy**, **Filipino eggplant**, **thin pizza
crust**, **33% less sodium ham** and **40% less sodium taco seasoning mix**.
Explicit apricot liqueur/eau-de-vie, raw/prebaked thin bases and generic
reduced-sodium product alternatives are provided without claiming to resolve the
bare phrase, exact comparison product or unknown regional variety.

Original-author evidence resolves the stroopwafel's schenkstroop and the Jamaican
bun's browning sauce only for their recipe rows; neither ambiguous phrase becomes
a global alias. The pinned congee instructions say ginger juice, while its
ingredient field says ginger cordial. That mismatch remains a source correction;
the linked author page could not be independently retrieved in this run. Real
sweetened ginger cordial and plain ginger juice have separate definitions.
Malformed `gelatine leafs` and `color food green` remain correction leads, with
properly named gelatin sheets and green food coloring available.

Current FDA evidence supports the new curated MSG material. The obsolete FoodOn
MSG identifier is never imported or reactivated. Captured FoodOn/USDA/Wikidata
records and manufacturer examples support definition/form review, not exact
source mappings, brand aliases, fixed nutrient values or substitution guarantees.
Definitions use original curated wording; raw reference pages/catalogs stay
outside Git. Evidence URLs and SHA-256 hashes are recorded in the ledger.

### Replay

Input is the unchanged preferred-label output at
`b1cedbea2f0e8d25973096094d16718615a8060a`:

- Input seed: `f6f5d4021ac18e7a4897752aa4c31aa9d2957ebf17608d4c6e5353ca0d280e4a`
- Output seed: `6e703364b9ab3b7476a0637b7d1063dcdf0096dfb26d6b7a42b740b39acfa821`
- Source manifest: `1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`

Obtain the complete input seed/source pair from that commit into an external
folder, then run:

```sh
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/second-gap-base/ingredients.json \
  --sources /tmp/second-gap-base/sources.json \
  --policy data/vocabulary/adapters/ingredient-second-sample-gaps.json \
  --output /tmp/second-gap-review
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
```

The full bundle reproduces byte-for-byte. Prior source, gap and label tests
verify/remove each later stage at its own immutable pin before checking earlier
invariants. New final-seed tests check existing fields/aliases, material
separation, qualified alternatives, scoped corrections, MSG evidence and name
ownership. All **101 Python tests** pass. The Rust validator accepts **13,284
entries/four sources with zero warnings**.

### Deployment and recipe checks

Local test deployment passes transactional dry-run rollback, final definition
comparison and zero-change repeat loading. All preexisting draft, recipe and
immutable-version hashes remain unchanged. All **446 new preferred/alias names**
return their intended UUID first through the production SQL query. Chromium
passes selection/autosave/reload, clearing the ID on edit, free-text fallback
and WASM delivery. All **181 authenticated lookup checks** pass against the
final canonical bundle. One private synthetic draft
was created for the UI probe; no recipe was imported or published.

In the held-out sample, material availability rises **10,185 → 10,359 of 10,891
lines (93.5% → 95.1%)**, with fully covered profiles rising **516 → 616**. There
are no remaining confirmed missing/partial names from that candidate queue;
the five ambiguous product/source phrases and three correction cases remain
explicitly tracked rather than counted as resolved ingredients. Other source
ambiguities, brands and components remain outside full coverage.

Full-wording expected candidate hits rise **8,790 → 8,973**; full-or-short-prefix
hits rise **9,019 → 9,199**, under the updated specific-material review. On the
frozen previous 10,185 covered lines and their original expected UUIDs, full
hits rise to **8,801** and full-or-prefix hits to **9,025**, with zero regressions.
The first 1,000-profile sample retains **10,349 covered lines/667 fully covered
profiles**, **9,370 full** and **9,520 full-or-prefix** candidate hits, also with
zero regressions. These are editorial material-availability and retrieval
measures, not automatic recipe recognition or population estimates. Ranking,
parser behavior and the optional Details-field design remain unchanged.

The full vocabulary JSON/CSV, new-entry list, decision/held audits, source pins
and both replayable recipe after-checks are external at
`/workspace/vocabulary-second-gap-fill`; download
`/workspace/noms-vocabulary-second-gap-fill.zip`. Earlier studies/downloads stay
frozen. Raw instructions, catalogs, reference HTML, browser sessions and private
draft data are excluded from the download. No production rollout is included.


## Grocery-material follow-up (2026-10-25)

The user requested ordinary grocery-store ingredients, including useful specific
forms even when a sampled recipe omits the qualifying information. This layer
adds **29 definitions and 142 aliases** (94 on new entries, 48 on 17 existing
identities), resulting in **13,232 active, 81 retired and 8,989 active aliases**.
There are 451 subsequently curated definitions across the four gap rounds.
The explicit ledger is `data/vocabulary/adapters/ingredient-grocery-gaps.json`.

- Dried angel hair/capellini is searchable separately from fresh angel hair.
- Bouillon additions cover chicken/vegetable/mushroom pastes, vegetable and shrimp
  cubes, shrimp granules, tomato-chicken cubes/granules and reduced-sodium chicken
  granules. Existing beef cube/granule/paste and vegetable-powder identities get
  familiar names. Checking the pinned FoodOn annotations establishes that existing
  `chicken bouillon mix` is a finely ground dehydrated dry broth/stock mix, so
  chicken bouillon powder and granules reuse its published UUID. The broader
  vegetable-flavored cube does not assert a specific vegetable-protein base;
  the existing narrower vegetable-protein cube remains intact.
- Whole-grain corn flour, white masarepa and instant polenta are added. Existing
  ordinary white/yellow corn flour, yellow masarepa and masa harina get qualified
  names. Flour, extracted starch, nixtamalized flour and precooked flour remain
  separate. The existing dialect alias `cornflour` still belongs to cornstarch;
  legacy overlapping corn/maize UUIDs are not hard-merged.
- Calvados, grappa, pisco and slivovitz supplement existing Cognac, Armagnac,
  apple/pear brandy, kirsch and apricot eau-de-vie. Sweet apricot liqueur stays
  separate; bare `apricot brandy` and `cherry brandy` do not become exact aliases.
- Grocery components include undressed broccoli/coleslaw mixes, unsauced frozen
  pepper-and-onion and stir-fry mixtures, spring mix, instant espresso powder,
  dry harissa seasoning, salt-free seasoning, whole-grain Dijon, sweet/hot Italian
  sausage and graham crackers. Plain graham crumbs reuse the cracker identity
  under the existing physical-preparation policy. Named styles and consequential
  forms are distinguished; brand formulas, nutrient values and substitution
  guarantees are not inferred. Bare `chicken base` and `broccoli slaw` remain held
  while qualified paste and undressed-mix options are available.

Definitions use original wording and retained retail/manufacturer evidence.
Examples include [Barilla angel hair](https://www.barilla.com/en-us/products/pasta/classic-blue-box/angel-hair),
[Knorr bouillon](https://www.knorr.com/us/en/products/granulated-bouillon.html),
[GOYA white masarepa](https://www.goya.com/en/products/masarepa-pre-cooked-white-corn-meal/),
[Calvados France](https://drinkcalvados.com/en/faq/),
[Nonino grappa](https://www.grappanonino.it/en/grappa-en/optima/) and
[Taylor Farms broccoli slaw](https://www.taylorfarms.com/products/broccoli-slaw/).
Evidence URLs, capture kinds and hashes are in the ledger; raw pages and generated
reports remain external. Brand examples do not create brand aliases or exact
provider mappings. Existing IDs, descriptions, labels, statuses, provenance,
source associations and alias prefixes are preserved. Sources are byte-identical.

### Replay and validation

The baseline is commit `998a69d06e2be4e7ab130bbf3ab8a81a68d00715`:

- Input seed: `6e703364b9ab3b7476a0637b7d1063dcdf0096dfb26d6b7a42b740b39acfa821`
- Output seed: `8a6c4f4054700d301dd293daa1de72f3e599723f631fe09eb8be45bee64515ab`
- Unchanged sources: `1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`

Extract the baseline seed from that commit into an external directory, then run:

```sh
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/grocery-base/ingredients.json \
  --sources data/vocabulary/sources.json \
  --policy data/vocabulary/adapters/ingredient-grocery-gaps.json \
  --output /tmp/grocery-replay
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
```

Full seed replay is byte-identical; all **108 Python tests** pass. The authoritative
Rust validator accepts **13,313 entries/four sources with no warnings**. Earlier
checks verify/remove this layer before asserting their original immutable pins.
Local deployment passes dry-run rollback, exact DB/JSON definition equality and
zero-change repeat loading. All **171 new preferred/alias names** return their
intended UUID first through the production SQL query; **176 authenticated API
checks** pass. Chromium verifies angel-hair keyboard/mouse selection, canonical
UUID autosave/reload, ID clearing on edit, failure/free-text fallback and WASM
responses. Existing authored row hashes are unchanged; one new private synthetic
UI draft was created. No recipe import/publication or production rollout occurs.

The two frozen 1,000-profile studies remain unchanged. Replaying their previously
covered lines with the same expected IDs loses no full-wording candidates:
first sample **9,370 -> 9,370**, second **8,973 -> 8,976**. There is a recorded
short-prefix tradeoff: new spring-mix suggestions displace spring-roll pastry at
three/four letters for four first-sample wrapper rows and one second-sample skin
row. Full-or-short-prefix hits are **9,520 -> 9,516** and **9,199 -> 9,199**
(one gain and one loss), respectively. The existing wrapper is still returned
for `spring roll`. Manufacturer evidence also shows that unqualified spring-roll
wrappers can mean rice pancakes or wheat pastry, so no unconditional alias is
invented to conceal this result. Ranking and the eight-result limit are unchanged.
No new material-availability percentage or automatic-assignment score is claimed.

External evidence, lookup audits and the full canonical JSON/CSV are in
`/workspace/vocabulary-grocery-gap-fill`; the portable download is
`/workspace/noms-vocabulary-grocery-gaps.zip`. Earlier studies/downloads stay frozen.
