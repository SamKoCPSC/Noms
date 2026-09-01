# NOMS-016 real-corpus parser evaluation

## Scope

The current generic clipboard parser was evaluated against the external NOMS-016 corpus captured on 2026-08-30. The corpus contains 1,500 paired `text/html` and `text/plain` samples: 100 samples from each of 15 providers. The evaluator verified every recorded byte count and SHA-256 digest before parsing.

The raw provider captures remain outside the repository. The committed evaluator and generated reports retain aggregate measurements, parser classifications, counts, warning codes, and corpus sample IDs only. They do not retain extracted titles, source URLs, ingredient text, instruction text, or raw parser errors.

## Structural results

Structural acceptance means that the parser returned a contract-valid candidate with at least one recognized ingredient list and instruction list. It does not mean that the selected title, lists, or section boundaries were semantically correct.

- Samples: 1,500
- Structurally accepted: 800 (53.3%)
- Rejected as unrecognized: 700
- Invalid candidates: 0
- Unexpected parser errors: 0
- Accepted through generic HTML: 800
- Accepted through plain-text fallback: 0
- Accepted candidates with a title: 720
- Accepted candidates with a description: 630
- Accepted candidates marked `needs_review`: 23
- Normalized title comparison: 127 exact, 538 contained, 55 mismatched, and 80 missing

| Provider | Accepted | Acceptance | Titles present | Needs review |
| --- | ---: | ---: | ---: | ---: |
| AllRecipes | 100 | 100% | 100 | 0 |
| BBCGoodFood | 0 | 0% | 0 | 0 |
| BonAppetit | 0 | 0% | 0 | 0 |
| BudgetBytes | 66 | 66% | 66 | 0 |
| Epicurious | 0 | 0% | 0 | 0 |
| Food.com | 90 | 90% | 90 | 0 |
| Food52 | 83 | 83% | 83 | 19 |
| FoodNetwork | 80 | 80% | 0 | 2 |
| KingArthurBaking | 0 | 0% | 0 | 0 |
| NYTCooking | 100 | 100% | 100 | 2 |
| RecipeTinEats | 19 | 19% | 19 | 0 |
| SallysBakingAddiction | 64 | 64% | 64 | 0 |
| SeriousEats | 98 | 98% | 98 | 0 |
| SimplyRecipes | 0 | 0% | 0 | 0 |
| Tasty | 100 | 100% | 100 | 0 |

## Semantic spot-check

A purposive sample of 26 structurally accepted candidates was compared with its captured page title, ingredient content, and instruction content. The selection included normal title matches, title mismatches, collection-like pages, and the anomalous title-free Food Network results. This is a risk-finding exercise, not a random sample or an accuracy estimate.

| Provider | Reviewed | Appeared aligned | Unacceptable or incomplete | Observation |
| --- | ---: | ---: | ---: | --- |
| AllRecipes | 2 | 2 | 0 | Recipe title, ingredients, and instructions aligned. |
| BudgetBytes | 2 | 2 | 0 | Content aligned; copied checkbox and cost annotations remain in ingredient text. |
| Food.com | 2 | 2 | 0 | Content aligned; fraction typography differs between representations. |
| Food52 | 1 | 1 | 0 | Recipe title, ingredients, and instructions aligned. |
| FoodNetwork | 5 | 0 | 5 | Every candidate lacked a title and used navigation categories as ingredients, although recipe instructions were present. These are false-positive or incomplete candidates. |
| NYTCooking | 2 | 2 | 0 | Content aligned; title mismatches reflected alternate recipe naming. |
| RecipeTinEats | 2 | 2 | 0 | Recipe title, ingredients, and instructions aligned. |
| SallysBakingAddiction | 6 | 4 | 2 | Four individual recipe pages aligned. Two collection/listicle pages paired the collection title with one embedded recipe and are false positives. |
| SeriousEats | 2 | 2 | 0 | Recipe title, ingredients, and instructions aligned. |
| Tasty | 2 | 2 | 0 | Content aligned; title differences reflected alternate naming. |
| **Total** | **26** | **19** | **7** | Purposive findings; do not extrapolate this ratio to the corpus. |

The five providers with zero structural acceptance—BBCGoodFood, BonAppetit, Epicurious, KingArthurBaking, and SimplyRecipes—had no accepted candidates to review. Their captures are not yet supported by the current generic heading/list rules.

## Conclusions

1. The generic HTML parser already recognizes useful candidates from several providers, but 53.3% is only an upper bound on compatibility because it includes false positives.
2. Food Network demonstrates that the current heading-pair rule can combine recipe instructions with a navigation ingredient list. Candidate completeness and region scoring must improve before a provider can be called supported.
3. Collection pages can pair a page-level title with an embedded recipe. Competing-region detection should explicitly identify and warn about this shape.
4. The current plain-text grammar did not recover any real HTML rejection. Its bullet and numbering assumptions should be measured against the captured text before further investment.
5. Provider adapters or Schema.org/JSON-LD extraction are justified where stable provider structures cannot be represented by the generic visible-HTML rules.

## Reproduction

```sh
node scripts/evaluate-recipe-parser-corpus.mjs \
  --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" \
  --data-root "/home/samko/Documents/Recipe Data" \
  --json-out /tmp/noms-016-real-corpus-parser-evaluation.json \
  --markdown-out /tmp/noms-016-real-corpus-parser-evaluation.md
```

Run `just recipe-import-parser-corpus-test` for the synthetic evaluator contract. The generated external report supplies deterministic sample IDs for deeper manual review without copying provider recipe bodies into the repository.
