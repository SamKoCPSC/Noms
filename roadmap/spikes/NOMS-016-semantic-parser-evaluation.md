# NOMS-016 semantic parser evaluation

## Outcome

The clipboard parser stack was manually reviewed against 27 accepted candidates from the verified external NOMS-016 corpus. The sample contains three recipes from each of nine providers: AllRecipes, BudgetBytes, Food.com, Food52, NYT Cooking, RecipeTinEats, Sally's Baking Addiction, Serious Eats, and Tasty. RecipeTinEats and Tasty were reviewed again after their source adapters were added.

The purposive sample includes candidates marked both ready and needs-review, title mismatches, competing regions, grouped components, and recipes with relatively complex page structures. It is intended to expose failure modes, not estimate provider-wide accuracy.

| Result | Samples |
| --- | ---: |
| Usable without correction | 12 |
| Usable after correction | 15 |
| Unusable | 0 |

Across the four core fields, 91 of 108 reviews were correct (84.3%), 17 were partial, and none were incorrect. Thirteen candidates had all four core fields correct. No reviewed candidate was stale: every human label was checked against the SHA-256 digest of the exact parser candidate that was reviewed.

| Benchmark | Core correct | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: |
| Generic baseline | 73 of 108 (67.6%) | 5 | 20 | 2 |
| With RecipeTinEats adapter | 79 of 108 (73.1%) | 6 | 20 | 1 |
| With Tasty adapter | 84 of 108 (77.8%) | 8 | 19 | 0 |
| With Tasty compound quantities | 85 of 108 (78.7%) | 9 | 18 | 0 |
| With BudgetBytes adapter | 91 of 108 (84.3%) | 12 | 15 | 0 |

## Review rubric

Core fields use `correct`, `partial`, or `incorrect`:

- **Title:** the selected value is the recipe's semantic title.
- **Ingredients:** the selected lines are correct and complete, including section boundaries.
- **Ingredient parsing:** the amount, unit, and name split is usable.
- **Instructions:** the selected steps are correct and complete and exclude unrelated page content.

Optional fields additionally permit `not_applicable` and `not_reviewed`. The first review covered description, times, and yield. The Tasty and BudgetBytes re-reviews also covered notes/equipment and image references for their six candidates; those fields remain unreviewed for the other 21. Each candidate also received an overall usability label based on the reviewed fields.

## Field results

| Field | Correct | Partial | Incorrect | Not applicable | Not reviewed |
| --- | ---: | ---: | ---: | ---: | ---: |
| Title | 26 | 1 | 0 | 0 | 0 |
| Ingredients | 26 | 1 | 0 | 0 | 0 |
| Ingredient parsing | 17 | 10 | 0 | 0 | 0 |
| Instructions | 22 | 5 | 0 | 0 | 0 |
| Description | 17 | 0 | 10 | 0 | 0 |
| Times | 9 | 1 | 16 | 1 | 0 |
| Yield | 9 | 0 | 18 | 0 | 0 |
| Notes/equipment | 3 | 0 | 0 | 3 | 21 |
| Images | 6 | 0 | 0 | 0 | 21 |

## Provider results

| Provider | Reviewed | Correct core fields | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| BudgetBytes | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| Food.com | 3 | 8 of 12 (66.7%) | 0 | 3 | 0 |
| Food52 | 3 | 9 of 12 (75.0%) | 1 | 2 | 0 |
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
- The RecipeTinEats, Tasty, and BudgetBytes adapters recovered yield for their nine reviewed candidates; yield remains absent or unusable in the other 18.
- Description and time extraction were inconsistent; selected descriptions sometimes contained author or page text rather than recipe descriptions.
- No reviewed candidate remains unusable. The Tasty-specific compound-quantity rule preserves two-unit expressions in the amount field while leaving the singular unit field empty; all three reviewed Tasty candidates are usable without correction.
- BudgetBytes now preserves complete ingredient rows, structured quantities, preparation notes and footnotes while removing price annotations from parsed names. The three reviewed candidates also retain their descriptions, card servings, recipe notes and image references, and are usable without correction. Notes remain flattened into text and require mapping before saving.

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

## Limitations

- Only structurally accepted candidates were reviewed; this does not measure recall across all 1,500 corpus samples.
- The 27 samples were selected purposively rather than randomly, so percentages are not population estimates.
- One reviewer supplied the labels; inter-reviewer agreement has not been measured.
- Notes/equipment and image references remain unreviewed for 21 candidates; the three Tasty and three BudgetBytes candidates were reviewed for these fields. Image bytes were not fetched or inspected.
- Overall usability reflects the reviewed fields and assumes a user can correct a candidate before saving.
