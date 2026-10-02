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

## BudgetBytes adapter rerun

All 100 BudgetBytes captures contain a WPRM recipe card. The adapter uses the supplied BudgetBytes hostname and one complete card to recover all ingredient and instruction groups, structured amounts and units, descriptions, authors, card servings controls, durations, notes and safe card image references. It excludes checkbox labels, surrounding promotions and editorial images. Narrow terminal dollar-price annotations are removed from parsed ingredient names while original ingredient text, preparation notes and footnote markers remain available.

Fourteen captures contain explicit parenthesized measurement conversions in 66 ingredient rows. These are preserved as one amount expression with a null singular unit, without recalculating or assuming numerical equivalence. Unparenthesized competing measurements, duplicate core fields, incomplete cards and multiple cards retain generic fallback. Notes are currently flattened to text and require mapping; image references remain unfetched and require confirmation.

BudgetBytes structural acceptance rises from 65 to 100 of 100, all through its adapter, retaining 159 ingredient sections, 1,297 ingredient rows and 764 steps. All 100 include descriptions and one card image; 94 include notes and 95 include card authors. Full-corpus acceptance rises from 800 to 835 of 1,500 (55.7%), with 535 generic HTML candidates and 300 adapter candidates, no invalid candidates and no unexpected errors. The 1,400 non-BudgetBytes parser outputs are unchanged. Twenty-six BudgetBytes card titles differ from captured page titles under the evaluator's normalized comparison; this is a review signal, not proof of incorrect card selection.

Re-reviewing the three existing BudgetBytes benchmark candidates raises their core score from 6 of 12 to 12 of 12. The refreshed 27-sample benchmark reaches 91 of 108 correct core fields (84.3%), with 12 candidates usable without correction, 15 after correction and none unusable. Only the three BudgetBytes candidate digests changed; the other 24 assessments remain current. These deliberately selected reviews diagnose gaps and do not estimate accuracy across all 100 recipes.

## AllRecipes adapter rerun — 2026-10-01

The fourth adapter requires an explicitly supplied `allrecipes.com` or `www.allrecipes.com` HTTP(S) URL and one coherent visible recipe: title, structured ingredient containers and ordered procedure in the same article/main region. Missing, competing, explicitly hidden or changed core markup refuses the adapter and retains generic fallback. Parsing remains inert and does not fetch the source page or any images.

All 100 verified AllRecipes captures select the adapter and validate. It retains 808 ingredients across 103 sections and 564 steps, including mixed Unicode fractions, package-size units and a narrowly recovered unmarked numeric prefix. A single-item nested procedure is retained with a review warning. Direct recipe paragraphs remain ordered; photo credits, ads and explicitly marked related-article promotions are excluded. Unknown direct procedure text triggers fallback rather than silently disappearing.

All 100 candidates recover descriptions, authors, prep time, total time and servings; 96 expose cook time. The adapter preserves 18 cook's-note/recipe-tip bodies and 488 deduplicated image references, including lazy `data-src` figures. References remain unfetched and are not saved by the current UI. Scaled selections retain literal ingredient amounts but clear inferred servings and require review.

Full-corpus structural acceptance remains 835/1,500, with zero invalid candidates or unexpected errors. Every one of the 1,400 non-AllRecipes parser outputs was compared directly with the pre-adapter parser and remained identical. The three existing AllRecipes benchmark captures were re-reviewed against their saved source content: core fields improve from 10/12 to 12/12 correct and all three are usable without correction. The complete purposive benchmark reaches 93/108 correct core fields (86.1%), with 14 usable without correction and 13 after correction. These results are not a 100-recipe semantic-accuracy claim.

Validation: `node --test scripts/tests/test-recipe-import-allrecipes.mjs` passes 14 acceptance/defensive tests; the provider, cumulative parser, corpus/semantic evaluator, fixture and clipboard bridge targets pass. WASM/server checks, both Clippy targets and the server suite pass (883 tests, 12 ignored). `just ci` initially stopped because the advisory cache was read-only; rerunning `just security-check` with cache access passed, including existing allowed dependency warnings. Independent review of `1edf407..edc0ce1` passes after fixing hidden-content and silent-procedure-loss findings.

The browser smoke test uses the saved Banana Banana Bread HTML/plain-text pair on the isolated NOMS-016 instance at port 32773. Preview preserves seven structured ingredients (including `2 ⅓` cups), six clean steps, 15 prep minutes, 60 cook minutes, twelve servings and author context. Explicit confirmation opens a private draft in the existing editor; image references are disclosed but not imported. Editing the test draft's title autosaves and survives direct reload with its private visibility and imported fields intact. It remains unpublished in the local mock test account. An incremental-build dependency now tracks JavaScript parser changes in Rust so an old asset fingerprint cannot survive an adapter-only source update. An uncached reload was necessary to replace the browser's old running bundle.

Raw reports remain `/tmp/noms016-post-allrecipes.json` and `/tmp/noms016-post-allrecipes.md`. Digest-bound review reproduction is described in the semantic report; raw provider captures remain outside the repository.

