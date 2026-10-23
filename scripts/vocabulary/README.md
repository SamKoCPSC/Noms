# FoodOn → Noms seed adapter

For Wikidata revision capture, complete name screening and canonical ingestion,
see the [Wikidata adapter guide](WIKIDATA.md).
For pinned USDA Foundation Foods and SR Legacy ingestion and name review,
see the [USDA adapter guide](USDA.md).

Requires Python 3.11+ and the standard library. This is offline ingestion tooling,
independent of autocomplete and server startup. It consumes a local RDF/XML OWL
snapshot and emits review candidates or complete files in Noms' version-1 seed
format. It does not fetch OWL imports, modify the maintained seed, or write a DB.
The separate Rust validation/loader commands use the existing server toolchain.

The pin in `data/vocabulary/adapters/foodon.json` identifies the same FoodOn
artifact as the coverage review: commit, release, full-file SHA-256, byte length,
source license and attribution. Ontology version IRI is also checked. A different
download fails rather than silently producing candidates from another version.

## Extract the pilot and source catalog

Obtain the pinned `source.url` as a local file using the environment's normal
network permissions and TLS verification. Raw OWL and generated catalogs belong
outside Git. Then, from the repository root:

```bash
just foodon-adapter extract \
  --owl /path/to/foodon.owl \
  --output /tmp/noms-foodon-review
```

The fresh output directory contains:

- `candidates.json`: named FOODON classes, parent IDs and external parent IRIs, English labels and
  definitions, synonyms separated by scope, canonical wording proposals, original
  literal/resource annotations and synonym/definition axiom annotations.
- `review.json`: the 50 usable FoodOn benchmark matches proposed against the
  existing canonical IDs, **all unapproved**, with no assumed mapping relationship.
- `report.json`: extraction counts and pilot summary.

The catalog retains ineligible terms with flags, including obsolete classes,
organism/source categories outside the food-material hierarchy, and missing or
ambiguous English labels. Technical FoodEx2 wording is flagged for editorial
review rather than stripped mechanically. Named subclass and intersection paths
are followed; restriction fillers are never mistaken for parent ingredients.
This is structural RDF extraction, not an OWL reasoner or an assertion that every
descendant is suitable for the Noms product. Remote imports are never resolved.

Full-snapshot validation found three reviewed edible materials outside that
named hierarchy: dried konbu, annatto seed and teff kernel. The pinned descriptor
lists these individually in `reviewed_ingredient_terms`, with the scope evidence
for each. Their outside-hierarchy flags and original parent IRIs remain visible.
This admits those reviewed materials without admitting the whole organism or
plant-anatomy branches. Exceptions cannot revive deprecated terms or admit the
root category. Scope changes invalidate existing review approvals.

`canonical_proposal` uses an English label, normalized/deduplicated **exact**
English synonyms as aliases, and the first English definition (empty if absent).
Display spelling is retained; normalization is only for comparisons. Broad,
narrow, related and non-English synonyms stay in the evidence catalog and never
become aliases automatically. Overlong/invalid text requires editing, not silent
truncation. Exact synonyms can still be culinarily ambiguous: review them.

## Review mappings and new entries

The pre-expansion audit mechanically screens all 20,152 structurally eligible
terms from 29,239 named classes. It flags 5,999 classification labels, 1,154 terms
with ontology wording, and seven proposals that violate the canonical text
contract. There are 346 shared normalized label/alias strings across 587 terms;
10,303 terms lack an English definition. These overlapping signals are review
hints, not accepted/rejected ingredient counts or semantic approval.

The 54 source-specific
[editorial findings](../../data/vocabulary/adapters/foodon-label-review.json)
cover broad categories, animal-feed/hazard branches, ambiguous aliases, source
text errors and meaningful ingredient forms. They remain inputs to the audit
and cooking selection; they are not adapter import approvals or UUID allocations.
Preserve distinctions such as active/inactive yeast, dried/canned tomatoes,
peppercorns/Capsicum, allspice/pimiento and caper buds/fruit. A friendlier label
cannot repair a source definition or establish exact equivalence.

