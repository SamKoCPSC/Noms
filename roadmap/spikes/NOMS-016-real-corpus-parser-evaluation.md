# NOMS-016 real-corpus parser evaluation

## Scope

The current clipboard parser was evaluated against the external NOMS-016 corpus captured primarily on 2026-08-30. On 2026-09-04, two RecipeTinEats editorial pages that did not contain individual recipes were replaced with genuine recipe captures. The corpus contains 1,500 paired `text/html` and `text/plain` samples: 100 samples from each of 15 providers. The evaluator verified every recorded byte count and SHA-256 digest before parsing.

The raw provider captures remain outside the repository. The committed evaluator and generated reports retain aggregate measurements, parser classifications, counts, warning codes, and corpus sample IDs only. They do not retain extracted titles, source URLs, ingredient text, instruction text, or raw parser errors.

## Baseline structural results

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

## Region-quality rerun

After adding competing-region selection and conservative title/navigation gates, the same verified corpus was evaluated again. Structural acceptance fell from 800 to 719 candidates (47.9%). This decrease is intentional: the parser no longer reports incomplete titleless candidates as accepted.

- Accepted: 719; rejected: 781
- Invalid candidates: 0; unexpected parser errors: 0
- Accepted candidates with a title: 719 of 719, up from 720 of 800
- `ready`: 661; `needs_review`: 58
- HTML / plain-text accepted: 719 / 0
- Food Network: 0 accepted, down from 80 titleless false positives
- The two previously identified Sally's collection pages now select the embedded recipe title and return `needs_review` with the page title recorded as competing evidence

| Provider | Baseline accepted | Region-quality accepted | Needs review after rerun |
| --- | ---: | ---: | ---: |
| AllRecipes | 100 | 100 | 0 |
| BBCGoodFood | 0 | 0 | 0 |
| BonAppetit | 0 | 0 | 0 |
| BudgetBytes | 66 | 65 | 17 |
| Epicurious | 0 | 0 | 0 |
| Food.com | 90 | 90 | 0 |
| Food52 | 83 | 83 | 19 |
| FoodNetwork | 80 | 0 | 0 |
| KingArthurBaking | 0 | 0 | 0 |
| NYTCooking | 100 | 100 | 2 |
| RecipeTinEats | 19 | 19 | 5 |
| SallysBakingAddiction | 64 | 64 | 15 |
| SeriousEats | 98 | 98 | 0 |
| SimplyRecipes | 0 | 0 | 0 |
| Tasty | 100 | 100 | 0 |

The collection-page check is a targeted regression review, not a new accuracy estimate. The remaining 719 accepted candidates still require broader semantic sampling before making provider-support claims.

## Measurement-field rerun

The same 1,500 verified samples were evaluated after adding labeled durations, selected yields, and visible measurement-variant handling. Structural results remained stable at 719 accepted and 781 rejected, with no invalid candidates or unexpected parser errors.

A supplemental aggregate field audit of the 719 accepted candidates found:

- 165 candidates with prep time, 152 with cook/active time, and 250 with total time
- Duration recovery across BudgetBytes, NYT Cooking, RecipeTinEats, and Tasty
- Zero selected yields and zero inactive `data-unit` alternatives recognized in the currently accepted real samples; these behaviors are verified synthetically, but the captured provider structures need additional generic rules or adapters
- Form controls reported as discarded structural evidence on all 719 accepted candidates

A targeted duration review exposed duplicated visual/accessibility text such as a full and abbreviated time appearing together. The parser now collapses a complete, exactly repeated amount/unit sequence while retaining the complete raw source value; repeated units that are not an exact duplicate remain additive. Rechecked examples from each provider with recovered durations normalized to the displayed time rather than double-counting duplicated tokens.

## Supplementary-content rerun

The notes/equipment/image slice kept structural acceptance stable at 719 accepted and 781 rejected, with no invalid candidates or unexpected parser errors. Because any extracted note or image requires confirmation, the status mix moved from 661 ready and 58 needing review to 360 ready and 359 needing review.

Across the 719 accepted candidates, the generic rules found 58 note paragraphs in 40 candidates, 2,140 absolute HTTP(S) figure references in 338 candidates, and a first figure caption suitable for preserved attribution evidence in 108 candidates. Within individual candidates, 2,136 references had a distinct URL and four repeated another URL in the same candidate; this does not measure duplicate URLs across different candidates. No candidate contained more than 30 references. No accepted sample exposed an explicit Equipment heading/list structure matching the generic rule, so equipment extraction remains synthetically verified only. Image volume is especially high for some providers and may include multiple instructional or presentation figures; these counts establish structural reach, not semantic accuracy, and the references remain unfetched and review-gated.

## Active-content rerun

The active-content slice again kept structural acceptance stable at 719 accepted and 781 rejected, with no invalid candidates or unexpected parser errors. Status counts also remained at 360 ready and 359 needing review because discarded active-content evidence is informational rather than a mapping ambiguity.

The same corpus remained unchanged after review hardened malformed quoted raw-content openings and preserved audit evidence across unsupported and HTML-to-plain-text fallback paths.

All 719 accepted clipboard pages contained form controls and unretained HTTP(S) references, 533 contained script elements, and 391 contained iframes. The aggregate audit counted 7,666 scripts, 711 iframes, 48,213 controls, and 97,986 unretained remote-reference attributes; one candidate contained at most 326 such references. These large page-wide counts demonstrate why the parser records only inert aggregate evidence. It does not retain those URLs, instantiate a DOM, execute content, or make network requests. Preserved figure image candidates are excluded from the discarded-remote count.

