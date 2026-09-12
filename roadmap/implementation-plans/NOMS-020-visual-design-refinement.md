# NOMS-020: Application Visual Design Refinement — Implementation Plan

**Status:** Visual direction selected — implementation scoping and validation required
**Created:** 2026-08-15
**Depends on:** Current application routes, components, design tokens, and the visual direction established by NOMS-013
**Coordination:** Feature tickets NOMS-016 through NOMS-019 should use shared primitives where stable; NOMS-020 must avoid redesigning feature behavior owned by those tickets.
**Approach:** Evolve the existing Noms identity into the selected Kitchen Console direction through an evidence-based visual audit, reusable token and component changes, and a route-by-route migration plan.

---

## Intended outcome

The application presents one deliberate, accessible, responsive visual system across public pages, authenticated workflows, dense data views, forms, and transient states.

## Planned application mark

The current planned NOMS mark is **A3 — Tilted Label Book**. It is a compact, gently clockwise-tilted closed book with an olive cover, segmented cream spine and page block, a coral elastic strap and bookmark, and a warm-white rounded label carrying `NOMS` in the application's Fredoka display face. The shallow dimensional treatment and restrained matte texture should be retained when the mark is later translated into production artwork.

Reference artwork: [`docs/design/NOMS-020/assets/noms-logo-planned-a3.png`](../../docs/design/NOMS-020/assets/noms-logo-planned-a3.png)

This is the approved visual direction for planning, not yet a production SVG/favicon. Production work must preserve legibility at favicon and app-icon sizes, provide light/dark and single-colour variants, and use the actual Fredoka font or converted outlines.

## Selected visual direction: Kitchen Console

Kitchen Console retains the warmth, tactility, and personality of the existing interface while making hierarchy and interaction state clearer. Neumorphism remains part of the product identity, but depth is functional rather than ornamental:

- **Flat:** page canvases, long-form content, and low-priority supporting regions.
- **Inset:** editors, graph canvases, input wells, and areas that receive content.
- **Raised:** buttons, cards, tabs, selectors, and other directly interactive controls.
- **Floating:** dialogs, inspectors, bottom sheets, transient trays, and focused overlays.

Visible borders, color, labels, and focus treatments must communicate boundaries and state without relying on shadows alone. The working palette combines warm cream, deep charcoal-olive, sage/olive structure, and restrained tomato-coral emphasis. Food imagery and editorial typography provide the culinary character; generic dashboard styling, glassmorphism, and developer-tool motifs do not.

These concept boards are directional references, not pixel-perfect specifications. Generated copy, data, and incidental spacing may be approximate. Implementation decisions must follow the documented visual principles, established product behavior, accessibility requirements, and shared component contracts.

## Responsive composition model

Phone, portrait tablet, and desktop are separate compositions within one system rather than scaled copies of one another. Initial visual-review targets are 390 × 844, 834 × 1194, and 1440 × 900; implementation breakpoints should be selected from content pressure and existing application conventions.

| Concern | Phone | Portrait tablet | Desktop |
| --- | --- | --- | --- |
| Primary navigation | Compact mobile navigation | Narrow collapsible icon rail | Persistent labeled sidebar |
| Dense graph direction | Top-to-bottom guided path | Top-to-bottom spatial graph | Left-to-right spatial graph |
| Version details | Bottom sheet or dedicated view | Resizable bottom sheet | Docked or floating side inspector |
| Comparison controls | Sticky bottom action | Floating compare tray | Persistent compare tray |
| Touch and density | Single focused task | Touch-first workspace | Pointer-efficient workspace |

For version history specifically, NOMS-010 owns branching and comparison behavior. NOMS-020 owns the visual hierarchy, responsive composition, surface treatment, and accessibility of those experiences.

## Visual reference boards

The following curated boards are checked into the repository so design decisions remain reviewable and do not depend on temporary generated-image locations.

### Core direction

| Light | Dark |
| --- | --- |
| [![Kitchen Console light direction](../assets/NOMS-020/selected-direction-light.webp)](../assets/NOMS-020/selected-direction-light.webp) | [![Kitchen Console dark direction](../assets/NOMS-020/selected-direction-dark.webp)](../assets/NOMS-020/selected-direction-dark.webp) |