Reproduce the complete review queue from an extracted catalog:

```bash
python3 scripts/vocabulary/audit_foodon_labels.py \
  --catalog /path/to/extracted/candidates.json \
  --output /tmp/noms-foodon-label-audit
```

The fresh output includes all eligible candidates with reason flags, shared
normalized names, canonical-name overlaps, diagnostic variant families and input
checksums, plus a Markdown report containing the detailed findings. Keep these
generated reports and the full CSV/catalog outside Git. No flag automatically accepts,
rejects, renames or merges an ingredient. Preparation, missing definitions and
unfamiliar names alone do not establish unsuitable ingredients.

For each pilot mapping, choose `exact`, `broader`, `narrower` or `related`, document
the judgment in `note`, and set `approved` to the JSON boolean `true` when ready.
The relationship describes the **FoodOn term relative to the Noms ingredient**:
`broader` means the source term covers more than that canonical ingredient;
`narrower` means it describes a more specific part/form. Coverage grade E is
usable discovery coverage and does not establish exact equivalence.

`action: "map"` changes only source mappings/references on an existing canonical
ID. It preserves the ID, label, aliases, description, locale, status and curated
origin. It never adopts FoodOn's display wording into existing curated entries.

To add an ingredient beyond the pilot, select an eligible term from the catalog
and append an import decision to `review.json`:

```json
{
  "approved": true,
  "action": "import",
  "term_id": "FOODON_00000000",
  "canonical_id": "<new UUIDv4 assigned once and saved in this review>",
  "relationship": "exact",
  "canonical": {
    "label": "Reviewed culinary label",
    "aliases": ["Reviewed alias"],
    "description": "Reviewed culinary description."
  },
  "note": "Why this source concept and culinary wording identify this ingredient."
}
```

Use a real selected term ID. Assign a UUIDv4 once (for example with
`python3 -c 'import uuid; print(uuid.uuid4())'`) and persist it in the review;
never derive identities from names or allocate fresh IDs during every build.
`canonical` may override any of label/aliases/description. Omitted fields use the
candidate's proposed wording, only after explicit approval. New entries use
`locale: "en"`, `status: "active"`, imported provenance and an exact mapping.
A persisted staging identity can instead be retained with
`canonical.status: "retired"` and an editorial reason in `note`. This preserves
its UUID/source evidence while hiding it from suggestions; it is never deleted.

Distinct canonical identities are never merged through matching strings. The
adapter rejects assigning the same exact source identity to two canonical IDs,
while explicitly reviewed broader/narrower/related mappings can overlap. Alias
collisions are reported and remain distinct, matching the seed validator's
warning behavior. In particular, do not promote generic shrimp-paste synonyms
into equivalence between belacan and bagoong alamang, or a plant into an edible
part. Review the source and per-term annotations before reusing upstream text;
the FoodOn descriptor carries CC-BY-4.0 attribution, not a blanket license claim
for every referenced third-party definition.

The review is bound to the ontology checksum, the complete adapter descriptor,
and both base input file checksums.
If those inputs change, regenerate and reconcile the decisions rather than using
stale approvals. A source ID already bound to different manifest metadata is
rejected; a new source release needs a reviewed descriptor/source-ID decision.

## Build, validate and preview loading

```bash
just foodon-adapter build \
  --owl /path/to/foodon.owl \
  --review /tmp/noms-foodon-review/review.json \
  --output /tmp/noms-foodon-seed

just seed-vocabulary --validate-only \
  --file /tmp/noms-foodon-seed/ingredients.json \
  --sources /tmp/noms-foodon-seed/sources.json
```

`build` reparses/verifies the OWL file rather than trusting an edited catalog.
It emits the **complete** seed, retaining every existing entry and retirement,
plus approved imports and source mappings. The merged source manifest and an
audit report accompany it. The report includes collision warnings, pending
decisions, selected source terms and their original attribution annotations.
Identical inputs and persisted decisions produce byte-identical output. Output
directories must be fresh; the tool cannot overwrite an existing output bundle
or the maintained data files.

