# NOMS-020 — How-to page

Base: `516103a` (accepted shared design foundation). Branch: `codex/NOMS-020-how-to`.
Worktree: `/home/samko/.codex/worktrees/noms-020-how-to/Noms`.

Reference: [G2 Cozy How-to](../assets/NOMS-020/public-how-to-cozy-v2.png), qualified by the [cozy v2 notes](../assets/NOMS-020/public-cozy-v2-prompts.md) and the current NOMS-020 plan. This is one page in the first four-page batch.

## Scope and implementation

- Three raised numbered cards for creating, choosing visibility and following version history, with readable miniature recipe/history examples, inset wells, coral badges and sage notes.
- Retain Fredoka/Nunito, the existing palette, and shared elevation tokens. A small decorative recipe-card, spoon and herb vignette adds character without replacing the app logo or requiring production logo assets.
- Preserve all ten existing topic anchors and the complete workflow guide. Keep the overview compact; supporting photo/editing/scaling/collection/discovery/fork instructions follow the native FAQ disclosures.
- Correct obsolete How-to copy: Explore searches public titles/descriptions on the server and supports Load more; Search requires sign-in and has recipe, collection and cook scopes. Preserve the distinction between private published recipes and unpublished drafts.
- Native links, named/focusable anchor destinations, semantic example figures and definition lists, and native details/summary. The examples contain no pretend editable controls. The surrounding app already provides the main landmark.
- Desktop: sticky contents rail, three overview columns and two supporting-topic columns. Below 1200px: contents rail with stacked cards. Below 700px: non-sticky two-column topic list and single-column content; the decorative illustration is omitted. The contents rail is independently scrollable on short viewports.
- Existing dark-theme tokens and forced-color boundaries remain supported. No new animation. Reduced-motion behavior comes from the existing shell.

Owned files: `src/pages/how_to.rs`, `assets/how-to.css`, and this evidence document. The route loads its own namespaced stylesheet. Other public pages, shared controls, global CSS, backend/auth contracts and the canonical multi-page plan are not edited, allowing the parallel page branches to integrate independently.

## Acceptance

- [x] Three-card desktop composition reviewed against G2, using actual fonts and tokens.
- [x] Phone, portrait tablet and short desktop render without horizontal overflow; contents and FAQ remain reachable.
- [x] Existing deep links retain unique focusable destinations; representative links and FAQ controls verified with the keyboard below navigation.
- [x] Existing dark theme, reduced motion and forced colors checked.
- [x] Rendered-content tests, existing marketing contracts, formatting and both app build targets pass.
- [x] Independent review and final diff review completed.

## Validation — 2026-09-24

- `SQLX_OFFLINE=true CARGO_TARGET_DIR=/tmp/noms-how-to-checks CARGO_BUILD_JOBS=4 cargo test --features server pages::`: **146 passed**, including the two new rendered How-to tests and existing marketing contracts.
- `cargo fmt --check` and `git diff --check`: passed.
- `just up noms020-how-to-preview`: isolated Docker deployment built server and WASM targets and served `/how-to` successfully. Preview: `http://127.0.0.1:32794/how-to` (local, not a production deployment).
- Browser review: 1440×900 desktop, 768×1024 portrait tablet, 390×844 phone and 1280×600 short desktop; no horizontal overflow. On short desktop the sticky contents navigation is constrained to 472px and independently scrollable.
- Keyboard: Collections link moved focus to `#collections`; Common questions link reached the FAQ; Enter opened its first disclosure and Space closed it. The target remained below the shared navigation. Rendered tests cover unique destinations for all ten original topic links plus FAQ.
- Existing dark theme remained readable. Forced-colors mode retained solid panel boundaries; reduced-motion emulation selected `scroll-behavior: auto`. These are browser smoke checks, not a full accessibility audit.
- Independent reviewer: **PASS**, no blocking or should-fix findings. An informational FAQ-link selector specificity issue was corrected. Final review also reduced hero height and moved getting-started guidance into the supporting grid to keep the desktop overview close to G2.
- Temporary visual evidence (not tracked): `/tmp/noms-how-to-evidence/before-desktop.png`, `desktop-light.png`, `tablet.png`, and `phone.png`.

The default web-only host test configuration hit an existing `AvatarEditSource` dead-code error; the supported server-feature test configuration above passed. No backend behavior changed, and no live sign-in or recipe mutations were needed to validate this static guide. Generated Tailwind churn from local startup was excluded. Implementation, page tests and evidence form one cohesive page-upgrade commit; no unrelated refactor was needed.
