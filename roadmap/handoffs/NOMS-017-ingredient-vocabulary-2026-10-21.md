# NOMS-017 ingredient vocabulary: handoff

Started 2026-10-21; current state updated 2026-10-24.
FoodOn branch: `codex/noms-017-ingredient-vocabulary`.
Wikidata follow-up: `codex/noms-017-wikidata-adapter`.
USDA follow-up: `codex/noms-017-usda-adapter`.
Alias follow-up: `codex/noms-017-vocabulary-aliases`.
PR: https://github.com/SamKoCPSC/Noms/pull/16
Wikidata PR: https://github.com/SamKoCPSC/Noms/pull/17
USDA PR: https://github.com/SamKoCPSC/Noms/pull/18
Combined alias/scope PR: https://github.com/SamKoCPSC/Noms/pull/19

## Delivered state

Ingredient autocomplete, PostgreSQL persistence, deployment-only seed loading,
source provenance and public-recipe popularity tracking are implemented. The
FoodOn baseline has **10,148 active ingredients** (100 curated plus 10,048
imported), **60 retired identities**, and 12,016 FoodOn mappings, of which 11,956
belong to active entries. The Wikidata follow-up adds 2,550 ingredients and
3,670 Wikidata mappings, producing **12,698 active entries** with the same retirements.
The USDA follow-up adds 104 ingredients and 1,246 USDA mappings, producing
**12,802 active entries and 60 retired entries** across four pinned sources.
The later label-scope review retires another 21 entries, leaving the current
bundle at **12,781 active and 81 retired entries**, with all 12,862 UUIDs retained.
All original curated IDs and authored wording remain
unchanged; all 101 prior development identities, including the cumin probe, are
retained. Retired entries remain available to recipe history and are hidden
from suggestions.

Autocomplete accepts custom text, shows labels only and stores an optional UUID
on selection. Editing the name clears that reference. Failed suggestions do not
prevent saving. Exact labels and aliases rank ahead of prefixes; public published
recipe counts rank prefix matches. Private/unlisted recipes and drafts do not
contribute. An ingredient ID does not imply substitution or quantity compatibility.

## Maintained inputs and reproduction

- [Seed contract and rollout](../../data/vocabulary/README.md): validation,
  transactional loading, retirement, permissions and count reconciliation.
- [Adapter guide](../../scripts/vocabulary/README.md): pinned source acquisition,
  extraction, audit, selection, deterministic builds and browser commands.
- [FoodOn descriptor](../../data/vocabulary/adapters/foodon.json): release
  2026-09-20, immutable source commit, checksum, byte length and attribution.
- [Selection policy](../../data/vocabulary/adapters/foodon-cooking-policy.json),
  [editorial findings](../../data/vocabulary/adapters/foodon-label-review.json)
  and [pilot decisions](../../data/vocabulary/adapters/foodon-pilot-review.json):
  cooking scope, exceptions and source relationship judgments.
- [Persistent review ledger](../../data/vocabulary/adapters/foodon-cooking-review.json):
  stable UUID assignments, canonical wording, mappings and retirements.
- The two `foodon-cooking-base-*` files preserve the original 100-entry baseline.
  They are inputs to reconstruction and stable database test fixtures.

Follow the adapter guide's extraction and cooking-selection commands using the
pinned local OWL file. Reuse the saved UUID registry and original baseline;
never allocate new identities merely to rerun a build. Generated seed/source
files must match the maintained files before promotion. Preserve source notices
in the generated annotation report and inspect per-term attribution; the source
manifest does not replace that review.

Raw OWL files, USDA ZIPs, full catalogs, generated summaries/audits, annotation
reports and deployment/browser results belong outside Git. Only reviewed inputs,
canonical seed files, tooling and tests are maintained in the repository. Local
artifact paths and runtime ports are not checkout requirements.

## Coverage findings and limits

The [coverage report](../../data/vocabulary/coverage/README.md) and its reviewed
JSON evidence cover a fixed, purposively selected **100-ingredient benchmark**,
not the expanded seed. Wikidata supplies 68 usable discovery matches, FoodOn 50,
and generic USDA datasets 27. Their union is **84/100** with generic USDA
records, or **91/100** including additional branded products. Branded records
were reviewed only for the 73 generic USDA gaps. These are candidate-discovery
lower bounds, not formal exact mappings or proof of source absence.

Regenerate the report/CSV with `python3 data/vocabulary/coverage/summarize.py`.
For archive evidence, run `python3 data/vocabulary/coverage/inspect_usda.py
--archives /path/to/downloads` against the manifest's checksum-pinned archives.
Stronger Wikidata revision-aware evidence may refine related-only judgments.

