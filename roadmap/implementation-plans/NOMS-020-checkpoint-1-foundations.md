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

- M4 ordinary-modal material is implemented in slice 2 below. Remaining modal/menu consumers require separate reviewed slices; do not apply glass indiscriminately to the shared navbar or editor panels.
- T2 feedback foundations and the collection Copy link/Share consumer are implemented (slice 6). P1 account-menu implementation is recorded in slice 7. Broader feedback adoption and P2–P3 overlay interaction contracts/picker semantics remain outstanding.
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

The initial slice-2 Phase 2 attempt was **incomplete/FAIL (insufficient evidence, no implementation defect established)**: media emulation stalled despite requested timeouts. Supported rendering and viewport behavior passed, but accessibility-preference runtime checks were initially unavailable. Report and screenshots: `/tmp/noms020-modal-material-20260919/report.html`; the original report rendered with all four embedded images loaded, and those captures have backend scaling/cropping limitations.

On the user's requested browser retry (2026-09-19), the existing isolated app remained healthy (HTTP 200), so no restart was needed. The main agent verified all six light/dark × reduced-transparency/forced-colors/reduced-motion cases through the **Codex in-app browser using CUA**. Actual `matchMedia` results were true: reduced transparency yielded opaque theme fills with no blur; forced colors yielded Canvas/CanvasText, visible 1px borders and no blur/shadow; reduced motion yielded no modal or backdrop animation. Main restored light theme, cleared all media overrides and canceled the confirmation (zero open modals); no destructive confirmation was activated. Exact observations are recorded at `/tmp/noms020-modal-material-20260919/iab-preference-verification.json`.

These are main-agent in-app-browser observations, distinct from the validator's earlier independent material/layout checks. A separate earlier headless-browser evidence appendix is historical provenance, not the basis for this browser retry. The full foundation/route-rollout gate remains closed pending the remaining behavioral and consumer slices, independently of this material verification.

## Implementation and checks

### Slice 3 contract — shared modal behavior, account deletion first

Started from `2ad7024`. Preserve M4 styling, server deletion/auth/logout behavior and the existing Confirming → exact typed phrase → Final workflow. Introduce a reusable named modal with safe initial focus (Cancel, phrase field, then Go Back), contained keyboard focus, inert background, locked background scrolling, and restoration of prior focus/scroll state on close or unmount. Prefer native modal-dialog semantics, supplementing Tab boundaries where browser navigation otherwise leaves the document. Stage transitions must move focus to their appropriate safe target; input must have an accessible name. Escape cancels any non-pending stage, never confirms deletion. Keep the existing no-backdrop-dismiss policy for the typing stage; other backdrop dismissal is allowed only while not pending.

Use an executable account-deletion state model to enforce exact phrase/current username and all three stages, reject duplicate submission, and block cancel/backdrop/Escape/back navigation/reopening/phrase edits while pending. Failure keeps the final stage and phrase, exposes an announced error, and permits retry or safe cancellation. Guard at the handler/state boundary as well as disabling UI controls. Actual account deletion is not necessary for browser validation; do not activate its irreversible final action. Pending/failure guards require deterministic automated state tests, with browser proof of ordinary navigation/focus/cancel/scroll behavior.

Main-owned tests in `src/components/modal_behavior_tests.rs` were added before production code; first compilation fails because the proposed pure state-model module does not yet exist. Collection deletion and image-editor migration remain separate; this slice must not modify their handlers or modal layout. No new menu, toast or route redesign is included.

Implementation commit `fdaacfe`: `SharedModal` uses native `showModal()`/cancel semantics and a synchronous wasm-bindgen lifecycle module. Its registry preserves prior body/root overflow values and priorities until the last dialog closes, avoids stealing focus on ordinary rerenders and restores connected prior focus targets. `AccountDeletionState` claims pending state before spawning a request; server deletion and logout code is unchanged. Account dialogs keep the M4 inner cards, add an accessible phrase label and an announced failure message, and guard all dismissal/navigation handlers.

