# NOMS-016 semantic parser evaluation

## Outcome

The clipboard parser stack was reviewed against 27 accepted candidates from the verified external NOMS-016 corpus. The sample contains three recipes from each of nine providers: AllRecipes, BudgetBytes, Food.com, Food52, NYT Cooking, RecipeTinEats, Sally's Baking Addiction, Serious Eats, and Tasty. RecipeTinEats, Tasty, BudgetBytes, AllRecipes, Food52, Food.com, NYT Cooking, Serious Eats and Sally's Baking Addiction were reviewed again after their source adapters were added. The AllRecipes, Food52, Food.com, NYT, Serious Eats and Sally's re-reviews on 2026-10-01 were performed by the implementation agent against the saved captures, not new human usability studies.

The purposive sample includes candidates marked both ready and needs-review, title mismatches, competing regions, grouped components, and recipes with relatively complex page structures. It is intended to expose failure modes, not estimate provider-wide accuracy.

| Result | Samples |
| --- | ---: |
| Usable without correction | 24 |
| Usable after correction | 3 |
| Unusable | 0 |

Across the four core fields, 104 of 108 reviews were correct (96.3%), 4 were partial, and none were incorrect. Twenty-five candidates had all four core fields correct. No reviewed candidate was stale: every review label was checked against the SHA-256 digest of the exact parser candidate that was reviewed.

| Benchmark | Core correct | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: |
| Generic baseline | 73 of 108 (67.6%) | 5 | 20 | 2 |
| With RecipeTinEats adapter | 79 of 108 (73.1%) | 6 | 20 | 1 |
| With Tasty adapter | 84 of 108 (77.8%) | 8 | 19 | 0 |
| With Tasty compound quantities | 85 of 108 (78.7%) | 9 | 18 | 0 |
| With BudgetBytes adapter | 91 of 108 (84.3%) | 12 | 15 | 0 |
| With AllRecipes adapter | 93 of 108 (86.1%) | 14 | 13 | 0 |
| With Food52 adapter | 94 of 108 (87.0%) | 15 | 12 | 0 |
| With Food.com adapter | 98 of 108 (90.7%) | 18 | 9 | 0 |
| With NYT Cooking adapter | 100 of 108 (92.6%) | 20 | 7 | 0 |
| With Serious Eats adapter | 102 of 108 (94.4%) | 22 | 5 | 0 |
| With Sally's Baking Addiction adapter | 104 of 108 (96.3%) | 24 | 3 | 0 |

## Review rubric

Core fields use `correct`, `partial`, or `incorrect`:

- **Title:** the selected value is the recipe's semantic title.
- **Ingredients:** the selected lines are correct and complete, including section boundaries.
- **Ingredient parsing:** the amount, unit, and name split is usable.
- **Instructions:** the selected steps are correct and complete and exclude unrelated page content.

Optional fields additionally permit `not_applicable` and `not_reviewed`. The first review covered description, times, and yield. The Tasty, BudgetBytes, AllRecipes, Food52, Food.com, NYT, Serious Eats and Sally's re-reviews also covered notes/equipment and image references for their twenty-four candidates; those fields remain unreviewed for the other three. Each candidate also received an overall usability label based on the reviewed fields. Qualified durations and ranged/output yields count as correct when retained literally without misleading numeric inference and preserved in optional commentary; this does not claim exact numeric recovery of those values.

## Field results

| Field | Correct | Partial | Incorrect | Not applicable | Not reviewed |
| --- | ---: | ---: | ---: | ---: | ---: |
| Title | 26 | 1 | 0 | 0 | 0 |
| Ingredients | 26 | 1 | 0 | 0 | 0 |
| Ingredient parsing | 25 | 2 | 0 | 0 | 0 |
| Instructions | 27 | 0 | 0 | 0 | 0 |
| Description | 27 | 0 | 0 | 0 | 0 |
| Times | 25 | 1 | 0 | 1 | 0 |
| Yield | 27 | 0 | 0 | 0 | 0 |
| Notes/equipment | 13 | 0 | 0 | 11 | 3 |
| Images | 23 | 0 | 0 | 1 | 3 |

