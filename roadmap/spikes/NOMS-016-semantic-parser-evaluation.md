# NOMS-016 semantic parser evaluation

## Outcome

The clipboard parser stack was reviewed against 27 accepted candidates from the verified external NOMS-016 corpus. The sample contains three recipes from each of nine providers: AllRecipes, BudgetBytes, Food.com, Food52, NYT Cooking, RecipeTinEats, Sally's Baking Addiction, Serious Eats, and Tasty. RecipeTinEats, Tasty, BudgetBytes, AllRecipes and Food52 were reviewed again after their source adapters were added. The AllRecipes and Food52 re-reviews on 2026-10-01 were performed by the implementation agent against the saved captures, not new human usability studies.

The purposive sample includes candidates marked both ready and needs-review, title mismatches, competing regions, grouped components, and recipes with relatively complex page structures. It is intended to expose failure modes, not estimate provider-wide accuracy.

| Result | Samples |
| --- | ---: |
| Usable without correction | 15 |
| Usable after correction | 12 |
| Unusable | 0 |

Across the four core fields, 94 of 108 reviews were correct (87.0%), 14 were partial, and none were incorrect. Sixteen candidates had all four core fields correct. No reviewed candidate was stale: every review label was checked against the SHA-256 digest of the exact parser candidate that was reviewed.

| Benchmark | Core correct | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: |
| Generic baseline | 73 of 108 (67.6%) | 5 | 20 | 2 |
| With RecipeTinEats adapter | 79 of 108 (73.1%) | 6 | 20 | 1 |
| With Tasty adapter | 84 of 108 (77.8%) | 8 | 19 | 0 |
| With Tasty compound quantities | 85 of 108 (78.7%) | 9 | 18 | 0 |
| With BudgetBytes adapter | 91 of 108 (84.3%) | 12 | 15 | 0 |
| With AllRecipes adapter | 93 of 108 (86.1%) | 14 | 13 | 0 |
| With Food52 adapter | 94 of 108 (87.0%) | 15 | 12 | 0 |

## Review rubric

Core fields use `correct`, `partial`, or `incorrect`:

- **Title:** the selected value is the recipe's semantic title.
- **Ingredients:** the selected lines are correct and complete, including section boundaries.
- **Ingredient parsing:** the amount, unit, and name split is usable.
- **Instructions:** the selected steps are correct and complete and exclude unrelated page content.

Optional fields additionally permit `not_applicable` and `not_reviewed`. The first review covered description, times, and yield. The Tasty, BudgetBytes, AllRecipes and Food52 re-reviews also covered notes/equipment and image references for their twelve candidates; those fields remain unreviewed for the other 15. Each candidate also received an overall usability label based on the reviewed fields.

## Field results

| Field | Correct | Partial | Incorrect | Not applicable | Not reviewed |
| --- | ---: | ---: | ---: | ---: | ---: |
| Title | 26 | 1 | 0 | 0 | 0 |
| Ingredients | 26 | 1 | 0 | 0 | 0 |
| Ingredient parsing | 18 | 9 | 0 | 0 | 0 |
| Instructions | 24 | 3 | 0 | 0 | 0 |
| Description | 20 | 0 | 7 | 0 | 0 |
| Times | 15 | 1 | 10 | 1 | 0 |
| Yield | 15 | 0 | 12 | 0 | 0 |
| Notes/equipment | 5 | 0 | 0 | 7 | 15 |
| Images | 12 | 0 | 0 | 0 | 15 |

## Provider results

| Provider | Reviewed | Correct core fields | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| BudgetBytes | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| Food.com | 3 | 8 of 12 (66.7%) | 0 | 3 | 0 |
| Food52 | 3 | 10 of 12 (83.3%) | 2 | 1 | 0 |
| NYT Cooking | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| RecipeTinEats | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| Sally's Baking Addiction | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| Serious Eats | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| Tasty | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |

Three samples per provider are too few for provider compatibility or ranking claims. The table is a diagnostic map for the next implementation slice only.

## Recurring findings