## Food52 adapter rerun — 2026-10-01

The fifth adapter requires an explicitly supplied `food52.com` or `www.food52.com` HTTP(S) URL, one visible main/article with its recipe header, and coherent Ingredients/Directions containers. It accounts for every marked recipe list and step label across the article: additional unconsumed groups, navigation-contained core sections, hidden required content, incomplete steps and conflicting mapped duration labels refuse the adapter. Generic fallback remains available; parsing never fetches or executes source content.

All 100 verified Food52 captures select the adapter and validate, improving structural acceptance from 83/100 to 100/100. It retains 1,423 ingredient rows across 136 groups and 619 steps across 117 direction groups. Marked mixed fractions, quantity ranges and unfamiliar units are preserved literally. Plain unmarked rows are retained with low confidence, not guessed into headings or equipment. Seventy samples carry that warning; two contain unfamiliar marked measurement expressions. One capture has empty ingredient-group headings: the complete groups remain unnamed and carry a review warning.

All 100 candidates preserve authors, literal yield and one unfetched hero reference; 96 contain a recipe description, 87 prep time and 82 cook time. Numeric servings are inferred only from a positive integer yield. Total time is not invented. Hidden hero/credit content is not imported. Sponsorship and video copy are excluded from author and recipe core extraction.

Full-corpus structural acceptance rises from 835/1,500 to 852/1,500, with zero invalid candidates or unexpected parser errors. Every one of the 1,400 non-Food52 parser results remains identical to the pre-adapter output. This is structural coverage, not a claim that all 100 recipes are semantically correct.

The three existing Food52 benchmark captures were re-reviewed against their saved sources. The adapter fixes the Chile-Cheese Toast quantity range and recovers complete groups and optional metadata. Toast and Gochujang Wings are usable without correction. The salmon noodle bowl retains four unmarked section labels as low-confidence ingredient rows and still needs correction. Food52 improves from 9/12 to 10/12 correct core fields; the overall purposive benchmark reaches 94/108 (87.0%), with 15 usable without correction, 12 after correction and none unusable. Only the three Food52 candidate digests changed; all 24 other reviews remain current. See the semantic report for the refreshed external review path.

The browser smoke test uses the saved Gochujang Wings HTML/plain-text pair and a new local mock account on the isolated NOMS-016 instance at port 32773. Full clipboard paste produces the complete Wings/Sauce preview, 12 structured ingredients, five clean steps, 15 prep minutes, 25 cook minutes, four servings and author context. Explicit confirmation opens a private draft; group labels survive in optional commentary and images remain references only. Renaming the test draft autosaves and survives reload with all imported fields and private visibility intact. The draft remains unpublished. Screenshot: `/tmp/noms016-food52-ui.png`.

Validation: `node --test scripts/tests/test-recipe-import-food52.mjs` passes 15 acceptance/defensive tests. The provider, bridge, cumulative active-content parser, fixture, corpus evaluator and semantic evaluator targets pass. The schema-plan and security-policy targets and `just security-check` pass, with the repository's existing allowed dependency warnings. Independent review passes after four reproduced findings were fixed: unconsumed recipe-group loss, navigation containment, hidden optional images/credits and duplicate mapped duration aliases.

`just fmt check lint` passes for both WASM and server targets. Full `just ci` did not finish: the unchanged `db::tests::runtime_grants_enforce_a_non_owning_data_plane_role` test hung. A bounded full server rerun with `PGCONNECT_TIMEOUT=5 SQLX_OFFLINE=true timeout 180 cargo test --features server -- --test-threads=8` also timed out in that test. Running `SQLX_OFFLINE=true timeout 180 cargo test --features server -- --test-threads=8 --skip db::tests::runtime_grants_enforce_a_non_owning_data_plane_role` passes 882 tests, with 12 ignored and that one test explicitly filtered. No test was edited or disabled in the repository; the full CI gate remains unverified until the database-test hang is resolved. The other CI targets were run separately and passed.

Raw aggregate reports remain `/tmp/noms016-food52-final.json` and `/tmp/noms016-food52-final.md`. The external corpus and recipe bodies are not committed. Implementation commits: `12ab415` (adapter and acceptance tests), `e656eb9` (review fixes and regression tests).

## Food.com adapter rerun — 2026-10-01

The sixth adapter requires a supplied `food.com` or `www.food.com` HTTP(S) URL and one complete visible `main#recipe.recipe`, with structured Ingredients/Directions lists. It accounts for every marked recipe row and explicit group heading inside that main. Missing, hidden, competing, unclosed or unaccounted core structures refuse the adapter and preserve generic fallback. Explicitly hidden ancestors are excluded even when their closing tags are missing. Active content remains opaque; no source page or image is fetched or executed.

