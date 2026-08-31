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

The eight fixtures reproduce those structural risk classes with invented names, values, reserved example URLs, and minimal markup. They contain no copied provider page or recipe content. They are parser acceptance cases rather than compatibility claims: real-corpus evaluation remains external, and a provider adapter is justified only when generic parsing fails a stable, synthetic reproduction.

Run both fixture suites with:

```sh
just recipe-import-fixture-test
```

## Parser acceptance boundary

The spike parser must export `parseRecipeClipboard` from `scripts/recipe-import-parser.mjs`. It accepts one object containing `html`, `plainText`, and `sourceUrl`, each either a string or `null`, and returns a Noms recipe import candidate v1. The function may be synchronous or asynchronous, but it must be deterministic and must not mutate or fetch from its inputs.

`just recipe-import-parser-acceptance-test` runs every synthetic input through that boundary twice, validates the result against the executable JSON Schema and semantic invariants, and compares it with the reviewed expected candidate. The target intentionally remains outside `just ci` and fails with a missing-module error until the parser implementation begins. An implementation must derive its result from the supplied input; reading the fixture's `expected.json` is not parser behavior.
