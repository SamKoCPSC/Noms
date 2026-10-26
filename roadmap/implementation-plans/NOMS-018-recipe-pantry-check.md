# NOMS-018 — Recipe detail pantry check

**Status:** Presence-only checks implemented and locally validated (2026-10-26).

## Scope

The live recipe details page includes an ingredient-only, read-only **Check pantry** disclosure with a native pantry selector. Users check recipes they can view, including public/unlisted recipes and their own private recipes, against one of their own pantries.

The feature reuses authenticated, owner-scoped `list_kitchens` and `get_kitchen` endpoints and their existing account binding, request budgets and database budgets. There is no new endpoint, table, migration, write, shared pantry or persisted viability state. Anonymous visitors see a sign-in link without fetching inventory; private read scopes reset on identity changes.

## Current matching contract

The user chose presence-based checks because pantry amounts are unlikely to stay current. This supersedes the original quantity-aware recipe check.

- Match ingredient names case insensitively using Unicode lowercase comparison. No aliases, whitespace normalization, singularization or vocabulary-based equivalence is inferred. Preserve the first authored spelling for display.
- Count an ingredient as present when any same-name ingredient entry has its explicit `available` flag set. Explicitly out entries do not satisfy presence.
- Ignore all recipe and pantry amounts, units and derived `quantity_zero` flags. A recorded zero amount counts as present when the user still marks the entry available. Unknown, descriptive or insufficient-looking amounts do not change the result.
- Deduplicate recipe names across capitalization and units in their authored order. An available same-name pantry entry satisfies every occurrence of that name.
- Check every authored ingredient, including optional-looking names and textual alternatives. The schema has no structured optional/alternative flags; do not infer them from prose.
- Never count equipment as an ingredient.
- Show **All ingredients available**, **Missing ingredients**, or **No ingredients to check**. List only missing or out-of-stock ingredients, showing only each ingredient name. Available ingredients are omitted; when all ingredients are available, show the summary without an ingredient list. Do not show quantity comparisons or numeric shortages.
- Show **Quantities aren’t checked.** Explain case-insensitive name matching and equipment exclusion alongside it.
- Serving counts, recipe scaling and target-ingredient scaling do not affect pantry results. Keep the recipe scaler's existing behavior independent of the check.
- Pantry quantities remain optional, editable reference data. This adjustment does not remove or alter stored amounts, units or availability flags, and does not change inventory management or grocery-list generation.

## Loading and selection

Fetch only after opening the disclosure. Remember the selected pantry with the existing account-specific browser preference. Opening the disclosure or selecting another pantry loads saved data. Reload the page to pick up pantry edits made elsewhere; there is no dedicated refresh button. Loading, switching and failed reads hide the prior verdict; retries retain the selection. Empty inventory offers a create-pantry link; checked inventory offers an edit-pantry link.

The selection effect and pending UI explicitly observe `Resource.state()`: Dioxus `pending()` only peeks and would miss completion after an early return. Browser regressions cover automatic/remembered selection and a held pantry read without a stale verdict.

## Boundaries

Cookable recipe search, grocery shortage-list generation, quantitative sufficiency, inventory reservations/deductions, grocery-to-pantry transfer, equipment checks, inferred optional requirements and historical recipe checks remain outside this slice.

## Verification

Focused matcher tests cover ignored amounts/units/zero, explicit out flags, duplicate entries, case-insensitive names without aliases, repeated recipe names across units, equipment exclusion, all authored requirements and empty recipes. The rendered result test checks accessible status text, exclusion of available ingredients, and absence of redundant row status notes and quantity comparisons. Browser checks cover these contracts against real reads, serving-invariant results, selection, page reload/retry, ownership, public/private/unlisted recipes, anonymous access, five viewport widths, and light/dark themes.

Validation passed: 18 pantry tests (including 7 presence-matcher tests and the result-rendering test), 24 recipe-scaler tests, and 60 recipe-detail tests. Chromium passed all 8 scenario groups with zero JavaScript page errors. The browser run verifies ignored zero/unknown amounts and mismatched units, explicit availability updates, unchanged results after desktop/mobile scaling, case-insensitive names, fresh reads, retry/loading behavior, remembered selection, owner isolation and anonymous reading.

Current screenshots and `result.json` are under `/workspace/noms-recipe-pantry-case-insensitive-e2e`. Earlier missing-only screenshots with row status notes remain under `/workspace/noms-recipe-pantry-missing-only-e2e`. Screenshots before missing-only presentation remain under `/workspace/noms-recipe-pantry-no-refresh-e2e`. Presence-only screenshots before removal of the refresh control remain under `/workspace/noms-recipe-pantry-presence-e2e`. Earlier quantity-aware screenshots/results are retained separately and do not describe the current check.

Strict server and WebAssembly Clippy checks passed with `-D warnings`; formatting and patch whitespace checks passed. The full repository suite, physical devices and other browser engines are not claimed. The browser script blocks unavailable remote Google fonts and uses the app's font fallbacks. No production deployment is performed.


## Refresh control removal

Removed the dedicated refresh button to keep the menu simple. Reloading the page picks up saved pantry changes. Error-only retry and loading/selection guards remain intact. Strict server and WebAssembly Clippy, formatting and patch whitespace checks passed. The updated Chromium suite passed all 8 scenario groups with zero JavaScript page errors, including page-reload freshness, selector persistence, failed-read retry, desktop/mobile layouts and unchanged presence matching. No new schema or API changes were required.


## Missing-only presentation

The recipe pantry check lists only missing or explicitly out-of-stock ingredients. Available ingredients are omitted, and an all-available result displays its confirmation without an empty list. Matching and availability rules are unchanged. The updated rendering regression test, strict WebAssembly Clippy, formatting and patch whitespace checks passed; Chromium passed all 8 scenario groups with zero JavaScript page errors. Mixed-pantry screenshots are saved as `missing-desktop.png` and `missing-phone.png` in the current artifact directory.


## Case-insensitive matching and simpler missing rows

Ingredient names now compare using Unicode lowercase on both recipe and pantry entries. Case variants deduplicate in first-authored order and retain the first recipe spelling in the result. Explicit availability, equipment exclusion, and ignored amounts/units remain unchanged. Missing rows show only the ingredient name beneath **Missing ingredients**, without repeating a status note. The hint now explains that ingredient names ignore capitalization.

All 18 focused pantry tests passed, including mixed-case and accented-name matching, deduplication, original display spelling, out-of-stock duplicates and the rendered missing-only result. The first native run could not bind temporary database ports in the default sandbox; rerunning the compiled tests with local network access passed all 18. Chromium passed all 8 scenario groups with zero JavaScript page errors, verifying capitalization changes still match while different ingredient names remain distinct. Current desktop/mobile screenshots and `result.json` are in `/workspace/noms-recipe-pantry-case-insensitive-e2e`.

Strict WebAssembly Clippy passed with `-D warnings`; Rust formatting, Python syntax and patch whitespace checks passed. No schema or API changes were needed.
