# NOMS-017 ingredient vocabulary: handoff

Started 2026-10-21; current state updated 2026-10-24.
FoodOn branch: `codex/noms-017-ingredient-vocabulary`.
Wikidata follow-up: `codex/noms-017-wikidata-adapter`.
USDA follow-up: `codex/noms-017-usda-adapter`.
Alias follow-up: `codex/noms-017-vocabulary-aliases`.
PR: https://github.com/SamKoCPSC/Noms/pull/16
Wikidata PR: https://github.com/SamKoCPSC/Noms/pull/17

## Delivered state

Ingredient autocomplete, PostgreSQL persistence, deployment-only seed loading,
source provenance and public-recipe popularity tracking are implemented. The
FoodOn baseline has **10,148 active ingredients** (100 curated plus 10,048
imported), **60 retired identities**, and 12,016 FoodOn mappings, of which 11,956
belong to active entries. The Wikidata follow-up adds 2,550 ingredients and
3,670 Wikidata mappings, producing **12,698 active entries** with the same retirements.
The USDA follow-up adds 104 ingredients and 1,246 USDA mappings, producing
**12,802 active entries and 60 retired entries** across four pinned sources.
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
held. All UUIDs, labels, original aliases/order, definitions, statuses, source
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

## Remaining scope

Other metadata dimensions, localization, further Wikidata/USDA curation, FNDDS and
Branded ingestion, nutrient linkage,
automatic recognition/backfill, vocabulary-aware portability policy, search
facets, merge redirects and personal suggestions remain future work. See the
[implementation plan](../implementation-plans/NOMS-017-recipe-metadata-autocomplete-vocabulary.md).
