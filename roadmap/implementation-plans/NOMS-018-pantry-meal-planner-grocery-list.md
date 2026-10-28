# NOMS-018: Kitchen Workspace, Pantries, Grocery Lists & Recipe Feasibility — Implementation Plan

**Status:** Grocery lists merged in [PR #21](https://github.com/SamKoCPSC/Noms/pull/21); pantry inventory implemented and locally validated with W01/C01. Production deployment is not verified here.
**Created:** 2026-08-15
**Updated:** 2026-10-25
**First delivery:** Multiple private, database-backed grocery lists; manual-only or generated as independent snapshots from selected recipes
**Selected UI:** Workspace 2 — context inspector, and Create list 10 — compact form.
**Implemented preview:** [Grocery list workspace](../assets/NOMS-018/implementation/workspace.png). Earlier exploratory HTML galleries remain local design artifacts.
**Depends on:** Existing published recipe content, recipe access rules, authentication, Dioxus routes, and declarative PostgreSQL schema. Kitchen profiles and NOMS-017 are not prerequisites for Slice 1.
**Coordination:** NOMS-017 owns vocabulary suggestions and optional references, not quantitative compatibility. NOMS-016 ingredient text must remain usable without conversion. NOMS-022 supplies existing mobile navigation, viewport, and touch-target conventions.

## Delivery sequence

1. **Slice 1 — Independent grocery lists (merged in PR #21):** Deliver saved-list management, compact creation, recipe ingredient snapshots, manual items, shopping progress, and optional recipe context.
2. **Slice 2 — Pantry inventory (merged in PR #25):** multiple named pantries, ingredients with optional authored quantities/units, equipment, and explicit availability. See the [pantry inventory implementation plan](NOMS-018-pantry-inventory.md) and its selected W01/C01 UI.
3. **Workspace unification:** [Kitchen workspace](NOMS-018-kitchen-workspace.md) combines pantry and grocery navigation. Kitchen names the whole workspace; pantry names each inventory profile. Existing lists stay independent.
4. **Slice 3 — Recipe feasibility and shortages:** start with an ingredient-only [recipe detail pantry check](NOMS-018-recipe-pantry-check.md), using case-insensitive ingredient names and explicit availability against one pantry. Amounts, units and servings do not affect this first check. The [What can I cook? section](NOMS-018-pantry-recipes.md) now checks owned and collected recipes within the Pantry tab. Shortage-list generation follows later.
5. **Later — Shopping enhancements:** explicit shortage-to-list generation, regeneration/reconciliation, offline synchronization, and deliberate purchased-stock updates. Define those contracts before implementation.

The current Kitchen/pantry terminology above supersedes the older kitchen-profile wording in historical sections below. The broader product scope remains recorded after the Slice 1 plan. Its pantry, equipment, and offline acceptance scenarios are not release gates for Slice 1. All saved lists remain independent: editing a source recipe never silently changes list contents or check-offs.

## Slice 1: decisions confirmed with the user

- Use **Workspace 2**: multiple saved lists in the left sidebar, a shopping checklist, and a contextual recipe contribution inspector. On narrow screens, keep the checklist usable and show details in an accessible responsive panel rather than squeezing three columns together.
- Use **Create list 10**: a compact form with list name, a manual/recipe starting choice, item entry, and an optional recipe picker with serving counts. The two choices are entry paths, not permanent incompatible list types.
- Support a complete list with **no recipes**. No kitchen profile, pantry entry, meal plan, or vocabulary match is required. Allow non-food items such as dish soap.
- Persist a list as a database row and each item as its own row. Persist names, quantities/descriptive amounts, ordering, and checked state. Saved lists can be reopened on another authenticated device.
- Support multiple private, user-owned lists, including manual lists, lists copied from recipes, and recipe lists with manual extras.
- Create recipe-derived lists as **unlinked snapshots**. Preserve enough copied source context to explain each generated amount without making the list depend on the continued existence of the recipe.
- Allow selection from **all recipes the signed-in user can currently access**, including their own private recipes and other users’ readable public/unlisted recipes under the existing access rules. Do not limit selection to the user’s recipe book.
- Account for each selected recipe’s **target servings relative to its base servings** before combining requirements. Apply scaling to parseable numeric amounts in any authored unit, not only grams.
- Use **naive exact matching of ingredient name and unit** for aggregation. For example, `Rice: 1 cup` plus `Rice: 2 cup` becomes `Rice: 3 cup`; `cup` and `cups`, or `g` and `kg`, remain separate. No unit conversion, alias resolution, singularization, case folding, or substitution is required.
- Allow **full item editing**, including name, amount, and unit, for both generated and manual rows. Users may add or remove any items. Editing a generated row changes only the list, not the source recipe.
- **Copy every authored ingredient** from the selected recipes. Do not infer exclusions for optional ingredients or choose among textual alternatives. Users can remove unwanted items and add more after generation.
- Keep pantry management, recipe feasibility, equipment shortages, offline synchronization, and live recipe synchronization out of this first slice.

These decisions supersede the earlier grams-only grocery proposal and the recommendation to keep generated rows read-only. The pantry plan was subsequently updated to use the same exact ingredient-name/unit compatibility rule, with optional amounts and explicit uncertainty.

The HTML prototypes use browser storage and sample data. Their numeric limits, default servings, storage messages, and controls demonstrate a design; they are not approved database/API contracts. Production integration must use the real session, database, shared components, and actual request failure states.

## Slice 1: current repository baseline

| Area | Existing evidence | Implementation implication |
|---|---|---|
| Ingredients and servings | [`RecipeIngredient` and `Recipe`](../../src/types.rs): `amount`, `unit`, and `name` are strings; `servings` is `Option<i32>`. [`recipes`](../../migrations/schema.sql) stores ingredient JSONB and nullable integer servings. | Keep arbitrary authored units and textual amounts. Scale numeric quantities using valid source/target servings; handle missing base servings explicitly. |
| Existing calculator | [`recipe_scaler.rs`](../../src/utils/recipe_scaler.rs) parses into `f64`; its display formatter takes absolute values, truncates to two decimals, and renders sufficiently small amounts as empty. | Its display output is unsuitable as the authoritative grocery aggregation/storage contract. Add bounded, validated grocery arithmetic without changing the existing recipe calculator incidentally. |
| Recipe snapshots | [`RecipeSnapshot`](../../src/db/diff.rs) and [`recipe_versions`](../../src/db/versioning.rs) preserve recipe content; published recipe versions cascade when their parent recipe is deleted. | Copy the grocery-specific attribution and amounts into list-owned data. A live recipe/version FK must not delete list contents or be required to render them. |
| Recipe access | [`get_recipe`](../../src/api/recipe.rs) checks ownership, then permitted public/unlisted access. | Recheck source access server-side at creation; knowing a recipe ID is not sufficient authority. The picker must cover all permitted sources, not just ownership; use existing discovery rules and resolve supplied accessible recipe links/IDs without introducing unlisted enumeration. |
| Authenticated mutations | [`collection.rs`](../../src/api/collection.rs) and recipe mutations use server session identity, input bounds, request budgets, and account-change checks where applicable. | Follow Dioxus server-function conventions and existing protections; do not introduce a client-supplied owner ID or a separate REST/auth stack. |
| Database delivery | [`migrations/schema.sql`](../../migrations/schema.sql), [`justfile`](../../justfile), and [`test_utils.rs`](../../src/test_utils.rs) use declarative `pgschema`, explicit SQLx cache preparation, and isolated PostgreSQL tests. Grocery list and pantry inventory tables now exist in the checked-in schema; both follow this declarative workflow. | Add schema using the established workflow, update test-schema coverage, review the generated plan, and refresh `.sqlx` only where compile-time queries change. |
| Historical design | [`DESIGN.md`](../../DESIGN.md), “Meal Planner & Shopping Lists,” contains earlier pantry-linked example SQL. | Treat it as historical design, not an existing migration or the selected first-release model. |

## Slice 1: implemented contracts

The following contracts implement the user-confirmed decisions. Minor technical choices were delegated by the user and are now recorded in the decision register. The HTML prototypes remain design references; production code uses server persistence.

### Storage and list lifecycle

- Add `grocery_lists` and `grocery_list_items` using the declarative schema. No pantry/profile foreign key is required.
- A list has a UUID, session-derived owner, nonblank name, optional note, timestamps, revision, a create-request key/payload fingerprint, and list-owned source snapshots. Name limit: 100 characters, consistent with existing collection naming.
- An item has a UUID, list FK, editable name, amount text and authored unit, deterministic position, checked state, timestamps/revision, original generated values when applicable, and structured copied recipe contributions. Manual items have no recipe contributions and may have no amount.
- The implemented two-table representation keeps bounded source-snapshot JSONB on the list and bounded contribution JSONB on each item. Contributions refer to list-local snapshot identities and copied ingredient positions, rather than depending on live recipe/version joins. Typed structures and validators are in `src/grocery.rs`; numeric calculation is transient and editable amounts are stored as text, with copied original values retained separately.
- Capture source title, recipe identifier when available, version/revision identity, original ingredient text and unit, base/target servings or scaling context, and each resulting contribution. Copy only list-relevant information; do not copy recipe instructions, images, or unrelated metadata.
- Keep immutable source contributions and original generated name/amount/unit distinct from the row’s current editable name/amount/unit. Mark changes as “Edited on this list.” After an edit, the inspector distinguishes the current shopping entry from the original recipe requirements; a renamed item or changed unit must not be presented as the unchanged sum of those requirements. Do not convert units, rewrite source contributions, or automatically merge/split rows in response to edits.
- Recipe deletion, later edits, and visibility changes leave the saved list’s captured content intact. Reauthorize any live source link before returning current recipe content; an inspector must remain readable using saved contributions alone.
- Implemented lifecycle: create (including empty/manual lists), list/open, rename, add/edit/remove any items (including ingredient names, amounts, and units on generated rows), check/uncheck items, and delete a list with an explicit UI confirmation. Archive, duplication, drag ordering, and adding/replacing source recipes after save are deferred unless explicitly added to scope.
- Deleting a list cascades only to its own items; account deletion follows existing user-owned-data cleanup. A recipe deletion must never cascade into a grocery list.
- Use stable list ordering and stable item positions. Sidebar ordering is descending creation time plus UUID, so item check-offs do not move a list under the pointer. Do not require unique list names.
- The owner/creation index and unique list/position index support these reads. Sidebar counts use one aggregate query; account lists are bounded at 100. Recipe discovery has 25-row pages, and item/source payloads are bounded as recorded below.

### Creation, snapshot consistency, and quantities

- Manual creation submits a name/note and zero or more explicit items. It never calls recipe discovery or requires a recipe selection.
- Recipe-based creation submits accessible recipe references and requested serving counts, plus manual extras. Fetch canonical accessible published content server-side; never trust client-submitted ingredient totals or source titles as authoritative recipe data. Support owned private recipes and other readable public/unlisted recipes; recheck current authorization at save as well as during selection.
- Copy every ingredient row before permitted numeric aggregation, retaining each original contribution. Do not skip optional-looking ingredient names, interpret prose alternatives, or add equipment as grocery ingredients. Users decide what to remove and what to add.
- Keep the list name, note, and manual entries when switching entry paths. Choosing a manual-only list must visibly remove recipe selections, rather than silently retaining recipe ingredients. The starting-mode action clears draft recipe selections and their serving controls together; it does not alter any saved list.
- Build a single coherent snapshot per selected recipe. Validate access and any reviewed source revision in the creation transaction. If a preview was shown and a source changed, return a “review updated ingredients” state without discarding manual edits or silently saving different quantities. Specify the preview/revision request fields in CP1.
- Insert the list, items, and attribution atomically. If any source is unavailable or validation fails, save none of the list; identify the affected selection without revealing inaccessible recipe content. Let the user remove the selection and retry.
- Give create requests an owner-scoped idempotency key so a network retry cannot produce duplicate lists. Bind that key to the serialized submitted payload; reject reuse with different content. Keep it for the lifetime of the saved list, enforced by a unique owner/request-key constraint.
- Preserve the ingredient name exactly for equality, including case and spelling; `scallion` and `green onion` do not merge. Ordering is first occurrence in selected-recipe/ingredient order, followed by manual extras in entry order.
- For each recipe with a valid positive base serving count, calculate `factor = target_servings / base_servings` and apply it to parseable numeric amounts **before** aggregation, regardless of unit. For example, 2 cups for 2 servings becomes 4 cups for 4 servings. Do not scale a source twice when combining contributions.
- Sum generated numeric requirements only when both the ingredient name and authored unit strings match exactly. This also permits matching count-style or blank units when both strings match; it never infers a physical conversion. `cup` and `cups`, `g` and `grams`, and differing case/spelling remain distinct. Keep unparseable/textual quantities separate even when names/units match. Keep manual items separate from generated rows, even when their names and units match, to preserve deliberate edits and attribution.
- Preserve unparseable/textual amounts and authored units. Show the requested scaling factor and an uncertainty label when a serving change cannot be applied numerically, such as “to taste.” A parseable amount in cups, ml, pieces, or another unit is not uncertain merely because it is not grams. Do not pretend an unchanged textual amount is a computed scaled quantity.
- Reject overflow, nonfinite values, invalid target servings, and malformed request data. Never silently drop an ingredient, turn a negative value positive, erase a tiny positive requirement, or infer a unit from an empty unit field. Separate unparseable authored quantities from numeric requirements instead of treating uncertainty as zero.
- Unit matching, full edits, and copying all ingredients are decided. Numeric representation/rounding and the missing-base-serving fallback are recorded below. Reuse existing parsing code only after proving that its accepted syntax and validation match the grocery contract.

### Server operations, authorization, and write recovery

Use typed Dioxus server functions with a service/DB layer, following existing application conventions. Implemented operations:

| Operation | Input | Required behavior |
|---|---|---|
| List summaries / get list | Bounded cursor/page or list ID | Session owner only; summaries include counts; details include ordered items and copied source context. |
| Preview recipe requirements | Selected published recipe IDs, target servings | Current read access, bounded server-generated totals, source revision identities, explicit uncertainty. Manual-only creation needs no preview request. |
| Create list | Name/note, selections/revisions, manual items, idempotency key | Session ownership, account-change guard, canonical source validation, one atomic write; return the saved list/summary. |
| Rename / edit / remove | List/item ID, editable name/amount/unit or other supported fields, expected revision | Owner-gated SQL; item must belong to the addressed list. Generated and manual rows are equally editable; copied source context remains immutable. Do not accept a whole stale checklist as an overwrite. |
| Set item checked | List ID, item ID, explicit boolean, expected item revision | Desired-state update, not a toggle command. A retry already matching current state is a no-op; a stale request that would change newer state conflicts. Persist before reporting success; return authoritative item/counts. |
| Delete list | List ID and expected revision | Owner-gated removal with list-local cascade; UI handles deleting the active list and removing the final list. |

- Derive the actor from the authenticated session. Mirror existing account-change guards so a form opened under account A cannot write into account B after a session switch. Scope reads and mutations by owner in SQL, not solely in the browser.
- Use parameterized queries and shared request-budget/error handling. For absent or other-owner IDs, return the same non-disclosing not-found outcome; unauthenticated users get the established sign-in flow. Define validation, stale revision, unavailable source, and transient failure results in shared typed contracts.
- Implemented write-conflict policy: compare-and-set with per-item revisions for check-offs and item edits, plus list revisions for list metadata/deletion. A stale write preserves current server data and offers refresh/retry. Item mutations advance the list revision as well as the edited item’s revision; separate item check-offs compare only their own revisions, so unrelated check-offs do not conflict. Rename/deletion compare the list revision.
- After success, refresh the affected item/list summary and counts. On failure, retain manual form values and selection; show an inline retry. If checkboxes update optimistically, roll back failed edits and prevent older responses from overwriting newer intent.
- Slice 1 requires a network for persistence. Distinguish pending, saved, conflict, and failed writes; do not show the prototype’s browser-storage success label as database confirmation. In-flight or offline actions must not be presented as saved. Reopening/refetching shows committed progress; live cross-device push updates are deferred.

### UI integration

- Use the implemented `/grocery-lists`, `/grocery-lists/new`, and `/grocery-lists/:id` routes using the shared route inventory. The workspace URL identifies the active list so reloads and Back/Forward remain useful.
- Reuse shared Noms typography, color roles, materials, inputs, buttons, dialogs, and loading/error components. Do not import the mockup DOM/JavaScript or duplicate the app shell.
- Connect **Create list 10** to bounded recipe-card/read APIs, extending the selector beyond owner-only results where needed while reusing existing discovery and access rules, with selection across all accessible recipes, per-recipe serving controls, explicit manual-item name/amount/unit entry, inline validation, and submit progress. No recipes is valid; do not require an invented default serving count.
- Connect **Workspace 2** to real list summaries and ordered items. Preserve selection, show correct empty/loading/not-found states, update counts after writes, and support manual lists without an empty recipe inspector.
- Open the inspector from an ingredient’s recipe-details control. Show the current item name/amount/unit, copied source names/amounts/units, serving context, uncertainty, and any “Edited on this list” distinction; never require a live recipe fetch to explain a saved item. Closing the inspector restores focus to its trigger.
- Provide name, amount, and unit editing plus removal for every list row, with the same validation and save/retry behavior for manual and generated items. Preserve checked state on edits unless the user explicitly changes it; editing a name/unit must not silently merge it with another row.
- Follow NOMS-022 mobile patterns at compact and phone widths. Keep 44px touch targets, visible keyboard focus, labels for controls, usable on-screen-keyboard layout, and an accessible drawer/stacked inspector. Verify both themes and stable reading order.
- Add one discoverable Grocery lists entry using existing desktop/mobile navigation conventions. Do not make the mockup’s demonstration header a second navigation system.

## Slice 1: resolved decision register

The user resolved recipe scope, serving-aware generation, unit matching, full edits, and ingredient inclusion, then delegated minor technical decisions. The remaining engineering details were settled during implementation as follows.

| ID | Status and decision | Implemented detail | Checkpoint |
|---|---|---|---|
| G1 | **Resolved:** all accessible recipes, not only owned recipes. | Use existing ownership/public/unlisted read rules at selection and save. Public discovery plus supplied accessible unlisted links/IDs must not become global unlisted discovery. Historical-version/draft selection is not implied by published-recipe selection. | CP1 request/source contract; CP4/CP5 integration |
| G2 | **Resolved:** target servings scale against positive base servings. | Positive integer targets 1–10,000; with missing/nonpositive base servings, copy original amounts at factor 1, show uncertainty, and disable target scaling. Do not invent a base serving count. | CP1/CP3 complete |
| G3 | **Resolved:** exact name/unit matching across all authored units; no conversion. | Bounded rational arithmetic supports integer, decimal (up to 9 fractional digits), fraction, mixed-number, and spaced-fraction inputs. The parser bounds decimal whole parts at 10^12 and checks/bounds arithmetic intermediates. Sum before formatting, round half-up to at most 6 decimal places, and display positive quantities below that threshold as `<0.000001`. Preserve source text, expose rounded/uncertain contributions, and store editable display amounts as text. Unparseable amounts remain separate. | CP1/CP2/CP3 complete |
| G4 | **Resolved:** fully edit names, quantities, and units on generated/manual items; remove any item and add more. | Keep immutable original recipe contributions separate from current row fields. Label edited rows and show the original requirement in the inspector without claiming it equals the edited entry. No automatic unit conversion or row merging on edit. | CP1 data shape; CP4 mutations; CP6 inspector |
| G5 | **Resolved:** copy all authored ingredients and let users add/remove items. | Preserve optional/alternative wording without interpreting it. No optional-ingredient flags or recipe-schema redesign is required for this slice. | CP3 generation; CP5/CP6 item controls |
| G6 | **Resolved:** source revision and write-recovery contract. | SHA-256 of canonical title/servings/ingredients/update time pins a reviewed source; shared source locks preserve access/content while creating. Owner/request-key fingerprints deduplicate create retries; add keys deduplicate item additions. Per-item compare-and-set governs edits/check-offs, list revisions protect rename/delete, and stale desired-state no-ops do not overwrite newer data. Optimistic checkboxes roll back failures; list resources follow route changes. | CP1/CP2/CP4 complete |
| G7 | **Resolved:** bounded fields and reads. | 100 lists/account; 20 selected recipes/create; 1,000 resulting items/list; 100-character names, 500-character notes/item names, 64-character amounts/units; 2 MiB input/output/list payload bound; 25 recipe choices/page with offset capped at 10,000. Shared request budgets and a 5-second DB execution budget apply. Changes that would exceed the readable payload bound roll back. | CP1/CP2/CP4 complete |

No additional product decision is required for this slice. Non-gram grocery quantities are implemented now; pantry conversion/feasibility and offline synchronization remain later work.

## Slice 1: implementation checkpoints

CP1–CP7 are complete for local implementation and verification. The table records their acceptance contracts; concrete evidence and remaining validation limits follow it.

| Checkpoint | Deliverable and likely touch points | Required acceptance evidence |
|---|---|---|
| **CP1 ✓ — Freeze first-slice contracts** | Apply resolved G1/G3/G4/G5 and serving-aware scaling; finalize G2’s missing-base edge case and G3/G6/G7 engineering details. Specify DTOs, field/byte limits, quantity examples, original/current item fields, source revision policy, write conflicts, and lifecycle. | Review examples for manual-only/empty/mixed lists, missing servings, mixed units, fractional grams, stale sources, source deletion, and account switching. No claim of implementation readiness for unresolved dependent contracts. |
| **CP2 ✓ — Schema and manual-list persistence** | Add additive `grocery_lists` / `grocery_list_items` definitions, constraints, indexes, owner-scoped queries, and the shared test-schema changes. Implemented in `migrations/schema.sql`, shared `migrations/grocery-lists.sql`, `src/db/grocery.rs`, `src/grocery.rs`, and `src/test_utils.rs`. New queries use bound runtime SQL, so `.sqlx` is unchanged. | Fresh and upgrade database plans; constraints and account/list cascades; multiple manual lists with empty/text amounts; zero-recipe provenance; atomic writes; isolation between two users; item/list mismatch rejection; stable order and bounded reads. Review `pgschema` output before applying outside a disposable DB. |
| **CP3 ✓ — Pure requirement calculation** | Add a target-independent grocery requirement builder and tests; preserve existing recipe scaling behavior. Input is canonical recipe content plus selected servings; output is ordered rows, contributions, and uncertainty. | 200 g + 300 g = 500 g and 1 cup + 2 cup = 3 cup; serving scaling before summation in any unit; exact name/unit identity; differently spelled units kept separate; fractions and unparseable text; missing base servings; invalid/nonfinite/overflow input; small positive values; duplicate ingredient rows; deterministic source attribution. |
| **CP4 ✓ — Authenticated server functions** | Use `src/api/grocery_list.rs`, DB/service transaction paths, validation, request budgets, revision checks, idempotency, and typed errors. Wire manual CRUD and server-derived recipe creation. | Manual and recipe creation via real server functions; all-or-nothing unavailable-source failure; no trusted client totals; same idempotency key does not duplicate rows; changed source/reused key conflicts; set-checked retries; all readable recipe visibility cases and inaccessible private-source rejection; two-account ID substitution; full generated/manual name/amount/unit edits with preserved original contributions; stale writes and account-change rejection; no partial writes. |
| **CP5 ✓ — Compact create page** | Add the selected C10 form using shared components and real recipe discovery/selection. Support manual-only and mixed lists, target servings, uncertainty, creation retry, and navigation to the saved workspace. | Keyboard and browser submission for manual, empty, recipe, and mixed lists; preserve values after failed save; owned/private, other-public, and accessible-unlisted selection; no hard-coded demo account/data/serving limits; changed-source review; one created row after retry; unauthorized source feedback. |
| **CP6 ✓ — Workspace and inspector** | Add the selected W02 sidebar/checklist/inspector, direct list routes, navigation, list rename/delete, supported item mutations, counts, responsive details, and mutation freshness. | Switch multiple lists without mixing items; reload/direct navigation and second-session reads; saved check/uncheck; manual list has no recipe dependency; inspector distinguishes the current edited entry from its unchanged original contributions; full item edits/removal/additions persist without changing source recipes; source edits/deletion/private transition do not mutate list; hidden source links do not leak current content; delete active/final list; no false “saved” on failure. |
| **CP7 ✓ — Integrated release verification** | Run the repository’s applicable formatting, WASM/server compile/lint, focused arithmetic/API/DB tests, reviewed schema checks, SQLx-cache checks, and focused E2E flows. Record exact commands/results and remaining limitations. | Both themes; desktop + 320/390px phone checks; keyboard/focus + error states; no horizontal overflow; browser/server round-trip proof; owner isolation; concurrent check-off conflict behavior; no regressions to existing recipe editing/scaling and navigation. No runtime claim based only on mockup checks. |

### Implementation and verification evidence — 2026-10-24

- Shared models/calculation: [`src/grocery.rs`](../../src/grocery.rs). Database service and integration tests: [`src/db/grocery.rs`](../../src/db/grocery.rs), [`src/db/grocery_tests.rs`](../../src/db/grocery_tests.rs). Typed session-bound server functions: [`src/api/grocery_list.rs`](../../src/api/grocery_list.rs).
- Real W02/C10 pages: [`src/pages/grocery.rs`](../../src/pages/grocery.rs) and [`assets/grocery.css`](../../assets/grocery.css), with the shared app shell and desktop/mobile navigation integration. Both manual and recipe-derived rows support name/amount/unit edits, removal, and persisted check-offs.
- Schema delivery: the isolated application initialized successfully against the new declarative schema. A separate disposable database was initialized from the prior `HEAD` schema, then planned/upgraded with pinned `pgschema`: **two tables added, no drops or unrelated changes**. The upgrade succeeded and a subsequent plan reported **no changes detected**. Shared test fixtures use the same grocery SQL definitions; the equality check passes.
- Focused native tests: **50 passed** across `grocery` (13), `components::navbar` (7), `components::mobile::tests` (1), `auth::account_binding` (5), and `utils::recipe_scaler` (24), using `cargo test --locked --features server <filter> -- --test-threads=2`.
- The grocery group includes temporary PostgreSQL tests for owner/item isolation, multiple/empty/manual lists, retry deduplication, private/public/unlisted access, source changes/deletion, all-ingredient copying, serving scaling, full edits with immutable contributions, atomic failure, and independent/stale check-offs.
- `cargo fmt --all -- --check`, `git diff --check`, server `cargo clippy --locked --features server -- -D warnings`, WASM `cargo clippy --locked --target wasm32-unknown-unknown -- -D warnings`, and schema workflow regression checks passed (six schema-plan fixtures plus pinned/declarative-workflow checks).
- The PR branch was rebased onto current `main`, preserving its newer collection portability and About-page work. All 50 focused tests and 18 browser checks were rerun successfully after integration. The earlier WASM lint allowance is no longer needed because the upstream settings-page lint was fixed on `main`. The live Dioxus client/server build also succeeded.
- Reproducible browser check: [`scripts/tests/test-grocery-list-browser.py`](../../scripts/tests/test-grocery-list-browser.py), run as `python3 scripts/tests/test-grocery-list-browser.py --instance noms-groceries --output /workspace/noms-grocery-pr-e2e`. Chromium 154 passed 18 checks with **no page errors**, including an aborted checkbox request and rollback, a second authenticated browser session reading committed progress, accessible-source search/link selection, serving validation, exact-unit totals, full edits, source deletion, focus restoration, both themes, rename, route switching/Back, delete-active-list, and anonymous isolation. Viewports: 320, 390, 760, and 1440px.
- Browser testing caught and fixed integration issues: ParadeDB rewrites made plain title `ILIKE` ambiguous, so source discovery now uses a bound literal substring query; list resources now explicitly follow the active route ID rather than retaining a prior checklist. Recipe deselection/reselection also resets displayed and effective serving counts together. All corrected paths passed the final E2E run.
- Query-plan evidence at the configured maximum list/item counts: 100 lists with 1,000 items each. The owner-scoped summary query completed in **35.416 ms**; an ordered 1,000-item read used the list/position index and completed in **0.450 ms** in this local environment. Synthetic performance fixtures were removed afterward. These are bounded local observations, not production latency guarantees.
- Screenshots, browser result JSON, schema plans, and query plans are runtime artifacts under `/workspace/noms-grocery-e2e` and `/workspace/noms-grocery-pr-e2e`, outside the checkout; the reviewed workspace screenshot is included with the implementation plan. No cookies, session state, runtime credentials, or generated build caches are added to the repository.

Local verification is complete. Production deployment has not been performed. Real-device/on-screen-keyboard behavior, other browser engines, live cross-device push updates, pantry/feasibility, and offline synchronization remain outside this verified slice. The full repository test suite was not rerun; the scoped suites and browser/database checks above were run.

### Release and recovery details

- Deliver the schema additively; the old application should continue to work with the new tables present. Deploy schema before code that requires it. Do not rewrite applied production state or add reset/prune commands to deployment.
- Review the declarative plan, test against both a fresh database and the previous schema, and ensure shared test setup includes the same invariants. Keep production credentials out of plan/log artifacts.
- Prefer application rollback that leaves new list data intact. A table-dropping rollback is not the default recovery path; retain created lists for the fixed release.
- Require numeric bounds, ownership, snapshot consistency, and persistence checks before release. Performance checks should exercise owner-scoped sidebar/list queries at agreed quotas, using query plans for meaningful access paths.
- Browser screenshots are review evidence; database and server tests establish persistence and isolation. Local prototype tests are not substitutes for these release checks.

## Slice 1: acceptance scenarios

- Create “Household essentials” with dish soap and coffee, with zero recipes and no kitchen profile. Save, reload, and reopen under the same account on another device/session.
- Create an empty named list, reopen it, and add the first item. Create two lists with the same name and distinguish them by their stable IDs/routes.
- Create a list from two recipes that need 200 g and 300 g of exactly named `Rice`. See 500 g on the checklist and the two saved contributions in the inspector.
- Select the user’s private recipe, another user’s public recipe, and an accessible unlisted recipe. Generate from each without an ownership-only picker restriction; reject another user’s inaccessible private recipe and avoid enumerating unlisted sources.
- Change selected serving counts before creation and verify saved amounts in grams, cups, ml, and count/blank units. A recipe with 1 cup for 2 servings contributes 2 cup for 4 servings; a second contribution of 2 cup yields 4 cup total. Apply the final G2 missing-base fallback when no valid ratio is available.
- Keep `Rice` and `rice` separate; keep `scallion` and `green onion` separate. Keep `Rice: 200 g` and `Rice: 1 cup` separate; likewise keep `cup` and `cups` separate. Only unparseable/textual amounts need a scaling-uncertainty label, not numeric non-gram quantities.
- Add manual rice and dish soap to a recipe-derived list. Preserve those manual rows as authored; no automatic recipe merge or false source attribution.
- Rename a generated `Rice: 500 g` row to `Brown rice`, change its amount to `2` and unit to `cup`, save, and reload. Show the edited entry as `Brown rice: 2 cup` and the original `Rice: 500 g` recipe contributions separately; never imply a conversion or modify the recipes. Preserve its checked state.
- Remove a generated ingredient, edit/remove a manual item, and add a new item. Keep the remaining rows, source context, and check-offs unchanged. Do not automatically merge rows when an edit makes names/units match.
- Copy every ingredient including optional-looking garnish and textual alternatives. Keep that authored text visible until the user edits or removes it.
- Save a recipe-derived list, edit/delete the source recipe or make it private, and verify the list’s copied contents and checked states are unchanged. Live source navigation continues to obey current access rules.
- Switch among manual and recipe lists, check an item, reload, and see the committed state on the correct list. Opening a manual list does not fetch or require recipe inspector data.
- Reject another account’s list or item IDs for every read/write and reject a valid item ID paired with a different list. Do not reveal whether another user’s list exists.
- Retry a create request after a dropped response and obtain the original list. Reject a reused idempotency key for different contents and avoid partial creation when one recipe is unavailable.
- Handle a source revision change between preview and save without silently changing the reviewed amounts or losing manual edits. Handle a stale item write without replacing newer committed state.
- Display pending/failed/conflicting checkbox writes honestly. A network interruption does not become a saved/offline-success claim; keep the full offline queue/sync feature deferred.
- Use the create page and recipe inspector with keyboard navigation and at compact phone widths, including the on-screen keyboard. Keep the existing mobile navigation and both themes usable.

## Slice 1: readiness checklist

- [x] First delivery is independent database-backed grocery lists; kitchen profiles and offline synchronization are deferred.
- [x] Manual-only, recipe-derived, and mixed lists are in scope; a recipe is never mandatory.
- [x] Workspace 2 and Create list 10 are selected; the implemented workspace preview is linked.
- [x] Current storage/scaling/auth/schema baseline and prototype limitations are recorded.
- [x] Checkpoints, acceptance scenarios, release checks, and recovery approach are identified.
- [x] All accessible recipes, serving-aware generation, naive name/unit matching, full item edits, and copying all ingredients are confirmed.
- [x] Missing-base-serving behavior, numeric precision, original/current provenance, conflicts, and input bounds are implemented under delegated technical decisions.
- [x] CP2–CP7 have local implementation and verification evidence below. Production rollout, physical-device/cross-browser testing, and later slices are not claimed.

---

## Pantry inventory and later feasibility outcome


Users can answer two complementary questions:

1. Given the ingredients and equipment available in a selected kitchen, what recipes can I make, and what is missing for near matches?
2. Given a set of recipes and serving counts, what ingredients and equipment do I need, and what must I obtain?

Saved kitchen profiles make these later workflows reusable. They build on the independent grocery lists in Slice 1; they are not prerequisites for creating or using those lists.

## Later product scope: kitchen profiles and feasibility

### Persistent kitchen profiles

- Support multiple private, named profiles, each containing its own ingredients and equipment, such as Home or Cabin.
- Allow creating, renaming, editing, duplicating, and deleting profiles, and quickly marking ingredients as out.
- Evaluate against one selected profile at a time and remember the last selected profile. Do not implicitly combine supplies across locations.
- Require explicit availability for every ingredient, including salt, oil, and water; there are no assumed staples.
- Allow ingredient presence without a quantity. Optional authored quantities and units support exact name/unit sufficiency checks in a later slice; unknown quantities remain distinguishable from sufficient quantities.
- Keep inventory changes deliberate. Selecting or making a recipe and checking off a shopping item do not automatically deduct or replenish inventory.

### Initial matching and quantity rules

**Current recipe-detail scope:** the [recipe pantry check](NOMS-018-recipe-pantry-check.md) uses case-insensitive ingredient-name presence and explicit availability only. It ignores amounts, units and recorded zero quantities. The quantitative rules below describe deferred sufficiency/shortage work and do not apply to the current check.

Pantry uses the grocery list’s exact ingredient-name/unit matching rule. This supersedes the older grams-only proposal. Inventory management ships before recipe feasibility; pantry integration must preserve authored units and explicitly report incompatible or unknown quantities.

- Identify ingredients by exact name equality. Distinct names are distinct ingredients; do not infer aliases, spelling/case equivalence, varieties, or substitutions. Using optional NOMS-017 vocabulary references requires a later explicit compatibility contract; recognition alone does not authorize aggregation.
- Identify equipment by exact name equality in the same way. Each distinct equipment name represents a unique type checked for presence; do not model physical instances, counts, size, capacity, or compatibility initially.
- Support bounded parseable numeric quantities in any authored unit, including count-style or blank units, when both ingredient name and unit match exactly. No unit conversion, alias matching, singularization, or case folding. `Rice: 2 cup` can cover `Rice: 1 cup`; `cups`, `g`, and `kg` remain distinct.
- Preserve textual or absent amounts as presence-only entries. A same-name entry in another unit or with an unknown amount cannot establish numeric sufficiency. Known zero and explicitly out entries do not count as available. A definitive numeric shortage requires a compatible, fully known stock total; uncertain stock remains visible alongside any known deficit.

### Recipe feasibility

- Match both ingredient and equipment requirements against the selected profile.
- Explain results with missing ingredients, missing equipment, insufficient quantities, and unknown quantities. Having every ingredient is distinct from having enough to make the recipe.
- Use the categories Ready; Ready, quantity uncertain; Missing ingredients; Missing equipment; and Missing both.
- Known ingredient shortages count under Missing ingredients, including when the ingredient is present but its compatible quantity is insufficient. Missing both means ingredient shortages and missing equipment coexist.
- Use Ready only when required ingredients have sufficient known quantities and required equipment is present. Use Ready, quantity uncertain when there are no known missing requirements but sufficiency cannot be established. Preserve uncertainty details alongside any missing-requirement category.
- Include near matches with explicit missing requirements. Feasibility ranking is deferred.
- Evaluate recipes individually while browsing and collectively when selected together: 200 g of rice may cover either of two recipes individually while the pair requires 400 g.
- Missing explicitly optional ingredients must not disqualify a recipe. Accept alternatives only where the recipe explicitly permits them.
- Explicitly declared alternatives use the same exact-name checks; no equipment compatibility inference is performed.

### Combined requirements

- Select multiple recipes and serving counts without requiring a calendar.
- Aggregate compatible ingredient quantities and subtract known availability once across the selection. Expose remaining shortages and unknown quantities.
- Deduplicate equipment by exact name across recipes. Counts, size/capacity compatibility, and scheduling simultaneous equipment use are outside the initial scope.
- Retain source-recipe attribution for every generated requirement and show which recipes contribute to it. Live recipe links must respect current access; saved list content remains independent.
- Recalculate requirements when serving counts or selected recipes change in a preparation preview. Updating an already saved list from recipes requires a separate, explicit future regeneration workflow; ordinary recipe edits never update saved lists.
- Sum requirements and stock only for exactly matching ingredient name and unit. Subtract stock once across the selection. Preserve incompatible and unknown quantities separately; never hide uncertainty behind a numeric total.

### Shopping checklist UI

- Allow future profile-aware checklists to associate with a selected kitchen profile and show what is available and what remains to obtain. Keep profile-free manual and recipe-based lists usable without a kitchen profile.
- Present ingredient shortages and missing equipment distinctly, with quantities, units, uncertainty, and source recipes available for inspection.
- Support checking and unchecking items with persisted state and a clear distinction between outstanding and completed items.
- Allow manual items unrelated to recipes, including non-food shopping needs such as dish soap.
- Provide a responsive phone layout with accessible, touch-friendly checkboxes and readable quantities.
- Keep a previously loaded checklist readable and checkable offline, persist local changes, and synchronize when connectivity returns. Show pending synchronization and failures without losing edits.
- In a future explicit regeneration workflow, preserve manual items and reconcile completed state; specify those rules before offering regeneration. Source-recipe changes alone do not trigger it.
- Checking an item off records shopping progress only. Pantry replenishment remains an explicit user action.

## Boundaries of the later kitchen/feasibility scope

- Meal-planning calendars, recurrence, and simultaneous cooking schedules.
- Household sharing or combining multiple kitchen profiles for a match.
- Expiration tracking, automatic inventory consumption or replenishment, and automatic substitution suggestions.
- Full offline editing of kitchen profiles and recipe selections; the offline commitment here is the shopping checklist.
- Unit conversions, inferred ingredient/equipment equivalence, equipment counts and compatibility, and feasibility ranking. Optional vocabulary-reference integration is a later step coordinated with NOMS-017.

## Questions for the later kitchen/feasibility slices

- How will exact-name entries use optional NOMS-017 references without changing authored names or assuming ingredient/equipment equivalence?
- How are optional requirements and equipment alternatives represented in recipe data?
- How should a future explicit regeneration workflow reconcile changed selections, serving counts, inventory, manual items, and completed purchases? Preserve the Slice 1 snapshot rule unless the user requests regeneration.
- How are offline edits synchronized across devices, conflicts resolved, and profile deletion handled for existing lists?
- What is the explicit workflow for adding purchased quantities to a kitchen profile?

## Deliverables for later implementation plans

- Exact-name ingredient/equipment matching and exact name/unit quantity contracts, with a later optional NOMS-017 vocabulary-reference integration path.
- Kitchen profile, recipe selection, combined requirements, checklist, and provenance data models.
- Authorization, privacy, migration, and offline behavior.
- Test-first checkpoints for each vertical slice and their integration.
- Automated and manual verification covering profile isolation, explicit staples, optional requirements, equipment matching, scaling, shared-stock aggregation, ambiguity, and mobile/offline checklist use.

## Acceptance scenarios for later kitchen/feasibility implementation

- A user switches between Home and Cabin and sees results based only on the selected profile, including after a reload.
- A recipe requiring salt reports it missing when salt has not been explicitly included.
- A recipe with all ingredient names present but unknown amounts shows quantity uncertainty; missing required equipment remains visible.
- Two selected recipes requiring 200 g of rice each, against a profile containing 200 g of rice, produce a 200 g shortage.
- A profile listing `scallion` does not satisfy `green onion`; distinct equipment names likewise do not match automatically.
- `Rice: 2 cup` covers a `Rice: 1 cup` requirement; `Rice: 200 g` cannot establish sufficiency for that requirement and reports uncertainty. `cup` and `cups` stay distinct.
- Feasibility categories distinguish known ingredient shortages, missing equipment, both, full coverage, and coverage with uncertain quantities; uncertainty remains visible even alongside a known shortage.
- Two recipes requiring the same reusable blender produce one equipment requirement with both source recipes retained.
- An absent optional garnish does not block a match; an explicitly permitted equipment alternative can satisfy its requirement.
- Changing serving counts in a new-list preview updates requirements. Any future explicit regeneration preserves manual shopping items and does not silently misrepresent completed purchases.
- A user opens a saved checklist on a phone, goes offline, checks and unchecks items, reloads it, and reconnects without losing shopping progress.
- Shopping check-off leaves the kitchen inventory unchanged until the user explicitly updates it.

## Readiness for later kitchen/feasibility slices

- [x] Product scope includes multiple persistent kitchen profiles, explicit ingredients and equipment, both matching directions, and a shopping checklist.
- [x] Pantry inventory scope and exact name/unit compatibility are recorded in the [Slice 2 plan](NOMS-018-pantry-inventory.md). Recipe-feasibility presentation remains a later slice.
- [ ] Availability subtraction, aggregation, and checklist reconciliation rules are specified.
- [ ] Authorization, profile/list lifecycle, and offline synchronization contracts are specified.
- [x] Pantry inventory W01/C01 selection, implementation, and local verification are recorded in the Slice 2 plan. Recipe feasibility and shortage generation remain later slices.