All 100 verified Food.com captures select the adapter and validate, improving structural acceptance from 90/100 to 100/100. The adapter retains 869 ingredient rows across 110 groups and 656 steps across 104 groups. It preserves marked mixed fractions, bounded ranges, package-size units, alternatives and explicit group headings. Directions remain source-ordered, including inline recipe notes. Unknown marked amounts are retained literally with low confidence and a review warning; the current corpus contains none of that warning.

The candidates contain 98 descriptions, 100 authors, 99 total-time values and 100 literal yields. Compact `Ready In` values are parsed as total time, not guessed into prep/cook time; unsupported duration expressions remain unclassified evidence with a warning. Only 60 positive integer `Serves` values become numeric servings. Ranges and output counts such as loaves or cups remain literal yield text. Thirty-nine captures expose 78 unfetched hero URL references; distinct responsive URLs are retained, while avatars and other-image thumbnails are excluded. This is reference recovery, not image-byte validation or import.

Full-corpus structural acceptance rises from 852/1,500 to 862/1,500 (57.5%), with zero invalid candidates or unexpected errors. Direct deep comparison against the pre-Food.com parser confirms all 1,400 other-provider outputs are unchanged. Forty-five Food.com candidates require review; the remaining 55 have no parser review warning. Neither status establishes semantic correctness.

The three existing Food.com benchmark candidates were re-reviewed against their saved captures by the implementation agent. All core ingredients/directions are complete and the quantity splits are usable. Food.com improves from 8/12 to 12/12 correct core fields, with all three usable without correction. The overall purposive benchmark reaches 98/108 (90.7%), with 18 usable without correction, 9 after correction and none unusable. Only the three Food.com review records changed; all 24 others remain identical and digest-current. This is not a 100-recipe semantic-accuracy claim or a new human usability study.

The browser smoke test uses the saved Italian Bread HTML/plain-text pair on the isolated NOMS-016 app at port 32773 and a local mock test account. Full clipboard paste produces 10 structured ingredients, 31 steps, six servings, description and author context. The first amount remains the literal `5 1/2 - 6` range. Prep/cook time remain missing; the original total-time and two-loaf yield text survive in optional commentary. Explicit confirmation creates a private draft. Renaming the test draft autosaves and survives direct reload with all imported fields and private visibility intact. It remains unpublished; images are disclosed as references and not imported. Screenshot: `/tmp/noms016-foodcom-ui.png`.

Validation commands and outcomes:

- `node --test scripts/tests/test-recipe-import-foodcom.mjs`: 14 acceptance/defensive tests pass. The initial tests failed for the expected unsupported provider behavior before implementation; new review regressions also failed before their fixes.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including all 64 provider tests.
- `just fmt check lint`: pass for WASM and server targets.
- `just schema-check-test security-policy-test` and `just security-check`: pass; the security check retains existing allowed dependency warnings and required access to its advisory cache.
- `SQLX_OFFLINE=true timeout 180 cargo test --features server -- --test-threads=8 --skip db::tests::runtime_grants_enforce_a_non_owning_data_plane_role`: 882 pass, 12 ignored, one explicitly filtered. The known full-CI database-test hang described in the Food52 section remains unresolved; this turn did not rerun or claim a successful full CI gate. No repository test was disabled or weakened.
- Corpus evaluator with the external manifest/data-root: 862 accepted, zero invalid/error candidates and integrity verified; semantic evaluator with `semantic-review-foodcom-v1.json`: 98/108 correct core fields, no stale labels.

Independent review passes after fixing compact-duration partial parsing/evidence loss, unclosed hidden ancestors and competing main openings, with synthetic regressions for each. Implementation commits: `954b0fb` (adapter and acceptance tests), `d7a5b12` (duration/hidden-root review fixes), `bbc61a1` (malformed competing main count).

Raw aggregate reports remain `/tmp/noms016-foodcom-verified.json` and `/tmp/noms016-foodcom-verified.md`. The semantic report documents the current external review and its reproduction. Raw provider captures remain outside the repository. Work remains on branch `codex/NOMS-016-recipe-import-export` in the canonical `/home/samko/GitRepos/Noms/.worktrees/NOMS-016-recipe-import-export`; no additional worktree was created because the user requested consolidation.

## Food.com primary-recipe and description follow-up — 2026-10-01

The user's Shredded Chicken Sandwiches check exposed a copied page containing two complete recipe mains. The adapter previously refused multiple mains, allowing generic region scoring to select the appended barbecue-sandwich recipe and omit its div-based description. Food.com navigation was observed temporarily retaining both roots, with only the original marked `main#recipe`.

The adapter now selects that unique primary marker when additional roots are unmarked, disjoint `main.recipe` siblings. Fields are read only from the primary, and ignored siblings produce a review warning. Duplicate primary IDs, unknown/id-bearing roots, nested roots (including nesting between secondary roots), unclosed mains and malformed or hidden primary content still refuse the adapter. Its description extraction removes only recognized checkbox-backed `See More`/`Less` toggle labels, retaining similarly worded prose and unknown controls.

Validation:

