# NOMS-020 checkpoint 2 — Navigation slice

Implementation commit: `12473b7`.

2026-09-23 visual follow-up: the original green-tinted, bordered active destination below was superseded. Active navigation now matches the opaque navbar/search surface, uses the shared inset elevation, and has no decorative dark border; keyboard focus and forced-colors boundaries remain explicit.

Started 2026-09-22 from `56b5028`, after checkpoint-1 close-out. Reuse the user's existing isolated `codex/NOMS-020-visual-design-refinement` worktree; no main changes, rebase or additional worktree is needed. This slice implements N1/N4 shared navigation, not Explore or the full checkpoint-2 exit gate.

## Contract and boundaries

- Opaque cream, inset rounded horizontal shell with existing raised/inset elevation, Fredoka/Nunito and coral accents. Glass is reserved for the account menu and navigation dialog. Active destinations use the existing green-tinted surface plus explicit border and `aria-current`; no decorative props or slogans.
- Signed-in primary destinations: Dashboard, Collections, Explore; coral New recipe plus existing P1 identity/account menu. Guest primary destinations: Home, Explore, About, How to; Sign In. Brand returns Home. All previous destinations remain in the consolidated Menu, including guest access to routes with existing sign-in gates. No invented Recipes, Drafts or Meal plan routes.
- Labelled search uses the landing page's canonical Rust route helper, default scope and permissions, with native GET fallback before hydration. Input respects the existing 256-character limit. No autocomplete or Search-page redesign; the existing encoded-character routing limitation below remains.
- One labelled SharedModal navigation dialog replaces both old drawers: initial Close focus, Escape/backdrop dismissal, focus containment/restoration, scroll locking, native background inertness, internally scrollable content and normal links. Both logout forms stay mounted until native POST. Route transitions close the dialog. Sign Out remains neutral, not destructive red.
- Full horizontal layout at 1100px and above; compact tablet layout keeps brand, create/account/theme and Menu, with search and destinations in Menu. Below 600px, brand plus labelled Menu; all actions remain in Menu. Long identity labels truncate in the bar and wrap in the dialog. Verify edge widths, portrait tablet, phone and short-height keyboard scrolling before acceptance; this is the shell's responsive discovery, not a mobile page redesign.
- Keep current brand mark while production S1 assets remain unprepared; no placeholder substitute is called final S1. Footer and route content are unchanged. Explore X7C remains the next separate slice. Preserve existing dark mode; no new dark visual direction.

## Ownership and verification

Owned paths: `src/components/navbar.rs`, `src/components/navbar/navigation_tests.rs`, the scoped navigation/app-shell block in `assets/main.css`, and this plan. Existing shared modal/account lifecycle contracts are consumed without new JavaScript or backend changes.

Tests first: four navigation tests initially fail on missing route helpers (`/tmp/noms020-navigation-red.log`), then pass. Automated route-family tests cover auth-specific primary lists and descendant active state; source guards cover native search, single modal shell, material and responsive rules. These guards do not replace browser keyboard or visual checks.

Runtime uses a restarted app container in the existing isolated `noms020-picker-20260922` deployment (port 32779), preserving the user's current fixture data. The development tooling prevents concurrent deployments of the same worktree. A fresh browser session provides guest/auth checks; collection fixture state cannot conceal shell rendering or keyboard behavior. Baseline screenshot: `/tmp/noms020-navigation/before.png`.

Independent review found one should-fix: submitting an unchanged search route left the drawer open. A tests-first callback guard failed, then explicit `on_navigate` dismissal was added. Chrome verified repeated `bread` searches close the drawer even when the route stays `/search?q=bread`. Final code review **PASS**, no remaining blocking/should-fix findings; reviewer independently ran five navbar tests.

## Browser evidence and limitations

CUA Chrome verification: desktop guest/signed-in composition; 1100px full header fit (search width about 289px), 600px compact controls, portrait 768px tablet, 390px phone, and 320×300 short-height drawer. On the shortest viewport the final theme control scrolled into view at y219–263 inside a panel ending at y284. Native modal state, initial Close focus, forward/reverse Tab wrapping, Escape restoring Menu focus and body overflow, route selection dismissal, normal header/drawer search, same-route search dismissal, and dark/light toggle work. Drawer native POST sign-out returns Home with Sign In visible. No collection/recipe data was modified.

The in-app browser continued executing obsolete drawer code after reload/restart/cache bypass even though the served WASM hash matched the rebuilt output; Chrome was used for final interaction evidence. Temporary cache/viewport overrides were restored. Chrome screenshots under viewport emulation can be scaled/clipped: images are partial visual evidence, not full-fidelity responsive captures. DOM geometry and keyboard observations support the bounds checks. Real assistive-technology announcements are not independently tested.

Existing Search issue discovered: `soup & herbs` is truncated to `soup` through both native GET and the already-existing landing-page route helper. Source inspection finds Dioxus 0.7.9 percent-decodes the whole query before the app splits parameters. This is not corrected or worked around in navigation; track it for the Search slice, including special characters, full reload, and auth-return URLs. Normal queries retain text and route correctly. Do not claim special-character end-to-end correctness from the helper's encoding alone.

Evidence directory: `/tmp/noms020-navigation/` (browser proof JSON and baseline/final captures). Logs: `/tmp/noms020-navigation-red.log`, `/tmp/noms020-navigation-search-red.log`, `/tmp/noms020-navigation-dismiss-red.log`, `/tmp/noms020-navigation-ci-final.log`, `/tmp/noms020-navigation-security.log`. Sandboxed CI passes compilation, both Clippy targets, 742 tests (12 ignored), schema and policy checks, then hits a read-only advisory-cache lock. Separate elevated dependency audit passes with eight policy-allowed warnings; the complete elevated gate below supersedes that environment limitation.

Final gate: elevated `just ci` passes in full (`/tmp/noms020-navigation-ci-complete.log`): both compilation targets and Clippy targets, 742 tests passed/12 ignored, schema/policy/secret/deployment checks and dependency audit. Both drawer and P1 account-menu native logout return Home with Sign In; mock sign-in was restored afterward. Backdrop dismissal also verified. Default-viewport final guest-menu capture was visually inspected after allowing the browser paint to settle. Startup-only Tailwind utilities were excluded.

Navigation implementation acceptance: **PASS**, with the existing Search routing issue, production S1 asset and screenshot/assistive-technology limitations explicitly retained above. This is only the navigation slice. Explore X7C and explicit combined checkpoint-2 review remain outstanding; wider parallel page rollout is still gated.