The FoodOn expansion uses batch policy screening with explicit exceptions,
not individual semantic certification of all 20,152 eligible source terms.
Regional ingredients, edible parts, named varieties, pantry products and useful
forms are included; taxonomy, feed, medical/industrial formulations, complete
meals and unclear text are held out. Source synonyms are not bulk copied.
Overlapping source records receive conservative related mappings. Remaining
synonym/species/scope overlaps may still need editorial consolidation.

## Verification and deployment

Before publication, the full merged branch passed **1,113 Rust tests**, with 14
ignored, the WASM check, 20 offline Python tests, formatting, coverage regeneration,
tracked-secret, deployment-artifact and workflow checks. Full-source selection
and canonical seed builds reproduced byte-for-byte. PR checks record subsequent
CI results; this handoff is not a live CI status report.

The disposable local deployment passed dry-run rollback, idempotent loading,
canonical definition checks, unchanged authored recipes/drafts and healthy normal
startup with the full seed. Browser verification passed mock OAuth, keyboard/mouse
selection, UUID autosave/reload, reference clearing and free-text fallback, plus
61 authenticated lookup cases. Detailed run evidence belongs in PR notes or
external artifacts. No production rollout occurred.

For a fresh checkout, provision the documented Rust/PostgreSQL tools and run:

```bash
python3 -m unittest discover -s scripts/vocabulary -p 'test_*.py' -v
python3 data/vocabulary/coverage/summarize.py
cargo fmt --all -- --check
SQLX_OFFLINE=true cargo test --locked --no-default-features --features server -- --test-threads=8
cargo check --locked --target wasm32-unknown-unknown
SQLX_OFFLINE=true cargo run --locked --features server --bin seed-vocabulary -- --validate-only
```

Database tests use disposable PostgreSQL instances and require local port binding.
For application startup, follow the [cloud development guide](../../docker/codex-cloud/README.md)
and inspect current runtime metadata instead of assuming an old stack survives.
For an existing database, follow the seed guide: pause recipe writes, apply schema
and runtime grants with deployment credentials, seed/reconcile, then resume writes.

## Wikidata follow-up

The [Wikidata guide](../../scripts/vocabulary/WIKIDATA.md) records the revision
capture, policy screening of 9,197 source entities and all 10,148 active baseline
names, and reproduction commands. Descriptor, policy and approved UUID/revision
ledger are maintained; raw snapshots and all-name CSVs remain outside Git.
The identity pass covers 4,267 possible new names and 182 ambiguous overlaps:
2,505 additional imports, 504 related mappings and 1,440 specific holds. The
complete batch now has 2,550 imports and 1,120 additional source associations.
Every candidate has a recorded outcome; insufficient definitions, technical or
medical materials, commercial products and unclear scope remain excluded.
Names and scoped aliases propose related associations; explicit exceptions
address synonym traps and geographic/food ambiguity. Existing duplicate UUIDs
are retained, with one representative receiving related evidence. No hard merge
or redirects are introduced. See `resolve_wikidata_identities.py` for replay.

The canonical bundle was validated offline; the subsequent USDA pass loads the
combined bundle into the disposable test stack. All 48 Python tests and six Rust vocabulary tests passed; the
Rust validator accepts 12,758 entries and two sources with no warnings.
The identity-pass regressions cover form/species/part
distinctions, negative scope before misleading food ancestry, legacy duplicate
preservation, maps to new imports and stable UUID reruns. Revision restoration,
review and canonical seed/source generation reproduce byte-for-byte; all prior
10,253 identities, authored wording, origins, references and mappings are preserved.

## USDA follow-up

The [USDA guide](../../scripts/vocabulary/USDA.md) documents checksum-pinned
Foundation Foods 2026-04-30 and SR Legacy 2018-04 ingestion. All 8,262 eligible
records and 12,698 active baseline names are mechanically screened. Foundation's
87,521 sample/acquisition rows are excluded. No API key or nutrient ingestion is
needed. FNDDS and Branded are deferred; named commercial records within SR Legacy
are held as well.

The batch adds 104 explicitly reviewed identities, 1,142 related associations
(including two to new name families), and retains 7,016 held source records.
Of the holds, 4,319 complex/unmatched descriptions still need a specific editorial
definition. Complete case coverage is not individual semantic certification.
Analytical/preparation simplifications only propose related associations, never
exact nutrient links or new import wording. New imports require explicit policy
decisions. All 12,758 previous IDs, authored fields, origins, references and
mappings are preserved. No hard merges or redirects are introduced.

