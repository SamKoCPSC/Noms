# Maintained ingredient vocabulary

`ingredients.json` is the reviewed source of truth for ingredient suggestions.
The seed contains **13,532 active ingredients**: the original 100 curated entries,
10,025 active FoodOn-derived cooking/pantry entries, 2,548 active Wikidata additions,
104 USDA additions and 755 subsequently curated cooking-material definitions.
Another **85 identities** are retained as retired: 60 from the original deployment
review, 21 from the ingredient-scope cleanup and four from the final definition review.
Every original ID, description and prior searchable label/alias is preserved,
including the original fifteen IDs. Preferred labels may change through the
explicit label-review layer below; authored recipes are not rewritten. The
previously tested cumin-seed UUID is also retained. Before gap curation there were
12,016 FoodOn source mappings (11,935 on active entries); overlapping source records use conservative `related` relationships
rather than inferred exact equivalence from matching names.

The expansion uses a recorded cooking-scope policy and explicit source-specific
exceptions. This is batch screening, not individual semantic certification of
every source annotation. It includes regional foods, edible parts, named varieties,
pantry products and useful forms; organizational/industrial/medical categories,
complete meals and unclear terms are held out. Source synonyms are not copied
wholesale. See `adapters/foodon-cooking-policy.json` and the persisted UUID/review
ledger in `adapters/foodon-cooking-review.json`.

The [Wikidata adapter](../../scripts/vocabulary/WIKIDATA.md) adds 2,550 ingredients
and 1,120 additional source associations, totaling 3,670 Wikidata mappings.
Its identity review gives all 4,449 initially unresolved candidates an explicit
outcome; 1,440 stay held for specific scope/evidence reasons. The combined seed
also retains the same 60 retired identities. Existing IDs and authored wording
are preserved; related evidence does not hard-merge duplicate legacy UUIDs.
The source snapshot is pinned by entity revisions and checksum, and full case
audits remain outside Git.

The [USDA adapter](../../scripts/vocabulary/USDA.md) screens all 8,262 food records
in pinned Foundation Foods and SR Legacy releases, adds 104 explicitly reviewed
identities and 1,142 related associations, and holds 7,016 records for scope or
further definition. Its 1,246 mappings retain FDC IDs and archive provenance.
Analytical/production qualifiers can propose related evidence for an existing
ingredient; they do not establish exact equivalence or link nutrient values.
All 12,758 pre-USDA IDs and authored fields are preserved. Raw archives and full
audits stay outside Git. FNDDS, Branded and nutrient ingestion remain deferred.

The subsequent [alias and scope review](../../scripts/vocabulary/ALIASES.md) screens every
active name and adds 7,009 reviewed plurals, regional spellings, alternative names
and qualified word-order variants to 4,823 entries. There are now 7,137 active
aliases, plus two retained on newly retired entries. Source/definition review of
200 flagged labels retires 21 unsuitable entries, including livestock salt blocks,
clinical formulations, manufacturing terms, catch-all categories and finished
meals. Their UUIDs, labels, aliases and source evidence remain available for
history; only active suggestion status changes. The recorded policy holds
conflicting aliases and reproduces from the immutable combined pre-alias seed.
It does not add ingredients or change runtime matching semantics.

The subsequent [gap curation](../../scripts/vocabulary/GAPS.md) reviews all 314
missing/partial names in a frozen 1,000-profile recipe sample. It adds 258 defined
cooking materials and 246 aliases on them, plus 25 aliases on 15 existing entries;
there are now **7,408 active aliases**. Stock/broth, cream, formulation, named
variety and meaningful processing distinctions remain separate. Verified
equivalents reuse their existing UUIDs; 19 unclear source names stay held. The
immutable ledger, builder and tests reproduce the final seed after the original
alias stage. Existing definitions/statuses/source evidence and all four source
descriptors are preserved. Source data, recipe text and generated audits stay
external; this layer does not change runtime recognition or substitution rules.