- Title selection was reliable in 26 of 27 reviewed candidates.
- The RecipeTinEats and Tasty adapters recovered complete named and unnamed ingredient groups in the reviewed recipes; other providers can still lose a subgroup or treat a heading as an ingredient.
- Structured ingredient splitting was the weakest core field. Checkbox and cost annotations and mixed-fraction typography commonly prevented usable amount/unit/name parsing.
- Instruction selection was generally usable. The Tasty adapter removed the provider's app promotion from each reviewed instruction list, while some other candidates still append photo credits, marketing text, or other page content.
- The RecipeTinEats, Tasty, BudgetBytes, AllRecipes and Food52 adapters recovered yield for their fifteen reviewed candidates; yield remains absent or unusable in the other 12.
- Description and time extraction were inconsistent; selected descriptions sometimes contained author or page text rather than recipe descriptions.
- No reviewed candidate remains unusable. The Tasty-specific compound-quantity rule preserves two-unit expressions in the amount field while leaving the singular unit field empty; all three reviewed Tasty candidates are usable without correction.
- BudgetBytes now preserves complete ingredient rows, structured quantities, preparation notes and footnotes while removing price annotations from parsed names. The three reviewed candidates also retain their descriptions, card servings, recipe notes and image references, and are usable without correction. Notes remain flattened into text and require mapping before saving.
- AllRecipes preserves its explicit quantity/unit/name fields and removes instructional photo credits from procedure text. Its three re-reviewed captures retain complete ingredients and steps, correct durations and servings, and applicable recipe tips/cook's notes. Their twelve core fields are correct; images were reviewed as unfetched references only.
- Food52 preserves complete explicit ingredient/direction groups, marked quantity ranges, descriptions, author and labeled metadata. Chile-Cheese Toast now retains `10 to 15` as one amount rather than placing the range suffix in the ingredient name. Toast and Gochujang Wings are usable without correction. The salmon noodle bowl still contains four unmarked section labels as low-confidence ingredient rows; those need user correction. Inferring headings from arbitrary plain list-item text is deliberately out of scope. All three preserve the correct unfetched hero reference and have no explicitly separate notes/equipment section.

The next parser work should therefore prioritize yield recovery, ingredient normalization and section completeness, and tighter instruction/description boundaries. Provider adapters or Schema.org extraction should be considered where a stable provider-specific gap can be reproduced synthetically.

## Privacy and repeatability

The 1,500 raw provider captures and the completed review manifest remain outside the repository. The committed evaluator and this report contain aggregate results, corpus identifiers, human labels, and candidate digests only; they do not contain source URLs or extracted recipe bodies.

Generate a review template for selected accepted samples:

```sh
node scripts/evaluate-recipe-parser-semantic.mjs \
  --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" \
  --data-root "/home/samko/Documents/Recipe Data" \
  --template-out "/home/samko/Documents/Recipe Data/semantic-review-v1.json" \
  --sample "Provider/Sample ID"
```

Evaluate a completed review:

```sh
node scripts/evaluate-recipe-parser-semantic.mjs \
  --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" \
  --data-root "/home/samko/Documents/Recipe Data" \
  --review "/home/samko/Documents/Recipe Data/semantic-review-v1.json" \
  --json-out /tmp/noms-016-semantic-evaluation.json \
  --markdown-out /tmp/noms-016-semantic-evaluation.md
```

Run `just recipe-import-parser-semantic-test` for the evaluator contract. A changed parser candidate has a different digest and invalidates its old labels instead of silently reusing them.

The original and AllRecipes-refreshed external review files are preserved. The current 27-sample review is `/home/samko/Documents/Recipe Data/semantic-review-food52-v1.json`; its evaluated artifacts are `/tmp/noms016-food52-semantic.json` and `/tmp/noms016-food52-semantic.md`. Only the three Food52 candidate digests and labels changed from the AllRecipes review. Use the current file with `--review` to reproduce the benchmark.

## Limitations

- Only structurally accepted candidates were reviewed; this does not measure recall across all 1,500 corpus samples.
- The 27 samples were selected purposively rather than randomly, so percentages are not population estimates.
- One reviewer supplied the labels; inter-reviewer agreement has not been measured.
- Notes/equipment and image references remain unreviewed for 15 candidates; the Tasty, BudgetBytes, AllRecipes and Food52 candidates were reviewed for these fields. Image bytes were not fetched or inspected.
- Overall usability reflects the reviewed fields and assumes a user can correct a candidate before saving.
