# NOMS-020 Search implementation checkpoint

Implemented 2026-09-25 on `codex/NOMS-020-search`, based on integrated design commit `9ed8749`.

## Delivered

- S2 Recipe Finder composition: wide inset search, compact scope tabs, filter disclosure, removable chips and result-first layout.
- Shared typography, coral actions, warm neumorphic materials and framed recipe cards; no shared stylesheet changes.
- Grouped All results with scope links, folder-style collection cards, cook avatars with broken-image fallback, loading skeletons and calm error/empty states.
- Existing authentication, API contracts, query URLs, advanced criteria, autocomplete, sorting and pagination retained.
- Responsive wrapping and adaptive card grids. Filters remain an in-flow disclosure on smaller screens rather than adding a separate bottom-sheet interaction.

Collection search summaries do not include images or authors; folder previews use the actual title, description and breadcrumb instead. No invented content or production fixtures were added. Dark-mode redesign remains deferred; existing tokens are respected.

## Validation

Commands run with `CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/target`:

- `SQLX_OFFLINE=true cargo test --features server pages::search::tests -- --test-threads=4`: 15 passed.
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings`: passed.
- `cargo clippy --target wasm32-unknown-unknown -- -D warnings`: passed.
- `cargo fmt --check` and `git diff --check`: passed.

Independent review: PASS after fixing avatar state reuse and explicit circular-avatar sizing.

## Browser evidence

Fresh Docker preview `noms020-search-v2-20260925`, app port **32809**, with mock OAuth and disposable database fixtures (24 public recipes, two cooks, two owned collections).

- Signed in through the preview's mock Google provider.
- Verified all three grouped result types, real titles/descriptions/author metadata, scope links and missing-avatar initial fallback.
- Recipe time filter reduced 24 results to 12; removing its chip restored the unfiltered request.
- Verified page 2 navigation, title sorting/reset to page 1, advanced-search disclosure, and zero-result Cooks state.
- Desktop screenshots reviewed in light and dark themes; restored light theme afterward.
- Phone 390px and tablet 820px DOM measurements showed no horizontal overflow; phone recipe grid resolved to one 343px column. Browser screenshot capture at emulated sizes was scaled incorrectly, so these responsive checks are geometry evidence, not final device visual approval.
- Browser review caught empty reserved columns when only one result group matched; fixed group-count-aware layout. Final browser readback confirmed the cook avatar is 64×64px.

Remaining limits: no new end-to-end failure injection or live advanced-query submission test; existing focused tests cover advanced-query construction. Autocomplete keyboard tests pass, but that interaction was not re-exercised manually. Fixture recipes had no photos, so Search reused the existing tested recipe image rendering and verified its no-photo fallback. This checkpoint does not replace later real-device visual refinement.

## Integration

Merge this task branch into the NOMS-020 design integration branch after review. The implementation is kept as one cohesive visual/interaction commit, followed by this evidence checkpoint; no backend changes or deployment configuration changes are included.