The approved UUID ledger and canonical seed/source files reproduce byte-for-byte
from the immutable pre-USDA baseline at `ab7ab53c32d48ff89f8fbd90833008f0ba292a73`.
All 61 Python tests and six Rust vocabulary tests pass, and the authoritative Rust
validator accepts 12,862 entries/four sources with no warnings. Raw ZIPs, catalogs,
all-name CSVs, summaries and deployment/browser evidence remain outside Git.
Both pinned download URLs returned HTTP 200 in the current environment. No
production rollout occurred.

The disposable local stack loaded the combined seed: 2,654 additions and 1,476
enriched definitions over its FoodOn-only database. Dry-run rollback, unchanged
authored recipes/drafts, all 12,862 canonical database definitions and idempotent
reload were verified. Headless Chromium passed UUID selection/autosave/reload,
reference clearing, free-text fallback and 109 authenticated lookup checks,
including all 104 USDA additions. Browser artifacts and runtime metadata remain
external. After environment recovery, inspect actual container ports and health;
old runtime metadata may not survive. Retained dependency archives allowed
offline Docker startup without a cold rebuild.

## Alias review follow-up (2026-10-24)

The [alias guide](../../scripts/vocabulary/ALIASES.md) documents the final overlay
on immutable pre-alias commit `c60162b0ad827963be7541ec12f21fd29764781d`.
All 12,802 active entries receive lexical screening; the 60 retired entries are
unchanged. The policy contains count-noun and regional spelling lexicons,
qualifier-preserving word order, 227 explicit ingredient alternatives, conflict
holds and reviewed suggestion representatives. It adds **7,009 aliases to 4,823
entries**, totaling 7,139 active aliases. Another 216 proposal/target pairs are
held. These counts precede the scope cleanup below. All UUIDs, labels, original aliases/order, definitions, statuses, source
relationships, provenance and source metadata are preserved. This is complete
lexical screening with explicit semantic exceptions, not individual semantic
certification of every imported concept. Upstream synonyms are not bulk copied.

The approved policy pins baseline seed/source and output checksums. Build/replay
must apply the overlay after source ingestion, rather than rewriting source
ledgers to absorb later authored aliases. Generated per-entry and per-alias CSVs,
source synonym queues and deployment/browser evidence stay outside Git. The
Rust validator accepts the complete bundle without warnings. Dry-run rollback,
canonical definition equality, repeat-load idempotence and unchanged existing
recipes/drafts were checked in the retained disposable local deployment.
All 73 offline Python regressions pass, including 12 alias tests; source-stage
tests now allow only appended aliases while retaining original wording/order.
Headless Chromium passed 31 additional lookup cases, keyboard/mouse selection,
UUID autosave/reload, edit clearing, free-text fallback and WASM delivery without
page errors. Its one new private synthetic draft is separate from the five
unchanged existing drafts. No production rollout occurred.

The frozen real-recipe check uses 100 TheMealDB recipes from 20 countries,
1,110 ingredient lines and 382 names. Source/ingredient availability decisions
are unchanged: 1,008 covered lines. Full-wording retrieval improves from
**747/1,008 (74.1%) to 941/1,008 (93.4%)**, with 194 recovered lines and zero
full-query regressions. Full or three-/four-character prefix retrieval improves
from 848 to 942 lines. Recipes with every line covered and found by full wording
increase from five to 22. This is a purposive sample, not a population estimate.
The run captured 978 API query forms; after 25 final spelling/plural aliases,
all forms were rechecked against the unchanged production SQL and the six
changed forms were also rechecked through the API. Remaining issues include
ambiguous names, preparation wording, named ingredient gaps and label scope;
aliases do not silently solve those by discarding food-form distinctions.

## Label scope cleanup (2026-10-24)

The same alias follow-up now includes source/definition review of **200 flagged
active labels**, plus an explicit bee-pollen ancestry exception. The pinned
policy retires **21 entries**: animal feed, clinical/supplement formulations,
manufacturing or unclear technical labels, catch-all classifications, unspecified
breakfast formulas and complete breakfast/pizza/eggs-Benedict meals. Exact source
mapping, original label and a case-specific rationale are required. Keyword
matches never retire entries automatically. Soup mixes, cereals/sausages, salad
dressings, pizza components, mold-ripened cheeses, edible blood, and rare/regional
foods remain available.

