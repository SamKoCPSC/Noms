# NOMS-016 Import Candidate Contract and Synthetic Fixtures

## Outcome

`Noms recipe import candidate v1` is a preview-only boundary between untrusted clipboard input and `RecipeEditorState`. It retains the evidence needed to review uncertain extraction without persisting the copied page or silently forcing provider data into fields Noms cannot currently represent.

The executable validator is `scripts/validate-recipe-import-fixtures.mjs`. The versioned schema, manifest, invented inputs, and expected candidates live under `tests/fixtures/recipe-import/v1`.

## Candidate semantics

- `null` means that the parser did not recover a candidate for that singular field. It does not mean the source asserted an empty value.
- An empty array means that no items of that category were recovered. A supported candidate must still have at least one ingredient and one instruction; an unsupported candidate instead preserves unclassified evidence and an error warning.
- Normalized empty strings are invalid. `raw` preserves the visible source text whenever a value was recovered, even if its normalized `value` is `null` or low-confidence.
- `ready` means the candidate has the minimum recipe regions, no warning/error-severity warning, no competing region, and no low-confidence recovered value. `needs_review` means it is usable but includes a material choice or lossy mapping. `unsupported` means deterministic extraction stopped.
- Every recovered field or line carries a clipboard representation and a human-readable locator. Warnings use stable codes plus a candidate path.
- `source.provenance` has one array for each source value. A non-null URL, provider, author, or attribution requires at least one evidence entry; the corresponding array must be empty when the value is `null`.
- `competingCandidates`, `unclassifiedLines`, and `discardedContent` make selection and sanitization decisions visible without retaining the complete clipboard payload.

The raw `text/html` and `text/plain` values exist only during the active preview session. They are inputs to parsing, not part of the candidate contract or a saved recipe.

## Mapping into `RecipeEditorState`

Mapping happens only after explicit user confirmation and starts with `visibility = "private"` and no ownership claims. Before confirmation can be implemented, the canonical recipe draft, snapshot, and export contracts—and `RecipeEditorState` at their client boundary—must gain durable source URL, source author, and attribution fields. This is a hard prerequisite: Noms must not discard those values when the preview closes or force the user to save them in commentary.

| Candidate | Editor mapping |
| --- | --- |
| `title.value`, `description.value` | `title`, `description` |
| Safe integer `prepTime.value`, `cookTime.value` | `prep_time_minutes`, `cook_time_minutes` |
| Positive integer `yield.value.servings` | `servings` |
| Ingredient sections in source order | Flatten to `ingredients`; section names remain preview evidence until Noms has a section model |
| Instruction sections in source order | Flatten to recursive `instructions`; only explicit source substructure creates substeps |
| `equipment[].value` | `equipment[].name` |
| Confirmed image references | `images`; parsing itself never downloads an image |

`totalTime`, textual/range yields, section names, notes, competing candidates, warnings, and unclassified lines have no lossless current editor destination. They remain visible during review. Source URL, source author, and attribution also have no current destination, but unlike transient review evidence they must be added to the durable canonical model before import confirmation ships. Notes are not silently placed in `commentary`; that requires a deliberate user choice. If an ingredient cannot be split deterministically, its complete `raw` line becomes the ingredient name with blank amount/unit and a warning.

## Safety boundary

Clipboard HTML is untrusted. A conforming parser neutralizes resource-bearing attributes before DOM insertion or uses a proven non-fetching parser. It never renders the input, executes scripts, loads frames, submits forms, follows links, or fetches images. Scripts, styles, event handlers, frames, forms, controls, and unrelated remote resources are discarded before recipe parsing. Only normalized HTTP(S) references may enter source or image candidate fields. Input byte, node, depth, and processing-time limits remain required before production parsing.

The parser must not bypass sign-in, subscription, consent, or other access boundaries. An access shell becomes `unsupported`; the evidence and reason remain in the candidate. Saving and publishing remain separate explicit actions.

## Why these fixtures

The external characterization corpus measured 1,500 paired samples from 15 providers, with 153 structural signatures. Ingredient and instruction signals appeared in 1,498 samples, while all 1,500 contained remote-resource signals; scripts appeared in 1,215 and frames in 1,027. Provider samples also exposed grouped and duplicate regions, unit/yield controls, notes, equipment, and images at materially different rates.

The eleven fixtures reproduce those structural risk classes with invented names, values, reserved example URLs, and minimal markup. They include the navigation-list, missing-title, and collection-page failure modes found during the first semantic corpus review. They contain no copied provider page or recipe content. They are parser acceptance cases rather than compatibility claims: real-corpus evaluation remains external, and a provider adapter is justified only when generic parsing fails a stable, synthetic reproduction.

Run both fixture suites with:

```sh
just recipe-import-fixture-test
```

## Parser acceptance boundary