## Provider results

| Provider | Reviewed | Correct core fields | Without correction | After correction | Unusable |
| --- | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| BudgetBytes | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| Food.com | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| Food52 | 3 | 10 of 12 (83.3%) | 2 | 1 | 0 |
| NYT Cooking | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| RecipeTinEats | 3 | 10 of 12 (83.3%) | 1 | 2 | 0 |
| Sally's Baking Addiction | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| Serious Eats | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |
| Tasty | 3 | 12 of 12 (100.0%) | 3 | 0 | 0 |

Three samples per provider are too few for provider compatibility or ranking claims. The table is a diagnostic map for the next implementation slice only.

## Recurring findings

- Title selection was reliable in 26 of 27 reviewed candidates.
- The RecipeTinEats and Tasty adapters recovered complete named and unnamed ingredient groups in the reviewed recipes; other providers can still lose a subgroup or treat a heading as an ingredient.
- Structured ingredient splitting was the weakest core field. Checkbox and cost annotations and mixed-fraction typography commonly prevented usable amount/unit/name parsing.
- Instruction selection was generally usable at baseline, but some candidates appended photo credits, marketing text or other page content. After adapter re-reviews, all 27 reviewed instruction fields are correct; this does not establish correctness outside the selected benchmark.
- The RecipeTinEats, Tasty, BudgetBytes, AllRecipes, Food52, Food.com, NYT, Serious Eats and Sally's adapters recovered literal yield for all twenty-seven reviewed candidates.
- Description and time extraction were inconsistent; selected descriptions sometimes contained author or page text rather than recipe descriptions.
- No reviewed candidate remains unusable. The Tasty-specific compound-quantity rule preserves two-unit expressions in the amount field while leaving the singular unit field empty; all three reviewed Tasty candidates are usable without correction.
- BudgetBytes now preserves complete ingredient rows, structured quantities, preparation notes and footnotes while removing price annotations from parsed names. The three reviewed candidates also retain their descriptions, card servings, recipe notes and image references, and are usable without correction. Notes remain flattened into text and require mapping before saving.
- AllRecipes preserves its explicit quantity/unit/name fields and removes instructional photo credits from procedure text. Its three re-reviewed captures retain complete ingredients and steps, correct durations and servings, and applicable recipe tips/cook's notes. Their twelve core fields are correct; images were reviewed as unfetched references only.
- Food52 preserves complete explicit ingredient/direction groups, marked quantity ranges, descriptions, author and labeled metadata. Chile-Cheese Toast now retains `10 to 15` as one amount rather than placing the range suffix in the ingredient name. Toast and Gochujang Wings are usable without correction. The salmon noodle bowl still contains four unmarked section labels as low-confidence ingredient rows; those need user correction. Inferring headings from arbitrary plain list-item text is deliberately out of scope. All three preserve the correct unfetched hero reference and have no explicitly separate notes/equipment section.
- Food.com preserves the complete ingredients and ordered directions for the baked ziti, shake and Italian Bread candidates, including mixed fractions, bounded ranges and package-size units. All twelve core fields are correct. Descriptions, author, total time and literal yield are retained; ranged servings remain null and output counts do not replace `Serves`. The Italian Bread inline note remains in source-ordered directions, not an invented standalone notes section. None of the three sources has a separate notes/equipment section. Ziti and bread retain unfetched responsive hero references; the shake has no source hero image and is marked not applicable. All three are usable without correction.

- NYT Cooking now retains recipe descriptions rather than update timestamps, author/adaptation credit, complete ingredient/step text, literal yield, explicit recipe tips and unfetched hero references for all three re-reviewed candidates. Cookie plus/minus measures and the ziti's dual-unit ricotta measure remain literal amount expressions; package-size cans and mixed fractions split correctly. The ziti's total is 90 minutes; the two qualified total times remain literal with null numeric values, not guessed complete durations. Ranged servings and cookie output counts remain nonnumeric. All twelve core fields are correct, with no correction needed for the reviewed fields. This is an agent source-to-output review, not a claim of native clipboard or live subscription-page compatibility.