- `node --test scripts/tests/test-recipe-import-foodcom.mjs`: 18 pass. Primary selection and description-control regressions failed before the fix; the independent-review nesting regression also failed before its repair. Existing tests remain intact.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including 68 provider tests.
- `just fmt check lint`: pass for WASM and server. Full CI/server tests were not rerun for this JavaScript-only follow-up; the previously documented database-test hang remains a limitation.
- Full external-corpus evaluation: still 862/1,500 accepted, all 100 Food.com samples accepted, integrity verified, zero invalid candidates or unexpected errors. Deep comparison to `faa49a1` leaves 1,466 outputs identical; the other 34 change only Food.com descriptions to remove toggle labels. All other fields and all 1,400 non-Food.com outputs remain identical.
- The existing 27 semantic-review records remain digest-current without relabeling: 98/108 correct core fields and all three Food.com samples usable without correction. Aggregate artifacts: `/tmp/noms016-foodcom-primary-corpus.{json,md}` and `/tmp/noms016-foodcom-primary-semantic.{json,md}`.
- A fresh isolated instance built from the canonical NOMS-016 worktree at port 32783 previews the complete live Shredded Chicken Sandwiches page with its description, six ingredients and four steps. The browser test supplied DOM-derived HTML and visible text through the session clipboard; it does **not** establish native OS Ctrl+A/C fidelity. The native clipboard bridge was unavailable in this browser session. The preview retains source/author, total time and literal `10-12` servings without inferring a numeric serving count. No draft was created or published. Screenshot: `/tmp/noms016-foodcom-description-fix-ui.png`.

A constructed two-root regression payload combines the live markup of the reported recipe and its appended barbecue-sandwich recipe, leaving only the first marked `#recipe`. The baseline selects the barbecue recipe with no description; the fixed parser and fresh UI select Shredded Chicken Sandwiches with its description and an appended-recipes warning. This is a live-markup-derived test payload, not a claim to have captured the user's original OS clipboard.

Independent code review passes after the nested-secondary-root finding was fixed. The canonical branch/worktree is unchanged; unrelated generated CSS was not included in the fix.

## NYT Cooking adapter — 2026-10-01

Following the user's Food.com verification, the exact-host `cooking.nytimes.com` adapter now uses stable visible component-name prefixes rather than build-specific CSS hashes. It requires one complete main/recipe root, accounts for every marked core component, preserves explicit ingredient groups and sequential steps, and excludes known grocery controls, step media and reader notes. Unmatched roots and required markers hidden inside otherwise ignored regions are refused rather than silently omitted. Missing or changed structure retains generic fallback, not a partial authoritative provider result. All committed recipe copy in its fixtures is invented; saved source bodies remain external.

The full integrity-verified corpus still accepts 862/1,500 candidates, including 100/100 NYT captures, with no invalid candidates or unexpected errors. Structural reach was already 100/100 under the generic parser; this slice repairs field extraction rather than increasing that count. Deep comparison against `05df7ac` leaves all 1,400 non-NYT candidates/rejections identical. Source-to-candidate checks account for all 1,098 ingredient rows, 435 complete step texts and 17 recipe-tip rows. The captures contain 103 ingredient groups, including two multi-group recipes. Structural accounting is not a semantic-accuracy claim for every field of every capture.

The adapter recovers recipe descriptions instead of update timestamps, authors plus adaptation/source credits, literal yield and safe unfetched hero URL references. It preserves mixed fractions, package-size units and compound plus/minus or conversion measurements without conversion guesses. Exact total times are numeric in 79 captures; 21 qualified/ranged total times remain literal with null numeric values. Fifteen prep times and 17 cook times are exact; one additional ranged cook time remains literal. Numeric servings require an exact positive integer serving count. Image bytes are neither fetched nor imported.

Validation and evidence:

- `node --test scripts/tests/test-recipe-import-nyt.mjs`: 18 pass. Initial adapter cases failed before implementation; three hardening regressions failed before their fixes. Independent review passes after unmatched-root and consumed-marker accounting repairs.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including 86 provider tests.
- `just fmt check lint`: WASM/server checks and strict lint pass. `SQLX_OFFLINE=true cargo test --features server recipe_import_mapping -- --nocapture`: 8 pass. Full CI/server tests were not rerun; the previously documented database-test hang remains a limitation.
- Corpus reports: `/tmp/noms016-nyt-corpus.{json,md}`. Full deep-comparison and source-accounting evidence: `/tmp/noms016-nyt-validation.txt` (external source-derived content, not committed).
- Re-review of the three NYT semantic candidates fixes compound quantity splits, descriptions, yield and the fractional-hour duration. Qualified times intentionally remain literal and are available in optional commentary. NYT scores 12/12 correct core fields; the full 27-sample benchmark is 100/108 (92.6%), with 20 usable without correction, 7 after correction and no stale labels or unusable candidates. Image review concerns references only. The refreshed review and reports are `/tmp/noms016-nyt-review.json` and `/tmp/noms016-nyt-semantic.{json,md}`; all 24 other-provider records remain unchanged. These temporary artifacts should be retained externally if needed across cleanup/restarts.
- Browser runtime proof uses an invented recipe with provider-shaped HTML plus plain text through the browser session clipboard. On port 32783 it previews the correct description, author, compound and package quantities, three ingredients, two clean steps, numeric prep/cook/servings and literal qualified total time. Creating a private draft and reloading preserves these values and commentary. Nothing was published. The existing canonical instance was restarted, then reloaded without cached assets after the first preview exposed an older asset. A fresh concurrent instance was refused by the one-instance-per-worktree guard. Native OS Ctrl+A/C and live NYT access were not tested; no NYT page/headline screenshots were taken.

