# NOMS-027: Animated Penguini kitchen on Introduction

**Status:** ✅ Done — animated hero and poster fallback delivered

**Parent:** [NOMS-013](../implementation-plans/NOMS-013-marketing-pages-3d-experience.md)

**Delivered:** 2026-11-17 ([PR #91](https://github.com/SamKoCPSC/Noms/pull/91),
[PR #93](https://github.com/SamKoCPSC/Noms/pull/93))

The Introduction hero shows Chef Penguini cooking in the Coral & Seafoam
kitchen. The cooking sequence loops, with a brief fade at the ingredient reset.
The responsive layout brings more of the kitchen into the first mobile screen
and places it alongside the search and entry actions.

- An accessible play/pause control supports keyboard and touch. Reduced motion
  uses the poster, and the scene suspends when offscreen or in a background tab.
- The matching poster covers loading, unavailable WebGL, asset failure and
  context loss. Navigation releases render resources. The kitchen shares the
  About renderer's baked-lighting and GPU cleanup helpers.
- The editable master, compressed animated model, bake provenance and rebuild
  tooling are retained with the assets.

The PR records eight marketing contract tests and Chromium checks for full
loops, controls, reduced motion, resource cleanup, failed assets and fallback.
The browser ran a compiled frontend fixture with a signed-out profile stub.
Desktop, mobile down to 320 px, and dark mode were visually reviewed.
