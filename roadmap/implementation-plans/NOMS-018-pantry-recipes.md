# NOMS-018 — What can I cook?

## Delivered scope

The Pantry tab includes **What can I cook?** below its inventory and item editor. It uses the selected pantry, with no extra selector or Kitchen tab. Compact recipe cards link to live recipe details and reuse the app's image, author and cooking-time presentation. Empty results explain that ingredients can be added or marked available. Results are paginated, 12 cards at a time.

Candidates are current recipes the user owns, plus accessible recipes saved in any of their collections, including nested collections. A recipe appears once even when owned and saved in multiple collections. Saved public and unlisted recipes remain eligible; other users' private recipes, unsaved discovery recipes, unpublished drafts, and empty or malformed ingredient lists do not establish cookability.

Matching uses the same reusable Rust presence matcher as recipe detail checks, including Unicode lowercase comparison. Every authored ingredient name must have an explicitly available ingredient entry. Any available duplicate satisfies presence. Amounts, units, recorded zero quantities, servings and equipment do not affect the check. Optional-looking names remain literal requirements. Pantry and grocery storage and matching contracts are otherwise unchanged.

## Read contract and data access

`find_pantry_recipes(expected_account_id, pantry_id, pantry_revision, cursor)` derives the actor from the authenticated session. The expected account is a consistency guard. The pantry owner is enforced in the database query; collection membership and current recipe visibility are also enforced there. Anonymous, mismatched-account and cross-owner requests fail without returning recipe cards.

The endpoint returns `PantryRecipePage`: pantry ID/revision, at most 12 bounded `RecipeCardSummary` values, and an optional opaque next cursor. Cursors contain the last returned recipe's update time and UUID and are bound to the pantry ID and revision. Cursor size, shape, date, identity and revision are validated. Ordering is update time descending, then UUID descending. Recipe updates between requests follow normal live keyset-pagination behavior; page reload starts from the current library.

One read-only repeatable-read transaction loads the owner-scoped pantry, rejects a stale revision, streams candidate ingredient names, then fetches cards for the matching IDs in one batch. It never fetches full recipe bodies for cards or issues a query per recipe. The presence index is constructed once per request. The stream stops after finding an extra match to determine whether another page exists; candidate work remains under the existing database timeout and expensive-read concurrency budget. Existing owner, membership and recipe indexes serve the query. Ingredient documents above the existing 1 MiB recipe snapshot bound are excluded from this first slice; full recipe fields never cross the response boundary. The existing card size, metadata, image-ownership and serialization limits apply. No schema migration is needed.

## Freshness and failure behavior

Pantry saves and availability changes trigger fresh results. Switching pantries, refreshing inventory or changing its revision replaces the recipe read scope and resets pagination. Pending inventory/read states hide previous results. Recipe-only failures leave inventory usable and offer **Retry recipe check**, which reloads both inventory and results. Session changes hide private results and retain the existing account guards. Page reload reflects recipe edits, collection membership changes and visibility changes made elsewhere.

## Verification

Focused matcher and database tests cover ownership, library scope, nested memberships, deduplication, Unicode case matching, unknown/zero amounts, equipment exclusion, empty lists, bounded lightweight cards, cursor pagination, stale revisions, explicit stock changes, current recipe edits and visibility/membership changes.

[`scripts/tests/test-pantry-recipes-browser.py`](../../scripts/tests/test-pantry-recipes-browser.py) exercises the real authenticated endpoint and UI, pantry switching with a held read, stock-driven updates, read failures/retry, recipe-detail agreement, pagination, anonymous/cross-owner access, five viewport widths and both themes. It also captures `EXPLAIN (ANALYZE, BUFFERS)` for a library containing 300 unmatched recipes. All 22 focused pantry tests passed, including the three new database integration tests and cursor validation. Chromium passed all eight scenario groups with zero JavaScript page errors. Screenshots, `result.json` and `query-plan.txt` are under `/workspace/noms-pantry-recipes-e2e`. The measured candidate scan returned 326 library rows in 57.183 ms (local development data, one measured run); the plan used the existing recipe-owner and collection-membership indexes. This is not a production performance guarantee. The browser run uses font fallbacks for unavailable remote Google fonts.

Strict WebAssembly Clippy passed with `-D warnings`. The initial local database test attempt could not find its retained PostgreSQL ICU library; the corrected library path passed all tests. No schema migration, repository-wide test run or production deployment is claimed.

Server Clippy passed with `-D warnings -A clippy::result_large_err`, retaining the previously documented local Rust/Clippy 1.99 exclusion for unchanged `src/api/vocabulary_index.rs:45`. No source-level lint suppression was added. Rust formatting, Python syntax and patch whitespace checks passed.