`--validate-only` uses Noms' Rust canonical validator without `DATABASE_URL` or
a database connection. After inspecting the generated diff and audit report,
the existing deployment-role loader can preview changes against an already
migrated development DB:

```bash
just seed-vocabulary --dry-run \
  --file /tmp/noms-foodon-seed/ingredients.json \
  --sources /tmp/noms-foodon-seed/sources.json
```

The normal reviewed seed-file rollout applies after that preview. The adapter
itself never promotes files into the maintained seed or applies them to a DB.
Custom base seed/source files and a different reviewed descriptor can be passed
through `--seed`, `--sources` and `--descriptor` to both commands.

## Reviewed 100-entry pilot

`data/vocabulary/adapters/foodon-pilot-review.json` contains the completed review
of the 50 FoodOn benchmark candidates: 28 exact, 8 broader, 12 narrower and 2
related mappings, each with its rationale. For example, the FoodEx2 soy-sauce
category includes tamari, while the Noms ingredient excludes it, so that mapping
is broader. Saffron's source definition says anther rather than stigma; taro's
definition says root rather than corm. Those two retain related mappings without
adopting the source definitions or asserting anatomical equivalence.

Reproduce the prepared pilot using the same pinned OWL file and base seed:

```bash
just foodon-adapter build \
  --owl /path/to/foodon.owl \
  --seed data/vocabulary/adapters/foodon-cooking-base-ingredients.json \
  --sources data/vocabulary/adapters/foodon-cooking-base-sources.json \
  --review data/vocabulary/adapters/foodon-pilot-review.json \
  --output /tmp/noms-foodon-pilot
just seed-vocabulary --validate-only \
  --file /tmp/noms-foodon-pilot/ingredients.json \
  --sources /tmp/noms-foodon-pilot/sources.json
```

Full-snapshot validation extracted 29,239 named FoodOn classes and 20,152 eligible
terms including the three explicit scope exceptions. Both the 100-entry mapping
pilot and a separate new-entry import probe passed the actual Rust seed CLI
without a database connection. Repeated pilot builds were byte-identical;
all 100 existing IDs, labels, aliases, descriptions, locales and statuses were
preserved. That pilot initially left maintained seed files unchanged. It was
followed by the cooking-focused expansion below, which is now the maintained
seed and is loaded into the disposable development database.

## Cooking-focused maintained seed

The full-catalog selection supplies **10,148 active entries** (100 curated plus
10,048 imported), with 60 staged UUIDs retained retired. All 101 prior development
identities, including the cumin probe, are retained. Original authored wording and
pilot mapping relationships are unchanged. The source manifest pins FoodOn and
the complete seed carries 12,016 mappings, of which 11,956 belong to active entries.

The selection is a **policy-screened batch**, not individual semantic verification
of every source annotation. It includes regional foods and named varieties as
well as ordinary pantry ingredients. It holds taxonomy/physical-state/dosage
categories, feed, medical formulations, industrial additives, complete meals,
unclear source text and incidental variants out of suggestions. Named FoodEx2
commodities can supply useful labels after removing classification presentation;
their source codes and original annotations remain in the generated report.

The exact selection rules live in `prepare_foodon_cooking.py`, with source-specific
exceptions and reviewed wording/aliases in
`data/vocabulary/adapters/foodon-cooking-policy.json`. New aliases are explicit
editorial choices; source exact synonyms are not bulk copied. Shared display
families/plural spellings and maintained-name matches supply conservative related
source mappings, not asserted exact source equivalence. Unresolved synonym/species
scope differences may still need later editorial consolidation.

Reproduce from the checksum-pinned OWL/catalog:

```bash
python3 scripts/vocabulary/prepare_foodon_cooking.py \
  --catalog /path/to/extracted/candidates.json \
  --output /tmp/noms-cooking-selection
python3 scripts/vocabulary/foodon.py build \
  --owl /path/to/foodon.owl \
  --seed data/vocabulary/adapters/foodon-cooking-base-ingredients.json \
  --sources data/vocabulary/adapters/foodon-cooking-base-sources.json \
  --review /tmp/noms-cooking-selection/review.json \
  --output /tmp/noms-cooking-seed
just seed-vocabulary --validate-only \
  --file /tmp/noms-cooking-seed/ingredients.json \
  --sources /tmp/noms-cooking-seed/sources.json
```