An additional UI paste containing only an access-shell message is rejected as unrecognizable and offers no draft-creation action. A clean import form is left open for manual testing. The synthetic private draft remains in the isolated development instance; it was not published.

Changes remain in the canonical NOMS-016 branch/worktree. Unrelated generated CSS is excluded. The next step is the user's manual copy/paste review against an accessible NYT recipe, without fetching or bypassing access controls in the parser.

## Serious Eats adapter — 2026-10-01

Following the user's NYT Cooking verification, the exact-host Serious Eats visible-article adapter was added. It recognizes one ordered header/ingredients/directions structure inside one main/article, accounts for all marked required regions, and refuses hidden, malformed, competing or unconsumed core markup. It requires the supplied HTTP(S) `seriouseats.com` or `www.seriouseats.com` URL; it does not fetch it. Only invented recipe bodies were added as fixtures.

All 100 saved Serious Eats captures select the adapter, up from 98 generic acceptances. Source-to-candidate accounting preserves all 1,300 complete ingredient rows and 571 complete step texts. This includes multi-span conversion quantities, named ingredient groups, day-by-day phases, a single-item nested procedure and supplements nested in a final step. Equipment and paragraph/list notes remain separate from directions; step-photo credits, ads, related links and reader comments are excluded. The integrity-verified full corpus accepts 864/1,500, with zero invalid candidates or unexpected errors. Deep comparison with `d0991a6` leaves all 1,400 non-Serious-Eats outcomes identical. Structural accounting is not a claim that every field in all 100 captures is semantically correct.

Description and author come from explicit header signals, not editorial biography or reviewer names. Card time/yield labels are retained; numeric durations require exact supported text and numeric servings require a positive integer serving count. Distinct conflicting `Total`/`Total Time` labels preserve both raw values with null numeric total and a warning, rather than arbitrarily choosing one. Unknown metadata remains unclassified review evidence. Only the first visible intro figure supplies safe, unfetched image references; hidden image nodes/ancestors are excluded. Images are not imported.

Validation and evidence:

- `node --test scripts/tests/test-recipe-import-serious-eats.mjs`: 21 pass. Initial cases failed 11/15 before implementation; five additional metadata/list/review regressions and an unmatched-marker regression failed before their fixes. Independent review: PASS after fixing dropped marked content inside ignored media/ads, hidden void images and malformed optional metadata.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including 107 provider tests.
- `just fmt check lint`: WASM/server checks and strict lint pass. `SQLX_OFFLINE=true cargo test --features server recipe_import_mapping -- --nocapture`: 8 pass. Full CI/database tests were not rerun; the previously documented database-test hang remains a limitation.
- `node scripts/evaluate-recipe-parser-corpus.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --json-out /tmp/noms016-serious-corpus-final.json --markdown-out /tmp/noms016-serious-corpus-final.md`: pass. Raw aggregate reports remain temporary/external. `/tmp/noms016-serious-validation.mjs` and `/tmp/noms016-serious-validation.json` record full outcome equality and source-row/step accounting.
- Re-review against the saved captures gives all three Serious Eats benchmark candidates 12/12 correct core fields and usable-without-correction labels. Full benchmark: 102/108 (94.4%), 22 without correction, 5 after correction, no stale labels or unusable candidates. Only these three records changed from the prior NYT review; the other 24 remain identical. Current temporary review is `/tmp/noms016-serious-review.json`. Reproduce with `node scripts/evaluate-recipe-parser-semantic.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --review /tmp/noms016-serious-review.json --json-out /tmp/noms016-serious-semantic.json --markdown-out /tmp/noms016-serious-semantic.md`. These are implementation-agent source-to-output reviews, not population estimates or human usability studies.
- Browser proof on port 32783 uses invented provider-shaped HTML/plain text via the session clipboard and a fresh local mock account. Preview retains the description, author, three structured ingredients (fraction, compound and package amount), two clean steps, 15 prep minutes, 65 cook minutes, four servings, equipment and list notes. Creating a private draft, renaming it, waiting for Saved and reloading preserves all fields and optional source/group/time/yield commentary. The synthetic draft remains private and unpublished. An access-shell paste is rejected and has no draft confirmation. The canonical instance was restarted and uncached-reloaded for the changed parser asset; an unresponsive old tab was replaced with a fresh background test tab. Native OS Ctrl+A/C and live Serious Eats access were not tested.

