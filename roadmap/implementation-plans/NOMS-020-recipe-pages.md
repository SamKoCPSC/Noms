# NOMS-020 — Recipe pages

Started 2026-09-25 from integrated design revision `7b2dab2` on
`codex/NOMS-020-recipes`. This slice owns the new-recipe and draft editors,
current recipe detail/history, historical snapshots, and version graph.

## Design contract

- **E3T1 v5:** one continuous editor with separate raised Basics, Photos,
  Ingredients, Method, Equipment and Notes panels. Basics and Photos share
  the top row on wide screens. A detached contents timeline links to each
  section; optional content does not become a publication requirement.
- **R3A:** a photo-led album, distinct readable ingredient/method panels,
  thumbnail version history, and explicit Current/Selected labels. Historical
  snapshots stay read-only; restoring creates a new version.
- **G4A:** a raised toolbar above an inset canvas, readable snapshot cards,
  independent relationship/current/comparison labels, and comparison controls
  and results in document flow below the graph. Preserve the accessible outline.
- Retain Fredoka/Nunito, the established palette/elevation tokens, borderless
  inset text fields, shared buttons and warm glass overlays. New page styles
  are scoped to recipe pages and loaded by their components.
- Preserve draft autosave/revision guards, explicit publishing, discard/restore
  confirmation, owner-only actions, scaling, nested steps, image editing,
  collection membership and exact graph topology/navigation contracts.

References: [editor](../assets/NOMS-020/recipe-editor-e3t1-panels-v5.md),
[album](../assets/NOMS-020/recipe-album-neumorphic-v2.md),
[graph](../assets/NOMS-020/g4a-integrated-page-light-v1.md),
[comparison states](../assets/NOMS-020/g4a-selection-comparison-states-v1.md),
[expansion states](../assets/NOMS-020/g4a-dense-expansion-states-v1.md).

## Ownership and integration

Originally implemented in the recipe worktree; source tip `31cac9b` is now
integrated by merge `74c4200` (verified 2026-09-27). See the
[combined browser review](NOMS-020-checkpoint-8-integration.md) for the private
draft → v1 → v2 → collection → graph/snapshot walkthrough and remaining gates.
Editor, detail/history and graph have separate Rust and CSS ownership.
Navigation, global tokens, collection pages, backend
schemas and production logo/icon artwork remain owned by their existing slices.
The shared image editor remains available through the existing caller contract.

## Acceptance

- [x] New, existing-recipe and standalone-draft editors render all six sections;
  section navigation and row editing work with keyboard input.
- [x] Autosave, save, publish and discard retain their existing behavior.
- [x] Detail and historical snapshots show real photos/fallbacks, attribution,
  recipe content, scaling, version context and authorized actions.
- [x] History selection, snapshot navigation, comparison and restore preserve
  their contracts and display explicit state labels.
- [x] Graph displays published versions and a fork, supports pan/zoom/Fit/Current,
  menus and Before/After comparison; results remain below the canvas.
- [x] Desktop, short desktop, phone and portrait-tablet layouts remain usable;
  inspect overflow, long text, touch targets and keyboard focus.
- [x] Existing dark mode and shared reduced-motion/high-contrast rules retained
  (dark editor smoke-tested; system accessibility modes not browser-emulated).
- [x] Relevant tests and both build targets pass; final diff review is recorded.

## Validation environment

Fresh isolated Compose instance: `noms-noms020-recipes-20260925`.
Only disposable local mock-user fixtures are used. Fixture coverage includes a
three-version public recipe, a public fork and a standalone private draft.
App URL: `http://127.0.0.1:32775`. Mock Google subject: `testuser`.

## Validation — 2026-09-25/26

Automated commands ran in the recipe worktree with
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`
to reuse the existing dependency cache:

- `SQLX_OFFLINE=true cargo test --features server -- --test-threads=8`:
  **768 passed, 12 ignored, 0 failed**. Raw output: `/tmp/noms020-recipes-tests.log`.
- `cargo clippy --target wasm32-unknown-unknown -- -D warnings`: passed.
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings`: passed.
- `cargo fmt --check` and `git diff --check`: passed.
- Added rendered checks for editor navigation/accessible names, album empty/photo
  states and comparison paths/escaped read-only values; graph card dimensions
  are checked against the layout constants.

Browser evidence (Codex in-app browser, isolated fixtures only):

- New recipe: all six sections present; a title-only recipe saved as a draft,
  confirming optional panels introduce no completion gates.
- Standalone draft: nested step edited, autosaved and retained after reload;
  focus updated the current section and native anchors focused section headings.
- Uploaded `assets/home-kitchen-illustration.png` as a disposable geometry fixture;
  existing image editor opened, applied edits and returned the photo to the form.
  Publishing produced a private v1 recipe with the image and nested step intact.
- Existing recipe editor opened with its populated state and expected actions.
- Current detail: photo fallback, owner actions, ingredients and nested method;
  selecting history displayed Current plus Selected for comparison and correct
  v2 → v3 changes. Historical v2 displayed read-only context and comparisons.
- Graph: three versions and a fork; visible menu selected v1 Before and v3 After;
  result displayed the correct ingredient/method changes and exact field paths.
  Pan, zoom, Fit, Current and Lineage only controls exercised.
- Screenshots inspected at 1440×900 desktop, 1280×720 short desktop,
  834×1194 tablet and 390×844 phone across representative pages. Editor nested
  controls wrap at tablet size, mobile panels stack, graph toolbar wraps, and
  measured document widths did not exceed their viewports. Existing dark editor
  remained legible and was restored to light afterward.

Independent source reviews: **PASS** for editor, detail/snapshots and graph.
Review fixes removed field-outline overrides, made the tracker follow field
focus, and replaced a redundant source-scanning test with rendered evidence.

## Remaining validation limits

This is a visual implementation pass, not an exhaustive new behavioral audit.
Discard/restore, ownership boundaries, stale requests and dense graph routing
retain their handlers and automated regression coverage; destructive actions,
forced network failures, every dense graph and every viewport/page combination
were not replayed manually. Dark redesign and production page-icon preparation
remain deferred. Graph menu dismissal retains the existing snapshot-link focus
destination. No backend/schema, shared component or production artwork changes.