### Representative routes and states

| Public and sign-in | Collections | Profile and empty states |
| --- | --- | --- |
| [![Public and sign-in examples](../assets/NOMS-020/public-and-sign-in.webp)](../assets/NOMS-020/public-and-sign-in.webp) | [![Collection examples](../assets/NOMS-020/collections.webp)](../assets/NOMS-020/collections.webp) | [![Profile and empty-state examples](../assets/NOMS-020/profile-and-empty-states.webp)](../assets/NOMS-020/profile-and-empty-states.webp) |

### Version history and comparison

| Version comparison | Spatial branching canvas |
| --- | --- |
| [![Version comparison examples](../assets/NOMS-020/version-comparison.webp)](../assets/NOMS-020/version-comparison.webp) | [![Spatial branching canvas](../assets/NOMS-020/version-graph-spatial-canvas.webp)](../assets/NOMS-020/version-graph-spatial-canvas.webp) |

| History lanes | Guided graph |
| --- | --- |
| [![History-lane graph](../assets/NOMS-020/version-graph-history-lanes.webp)](../assets/NOMS-020/version-graph-history-lanes.webp) | [![Guided version graph](../assets/NOMS-020/version-graph-guided.webp)](../assets/NOMS-020/version-graph-guided.webp) |

The three graph treatments are complementary rather than final feature commitments. Spatial Canvas preserves an at-a-glance map, Guided Graph prioritizes comprehension, and History Lanes improves scanning when several contributors or forks are present. Product selection and interaction behavior should be coordinated with NOMS-010 before implementation.

### Portrait-tablet adaptations

| Spatial canvas | Guided history | History lanes |
| --- | --- | --- |
| [![Portrait-tablet spatial canvas](../assets/NOMS-020/tablet-spatial-canvas.webp)](../assets/NOMS-020/tablet-spatial-canvas.webp) | [![Portrait-tablet guided history](../assets/NOMS-020/tablet-guided-history.webp)](../assets/NOMS-020/tablet-guided-history.webp) | [![Portrait-tablet history lanes](../assets/NOMS-020/tablet-history-lanes.webp)](../assets/NOMS-020/tablet-history-lanes.webp) |

The tablet boards establish portrait tablet as a touch-first workspace: nodes retain usable size, the primary lineage turns vertical, the desktop inspector becomes a bottom sheet, and comparison controls remain available without covering the graph.

## Consolidated planning home and provenance

As of 2026-09-12, this document on `codex/NOMS-020-visual-design-refinement` is the canonical visual-overhaul plan. Continue design planning in `.worktrees/NOMS-020-visual-design-refinement`.

- Current application baseline: `main` at `ce080b4`.
- Kitchen Console principles, responsive guidance, and 21 reference/proposal boards: merged from `codex/noms-020-visual-examples` (`396d24b`).
- Selected A3 logo and original application screenshots: inherited from `main` (visual documentation commit `257402f`).
- [Original visual-audit screenshots](../../artifacts/noms-visual-audit/) are historical baselines, not screenshots of current main.
- [Earlier logo proposals](../assets/NOMS-020/) are retained for history; A3 supersedes them as the planned application mark.
- [NOMS-013 content inventory](../../docs/design/NOMS-013/content-inventory.md) and [page copy](../../docs/design/NOMS-013/page-copy.md) remain supporting references. Their search-related claims need reconciliation with the current Search page.

The source branch is retained as history. Its missing temporary worktree is not an active planning location. Feature-specific behavior remains owned by the corresponding feature ticket; this plan owns its visual integration.

## Remaining page and state planning

The 2026-09-12 review combined live desktop inspection of current main with route/component source review. Populated version history, all dialogs, dark mode, and responsive states still require dedicated browser review. Existing boards establish direction, not complete interaction specifications.