The clean import form is available for the user's manual Serious Eats check. Changes remain in the canonical NOMS-016 branch/worktree; unrelated generated CSS is excluded. Temporary review manifests should be retained externally if needed after cleanup/restarts.

## Sally's Baking Addiction adapter — 2026-10-01

Following the user's Serious Eats verification, the exact-host Sally's visible-card adapter was added. It requires a supplied HTTP(S) bare/www `sallysbakingaddiction.com` URL and one complete recipe card in one main region. It neither fetches the page nor inserts untrusted markup into a DOM. Only invented recipe bodies were committed as fixtures.

All 100 saved captures activate the adapter, compared with 64 generic acceptances. Full source-to-candidate accounting preserves all 1,260 ingredient rows across 175 groups, 808 complete instruction texts and 656 notes. Written `and` fractions, Unicode fractions, bounded ranges, package units, conversions and compound measures retain literal information without consulting checkbox/scaling attributes. Explicit descriptions, author, time/yield labels and safe lazy image references are retained. Qualified durations remain nonnumeric; output counts do not become servings. Special-tools text remains in notes rather than an inferred equipment list. Twenty-one captures expose a differing surrounding page title as competing evidence, not as the selected recipe title.

Four initial fallback layouts were reproduced synthetically: two leaf DIV descriptions, one empty editor list and one sequence of empty editor headings/lists between complete groups. Narrow handling now preserves these sources. Named groups cannot lose their only empty list, nonempty placeholders cannot be ignored, and unknown required structural content refuses the adapter. Independent review found that text beside a required body/header could disappear. A failing regression preceded strict child accounting; the reviewer independently confirmed the fix and returned PASS with no remaining blocking or should-fix findings.

Validation and evidence:

- `node --test scripts/tests/test-recipe-import-sallys.mjs`: 21 pass. The initial 18 cases failed 12/18 before implementation; the required-region and two later layout regressions failed before their fixes.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including 128 provider tests.
- `just fmt check lint`: WASM/server checks and strict lint pass. `SQLX_OFFLINE=true cargo test --features server recipe_import_mapping -- --nocapture`: 8 pass. Full CI/database tests were not rerun; the previously documented database-test hang remains a limitation.
- `node scripts/evaluate-recipe-parser-corpus.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --json-out /tmp/noms016-sally-corpus.json --markdown-out /tmp/noms016-sally-corpus.md`: integrity verified, 900/1,500 structurally accepted, zero invalid candidates or unexpected errors. `/tmp/noms016-sally-validation.mjs` and `/tmp/noms016-sally-validation.json` assert raw ingredient/step/note equality and deep equality of every one of the 1,400 other-provider outcomes against `190d46d`. Structural retention is not semantic verification of every field in all 100 captures.
- The three source-to-output Sally's benchmark re-reviews cover all fields, including notes and unfetched image references. All 12 core fields are correct and all three are usable without correction of reviewed fields. The full purposive benchmark reaches 104/108 (96.3%), with 24 without correction, 3 after correction and no stale labels. Only these three records changed; the other 24 are identical. Reproduce with `node scripts/evaluate-recipe-parser-semantic.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --review /tmp/noms016-sally-review.json --json-out /tmp/noms016-sally-semantic.json --markdown-out /tmp/noms016-sally-semantic.md`. These temporary external artifacts should be retained if needed after cleanup/restarts.
- Browser proof on port 32783 uses invented provider-shaped HTML/plain text through the session clipboard and a fresh local mock identity. Preview retains description, author, three structured ingredients across two groups, package/mixed/compound measures, two complete steps, 15 prep minutes, literal 12-bar yield and a source note. Creating a private draft, renaming it, waiting for Saved and reloading preserves all fields and optional group/source/yield commentary. The draft remains private and unpublished. The canonical instance was restarted and uncached-reloaded for the changed parser asset. An access-shell paste is rejected without a draft-creation button. Native OS Ctrl+A/C and live Sally's access were not tested.

The clean import form is left open for the user's manual Sally's review. Changes remain in the canonical NOMS-016 branch/worktree; unrelated generated CSS is excluded. Saved captures do not establish ongoing website compatibility, and plain-text-only pastes still use the generic best-effort parser.

## Simply Recipes adapter — 2026-10-01

The user postponed manual Sally's verification and requested the next provider. Simply Recipes was selected from the remaining saved corpus. It shares measured Dotdash article/card markup with Serious Eats, so the existing extraction routine was made explicitly dual-host rather than duplicated. All Simply-only title, procedure and metadata extensions are gated by the exact HTTP(S) bare/www `simplyrecipes.com` hostname; Serious Eats output remains unchanged. Parsing still performs no fetch, DOM insertion, image loading or access-control bypass. Only invented fixtures were committed.