Current totals are **12,781 active, 81 retired, 7,137 active aliases** and two
aliases retained on newly retired entries. Relative to the alias-only commit
`12658575d3ca86f1cbfc83a8c2db369503252af7`, only 21 statuses change. Every UUID,
label, alias, definition, source relationship and provenance field is unchanged.
Replay from the original pre-alias seed still produces the final bundle
byte-for-byte. Full entry/proposal/scope audits remain outside Git.

All **76 Python tests** pass, including 15 alias/scope regressions. The Rust
validator accepts 12,862 entries/four sources without warnings. The disposable
deployment passes dry-run rollback, exact definition checks, idempotent reload
and unchanged authored rows. A private test draft selected a livestock-salt UUID
before retirement; after seeding it reloads with the same name/UUID and unchanged
stored row. **53 authenticated live lookups** confirm all newly retired IDs and
aliases are hidden while reviewed pantry ingredients remain available; Chromium
reports no page errors. The frozen 100-recipe/978-query SQL replay retains
941/1,008 full-wording hits and 942/1,008 full-or-prefix hits with zero regressions.
No production rollout occurred.

## Recipe-gap curation (2026-10-24)

The user-approved [identity policy](../../data/vocabulary/README.md#ingredient-identity-for-gap-curation)
keeps defined cooking materials separate when ordinary substitution would likely
require recipe adjustments or materially change the result. Stock/broth, cream
types, sodium/fat claims, meaningful processing, rice varieties and pasta shapes
are preserved; preparation wording is retained in recipes.

The frozen breadth study expands the original 100 to **1,000 profiles**: 500 from
the captured TheMealDB catalog and 500 from Yummly/Kaggle (25 per cuisine). It has
**10,768 ingredient lines and 1,769 names**, with unique IDs/normalized ingredient
profiles. These are purposive breadth samples, not population coverage estimates.
Provider facts, measures, IDs and selection are unchanged in the after-check;
full catalogs, instructions and generated audit/search outputs stay external.

The [gap guide](../../scripts/vocabulary/GAPS.md) and committed
`ingredient-gap-review.json` cover every one of the **314 missing/partial names**.
There are **258 new curated UUIDs**, 246 aliases on them and 25 aliases appended to
15 prior entries. The complete seed has **13,039 active, 81 retired and 7,408
active aliases**. All 12,862 previous UUIDs, original wording/order, definitions,
statuses, source evidence and provenance are retained. Source metadata remains
four pinned descriptors. The new layer records 21 related source associations,
authored definitions, public evidence URLs/hashes and explicit ambiguity holds;
it does not assert nutrient equivalence or automatically assign recipe IDs.

Name dispositions are 279 new-identity uses, 12 verified existing-identity uses,
three context-dependent cases, one probable source error and 19 holds. Spaghetti,
date syrup, shell pasta, 1% milk and several other names reuse verified existing
identities. Unqualified sweet soy sauce stays separate from specifically Javanese
kecap manis. Rice for sushi, cooked short-grain rice and seasoned sushi rice are
separate; only explicitly reviewed source contexts resolve bare sushi rice.
Black moss and lily buds need focused botanical/processing evidence. Bare
polenta remains ambiguous between the two newly represented forms.

The original alias output at `aa53490bedf674643a75ea9a8c71b57dff6da759` is the new
builder's immutable input. Its SHA-256 remains
`1726e0d5332dd3869cf6ba9cb7a5ee3e6adc76cb606a9f4d00aee2d9c67a73ca`.
The final seed checksum is
`bb4171b4892450ac5e61103f4cc61219b8c9e432fbd16388d35b95a0b2717e46`.
Replaying the original alias builder followed by `fill_gaps.py` reproduces the
final complete bundle byte-for-byte. Historical ingestion/alias pins are not
rewritten; source-stage tests verify the restored original alias stage.

All **82 Python tests** pass and the Rust validator accepts **13,120 entries/four
sources with zero warnings**. The disposable local stack passes dry-run rollback,
idempotent reload and equality of every deployed definition with the seed. All
original recipes, versions and drafts retain their stored-row hashes. Two new
private synthetic drafts test double-cream and chicken-stock selection, autosave,
reload, ID clearing and free-text fallback. The final Chromium run passes **529
authenticated label/alias lookups**, all new labels/aliases included, with no page
errors and successful WASM delivery. Two broader aliases were removed during
final review and their absence was verified through authenticated live requests. No recipes were published or imported and
no production rollout occurred.

Covered material rises from **9,706/10,768 (90.1%) to 10,339/10,768 (96.0%)**;
fully covered profiles rise from 369 to 663. This includes verified reuse and
scoped context review, not only new entries. The original 100 now have 1,077/1,110
covered lines. The 2,952-form SQL replay is read-only and separate from browser
checks. Among the original 9,706 covered lines, full-name retrieval improves from
8,347 to 8,357 and full-or-prefix retrieval from 8,542 to 8,551, with zero
regressions. Remaining source ambiguity and retrieval wording are separate queues.

Research, before/after reports, all CSVs, snapshots and browser evidence are under
`/workspace/vocabulary-gap-fill`; the loadable vocabulary plus reviewed additions
and frozen 1,000-profile after-check are in `/workspace/noms-vocabulary-gap-review.zip`.
Neither these local paths nor container state are guaranteed permanent. The
committed policy/builder/guide are the durable reproduction instructions.

After environment replacement, existing stopped containers were restarted
without deleting volumes. The app is healthy on `http://127.0.0.1:32768`, with
Postgres 32769 and mock OAuth 32772. Original `/tmp/noms-dev` lifecycle metadata
was lost; the lifecycle helper therefore refuses to attach to the existing
project. Browser-only metadata is external under the gap-review directory.
Inspect actual owned containers/ports before recovery, preserve the retained
volumes, and do not confuse browser-only metadata with a full lifecycle manifest.

## Source and name follow-up (2026-10-25)

The user deferred the proposed ingredient-details field and requested further
gap filling. The new `ingredient-gap-followup.json` is a separate reviewed layer
after `f1ed36568801bfe1b77af10ffcc163246031954b`; the original gap policy and all
earlier adapter/alias pins are unchanged. Replay the first gap stage and then
invoke the same builder with the explicit follow-up `--policy`. See the
[follow-up guide](../../scripts/vocabulary/GAPS.md#source-follow-up-2026-10-25).

This round adds **16 ingredients and 114 aliases**: 55 aliases on new entries and
59 appended to 26 prior entries. Totals are **13,055 active, 81 retired and 7,522
active aliases**, with four unchanged source descriptors. Every prior UUID,
definition, status, original alias/order, source association and provenance field
is preserved. The new seed SHA-256 is
`373d0b7b4c19f6c8e664655d1ebafedbae97e8028503cca63733c62e9609dd80`.

Government/university sources resolve fat choy and daylily material identity.
Original recipe sources resolve roselle calyces/jam, Colombian aliños and Lao som
moo; captured provider-country metadata is not used to infer those meanings.
Whole/ground coriander, generic/dry/liquid pectin and extract/imitation vanilla
remain separate. Generic golden raisins do not imply seedlessness, and generic
olive-oil-packed tuna does not imply the prior solid-pack canned subtype.
Pul biber and red vermouth preserve unspecified subtype information.

All **53 reviewed names** have dispositions: eight new-identity uses, five scoped
source contexts, 31 existing-identity uses and nine holds. Ten of the original
nineteen holds are resolved; remaining holds are ground red pepper, almond/vanilla
essence, light raisins, bare polenta, salmon sashimi, Sicilian olives, a branded
sweet pepper product and zapallo. Alternative pectin/daylily/vanilla forms and a
generic sweet-pickled piquanté material do not force an uncertain source wording
to a product. Bare sorrel and brands do not gain unsafe global aliases.

Useful names include Dijon mustard, beansprouts, purple onion, jalapeño variants,
apple cider vinegar and skinless/boneless cut word order. Qualified names such as
`pepper (black)`, `red pepper (bell)` and `dried sorrel (Caribbean roselle)` make
explicit candidates discoverable by prefix. Bare pepper/red pepper are not exact
aliases, and the runtime still attaches no ID until an author selects a suggestion.
Reserved legacy names, including `swedes` and `apple vinegar`, retain ownership.

All **88 Python tests** pass. The authoritative Rust validator accepts **13,136
entries/four sources with zero warnings**. The test deployment passes dry-run
rollback, definition equality and zero-change repeat loads. All originally
existing authored rows retain their hashes. One new private synthetic draft
tests ground-coriander selection, UUID save/reload, clearing and free-text
fallback. Read-only live checks pass **130 exact label/alias cases plus nine
prefix/ambiguity/escaping guards**. No page errors or failed WASM responses; no
recipes are imported/published and no production rollout occurs.

The unchanged 1,000-profile corpus now has **10,349/10,768 covered material lines
(96.1%)** and **667 fully covered profiles**. Both previously missing botanical
names are represented. Full-wording candidate hits rise from 8,978 to **9,359**;
full-or-three/four-character prefix hits rise from 9,174 to **9,531** under the
updated specific-form review. These are candidate retrieval metrics, not an
automatic parser or source-disambiguation score. On the frozen previous 10,339
covered lines and their original expected IDs, full-wording hits rise to 9,319
and full-or-prefix hits to 9,496, with zero regressions in either measure.

External evidence, CSVs and browser/SQL checks are in
`/workspace/vocabulary-gap-followup`; the new download is
`/workspace/noms-vocabulary-gap-followup.zip`. Earlier report archives remain
frozen. The new archive includes full before/after seeds, the explicit review,
source pins and the 1,000-profile after-check with verified offline replay; raw
provider catalogs, recipe instructions, reference HTML and browser sessions are
excluded. The local app is healthy on the retained ports noted above.


## Preferred-label review (2026-10-25)

The [label ledger and replay guide](../../scripts/vocabulary/LABELS.md) follow
commit `1d015efd6f9409817056a3d62283d970a68e907a`. All 2,868 active parenthetical
labels have decisions: 2,624 become natural cooking names and 244 retain their
wording for scientific/contextual or name-ownership reasons. `tomato (canned)`
becomes `canned tomatoes`; `chicken breast (skinless, boneless)` becomes
`boneless skinless chicken breast`. Former names remain aliases with the same
UUID. Species, edible parts, cooking states, fat/sodium formulations and packing
media retain their distinctions. No source definitions, statuses, mappings or
authored recipe fields change; duplicate-identity merges remain deferred.

Current seed SHA-256 is
`f6f5d4021ac18e7a4897752aa4c31aa9d2957ebf17608d4c6e5353ca0d280e4a`:
13,055 active, 81 retired, 8,549 active aliases, four unchanged source descriptors.
The per-ID ledger pins input/output, records alias promotions and holds, and
reconstructs the exact preceding seed for historical source/curation checks.
Generated full audits and projected recipe reports remain external.

All 94 Python tests pass; Rust validation has zero warnings. Local loading
updates 2,624 entries, dry-run fully rolls back and repeat load is a no-op.
Database definitions equal JSON; all preexisting authored row hashes remain
unchanged. All 5,248 old/new full-label SQL queries return their original UUID
first. Browser checks pass 192 authenticated lookup cases, keyboard/mouse
selection, autosave/reload, ID clearing, free-text fallback and WASM delivery.
A before-update private draft still reloads `tomato (canned)` with its original
UUID; new selection uses `canned tomatoes` with that same UUID.

Frozen 1,000-profile material availability stays 10,349/10,768 lines (96.1%),
667 fully covered profiles. Full-wording expected candidate hits improve
9,359 → 9,370 (11 gained, zero lost). Full-or-three/four-character-prefix hits
shift 9,531 → 9,520 (19 gained, 30 lost across 12 names), as the label tie-breaker
changes broad eight-result prefixes. Every old/new full label still resolves
first. Ranking/runtime behavior was not changed to conceal this tradeoff.

Review and test artifacts: `/workspace/vocabulary-labels`. New download:
`/workspace/noms-vocabulary-readable-labels.zip`. Older reports remain frozen.
Two new private synthetic drafts were used; nothing was published or imported,
and no production rollout or Details field is included.


## Second held-out recipe sample (2026-10-25)

The [gap-research guide](../../scripts/vocabulary/GAPS.md#second-held-out-1000-profile-study-2026-10-25)
records another **1,000** disjoint source profiles: 200 TheMealDB recipes and 800
Yummly/Kaggle ingredient lists, 40 from each of 20 cuisine labels. Zero prior-ID
or exact ingredient-set overlaps; 10,891 lines, 1,850 normalized names, 732 new
name strings. Selection uses the prior pinned catalogs and never vocabulary
results. Titles/measures absent from Yummly remain absent.

Current unchanged seed represents **10,185 lines (93.5%)**, with 516 fully covered
profiles. **150 candidate gap phrases** (27 absent named materials, 123 specific
forms/formulations) affect 182 lines across 154 profiles. These are research
candidates, not approved new ingredients/aliases or a new-ID count. Source
ambiguity, malformed wording, brands and subrecipes remain separate. Source and
definition cross-checks identify rotini/fusilli, chicken bones and chevre frais
as existing materials with naming/discoverability issues. Inactive/taxon source
leads are not approved edible ingredient mappings.

Read-only SQL captures 3,091 query forms; 100 authenticated live lookups plus two
wildcard guards agree. Full-wording expected candidates appear for 8,790 covered
lines, or 9,019 using full wording/short prefixes. This is retrieval evidence,
not automatic recipe recognition. Selection and analysis reproduce exactly;
canonical/source files are unchanged and no recipe/draft writes occur.

External report, all 1,850 decisions, per-recipe/line CSVs, candidate families,
147 upstream lexical leads and offline verification are at
`/workspace/recipe-vocabulary-second-1000`; download
`/workspace/noms-second-1000-recipe-gap-review.zip`. The mix differs from the first
sample, so its raw coverage rate is not a before/after comparison. Define and
identity-review the selected candidates before making seed changes.


## Second-sample common-ingredient fill (2026-10-25)

The [curation/replay guide](../../scripts/vocabulary/GAPS.md#second-sample-curation-2026-10-25)
and `ingredient-second-sample-gaps.json` review all 150 second-sample candidate
phrases plus nine naming discoveries. They add **148 defined ingredients**,
including qualified alternatives, and **298 aliases** (273 on new entries,
25 appended to 13 existing identities). Examples include MSG, named pasta shapes,
fresh mozzarella, Greek-yogurt/fat variants, stock/broth/sodium forms, preserved
produce and common baking/seasoning products. Existing identities are reused
for verified equivalents such as ground ancho/ancho powder, rotini/fusilli,
unsweetened chocolate/chocolate liquor and soppressata/sopressata salami.

Current seed SHA-256:
`6e703364b9ab3b7476a0637b7d1063dcdf0096dfb26d6b7a42b740b39acfa821`.
Totals: **13,203 active, 81 retired, 8,847 active aliases**. All original preferred
labels, definitions, statuses, mappings and provenance remain unchanged; only
approved aliases are appended. All four source descriptors retain their bytes.
The new stage follows the immutable preferred-label output. Historical tests
verify/remove each later layer before asserting source/curation/label invariants.

Five phrases remain held: apricot brandy, Filipino eggplant, thin pizza crust,
33%-less-sodium ham and 40%-less-sodium taco mix. Explicit liqueur/eau-de-vie,
raw/prebaked thin bases and generic reduced-sodium alternatives do not force a
bare name, cultivar or unknown comparison product into a precise identity.
Schenkstroop and Jamaican browning are resolved only in their source recipe
contexts. The congee's cordial/juice mismatch and malformed gelatin-leaf/coloring
names remain correction cases, with the intended materials available. Current
FDA evidence supports MSG; the obsolete FoodOn identifier is not reactivated.
Raw source instructions/reference pages remain external; brand examples do not
create brand aliases or exact nutrition/source mappings.

All **101 Python tests** pass; authoritative validation accepts **13,284 entries
and four sources with no warnings**. Full seed/source replay is byte-identical.
Local deployment passes transactional rollback, definition equality, no-op repeat
loading and preservation of all preexisting authored row hashes. All **446 new
preferred/alias names** resolve to their intended UUID first. Chromium passes
181 final lookups, keyboard/mouse selection, UUID autosave/reload, clearing on
edit, lookup-failure free text and WASM delivery. One new private synthetic draft; no recipe import/publication or
production rollout.

The second sample rises **10,185 → 10,359 covered lines out of 10,891**
(**93.5% → 95.1%**) and **516 → 616 fully covered profiles**. The candidate-gap
queue is empty after clear materials, ambiguity and source corrections receive
their dispositions; this does not imply all source rows are resolved. Full
candidate hits rise **8,790 → 8,973** and full/short-prefix hits **9,019 → 9,199**.
On the frozen 10,185 previously covered lines/expected IDs, full hits rise to
8,801 and full-or-prefix to 9,025, with zero losses. The original 1,000 sample
retains 10,349 covered lines/667 fully covered profiles and 9,370/9,520 retrieval
hits, also with zero losses. No matching/ranking/parser or Details-field changes.

External evidence, audits, full vocabulary JSON/CSV, decision/held lists and both
recipe after-checks are in `/workspace/vocabulary-second-gap-fill`. Download:
`/workspace/noms-vocabulary-second-gap-fill.zip`. Earlier reports/archives stay
frozen. Generated catalogs, instructions, raw reference pages, browser sessions
and private draft data are excluded from Git/downloads.

## Grocery-material follow-up — 2026-10-25

User-authorized grocery-store curation adds **29 ingredients and 142 aliases**
(94 on new entries, 48 on 17 published identities). Current seed: **13,232 active,
81 retired, 8,989 active aliases**. New forms include dried angel hair, distinct
bouillon products, whole-grain corn flour, white masarepa, instant polenta,
Calvados/grappa/pisco/slivovitz and practical undressed/unsauced grocery mixtures.
Original FoodOn dry-mix annotations justify reusing the published chicken bouillon
mix UUID for powder/granules. Flour/starch/masa/masarepa, fresh/dried pasta,
cube/powder/paste and spirit/liqueur distinctions remain intact. Broad source
phrases are not automatically assigned to a narrower qualified retail form.

Ledger: `data/vocabulary/adapters/ingredient-grocery-gaps.json`. Input is commit
`998a69d06e2be4e7ab130bbf3ab8a81a68d00715`, seed SHA-256
`6e703364b9ab3b7476a0637b7d1063dcdf0096dfb26d6b7a42b740b39acfa821`.
Output: `8a6c4f4054700d301dd293daa1de72f3e599723f631fe09eb8be45bee64515ab`.
Four-source manifest and every prior identity field/alias prefix are preserved.
Earlier checksum-pinned stages remain immutable; the generic builder reproduces
the complete new seed byte-for-byte. Evidence URLs/hashes live in the ledger,
while raw pages, full CSVs and generated audits remain outside Git.

**108 Python tests**, authoritative Rust validation of **13,313 entries/four
sources**, dry-run rollback, no-op repeat loading, DB/JSON equality, **171 new-name
SQL checks**, **176 authenticated API checks** and Chromium selection/autosave/
reload/failure fallback all pass. All prior authored row hashes are preserved;
one new private synthetic draft, no recipe import/publication or production load.
Both frozen recipe studies retain every full-wording candidate; second-sample
full hits gain three. New spring mix displaces spring-roll pastry at only 3/4
letters for five rows; `spring roll` still returns the existing wrapper. This is
recorded in [GAPS.md](../../scripts/vocabulary/GAPS.md#grocery-material-follow-up-2026-10-25),
without inventing an alias that equates rice wrappers and wheat pastry.
No new recipe coverage percentage, Details field or runtime/ranking change.

Artifacts: `/workspace/vocabulary-grocery-gap-fill`; complete vocabulary/review
archive: `/workspace/noms-vocabulary-grocery-gaps.zip`. Earlier reports and
archives remain frozen.

## Final random gap discovery — 2026-10-26

Discovery-only random sample: **5,000 ingredient occurrences**, 250 per each of
20 Yummly/Kaggle cuisine labels; **1,243 names**, **4,375 source profiles**,
**292 names new to both earlier samples**. Zero prior recipe-ID or exact profile
overlap. Sampling uses a fixed SHA-256 seed and the pinned 39,774-profile archive,
without vocabulary-based selection. Only selected rows count; full profiles are
source context, not additional reviewed ingredients or complete-recipe checks.

Results: **4,643 covered**, **58 clear-gap occurrences across 56 names**, **25
qualified-review occurrences across 20 names**, 227 ambiguous, 17 branded, 23
source errors and seven composite rows. The 76 candidates need curation; none
is automatically assigned a new UUID. The 92.86% editorial availability rate is
specific to this balanced source-corpus sample and is not comparable with earlier
purposive rates or automatic-assignment accuracy. No vocabulary/deployment change.

Important follow-up: exact `sweet peas` currently names *Lathyrus odoratus*, while
culinary garden peas already exist as `green pea`; plural `Lamb chops` describes
a grilled dish, while singular `lamb chop` defines the raw cut. Track these as
name/definition cleanup, not new ingredient gaps. Rotelle can mean spiral pasta
in manufacturer naming and is left for shape review. Rock sugar, bacon drippings,
shungiku and other verified synonyms are removed from the new-ingredient queue.

The seed remains commit `70c01447aa46f35b90ea539a5b972ce7dd7d2b91`, SHA-256
`8a6c4f4054700d301dd293daa1de72f3e599723f631fe09eb8be45bee64515ab`.
[GAPS.md](../../scripts/vocabulary/GAPS.md#final-random-ingredient-discovery-2026-10-26)
records scope, counts, examples, source pins and limitations. Complete sample,
explicit decisions, candidate/quality CSVs and verified independent replay are in
`/workspace/ingredient-gap-final-5000`; download
`/workspace/noms-final-5000-ingredient-gap-review.zip`. Generated full audits,
provider snapshots and raw reference captures remain outside Git.

## Remaining scope

Other metadata dimensions, localization, further Wikidata/USDA curation, FNDDS and
Branded ingestion, nutrient linkage,
automatic recognition/backfill, vocabulary-aware portability policy, search
facets, merge redirects and personal suggestions remain future work. See the
[implementation plan](../implementation-plans/NOMS-017-recipe-metadata-autocomplete-vocabulary.md).