| Area | Existing direction | Remaining planning |
| --- | --- | --- |
| Shared shell | Kitchen Console navigation and responsive model | Public versus authenticated navigation, search access, sidebar hierarchy, footer density, A3 logo placement, focus and active states |
| Search | No dedicated board | All/Recipes/Collections/Cooks results, suggestions, filters, advanced fields, facets, sorting, pagination, empty/error states, compact phone/tablet controls |
| Home, About, How-to, sign-in | Public/sign-in board | Expanded marketing content, illustrations, reading widths, anchor navigation, callouts, troubleshooting disclosures, authentication feedback; reconcile guide copy with Search |
| Dashboard and Explore | Core light/dark boards | Published versus draft cards, populated/empty/loading/error views, discovery controls, pagination and card metadata |
| Recipe editor and drafts | Core direction | Ingredient and nested-step editing, equipment, image upload/edit/reorder, validation, autosave/save failure, publishing, visibility, discard confirmation and action placement |
| Recipe detail and snapshots | Core and comparison boards | Scaling controls, historical-version context, comparisons, restore confirmation, fork attribution, sharing and collection membership |
| Version graph | Spatial, guided, lane and tablet boards | Select treatment with existing product behavior, inspect populated graphs, selection/compare/inspector states, keyboard and touch navigation |
| Collections | Collections board | Nested paths, breadcrumbs, covers, create/edit forms, recipe/child reordering, membership picker, sharing and deletion-impact preview |
| Public profile and settings | Profile/empty-state board | Public recipe/collection sections, avatar editing, validation, save feedback, linked accounts, linking failures, unlink and account-deletion dialogs |
| Shared feedback | Empty-state examples | Loading, retry, permission/sign-in required, missing content/404, success notices, disabled/pending controls and image failures |
| Upcoming feature integration | NOMS-016 through NOMS-019 coordination | Import/export, taxonomy, pantry/planning and visual recipe flow must receive visual specifications when their behavior is sufficiently defined; do not treat them as implemented main routes |

For each area, specify light/dark mode, phone (390 × 844), portrait tablet (834 × 1194), desktop (1440 × 900), keyboard focus, touch targets and reduced motion. Retain Fredoka display and Nunito body typography, with exact sizes, contrast, colors, spacing and elevation tokens still to be defined and validated.

## Next planning sequence

1. Define shared tokens and navigation; produce Search layouts and interaction states.
2. Adapt Home, About, How-to and sign-in to the selected system.
3. Complete editor, collection and settings states against current behavior.
4. Resolve graph presentation and validate populated detail/history/comparison flows.
5. Produce production A3 artwork using the actual Fredoka face, including favicon, small-size, monochrome and light/dark variants.
6. Turn the coverage table into implementation checkpoints with screenshot baselines and accessibility acceptance criteria.

Consolidation does not mark the visual audit or implementation readiness complete. No application behavior changes are part of this planning merge.

## Proposed planning scope

- Audit typography, spacing, color, elevation, density, iconography, and responsive layout.
- Inventory duplicated or inconsistent components and interaction states.
- Refine design tokens and reusable primitives before migrating product screens.
- Cover navigation, cards, forms, dialogs, tables, diagrams, empty states, loading states, errors, and feedback.
- Define light and dark mode parity, focus visibility, motion, touch targets, and reduced-motion behavior.
- Validate representative phone, tablet, desktop, ultrawide, keyboard, and assistive-technology experiences.
- Separate cohesive system improvements from unrelated product behavior changes.

## Questions to resolve

- Which parts of the existing Noms visual identity are retained, evolved, or removed?
- What density and elevation rules replace one-off styling decisions?
- Which shared components must be consolidated before route migration?
- How will visual regressions be reviewed and approved consistently?
- Does the work ship incrementally by component family or behind a coordinated release gate?

## Required implementation-plan deliverables

- Route, component, state, icon, and accessibility audit.
- Approved visual principles, tokens, component contracts, and migration boundaries.
- Representative high-fidelity designs for core workflows and responsive states.
- Reviewable implementation checkpoints with screenshot baselines.
- Automated accessibility checks and a comprehensive manual visual QA matrix.

## Definition of ready

- [ ] The visual audit and retained identity are approved.
- [ ] Token and component changes are explicitly scoped.
- [ ] Representative responsive designs and states are documented.
- [ ] Checkpoints, screenshot evidence, and release gates are defined.