All 100 saved Simply Recipes captures select the adapter, up from zero generic acceptances. Full source accounting retains all 985 ingredient rows across 114 groups, 442 complete steps, 11 inline callouts and 65 explicit notes. Thirty exact observed review prompts are excluded; any extra cooking text prevents that exact match. Forty-one differing card/article titles are exposed as competing evidence. Explicit card titles take precedence, while the article heading remains the fallback when no card title exists. Named step headings are retained inline, including complete heading-only actions. Inline tips retain their position in the enclosing step; explicit trailing Recipe Notes and card Note paragraphs remain notes. Ordinary editorial sections, reader comments and step-photo captions do not become procedure text. Inline span adjacency, conversions, fractions, compound measures, jar/block package units and singular pinch measures are preserved. Qualified durations and ranged/output yield remain literal rather than guessed numeric values.

Review found three should-fix metadata gaps: detached marked notes could disappear, nutrition controls could conceal cooking text, and a metadata body nested inside the card title contaminated that title. Tests failed before strict direct-child/marker ownership and control checks fixed these cases. Corpus profiling also exposed heading-only instructions; a failing test preceded support for those source-positioned actions. Further regressions cover text-bearing wake controls/ads, sentence-heading punctuation and singular pinch units. The independent reviewer repeated the metadata probes and returned PASS. All 1,400 other-provider outcomes deep-compare identically with `69ac75c`, including all saved Serious Eats outputs.

Validation and evidence:

- `node --test scripts/tests/test-recipe-import-simply-recipes.mjs`: 22 pass. Initial 16 cases failed 12/16 before implementation; four later structural/retention cases and the punctuation/pinch cases failed before their fixes.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including 150 provider tests. Raw output: `/tmp/noms016-simply-gates.txt`.
- `just fmt check lint`: WASM/server checks and strict lint pass. `SQLX_OFFLINE=true cargo test --features server recipe_import_mapping -- --nocapture`: 8 pass. Outputs: `/tmp/noms016-simply-rust-gates.txt` and `/tmp/noms016-simply-mapping-tests.txt`. Full CI/database tests were not rerun; the previously recorded database-test hang remains a limitation.
- `node scripts/evaluate-recipe-parser-corpus.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --json-out /tmp/noms016-simply-corpus.json --markdown-out /tmp/noms016-simply-corpus.md`: integrity verified, 1,000/1,500 structurally accepted, zero invalid candidates or unexpected errors. `/tmp/noms016-simply-validation.mjs` and `/tmp/noms016-simply-validation.json` assert complete row/step/note equality and all 1,400 other outcomes against the baseline. This is structural retention, not semantic verification of every field across the provider.
- A separate three-candidate source-to-output review covers title, ingredients/splits, instructions, description, time, literal yield, explicit notes and unfetched image/credit references. All 12 core fields are correct and all three are usable without correction of reviewed fields. These labels are agent assessments, not human usability verification or population accuracy. The previous 27-candidate benchmark remains unchanged and digest-current. Reproduce the new subset with `node scripts/evaluate-recipe-parser-semantic.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --review /tmp/noms016-simply-review.json --json-out /tmp/noms016-simply-semantic.json --markdown-out /tmp/noms016-simply-semantic.md`. The source comparison is `/tmp/noms016-simply-spotcheck.txt` (external source-derived content, not committed). Retain temporary manifests externally if needed across cleanup/restarts.
- Browser proof on port 32783 uses invented provider-shaped HTML/plain text via the browser session clipboard and a fresh mock identity. Preview selects the explicit card title and exposes the different article title, keeps mixed fractions, a package jar, singular pinch, a named step with an inline tip, a heading-only action, two notes, 15 prep minutes and four explicit servings. Confirming a private draft, renaming it, waiting for Saved and reloading preserves these values and optional source/group/yield commentary. The draft remains private and unpublished. An access-shell paste is rejected without draft confirmation. The canonical instance was restarted and uncached-reloaded for the parser asset; no live provider page or native OS Ctrl+A/C was tested.

The clean import form is available for later manual testing. Sally's and Simply Recipes both remain pending the user's manual review. Plain-text-only paste still uses generic best-effort parsing; website layout changes may require further synthetic fixtures. Changes remain in the existing canonical NOMS-016 branch/worktree, honoring the user's earlier consolidation request. Unrelated generated CSS is excluded.

## Food Network, Bon Appétit and Epicurious sequential rollout — 2026-10-01

After the user confirmed Sally's and Simply Recipes working, the requested adapters were implemented and committed sequentially: Food Network (`a149ad2`), Bon Appétit (`aa531ca`), then Epicurious (`b6981cc`). Each requires its exact HTTP(S) bare/www hostname and one complete, visible owned recipe structure. Plain-text-only paste retains the generic best-effort path. No source fetching, raw HTML execution, remote image downloading or access restriction bypass is added.

| Provider | Activated saved captures | Retained ingredient rows | Retained steps | Ingredient groups | Notes | Unfetched image references |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Food Network | 100/100 | 1,122 | 418 | 128 | 29 | 82 |
| Bon Appétit | 100/100 | 1,203 | 508 | 125 | 73 | 100 |
| Epicurious | 100/100 | 1,123 | 432 | 124 | 47 | 100 |

