# NOMS-018: Kitchen workspace unification

**Updated:** 2026-10-26
**Status:** Implemented and locally validated.
**Scope:** Follow-up to grocery lists ([PR #21](https://github.com/SamKoCPSC/Noms/pull/21)) and pantry inventory ([PR #25](https://github.com/SamKoCPSC/Noms/pull/25)).

## Product decisions

The user approved **Kitchen** as the combined workspace and **pantry** as the name of an individual inventory profile. Bring the selected pantry and grocery workspaces together through shared navigation and materials. Keep Home, Cabin, and other pantries independent, and allow grocery lists without any recipe or pantry association.

- One Kitchen destination in desktop navigation, the navigation drawer, and signed-in mobile navigation.
- A shared Kitchen header and Pantry / Grocery lists view links on workspace, detail, and creation pages. Place the view switcher directly beneath the Kitchen title, aligned to the upper left; keep it full width on phones.
- Below the view switcher, use the requested descriptions: “Track what ingredients you have and determine what recipes are available.” and “Create a grocery list based on selected recipes.” The pantry recipe-availability wording describes the intended use; automated recipe matching remains deferred.
- Each view retains its existing sidebar and optional item or recipe inspector. Switching views returns to the last viewed pantry/list during that workspace visit. Pantry's existing per-account/device selection preference is retained across visits.
- Pantry is ingredient-only for now: remove equipment tabs, editors, creation fields, search wording, and counts. Existing equipment rows, storage schema, and legacy API support remain intact so this UI scope change does not delete stored data. New pantry forms submit an empty equipment list.
- Inventory forms, actions, empty states, validation, and error messages call profiles pantries. User-authored names and notes remain as written.
- Canonical routes are `/kitchen/pantry`, `/kitchen/pantry/new`, `/kitchen/pantry/:id`, and the corresponding `/kitchen/grocery-lists` routes. `/kitchen` opens Pantry. Old `/pantry` and `/grocery-lists` root/create/detail bookmarks resolve to the same views and IDs.

## Technical boundaries

Use one nested Dioxus layout inside the existing authenticated application/session shell. View selection and global navigation follow the same route classification, including create/detail pages. The account-scoped session boundary resets remembered view destinations when the signed-in identity changes. Keep the existing create and write handlers, quotas, optimistic concurrency, and retry behavior.

Section links set `aria-current` explicitly: Dioxus's standard Link overrides it using exact URL equality, which loses the active section on create pages and legacy aliases. Preserve ordinary anchor destinations and modified clicks while routing unmodified clicks within the app.

This is a navigation and terminology change. Preserve the `kitchen_profiles` table, saved UUIDs, existing RPC names, and DTOs for compatibility; no data migration is necessary. Kitchen does not introduce another database entity. User-visible backend validation/conflict messages use pantry terminology.

Recipe cookability, cookable search, shortage calculations, pantry-linked grocery lists, purchased-stock transfer, and automatic inventory deductions are future work. Shopping check-offs continue to record shopping progress only.

## Validation

- Formatting and patch whitespace checks passed.
- Strict server and WASM Clippy checks passed with `-D warnings`.
- Focused native suites passed: 187 component tests (including route compatibility and rendered section-link accessibility), 10 pantry tests, 14 grocery tests, and 8 session lifecycle tests. These filters overlap; this is not a full repository test run.
- Chromium 154 passed **44 browser checks** with zero JavaScript page errors: 8 Kitchen navigation checks, 18 pantry checks, and 18 grocery checks. The existing suites also observe JavaScript errors in the second authenticated browser session.
- Verified independent pantry/list creation, record selection across view switches, browser Back, Ctrl-click in another tab, legacy root/create/detail bookmarks, matching active sections in desktop/mobile navigation, and anonymous access gates.
- Both workspaces and create forms fit 320/390/760/1100/1440px viewports. Verified light/dark themes, full edits, quantities/units, recipe contributions and serving scaling, failure/retry behavior, database persistence across browser sessions, duplicate/delete flows, and account isolation.

The development app was restarted after the final build before the last browser runs. Browser checks use a bounded 20-second expectation for fresh-context WASM hydration; `networkidle` alone can precede client readiness. The grocery theme check waits for loaded list items before invoking a client event handler after a full navigation.

Browser scripts: [`test-kitchen-browser.py`](../../scripts/tests/test-kitchen-browser.py), [`test-pantry-browser.py`](../../scripts/tests/test-pantry-browser.py), and [`test-grocery-list-browser.py`](../../scripts/tests/test-grocery-list-browser.py). Local results and screenshots are saved outside the checkout under `/workspace/noms-kitchen-e2e`, `/workspace/noms-kitchen-pantry-e2e`, and `/workspace/noms-kitchen-grocery-e2e`.

No schema change or production deployment was performed. Physical devices and other browser engines were not validated.


## Ingredient-only follow-up validation — 2026-10-26

The pantry interface now manages ingredients only. Both requested descriptions are in the shared header. Recipe matching remains deferred despite the intended-use wording in the pantry description.

Strict server and WASM Clippy checks, Rust/RSX formatting, and patch whitespace checks passed. Chromium passed 18 updated pantry checks and 8 Kitchen navigation checks, with no JavaScript page errors. The pantry suite verifies that equipment controls are absent, creation retains an unsubmitted ingredient, and a legacy equipment row remains in storage while being excluded from search, counts, and displayed inventory. Ingredient editing, quantities/units, out/restore, retries, browser-session persistence, duplication, profile switching, removal/deletion, and account isolation continue to pass.

These focused browser checks replace equipment-management cases in the earlier pantry suite. New result files and screenshots are under `/workspace/noms-ingredient-only-e2e` and `/workspace/noms-ingredient-only-kitchen-e2e`. No schema migration or production deployment was performed.
