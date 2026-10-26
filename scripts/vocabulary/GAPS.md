# Reviewed cooking-material gaps

Current maintained totals after the [next-sample gap fill](#next-sample-gap-fill-and-grocery-expansion-2026-10-26)
are **13,463 active, 85 retired and 9,587 active aliases**. Earlier stage counts,
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


## Final random ingredient discovery (2026-10-26)

This is a discovery-only audit of **5,000 ingredient occurrences**, balanced as
**250 from each of 20 Yummly/Kaggle cuisine labels**. The sample contains **1,243
normalized names** from **4,375 contributing profiles**, including **292 names
not seen in either earlier study**. All 2,000 earlier recipe IDs and exact
normalized ingredient-set profiles are excluded before selection. The sampled
records have zero overlap with those IDs/profiles. Repeated staple occurrences
are retained; this is not 5,000 unique ingredients or 5,000 complete recipes.
Only selected occurrences are reviewed; full profiles provide source context.

Selection ranks the remaining ingredient occurrences with SHA-256 using fixed
seed `noms-final-ingredient-gap-5000-2026-10-26-v1`, taking 250 per cuisine after
complete-profile deduplication. It never consults vocabulary matches. The source
is the previously captured 39,774-profile [Yummly/Kaggle training set](https://www.kaggle.com/datasets/kaggle/recipe-ingredients-dataset),
not a fresh crawl. Archive SHA-256 is
`53d7ab877216c1dc02ab726087c890fa4ff36b75caab64c76d6ebfbc7e51b42d`;
`train.json` is `f18e7e568d1c606774bfb134ece51ef0a143cd5069e6f9241bb6036b45b7d2b0`.
The unchanged vocabulary is commit `70c01447aa46f35b90ea539a5b972ce7dd7d2b91`,
seed `8a6c4f4054700d301dd293daa1de72f3e599723f631fe09eb8be45bee64515ab`.

| Review outcome | Distinct names | Sampled occurrences |
| --- | ---: | ---: |
| Suitable material represented | 1,023 | 4,643 |
| Clear missing-material candidates | 56 | 58 |
| Qualified form/identity review | 20 | 25 |
| Ambiguous source | 117 | 227 |
| Brand/product specific | 16 | 17 |
| Malformed source | 7 | 23 |
| Composite/dish/component | 4 | 7 |
| Total | 1,243 | 5,000 |

The **76 discovery candidates account for 83 occurrences (1.66%)**. They are not
76 approved new UUIDs. All clear candidates occur once or twice in this sample.
Material availability is **4,643/5,000 (92.86%)**, an editorial availability
measure for this balanced source corpus. It is not automatic assignment accuracy,
live autocomplete performance, a whole-recipe completeness score or a global
population estimate. The selection differs from the earlier purposive studies,
so their percentages are not a before/after comparison. The provider's 20 labels
do not cover every popular cuisine; missing titles, measures, instructions and
individual recipe URLs are not invented.

Clear candidates include annatto oil, cavatelli, guanciale, seitan, branzino,
kelp noodles, yu choy, dried porcini, toasted nori, tarragon vinegar, char-siu
sauce, tikka-masala paste, crème anglaise, coconut/maple/peppermint baking
extracts, specific plain-yogurt and broth formulations, named produce varieties,
meat cuts/parts and smoked meat products. Existing broader ingredients or a
collection of separate components do not automatically establish a qualified
formulation. Partial cases include lo-mein/Korean noodle forms, fines herbes,
farofa, instant couscous, light cream cheese, frozen puff-pastry sheets, and tofu
puffs versus the existing broader fried-tofu definition. Check qualifiers and
existing definitions before approving additions or aliases.

### False gaps and definition review

Bacon drippings/grease are represented by cooked bacon fat; shungiku by garland
chrysanthemum leaves; crab meat by shelled crabmeat; crawfish by crayfish; green
grapes by white grapes; and pork butt by Boston butt. Culinary references and
original FoodOn sucrose/water annotations support rock sugar as existing rock
candy, so it is not another new ingredient. Ceylon cinnamon is represented by
the existing cinnamon-bark definition naming *Cinnamomum verum*.

Two misleading exact matches need separate cleanup, without inventing new IDs:

- `sweet peas` currently resolves by exact alias to an entry defined as
  *Lathyrus odoratus*. Culinary garden peas already exist as `green pea`.
- `Lamb chops` is defined as a grilled dish, while existing singular `lamb chop`
  defines the raw meat cut. A plural string hit must not select the dish ID for
  the cooking cut.

There are also **318 covered nonliteral names**, including physical preparation
wording. This does not mean 318 absent ingredients, mandatory global aliases or
proven UI failures; exact-name availability is a separate measure from retrieval.

[Ronzoni](https://ronzoni.com/product/rotelle/) explicitly sells rotelle as spiral
pasta. Other suppliers use the name for wheels, so the source phrase is retained
for qualified shape review rather than forcing a wheel identity. Likewise,
[sirloin tip](https://www.beefitswhatsfordinner.com/cuts/cut/2793/sirloin-tip)
comes from the round/knuckle in US retail naming, not the similarly named sirloin
primal. [Prepared seitan](https://uptonsnaturals.com/products/traditional-seitan/)
is distinct from the existing finely ground wheat-gluten source record.

### Artifacts and verification

Full occurrence/name audits, all 56 clear candidates, all 20 qualified-review
candidates, definition-quality flags, explicit decisions, source leads, hashes,
source contexts and standard-library replay tools are external at
`/workspace/ingredient-gap-final-5000`. Download
`/workspace/noms-final-5000-ingredient-gap-review.zip`.

The independent sampling replay reproduces every selected record from the pinned
full archive without reading vocabulary or review outcomes. Offline verification
checks all 5,000 rows, cuisine balance, exclusions, source-context positions,
all 1,243 decisions, active UUID references, CSV counts and file pins. The seed
and four-source manifest remain byte-identical; no recipes, aliases, canonical
entries or deployment state change. Raw reference pages, the full provider
archive and private browser/session data stay outside Git and the download.
Earlier reports and download bundles remain frozen.


## Final gap fill and pantry expansion (2026-10-26)

The user approved the discovery candidates and requested nearby grocery staples.
This layer adds **133 defined ingredients and 322 aliases**: 257 on new entries,
65 appended to 31 published identities. It records decisions for all **76 original
candidate phrases** and a **154-name pantry checklist**. Totals are **13,361 active,
85 retired and 9,307 active aliases**. All four source descriptors are unchanged.
The explicit ledger is `data/vocabulary/adapters/ingredient-final-gap-fill.json`.

- Fourteen new liquid baking-extract types include orange, peppermint, coconut,
  maple, anise, butter, banana, cherry, strawberry, mint, hazelnut, pandan, ube and
  caramel. Coffee extract reuses the existing `liquid coffee concentrate` UUID:
  its definition already explicitly identifies liquid concentrated coffee extract.
  Extracts are distinct from juice, syrup, fresh herbs and concentrated essential
  oils; carrier, exact concentration and natural/imitation formulation stay
  unspecified where the source does not establish them.
- Asian produce includes tatsoi, gai choy, pea shoots, flowering garlic chives,
  Thai eggplant, Korean radish, culinary lily bulbs, banana-stem pith and purple
  yam/ube. Fresh/dried lily bulbs, fresh/frozen ube, ube jam and ube extract stay
  separate. Green/red shiso types remain distinct from Korean perilla leaves.
- Mushroom additions include dried porcini, morels, chanterelles, oyster mushrooms,
  snow fungus, canned straw mushrooms, dried mixtures and qualified cultivated
  Pholiota chestnut mushrooms. Existing jelly-ear, horn-of-plenty and pioppini
  identities receive familiar wood-ear, black-trumpet and pioppino names.
- Vinegars include tarragon, unseasoned rice, red rice, brown rice, cane, coconut,
  date, ume pickling brine and a properly defined raspberry cooking vinegar.
  Ume vinegar is explicitly a salty pickling liquid. Rice wine, seasoned/plain
  vinegar and preserved tarragon herb are not equated.
- Cured/processed meats include guanciale, coppa/capocollo, saucisson sec, Spanish
  cured versus fresh Mexican chorizo, smoked pancetta, jowl bacon, lap cheong,
  lap yuk, duck-liver sausage, nduja and finocchiona, alongside reviewed cuts,
  bones and smoked/seasoned products from the sample.

The review removes false gaps through published-ID reuse. `yu choy` is existing
`choi sum`, `broccoli stems` are `broccoli stalk`, and `coffee extract` is already
explicitly defined by `liquid coffee concentrate`. Also reuse Malabar spinach,
Vietnamese coriander, banana blossom, garlic scapes, Chinkiang vinegar, prosciutto
crudo, salt pork and other qualified equivalents. Similar words or related source
mappings alone do not authorize an identity merge or nutrient linkage.

### Explicit label and scope corrections

Four entries change only from active to retired. Their UUIDs, labels, aliases,
definitions, source mappings and provenance remain byte-for-byte unchanged:

- `sweet pea` explicitly identifies *Lathyrus odoratus*. Its canned FoodOn child
  inherits that species class. Both are retired; ordinary culinary names reuse
  existing `green pea` and `canned green pea` IDs.
- Plural `Lamb chops` describes a grilled dish. It is retired; the plural spelling
  now belongs to the existing raw `lamb chop` definition.
- Existing `raspberry vinegar` describes a sweet drink. It is retired from default
  cooking suggestions; a distinct raspberry cooking-vinegar definition owns the
  culinary name. The drink ID is not silently converted or merged.

The generic builder now accepts optional, explicit status retirements with full
before snapshots and reasons. It rejects stale/unknown retirements, preserves
all other existing fields, and restores the exact prior seed at its own checksum.
Historical layers remain immutable. Neither a retirement nor an alias moves
an authored recipe reference; historical rendering and stored text are preserved.

Fifteen reviewed bare phrases retain qualified choices rather than unconditional
aliases. These include fresh/dried cavatelli, fines herbes, lo mein, Black Mission
figs and lavender flowers; condensed/ready chicken consommé; plain chili puree
versus seasoned paste; farofa products; Kashmiri chili forms; wheel/spiral rotelle;
and unspecified palm/black vinegar, lily, Iberico or liver-sausage types. All
candidate families have useful forms available; missing source qualifiers are
not invented. The original discovery report stays frozen.

### Replay and checks

Baseline commit: `1e001d304da8ba085b6b7b7404fac2aa92f2b38d`.
Input seed: `8a6c4f4054700d301dd293daa1de72f3e599723f631fe09eb8be45bee64515ab`.
Output seed: `0b1bda94a192383da92a95ed6f0834c65024a088a2a0bc4618d43147e0c59f45`.
Sources remain `1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`.

```sh
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/final-gap-base/ingredients.json \
  --sources data/vocabulary/sources.json \
  --policy data/vocabulary/adapters/ingredient-final-gap-fill.json \
  --output /tmp/final-gap-replay
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
```

All **117 Python tests** pass. The authoritative Rust validator accepts **13,446
entries/four sources without warnings**. The complete canonical seed replays
byte-for-byte, including restoration of every prior status/alias. Test deployment
passes transactional dry-run rollback, DB/JSON definition equality and a final
no-op load with all **13,446 unchanged**. All preexisting authored row hashes are
preserved; one private synthetic UI draft is added, with no import/publication.

All **455 new preferred/alias names** return their intended UUID first through
the production SQL query. **215 final authenticated API checks** pass. Chromium
verifies pandan-extract selection, UUID autosave/reload, clearing the ID on edit,
free-text fallback after lookup failure, WASM responses and twelve representative
lookup cases including the corrected pea/lamb labels and published-ID reuse.

On the same frozen expected IDs, all **25,351 previously covered occurrences**
across the two recipe studies and final occurrence sample retain full-wording
and full-or-short-prefix retrieval: zero losses in this round. First-study full
hits rise **9,370 -> 9,375**, second **8,976 -> 8,982**, final sample **3,902 ->
3,905**. Full-or-prefix hits rise **9,516 -> 9,521**, **9,199 -> 9,201** and
**4,020 -> 4,022**. Reviewed ordinary aliases for quick-cooking oats and seasoned
rice-wine vinegar preserve their existing IDs and avoid crowding them out.
Ranking and the eight-result limit are unchanged.

On an after-copy of the same 5,000 sampled occurrences, represented materials
rise **4,643 -> 4,714 (92.86% -> 94.28%)**. The 76-candidate missing/partial queue
is addressed by explicit definitions, existing-ID reuse or qualified alternatives;
unresolved forms remain source ambiguities. The 239 ambiguous, 17 branded, 23
malformed and seven composite occurrences are excluded from full coverage. This
is editorial material availability, not automatic recognition or a population
estimate. The sample itself and earlier reports/downloads are unchanged.

External evidence, complete canonical JSON/CSV, all new entries, original-case and
pantry-checklist decisions, retirement review, lookup audits and replay tools are
at `/workspace/vocabulary-final-gap-fill`. Download
`/workspace/noms-vocabulary-final-gap-fill.zip`. Raw provider archives, reference
pages, browser/session data and private draft contents remain outside Git and the
portable bundle. No Details field, source/nutrient ingestion, parser/ranking
change, automatic recipe assignment or production rollout is included.

## Next-sample gap fill and grocery expansion (2026-10-26)

A fresh outcome-blind draw selects **5,000 ingredient occurrences**, 250 from
each of 20 cuisine labels in the same checksum-pinned 39,774-profile
Yummly/Kaggle corpus. It excludes all 6,375 recipe IDs and exact normalized
ingredient-set fingerprints from the three earlier studies. The result contains
**1,237 normalized names from 4,143 previously unused profiles**: 242 names new to
prior name-level reviews, and 101 new even to all previous full-profile ingredient
contexts. The fixed seed is `noms-next-ingredient-gap-5000-2026-10-26-v2`.
Independent selection replay reproduces every occurrence. This is a new sample
from the historical snapshot, not a fresh crawl or 5,000 unique ingredients.

Against the preceding complete seed, explicit definition review finds 26 likely
missing-material names and 32 qualified-form/product cases: **58 phrases affecting
67 occurrences**. Another 265 occurrences contain ambiguity, brands, malformed
wording or grouped/components; these are not counted as proven material gaps.
Existing definitions represent 4,668/5,000 occurrences (93.36%) under this review.
A few older broad mappings are reassessed to preserve plain/nonfat yogurt,
white sesame, dry breadcrumbs and black-treacle qualifiers, so these percentages
are not a before/after score against earlier studies. Titles, quantities, methods
and individual recipe URLs are absent from the source and are not invented.

The user-approved curation adds **102 ingredients and 280 aliases**: 223 aliases
on the new entries and 57 on 29 existing identities. Current totals become
**13,463 active, 85 retired and 9,587 active aliases**. All 13,446 prior records,
including retired identities, retain their exact fields except approved alias
suffixes. Source descriptors and source mappings are unchanged; all new entries
have curated origin, original descriptions and no inferred external mappings.
The explicit ledger is `data/vocabulary/adapters/ingredient-next-gap-fill.json`.

### Material boundaries and reuse

- Fresh/dried tofu skin, raw/cooked lamb sausage, unspiced tarako/spicy mentaiko,
  dried mung-bean starch noodles and wasabi seasoning/pure wasabi powder keep
  consequential processing and formulation distinctions.
- Plain nonfat and plain Greek yogurt retain flavor/fat qualifications. Similar
  additions include unsweetened almond/oat/soy/cashew/coconut drinking beverages
  and barista oat milk; coconut drinking milk is not canned culinary coconut milk.
- Veal stock, white/brown veal stock, veal broth, bone-broth styles and existing
  demi-glace remain separate. Canned chicken broth does not imply low sodium or
  interchangeability with stock, dry bouillon or a concentrate.
- Pantry components include candied ginger, dry/plain-dry breadcrumbs, condensed
  tomato soup, refried black beans, raw crescent dough/baked crescent rolls,
  baked/frozen pound cake, gumbo vegetable blends and specified tomato packings.
- Truffle oils/pastes, food-grade flavoring oils, dried/ground guajillo, ground
  Sichuan pepper, coarse sparkling/sanding sugar, named sherries, port styles and
  spiced/amber rum retain useful product distinctions. Concentrations, exact
  formulas and substitution ratios are not inferred from brands.
- Roasted hazelnut forms reuse the existing dry-/oil-roasted filbert UUIDs.
  Thin spaghetti reuses spaghettini, artichoke bottom reuses the FoodOn exact
  heart synonym, and familiar sheep/beef/collagen casing names reuse their IDs.
  Existing tarako, lemon oil, cream sherry and white balsamic vinegar are retained.

All 58 discovery phrases have an explicit outcome. Forty-seven use reviewed new
material definitions; **11 remain source-ambiguous with usable qualified choices**:
`baking spray`, `crescent rolls`, `fideos`, `lamb sausage`, `natural peanut butter`,
`roasted hazelnuts`, `wasabi powder`, `diced tomatoes in juice`, `chinese rose wine`,
`mung bean noodles` and `pork and beans`. Their omissions do not become assumptions
about flour content, baking state, shape, roasting method, plant base or packing.
Qualified searchable aliases offer the choices without claiming a single raw
phrase is equivalent to all of them. No source recipe is automatically assigned
an ID. The separate green-tomato maturity and chrysanthemum-species review leads
remain outside this addition; existing definitions/statuses are not silently fixed.

Pinned FoodOn/Wikidata/USDA leads, current definitions, sampled ingredient wording
and public culinary/producer references support the review. References include
[Just One Cookbook's mitsuba](https://www.justonecookbook.com/mitsuba/),
[mentaiko](https://www.justonecookbook.com/mentaiko/) and
[matcha](https://www.justonecookbook.com/green-tea-powder-matcha/) guides,
[The Ginger People](https://gingerpeople.com/products/fiji-crystallized-ginger/),
[PAM baking spray](https://www.pamcookingspray.com/products/baking),
[Smucker's natural peanut butter](https://www.smuckers.com/peanut-butter/natural),
[Pictsweet gumbo vegetables](https://pictsweetfarms.com/product/vegetables-for-gumbo/frozen/steamables/)
and the [sherry regulatory council](https://www.sherry.wine/sherry-wine).
Capture types and hashes distinguish real response bodies, frozen ingredient
contexts, canonical baseline JSON and retained source-lead extracts. Commercial
examples are family/form evidence, not certification of every possible formula,
exact source equivalence, brand aliases or nutrient values. Raw pages, full
provider catalogs and copyrighted recipe methods remain external.

### Replay

The preceding complete bundle is main commit
`bd80fa349a2fe6231a4b42e0d4f9452354f0403d`:

- Input seed: `0b1bda94a192383da92a95ed6f0834c65024a088a2a0bc4618d43147e0c59f45`
- Output seed: `13ab2a642029fed668e6713c9cd31fb4dc0071000faac0822cc469a1712fc6d6`
- Unchanged sources: `1cb1ba876c7623c323889671bf87396e592fc1e2517b2a24a62fb8be623f181c`

Copy the preceding seed/source files from that commit into a fresh external
folder, then run:

```sh
python3 scripts/vocabulary/fill_gaps.py \
  --seed /tmp/next-gap-base/ingredients.json \
  --sources /tmp/next-gap-base/sources.json \
  --policy data/vocabulary/adapters/ingredient-next-gap-fill.json \
  --output /tmp/next-gap-replay
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py'
just seed-vocabulary --validate-only \
  --file /tmp/next-gap-replay/ingredients.json \
  --sources /tmp/next-gap-replay/sources.json
```

The builder never generates IDs or accesses the network. The ledger owns the
new UUIDs; stale baselines, name shadowing and unreviewed rewrites are rejected.
Historical-stage tests remove this explicit forward layer before checking their
original pins; earlier ledgers and checksums remain unchanged.

### Verification and scope

All **124 Python tests** pass, including exact replay, existing-field preservation,
source contracts, every candidate/checklist decision, qualified-search choices
and recipe-significant identity boundaries. The Rust seed validator accepts
**13,548 total records/four sources with no warnings**.

On the local mock-auth deployment, a dry run rolls back completely, the actual
load adds 102 entries/updates 29 alias-bearing entries, and a second `--if-changed`
run skips with no writes. Existing recipe, draft, version, source and mapping
hashes are unchanged by the load. All **382 new preferred/alias names** return
the intended UUID first through the production SQL query. Chromium verifies
catalog refresh from the previous 13,361-entry snapshot to 13,463 entries,
qualified ingredient choices and six local searches without per-query server
calls. The existing smoke flow verifies canonical UUID autosave/reload, clearing
IDs on text edits, optional server fallback and 161 authenticated lookup checks.
One private synthetic draft is used by the smoke check; no recipe is published.

Replaying the unchanged 5,000-occurrence review increases represented materials
**4,668 -> 4,724 (93.36% -> 94.48%)**, with no previously covered name losing its
material. Eleven newly represented families retain source ambiguity; there are
no unreviewed missing/partial phrases in this candidate queue. This is editorial
material availability, not automatic recognition, substitution or a whole-recipe
completeness score. No details field, schema/search rule or production deployment
is introduced. A later merge uses the existing fingerprinted deployment seeder.

Full discovery artifacts remain at `/workspace/ingredient-gap-next-5000`.
The curation ledger, full canonical JSON/CSV, new-entry/alias lists and verification
summaries are external at `/workspace/vocabulary-next-gap-fill`; the portable
review archive is `/workspace/noms-vocabulary-next-gap-fill.zip`.
