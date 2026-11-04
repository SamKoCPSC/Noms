# Noms — Product Roadmap

**Last reconciled:** 2026-11-03 against `main` at `c112ca9a194917dd811e031e388c4d3e73ae187b`.

The [current-state review](reviews/2026-11-03-current-state.md) records delivered
features, source/merge evidence, remaining work and validation limits. All
numbered items are indexed here; older plans/checklists preserve design history.

## Statuses

| Indicator | Meaning |
| --- | --- |
| ⚪ Backlog | Proposed work/discovery; implementation scope not yet approved. |
| 🔵 Ready | Scoped and approved work that has not started. |
| 🟡 In Progress | Partly delivered or stated acceptance remains open; does not imply someone is actively working on it. |
| ✅ Done | The stated baseline is merged with recorded verification, not blanket production/device/accessibility certification. |

## Board

| ID | Title | Status | Delivered baseline and remaining scope |
| --- | --- | --- | --- |
| [NOMS-000](issues/NOMS-000-open-code-editor.md) | Open code editor | ✅ Done | Foundational setup. |
| [NOMS-001](issues/NOMS-001-design-document.md) | High-level design | ✅ Done | Design delivered; later decisions supersede historical architecture. |
| [NOMS-002](issues/NOMS-002-initial-project-setup.md) | Project and infrastructure setup | ✅ Done | Rust/Dioxus, PostgreSQL, media, CI and deployment configuration exist. Release operations: NOMS-021. |
| [NOMS-003](issues/NOMS-003-ui-scaffold.md) | UI scaffold and shell | ✅ Done | Routes, themes, navigation and shared components implemented. Later design/mobile work: NOMS-020/022. |
| [NOMS-004](issues/NOMS-004-oauth-auth.md) | Google and GitHub OAuth | ✅ Done | Login, sessions and redirects implemented; explicit linking/revocable sessions supersede the original proposal. |
| [NOMS-005](issues/NOMS-005-user-profile.md) | Profiles and account management | ✅ Done | Profile/avatar editing, provider management, logout/deletion and account hub. Database-unique display names (PR #53). |
| [NOMS-006](issues/NOMS-006-auth-security-hardening.md) | Initial auth hardening | ✅ Done | Historical baseline delivered; later hardening/release assessment: NOMS-015/021. |
| [NOMS-007](issues/NOMS-007-minor-ui-fixes.md) | Original UI/routing cleanup | 🟡 In Progress | Largely superseded by current shell/design work. Per-case acceptance remains unreconciled; do not reopen obsolete visual proposals. |
| [NOMS-008](issues/NOMS-008-recipe-crud.md) | Recipe CRUD, visibility and discovery | ✅ Done | CRUD, visibility, scaling and public reading implemented. Unique owner recipe names (PR #51); drafts/history: NOMS-010. |
| [NOMS-009](issues/NOMS-009-image-uploads.md) | Images and editing | ✅ Done | Recipe/step/avatar/cover uploads, editing, managed references and recovery implemented. Device/storage release checks separate. |
| [NOMS-010](issues/NOMS-010-versioning-drafts-branching.md) | Versions, drafts and forks | ✅ Done | Immutable full snapshots, private autosave, publication, restore, comparison and graphs. Reverse-diff proposal superseded. |
| [NOMS-011](issues/NOMS-011-recipe-collections.md) | Collections | ✅ Done | Hierarchy, memberships, move/reorder, visibility/sharing and covers. Owner-wide names (PR #56); portability: NOMS-016. |
| [NOMS-012](issues/NOMS-012-recipe-search.md) | Search and title suggestions | ✅ Done | Authenticated recipe/user/collection search, URL state and title suggestions (PR #50). Anonymous full Search/bounds follow-ups: NOMS-021. |
| [NOMS-013](implementation-plans/NOMS-013-marketing-pages-3d-experience.md) | Introduction, About and How-to | 🟡 In Progress | Public pages, painted art, search-first landing and five-scene Rust tour implemented. Original no-3D scope superseded; release acceptance remains. |
| [NOMS-014](issues/NOMS-014-isolated-development-environments.md) | Isolated development environments | ✅ Done | Disposable infrastructure, mock auth, cloud setup and browser tooling delivered. |
| [NOMS-015](issues/NOMS-015-security-hardening.md) | Integrated security hardening | 🟡 In Progress | OAuth, identity, media, quotas and bounded reads implemented. CP9C acceptance reconciliation and CP10 final independent review remain. |
| [NOMS-016](implementation-plans/NOMS-016-recipe-import-export.md) | Recipe and collection portability | 🟡 In Progress | Clipboard/provider import, JSON/ZIP round trips and offline HTML implemented. Direct URL/Schema.org and compatibility closeout deferred. |
| [NOMS-017](implementation-plans/NOMS-017-recipe-metadata-autocomplete-vocabulary.md) | Metadata and autocomplete | 🟡 In Progress | Ingredient vocabulary/cache, unit/equipment suggestions and structured ingredient options implemented. Wider metadata remains discovery. |
| [NOMS-018](implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md) | Kitchen, pantries and groceries | 🟡 In Progress | Persistent lists/pantries, recipe-derived lists, presence checks and owned/collected cookable recipes implemented. Optional-aware checks are implemented; substitutions, shortages and shopping/offline work remain. |
| [NOMS-019](implementation-plans/NOMS-019-visual-recipe-flow.md) | Visual recipe flow and cooking timeline | ⚪ Backlog | Discovery/prototype required; implemented version-history graphs are a different feature. |
| [NOMS-020](implementation-plans/NOMS-020-checkpoint-0-baseline.md) | Visual design refinement | 🟡 In Progress | Page redesigns, overlays, painted icons and raised/inset refinements integrated. Final acceptance/device/screen-reader limitations remain. |
| [NOMS-021](issues/NOMS-021-production-readiness-follow-ups.md) | Production readiness and reliability | 🟡 In Progress | Some checkpoints delivered. Reconciled residuals: projections, search/cleanup/lineage bounds, public Search, metadata, CI reliability and release coverage. |
| [NOMS-022](issues/NOMS-022-mobile-responsive-design.md) | Mobile responsive baseline | ✅ Done | PR #6 merged; compact continuous editor and keyboard-aware popovers also merged. Physical-device/assistive-technology release checks remain. |
| [NOMS-023](issues/NOMS-023-kitchen-ingredient-options.md) | Optional/substitute-aware Kitchen checks | 🟡 In Progress | Optional ingredients are nonblocking with distinct full/optional-missing results. Substitution semantics remain backlog. |

## Recommended next work

These are proposed priorities, not approval to implement new features.

1. **NOMS-023:** optional-only matching is implemented in both feasibility paths.
   Agree substitute rules before extending it further; keep checks presence-based.
2. **NOMS-018 shortages:** agree pantry, servings, optional/alternative choices
   before generating independent editable missing-ingredient lists.
3. **NOMS-021 reliability:** verify recent fixture isolation and investigate
   residual database setup collisions/timeouts. The eight-thread cap already
   exists. Establish a required browser smoke gate using current scripts.
4. **NOMS-021/015 release gaps:** projections, aggregate search/cleanup/lineage
   work, anonymous Search, route metadata and final independent review.
5. **NOMS-020/022/013 acceptance:** actual keyboards/safe areas, graph touch,
   screen readers and current About-tour fallback/performance coverage.

Near-cookable ranking, purchased-item transfer, shared pantries, meal planning
and offline sync remain later NOMS-018 proposals. Likes/comments, installable
PWA and native packaging are not delivered features.

## Conventions

- **IDs:** stable `NOMS-NNN` references; do not renumber existing work.
- **Evidence:** distinguish implementation, historical tests, current CI,
  browser/device acceptance and deployment. CI cannot close unperformed gates.
- **History:** dated logs preserve their revision's state. Current-state notices
  and this review supersede stale status summaries and old scope assumptions.
- **New proposals:** settle contracts/acceptance before promoting to Ready.