The immutable baseline files preserve the original 100-entry seed from commit
`aee6b6ae2ad4e3f5eed7e59e8b82d63971d2139d`. The default UUID registry is the
versioned `foodon-cooking-review.json`; reruns reuse its UUIDv4 assignments.
Previously staged identities removed by later selection changes remain retired,
with their mapping evidence, rather than disappearing. The selection command emits
`summary.json` with disposition counts and the policy checksum, alongside
`review.json` and the full `selection.json` queue in the fresh output directory.
Keep the generated summary, selection queue and term-annotation report outside
Git; maintain the approved review/UUID ledger and policy in the repository.
Promoted seed files reproduce
from the saved review; do not regenerate random identities on each run.

The fixed 100-ingredient source-coverage benchmark and database test fixtures use
the preserved baseline independently of catalog growth. Coverage percentages are
not claims about the expanded seed.

## Check a loaded bundle in the UI

The adapter remains command-line ingestion tooling. Autocomplete already reads
the canonical database, so loading a reviewed bundle makes its active entries
available in the existing recipe editor without a separate UI import step.
Source mappings/provenance are stored in the database; suggestion options show
only the culinary label.

After loading a complete bundle into a ready **disposable local** stack, run:

```bash
python3 scripts/vocabulary/browser-smoke.py \
  --instance noms-noms017 \
  --seed /path/to/loaded-bundle/ingredients.json \
  --entry-id <imported-entry-UUID> \
  --query <unique-alias> \
  --chromium /usr/bin/chromium \
  --output /tmp/noms-foodon-browser
```

This requires Python Playwright and Chromium as described in the
[cloud development guide](../../docker/codex-cloud/README.md). It signs in through
the instance's loopback mock OAuth server, creates a private draft, checks alias
lookup, label-only options, keyboard/mouse selection, the saved canonical UUID,
reload persistence, UUID clearing on edits, and free-text saving while lookup
requests fail. It records screenshots/results outside the checkout and leaves
the private draft available for inspection. It never publishes a recipe.

The 2026-10-22 live check used `FOODON_00003396` (cumin seed; alias `cumin`) with
persisted probe UUID `1745729e-d12c-454d-92cd-1c448f08dd23`. Loading the 100-entry
pilot updated 50 entries and one source; loading it again changed nothing.
Both the pilot and new-entry dry-runs left all five vocabulary tables unchanged.
The extra probe produced 101 entries and 51 mappings. Chromium passed all checks
with no page errors; a direct database read confirmed the private draft's UUID.

The cooking-focused bundle has now been promoted into the maintained seed, which
includes the cumin UUID and all staged retirements. Development startup therefore
uses the complete bundle. The loader still requires subsequent bundles to retain
every database UUID; loading only the original 100-entry baseline is rejected.

For the expanded-catalog checks, pass
`--quality-checks scripts/vocabulary/cooking-lookup-checks.json` to the browser
script. Its 61 authenticated API cases cover common ingredients, culinary aliases,
Unicode/whitespace normalization, result caps, literal wildcard handling and held
categories. These checks reuse the signed-in browser context without saving its
cookies. Timings describe local authenticated HTTP lookups and exclude the UI's
200 ms debounce; they are not a concurrent-load benchmark.

## Verification

```bash
just foodon-adapter-test
```

The offline fixtures exercise RDF hierarchy versus restriction semantics,
language/synonym scope, annotations and licensing evidence, pin/version checks,
approval binding, canonical ID preservation, new imports, deterministic reruns,
alias warnings, and rejection of obsolete/organism and conflicting exact maps.
CI runs this suite without downloading the ontology. A full-snapshot extraction
is a separate integration check requiring the pinned local OWL file.
