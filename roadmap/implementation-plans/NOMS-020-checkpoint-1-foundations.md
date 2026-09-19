# NOMS-020 checkpoint 1 — Shared foundations

Started 2026-09-19 from `25d65cf` in the existing isolated `codex/NOMS-020-visual-design-refinement` worktree. Main's unrelated changes remain excluded.

## Slice 1: elevation, controls and accessibility roles

This is the first foundation slice, not a page redesign or completion of the full checkpoint. Preserve existing Fredoka/Nunito, cream/coral identity, light/dark palette values, component props/events and route behavior. Introduce semantic roles only where a repeated value or measured accessibility issue warrants it.

Acceptance criteria established before production edits:

- Existing brand values and font families remain unchanged in both themes.
- Repeated raised/inset shadow definitions use shared elevation tokens without changing their geometry. Theme-dependent aliases must resolve correctly for dark mode.
- Shared primary/secondary-filled/danger button text reaches 4.5:1 against its enabled default/hover fill in each theme. Existing coral fills stay unchanged; foreground roles are separate from brand colors.
- Shared fields have visible boundaries and legible placeholder text. Shared keyboard focus has an explicit outline, not shadows alone; remove Input's inline outline suppression while preserving its input event/value contract.
- Reduced motion stops the animated background and shared spinner and suppresses shared control transitions. Forced colors retains visible control boundaries and focus.
- No auth, data, route, editor, graph, or feature behavior changes. Preserve narrow/dark usability; do not introduce a new dark visual direction.

The pre-change primary-button combination white on `#D9735A` measures 3.21:1, below AA for ordinary text. Contrast-specific foreground/ink roles are justified accessibility adjustments, not replacement palette values. Full-page contrast conformance is not implied: many route-local controls and legacy muted labels are outside this slice.

## Test and review strategy

Main-agent-owned `src/components/design_foundation_tests.rs` provides std-only source contracts and WCAG relative-luminance calculations. It is wired into the normal Rust test suite and can run independently with:

```sh
rustc --edition=2021 --test src/components/design_foundation_tests.rs -o /tmp/noms020-foundation-tests
/tmp/noms020-foundation-tests
```

Before production changes: one preservation test passed; four tests failed on missing roles/elevation/fallbacks and Input outline suppression. Source assertions are guardrails, not proof of the CSS cascade; browser-computed styles and actual interaction checks are required after code review.

Run `just ci`, inspect generated CSS if startup/build regenerates it, obtain independent code review, then validate in a fresh isolated instance. Verify shared input focus, default/hover/disabled controls, light/dark computed roles, reduced motion and forced colors. Include desktop and narrow screenshots and representative existing navigation/input flows. Temporary portable HTML evidence belongs under `/tmp`, not the repository.

## Remaining checkpoint-1 work

- M4 overlay material/fallback primitives and existing modal/menu consumer migration require a separate reviewed slice; do not apply glass indiscriminately to the shared navbar or editor panels.
- T2 feedback and P1–P3 overlay interaction contracts (focus, dismissal, live regions and picker semantics) still need implementation specification.
- Remaining shared component/state coverage and visual review must be closed before the navigation/Explore checkpoint or parallel route rollout.

## Implementation and checks

Production/test commit: `ebb1345` — shared control foundations. New `--elevation-raised`/`--elevation-inset` reuse existing shadow geometry, with dark-scope recomputation. Contrast roles are separate from unchanged brand colors. Shared Input's former inline presentation lives in `.input--shared`; generic `.input` retains boundaries and focus without imposing full-width sizing on page-owned controls.

| Contrast role against intended background | Light | Dark |
| --- | --- | --- |
| Primary text / coral fill | 5.92:1 | 7.45:1 |
| Primary text / hover fill | 4.64:1 | 9.15:1 |
| Danger text / danger fill | 4.57:1 | 5.05:1 |
| Secondary button ink / surface | 5.93:1 | 6.22:1 |
| Field placeholder / surface | 4.72:1 | 5.48:1 |
| Field border / surface | 3.63:1 | 5.48:1 |
| Focus ring / surface | 5.93:1 | 6.22:1 |

