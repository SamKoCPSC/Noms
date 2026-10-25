# NOMS-018: Pantry inventory — Slice 2

**Status:** W01 / C01 selected by the user; pantry inventory implemented and locally validated. No production deployment performed.
**Updated:** 2026-10-25
**Foundation:** Independent grocery lists merged in [PR #21](https://github.com/SamKoCPSC/Noms/pull/21).
**Parent scope:** [NOMS-018](NOMS-018-pantry-meal-planner-grocery-list.md).
**Selected UI:** W01 — Kitchen inspector, and C01 — Compact starter. [Runtime workspace](../assets/NOMS-018/implementation/pantry-workspace.png) · [Creation form](../assets/NOMS-018/implementation/pantry-create.png).

## Outcome and delivery order

Create a persistent inventory of what is available in each kitchen. Users can create Home and Cabin, add ingredients with or without quantities, track equipment, and mark items as out. Each kitchen remains independent.

Ship inventory management first. Recipe feasibility comes next, followed by an explicit workflow to create grocery lists from shortages. Offline synchronization, meal calendars, expiration tracking, shared households, automatic deductions, and automatic replenishment are deferred. Shopping check-offs continue to record shopping progress only.

## Implemented decisions

The user approved pantry management followed by recipe matching and shortage generation, and the recommendation to use the same exact ingredient-name/unit compatibility as grocery lists. This replaces the old grams-only pantry proposal. The user selected W01 and C01 and authorized end-to-end implementation with sensible technical defaults. The contracts below are implemented.

- Use **Pantry** as the product section and **kitchen** for each named profile. UI examples use Home kitchen, Cabin, and Office kitchenette. Do not silently combine stock across them.
- Support creating an empty kitchen, naming/renaming it, editing its optional note, duplicating its inventory, and deleting it with confirmation. Remember the last selected kitchen per account/device. Switching kitchens never carries a stale edit into the new kitchen.
- Each kitchen has ingredients and equipment. Ingredient names, amounts, and units are editable. Equipment has an editable name and availability, without quantities, instances, sizes, or compatibility inference.
- Quantity is optional. `Olive oil` with no amount means present, quantity unknown; it never means unlimited. Permit arbitrary authored units and bounded descriptive amounts. Zero means no measured stock; explicit out status overrides any retained amount.
- Keep **availability** explicit: In stock / Out for ingredients, Available / Unavailable for equipment. Marking an item out retains the authored quantity for editing, excludes it from available stock, and never silently zeroes or consumes it. Restoring availability requires deliberate action. A numeric zero still displays as zero and is excluded from usable stock.
- Additions and edits remain separate rows even when names/units match. Do not silently merge or rewrite existing amounts. Future quantity math can total compatible rows without changing inventory records.
- No assumed staples, preset stock, or recipe import at creation. Starting ingredients/equipment are optional and explicitly entered. A duplicate action copies only the selected kitchen, including quantities and availability, after explicit selection; the copy is independent.
- Keep names and units exactly as authored, including case and spacing except rejecting blank names. `Rice` / `rice`, `cup` / `cups`, and `g` / `kg` do not match. No aliases, conversion, substitutions, singularization, or fuzzy matching.
- Use existing authenticated account/session guards, materials, fonts, route inventory, mobile navigation, and database workflows. No new identity or API stack.

The prototypes contain synthetic stock to illustrate layouts. Their seeded data, browser storage, visual grouping, and convenience controls do not define production defaults or promise database persistence.

## Persistence and API contracts

Implemented additive `kitchen_profiles`, `pantry_ingredients`, and `pantry_equipment` tables in the declarative schema. Profiles have an owner, name, optional note, revision, and timestamps; rows have a profile FK, deterministic position, their editable fields, availability, revision, and timestamps. Deleting a profile cascades to its own inventory only. There is no required FK from a grocery list to a kitchen and no changes to existing list snapshots or source contributions.

Use the established grocery conventions: session-derived ownership, account-change consistency guard, owner/profile-scoped SQL, parameterized queries, request budgets, typed validation/conflict/not-found outcomes, transactional creates/duplicates, and payload-bound idempotency keys for creates/additions. Duplication takes a consistent source snapshot and copies into new row identities. Never authorize by a client-supplied owner ID.

Implemented bounds follow the existing grocery feature: 100-character profile names, 500-character notes/names, 64-character amount/unit strings, at most 100 profiles per account, 1,000 ingredient and 200 equipment rows per profile, and a 2 MiB response/request ceiling. Client/server validation and bounded query plans are verified below. Reuse bounded rational parsing only where numeric interpretation is needed. Reject numeric overflow/nonfinite/negative stock; preserve legitimate descriptive text as unknown quantity.

Operations: list profile summaries, read one profile, create/duplicate/rename/edit note/delete profile, add/edit/remove ingredient or equipment, and explicitly set availability. A derived `quantity_zero` flag on ingredient rows supports honest summary counts without interpreting arbitrary quantity text in SQL. It is recomputed from the shared bounded grocery parser on every ingredient write; authored amounts/units remain unchanged. Descriptive amounts stay quantity-uncertain.

Per-row compare-and-set revisions allow edits to different rows to coexist; profile metadata/deletion uses a profile revision. Idempotent desired-state retries do not toggle availability twice. Failed writes retain form values and show a recoverable error; optimistic availability controls must roll back on failure.

List summaries use bounded aggregate reads, with owner/profile and profile/position indexes. Avoid fetching every inventory on each switch or querying once per profile. Remember active selection in account-scoped browser state as a preference, not as inventory storage; clear invalid IDs after deletion/account changes.

## Selected UI and routes

Implemented `/pantry`, `/pantry/new`, and `/pantry/:id` routes mirror grocery navigation. Inventory workspaces support profile switching, search, ingredient/equipment views, explicit availability filters, optional quantities, full edits, quick add, and profile management. Unknown amount and out state must stay visually distinct; do not infer low-stock thresholds. The workspace heading reads “Available ingredients” (or “Available equipment” for that tab), and the kitchen note is editable without being repeated beneath the kitchen name.

The creation form supports a name and optional note, optional starting ingredients/equipment, and an empty start. An explicit duplicate action in the workspace copies an authorized kitchen with fresh row identities, then opens the independent copy. The 20 exploratory HTML proposals compare compact forms, guided entry, notebooks, visual cupboards, ledgers, status boards, and inspector workspaces. Browser-only interactions connect creation to a workspace. The user selected W01 and C01. The runtime form starts empty; synthetic stock exists only in the prototypes.

Selected and implemented: **W01 — Kitchen inspector** preserves the familiar grocery workspace; **C01 — Compact starter** follows the chosen compact grocery form. The other proposals explore different browsing and entry priorities without requiring additional product features.

## Later quantity and recipe integration contract

This slice stores stock; it does not label recipes ready or mutate grocery lists. Later feasibility should scale recipe requirements by servings, sum compatible requirements, and subtract compatible in-stock quantities once for the whole recipe selection. Presence matching uses exact ingredient names; numeric compatibility additionally requires exact unit strings, including matching blank/count units.

`Rice: 2 cup` can cover `Rice: 1 cup`. `Rice: 200 g` cannot establish sufficiency for that cup requirement. Presence-only or textual stock yields quantity uncertainty; it never becomes zero, infinite stock, or a numeric shortage. If there is both known compatible stock and uncertain stock, expose the known deficit alongside uncertainty instead of claiming an exact purchase amount. Out entries never count; each kitchen is evaluated separately.

Equipment matching uses exact name and explicit availability. Optional ingredients and permitted equipment alternatives require a structured recipe-data contract before feasibility ships; do not infer them from free text. New shortage lists will be explicit independent snapshots. Subsequent stock edits and grocery check-offs will not alter either side automatically.

## Implementation checkpoints

| Checkpoint | Deliverable | Verification gate |
|---|---|---|
| P1 ✓ — Select UI and contracts | Choose creation/workspace concepts and refine proposed field/lifecycle defaults. | User review of mockups; documented quantity/availability behavior and mobile editing flow. |
| P2 ✓ — Shared domain and schema | Models, validation, additive tables/indexes, shared test fixture. | Fresh and existing-schema plans, no unrelated destructive changes, quantity/availability edge cases. |
| P3 ✓ — Authenticated persistence | Profile/inventory CRUD, duplication, idempotency and revision handling. | PostgreSQL ownership/profile isolation, concurrent edits, retries, atomic duplicate/create, deletion boundaries. |
| P4 ✓ — Creation and workspace | Real server-backed pages, navigation, switching, edit/add/out flows. | Manual/empty start, arbitrary units, unknown amounts, equipment, account changes and failure recovery. |
| P5 ✓ — Integrated verification | Focused domain/DB/API tests and browser checks, documented limits. | Reload/second-session persistence, responsive light/dark layouts, keyboard/focus, stale edits, bounded query plans, formatting and server/WASM lint. |

The checkpoint tests apply to runtime implementation. HTML screenshot checks establish visual/demo behavior only.

## Acceptance scenarios

- Create an empty Cabin and an independent Home kitchen containing `Rice: 2 cup`, `Olive oil` with unknown amount, `Salt: 0 g`, and a Blender. Persist and reopen each in a second authenticated session.
- Edit every ingredient field, including switching units or clearing an amount. Unknown quantity remains distinct from zero, and another row with the same name/unit is preserved.
- Mark rice out, reload, and restore it deliberately. Keep its recorded amount visible while out; no stock is deducted, inferred, or transferred.
- Switch profiles while an edit or request is in flight. Do not show one profile’s result in another profile or write through a changed account session.
- Duplicate a kitchen, including unavailable rows, then edit/remove rows in the duplicate without modifying the original.
- Add/remove equipment and change availability. Do not assume an oven, blender, water, or any staple exists by default.
- Retry interrupted creation/additions without duplicates. Reject stale conflicting edits without losing newer saved state.
- Reject another user’s profile/item IDs and an owned row paired with a different profile. Profile deletion leaves every grocery list intact.
- Show loading, empty, failed, and conflict states honestly. Inventory requires an online save; browser-only prototype persistence is not production evidence.
- Use profile selection, ingredient/equipment editing, and confirmation dialogs at desktop and 320/390px widths, in both themes, with keyboard focus recovery.

## Remaining decisions

- **Inventory slice:** W01/C01 are selected; field/lifecycle defaults and quotas are implemented. Real-device/on-screen-keyboard and other browser-engine verification remain outside the local checks.
- **Before recipe feasibility:** structured optional/alternative requirements, multi-unit/unknown-stock explanations, recipe discovery/results UI, and exact shortage-list creation semantics.
- **Before later shopping automation:** explicit replenishment/consumption actions, regeneration reconciliation, and offline conflict/synchronization rules.

Inventory implementation is complete locally. Recipe feasibility, shortage generation, unit conversion, automatic replenishment/deduction, and offline inventory editing are deferred.

## Design review evidence — 2026-10-24

- Delivered 10 creation forms (`C01–C10`) and 10 workspaces (`W01–W10`) in a self-contained HTML gallery retained as a local design artifact, with bundled Noms fonts, licenses, logo, and design-system CSS.
- Captured desktop and phone previews for every design, plus comparison sheets and W01/C01 dark-mode examples. The gallery switches between desktop and phone previews.
- Browser checks passed on all 20 pages at 320, 390, 760, and 1440px without horizontal overflow or JavaScript page errors. Checks covered creation/reload, independent profile switching, ingredient name/amount/unit edits, out-state retention, profile duplication/deletion, and theme changes as applicable.
- The exploratory verification results and optional local `capture.py` script remain bundled with the local proposals. These are static-prototype checks; they do not claim pantry database/API persistence or production readiness.
- The user subsequently selected W01/C01. Runtime implementation and evidence follow below; prototype checks remain separate from persistence evidence.

## Runtime implementation and verification — 2026-10-25

- Shared contracts/validation: [`src/pantry.rs`](../../src/pantry.rs). Storage/service: [`src/db/pantry.rs`](../../src/db/pantry.rs). Session-bound API: [`src/api/pantry.rs`](../../src/api/pantry.rs). W01/C01 pages and styling: [`src/pages/pantry.rs`](../../src/pages/pantry.rs), [`assets/pantry.css`](../../assets/pantry.css).
- Supported flows: empty or stocked creation, ingredients with optional authored amounts/units, equipment availability, full field edits, explicit out/restore, row removal, profile metadata edits, independent duplication, deletion, search/filtering, remembered account-scoped active profile, and desktop/mobile navigation. Starting stock typed without pressing Add is included in creation.
- Inventory writes are transactional and owner/profile scoped. Create/add retry keys bind to payloads; copies validate and lock an authorized source revision. Row conflicts preserve newer state; availability writes are explicit desired states. Failed requests retain entries and never show an optimistic saved state. Copied row IDs, revisions, and retry keys are independent of the source.
- Focused native verification: **58 distinct tests passed** (61 executions across overlapping filters): `pantry`, `grocery`, `components::navbar`, `components::mobile::tests`, `auth::account_binding`, and `utils::recipe_scaler`. Additional pantry boundary checks cover signed/nonfinite/negative values and quantity uncertainty. Database tests cover owner/profile/kind substitution, exact authored values and duplicate rows, zero/unknown stock, atomic failures, concurrent writes, stale conflicts, idempotency, independent copies, and kitchen deletion preserving grocery lists.
- `cargo fmt --all -- --check`, `git diff --check`, strict server Clippy, and strict WASM Clippy passed with no lint allowances. Queries use runtime-bound SQL; no compile-time SQLx query/cache changes or new dependencies were needed.
- Declarative schema: fresh apply and upgrade from the merged grocery schema passed using pinned `pgschema` 1.12.1 in isolated databases. Upgrade adds **three tables and their constraints/indexes**, with no drops or unrelated changes; both subsequent plans report **No changes detected**. The isolated plan databases install the extensions needed by the schema; pg_cron remains configured only in the development application database. Six destructive-plan fixtures and the pinned schema-workflow regression check passed. Shared grocery fixture comparison now permits additional feature tables while checking the complete grocery SQL fragment.
- Reproducible runtime browser check: [`scripts/tests/test-pantry-browser.py`](../../scripts/tests/test-pantry-browser.py), run with `python3 scripts/tests/test-pantry-browser.py --instance noms-groceries --output /workspace/noms-pantry-e2e`. **18 Chromium checks passed, with no page errors**. Includes server-function access checks using their actual generated routes, authenticated second-session reads, request-failure recovery, full edits, unknown/zero amounts, equipment, independent copies, switching/Back, metadata, remembered selection, removal, empty creation, deletion, and anonymous isolation. Layouts checked at 320, 390, 760, 1100, and 1440px; light/dark themes and shared dialog interactions verified.
- Browser validation caught dialog click propagation to the backdrop, which is now handled by the existing modal-card pattern for all four pantry dialogs. Active-profile and heading classes use the shared grocery styles. User-provided pantry names/amounts/units remain ordinary escaped text.
- Query plans at 100 kitchens × 1,000 ingredients + 200 equipment rows: summary query **40.293 ms**; ordered 1,000-ingredient read **0.499 ms**, using the profile/position index. These are local observations, not production latency guarantees; synthetic fixtures were rolled back.
- Reviewed runtime previews: [workspace](../assets/NOMS-018/implementation/pantry-workspace.png) and [compact creation](../assets/NOMS-018/implementation/pantry-create.png). Browser JSON/screenshots, schema plans, and query plans are outside the checkout under `/workspace/noms-pantry-e2e`. Cookies/session material remain in memory and are not committed.

No production deployment was performed. The full repository test suite was not rerun; the focused suites, schema checks, lint/build checks, and browser flows above were run. Pantry inventory requires a successful online save; live push updates and offline synchronization remain deferred.
