# NOMS-019: Visual Recipe Flow & Cooking Timeline — Implementation Plan

**Status:** Placeholder — discovery and implementation planning required
**Created:** 2026-08-15
**Depends on:** Current ingredient and instruction models; stable recipe scaling and versioning behavior
**Coordination:** NOMS-017 may supply cooking methods and timing metadata; NOMS-020 defines the final visual language and interaction polish.
**Approach:** To be finalized after the recipe execution graph, authoring experience, layout algorithm, accessibility fallback, and rendering constraints are prototyped.

---

## Intended outcome

Noms can generate an understandable diagram showing how ingredients, intermediate results, active work, passive time, and parallel steps combine into a finished recipe.

## Proposed planning scope

- Define structured step inputs, outputs, dependencies, durations, temperatures, and active/passive classification.
- Generate a deterministic execution graph from recipe data rather than storing a manually drawn diagram.
- Render ingredient flows, intermediate mixtures, parallel work, and waiting periods.
- Provide selection, focus, zoom, overflow, and responsive behavior.
- Keep conventional numbered instructions as the complete accessible and printable fallback.
- Define editor controls that capture graph relationships without requiring authors to draw nodes and edges.
- Ensure scaling, version history, import, export, and incomplete legacy recipes degrade safely.

## Questions to resolve

- What is the smallest graph model that represents real recipes without excessive authoring work?
- Which relationships can be inferred, and which require explicit author input?
- How are cycles, missing dependencies, optional steps, and repeated sub-recipes handled?
- Should the first renderer use semantic HTML, SVG, canvas, or a hybrid?
- Which layout and performance limits apply to large recipes and small screens?

## Required implementation-plan deliverables

- Execution-graph schema and validation contract.
- Authoring and legacy-recipe fallback design.
- Rendering prototype with accessibility and performance evidence.
- Test-first checkpoints for model, layout, interaction, and integration.
- Automated, visual, responsive, print, and assistive-technology verification matrix.

## Definition of ready

- [ ] The execution graph and fallback semantics are approved.
- [ ] A representative prototype validates the rendering approach.
- [ ] Authoring complexity and legacy-recipe behavior are bounded.
- [ ] Checkpoints, acceptance evidence, and release gates are defined.