Initial checks: five Rust behavior contracts pass; six deterministic JS lifecycle tests pass (safe focus, stage changes, exact overflow restoration, nested/out-of-order disposal, missing mounts and duplicate cleanup). The JS tests are included in `just test` and CI. Full `just test`: **714 Rust tests passed, 12 ignored; six JS tests passed**. Client/server compilation, WASM Clippy, formatting/whitespace, six schema guard cases, declarative workflow and security-policy checks pass. `just ci` still stops on the unchanged `informational/mod.rs:34` server Clippy warning. Independent Phase-1 review of all 13 changed/untracked files and affected call paths: **PASS**, no introduced P0–P2 findings. Browser acceptance is pending in a new isolated `noms020-modal-behavior-20260919` deployment; the prior material-test deployment was removed after preserving its evidence.

Browser review found a P2: Tab from the final focusable control escaped into browser chrome. Tests were added first for forward/reverse boundaries and unavailable controls; a further failing test covers proactively retaining focus when pending disables every control, then restoring the safe target after failure. The correction supplements native inertness with explicit boundary wrapping and pending-transition focus. Final automated rerun: **714 Rust tests passed, 12 ignored; nine JS tests passed**. `just ci` repeats the same pre-existing server Clippy failure after passing compilation and WASM lint. Correction commit: `0d4cd4f`. Renewed independent review: **PASS**, no introduced P0–P2 findings.

Final runtime: the initial deployment had stale generated WASM/JS, so only its app container was restarted, preserving its disposable database and port. Emitted and HTTP-served lifecycle JS hashes matched the reviewed source, and compiled bindings included the new wrapper. After reload, main-agent CUA in-app-browser observations verify forward/reverse Tab wrapping in all three stages, safe initial/stage focus, disabled invalid phrase, enabled exact phrase, Go Back preserving the phrase, typing backdrop guard, Escape at every stage, Cancel restoration, native modal state and restored root/body overflow. Independent validator repeated five Rust/nine JS tests and reviewed the raw main-browser evidence; final browser access was unavailable to that agent. Scoped validation **PASS**, not full checkpoint acceptance.

Evidence: `/tmp/noms020-modal-behavior-20260919/report.html` (self-contained screenshots and raw AX evidence), `iab-proof.json`, and `rebuild-provenance.txt`. Browser policy blocked opening the local HTML report; its browser-render verification is unavailable and was not bypassed. Embedded screenshots were separately viewed. Narrow smoke used an effective 487px viewport despite requesting 390px; measured card bounds fit, but cropped/distorted capture output is not a full mobile visual sign-off. Dialogs were canceled and viewport restored. Pending/failure, nested cleanup and unmount restoration are deterministic-test evidence, not live destructive-request evidence. Actual deletion was never activated. Existing full-CI blockers remain unchanged.

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

### Slice 7 — P1 account menu, 2026-09-20

Started from `310958b` in the existing isolated NOMS-020 worktree. Replace only the desktop account dropdown, retaining the current navbar, mobile drawer, real avatar/initials, profile route, Settings route and native logout POST. Add the approved Dashboard shortcut and identity handle, with neutral Sign Out and simple decorative line icons. Use existing Nunito and warm overlay tokens, rounded rows, visible focus, opaque/forced-color material fallbacks and reduced-motion trigger behavior; do not introduce new palette or page-wide blur.

The real named button opens a nonmodal menu. Enter/Space use native activation, ArrowDown/ArrowUp open at first/last item, arrows wrap, Home/End jump, and character keys select matching items. Escape closes and returns focus; Tab/Shift+Tab close and resume normal page traversal from the trigger. Outside pointer/focus closes without forcing focus back. The panel remains mounted when an action is chosen so logout's form can submit normally. Positioning follows scroll/resize, clamps to edges and chooses available vertical space with internal scrolling; hiding the desktop trigger closes its top-layer panel. Lifecycle listeners are disposed with the navbar. Mobile layout discovery and P2/P3 remain separate.