The separately pinned October 25 follow-up resolves ten of the nineteen held
names through source evidence or explicit generic material definitions, and
adds useful specific forms and names. It adds **16 ingredients and 114 aliases**:
55 on new ingredients and 59 appended to existing entries. Totals at that stage are
**13,055 active, 81 retired and 7,522 active aliases**. Qualified botanical and
regional names preserve context; ground coriander, dry/liquid pectin and vanilla
extract/imitation flavouring remain distinct. All prior IDs, definitions and
source metadata remain intact. See the [follow-up evidence and replay guide](../../scripts/vocabulary/GAPS.md#source-follow-up-2026-10-25).

The subsequent [preferred-label review](../../scripts/vocabulary/LABELS.md)
renames **2,624** parenthetical labels into ordinary cooking word order, such as
`tomato (canned)` → `canned tomatoes`. All old names remain searchable against
the same UUIDs. Another **244** labels retain their wording for contextual,
scientific or name-ownership reasons. Totals at that stage are **13,055 active,
81 retired and 8,549 active aliases**. Cooking states, species, edible parts,
fat/sodium qualifiers and packing media retain their distinctions. This layer
changes preferred vocabulary labels only; recipes and source definitions remain
unchanged. Historical stage counts and checksums above remain frozen.


The subsequent [second-sample curation](../../scripts/vocabulary/GAPS.md#second-sample-curation-2026-10-25)
adds **148 defined cooking materials** and **298 aliases** (273 on new entries,
25 on 13 existing identities). These include MSG, named pasta shapes, fresh
mozzarella, Greek-yogurt formulations, preserved produce and stock/broth forms.
Ambiguous names get qualified alternatives or scoped corrections rather than
forced identities. All prior preferred labels, descriptions, source evidence,
statuses and alias order remain intact. Totals at that stage are **13,203 active,
81 retired and 8,847 active aliases**; the four-source manifest is unchanged.

The subsequent [grocery-material follow-up](../../scripts/vocabulary/GAPS.md#grocery-material-follow-up-2026-10-25)
adds **29 ingredients and 142 aliases**: 94 on new definitions and 48 on 17
published identities. Dried angel hair, bouillon forms, whole-grain corn flour,
white masarepa, instant polenta, named brandy styles and practical retail mixtures
are represented. Chicken bouillon powder reuses the existing dry-mix UUID after
checking its original source annotations. Totals at that stage are **13,232 active,
81 retired and 8,989 active aliases**. The separate checksum-pinned ledger is
`adapters/ingredient-grocery-gaps.json`; all prior stages and source descriptors
remain immutable. This adds suggestion data without assigning recipe identities.

The [final random discovery audit](../../scripts/vocabulary/GAPS.md#final-random-ingredient-discovery-2026-10-26)
checks 5,000 occurrences across 20 cuisine labels: 1,243 names from 4,375 unused
profiles. It identifies 56 clear missing-material candidates and 20 qualified
review candidates, plus two misleading exact-name definitions. This audit does
not add ingredients or aliases; totals at that stage remain unchanged.

The subsequent [final gap fill and pantry expansion](../../scripts/vocabulary/GAPS.md#final-gap-fill-and-pantry-expansion-2026-10-26)
adds **133 ingredients and 322 aliases** (257 on new entries, 65 on 31 existing
identities). The review covers all 76 discovery candidates plus a 154-name
checklist of extracts, Asian produce, mushrooms, vinegars and cured meats.
Qualified alternatives preserve unknown source forms. Verified synonyms reuse
published IDs, including yu choy/choi sum, broccoli stems/stalks, coffee extract/
liquid coffee concentrate, black trumpets/horn of plenty and Chinkiang/Zhenjiang.
Four misleading source entries are retired with every historical field retained:
ornamental sweet pea and its canned child, a grilled lamb-chop dish and a sweet
raspberry-vinegar drink. Culinary names use the appropriate existing pea/lamb IDs
and a separately defined cooking vinegar. No authored recipe is rewritten.
The ledger is `adapters/ingredient-final-gap-fill.json`; totals at that stage are
**13,361 active, 85 retired and 9,307 active aliases**, with the same four sources.

The subsequent [next-sample gap fill](../../scripts/vocabulary/GAPS.md#next-sample-gap-fill-and-grocery-expansion-2026-10-26)
reviews all 58 candidate phrases from another 5,000-occurrence sample and adds
**171 definitions and 438 aliases** (367 on new entries, 71 on 34 existing
identities). It covers useful related forms of tofu skin, yogurt, plant drinks,
stocks/broths, preserved pantry components, flavoring oils, seasonings and named
wine/rum styles. Existing filbert/hazelnut, spaghettini, casing and other synonym
identities are reused. Eleven underspecified source phrases retain explicit
qualified choices rather than forced aliases. Totals at that stage are **13,532 active,
85 retired and 9,745 active aliases**. The forward ledger is
`adapters/ingredient-next-gap-fill.json`; every previous definition, UUID, status,
source association and existing alias is retained. One recorded spelling
correction changes `crystaline fructose` to `crystalline fructose`, retaining
the old label as an alias on the same UUID. The four-source manifest is
unchanged, and authored recipes are not rewritten.

The plant-milk follow-up adds hazelnut, macadamia, walnut, pecan, Brazil nut,
flaxseed, sesame, sunflower seed, pumpkin seed, quinoa, buckwheat, amaranth, teff,
millet and lupin milks, plus explicitly unsweetened tiger-nut milk. Existing
spelt drink and sweetened horchata de chufa retain their IDs and gain qualified
milk aliases. Base-only names do not assert an unsweetened or fortified formula.

The flour/starch follow-up adds teff, einkorn, emmer, lupin and tiger-nut flours;
generic, white and wholemeal khorasan wheat flours; and sweet-potato, mung-bean
and Chinese water-chestnut starches. Explicit milling forms and extracted starches
retain their distinctions. Tapioca/cassava/manioc starch names reuse the existing
tapioca ID; sweet/sticky rice flour names reuse glutinous rice flour. This is a
separate user-requested expansion, outside the frozen recipe-sample score.

The molecular-ingredient follow-up adds 42 food-grade materials: alginate and
calcium salts; carrageenan/gellan subtypes; cellulose thickeners; specialized
pectins; lecithins and emulsifiers; dextrins/fibers; enzyme preparations; food
acids, trehalose and glycerin. Generic entries retain unspecified grades and
formulations; explicit powder forms, resistant dextrin and fat-powdering carrier
grades remain separate. Existing crystalline fructose gains powder aliases.
Liquid nitrogen, dry ice, carbon dioxide and nitrous oxide are excluded.

The subsequent [readable-label review](../../scripts/vocabulary/LABELS.md#readable-label-follow-up-2026-10-27)
renames **488 ingredients** into ordinary cooking word order, including the
dried chicken egg forms, named cheeses, juices, herbs and processing qualifiers.
It promotes **139 existing aliases** to preferred labels and retains every old
label, adding **349 searchable aliases** overall. The **1,009 held
labels** retain their wording because of name ownership, unclear definitions or
meaningful technical detail. All IDs, descriptions, statuses, source mappings and
recipe text remain unchanged. At that stage totals are **13,532 active, 85 retired and
10,094 active aliases**. The separate forward ledger is
`adapters/ingredient-readable-label-review.json`; prior curation pins remain frozen.

The user-approved berry follow-up changes `berry (impregnated with sugar)` to
`sweetened berries`, retaining both upstream names as aliases. The separate
[awkward-language review](../../scripts/vocabulary/AWKWARD_LABELS.md) records
36 further candidates. The 12 approved wording replacements are now applied;
the subsequent kitchen-suitability review resolves the remaining 24 candidates.
Five of these 12 approved ingredients extend the original 1,492-ID review scope,
bringing the ledger's explicit scope to 1,497 IDs.

The [kitchen-suitability review](../../scripts/vocabulary/KITCHEN_REVIEW.md)
keeps `shumai wrappers`, `liver spread` and `peeled nuts` under natural names and
retires 21 unsuitable or redundant selections, including cattle-feed grains,
corn for degermination and the regulatory classification reference. Retired
UUIDs and definitions remain available for existing recipe references; they
are excluded from active search and the client catalogue. No recipe IDs are
reassigned. Current totals are **13,511 active, 106 retired and 10,088 active
aliases**. The separate ledger is `adapters/ingredient-kitchen-suitability-review.json`.

The source coverage research still measures the original **100-ingredient**
benchmark across 19 strata. Those percentages do not describe the expanded seed.
See `coverage/benchmark.json` and the coverage review for source-matching evidence.
Personal suggestions are deferred. Time classification
is outside this dataset and belongs in shared application code.

The [FoodOn adapter](../../scripts/vocabulary/README.md) extracts a checksum-pinned
OWL snapshot into review candidates and converts approved decisions into complete
canonical seed/source files. It preserves existing IDs and authored wording.
Use `just seed-vocabulary --validate-only --file PATH --sources PATH` to run the
Rust validator without connecting to a database before a normal loader dry-run.

Apply the declarative schema with the existing `just migrate` workflow first.
Production database preparation now runs the packaged loader automatically with
`--if-changed` after schema/grants. A fingerprint of both parsed JSON contracts is
checked against the latest committed seed receipt under the writer lock. Matching
inputs skip the entry scan and all seed writes. Changed inputs load atomically;
failures and dry runs do not mark them applied. JSON whitespace/key order does
not change the fingerprint; ingredient/alias/status/source changes do.

For an explicit check or repair, with the deployment/seed role's `DATABASE_URL`
exported, run:

```bash
just seed-vocabulary --dry-run
just seed-vocabulary
```

Omitting `--if-changed` checks actual database definitions even if the fingerprint
matches. This is the repair path for out-of-band database edits. Deployment uses
the committed files without fetching or regenerating any upstream dataset.

The recipe editor searches maintained English labels and aliases by prefix after
two characters. Results prefer exact labels, then exact aliases, then prefixes;
prefix matches sort by public recipe count descending, then label and ID. Label
and alias prefixes share the same popularity tier. Results show labels only,
are deduplicated by entry ID and capped at eight. Selecting a result uses
its label and optional ID. Typing always accepts custom text and clears an old ID.
Missing suggestions never prevent saving. Existing ingredient JSON without IDs
remains readable; draft and published snapshots retain optional IDs independently
of subsequent vocabulary changes. A reference does not imply interchangeability.

## Client autocomplete cache

Recipe creation, draft editing and published-recipe editing share one browser
search index. It contains active English ingredient IDs, labels and aliases;
descriptions, provenance and authored recipe data are excluded. Search uses the
same Unicode normalization, exact/alias priority, public popularity ranking and
eight-result limit as the server. Once loaded, typing searches locally without
the server lookup's 200 ms debounce or a request per query.

The authenticated `GET /api/vocabulary/ingredients/version.json` returns the
schema version, catalog SHA-256, content URL and sparse public-recipe counts.
The content URL, `/api/vocabulary/ingredients/{sha256}.json`, serves the compact
catalog with gzip support, a private immutable cache policy and conditional
ETags. Errors are never cached. Both routes use the existing request/work budgets.

The browser validates the schema and checksum before replacing its index, then
persists only catalog data and public counts in CacheStorage. Reopening an editor
uses the stored index immediately while revalidating the small manifest. Returning
to the tab or focusing an ingredient checks again, throttled to once a minute
(five seconds after failure). A changed catalog hash triggers one new download;
popularity changes update ranking without downloading the catalog again. Storage
failures are tolerated. Failed refreshes retain the last usable index; without an
index, the original debounced server suggestions remain available. Custom text
and existing selection/reference behavior are preserved.

Each server process coalesces snapshot builds and caches the compressed response.
Manifest requests check the ingredient count/update timestamp at most once per
second; committed seed edits, including aliases and retirement, invalidate the
catalog. Public counts refresh within five minutes of the next manifest request.
Two previous catalog versions remain available for concurrent requests; a client
receiving `410 Gone` rechecks the manifest once. No schema or seed changes are
needed for this feature.

## Seed contract (version 1)

- `dimension`: currently `ingredient`; `locale`: currently `en`.
- `id`: a non-nil UUID assigned once and retained across renames and retirement.
- `label`, `aliases`: display wording, 1–200 characters; comparison keys use NFKC,
  lowercase, and collapsed whitespace. No stemming or inferred equivalence.
- `description`: up to 2,000 characters of cooking context.
- `status`: `active` or `retired`. Retired entries retain their IDs and are hidden
  from autocomplete. Keep them in the file; omission fails rather than deleting.
- `provenance`: `origin` is `curated` or `imported`; `references` identifies source
  manifest entries. Imported entries require at least one reference.
- `source_mappings`: source ID, external ID, and `exact`, `broader`, `narrower`, or
  `related` relationship. These mappings do not merge Noms entries.

`sources.json` records external source IDs, HTTPS URLs, versions, SHA-256 checksums
(as 64 hexadecimal characters), licenses, and attribution. The manifest now pins
FoodOn's 2026-09-20 snapshot with CC-BY-4.0 attribution and a revision-pinned
Wikidata structured-data capture under CC0-1.0, and USDA Foundation Foods
2026-04-30 and SR Legacy 2018-04 public-domain archives under CC0-1.0.
Per-term FoodOn source notices
remain in the adapter's generated report; the manifest does not replace review
of third-party definition attribution. The original entries retain curated origin;
new source-derived entries use imported origin.

A normalized label/alias must be unique within an entry. Collisions between
entries produce warnings and remain separate suggestions for review. IDs are
never deduced from names. The loader does not ingest upstream data directly.

## Ingredient identity for gap curation

Use the recipe effect of substitution when deciding whether related cooking
materials need separate identities. If replacing one with the other in ordinary
uses would likely require changing the recipe's balance or method, or materially
change flavor, texture, body, whipping, setting or other cooking behavior,
represent them as distinct ingredients. An occasional successful substitution
does not establish identity. Base the decision on a defined product/material
distinction, rather than small differences between batches or brands.

Apply this rule to the reviewed gap candidates as follows:

| Distinction | Curation decision |
|---|---|
| Stock versus broth | Separate ingredients for each animal/vegetable base and product type; describe intended body, gelatin and seasoning where supported. Commercial names overlap, so labels alone do not establish composition or equivalence. |
| Ready-to-use stock/broth versus concentrate, cube or powder | Separate forms; dilution and seasoning can change the recipe. |
| Regular versus reduced-sodium or unsalted stock/broth | Separate formulations; retain the stated sodium qualifier and its source definition. Reduced-sodium and unsalted are distinct claims. |
| Single, double, whipping and heavy cream | Separate defined cream types when fat content or cooking/whipping behavior differs. Record locale and supported fat range in descriptions; regional names do not automatically establish aliases. |
| Full-fat, reduced-fat and nonfat dairy; salted and unsalted butter | Separate formulations where the difference changes richness, setting, cooking behavior or seasoning. |
| Fresh and dried herbs; fresh, dried and smoked peppers | Preserve the distinct material/form when flavor, strength, water content or preparation differs. |
| Named rice varieties, pasta shapes and corn/wheat tortillas | Preserve the named variety, shape or base when cooking behavior or the intended dish differs. |
| Spelling, plurals and equivalent regional names | Alias candidates only after confirming the same defined cooking material and formulation. |

Chopping, portion size, melting and similar preparation instructions remain in
authored recipe text. They do not by themselves create a new purchased-material
identity. Preserve a distinct manufactured or preserved form when that processing
changes cooking behavior. Review other qualifiers case by case rather than
creating an entry for every modifier combination.

An unspecified ingredient such as `cream` or `stock` remains unspecified; do not
infer a precise type, fat level or sodium formulation. Existing broad entries can
coexist with specific entries, with source relationships recorded conservatively.
Each new identity needs a definition, duplicate review, stable UUID and source or
curation provenance. This policy does not change existing IDs, aliases, recipe
references, or NOMS-018 quantity/substitution behavior by itself.


## Loading and maintenance

The CLI validates the complete file and manifest before connecting, then acquires
one transaction-scoped advisory lock. Entry updates, aliases, mappings, source
metadata, and the seed-run checksum record commit together. A failed run rolls
back; `--dry-run` runs the same comparison and SQL constraints but rolls back.
A repeated unchanged input does not update timestamps or add audit records.
Changed aliases and mappings replace the seed-owned sets for that entry.

Use `--file PATH` and `--sources PATH` for an explicit alternative file pair. Each
ingredient file must represent the complete maintained ingredient vocabulary.
Run one loader after schema setup as a deployment step, not per application
replica. Local Docker development does this before `dx serve`. Production runtime
credentials have SELECT access only; seed loading requires deployment credentials.
Seeds own maintained vocabulary rows; direct administrative editing is deferred.

Before an upstream update, review additions, changes, alias ambiguity, and source
notices in the dry-run. Recovery is a reviewed forward seed change with the same
IDs. Preserve entries once published; retirement is safer than deleting an ID
that may occur in recipe history. This initial slice does not auto-link typed
aliases, backfill recipes, or provide cuisine/equipment vocabulary.

## Ingredient popularity

`vocabulary_entries.recipe_count` counts distinct **current public published
recipes** that carry the ingredient's canonical ID. Repeated ingredient rows
count once. Private and unlisted recipes, drafts, immutable versions, custom text,
and unknown IDs do not affect global ranking. Visibility changes, publishing,
ingredient edits, restores, recipe deletion, and account-deletion cascades update
the count in the recipe transaction. A failed publication rolls the delta back.
Retired ingredients retain counts but remain excluded from suggestions.

The database trigger uses atomic increments in UUID order. Its fixed search path
and qualified relations allow runtime recipe writes without granting runtime
write access to vocabulary rows. An exclusive maintenance advisory lock serializes
seed loading and reconciliation with the trigger's shared lock. No popularity
values are accepted from the browser or stored in seed definitions.

For rollout to an existing database, pause recipe writes, apply `just migrate`,
reapply `migrations/runtime-grants.sql` using the deployment role, then backfill
before resuming writes:

```bash
just reconcile-vocabulary-counts --dry-run
just reconcile-vocabulary-counts
```

These commands require the deployment role's `DATABASE_URL`; runtime credentials
cannot rewrite counters. Dry-run reports how many rows differ without modifying
them. Repeating a completed reconciliation reports zero corrections. Existing
recipes are backfilled from their IDs, without changing authored text or history.
New seed entries also reconcile existing references before committing. Do not
delete or zero counts while recipes still reference the entries; use this repair
command instead. The full rebuild scans public recipes, so schedule large rebuilds
during quiet periods; ordinary recipe writes only update changed ingredient IDs.

## Verification

```bash
SQLX_OFFLINE=true cargo test --features server --lib vocabulary
SQLX_OFFLINE=true cargo test --features server --bin noms ingredient_vocabulary
SQLX_OFFLINE=true cargo test --features server --bin noms ingredient_popularity
SQLX_OFFLINE=true cargo test --features server --bin noms runtime_grants
SQLX_OFFLINE=true cargo test --no-default-features --features server --bin noms api::vocabulary_index::tests
```

Database tests require PostgreSQL tools on PATH and permission to bind a local
port. They use disposable databases, not the configured application database.
The seed test loads the first ten entries, adds the remaining entries, and checks
no-op reruns, alias matching/removal, renames, retirement, rollback, and the prefix
index. History tests cover draft saves, publication, forks, and restored versions.
Popularity tests cover duplicate rows, prefix ranking, exact-match priority,
seed preservation, reconciliation, concurrent writes, visibility transitions,
late-write rollback, ownership failures, deletion cascades, and runtime grants.
Index tests cover local/SQL matching parity, invalid cached content, atomic
catalog snapshots, popularity-only updates, seed invalidation, authentication,
conditional responses and gzip downloads.