The spike parser must export `parseRecipeClipboard` from `scripts/recipe-import-parser.mjs`. It accepts one object containing `html`, `plainText`, and `sourceUrl`, each either a string or `null`, and returns a Noms recipe import candidate v1. An optional `providerId` selects a provider from the exported, frozen `RECIPE_IMPORT_PROVIDERS` catalogue. Omitted, null or empty provider IDs mean Automatic: retain existing URL routing or generic parsing without a URL. Unknown IDs and URLs whose hostname does not match the selected provider are rejected. The function may be synchronous or asynchronous, but it must be deterministic and must not mutate or fetch from its inputs.

An explicit provider selection can route parsing without a recipe URL; its provider provenance uses representation `provider_selection`. Routing is separate from attribution and image resolution: a catalogue origin is never saved as the recipe's source URL or used to resolve relative images. Only an actual supplied recipe URL provides that base. Absolute HTTP(S) image references remain eligible without a URL; unresolved relative references generate a review warning. If the selected adapter cannot recognize the input, existing generic fallback remains available with `SELECTED_PROVIDER_FALLBACK`. Neither selection nor an optional URL authorizes network fetching.

`just recipe-import-parser-acceptance-test` runs every synthetic input through that boundary twice, validates the result against the executable JSON Schema and semantic invariants, and compares it with the reviewed expected candidate. The target intentionally remains outside `just ci` until all acceptance slices pass; its remaining failures track unimplemented parser behavior. An implementation must derive its result from the supplied input; reading the fixture's `expected.json` is not parser behavior.

The first implementation slice is measured by `just recipe-import-parser-basic-test`. It covers the clean HTML recipe and plain-text fallback while the full acceptance target remains red for the six advanced ambiguity, grouping, media, sanitization-reporting, and unsupported-input cases.

The grouped-components slice is measured cumulatively by `just recipe-import-parser-grouped-test`. It adds ordered named ingredient and instruction sections, preserves those names in the preview candidate, and warns that the current editor cannot persist them. Five advanced acceptance cases remain intentionally red.

The region-quality slice is measured cumulatively by `just recipe-import-parser-region-test`. It enumerates coherent ingredient/instruction regions, prefers titled and tightly bounded candidates, rejects navigation ingredients and titleless candidates with preserved evidence, and marks collection pages or competing regions for review.

The measurement-variant slice is measured cumulatively by `just recipe-import-parser-variant-test`. It selects one visible `data-unit` ingredient list, retains hidden alternatives as inactive evidence, and retains multiple visible alternatives as ambiguous evidence instead of silently choosing one. It parses explicitly selected yield controls described by a wrapping label, an associated external label, or accessible select attributes; converts labeled day/hour/minute durations to integer minutes; and sends contradictory cook/active and total times to review. Complete repeated duration token sequences from visual-plus-accessibility clipboard text are collapsed while legitimate repeated units remain additive and the complete raw value remains available.

The supplementary-content slice is measured cumulatively by `just recipe-import-parser-media-test`. Within the selected recipe root it recognizes explicit Equipment and Note/Notes sections, preserves absolute HTTP(S) figure image references and optional alt/caption evidence without fetching them, and keeps notes and image mapping behind review warnings.

The active-content slice is measured cumulatively by `just recipe-import-parser-active-content-test`. It audits the original paste as inert text, makes script, style, iframe, template, and noscript bodies opaque before region parsing, reports script, iframe, form-control, and unretained HTTP(S) reference counts in deterministic order, and never fetches or executes them. Closed, unclosed, and malformed quoted raw-content openings cannot inject recipe-shaped markup. Audit evidence is attached to normal candidates, deterministic unsupported candidates, and candidates recovered through HTML-to-plain-text fallback. Preserved candidate image references are not misreported as discarded remote content. This cumulative slice and the real-corpus evaluator test run in `just ci`; only the access-shell case remains intentionally red in the full acceptance target.

`just recipe-import-parser-semantic-test` verifies the manual-review evaluator. The evaluator generates content-free review templates for explicitly selected corpus samples, binds every label to a canonical SHA-256 digest of the complete parser candidate, rejects stale labels after parser output changes, and reports aggregate core, optional-field, usability, and provider results without retaining extracted recipe bodies or source URLs. The first manually reviewed real-corpus benchmark is recorded in [the semantic parser evaluation](NOMS-016-semantic-parser-evaluation.md).

`just recipe-import-provider-adapter-test` verifies the source-adapter boundary. The RecipeTinEats adapter requires an exact supplied hostname and exactly one complete visible WPRM card after raw content is made inert. It returns the same candidate contract with `tier: source_adapter`, provider provenance, complete WPRM groups, structured ingredient evidence, times, and servings. A structurally empty instruction placeholder may be ignored only when another complete instruction group remains; the candidate records `EMPTY_PROVIDER_SECTION_IGNORED` and requires review, while all-empty or malformed groups still force fallback. Safe supplementary image and attribution evidence is extracted through the shared inert rules and reconciled into the adapter result with the existing confirmation warnings and audit accounting. Missing or ambiguous adapter structure falls back to generic parsing; lookalike domains never activate the adapter.
