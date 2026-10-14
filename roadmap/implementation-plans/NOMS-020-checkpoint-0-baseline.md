# NOMS-020 checkpoint 0 — Baseline and boundaries

Started 2026-09-19. Status: complete for the representative checkpoint-0 baseline; source review and independent evidence validation PASS. No intentional application behavior or visual changes in this checkpoint.

## Revision and scope

- Worktree: `/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement`.
- Branch: `codex/NOMS-020-visual-design-refinement`; starting revision `b1218d2`.
- Local main: `ce080b42f20f6c19b12fab1151c67fabf3e86491`. `git rev-list --left-right --count main...HEAD` returned `0 4`: main is already an ancestor. No remote fetch or rebase performed.
- Application source matches local main: no differences outside roadmap, design documentation and audit artifacts. Main's uncommitted stylesheet and roadmap changes are excluded. Reuse the existing isolated NOMS-020 worktree for this baseline; no parallel implementation stream is launched.
- No applicable AGENTS.md found during repository discovery. Follow repository README, justfile, CI and the implementation skills. This checkpoint records existing contracts; new behavior tests belong with subsequent implementation slices.

## Route inventory and fixtures

Source of truth: `src/main.rs::Route`; all routes use `AppLayout`.

| Group | Routes | Required baseline fixtures/states |
| --- | --- | --- |
| Public and entry | `/`, `/about`, `/how-to`, `/login` | Signed-out and signed-in shell; auth return destination |
| Discovery | `/explore`, `/search` with query parameters | Empty and populated Explore; Search signed-out gate and authenticated results |
| Workspace | `/dashboard` | Empty account, published recipe and private draft |
| Recipes | `/recipes/new`, `/recipes/:id`, `/recipes/:id/versions/:version_number`, `/recipes/:id/edit`, `/recipes/drafts/:id/edit` | New form, image upload/editor, saved recipe, historical version and edit draft |
| Graph | `/recipes/:id/version-graph` | At least two versions and one fork; comparison/expanded state |
| Collections | `/collections`, `/collections/new`, `/collections/:id`, `/collections/:id/edit` | Empty list, nested populated collection, membership and deletion safeguards |
| Account | `/u/:username`, `/settings/profile`, `/settings/accounts`; `/settings` redirects to profile | Public profile, private settings, avatar editor, provider linking and confirmation states |
| Missing content | Catch-all route plus missing/inaccessible recipe/collection | Explicit missing or permission state without private content disclosure |

New runtime fixtures must use local mock OAuth `testuser`, not real providers or existing users' content. This table is a coverage checklist, not a claim that each state has been captured. Capture desktop light first; record viewport, auth state and fixture identifiers. Keep phone/tablet discovery with each page and preserve existing dark-mode behavior.

## Shared implementation boundaries

| Owner/workstream | Existing source paths | Contract to preserve |
| --- | --- | --- |
| Foundations | `assets/main.css`, `tailwind.css`, generated `assets/tailwind.css`, `src/utils/theme.rs`, font loading in `src/main.rs` | Existing light/dark tokens, Fredoka/Nunito, theme persistence and explicit focus/contrast |
| Shared shell | `src/components/app_layout.rs`, `navbar.rs`, `footer.rs`, `auth_required.rs`, `error_fallback.rs` | Navigation destinations, signed-in/out access, sidebar/account/search entry, content outlet |
| Shared primitives | `src/components/base/{button,input,card,page_header,empty_state,loading_spinner,avatar}.rs` | Props/events, semantic controls, pending/disabled/focus/error presentation |
| Shared content | `src/components/base/{recipe_card,collection_cover,collection_sidebar,collection_form,collection_visibility,add_to_collection_dropdown,reorder_handle}.rs` | Reusable card attribution/navigation, nested memberships, visibility and ordering |
| Editing | `src/pages/{recipe_editor_form,recipe_new,recipe_edit,draft_save}.rs`, `src/components/base/{image_upload,image_editor_modal,edited_recipe_image}.rs` | Continuous form, draft lifecycle and image transformation contract shared with avatar settings |
| Detail/graph | `src/pages/{recipe_detail,recipe_version_graph}.rs` | Historical context, lineage, comparisons, permissions and accessible navigation |
| Account | `src/pages/settings/`, `src/pages/user_profile.rs`, `src/components/base/settings_tabs.rs` | Privacy separation, save validation, linking/unlinking and destructive confirmations |