## RecipeTinEats adapter rerun

The first source adapter uses the explicitly supplied RecipeTinEats hostname and visible WPRM recipe-card structure after active content has been made inert. It activates only when exactly one complete card contains a title, ingredients, and instructions; zero or multiple complete cards fall back to the generic parser. Exact-host matching prevents lookalike domains from activating the adapter.

Across the original verified corpus, the initial RecipeTinEats adapter increased structural acceptance from 19 of 100 to 81 of 100. Seventeen additional recipe cards contained valid instruction groups alongside one structurally empty WPRM placeholder group. Ignoring only those empty placeholders when another complete group remains raised RecipeTinEats acceptance to 98 of 100. The two remaining captures were a cooking-hotline article and a 20-recipe roundup, neither of which represented an individual recipe; they were archived outside the active corpus and replaced by Dirty Martini and One-pan Portuguese Chicken and Potato Bake captures. Both replacements produce valid source-adapter candidates. RecipeTinEats is therefore 100 of 100 across the corrected applicable-recipe sample, with no invalid candidates or unexpected errors. Across the current full corpus, 800 of 1,500 candidates are accepted (53.3%). The final two acceptances reflect corpus correction rather than a parser behavior change.

The adapter recovered 1,356 ingredient items and 946 instruction steps across its 100 accepted samples, including named and unnamed groups, structured WPRM amount/unit/name fields, descriptions, times when present, and servings. Each of the 17 placeholder recoveries carries `EMPTY_PROVIDER_SECTION_IGNORED`; no emitted instruction section is empty. The adapter also preserves safe image-reference and attribution evidence recognized by the shared inert extraction rules, with the same review warnings and audit accounting as generic candidates. It does not fetch resources or execute page content. These are structural reach measurements; the digest-bound semantic rerun remains unchanged at 79 of 108 correct core fields overall and 10 of 12 for RecipeTinEats, with no stale review labels.

## Tasty adapter rerun

The second source adapter uses the supplied `tasty.co` hostname and Tasty's visible recipe classes after active content has been made inert. It activates only when there is exactly one title, ingredient container, preparation container, and complete preparation list; missing or ambiguous structures fall back to the generic parser. Exact-host and lookalike-host tests enforce the same boundary as the RecipeTinEats adapter.

Tasty was already structurally accepted at 100 of 100 by generic heading rules, so the adapter does not change corpus acceptance. It instead corrects repeated completeness gaps. The generic parser retained only the first ingredient group and included the provider's app-download row as an instruction. Across the same 100 verified captures, the adapter retains all 192 visible ingredient sections, including 138 named sections, and increases ingredient recovery from 773 to 1,265 rows. It retains 743 recipe steps after excluding 99 rows that contain the exact Tasty `/download?ct=prepsteps` link; ordinary linked instruction text remains intact.

All 100 candidates include title, description, servings, total time, and one deduplicated visible recipe image. The 86 pages with separately labeled prep and cook times retain those fields, and 99 expose a visible author. Each candidate is contract-valid and review-gated because it contains an image reference, a named ingredient section, or both. Full-corpus acceptance remains 800 of 1,500, with no invalid candidates or unexpected errors; 600 candidates now use generic HTML and 200 use source adapters. No non-Tasty sample changed its structural evaluation summary.

The adapter changes the candidate digest for the three previously reviewed Tasty samples and leaves the other 24 reviewed candidate digests stable. After re-review, Tasty initially improved from 6 of 12 to 11 of 12 correct core fields. A follow-up Tasty-specific rule now preserves compound measurements such as two quantities with distinct units as one amount expression, leaving the singular unit field empty. It matched three of the 100 Tasty captures and did not change structural acceptance or any non-Tasty output. After re-reviewing the one affected benchmark candidate, Tasty reaches 12 of 12 correct core fields and all three candidates are usable without correction. The refreshed 27-sample benchmark has no stale labels and improves to 85 of 108 correct core fields (78.7%), with 9 candidates usable without correction, 18 after correction, and none unusable.

## Conclusions

1. The generic HTML parser recognizes useful candidates from several providers, but structural acceptance remains only an upper bound on compatibility.
2. Navigation-contained ingredients and titleless candidates are now rejected conservatively; this removes the observed Food Network false positives without provider-specific code.
3. Collection pages now select a coherent embedded title when heading hierarchy supports it and expose the page title as competing evidence instead of silently returning `ready`.
4. The current plain-text grammar did not recover any real HTML rejection. Its bullet and numbering assumptions should be measured against the captured text before further investment.
5. The RecipeTinEats and Tasty adapters retain conservative fallback while targeting different measured gaps: structural reach for RecipeTinEats and field completeness for Tasty. Additional adapters or Schema.org/JSON-LD extraction should use the same evidence and safety gates.

## Reproduction

```sh
node scripts/evaluate-recipe-parser-corpus.mjs \
  --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" \
  --data-root "/home/samko/Documents/Recipe Data" \
  --json-out /tmp/noms-016-real-corpus-parser-evaluation.json \
  --markdown-out /tmp/noms-016-real-corpus-parser-evaluation.md
```

Run `just recipe-import-parser-corpus-test` for the synthetic evaluator contract. The generated external report supplies deterministic sample IDs for deeper manual review without copying provider recipe bodies into the repository.