- Serious Eats now excludes instructional photo credits, read-more links and reader comments while retaining every procedure paragraph, equipment and explicit notes/storage sections for the stew, salmon bowl and cookies. All twelve core fields are correct. Their prep/cook/total labels are numeric; the cookie output count stays literal rather than becoming servings. Complete ingredient conversions and preparation text survive in names/raw rows. All three retain the proper unfetched lead-photo reference and credit and are usable without correction of the reviewed fields.

- Sally's Baking Addiction now retains complete card groups, written mixed fractions, conversion and preparation text, descriptions, author, literal yields, notes and unfetched card-photo references. The zucchini collection selects its embedded card and exposes the differing page title for confirmation. Qualified chilling time remains literal rather than guessed numeric minutes. All twelve reviewed core fields are correct and all three candidates are usable without correction of reviewed fields. These are source-to-output labels, not native clipboard or provider-wide compatibility claims.

The next parser work should therefore prioritize yield recovery, ingredient normalization and section completeness, and tighter instruction/description boundaries. Provider adapters or Schema.org extraction should be considered where a stable provider-specific gap can be reproduced synthetically.

## Simply Recipes follow-up — separate three-candidate sample

The user postponed manual Sally's verification and requested another provider. After the Simply Recipes adapter was added, the implementation agent compared three saved captures with their exact outputs: the banana pancakes, apple/cheddar oat cookies and banana bread. This is a separate purposive subset, not an expansion of the historical 27-candidate benchmark. That benchmark remains 104/108 (96.3%) with all its candidate digests current; evaluating `/tmp/noms016-sally-review.json` against the new parser confirms unchanged output. Neither Sally's nor Simply Recipes has received the pending user manual review.

All twelve Simply Recipes core fields are correct, and all three candidates are usable without correction of reviewed fields. Descriptions, labeled times and literal servings/yield are correct for all three; ranged servings stay null. Explicit notes are correct for banana bread and not applicable to the other two selected recipe cards. Inline procedure tips stay in the enclosing step, not an invented equipment section; editorial advice outside the card is not claimed as fully imported. Image references and credits match the saved lead figures; no image bytes were fetched or reviewed. Card titles replace marketing headlines in the two newer captures, preserving the different article headings as competing evidence. Complete ingredient conversion/preparation suffixes and adjacent split spans survive. Named actions, heading-only actions and procedure paragraphs retain order; exact review prompts and photo credits are excluded.

The current digest-bound subset is `/tmp/noms016-simply-review.json`; evaluated artifacts are `/tmp/noms016-simply-semantic.{json,md}`. Reproduce with `node scripts/evaluate-recipe-parser-semantic.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --review /tmp/noms016-simply-review.json --json-out /tmp/noms016-simply-semantic.json --markdown-out /tmp/noms016-simply-semantic.md`. Retain the temporary external review if needed across cleanup/restarts. Three recipes cannot establish provider-wide semantic accuracy or native clipboard compatibility.

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

The previous external review files are preserved. The current 27-sample review is temporarily `/tmp/noms016-sally-review.json`; its evaluated artifacts are `/tmp/noms016-sally-semantic.json` and `/tmp/noms016-sally-semantic.md`. Only the three Sally's candidate digests and labels changed from `/tmp/noms016-serious-review.json`; the other 24 records remain identical and current. Use the current file with `--review` to reproduce the benchmark. Retain this temporary review externally if it is needed across cleanup/restarts.

## Limitations

- Only structurally accepted candidates were reviewed; this does not measure recall across all 1,500 corpus samples.
- The 27 samples were selected purposively rather than randomly, so percentages are not population estimates.
- One reviewer supplied the labels; inter-reviewer agreement has not been measured.
- Notes/equipment and image references remain unreviewed for three candidates; the Tasty, BudgetBytes, AllRecipes, Food52, Food.com, NYT, Serious Eats and Sally's candidates were reviewed for these fields. Image bytes were not fetched or inspected.
- Overall usability reflects the reviewed fields and assumes a user can correct a candidate before saving.
