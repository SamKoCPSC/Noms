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

- M4 ordinary-modal material is implemented in slice 2 below, with preference-emulation validation still pending. Remaining modal/menu consumers require separate reviewed slices; do not apply glass indiscriminately to the shared navbar or editor panels.
- T2 feedback and P1–P3 overlay interaction contracts (focus, dismissal, live regions and picker semantics) still need implementation specification.
- Remaining shared component/state coverage and visual review must be closed before the navigation/Explore checkpoint or parallel route rollout.

### Slice 2: M4 ordinary-modal material

Apply the selected warm frosted material to ordinary `.modal-card` consumers (collection and account deletion), preserving all existing handlers, wording, pending guards and typed confirmations. Exclude `.modal-card--editor`, the navbar, membership menus and regular neumorphic page panels from this first material migration. Their layout/interaction work remains separate. This is not acceptance of existing modal accessibility: account/image-editor dialog semantics and shared focus/inert/scroll-lock lifecycle still require a later behavioral slice.

Acceptance contract before implementation:

- Dedicated overlay tokens preserve the existing typography and cream/coral identity; do not repurpose global `.glass`/navbar tokens.
- Opaque warm surface is the default; only enable translucent fill and blur inside a backdrop-filter support query. Use a thin light edge and floating shadow instead of the ordinary modal's raised bevel.
- Modal primary and muted text remain at least 4.5:1 over worst-case black/white underlays in both themes. Fields stay opaque using existing surface roles; use overlay-scoped muted text only where needed.
- Reduced-transparency preference restores opacity and removes blur. Forced colors uses system surface/text/border without shadow. Reduced motion disables ordinary-modal entrance animation and its backdrop animation.
- Ordinary modal content remains reachable on short viewports with constrained height and scrolling; do not alter the image editor's independent flex/overflow contract.
- Verify ordinary dialog open/cancel, supported glass, opaque preference fallback, light/dark, short/narrow viewport, and no changed destructive workflow. Full shared modal keyboard semantics remain explicitly pending rather than implied by a material change.

Implementation: `948f4d0` adds dedicated overlay tokens, warm translucent ordinary-dialog shells with thin edges/floating shadows, support-gated blur, opaque reduced-transparency fallback, system-color fallback, entrance-motion suppression and viewport-constrained scrolling. `.modal-card--editor` and existing global/navigation glass are excluded. The 94%-opaque fill intentionally favors legibility over pronounced translucency; scoped muted text uses `#635D55` / `#B8B1A7` while the original theme colors and font families remain unchanged.

Three new source/contrast tests first failed against the previous implementation (six existing tests passed), then all nine passed. Full server suite: 709 passed, 0 failed, 12 ignored. `just check`, formatting/diff checks, six schema guard cases, declarative workflow and security-policy tests pass. `just ci` again stops at the unchanged server Clippy issue described below. Independent Phase-1 review of the complete diff and consumers: **PASS**, no introduced P0–P2 findings.

Fresh runtime: `just up instance=noms020-modal-material-20260919`, application `http://127.0.0.1:32778`, healthy infrastructure/schema and mock `testuser` sign-in. Chrome fallback was used because the in-app browser was unavailable. Actual light/dark fill, blur, border, floating shadow and text roles matched the implementation. At 800×180, the ordinary modal had 146px client height and 208px scroll height; keyboard focus scrolled it 62px to expose Cancel, and clicking Cancel removed the modal. A 390px viewport was also checked. Only the initial account-deletion confirmation was opened/canceled; no destructive confirmation was activated. Image-editor exclusion and unchanged collection handlers are source-review evidence, not separate runtime walkthroughs.

Slice-2 Phase 2 remains **incomplete/FAIL (insufficient evidence, no implementation defect established)**: media emulation of reduced transparency/motion stalled despite requested timeouts, and no alternate media capability was available. Supported rendering and viewport behavior pass, but reduced-transparency and forced-color/motion fallbacks currently have source-test evidence only for this slice. Theme, media and viewport were restored. Report and screenshots: `/tmp/noms020-modal-material-20260919/report.html`; the report rendered with all four embedded images loaded, and captures have backend scaling/cropping limitations. Re-run the preference matrix before accepting slice 2. The full foundation/route-rollout gate remains closed.

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

The initial runtime attempt was **incomplete**; independent Phase 2 verdict was **FAIL (insufficient evidence, no implementation defect established)**. CDP viewport operations in Chrome also exceeded their requested timeouts. Hover/danger states, reduced-motion and forced-color emulation, narrow-layout verification, and the recipe-detail intrinsic visibility-selector layout were then unverified. The editor's deliberately full-width visibility selector is a different control and is not regression evidence for recipe detail.

### Resumed slice-1 validation — PASS, 2026-09-19

Continued the same unfinished isolated validation run with unchanged production revision. Chrome's documented viewport capability recovered the blocked checks. Independent Phase 2 now reports **PASS** for slice 1: actual light/dark default and hover button roles, disabled states, field focus/boundaries, reduced-motion background/shared transitions, forced-color borders/focus, 390px layout without horizontal overflow, and recipe-detail selector intrinsic width (144px) all verified. The private fixture exercised normal autosave/publication without public sharing; account-deletion confirmation was opened and canceled without confirming deletion. Spinner-specific reduced-motion behavior remains source-contract evidence because no live spinner was observed.

The same portable report was updated and rendered in Chrome with all four embedded images loaded. Capture-backend tiling beyond the first image tile prevents a full-screenshot-fidelity claim; computed viewport/layout measurements and the reliable first tiles support the scoped verdict. Theme, media and viewport were restored. This supersedes the incomplete attempt above, not the separate pre-existing CI blockers or the unfinished checkpoint-1 work. After capturing evidence, `just down instance=noms020-foundations-20260919` removed only that disposable deployment, its fixture data and build caches to free the worktree/network slot for fresh slice-2 validation; the temporary report/screenshots remain.

## Status

- [x] Scope and regression contracts recorded; tests fail before production changes.
- [x] Production slice and automated checks executed; unrelated full-CI failures recorded above.
- [x] Independent code review passes (P2 sizing regression fixed and guarded).
- [x] Slice-1 fresh-runtime validation and evidence passes (Chrome fallback and capture limitations recorded).
- [x] Slice-2 ordinary-modal material implemented, tested and code-reviewed.
- [ ] Slice-2 accessibility-preference runtime checks complete.
- [ ] Full checkpoint-1 gate complete (includes remaining slices above).