These ratios use WCAG relative luminance of the resolved opaque color roles, not a claim that every app control/state meets AA. Disabled controls retain existing opacity and are not included in the normal-text contrast claim. Browser cascade checks remain necessary.

Independent code review initially found a P2: broad `.input { width: 100% }` would wrap the recipe visibility selector's desktop row. Added a failing sixth test, scoped the former inline presentation to `.input--shared`, and repeated review: PASS. Forced-color focus uses a scoped important outline to override legacy page-owned inline outline suppression after the platform removes shadows.

Automated outcomes so far:

- Six focused foundation tests pass, both standalone and through `cargo test --features server components::design_foundation_tests` with SQLx offline mode.
- Final full server suite (`SQLX_OFFLINE=true cargo test --features server -- --test-threads=8`): 706 passed, 0 failed, 12 ignored, including the sixth regression test.
- Client/server compilation and WASM Clippy passed; formatting and diff checks passed.
- Schema-plan six cases, declarative workflow and security-policy script checks passed.
- Full `just ci` is **not green**: server Clippy reports pre-existing `clippy::needless_return` in `src/components/informational/mod.rs:34`, present in `25d65cf` and untouched here.
- Separately running `just security-check` reports pre-existing locked `rustls 0.23.40`, advisory `RUSTSEC-2026-0285`, requiring at least 0.23.45. Cargo.lock is unchanged. The advisory-cache permission limitation was resolved for the retry; the vulnerability gate still fails. No suppression, dependency update or unrelated code cleanup is included.

These existing CI failures prevent a clean release-gate claim even if scoped visual validation passes. Address them separately with appropriate authority and regression checks.

## Runtime evidence and remaining verification

Fresh isolated instance: `noms020-foundations-20260919`, started with `just up`, application at `http://127.0.0.1:32773`. Infrastructure/schema initialization and HTTP health passed. Mock OAuth `testuser` authentication and dashboard access passed in the in-app browser. Its viewport/navigation call then stalled and the browser disconnected; recovery inventory no longer offered an in-app browser.

Chrome fallback loaded the app, authenticated and opened New Recipe. Direct computed-style checks confirmed light/dark shared button default foreground/background roles, disabled styling, shared Input border/placeholder and explicit 2px focus outline with 3px offset, Nunito body typography, and dark elevation aliases resolving to the dark shadows with unchanged geometry. Light/dark screenshots preserve the existing visual character; they are clipped browser-surface captures, **not** full-viewport or responsive-layout evidence.

Temporary portable report: `/tmp/noms020-foundations-20260919/report.html`; screenshots: `light-clipped.png` and `dark-clipped.png` in the same directory (the browser returned 963×1034 JPEG data despite the filenames; report embeds use the correct MIME type). The report distinguishes observed runtime checks from source-contract evidence and tool limitations. The report served HTTP 200, but its browser-render verification also timed out; embedded screenshot files were visually inspected separately. Theme was restored to light; temporary-tab closure and viewport reset are unconfirmed after the final tool timeout. Startup-only unrelated generated Tailwind utilities were removed from the working diff.

Runtime verification remains **incomplete**; independent Phase 2 verdict is **FAIL (insufficient evidence, no implementation defect established)**. CDP viewport operations in Chrome also exceeded their requested timeouts. Hover/danger states, reduced-motion and forced-color emulation, narrow-layout verification, and the recipe-detail intrinsic visibility-selector layout have not been proved in this run. The editor's deliberately full-width visibility selector is a different control and is not regression evidence for recipe detail. Source tests and code review cover those contracts but do not replace the missing browser checks. Re-run this matrix before accepting the slice; no route-rollout gate is opened.

## Status

- [x] Scope and regression contracts recorded; tests fail before production changes.
- [x] Production slice and automated checks executed; unrelated full-CI failures recorded above.
- [x] Independent code review passes (P2 sizing regression fixed and guarded).
- [ ] Fresh-runtime validation and evidence passes.
- [ ] Full checkpoint-1 gate complete (includes remaining slices above).
