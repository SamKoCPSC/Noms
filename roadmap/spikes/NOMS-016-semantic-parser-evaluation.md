# NOMS-016 semantic parser evaluation

## Outcome

The generic clipboard parser was manually reviewed against 27 accepted candidates from the verified external NOMS-016 corpus. The sample contains three recipes from each of the nine providers for which the generic parser currently returns accepted candidates: AllRecipes, BudgetBytes, Food.com, Food52, NYT Cooking, RecipeTinEats, Sally's Baking Addiction, Serious Eats, and Tasty.

The purposive sample includes candidates marked both ready and needs-review, title mismatches, competing regions, grouped components, and recipes with relatively complex page structures. It is intended to expose failure modes, not estimate provider-wide accuracy.

| Result | Samples |
| --- | ---: |
| Usable without correction | 5 |
| Usable after correction | 20 |
| Unusable | 2 |

Across the four core fields, 73 of 108 reviews were correct (67.6%), 27 were partial, and 8 were incorrect. Five candidates had all four core fields correct. No reviewed candidate was stale: every human label was checked against the SHA-256 digest of the exact parser candidate that was reviewed.

## Review rubric

Core fields use `correct`, `partial`, or `incorrect`:

- **Title:** the selected value is the recipe's semantic title.
- **Ingredients:** the selected lines are correct and complete, including section boundaries.
- **Ingredient parsing:** the amount, unit, and name split is usable.
- **Instructions:** the selected steps are correct and complete and exclude unrelated page content.

Optional fields additionally permit `not_applicable` and `not_reviewed`. The first review covered description, times, and yield. Notes/equipment and images were deliberately left for a later review. Each candidate also received an overall usability label based on the reviewed fields.

## Field results

| Field | Correct | Partial | Incorrect | Not reviewed |
| --- | ---: | ---: | ---: | ---: |
| Title | 26 | 1 | 0 | 0 |
| Ingredients | 18 | 7 | 2 | 0 |
| Ingredient parsing | 11 | 10 | 6 | 0 |
| Instructions | 18 | 9 | 0 | 0 |
| Description | 11 | 0 | 16 | 0 |
| Times | 8 | 1 | 18 | 0 |
| Yield | 0 | 0 | 27 | 0 |
| Notes/equipment | 0 | 0 | 0 | 27 |
| Images | 0 | 0 | 0 | 27 |

## Provider results

| Provider | Reviewed | Correct core fields | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| BudgetBytes | 3 | 6 of 12 (50.0%) | 0 | 3 | 0 |
| Food.com | 3 | 8 of 12 (66.7%) | 0 | 3 | 0 |
| Food52 | 3 | 9 of 12 (75.0%) | 1 | 2 | 0 |
| NYT Cooking | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| RecipeTinEats | 3 | 4 of 12 (33.3%) | 0 | 2 | 1 |
| Sally's Baking Addiction | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| Serious Eats | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| Tasty | 3 | 6 of 12 (50.0%) | 0 | 2 | 1 |

Three samples per provider are too few for provider compatibility or ranking claims. The table is a diagnostic map for the next implementation slice only.

## Recurring findings

- Title selection was reliable in 26 of 27 reviewed candidates.
- Ingredient-region selection was often useful, but some grouped recipes lost a subgroup or treated a heading as an ingredient.
- Structured ingredient splitting was the weakest core field. Checkbox and cost annotations and mixed-fraction typography commonly prevented usable amount/unit/name parsing.
- Instruction selection was generally usable, but some candidates appended photo credits, marketing text, or other page content.
- Yield was absent or unusable in every reviewed candidate.
- Description and time extraction were inconsistent; selected descriptions sometimes contained author or page text rather than recipe descriptions.
- The two unusable candidates had materially incomplete ingredient selection rather than merely cosmetic normalization problems.

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
- Notes/equipment and images remain unreviewed.
- Overall usability reflects the reviewed fields and assumes a user can correct a candidate before saving.