Actual controls and overlays are not all centralized: account pages contain inline modal markup. Foundations must inventory those consumers before introducing M4 shells; do not assume one existing reusable modal/toast API. Global CSS and shared primitives have one coordinating owner; route workers must request shared changes rather than duplicate implementations.

## Identity baseline

`assets/main.css` currently defines Fredoka display and Nunito body; `src/main.rs` loads Fredoka 500/600/700 and Nunito 400/500/600/700. Preserve these rather than sampling generated artwork.

Core light values: background `#FAF7F2`, surface `#F5F0E8`, coral `#D9735A` / hover `#C4613F`, primary text `#2D2A26`, secondary `#7A756D`, tertiary `#A8A29A`, shadow light `#FAF7F1`, shadow dark `#C8C1B4`. Existing `.dark` overrides remain supported.

Committed stylesheet hashes before startup:

- `assets/main.css`: `3fb43672eec1fef4f89cc96c0915a61b1ea86b156b2d516f755a637fc90ca998`.
- `assets/tailwind.css`: `634e2f491052eafb4d47ee315467eddc0b373e398e1d01a91d28352d2ea6b7c2`.

The 2026-09-17 historical audit used the same main revision but an uncommitted Tailwind file with a different hash. It remains historical evidence, not a pristine baseline for this run. Fresh startup also regenerates Tailwind; record its actual runtime hash and handle generated differences separately from visual implementation.

## Existing issues and exclusions

- Explore tag filtering is still unwired in source: `on_tag_click` changes `selected_tag`, but `get_public_recipes(off, PAGE_SIZE, query)` and the displayed cloned result list do not consume it. Do not silently fix this while restyling chips; tag-filter behavior has not been browser-verified.
- `/settings/profile` appears twice on the same route variant in `src/main.rs`; recorded as a source anomaly, not a demonstrated runtime defect or scope for cleanup.
- Historical mobile editor overflow has not yet been reverified on this runtime. Do not call it a new regression.
- Preserve server/API/auth, visibility, publishing and destructive contracts. Dashboard expansion remains separately gated; no feature-ticket work, dark redesign or production asset generation is included here.

## Verification commands and current outcomes

| Check | Outcome |
| --- | --- |
| `cargo fmt --check` | Passed |
| `just isolated-dev-config-test` | 39 passed, 0 failed |
| `just schema-check-test` | 6 schema-plan cases passed; pinned declarative workflow check passed |
| `just security-policy-test` | Passed, including its tracked-secret and deployment-artifact checks |
| `just up instance=noms020-baseline-20260919` | Failed before container creation: Docker address pools fully subnetted |
| `just up instance=noms020-baseline-20260919b` | Passed readiness; infrastructure healthy, schema-init exited 0, application HTTP 200 and browser flows verified |

Removed only the unused `noms-visual-audit_default` network (created 2026-08-23; no attached or stopped member containers), under prior test-deployment cleanup authorization. No containers, volumes or application data removed. Compose can recreate the network when needed.

The retry is a new isolated Compose project, with application mapping `http://127.0.0.1:32768/`. The first in-app browser navigation reported `net::ERR_BLOCKED_BY_CLIENT` while the application was still building; navigation succeeded after readiness. This was not reproduced as an application defect.

## Fresh runtime evidence

Captures are temporary validation artifacts at `/tmp/noms020-baseline-20260919/`, not replacements for the retained historical audit. Captured in the in-app browser at 1920×1080, devicePixelRatio 1, light mode; viewport override reset afterward. Used native CDP capture with captureBeyondViewport and an explicit 1920×1080 clip because the ordinary screenshot surface returned a clipped 1280×720 image. No image stretching or content editing.

