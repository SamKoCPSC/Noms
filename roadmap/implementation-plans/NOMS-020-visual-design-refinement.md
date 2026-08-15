# NOMS-020: Application Visual Design Refinement — Implementation Plan

**Status:** Placeholder — visual audit and implementation planning required
**Created:** 2026-08-15
**Depends on:** Current application routes, components, design tokens, and the visual direction established by NOMS-013
**Coordination:** Feature tickets NOMS-016 through NOMS-019 should use shared primitives where stable; NOMS-020 must avoid redesigning feature behavior owned by those tickets.
**Approach:** To be finalized through an evidence-based visual audit, an approved design direction, reusable token and component changes, and a route-by-route migration plan.

---

## Intended outcome

The application presents one deliberate, accessible, responsive visual system across public pages, authenticated workflows, dense data views, forms, and transient states.

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
