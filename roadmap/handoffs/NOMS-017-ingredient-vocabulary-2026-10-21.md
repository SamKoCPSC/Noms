# NOMS-017 ingredient vocabulary: handoff

Started 2026-10-21; current state updated 2026-10-23.
FoodOn branch: `codex/noms-017-ingredient-vocabulary`.
Wikidata follow-up: `codex/noms-017-wikidata-adapter`.
PR: https://github.com/SamKoCPSC/Noms/pull/16
Wikidata PR: https://github.com/SamKoCPSC/Noms/pull/17

## Delivered state

Ingredient autocomplete, PostgreSQL persistence, deployment-only seed loading,
source provenance and public-recipe popularity tracking are implemented. The
FoodOn baseline has **10,148 active ingredients** (100 curated plus 10,048
imported), **60 retired identities**, and 12,016 FoodOn mappings, of which 11,956
belong to active entries. The Wikidata follow-up adds 2,550 ingredients and
3,670 Wikidata mappings, producing **12,698 active entries** with the same retirements.
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

The canonical bundle is validated offline; this follow-up has not been applied
to a live database. All 48 Python tests and six Rust vocabulary tests pass; the
Rust validator accepts 12,758 entries and two sources with no warnings.
The identity-pass regressions cover form/species/part
distinctions, negative scope before misleading food ancestry, legacy duplicate
preservation, maps to new imports and stable UUID reruns. Revision restoration,
review and canonical seed/source generation reproduce byte-for-byte; all prior
10,253 identities, authored wording, origins, references and mappings are preserved.

## Remaining scope

Other metadata dimensions, localization, further Wikidata curation, USDA ingestion,
automatic recognition/backfill, vocabulary-aware portability policy, search
facets, merge redirects and personal suggestions remain future work. See the
[implementation plan](../implementation-plans/NOMS-017-recipe-metadata-autocomplete-vocabulary.md).