Tests-first Node run failed because the new lifecycle module did not exist. Final lifecycle/geometry/markup tests pass, including disabled-item skipping, typeahead, outside dismissal, breakpoint closure, idempotent mount/disposal and retained submit DOM. Existing Rust logout form test passes. Independent code review **PASS**, no blocking/should-fix findings; native browser Tab and logout behavior require runtime evidence beyond mocked tests.

Runtime exposed a P2: resetting the panel's maximum height during scroll repositioning left the final focused row partly clipped in a short viewport. Added a failing regression test, then measured natural content height without temporarily resizing the scroll container. Renewed independent review **PASS** and 15 overlay Node tests pass. One focused Rust logout test passes; `just ci` passes client/server compilation and WASM Clippy, then fails the unchanged server `needless_return` at `informational/mod.rs:34`. Formatting/whitespace, six schema cases and security-policy checks pass. A 120-second full Rust run again timed out on unchanged `db::tests::runtime_grants_enforce_a_non_owning_data_plane_role`; no full-suite pass is claimed.

Fresh isolated `noms020-account-menu-20260920` serves port 32803 with mock OAuth. Docker build metadata initially required elevated filesystem access; the exact failed deployment was removed and restarted successfully. Previous feedback deployment/data/caches were removed while retaining its evidence. The app container was rebuilt after the scrolling fix. Main-agent CUA verifies real named trigger/expanded state, initial Dashboard focus, ArrowUp opening at Sign Out, End, Space/Enter opening, typeahead, Escape focus return, forward Tab to theme toggle, reverse Tab to How to, all three navigation destinations and actual native Sign Out returning to `/` with Sign In present and no account trigger. Menu bounds fit the viewport; runtime material uses existing Nunito and 94%-opaque warm glass, without a modal or body scroll lock.

Corrected short-height evidence: effective viewport 1000×225; menu bottom 216.97px, focused Sign Out bottom 206.35px, scroll height 294px versus client height 153px. At effective 750px width, hiding the desktop trigger closes its menu; the viewport override was reset. Browser screenshot capture is scaled/clipped at the right edge, so it is partial visual evidence, not a full-fidelity screenshot or responsive-design sign-off. Outside dismissal, disabled-item skipping and cleanup have deterministic test evidence; assistive-technology announcements and preference emulation are not newly browser-verified. Evidence: `/tmp/noms020-account-menu-20260920/browser-proof.json`, `menu-clipped.png`; logs `/tmp/noms020-account-menu-scroll-red.log`, `/tmp/noms020-account-menu-node.log`, `/tmp/noms020-account-menu-rust.log`, `/tmp/noms020-account-menu-ci.log`, `/tmp/noms020-account-menu-full-tests.log`. No new dependency or backend behavior. P2 action menus are next; checkpoint 1 remains incomplete.

### Slice 6 — T2 tooltips and toast feedback, 2026-09-20

Based on `10211a9`, shared `TooltipAction` and `FeedbackToast` introduce the selected soft warm-glass capsules using existing Nunito, palette and overlay tokens. Collection Copy link/Share are the first consumers; their API calls and edit/visibility gating remain unchanged. Duplicate pending actions are guarded. No global notification migration, page redesign, menus or image-editor layout changes are included.

The first-slice lifetime contract is one persistent toast per page, replaced by the next completed action or explicitly dismissed, without auto-dismiss or invented Retry/Undo actions. Pending actions retain the previous result. A permanently mounted polite live region receives a new keyed child per completion, including repeated identical results; notification presentation does not take focus, dim the page or lock scrolling. Tooltips supplement named buttons on hover/focus, support Escape and a hoverable gap, clamp/flip at viewport edges, follow scroll/resize and clean up listeners on unmount. Long messages wrap; opaque and forced-color fallbacks are defined without introducing motion.

Tests were added before implementation and failed on missing modules. Independent review found a P2: starting another action cleared the persistent toast prematurely. A failing regression test was added, that clearing was removed, and renewed review returned **PASS**, no blocking/should-fix findings. Focused Rust feedback contracts: three passed. `node --test scripts/tests/test-feedback.mjs scripts/tests/test-shared-modal.mjs`: 12 passed. Formatting, client/server compilation, WASM Clippy, whitespace, six schema cases and security-policy checks pass. Final `just ci` still fails the untouched server Clippy `needless_return` at `informational/mod.rs:34`. The bounded full Rust suite stalled again on unchanged `db::tests::runtime_grants_enforce_a_non_owning_data_plane_role`; no full-suite pass is claimed.