| Capture | State |
| --- | --- |
| `01-home.png` | Signed-out Home and shared navigation |
| `02-dashboard-empty.png` | New mock account, empty dashboard |
| `03-editor-draft.png` | Private edit draft of the published fixture, recaptured at scroll position 0; viewport capture, not whole form |
| `04-recipe-detail.png` | Published local fixture, ingredients, steps and v1 history |
| `05-explore-populated.png` | One public recipe with author attribution and no-photo fallback |
| `06-explore-no-results.png` | Search for `no-such-baseline-recipe` |
| `07-sidebar.png` | Authenticated sidebar open over Explore |
| `08-profile-settings.png` | Fresh mock profile, unchanged settings |

Mock OAuth subject `testuser` created local username `user-a998`. Browser sign-in returned to Dashboard. Created only a disposable local recipe, **Baseline Tomato Soup**, ID `c76b5d5c-e8ad-4796-908a-0e80256da220`, with two cups tomatoes, one simmer/blend step, tags `soup, baseline`, no images, and public visibility inside this loopback-only test instance. Save Draft displayed “Autosaved privately”; Publish Recipe produced v1 and a populated Explore card. A no-match query displayed “No recipes found”; keyboard clearing restored the result. Sidebar Settings navigation reached the profile form.

The Dioxus development rebuild banner remained in DOM snapshots but was not visible in inspected captures; do not infer a visible overlay regression from that DOM text. Startup-generated Tailwind SHA-256 was `2158021740a3b9275d8a7cf63011c4703a5a0fd21136508703cdaa5c11414f56` during initial captures; additions were generated utility rules, not intentional visual changes. Those startup-only additions were reverted before completion. The replacement editor capture uses the restored committed CSS. Opening its edit route initialized a private edit draft; no fields were changed or republished. The initial editor screenshot had a scrolled fixed-header capture artifact and was replaced after explicit scroll-position verification.

This is a representative baseline sufficient to begin foundations, not exhaustive route acceptance. Nested collections, image-upload states, multi-version/fork graph, backend errors, full keyboard/accessibility assessment and responsive/dark captures must be added for their relevant slices. No claim of full regression coverage or completed production assets.

Full `just ci` has not been run: these are baseline checks, not evidence of a passing complete application suite. For foundation implementation, run focused tests first, then the discovered CI gate (format, client/server compilation, Clippy, server tests, schema/security-policy tests and security audit/artifact checks). Tailwind changes require a generated CSS review. Browser evidence must supplement, not replace, tests.

## Exit gate

- [x] Starting revision, clean-source relationship to local main and unrelated work recorded.
- [x] Routes, shared paths, identity tokens and known behavior boundaries inventoried.
- [x] Reproducible fast checks run and outcomes recorded.
- [x] Fresh application ready; representative desktop captures with fixture/state metadata recorded.
- [x] Populated fixture and auth/navigation behavior verified.
- [x] Baseline reviewed and accepted before checkpoint 1 visual changes.

## Review and handoff

Independent source review: PASS; corrected ambiguous wording about tag-filter browser coverage. Independent evidence validation: PASS for checkpoint 0 only; verified HTTP 200, all eight 1920×1080 PNG dimensions, representative image content, replacement editor capture, formatting and diff checks. Browser interactions above were performed by the main agent, not independently replayed by the reviewer.

Portable self-contained report: `/tmp/noms020-baseline-20260919/report.html`, with all eight screenshots embedded. Temporary artifacts may expire; retain or archive them before relying on them across environments. This is one coherent documentation-only checkpoint, so a single commit is appropriate rather than artificial multiple commits. Existing isolated NOMS-020 branch/worktree is reused per repository skill guidance. Next: checkpoint 1 shared foundations, with behavior tests before production edits and current palette/font preservation.
