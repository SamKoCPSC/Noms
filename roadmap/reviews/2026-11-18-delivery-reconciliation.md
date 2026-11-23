# Noms delivery reconciliation — 2026-11-18

**Reviewed:** GitHub `main` at `e6dea778a40a1f93ad5f682188e7aab4a7d6e270`.

**Comparison:** [November 3 current-state review](2026-11-03-current-state.md)
and the first-parent history since its source revision `c112ca9`.

This record maps subsequent merged work to the roadmap. Verification below
comes from the linked delivery records. The board's **Done** status applies
to the closed roadmap item. Future work gets a new task. The notes below
record what was verified and where the delivery evidence stopped.

| Area | What shipped | Roadmap and delivery evidence |
| --- | --- | --- |
| Public Search | Guests can open Search from direct links, Introduction and navigation and use Recipes, Collections, Cooks and All. Guest results are public. Signed-in users can additionally see content they own. Visibility applies to suggestions, counts, facets and ancestor breadcrumbs. Collection and cook searches enforce the aggregate advanced-term limit. | [NOMS-012](../issues/NOMS-012-recipe-search.md), [NOMS-021 CP2/CP11](../issues/NOMS-021-production-readiness-follow-ups.md), [public Search contract](../../docs/public-search.md), [PR #61](https://github.com/SamKoCPSC/Noms/pull/61) |
| Pantry feasibility and grocery creation | Missing optional ingredients do not block cooking. Both pantry checks distinguish them from full matches. Grocery generation can copy missing ingredients from a selected pantry, use recipe serving targets and include optional ingredients by default with an exclusion toggle. Presence checks use case-insensitive names and availability. | [NOMS-018](../implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md), [NOMS-023](../issues/NOMS-023-kitchen-ingredient-options.md), [PR #62](https://github.com/SamKoCPSC/Noms/pull/62), [PR #64](https://github.com/SamKoCPSC/Noms/pull/64) |
| Purchased groceries to pantry | Select checked grocery rows and a destination pantry. Existing ingredients become available and keep their amounts. New entries copy amounts and units. Keep the rows by default or remove selected rows in the same transaction. Uncertain saves ask users to refresh before retrying. | [NOMS-018](../implementation-plans/NOMS-018-pantry-meal-planner-grocery-list.md), [PR #70](https://github.com/SamKoCPSC/Noms/pull/70) |
| Draft conflict recovery | Both recipe editors pause autosave on conflicting revisions. A three-way review proposes independent changes, asks the user to settle competing edits and confirms against the reviewed revision. | [NOMS-024](../issues/NOMS-024-draft-conflict-recovery.md), [PR #77](https://github.com/SamKoCPSC/Noms/pull/77) |
| Collection JSON import | Editable collection names and recipe titles resolve conflicts in the preview. The original archive stays checksum-verifiable and writes remain atomic. | [NOMS-025](../issues/NOMS-025-collection-import-name-recovery.md), [PR #80](https://github.com/SamKoCPSC/Noms/pull/80) |
| About | Twelve story stops in a baked Three.js village, guided camera navigation, reading fallback and free exploration across the fenced world. | [NOMS-026](../implementation-plans/NOMS-026-about-village-tour.md), [PR #89](https://github.com/SamKoCPSC/Noms/pull/89), [PR #103](https://github.com/SamKoCPSC/Noms/pull/103) |
| Introduction | Animated Chef Penguini kitchen with playback control, reduced-motion and poster fallback, plus mobile framing and palette refinements. | [NOMS-027](../issues/NOMS-027-animated-introduction-kitchen.md), [PR #91](https://github.com/SamKoCPSC/Noms/pull/91), [PR #93](https://github.com/SamKoCPSC/Noms/pull/93) |
| Reusable art and shared UI | Penguini's SVG and 3D animation assets, Kitchen Crew mascots, reusable dioramas, stronger inset rims and consistent Introduction menu alignment. | [NOMS-028](../issues/NOMS-028-reusable-brand-and-scene-assets.md), [NOMS-020](../implementation-plans/NOMS-020-visual-design-refinement.md), [PR #73](https://github.com/SamKoCPSC/Noms/pull/73), [PR #75](https://github.com/SamKoCPSC/Noms/pull/75) |
| Test and release tooling | Private sockets and bounded PostgreSQL fixture readiness address test-cluster collisions. Updated dependency locks and PostgreSQL extension pins restored the affected CI jobs. | [NOMS-021](../issues/NOMS-021-production-readiness-follow-ups.md), [fixture diagnosis](../../docs/database-test-isolation.md), [PR #60](https://github.com/SamKoCPSC/Noms/pull/60), [PR #79](https://github.com/SamKoCPSC/Noms/pull/79), [PR #83](https://github.com/SamKoCPSC/Noms/pull/83) |

## Recorded validation and possible follow-ups

- Search and Kitchen PRs record real browser and database tests, including guest
  visibility, retry behavior, pantry ownership and list persistence. Kitchen
  substitution rules, near-cookable ranking, shared pantries, meal planning and
  offline synchronization remain separate scope.
- Draft and import recovery have focused tests and component previews. Their
  delivery records leave authenticated multi-tab draft and live import browser
  replay open. The About and Introduction checks cover camera, touch/keyboard
  inputs and fallback behavior across relevant revisions. Physical devices,
  assistive technology and production performance acceptance remain open.
- NOMS-021 recorded dashboard projections, cleanup fairness, graph/lineage
  work limits, identity lookup, route metadata and release regressions as possible
  follow-ups. NOMS-015 recorded the independent security review separately. The conditional inactive SQLite
  audit exception recorded in PR #79 expires on November 30, 2026. Review it
  against the current dependency graph and policy before expiry.
- Merge and PR validation establish the delivery baselines above. Production
  rollout and release acceptance have their own evidence.