Fresh isolated `noms020-feedback-20260920` at port 32798 was rebuilt with the reviewed source. Main-agent CUA browser checks on a disposable unlisted collection verify actual clipboard success, one toast after repeated Copy link actions, preserved action focus, explicit toast dismissal, Escape dismissal of the tooltip, keyboard focus opening Share help, and no native modal or body scroll lock. The desktop screenshot was visually inspected, including the right-edge tooltip placement. Native Share-sheet success/failure, real assistive-technology announcements, narrow layouts and media-preference rendering are not newly browser-verified; geometry/lifecycle and fallback contracts have automated/source evidence. This is scoped foundation acceptance, not completion of checkpoint 1.

Evidence: `/tmp/noms020-feedback-20260920/browser-proof.json` and `tooltip-toast.png`; logs `/tmp/noms020-feedback-red.log`, `/tmp/noms020-feedback-persistence-red.log`, `/tmp/noms020-feedback-ci-final.log` and `/tmp/noms020-feedback-full-tests.log`. Previous disposable image-editor deployment/data/caches were removed, retaining its evidence. Startup-only generated Tailwind utilities were excluded.

### Slice 5 — image-editor modal migration, 2026-09-20

Implementation `df993f8`, based on `073867f`, mounts `SharedModal` while the editor has an image. The existing specialized editor card, crop modes, controls and save/close callbacks remain unchanged. The shell provides a named dialog, initial Cancel focus, Tab containment, native background inertness, scroll locking and cleanup. An optional backdrop-specific callback preserves the crop drag-release guard without suppressing Escape; ordinary modal consumers retain their existing fallback callback. Close now has an explicit accessible name. No artificial pending state was added: the editor reports adjustments synchronously and upload persistence belongs to its consumers.

Tests-first migration contract failed before implementation. Runtime then exposed a pre-existing adjacent defect: clicking Flip inside the collection form submitted that form. A second failing regression test required explicit `type="button"` for every editor button; all eight now have it. This created one private fixture collection in the disposable environment before correction; no real user data was changed. Independent review of the complete corrected diff: **PASS**, no introduced blocking/should-fix findings. Enter-based implicit form submission from numeric fields was not exercised and is not claimed as covered by the button-type fix.

Final focused `SQLX_OFFLINE=true cargo test --features server image_editor`: four passed. Shared lifecycle Node tests: nine passed. `just fmt`, both `just check` targets, WASM Clippy, whitespace, schema/workflow and security-policy checks pass. Full `just test` stalled on unchanged `db::tests::runtime_grants_enforce_a_non_owning_data_plane_role` and was interrupted; a bounded 120-second full-suite retry stalled on the same test and timed out. No full-suite pass is claimed, and no test was weakened or skipped. `just ci` still fails the pre-existing server Clippy warning at `informational/mod.rs:34`.

Fresh `just up instance=noms020-image-modal-20260920` completed build/schema/health at port 32793. The previous collection deployment/test data/caches were removed, retaining its evidence. Main CUA in-app-browser checks used mock `testuser` and a prior disposable screenshot as the image fixture: upload opened the named editor with Cancel focused; Tab and Shift+Tab wrapped; native modal/overflow locks were observed; crop resizing/dragging preserved the drag-release backdrop guard; Escape restored the file-picker focus. After rebuilding the button fix, Flip and keyboard undo/redo kept the editor open; OK updated the local image preview and closed only the editor, remaining on the unsaved collection form; reopening and Cancel restored overflow. No collection Save/Create was intentionally invoked in the corrected flow. Avatar-specific behavior, mobile layout and Enter-based implicit submission are not newly browser-verified.

