# NOMS-020 checkpoint 2 — Explore X7C

Started 2026-09-22 from `56c120f` in the user's existing isolated `codex/NOMS-020-visual-design-refinement` worktree. Navigation remains the accepted N1/N4 slice. **Implementation is awaiting browser visual/interaction acceptance; checkpoint 2 is not closed.**

Implementation commit: `dab9d77`. Documentation is a separate local commit; nothing is pushed.

## Contract and implementation

- X7C opt-in framed vertical recipe cards: existing cream surface, raised shadow, Fredoka/Nunito, 12px frame, inset 16:9 image, title (two lines), description (two lines), available preparation/cooking time and servings. No disabled decorative rating/bookmark controls in this variant.
- Grid uses `repeat(auto-fill, minmax(min(100%, 260px), 1fr))`, 24px gaps and the existing page container. Column count follows available width, not fixed breakpoints. Sub-260px containers can shrink without overflowing. Phone/tablet discovery still needs browser validation.
- Recipe and author are separate native links, with visible keyboard focus. Images retain the shared edited-image crop/thumbnail behavior; missing/failed recipe images show a quiet text fallback. Missing/failed avatars use initials. Long titles/handles wrap rather than widen the grid.
- Summary API supplies a handle and optional avatar, not a display name. Show the real handle, not an invented display name or an extra request per card. No API/schema/auth/visibility changes.
- Labelled substring search still queries full descriptions on the server. Debounce and request-generation checks preserve stale-response protection. Search clears prior results and enters pending state immediately.
- Initial loading, no recipes, no matches and error states are explicit. Failed pagination keeps existing cards. Retry repeats the failed offset; Load more is synchronously guarded against rapid clicks, disabled while pending and unavailable after a failure until retry succeeds. Result count says “recipes loaded,” not a fabricated total.
- Community tags are noninteractive labels. The previous cosmetic selected state never filtered results; functional tag filtering remains a separately scoped follow-up. Do not describe these labels as filters.
- `RecipeCard.framed` defaults to false: Dashboard, Search and Profile keep their existing presentation. Navigation, footer, backend and selected artwork are untouched. Production EC1/S1 assets remain deferred. Existing theme tokens are retained; dark redesign is deferred.

## Ownership and evidence

Owned implementation paths: `src/pages/explore.rs`, `src/components/base/recipe_card.rs`, and the scoped Explore/framed-card section of `assets/main.css`. Master-plan status and this document are the documentation slice. No new JavaScript or dependencies.

Tests first: pagination/source-contract tests initially failed on missing `next_page_offset` (`/tmp/noms020-explore-red.log`). Four Explore tests pass, covering offset bounds/loading/error/end guards, reactive fetch inputs, opt-in layout, accessible labels and honest tag semantics. Rendered-card tests additionally exercise actual Dioxus markup for metadata, fallback, separate links, absence of placeholder buttons and the unchanged legacy structure. Source guards are not browser tests and do not execute out-of-order asynchronous completions.

Independent review: PASS, no blocking or should-fix findings. Reviewer independently ran all four Explore tests and `git diff --check`. Informational limitation: async lifecycle and browser layout remain unverified at runtime.

Final gate: `just ci` passes (`/tmp/noms020-explore-ci-final.log`), including both build and Clippy targets, 747 tests passed / 12 ignored, schema/policy checks, tracked-secret and deployment-artifact checks, and dependency audit (eight existing policy-allowed warnings). Focused rendered-card command: `SQLX_OFFLINE=true cargo test --features server components::base::recipe_card::tests` — two passed (`/tmp/noms020-explore-cards.log`). Independent final re-review of those tests and acceptance documentation also passes. The gate used elevated access for the dependency advisory cache; no policy was weakened.

Deployment: restarted only `noms-noms020-picker-20260922-app-1` in the existing isolated deployment, preserving all user fixtures. Server logs show a successful rebuild and healthy local responses. Local review URL: `http://127.0.0.1:32779/explore`. Startup is not feature acceptance. Startup-only Tailwind utility churn is excluded.

Browser verification was attempted but the browser tool explicitly rejected access under its URL policy. No alternate surface, raw browser command or network workaround was attempted. No new screenshot, viewport geometry, keyboard flow or live search/pagination claim is made for this slice.

## Remaining acceptance checklist

- [ ] Inspect populated desktop X7C against the selected board, including edited and failed images, long text, missing metadata and author links.
- [ ] Verify phone, portrait tablet, wide desktop, zoom/short viewport and adaptive column transitions; no horizontal overflow.
- [ ] Exercise search, rapid typing/out-of-order responses, empty results, initial/pagination failure and same-offset retry with local fixtures.
- [ ] Exercise keyboard focus and screen-reader status announcements; verify pending pagination feedback.
- [ ] Check representative legacy cards and existing dark/reduced-motion/forced-colors behavior in the browser.
- [ ] Review navigation and Explore together before closing checkpoint 2 and opening downstream page streams.

Automated checks cannot close these browser acceptance items. No database fixture data was modified for this implementation.