Food Network accounts for checkbox labels, explicit ingredient/direction phases, chef notes/warnings, identical responsive metadata copies and the measured extended article's explicit card title. `Active` does not become guessed prep/cook time; supplementary method yields remain distinct preview metadata. Bon Appétit reconstructs quantity/description sibling pairs and actual numbered steps inside list-item groups, keeps coauthors distinct from photography, and separates variations, make-ahead and editor's notes from the last action. Nutrition wrappers sharing the ingredient test ID remain metadata, not ingredients. One photo-only numbered step remains the literal `Step 10`, low-confidence and flagged for source review. Epicurious instead has complete ingredient text in each `Description` row; dotted units and compound quantities remain literal without numeric arithmetic. A numbered make-ahead action remains a step; an unnumbered trailing make-ahead paragraph becomes a note. One empty video-teaser shell is recognized narrowly, never arbitrary cooking content inside a DIV.

Validation evidence:

- `node --test scripts/tests/test-recipe-import-foodnetwork.mjs scripts/tests/test-recipe-import-bonappetit.mjs scripts/tests/test-recipe-import-epicurious.mjs`: 60 focused tests pass (18/25/17). The initial tests failed before each adapter, and measured variants/review findings were fixed tests-first. Independent reviews pass after repairing unaccounted cooking text, metadata disagreement, navigation-derived optional/core fields, unsafe image-only evidence, inherited hostname keys and 21 understated Epicurious compound quantities. Source-derived captures remain external; only invented fixtures are committed.
- `just recipe-import-provider-adapter-test recipe-import-bridge-test recipe-import-parser-active-content-test recipe-import-fixture-test recipe-import-parser-corpus-test recipe-import-parser-semantic-test`: pass, including all 210 provider tests. `/tmp/noms016-wave3-gates.txt` retains raw outcomes. `just fmt check lint` passes WASM/server checks and strict Clippy; `SQLX_OFFLINE=true cargo test --features server recipe_import_mapping -- --nocapture` passes eight tests. Outputs are `/tmp/noms016-wave3-rust-gates.txt` and `/tmp/noms016-wave3-mapping-tests.txt`. Full CI/database tests were not rerun.
- `node scripts/evaluate-recipe-parser-corpus.mjs --manifest "/home/samko/Documents/Recipe Data/corpus-manifest.json" --json-out /tmp/noms016-wave3-corpus.json --markdown-out /tmp/noms016-wave3-corpus.md`: integrity verified, 1,300/1,500 structurally accepted, 200 unrecognized, zero invalid candidates or unexpected errors. Remaining unrecognized providers are BBC Good Food and King Arthur Baking, outside this request. The evaluator's HTML/Text columns do not count source-adapter candidates; zero there does not mean clipboard HTML was absent.
- `node /tmp/noms016-wave3-validate.mjs Wave3 4ed8a7d` writes `/tmp/noms016-wave3-validation.json`: all 300 requested samples activate the intended adapter; all 3,448 ingredient rows, 1,358 steps, explicit groups and owned procedure paragraphs are accounted for. The other 1,200 complete outcomes, including rejection outcomes, are deeply identical to the pre-rollout parser. Sequential checkpoints also compared each adapter's 1,400 other outcomes with its immediate baseline. This is retention, not semantic correctness of all normalized fields.
- Browser-session rich paste → preview → explicit private draft → title autosave → direct reload succeeds for all three on port 32783 using invented provider-shaped content and an isolated mock identity. Mixed fractions, dotted units, compound expressions, package-size literals, groups, make-ahead/editorial notes, exact/missing time and serving fields, and consented source/yield/total-time commentary survive mapping. Each access-shell paste is rejected with no draft-confirmation button; no test draft was published. The canonical instance is reused to honor the one-instance-per-worktree guard and earlier consolidation request. A transient truncated development asset copied during ongoing edits was resolved by restarting with stable source and checking the rebuilt module's syntax; uncached browser verification was followed by restoring normal cache behavior.

Manual review still matters: an unmarked Food Network checkbox row that looks like a section heading remains literal, and some package expressions remain in ingredient names rather than being guessed into units. The Bon Appétit photo-only step needs source inspection. Lead images are unfetched references only; `Active`, supplementary yields and nutrition are currently preview-only unclassified content, not persisted commentary. Source-to-output spot checks and digest-bound assessments are described in the semantic report, separately from the historical benchmark. These three providers are ready for the user's manual live/native-copy checks; this rollout does not establish fidelity for every contemporary provider layout.

The separate nine-candidate independent agent semantic review reports 28/36 correct core fields, with three usable without correction and six after correction, none unusable. In addition to package splitting and the photo-only step, Lasagna retains a source-marked cookbook attribution as an extra instruction and Pizza Dough misses a footer photo credit. These require manual cleanup; no provider-wide semantic-accuracy claim is made. The historical 27-candidate and separate three-candidate Simply Recipes digest-bound benchmarks are unchanged. See the semantic report for exact labels and reproduction commands.

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
