# Noms — Product Roadmap

**Last reconciled:** 2026-11-18 against `main` at `e6dea778a40a1f93ad5f682188e7aab4a7d6e270`.

The [November delivery review](reviews/2026-11-18-delivery-reconciliation.md) links merged features to the board and records verification and potential follow-ups. The [November 3 review](reviews/2026-11-03-current-state.md) preserves the earlier baseline.

## Statuses

| Indicator | Meaning |
| --- | --- |
| ⚪ Backlog | Proposed work/discovery; implementation scope not yet approved. |
| 🔵 Ready | Scoped and approved work that has not started. |
| 🟡 In Progress | Partly delivered or stated acceptance remains open; does not imply someone is actively working on it. |
| ✅ Done | The roadmap item is closed. Use the linked records for the scope delivered and the checks performed. |

## Board

| ID | Title | Status | Scope and delivery |
| --- | --- | --- | --- |
| [NOMS-000](issues/NOMS-000-open-code-editor.md) | Open code editor | ✅ Done | Foundational setup. |
| [NOMS-001](issues/NOMS-001-design-document.md) | High-level design | ✅ Done | Design delivered; later decisions supersede historical architecture. |
| [NOMS-002](issues/NOMS-002-initial-project-setup.md) | Project and infrastructure setup | ✅ Done | Rust/Dioxus, PostgreSQL, media, CI and deployment configuration exist. Release operations: NOMS-021. |
| [NOMS-003](issues/NOMS-003-ui-scaffold.md) | UI scaffold and shell | ✅ Done | Routes, themes, navigation and shared components implemented. Later design/mobile work: NOMS-020/022. |
| [NOMS-004](issues/NOMS-004-oauth-auth.md) | Google and GitHub OAuth | ✅ Done | Login, sessions and redirects implemented; explicit linking/revocable sessions supersede the original proposal. |
| [NOMS-005](issues/NOMS-005-user-profile.md) | Profiles and account management | ✅ Done | Profile/avatar editing, provider management, logout/deletion and account hub. Database-unique display names (PR #53). |
| [NOMS-006](issues/NOMS-006-auth-security-hardening.md) | Initial auth hardening | ✅ Done | Historical baseline delivered; later hardening/release assessment: NOMS-015/021. |
| [NOMS-007](issues/NOMS-007-minor-ui-fixes.md) | Original UI/routing cleanup | ✅ Done | Current shell and navigation work superseded the original styling and routing proposals. |
| [NOMS-008](issues/NOMS-008-recipe-crud.md) | Recipe CRUD, visibility and discovery | ✅ Done | CRUD, visibility, scaling and public reading implemented. Unique owner recipe names (PR #51); drafts/history: NOMS-010. |
| [NOMS-009](issues/NOMS-009-image-uploads.md) | Images and editing | ✅ Done | Recipe/step/avatar/cover uploads, editing, managed references and recovery implemented. Device/storage release checks separate. |
| [NOMS-010](issues/NOMS-010-versioning-drafts-branching.md) | Versions, drafts and forks | ✅ Done | Immutable snapshots, drafts, publication, restore, comparison and graphs. Concurrent draft edits use revision-guarded merge review (NOMS-024). |
| [NOMS-011](issues/NOMS-011-recipe-collections.md) | Collections | ✅ Done | Hierarchy, memberships, move/reorder, visibility/sharing and covers. Owner-wide names (PR #56); portability: NOMS-016. |
| [NOMS-012](issues/NOMS-012-recipe-search.md) | Search and title suggestions | ✅ Done | Recipe, collection and cook search, URL state and title suggestions. Signed-out visitors can search public content (PR #61). |
| [NOMS-013](implementation-plans/NOMS-013-marketing-pages-3d-experience.md) | Introduction, About and How-to | ✅ Done | Introduction, How-to, the About village tour and the animated Introduction kitchen are implemented (NOMS-026/027). |
| [NOMS-014](issues/NOMS-014-isolated-development-environments.md) | Isolated development environments | ✅ Done | Disposable infrastructure, mock auth, cloud setup and browser tooling delivered. |
| [NOMS-015](issues/NOMS-015-security-hardening.md) | Integrated security hardening | ✅ Done | OAuth, identity, media, quotas and bounded reads were hardened. Verification records are linked from the issue. |
| [NOMS-016](implementation-plans/NOMS-016-recipe-import-export.md) | Recipe and collection portability | ✅ Done | Clipboard/provider import, JSON/ZIP round trips, offline HTML and collection import name recovery (NOMS-025). |
| [NOMS-017](implementation-plans/NOMS-017-recipe-metadata-autocomplete-vocabulary.md) | Metadata and autocomplete | ✅ Done | Ingredient vocabulary and cache, unit and equipment suggestions, and structured ingredient options. |
| [NOMS-018](implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md) | Kitchen, pantries and groceries | ✅ Done | Persistent pantries and lists, cookable recipes, optional-aware presence checks, missing-ingredient lists, and purchased grocery transfer with keep/remove choices. |
| [NOMS-019](implementation-plans/NOMS-019-visual-recipe-flow.md) | Visual recipe flow and cooking timeline | ⚪ Backlog | Discovery/prototype required; implemented version-history graphs are a different feature. |
| [NOMS-020](implementation-plans/NOMS-020-checkpoint-0-baseline.md) | Visual design refinement | ✅ Done | Redesigned pages, overlays, painted icons, inset rim shadows and Introduction menu alignment. Reusable art is indexed in NOMS-028. |
| [NOMS-021](issues/NOMS-021-production-readiness-follow-ups.md) | Production readiness and reliability | ✅ Done | Public Search, aggregate term limits, private PostgreSQL fixture sockets, and CI audit and image fixes. |
| [NOMS-022](issues/NOMS-022-mobile-responsive-design.md) | Mobile responsive baseline | ✅ Done | PR #6 merged; compact continuous editor and keyboard-aware popovers also merged. Physical-device/assistive-technology release checks remain. |
| [NOMS-023](issues/NOMS-023-kitchen-ingredient-options.md) | Optional/substitute-aware Kitchen checks | ✅ Done | Optional ingredients are nonblocking, with distinct full and optional-missing results. Grocery generation includes optional items by default and can exclude them. |
| [NOMS-024](issues/NOMS-024-draft-conflict-recovery.md) | Recipe draft conflict recovery | ✅ Done | Three-way review and revision-guarded confirmation in both recipe editors. |
| [NOMS-025](issues/NOMS-025-collection-import-name-recovery.md) | Collection import name recovery | ✅ Done | Rename collections and recipes in the preview and retry with the original archive. |
| [NOMS-026](issues/NOMS-026-about-village-tour.md) | Guided About village and first-person exploration | ✅ Done | Twelve story stops, reading fallback, fenced exploration and resume at the saved camera position. |
| [NOMS-027](issues/NOMS-027-animated-introduction-kitchen.md) | Animated Penguini kitchen | ✅ Done | Responsive looping Introduction hero with pause, reduced motion and poster fallback. |
| [NOMS-028](issues/NOMS-028-reusable-brand-and-scene-assets.md) | Reusable mascots and 3D scenes | ✅ Done | Editable mascot art, SVG and 3D animations, diorama sources and exports. |

## Open work

[NOMS-019 — Visual recipe flow and cooking timeline](implementation-plans/NOMS-019-visual-recipe-flow.md)
is the only open item. Give future work a new NOMS ID and link back to related
delivery records as needed.

## Conventions

- **IDs:** stable `NOMS-NNN` references; do not renumber existing work.
- **Evidence:** distinguish implementation, historical tests, current CI,
  browser/device acceptance and deployment. CI cannot close unperformed gates.
- **History:** dated logs preserve their revision's state. Current-state notices
  and this review supersede stale status summaries and old scope assumptions.
- **New tasks:** assign a new NOMS ID and define scope and acceptance before promoting to Ready.
