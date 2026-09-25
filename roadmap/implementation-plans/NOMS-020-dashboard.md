# NOMS-020 — Dashboard implementation

## Scope and integration

Implemented on `codex/NOMS-020-dashboard`, in `.worktrees/NOMS-020-dashboard`,
from design integration commit `9ed874922a2be66e4c8c8cc9538de91b4e516a89`.
This branch is intended for integration into the NOMS-020 design worktree;
it has not been merged there by this task.

Reference: [Central Workspace v1](../assets/NOMS-020/dashboard-central-workspace-v1.md).
The user explicitly selected **visual redesign with existing behavior**, not
the proposed functional expansion.

## Delivered

- Raised workspace sidebar with section anchors, the existing nested collection
  tree, collection creation/browsing, profile and settings links.
- Compact Continue editing panel, two recent drafts initially, expandable full
  draft list, revision and fork-source metadata, and untitled-draft fallback.
- List-first library with thumbnails, publication status, actual visibility,
  updated date, existing Edit and Add to Collection actions.
- Alternate grid using the existing RecipeCard and an owner-edit shortcut.
- Existing owner-scoped Search route linked from the library toolbar.
- Existing 12-record API pages presented as Previous/Next with accurate ranges.
  The old Load more label replaced the visible page rather than accumulating
  records. Controls now remain mounted and focusable through loading and page
  boundaries, with guarded activation and `aria-disabled` states.
- Separate loading, empty and recoverable error states for recipes, drafts and
  collections. Authentication remains required before loading workspace data.
- Route-owned `assets/dashboard.css`, reusing the shared surface/elevation,
  typography and button system. No global token or navigation restyling.
- Opt-in `CollectionSidebar { embedded: true }`; default callers retain their
  existing frame and heading/action behavior.

Overview and All recipes are section anchors in this pass, not separate new
dashboard modes. Saved-recipe counts exclude private drafts. “Published” means
a saved recipe exists, independently of its public/private/unlisted visibility.

## Explicitly deferred

- Embedded whole-library search, publication/visibility filters and sorting.
- Bulk selection/actions and a multiple-membership collection column.
- Draft collection membership, numbered pagination and a separate Overview view.
- Bespoke mobile/tablet design discovery; this pass provides responsive stacking
  and overflow protection, not a newly approved responsive design direction.
- Logo/page-icon rollout and any unrelated shared-component changes.

## Verification — 2026-09-25

Commands run from the Dashboard worktree (shared compilation cache only):

```sh
CARGO_TARGET_DIR=/tmp/noms-about-target SQLX_OFFLINE=true cargo check --features server
CARGO_TARGET_DIR=/tmp/noms-about-target cargo check --target wasm32-unknown-unknown
CARGO_TARGET_DIR=/tmp/noms-about-target SQLX_OFFLINE=true cargo clippy --features server -- -D warnings
CARGO_TARGET_DIR=/tmp/noms-about-target cargo clippy --target wasm32-unknown-unknown -- -D warnings
CARGO_TARGET_DIR=/tmp/noms-about-target SQLX_OFFLINE=true cargo test --features server pages::dashboard -- --test-threads=2
CARGO_TARGET_DIR=/tmp/noms-about-target SQLX_OFFLINE=true cargo test --features server collection_sidebar -- --test-threads=2
cargo fmt --check
git diff --check
```

All passed. Final Dashboard suite: 3 tests; collection-sidebar filtered suite:
6 tests. These are focused tests, not a claim that the full repository suite ran.
Tests cover thumbnail/edit-metadata preservation, page ranges including empty
pages, existing sidebar behavior and Dashboard composition.

Browser validation used a fresh isolated Docker deployment started with
`just up noms020-dashboard-v2-20260925`, at `http://127.0.0.1:32814/dashboard`.
The local mock OAuth subject was `dashboard-review`. Disposable fixtures were
inserted only into this instance: 14 saved recipes, 3 drafts (including forked
and untitled), and 3 collections including a nested collection.

Observed in Chrome:

- Signed-out Dashboard redirects to sign-in; mock sign-in opens the workspace.
- Empty library/drafts/collections and initial loading states render honestly.
- Desktop raised sidebar, two-draft strip, toolbar and table match the selected
  hierarchy and shared materials; no decorative quotes or fabricated controls.
- Draft expansion exposes all three drafts and retains revision/fork details.
- Collection expansion reveals the nested collection; the existing picker opens
  with current membership checked/disabled and closes with Cancel.
- List/grid switching works. Previous/Next shows 1–12 then 13–14 of 14.
- After the keyboard fix, Enter on Next retains focus on Next at the last page;
  Previous returns to the first page.
- At 390px viewport width, the table becomes readable stacked rows, controls
  wrap, and document width is 375px (scrollbar excluded), with no horizontal
  overflow. A 1024px viewport was also checked for overflow. Temporary viewport
  overrides were reset afterward.
- Temporarily blocking the three workspace API requests exposes distinct retry
  controls for recipes, drafts and collections; removing the block and clicking
  each Retry restored all three sections. Temporary network/cache overrides
  were reset afterward.

An independent read-only review returned PASS, with no blocking/should-fix
findings. Two informational findings were fixed: invalid span/div thumbnail
nesting and missing contextual labels on grid Edit links. A subsequent review
of the keyboard-pagination fix also returned PASS.

The dev server once served mismatched cached browser assets after restart;
reloading with cache disabled restored hydration. No application workaround
was added for this development-only condition.
