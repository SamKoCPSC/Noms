# NOMS-020: Application Visual Design Refinement — Implementation Plan

**Status:** Visual direction selected — implementation scoping and validation required
**Created:** 2026-08-15
**Depends on:** Current application routes, components, design tokens, and the visual direction established by NOMS-013
**Coordination:** Feature tickets NOMS-016 through NOMS-019 should use shared primitives where stable; NOMS-020 must avoid redesigning feature behavior owned by those tickets.
**Approach:** Evolve the existing Noms identity into the selected Kitchen Console direction through an evidence-based visual audit, reusable token and component changes, and a route-by-route migration plan.

---

## Intended outcome

The application presents one deliberate, accessible, responsive visual system across public pages, authenticated workflows, dense data views, forms, and transient states.

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
