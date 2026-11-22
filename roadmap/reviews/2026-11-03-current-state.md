# Application current-state review — 2026-11-03

**Baseline:** fetched `origin/main`, commit
`c112ca9a194917dd811e031e388c4d3e73ae187b` (PR #56).
**Method:** routes, API/database contracts, migrations, UI, maintained docs,
tests/browser scripts, merge history and GitHub CI. Includes PRs #50–56; the
immutable revision identifies the reviewed code regardless of author dates.

This is an implementation and roadmap reconciliation, not a fresh exhaustive
runtime regression, production inspection, security audit, physical-device test,
screen-reader run or performance benchmark. No runtime code/database changes.
Historical verification is distinguished from checks performed for this review.

**PR preparation:** the documentation branch is based on later main revision
`f533478366a2f2d03ea58fb48bbb0bafece4e51b` (PR #58). Its About-tour entrance and
rendering refinements do not change this feature inventory or its priorities.
The CI evidence below still refers to the explicitly reviewed baseline.

## Assessment

Noms has a substantial working web app: public discovery, authenticated recipe
authoring, drafts/history/forks, collections, media editing, portability and
Kitchen. The next gaps are ingredient/Kitchen integration, specific production
follow-ups and release acceptance/automation. Several foundation features were
incorrectly still Backlog/Ready; the mobile PR was merged despite its pending
status. The board also omitted many existing numbered items.

## Implemented capabilities and boundaries

| Area | Implemented | Remaining boundary | Evidence |
| --- | --- | --- | --- |
| Shell/platform | Typed routes, desktop/drawer and mobile navigation, themes, overlays and account hub. | Native feature flags do not establish packaged native releases. PWA/offline delivery remains planned. | [Routes](../../src/main.rs), [components](../../src/components). |
| Authentication/accounts | Google/GitHub login, explicit linking, browser-bound OAuth, revocable sessions, profile/avatar editing, unlink/logout/deletion recovery. | Original automatic email-linking/stateless logout proposals superseded. Real-provider deployment and independent final assessment remain separate. | [Auth](../../src/auth), [settings](../../src/pages/settings), [production contract](../../docs/production-security.md). |
| Display names | Database uniqueness by trimmed case-insensitive name, login/profile collision feedback. | Production migration state not inspected. | PR [#53](https://github.com/SamKoCPSC/Noms/pull/53), [schema](../../migrations/schema.sql). |
| Recipes | CRUD, private/unlisted/public visibility, servings/scaling, ingredients/steps/equipment, tags and public/owner reading. | Original overwrite-only description superseded by versioning. | [API](../../src/api/recipe.rs), [types](../../src/types.rs). |
| Drafts/history/forks | Private autosave, explicit publication, immutable full snapshots, restore, comparison, attribution and lineage graphs. | Real touch/screen-reader acceptance remains; history graphs are not cooking execution graphs. | [Versioning](../../src/db/versioning.rs), [NOMS-010](../implementation-plans/NOMS-010-versioning-drafts-branching.md). |
| Recipe names | Saved names unique per owner after case/whitespace normalization. Drafts/history do not reserve names; confirmed collisions preserve drafts. | Existing duplicates require explicit migration preflight/remediation. | PR [#51](https://github.com/SamKoCPSC/Noms/pull/51), [policy](../../docs/recipe-name-policy.md). |
| Collections | Hierarchy, membership, moves/reorders, visibility/sharing, covers and recovery. | Sharing does not broaden member-recipe visibility. | [API](../../src/api/collection.rs), [NOMS-011](../implementation-plans/NOMS-011-recipe-collections.md). |
| Resource names | Owner-wide normalized uniqueness separately for collections, pantries and grocery lists; collections span all folders. Typed conflicts and atomic import/migration rejection. | No silent renaming/deletion. Pantry copy names can conflict. | PR [#56](https://github.com/SamKoCPSC/Noms/pull/56), [policy](../../docs/resource-name-policy.md). |
| Search | Public Explore/profile browsing; authenticated recipe/user/collection/All search, URL-backed filters/pagination and live title suggestions. | Full Search still requires login. Explore tags are informational spans, not working filters. | [API](../../src/api/search.rs), [Search](../../src/pages/search.rs), [Explore](../../src/pages/explore.rs), PR [#50](https://github.com/SamKoCPSC/Noms/pull/50). |
| Images/media | Managed uploads/references, recipe/step/cover/avatar images, editing, bounded browser processing and recovery. | Real-storage/device/decoder diversity and final release checks remain separate. | [Uploads](../../src/api/upload.rs), [media](../../src/db/media_assets.rs), [lifecycle](../../src/utils/image_processing_lifecycle.rs). |
| Portability | Clipboard/provider import to private drafts; recipe JSON/ZIP, collection-tree JSON, image recovery, offline HTML and cookbook/HTML ZIP exports. | Direct URL fetching, Schema.org and complete cross-browser/provider compatibility deferred. | [NOMS-016](../implementation-plans/NOMS-016-recipe-import-export.md), [collection API](../../src/api/collection_portability.rs), [HTML plan](../spikes/NOMS-016-html-export.md). |
| Ingredient authoring | Vocabulary/aliases/cache; optional flags, independent alternatives and before/after modifiers preserved through history/portability. | Wider metadata dimensions remain discovery. Recognition does not imply pantry equivalence. | [NOMS-017](../implementation-plans/NOMS-017-recipe-metadata-autocomplete-vocabulary.md), [options](../implementation-plans/recipe-ingredient-options.md). |
| Units/equipment | Required precise units/validation; optional local equipment suggestions. | No conversion. New blank/count/container/approximate units rejected; legacy reads/exports retained. Equipment excluded from pantry checks. | [Units](../../docs/ingredient-unit-autocomplete.md), [equipment](../../docs/equipment-autocomplete.md). |
| Groceries | Multiple persistent independent lists, manual/accessibly sourced recipes, serving-aware totals, full edits/checking and copied provenance. | Copies all primaries; no optional filtering, alternative choice, missing-only generation or live source sync. | [Calculation](../../src/grocery.rs), [API](../../src/api/grocery_list.rs). |
| Pantry | Multiple named private pantries, ingredients, explicit availability, optional quantities. | Equipment excluded from current UI. Quantities ignored for feasibility; shared households not implemented. | [API](../../src/api/pantry.rs), [inventory plan](../implementation-plans/NOMS-018-pantry-inventory.md). |
| Feasibility | Case-insensitive presence in recipe details and Pantry's owned/collected recipe list, including nested collections. | Missing optional primaries still block; alternatives ignored. No near-cookable ranking. | [Matcher](../../src/pantry/feasibility.rs), [library query](../../src/db/pantry_recipes.rs). |
| Public pages/About | Introduction, How-to and Rust About tour: kitchen, office, bedroom, potluck and garden, loading feedback/assembly transitions. | Original no-production-3D proposal superseded. Current fallback/device/accessibility/performance acceptance remains. | [About](../../src/pages/about.rs), [tour](../../src/pages/about_tour), PRs [#54](https://github.com/SamKoCPSC/Noms/pull/54), [#55](https://github.com/SamKoCPSC/Noms/pull/55). |
| Visual/mobile | Painted Kitchen/Introduction/Accounts, shared raised/inset surfaces, compact continuous editor, confirmable rows, keyboard-aware autocomplete. | Simulated Chromium viewports do not establish actual keyboard, safe-area, native activation or assistive-technology behavior. | PRs [#6](https://github.com/SamKoCPSC/Noms/pull/6), [#45](https://github.com/SamKoCPSC/Noms/pull/45), [#47](https://github.com/SamKoCPSC/Noms/pull/47), [#49](https://github.com/SamKoCPSC/Noms/pull/49), [#52](https://github.com/SamKoCPSC/Noms/pull/52). |
| Development/release | Isolated stacks, mock auth, pinned services, schema tooling, CI/release images and guarded staging login. | External deployment, backups/restore and production migration state not inspected. | [Setup](../../docker/codex-cloud/README.md), [CI](../../.github/workflows/ci.yml), [staging login](../../docs/staging-test-login.md). |

README claims of direct URL import and likes/comments overstated delivered
functionality. Recipe commentary is authored content, not a social comment
thread. No likes/comments workflow was found in the reviewed routes, types,
API or database modules.

## Production follow-up reconciliation (NOMS-021)

Original findings are historical evidence, not freshly reproduced bugs. This
matrix is source-based triage; reproduce residuals before implementation.

| Checkpoint | Assessment | Evidence / remaining work |
| --- | --- | --- |
| CP1 dashboard projections | Open | `list_my_recipes`/`list_recipe_drafts` still return full models. Add bounded cards/pagination while preserving recovery. |
| CP2 aggregate search terms | Open | Recipe counting exists; user/collection validation lacks the same aggregate count in `src/search.rs`. |
| CP3 cleanup fairness/config | Open | `media_assets.rs` prioritizes `delete_claimed`; interval config accepts zero. Reproduce and ensure fair forward progress/safe configuration. |
| CP4 test reliability | Partial | CI already uses `--test-threads=8`. PR #56 includes fixture cleanup isolation (`ebd7809`); verify repeated clean runs and investigate residual setup collisions/timeouts. Packaging now relies on CI-validated release images. |
| CP5 processing termination | Implemented, scoped acceptance recorded | Lifecycle/client code owns deadlines/cancellation; NOMS-020 records corrupt-input, late completion and resumed-page checks. Device limits remain. |
| CP6 graph/lineage budgets | Partial | History/graph/comparison use shared work budgets; attribution/lineage use request limits and separate database wrappers. Reconcile concurrency and aggregate deadlines. |
| CP7 indexed username | Open | `(username \|\| '') = $1` still used; no new indexed-plan proof found. |
| CP8 Explore tags | Old toggle superseded; filtering deferred | Tags now informational. Agree filtering behavior before adding it; do not report a broken selected-tag control. |
| CP9 readability | Ongoing | Later extractions exist; no completed bounded hotspot inventory recorded. Keep refactors separate. |
| CP10 route metadata | Open | Informational metadata exists; complete application-route replacement not established. |
| CP11 anonymous Search | Open | Page uses `AuthRequired`; core functions require session. Public Explore/title suggestions do not close this contract. |
| CP12 regression coverage | Partial | Many browser scripts exist; no required CI browser smoke job. Manual guide still assumes fixed ports/old workflows. Consolidate current checks. |
| CP13 release verification | Open | CI is not device, screen-reader, production storage/deployment or independent security acceptance. |

NOMS-015's hardening should not remain Ready, but no completed CP10 independent
verdict was found. Map later public-profile acceptance to CP9C before closing
its exact gate. An unavailable browser tool in an old session is not a permanent
current blocker.

## Validation evidence

- PR [#6](https://github.com/SamKoCPSC/Noms/pull/6) confirmed merged; remove old
  awaiting-merge status. Historical evidence remains linked from NOMS-022.
- PR [#56](https://github.com/SamKoCPSC/Noms/pull/56), head
  `397133b9805fa2a267f8b7d74f7d47b5777753a2`, confirmed merged. Server/WASM checks
  and Clippy, formatting, tests, search integration, security-policy and schema
  checks passed. PR image jobs skipped as expected.
- Main [CI run 41681882426](https://github.com/SamKoCPSC/Noms/actions/runs/41681882426)
  passed checks/Clippy for both targets, formatting, tests, search integration
  and security-policy checks. Main schema-check skipped; PR schema passed.
  Release-image publishing subsequently completed successfully on this same
  run. This proves image publishing, not external deployment health.
- Earlier PRs #47/#49 needed retries for database setup collisions/timeouts.
  Recent cleanup isolation/current pass do not prove all intermittency resolved.
- Browser scripts for Kitchen, pantry recipes, ingredient options, editor,
  autocomplete, title suggestions and name conflicts were inventoried, not all
  rerun. No fresh exhaustive test count or staging acceptance claimed.

## Recommended sequence

1. Agree optional/substitute semantics and implement proposed NOMS-023 in both
   check surfaces, preserving presence-only behavior and access rules.
2. Define missing-only grocery generation under NOMS-018: pantry, servings,
   optional items and alternatives; independent editable snapshots.
3. Stabilize fixtures/CI and require browser smoke under NOMS-021; map existing
   scripts to stable manual cases and retain failure artifacts.
4. Close dashboard/search/cleanup/metadata/public-Search gaps and the separate
   final independent security review.
5. Finish phone/tablet, graph-touch, screen-reader and current About-tour
   fallback/performance acceptance.

Later backlog: near-cookable ranking, explicit purchased-item transfer, shared
pantries, meal planning and offline grocery sync. Direct URL/Schema.org belongs
to NOMS-016; broader metadata to NOMS-017; cooking execution graphs to NOMS-019.
This review does not approve their implementation.

## Documentation decisions

Restore all numbered items and honest statuses. Add current-state notices to
stale issue/plan summaries while preserving historical checklists. Correct
README claims. Track new name/artwork/editor work in existing feature tracks.
Add NOMS-023 as Backlog with explicit open decisions; no application changes.

## Documentation verification

The update changes Markdown only. The board indexes NOMS-000 through NOMS-023
exactly once; introduced local links and whitespace were checked. Source/merge
and CI evidence refer to the pinned baseline above. No new runtime test result
is inferred from these documentation checks.

The local workspace reached its storage limit during this review. Documentation
was prepared in temporary storage and saved to the repository; this is a local
development constraint, not evidence of a production defect. No application
builds or new browser suites were run for this documentation-only update.