Evidence: `/tmp/noms020-image-modal-20260920/browser-proof.json` (includes initial failure and corrected checks) and `editor.png`. Logs: `/tmp/noms020-image-modal-red.log`, `image-modal-button-red.log` with the same `noms020-` prefix, `/tmp/noms020-image-modal-focused-final.log`, `/tmp/noms020-image-modal-tests.log`, `/tmp/noms020-image-modal-tests-retry.log`, and `/tmp/noms020-image-modal-ci-final.log`. The upload tool had a prolonged response delay; no non-CUA automation workaround was used. This is a scoped behavior migration, not an I1B layout implementation or full checkpoint-1 acceptance.

### Slice 4 — collection deletion migration, 2026-09-20

Implementation `821287e`, based on `6c7837c`, reuses `SharedModal` for collection deletion. The inner M4 card, warning about recursive collection/membership deletion and retained recipes, exact case-sensitive phrase, initial phrase-field focus, API call and Dashboard destination are preserved. The page-specific document keyboard listener/eval lifecycle is removed. Shared native inertness, Tab boundaries, scroll locking, pending focus and cleanup now apply. Open/input/Cancel handlers explicitly reject pending interaction; dismissal clears phrase/error, failure retains phrase, and success closes the modal before navigation.

Tests-first: the updated migration contract failed because `SharedModal` was absent. `just fmt`, `just check`, full `just test` (714 Rust passed, 12 ignored; nine JS passed), and a final `cargo test --features server recursive_delete` (four passed, SQLx offline) pass. Schema/workflow/security-policy checks pass. `just ci` passes both compilations and WASM Clippy, then stops at the same pre-existing server `needless_return` warning in `informational/mod.rs:34`. Independent review of the complete migration and shared lifecycle call paths: **PASS**, no introduced P0–P2 findings.

Fresh `just up instance=noms020-collection-modal-20260920` completed infrastructure/schema/build and served HTTP 200 at port 32788. The previous disposable modal-behavior deployment and its test data/caches were removed; its evidence remains under `/tmp`. CUA in-app-browser checks authenticated mock `testuser`, created one private `Modal validation` collection, and verified the named native dialog, labelled phrase-field initial focus, reverse/forward Tab boundaries with disabled/enabled submit, case-sensitive invalid phrase rejection, exact phrase enabling, Escape/Cancel/backdrop dismissal, empty phrase on reopen, trigger-focus restoration and root/body overflow restoration. The captured desktop card preserves the existing styling. No deletion request was submitted. Pending/failure integration and success navigation are source-review evidence plus shared lifecycle tests, not newly executed destructive flows; this is not a responsive redesign sign-off.

Evidence: `/tmp/noms020-collection-modal-20260920/browser-proof.json` and `dialog.png`; automated logs `/tmp/noms020-collection-tests.log`, `/tmp/noms020-collection-focused.log`, `/tmp/noms020-collection-ci.log`, and tests-first `/tmp/noms020-collection-red.log`. No full portable HTML report was needed for this bounded consumer migration. Image-editor migration and the rest of checkpoint 1 remain outstanding.

- [x] Scope and regression contracts recorded; tests fail before production changes.
- [x] Production slice and automated checks executed; unrelated full-CI failures recorded above.
- [x] Independent code review passes (P2 sizing regression fixed and guarded).
- [x] Slice-1 fresh-runtime validation and evidence passes (Chrome fallback and capture limitations recorded).
- [x] Slice-2 ordinary-modal material implemented, tested and code-reviewed.
- [x] Slice-2 accessibility-preference runtime checks complete (six CUA in-app-browser cases, main-agent observations).
- [x] Slice-3 shared modal behavior implemented and code-reviewed; account deletion is the first consumer.
- [x] Slice-4 collection deletion migrated, code-reviewed and browser-checked without executing deletion.
- [x] Slice-5 image editor migrated, code-reviewed and scoped browser flows checked; full-suite stall recorded.
- [x] Slice-6 T2 tooltip/toast foundations implemented; collection Copy link/Share adopted and scoped browser checks recorded.
- [x] Slice-7 P1 desktop account menu implemented, reviewed and browser-checked, including native logout and corrected short-height scrolling.
- [ ] Full checkpoint-1 gate complete (includes remaining slices above).
