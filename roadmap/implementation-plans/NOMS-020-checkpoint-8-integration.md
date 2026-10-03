# NOMS-020 — Integrated browser review

**Date:** 2026-09-27

**Status:** Page implementations integrated; release acceptance OPEN.

**Tested source:** `74c420035ce29ccbb09817f4325252f56b4c5a80` on `codex/NOMS-020-visual-design-refinement`.

This header and the original review below describe the 2026-09-27 baseline.
See the appended **2026-09-29 and 2026-09-30 follow-ups**, including **Editor
failure recovery fixes**, for current fixes, CI results and remaining gates.

## Environment and scope

Used the already-running local deployment at `http://127.0.0.1:32768`, container
`noms-ca6962a21bc5bc35-app-1`. Its Compose working-directory label points to
`.worktrees/NOMS-020-visual-design-refinement`. No restart, new deployment,
production-code edit or application fix was made for this review. Existing
generated `assets/tailwind.css` changes were preserved; this was a live dev
deployment, not an immutable release build.

Used the Codex in-app browser and local mock Google subject `testuser`
(`@user-c5a3`). Added private validation fixtures through the UI, not to
production. They remain available for reproduction:

- Collection: **NOMS020 integration review 2026-09-27**, ID
  `75c55acb-5eed-47a6-a4dc-f7fef5092798`.
- Recipe: **NOMS020 integration soup 2026-09-27**, ID
  `233b1cf1-e024-47b9-ae56-15c1f55b8263`, private, two published versions.
- Mock profile display name saved as **Integration Review** during the save test.

No real-provider linking, unlinking, deletion, restoration or public publication
was performed. Findings below are follow-up work, not silently implemented fixes.

## Integrated inventory

These source tips are ancestors of the tested revision. Ancestry proves inclusion,
not complete runtime acceptance.

| Workstream | Included tip | State |
| --- | --- | --- |
| Home | `6a570cd` | Integrated |
| About | `0cb878e` | Integrated |
| How-to | `df53ae9` | Integrated |
| Sign-in | `eedc979` | Integrated |
| Search | `ac9acb3` | Integrated; [source evidence](NOMS-020-search.md) |
| Dashboard | `c822f03` | Visual migration integrated; [functional expansion deferred](NOMS-020-dashboard.md) |
| Collections | `aa5280e` | Integrated; [source evidence](NOMS-020-collections.md) |
| User pages | `6a8b9c6` | Integrated; source evidence in [master plan](NOMS-020-visual-design-refinement.md) |
| Recipe editor/detail/history/graph | `31cac9b` | Integrated by `74c4200`; [source evidence](NOMS-020-recipe-pages.md) |

Foundations, navigation and Explore precede these workstreams in the integrated
history. Source-worktree checks are not a fresh full-suite run of this revision.

## Browser observations

| Flow | Observed result |
| --- | --- |
| Public entry | Home, About and How-to rendered redesigned compositions. Home query submission reached `/search?q=tomato`; signed-out Search displayed a sign-in gate. L6B displayed both providers and guest/account copy. |
| Authentication/workspace | Mock Google sign-in reached Dashboard. Empty library/draft states, sidebar navigation and list/grid controls were present. After publishing, the library showed the private fixture and owner actions. |
| Dashboard → Search | Search my recipes retained `scope=recipes&vis=mine`. Empty state worked before fixture creation. Afterwards the recipe appeared; submitting `NOMS020` retained Mine only and returned the fixture with title, description, servings and author link. |
| Collection create/edit | Empty list → new private collection → detail succeeded. Editing and saving the description returned updated detail. |
| Draft/publish | All six raised editor sections and tracker were present. Title, description, two servings, `2 cups water` and method saved privately, then published as private v1. |
| Membership | Recipe detail's picker selected the new collection and confirmed addition. Collection detail showed one recipe and author link; singleton reorder actions were disabled. |
| Edit/history/graph | Editing the method and publishing produced v2. History exposed both versions. Graph showed two nodes and comparison/outline controls. v1's Compare with current showed the exact method change at `/instructions/0/text`. Zoom in and Fit were exercised. |
| Historical snapshot | v1 opened read-only historical context with the old method. Comparison against v2 showed the change; comparison against original v1 reported no differences. Back to current worked. |
| Profile/settings | Empty-name defect observed before editing (below). Saving Integration Review produced success; reload followed by public-profile navigation retained the name. Navbar identity updated. Linked accounts showed Google connected and disabled Unlink for the only provider. |
| Private-content boundary | Owner saw the fixture in Dashboard/Search/collection detail. Public profile showed zero public recipes/collections, and Explore remained empty. This is fixture-specific evidence, not a complete authorization audit. |

The exact method comparison was **“Warm the water.” → “Warm the water, then let
it cool briefly.”** No fabricated comparison data was used.

Representative screenshots were inspected in-session. Viewport checks used
measured CSS sizes (the browser applied a 1.25 scale factor): 1440×900 desktop,
820×1025 tablet and 390×843 phone. Search, collection edit and recipe editor had
no horizontal document overflow at 390px; populated collection detail had none
at 390px or 820px. Document widths were 371px/801px with scrollbars. These are
selected-route geometry checks, not a complete responsive acceptance matrix.

Emulated screenshot capture showed scale/crop discontinuities; those artifacts
were not treated as application clipping. No screenshot files were added to the
repository. Final visual acceptance needs reliable full-page/device captures.
Temporary viewport overrides were reset. The final browser warning/error query
returned no entries, not proof that all error paths work. Development
rebuild/hot-patch messages appeared in DOM snapshots during the session.

## Findings and acceptance tasks

### Resolved: subcollection parent context

- [x] Fix and regression-test nested creation, then recheck creation, breadcrumb,
  parent listing and Cancel in the integrated browser.

Resolved by `9b6d667` on 2026-09-27; see the fix validation below. The original
reproduction is retained for context.

Open the fixture collection and choose **New subcollection**. The URL contains
`?parent=75c55acb-5eed-47a6-a4dc-f7fef5092798`, but the settled form says
**New collection** and **“This album will appear at the top level of your
collections.”** Cancel returns to the top-level list instead of the parent.
Reproduced without a second submission; no successful child creation is claimed.

`src/pages/collection_new.rs` initializes the parent signal from browser location.
Investigate route-query initialization/hydration rather than assuming an exact
cause. Form/cancel context is wrong; storage behavior and whether this predates
the visual work have not been established.

### Resolved: empty display-name public profile

- [x] Define a nonempty identity fallback consistent with the shell, add
  regression tests and verify the rendered page with an empty-name account.

Resolved by `9cbeef5` on 2026-09-27. Unit tests cover blank/whitespace and nonblank
names; rendered-page checks were performed in the browser, as recorded below.
The original observation follows for context.

Before saving a name, `/u/user-c5a3` had an empty h1, `?` avatar fallback and copy
beginning “hasn't shared…”, while the navbar used `user-c5a3`.
`src/pages/user_profile.rs` renders `display_name` directly in those areas.
Saving a name makes the page display correctly but does not fix the empty-name
case. The issue's introduction revision is not established.

### Resolved: guest-search promise and destination

- [x] Reconcile Home/sign-in copy and destination with existing access rules.
  Do not remove authentication from Search without explicit scope.

Resolved by `453eb0e`, verified 2026-09-29. The landing page now explains full
Search requires sign-in and links to public Explore search; sign-in copy names
Explore explicitly. Encoded-query and client sign-in-return findings from that
later pass remain separate open issues. Original observation follows.

Sign-in says guests can browse and search public recipes; Home Search navigates
to authenticated Search and prompts sign-in. Explore has public search, so the
promise may be intentional but its entry path needs clarification. This is not
established as a new regression or authorization bug.

## Remaining gates and next order

Historical queue from the original review; the 2026-09-29 section below and the
master plan supersede it. Production SVG/page artwork has since been integrated.

1. Address the remaining collection-navigation finding in a focused fix;
   resolve the guest-search copy/destination question.
2. Run full repository-required CI on the resulting combined revision. No fresh
   automated application suite ran during this docs/browser pass. Source
   checkpoints retain their exact historical commands/results and caveats.
3. Finish cross-route checks: image editor through recipe and avatar callers;
   nested/multiple membership; populated Explore and search pagination/encoded
   queries; save/retry failures; auth return destinations; permission/destructive
   cancel boundaries; dense/forked/stale-request graphs. These are untested cases,
   not additional observed failures.
4. Complete keyboard/focus, screen-reader/status, contrast, reduced-motion and
   per-page phone/tablet acceptance. Existing dark-mode smoke is still required
   on the combined revision; dark redesign stays deferred. Earlier source-branch
   dark checks are not a new integrated dark pass.
5. Review the app as one system and obtain visual acceptance. Production S3
   (selected 2026-09-27, superseding S1),
   SVG/favicon/3D and optional page icons remain separate pending deliverables;
   keep the current mark until replacement assets are approved. Dashboard bulk
   operations/expanded management are not part of this close-out.
6. Record final known/deferred issues and a rollback target before a separately
   authorized deployment. Keep slices reversible; revert a specific reviewed
   change on a new branch if needed, not a destructive reset of this worktree
   or its test database.

**Verdict:** Integrated happy-path checks passed where listed; checkpoint 8 is
**not accepted** while the confirmed findings and remaining gates are open.

## Parent-context fix validation — 2026-09-27

Implementation commit: `9b6d667`, developed on `codex/NOMS-020-subcollection-parent`
in `.worktrees/NOMS-020-subcollection-parent`, then fast-forwarded into the
NOMS-020 integration branch. Tests and the route fix form one cohesive commit;
this follow-up records runtime evidence separately. Existing Tailwind changes
were not committed or overwritten.

Cause: the router did not declare `parent`, so route parsing/serialization dropped
it. The page separately initialized a signal from browser-only `window.location`,
returning `None` on the server. The fix declares `parent: Option<uuid::Uuid>` in
the route and consumes the prop directly for form/breadcrumb text, Cancel and
the pending create request. No backend or permissions behavior changed; missing,
empty and invalid parent values retain the existing top-level fallback.

Automated evidence (commands run from the fix worktree with
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`):

- Before the fix, `SQLX_OFFLINE=true cargo test --features server collection_creation_route_preserves_parent_without_a_browser -- --nocapture`
  failed as intended: serialized URL was `/collections/new`, losing `?parent=…`.
- `SQLX_OFFLINE=true cargo test --features server pages::collection_new::tests -- --nocapture`:
  4 passed, including actual route parsing and server-rendered nested/top-level forms.
- `SQLX_OFFLINE=true cargo test --features server collection -- --test-threads=4`:
  93 passed, 0 failed, 2 existing ParadeDB integration tests ignored. Includes
  hierarchy/ownership tests and selected-navbar coverage for both route variants.
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings`,
  `cargo clippy --target wasm32-unknown-unknown -- -D warnings`,
  `cargo fmt --check`, and `git diff --check`: passed.
- Second-pass self-review of the complete change: PASS for this scoped fix.
  No independent agent review or fresh full repository suite is claimed.

Runtime evidence on the existing `:32768` deployment:

- Restarted only the app container to pick up the route change; the database and
  fixtures were retained. A stale cached client initially disagreed with the new
  server markup. Reloading with cache temporarily disabled restored consistent
  behavior; caching was restored after verification.
- The original New subcollection link and a direct reload retained `?parent=…`;
  heading/breadcrumb showed New subcollection and location copy described nesting.
- Cancel returned to the correct parent URL **and rendered parent content**.
- Created private child `df67b11e-f634-4e77-962d-830da8f60bd8` under the original
  fixture. Its detail breadcrumb and edit parent selector confirmed the parent.
  The first immediate input fill was not retained during page initialization,
  so the child initially used the default name; it was then renamed through Edit
  to **NOMS020 child verification 2026-09-27**. The fixture remains for reproduction.
- Reloading the parent showed the child under Subcollections; the top-level list
  still contained only the parent and reported one subcollection.
- Top-level New collection after the nested flow showed top-level copy, with
  Cancel returning to the collection list rather than retaining the old parent.

### Separate follow-up: collection-to-collection navigation retains stale content

- [ ] Reproduce and fix data refresh when navigating between collection IDs
  without a full reload; add a reactive-route regression test.

During the child-detail → parent breadcrumb navigation, the address changed to
the parent ID but the child heading/content remained until reload. Reload showed
the correct parent with the newly created child. This is distinct from creation
parent context; it was recorded, not modified, in this fix. Introduction revision
is not established. Checkpoint 8 remains open for this and the other acceptance
gates above; the profile-name issue is resolved below.

## Painted book standardization — 2026-09-27

Follow-up implementation `46263cf` on `codex/NOMS-020-painted-brand`, developed
in `.worktrees/NOMS-020-painted-brand` and fast-forwarded into NOMS-020.
At the user's request, all production book-logo placements now use the same
`noms-logo-painted-s3-v1.png` source and the same optimized 192 × 240 WebP URL.
This supersedes the SVG navbar/footer choice recorded in the original integration
below. The SVG remains an unused alternative; no source assets were removed.

`BrandMark` now has a size-only `large` option instead of the asset-selecting
`painted` option. Navbar/footer remain 32 × 40 and sign-in remains 96 × 120,
with the existing 80 × 100 narrow-screen rule unchanged. Decorative accessibility
treatment, wordmarks, named home links and page icons are unchanged.

Validation reused the integration target cache and existing port-32768 deployment:

- The updated source-equality regression failed against the old SVG default, then
  passed with the unified painted source. Both brand-artwork tests passed.
- `SQLX_OFFLINE=true cargo test --features server --bin noms`: 781 passed,
  0 failed, 12 ignored.
- `cargo fmt --all --check`, `git diff --check`,
  `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings` and
  `SQLX_OFFLINE=true cargo clippy --target wasm32-unknown-unknown -- -D warnings`
  passed from the task worktree.
- The existing app container alone was restarted to rebuild. A fresh sign-in
  browser inspection confirmed that navbar, sign-in and footer images all loaded
  the exact same hashed WebP URL, with respective 32 × 40, 96 × 120 and 32 × 40
  measured dimensions, empty alt text and aria-hidden. A desktop screenshot
  confirmed the navbar painting and larger sign-in painting match.
- Temporary cache disabling was restored and the test tab was closed. Existing
  deployment data and the pre-existing generated Tailwind change were preserved.

Second-pass self-review: PASS for this source-standardization change; no
independent-agent review. Phone/tablet and dark acceptance were not rerun;
their existing sizing rules and theme tokens were not changed.

## Brand artwork integration — 2026-09-27

Implementation: `6f24dfb`, developed on `codex/NOMS-020-brand-ui` in
`.worktrees/NOMS-020-brand-ui`, then fast-forwarded into the integration worktree.
The existing deployment at `http://127.0.0.1:32768` was reused; only its app
container was restarted to rebuild. Database, OAuth and storage fixtures were
preserved. The pre-existing generated `assets/tailwind.css` change was not staged.

Placements:

- S3 SVG beside the existing navbar/footer wordmark, replacing the cutlery emoji.
- S3 painted book above the sign-in heading.
- Dashboard pinboard, Explore compass, Search magnifier/book, Collections bookshelf,
  editor clipboard, History ledger/clock, Profile identity card, Settings controls.
- Collections list/detail/create/edit share the bookshelf; new/edit recipe headers
  share the clipboard. History appears in the recipe history section and graph
  loading/error/empty/populated headers. Both settings routes share the controls.
- Profile artwork accompanies the identity eyebrow, not the user's avatar.
  Small menu/action icons, existing fonts, colour tokens and controls are unchanged.

The shared Rust components reserve dimensions, retain aspect ratio, use empty
alt text and aria-hidden for decorative artwork, and preserve text headings and
named home links. Dioxus/Manganis produces hashed transparent WebP derivatives:
192 × 192 for page art and 192 × 240 for the painted book. Header display is 72px,
compact placements 48px, reducing to 56/40px on narrow screens; the book is
96 × 120 (80 × 100 on narrow screens). Source PNGs remain unchanged.

Automated checks from the task worktree, reusing the integration target cache:

- The new sign-in regression failed before wiring, then passed after integration.
- `SQLX_OFFLINE=true cargo test --features server --bin noms`: **781 passed,
  0 failed, 12 ignored**. Includes all eight rendered icon identities, logo sizes,
  painted sign-in artwork, header text/action preservation and parent-route tests.
- `cargo fmt --all --check`, `git diff --check`,
  `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings` and
  `SQLX_OFFLINE=true cargo clippy --target wasm32-unknown-unknown -- -D warnings`
  passed. These are not a substitute for the full release CI gate.

Browser evidence on the rebuilt existing deployment:

- Inspected navbar/footer, sign-in, Explore, populated Dashboard, Search,
  Collections, new recipe editor, public profile, both Settings routes, recipe
  history and populated version graph. All eight icon sources loaded; observed
  natural sizes confirmed optimized WebP output. Both SVG home links loaded with
  32 × 40 artwork; footer retained its inline wordmark alignment.
- Screenshots assessed desktop Explore, sign-in, Dashboard and graph compositions.
  Desktop pages measured without horizontal document overflow.
- At a measured 312px CSS viewport, long graph and collection titles wrapped
  within the container with 56px icons and no horizontal overflow. Subcollection
  creation retained its parent query and contextual heading after navigation.
  This is representative compact-width smoke coverage, not all-page mobile or
  tablet acceptance. The compact screenshot renderer produced a scaled/cropped
  capture, so DOM measurements were used for that geometry check.
- Local Google mock sign-in, Settings-tab navigation and recipe-actions → graph
  navigation worked. Explore search reached the expected no-results state.
  No error-level console messages were captured in the test tab.
- Temporary viewport/cache overrides were restored. No recipes, collections or
  profile data were modified during this artwork verification.

Second-pass self-review: **PASS** for this scoped integration; no independent
agent review was performed. Favicon/single-colour exports, the 3D model, dark
regression coverage and the broader per-page/release gates remain open.

## Public-profile name fix validation — 2026-09-27

Implementation commit: `9cbeef5`, developed in
`.worktrees/NOMS-020-profile-name-fallback` on
`codex/NOMS-020-profile-name-fallback`, then fast-forwarded into the NOMS-020
integration branch. The code and regression tests form one cohesive fix commit;
this separate documentation commit records its verification.

The public page now computes one effective identity: use the username when the
display name is empty or whitespace-only; otherwise preserve the display name
exactly. The heading, avatar initials/image alt text, and both public-content
empty-state messages consume that value. Saved profile data, handles/URLs,
visibility, settings validation and shared avatar styling are unchanged.

Automated commands used
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`
from the fix worktree:

- Before the fallback was implemented,
  `SQLX_OFFLINE=true cargo test --features server pages::user_profile::identity_tests -- --nocapture`
  produced 1 pass and 1 expected failure: empty display name instead of username.
- `SQLX_OFFLINE=true cargo test --features server user_profile -- --test-threads=4`:
  10 passed. The new tests cover empty strings, ASCII/Unicode whitespace, ordinary
  and Unicode names, surrounding whitespace, punctuation and unchanged stored data.
- `SQLX_OFFLINE=true cargo test --features server pages::settings -- --test-threads=4`:
  13 passed.
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings`,
  `cargo clippy --target wasm32-unknown-unknown -- -D warnings`,
  `cargo fmt --check`, and `git diff --check`: passed.
- Second-pass self-review: PASS for this scoped change. No independent-agent
  review or fresh full repository suite is claimed.

Browser evidence on the existing local `:32768` deployment:

- A fresh mock Google subject `noms020-profile-fallback-20260927` created the
  disposable local account `@user-00b4`. Before the fix, its page reproduced an
  empty heading, `?` avatar and missing-name messages. The account was not given
  a display name to conceal the defect; it remains available for reproduction.
- Restarted only the app container after integration. Database/fixtures and the
  existing generated Tailwind changes were preserved. Browser cache was disabled
  temporarily for the rebuilt client, then restored.
- After reload, `/u/user-00b4` showed heading `user-00b4`, initials `UU`, unchanged
  handle `@user-00b4`, and both messages beginning `user-00b4 hasn't shared…`.
  A screenshot and the rendered accessibility/DOM state were reviewed in-session.
- Signed out the disposable account and reopened its public profile: the same
  identity and empty-state content remained visible without authentication.
- Opened `/u/user-c5a3` while signed out: existing name **Integration Review**,
  initials **IR**, handle and name-bearing empty-state messages were unchanged.
- No avatar upload was performed. Image alt text uses the same name passed to
  the existing Avatar component; live uploaded-image rendering was not retested.

The two original integration findings are now resolved. Collection-to-collection
stale content, the guest-search entry question and broader checkpoint 8 gates
remain open; this focused fix does not claim release acceptance.

## Drawer and sidebar artwork — 2026-09-27

Implementation commit `7117482` extends the existing shared brand artwork to the
navigation drawer and dashboard workspace sidebar. Developed in
`codex/NOMS-020-drawer-artwork`, then fast-forwarded into NOMS-020. The existing
generated Tailwind changes were preserved; only the existing app container was
restarted, keeping database fixtures and deployment port 32768 intact.

The v2 book marks the drawer heading, Workspace heading and All recipes link.
Matching approved identities mark Dashboard, Collections, Explore, Search,
New recipe, My profile and Settings in the drawer, and Overview, Drafts,
Collections, My profile and Settings in the workspace sidebar. Artwork uses a
32px well (book 32 × 40, page icons 32 × 32). Existing page-heading sizes,
labels, routes, collection disclosures, avatars and action controls are unchanged.

Validation:

- Tests-first CSS sizing regression failed before implementation and passed after.
- `SQLX_OFFLINE=true cargo test --features server --bin noms -- --test-threads=8`:
  783 passed, 0 failed, 12 ignored.
- Server and WASM `cargo clippy ... -- -D warnings`, `cargo fmt --all --check`,
  and `git diff --check`: passed.
- Second-pass self-review: PASS for the scoped change. No independent-agent
  review is claimed.
- Existing deployment browser review confirmed signed-out and signed-in drawer
  artwork, including the profile/settings images, loaded at the expected sizes.
  Escape dismissed the drawer and returned focus to its trigger.
- Signed-in dashboard retained its saved recipe and collection fixture, draft
  count and table actions; all seven sidebar artwork placements loaded correctly
  without horizontal sidebar overflow. Screenshots were reviewed in-session.
- At a measured 390px viewport, drawer page icons remained 32 × 32 and the book
  32 × 40, with no drawer/page horizontal overflow. Selecting Explore closed the
  drawer and navigated to `/explore`. Temporary viewport/cache overrides were
  restored and the test tab closed.

This navigation-artwork extension does not close broader checkpoint 8 gates or
the remaining collection stale-content/guest-search questions.

## About and How to artwork — 2026-09-28

Supplied selections/assets were preserved in `ec1aec6`; implementation `adb18e7`
adds About A3 and How-to H3 to `PageIconKind`, their existing page titles, and
navigation drawer destinations. Developed in `codex/NOMS-020-information-icons`
at `.worktrees/NOMS-020-information-icons`, then fast-forwarded into NOMS-020.
No changes to typography, palette, hero illustrations, title copy, routes or
navigation behavior. Existing generated Tailwind changes were preserved.

Validation used the integration worktree's shared Cargo target directory:

- Tests-first shared identity expectations failed because the new enum variants
  were not yet implemented. The How-to heading regression now verifies its
  approved asset, decorative alt treatment, existing text and single h1.
- `SQLX_OFFLINE=true cargo test --features server --bin noms -- --test-threads=8`:
  784 passed, 0 failed, 12 ignored.
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings` and
  `SQLX_OFFLINE=true cargo clippy --target wasm32-unknown-unknown -- -D warnings`:
  passed. `cargo fmt --all --check` and `git diff --check`: passed.
- Second-pass self-review: PASS; no independent-agent review claimed.
- Existing deployment `:32768` was reused, restarting only the app container.
  A stale-client hydration error during the rebuild resolved after a cache-disabled
  reload; the rebuilt drawer then opened and navigated normally.
- Desktop browser screenshots and DOM review confirmed both transparent WebPs
  loaded at 72px beside unchanged titles. About retained its labelled heading id;
  both pages retained one h1 and no horizontal overflow. Images have empty alt
  and aria-hidden, with labels supplied by surrounding text.
- Drawer About and How-to assets loaded at 32px, with the active About destination
  selected correctly. Selecting How to closed the drawer and loaded that page.
- At measured 390px width, both heading icons loaded at 56px and drawer icons
  remained 32px, with no horizontal page/drawer overflow. Selecting About from
  the compact drawer navigated and closed it, returning focus to the trigger.
  Viewport and cache overrides were restored; the temporary test tab was closed.

Broader checkpoint acceptance and existing unrelated findings remain open.

## Wider desktop composition — 2026-09-28

User requested more use of typical 16:9 desktop space. `171c7a6` introduces
`--content-max-width: 1760px` and makes the shared container/navbar and existing
page-specific outer caps consume it. This replaces the inconsistent 1060–1600px
caps across About, Home, How to, dashboard, search, settings, collection forms,
recipe editor and recipe detail. Inner text measures, component sizes, breakpoints,
mobile gutters, modal widths and sign-in panel sizing remain unchanged. New
outer page compositions should use the shared token rather than a fixed cap.

Developed in `codex/NOMS-020-wide-layout` / `.worktrees/NOMS-020-wide-layout` and
fast-forwarded into NOMS-020, preserving the existing generated Tailwind changes.
The existing local deployment was reused; only the app container restarted.

Validation:

- Before implementation, the new foundation contract failed with missing
  `--content-max-width`. Afterward,
  `rustc --edition=2021 --test src/components/design_foundation_tests.rs -o /tmp/noms-wide-layout-tests`
  followed by `/tmp/noms-wide-layout-tests`: 15 passed. The contract covers the
  shared cap, all route-owned outer overrides and preserved text/mobile measures.
- `SQLX_OFFLINE=true cargo test --features server --bin noms -- --test-threads=8`
  with the integration target directory: 785 passed, 0 failed, 12 ignored.
- `cargo fmt --all --check` and `git diff --check`: passed. No fresh Clippy run
  claimed for this CSS-only production change.
- Second-pass scoped self-review: PASS; no independent-agent review claimed.
- At measured 1920 × 1080, browser DOM review showed About widening from 1320
  to 1760px and Explore/navbar from 1600 to 1760px. Home, dashboard, search,
  settings and recipe detail also measured 1760px without page overflow.
  Dashboard sidebar remained 230px; Home lede retained its 470px text measure.
- At 1440 × 810, Home and recipe detail filled the 1425px available body width
  (15px vertical scrollbar), with no horizontal overflow. At 390px, Home, About,
  search and recipe detail filled 375px without horizontal overflow; About retained
  its 16px inner gutter. Desktop screenshots were reviewed in-session.
- The rebuilt client initially served cached route-specific recipe CSS; a
  temporary cache-disabled reload confirmed the current 1760px rule. Cache and
  viewport overrides were restored and the temporary tab closed. Existing local
  mock account/data were reused without editing recipes or account settings.

Broader checkpoint gates and unrelated existing findings remain open.

## Dedicated personal Home — 2026-09-28

Historical implementation evidence: the personal composition below was superseded
by the user's community-oriented clarification later the same day. See the
[current community Home contract and evidence](#community-home-rework--2026-09-28).

User approved an everyday Home distinct from the landing page and management
Dashboard. Implementation `c5384b5` and review corrections `821e158` were developed
in `codex/NOMS-020-personal-home` / `.worktrees/NOMS-020-personal-home`, based on
`42c0751`, then fast-forwarded into NOMS-020. The existing generated Tailwind
changes were preserved. Existing deployment `:32768` was reused, restarting only
the app container; no new deployment, schema changes or content edits were made.

Route/composition contract:

- `/` remains the public cozy v2 landing page. `/home` is authenticated **Your
  kitchen**; `/dashboard` retains its workspace sidebar and management library.
- Search submits to existing Search with `scope=recipes&vis=mine`; creation
  actions use existing editor/collection routes. Recent saved recipes use the
  owner-only search API with `UpdatedNewest`, page 1 and limit 6, sorting the
  full library before limiting rather than sorting a created-date page locally.
- Continue editing shows at most three private drafts with real revision/update
  metadata and editor links; empty titles use Untitled recipe. Collections show
  at most three top-level albums in their saved order, not falsely labelled recent.
- Resources have independent loading/error/retry states, clear empty guidance
  and existing shared cards/icons/materials. No quotes, made-up metrics, management
  tables or new bulk actions. AuthRequired contains private resource mounting;
  server middleware also protects `/home`.
- Signed-in navbar/book/drawer Home leads to `/home`; drawer Noms introduction
  and footer brand retain `/`. Guest navigation remains unchanged. Default
  sign-in returns to `/home`; explicit safe recipe/dashboard return destinations
  and redirect validation are preserved.

Validation at `821e158`, using the integration worktree's shared target cache:

- Tests-first default destination expectation failed (`/dashboard` vs `/home`)
  before implementation. Added owner-scope/sort/bound, empty composition, escaped
  identity, draft loading/error/untitled/limit, middleware and navigation contracts.
  Existing provider-render test was updated to the new default after a full run
  caught its old expectation.
- `SQLX_OFFLINE=true cargo test --features server --bin noms -- --test-threads=8 --skip runtime_grants_enforce_a_non_owning_data_plane_role`:
  **789 passed, 0 failed, 12 ignored, 1 filtered**. Two unfiltered attempts stalled
  in that pre-existing DB-grants test's external `psql` child; only those test
  children were terminated. Neither unfiltered run is a full-suite pass. This
  environment-dependent permissions gate remains open, not silently disabled.
- Server and WASM `cargo clippy ... -- -D warnings`, `cargo fmt --all --check`
  and `git diff --check`: passed. No independent-agent review was performed;
  second-pass self-review covered owner access, real sort/order semantics,
  resource states, escaped metadata, routes, shared styles and screenshots.
- Browser review found that a new route-local document title would persist into
  private routes lacking their own title. The addition was removed to preserve
  the existing private-page title convention. Footer brand's accessible name
  now identifies its actual landing destination as Noms introduction.

Browser evidence on the existing deployment:

- Reused existing mock Google `testuser` / `@user-c5a3` (Integration Review),
  containing four saved recipes and two collections. Sign-in provider links
  defaulted to `/home`, and mock sign-in landed there. No account/content data
  was edited. Initial resource loading settled into the real saved content;
  the account has no private drafts, so their empty guidance was verified.
- Home search for `soup` opened recipe results with Mine only selected and the
  existing private soup fixture visible. Navbar Home returned to `/home`.
  Open dashboard retained the management table, four saved recipes and zero
  drafts. Footer brand opened `/` with the unchanged marketing sections.
  Drawer Home and Noms introduction had distinct destinations; selecting the
  introduction closed the drawer, opened `/` and returned focus to its trigger.
- Anonymous client navigation showed the existing sign-in prompt rather than
  private Home content; the production anonymous-owner-route regression also
  includes `/home`. Explicit return preservation is covered by sign-in tests;
  a separate live explicit-return OAuth round trip was not repeated.
- At measured 1920 × 1080, Home used the shared 1760px cap with one h1 and no
  horizontal overflow. At 1440 × 810, it used 1425px available body width without
  overflow. At 390 × 844, search/actions and sections stacked in one column,
  with no horizontal overflow. Desktop and compact screenshots were reviewed
  in-session. Tab from the library search focused Search with a visible 2px
  outline. No bespoke tablet composition or new dark-mode design is claimed.
- Temporary viewport/cache overrides were restored and the test tab closed.
  Loading/error/untitled draft states are automated rendering coverage; live
  populated drafts, forced request failures/retry, exhaustive keyboard and
  measured contrast/dark acceptance remain unverified in this browser slice.

The new Home is implemented and available for user review, not a checkpoint 8
release sign-off. Existing collection-navigation, guest-search and broader gates
remain open. The two scoped implementation commits can be reverted in reverse
order; no database migration or fixture rollback is required.

## Community Home rework — 2026-09-28

The user clarified that Home is for community information, not personal recipes,
drafts or collections; Dashboard already owns those responsibilities. This
supersedes the preceding personal-Home contract. `/` remains the public landing
page, `/home` becomes Community Home, and `/dashboard` stays personal management.
Existing auth/default sign-in and explicit safe return rules are preserved.

Developed in `codex/NOMS-020-community-home` /
`.worktrees/NOMS-020-community-home`, based on `8a65f68`. Ordered code commits:

1. `786a7bd` — replace personal Home with the community composition and tests;
   rename the route component/stylesheet and put its footer link under Discover.
2. `e09dda2` — keep additional recipes directly beneath the spotlight instead
   of below the entire news sidebar, fixing a browser-observed blank gap.
3. `5440fd0` — use a native anchor for the forking-guide destination, preserving
   fragment focus/scroll rather than client navigation resetting to the top.

All were fast-forwarded into NOMS-020. The existing generated `assets/tailwind.css`
changes were preserved. Only the existing `:32768` app container was restarted;
no new deployment, backend/schema change, recipe edit or account edit occurred.

Acceptance contract and implementation:

- Shared Fredoka/Nunito, current palette, book/approved section artwork, raised
  panels, inset participation links and framed 16:9 recipe cards preserve NOMS's
  character. Desktop has a large spotlight and additional recipe grid beside
  announcements/participation. Narrow layouts stack.
- Reuse `get_public_recipes(0, 6, "")`, the existing Explore endpoint, which
  enforces public visibility and orders by creation time newest first. Feature
  the first card as the newest-public-recipe spotlight and show the remaining
  five at most, without duplicates. This is not curated or popularity-ranked.
  A defensive presentation filter also excludes private/unlisted cards.
- No private draft, owned-collection, owner-search or current-user reads remain
  in the Home component. Real recipe details and creator profiles are linked.
- Announcements initially contain one accurate, dated authored product notice
  about Home and Dashboard. This is static editorial content, not a live news
  service, admin/CMS feature or simulated activity feed. Get involved links to
  the existing Cooks search, recipe creation and forking guide; copy explicitly
  says creation starts privately and the user chooses public publication.
- Recipe loading/empty/error/retry does not remove news/participation. No
  invented reactions, comments, challenges, events, rankings or community counts.

Automated evidence at `5440fd0` (shared integration Cargo target cache):

- Before implementation, `home_is_a_community_hub_not_a_personal_workspace`
  failed with the old Your kitchen markup and missing Community Home heading.
  Replacement tests cover composition/destinations, public-only bounded order,
  real recipe/creator links, no duplicates, escaped metadata, loading/empty/error
  availability, retries hiding stale results and generic failure messages.
- `SQLX_OFFLINE=true cargo test --features server --bin noms community_home -- --nocapture`:
  7 passed, including sign-in default and middleware contracts (initial feature
  revision). Navigation tests retain the root versus `/home` distinction.
- Final `SQLX_OFFLINE=true timeout --kill-after=5s 90s cargo test --features server --bin noms -- --test-threads=8`:
  **792 passed, 0 failed, 12 ignored, 0 filtered**, finishing in 15.07s. The DB-grants
  test that stalled during the prior personal-Home run completed this time; no
  test was skipped or disabled for this run. Ignored integration gates remain
  unclaimed; this is not a full CI/security/deployment acceptance run.
- Server and WASM Clippy with `-D warnings`, `cargo fmt --all --check` and
  `git diff --check`: passed. Second-pass self-review: **PASS** after fixing the
  two should-fix findings (layout gap and guide anchor); no blocking findings
  remain in this slice. No independent-agent review claimed.

Live browser evidence:

- Reused existing mock Google `testuser` / `@user-c5a3` (Integration Review),
  whose Dashboard has four saved recipes, one private, and zero drafts. Community
  Home displayed only its three public recipes, with the newest in the spotlight
  and two others below, all linked once with author attribution. The existing
  private soup fixture did not appear. No fixture content was changed.
- Temporarily blocked only `get_public_recipes` requests in the test tab.
  Reload showed the generic recipe error/Retry while Announcements and Get
  involved remained usable. Unblocking and Retry restored the real cards.
- Meet the cooks opened `/search?scope=users`, with Cooks selected and two real
  public profiles; opening Integration Review showed its three public recipes.
  The native variation-guide link focused the `#fork` section and scrolled it
  into view (section top ~110px after smooth scrolling settled).
- Open Dashboard retained the four saved recipes, private soup, draft section,
  collections sidebar and management table. The footer brand still opened `/`
  with unchanged marketing sections. Creation destinations were checked in
  markup/tests; no live recipe was created or published in this pass.
- At measured 1920 × 1080, Home used the 1760px shared cap; at 1440 × 810 it
  used 1425px available width. Both had one h1, no horizontal overflow, and a
  28px gap between spotlight and additional recipes. Before the layout fix the
  1440px gap was ~377px, caused by the taller sidebar.
- At 900 × 1100, the main/sidebar remained two columns with a stacked spotlight
  image/text card; at 390 × 844 the page stacked in one column. Neither overflowed
  horizontally. Desktop/phone screenshots were reviewed in-session; wide in-app
  captures can clip the right edge, so DOM geometry supports the width evidence.
  Tab from Meet the cooks focused Share a recipe with a visible 2px outline.
- Request blocking, cache and viewport overrides were restored; the temporary
  tab was closed. Existing auth rules are exercised by the automated production
  owner-route tests and the observed guest redirect before local sign-in.

Remaining limits: empty/single-recipe feed states have deterministic rendering
coverage, not a separately emptied live database. No dark redesign, measured
contrast, exhaustive assistive-technology pass or final per-page responsive
sign-off is claimed. Editorial curation, announcements authoring and new community
engagement features need separate product contracts. Broader checkpoint 8 issues
remain open. Rollback is the three code commits in reverse order; no schema or
fixture rollback is required.

## Brand logo landing destination — 2026-09-28

User requested that the NOMS logo return to the landing page instead of Community
Home. `5daf9e4`, developed in `codex/NOMS-020-logo-landing` /
`.worktrees/NOMS-020-logo-landing` from `a638966`, makes the navbar book/wordmark
unconditionally link to `/`, with accessible name Noms introduction. The footer
brand already has that destination. Signed-in Home navigation remains `/home`;
guest navigation, artwork/styling and sign-in/return behavior are unchanged.
This supersedes the brand-destination statement in the preceding Home evidence.

The new SSR regression renders the actual shared navbar in both auth states and
checks its logo href/accessibility label. It failed before implementation due to
the old accessible label; live signed-in baseline also showed the old `/home`
logo href. `SQLX_OFFLINE=true cargo test --features server --bin noms components::navbar -- --nocapture`:
6 passed, including auth-specific Home destinations, responsive navigation and
logout-form contracts. Server and WASM Clippy with `-D warnings`,
`cargo fmt --all --check` and `git diff --check` passed, using the shared integration
target cache. No fresh full-suite or independent-agent review is claimed;
second-pass self-review **PASS**, with no blocking/should-fix findings.

Fast-forwarded into NOMS-020, preserving existing generated Tailwind edits;
only the existing app container was restarted. No backend/data changes or new
deployment. The code commit can be reverted independently.

Live browser verification on the rebuilt `:32768` deployment reused the existing
signed-in mock account. From Community Home, clicking the navbar logo opened `/`
and its landing heading/marketing sections; clicking Home then reopened `/home`
with Community Home and the public recipe feed. Guest logo behavior has SSR
coverage, not a separate logout round trip. Temporary cache bypass was restored
and the test tab closed.

## Checkpoint 8 close-out pass — 2026-09-29

**Status: scoped fixes integrated; overall release acceptance OPEN.**

Worked in `codex/NOMS-020-checkpoint-8` / `.worktrees/NOMS-020-checkpoint-8`,
based on `2a940e2`. Final tested code is `cf0696c`, fast-forwarded into
`codex/NOMS-020-visual-design-refinement`. The integration worktree's existing
uncommitted generated `assets/tailwind.css` was preserved, not included in these
commits. This is a local development review, not a production/release build or
authorization to deploy.

### Contract and ordered implementation

1. `3a6ab7a` — separate the collection route boundary from stateful contents so
   different IDs can own independent loaders, pagination, local overrides,
   pending tasks and confirmations. The initial lone-root key was insufficient;
   this commit alone is **not** the accepted navigation fix.
2. `453eb0e` — explicitly distinguish public Explore search from authenticated
   full Search in landing/sign-in copy. Guest landing input describes its access
   note; signed-in markup has no dangling description reference. Auth/search
   permissions, providers, explicit safe-return handling and query helpers are
   unchanged.
3. `2e7f5d0` — darken only light-theme neutral secondary/tertiary/placeholder
   text and brighten dark tertiary text. Keep Fredoka/Nunito, base/surface/gradient
   colours, coral/olive accents and shared neumorphic elevation unchanged.
4. `1ccc064` — give image-editor range and paired numeric inputs individual
   accessible names, expose display units, label Undo/Redo/Flip actions, and
   announce flip pressed state. No image transforms, callbacks or save semantics
   changed.
5. `cf0696c` — correct the collection boundary to a singleton keyed fragment.
   Dioxus 0.7.9 reconciles keys in fragments, not a lone root VNode. Strengthened
   tests exercise the real wrapper composition with a local stateful probe,
   without a network resource or shared application database.

### Automated evidence

Commands ran from the checkpoint worktree, using
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`
and `SQLX_OFFLINE=true` for server builds/tests.

- Guest landing and sign-in rendering regressions failed before the copy
  changes, then passed; they verify guest/account text and actual destinations.
- Neutral contrast test initially failed at 4.28:1 for light secondary copy on
  the surface. After the token adjustment, all 16 standalone foundation tests
  passed via `rustc --edition=2021 --test src/components/design_foundation_tests.rs`
  and the resulting test binary. Primary/secondary/tertiary/placeholder roles
  now test at least 4.5:1 against each theme's opaque base, surface and four
  gradient stops. This does not certify every composited/translucent consumer
  or every accent/control state as AA.
- Image-slider SSR and icon-state regressions failed before the naming changes;
  all six focused image-editor tests subsequently passed.
- Both the fragment-composition check and lifecycle regression failed against
  the initial lone-root key. The lifecycle failure rendered `<p>edited child
  state</p>` after switching to the parent. With the fragment correction,
  `cargo test --features server --bin noms pages::collection_detail::tests -- --nocapture`
  passed **16 tests**: same-ID rerenders preserve edited local state, whereas
  child → parent → child replaces it. This is real Dioxus reconciliation with
  the network contents substituted, not a populated browser API test.
- Final `just ci` at `cf0696c`: **exit 0; 799 passed, 0 failed, 12 ignored,
  0 filtered**, in 15.71s. Includes native/WASM checks and Clippy with
  `-D warnings`, formatting, six schema-plan policy tests, pinned declarative
  schema workflow checks, security policy tests, tracked-secret/deployment checks
  and the pinned dependency audit. Audit retained **eight policy-allowed
  warnings**; no new exceptions or dependency changes were made.
- Separately ran
  `cargo test --features server search_integration_ -- --ignored --test-threads=4`
  with `SEARCH_TEST_DATABASE_URL` pointing to the isolated CI database:
  **11 passed, 0 failed**, in 4.91s on final code. No search integration test was
  skipped in that run. The remaining ignored real-storage test was not run.
- `git diff --check` passed. No remote CI run or independent-agent review claimed.

Search integration used disposable `noms020-checkpoint8-search-ci` at local
port 32773, based on the existing pinned ParadeDB image, with its own database
and anonymous volume—not the application database. Host pgschema v1.9.0 failed
on the repository's BM25 schema and rolled its transaction back; the deployment's
pinned v1.12.1 binary applied the unchanged schema successfully. No schema files
or application database were modified for this check. The disposable test
container/volume was removed after validation; the app deployment remains running.

### Browser environment and observations

Restarted the local development deployment at `http://127.0.0.1:32768`, app
`noms-5063d4a16468023f-app-1`, mounting the integration worktree. The old
2026-09-27 fixture database was no longer available: this deployment has zero
recipes and zero collections. Local mock Google `testuser` resolves to
`@user-332d`. Routine sign-in/sign-out was used; no account/profile/avatar save,
link/unlink, recipe/collection creation, publication or deletion was performed.
Final read-only counts confirmed zero recipes, zero collections and zero saved
avatars. The app was restarted again after `cf0696c` and became healthy.

- Desktop 1440×810, phone 390×844 and tablet 820×1180 geometry smoke checks
  covered landing, Community Home, About, How-to, Dashboard, Explore, Search,
  collections/new, and settings. No horizontal document overflow was observed
  in those empty/basic compositions. These initial-render checks do not prove
  fully hydrated populated workflows. The public-profile matrix capture occurred
  before its resource settled and is not accepted identity-heading evidence.
- Existing dark mode was verified **after hydration** on eight desktop routes:
  landing, Community Home, Explore, Search, Dashboard, Collections and both
  settings pages. Correct dark class/tokens and no horizontal overflow observed;
  light mode restored. Earlier pre-hydration samples were discarded. This is
  dark regression smoke, not a dark redesign or comprehensive visual acceptance.
- Reduced-motion emulation matched and disabled background animation (0s);
  emulation restored. Navigation drawer opened as a named dialog, focused Close,
  moved Tab to Search, and Escape returned focus to its trigger.
- Account deletion confirmation was opened but never confirmed. Initial Cancel
  focus, Shift+Tab wrapping to Confirm, Escape dismissal and focus return worked.
  The sole linked Google provider could not be unlinked; Connect GitHub retained
  `purpose=link`. No provider action was executed.
- Avatar file selection opened the image editor without saving. All ten range
  and ten numeric controls were named in the accessibility snapshot. Keyboard
  ArrowRight changed Rotate 0→1 and its paired value, enabling Undo; Undo, Reset
  and horizontal flip pressed state were verified. Cancel removed the dialog and
  returned focus to Choose Photo. No OK/apply/avatar upload callback was invoked.
- Image-editor Cancel/OK stayed within the viewport at 1280×640, 820×1180 and
  390×844; controls used their internal scroll region and document width did not
  overflow. This is reachability, not a claim that every control fits on-screen
  simultaneously on a short/phone viewport. Desktop and phone captures were
  inspected; I1B landscape composition and coral controls were retained.
- Guest landing exposes the access note with `aria-describedby` and links to
  public Explore. The link opened Explore; a public search reached its real
  empty-results state. Full Search remained protected. Updated sign-in feature
  groups and the existing default provider return `/home` were visible.
- Temporary viewport/cache overrides were restored; the temporary browser tab
  was closed. Dev rebuild messages appeared in snapshots and are not evidence
  of a production defect. Screenshots/geometry JSON remain temporary review
  artifacts outside the repository, not permanent baseline assets.

### New open integration issues

1. **Encoded-query round trip (should fix before acceptance).** Guest landing
   input visibly contained `tomato & basil`, but client navigation settled at
   `/search?q=tomato`; reproduced with ordinary typing and again after the final
   restart. `silky noodles` correctly retained `%20`. Dioxus 0.7.9's generated
   `Routable::from_str` decodes the entire query before constructing
   `SearchRouteQuery`; search then splits parameters at `&` and canonicalizes,
   treating the formerly encoded ampersand as a delimiter. Existing URL-helper
   tests bypass this router boundary. Fix requires actual route round-trip
   regressions, including reserved characters/advanced criteria; do not claim
   the green search suite establishes this UI contract.
2. **Client auth-gate return context (should fix before acceptance).** Search's
   client-side gate renders a plain `/login` Sign In link, defaulting providers
   to `/home` instead of preserving the intended search destination. The direct
   server request path separately redirects to `/login?redirect_uri=…`; explicit
   safe-return handling remains intact. Update the client gate within existing
   validation/authorization rules, not by weakening Search access.

Neither issue's introduction revision was established. Neither was changed in
this pass. They are cross-route integration follow-ups, not new visual concepts.

### Review, remaining gates and rollback

Second-pass self-review examined the ordered diff, token consumers, auth
boundaries, image-control semantics and Dioxus reconciliation. It caught the
ineffective lone-root key and replaced it with the tested keyed fragment.
**Implemented scoped fixes: PASS. Overall checkpoint 8 acceptance: OPEN** due
to the two new should-fix issues and incomplete runtime acceptance. No
independent review or full screen-reader walkthrough is claimed.

Queue at the end of this initial pass (fixture approval and representative
populated checks are superseded by the follow-up below):

1. Resolve and regression-test encoded search routing and client auth-gate return.
2. Obtain approval to create private disposable recipes/collections in this local
   mock account. A collection submission was blocked by the tool's approval
   policy before creation; it was cancelled, not bypassed with SQL/API writes.
   The user was asked for fixture permission; no response received in this pass.
3. With approved fixtures, recheck real collection breadcrumbs/nesting and
   multiple memberships; recipe image caller/apply, draft/autosave/publish,
   historical snapshots and forked/dense graph/comparison flows; relevant
   request-failure/retry and owner/non-owner boundaries. Historical fixture
   evidence does not substitute for combined-revision validation.
4. Finish populated per-page phone/tablet acceptance, composited contrast,
   keyboard/status/screen-reader checks and whole-app user visual acceptance.
   Dark redesign, Dashboard functional expansion, favicon/single-colour/3D
   exports and mascots remain separate deferred deliverables.

Rollback baseline is **`2a940e2`**, preserving the integrated pages, Community
Home and logo-to-landing work. On a new review branch, revert these five code
commits in reverse order for a full scoped rollback, or revert a coherent concern
selectively; the collection pair must be handled together. Do not reset the
integration worktree or its generated CSS. No migrations, persisted content or
image uploads need rollback from this pass. Deployment remains separately scoped.

## Approved private-fixture validation follow-up — 2026-09-29

**Status: representative populated checks recorded; checkpoint 8 acceptance
remains OPEN.** The user approved creating private test recipes and collections
in the existing local deployment. Reused `http://127.0.0.1:32768` and mock Google
`testuser` / `@user-332d` at documentation revision `c129eaf`, whose application
code is `cf0696c`. No application code, schema, provider linkage or account
profile was changed, and the deployment was not restarted during this follow-up.
Creation, editing, collection membership and publication were performed through
the UI; read-only database checks confirmed the resulting fixture state.

### Retained private fixtures

| Fixture | ID | Persisted state |
| --- | --- | --- |
| NOMS020 CP8 parent 2026-09-29 | `7f535c8e-d0a1-4052-87a5-65b203479917` | Private root collection; one child and one direct recipe |
| NOMS020 CP8 child 2026-09-29 | `804d4edb-ae22-4949-ae1b-e57792c0b6d9` | Private collection under the parent; one direct recipe |
| NOMS020 CP8 validation soup 2026-09-29 | `490fa456-22d9-4f99-83f2-a7721184bcca` | Private recipe; published v1/v2; member of both collections |
| NOMS020 CP8 private fork 2026-09-29 | `017e8dfb-3bcb-4894-b3d6-81bf58fe546d` | Private published v1; forked from soup v2 |

Final read-only counts: **two recipes, both private; two collections, both
private; three versions; zero drafts; two memberships**. The child's parent ID
matched the parent fixture above. No public/unlisted visibility, destructive
confirmation, snapshot restore or fixture cleanup was performed. Fixtures remain
available for reproduction; removing them requires a separately scoped action.

### Workflow evidence

- **Collections:** nested New collection retained parent context; Cancel returned
  to the correct parent route and content. Child → parent breadcrumb → child
  card rendered the correct headings, descriptions and recipe data without stale
  child state. The child Edit form selected the correct parent. Add to Collection
  expanded the hierarchy, exposed the child's full parent/child accessible name,
  and saved both memberships with `Added to 2 collection(s)`. Both collection
  detail pages showed their direct recipe and image. Singleton reorder controls
  were disabled. This closes the representative populated revalidation of the
  keyed-fragment fix, not pagination/concurrent-loader acceptance.
- **Destructive safeguards, without deletion:** the parent confirmation named
  the target and explained recursive collection/membership deletion while keeping
  recipes. Blank and incorrect confirmation phrases kept Delete collection
  disabled. Cancel dismissed the dialog and returned focus to Delete. No valid
  confirmation phrase or final delete was submitted.
- **Recipe/image editor:** populated the six raised form sections with title,
  description, timings, servings, `2 cups water`, method and notes. Selected
  `assets/brand/page-icons/editor-v1.png` strictly as a test photo. After image
  loading, the existing Frame/Transform/Color controls appeared; keyboard crop
  adjustment and OK applied the image, returned focus to Choose recipe images
  and produced a Ready converted image. Client conversion reported about
  1.1 MB → 98 KB (91% smaller). This tests the real recipe caller, not just the
  standalone editor.
- **Genuine failure/retry:** the browser file chooser took about 40 minutes,
  exceeding the application's 15-minute session lifetime. Publication then
  failed with an unauthenticated image-upload error, retained the form/image and
  offered Retry. Reauthenticated the same mock account in a temporary tab;
  Retry reached Uploaded, then private publication succeeded. The initial
  general upload-error summary remained until the next publish attempt. This
  is session-expiry retry evidence, not an injected storage/network-failure test.
- **Autosave/publication/history:** v1 contained `Warm the water.`; editing it to
  `Warm the water, then let it cool briefly.` reached Saved and survived reload
  before publication. Publish Changes created private v2 with the persisted
  image and updated method. Read-only v1 retained its original instruction;
  comparison with current showed the exact change, while comparison with the
  original showed No content differences. No restore/discard action was taken.
- **Private fork/graph:** forked soup v2, preserved its source/image and added
  `Check the temperature before serving.` as an indented `1-a.` method step.
  Private publication produced a three-node family: soup v1 → soup v2 → fork v1.
  The SVG used solid version-history edges and a `10px, 7px` dashed fork edge.
  The accessible outline named all three snapshot links and both relationships;
  Tab from its summary focused the first snapshot link. Comparing soup v1/v2
  exposed `/instructions/0/text` with the exact before/after strings. Selecting
  the same fork snapshot twice showed Choose two different snapshots and disabled
  Compare; Clear reset selection. Zoom in, Pan right and Fit were exercised, but
  transform stability was not numerically asserted. Lineage-only checked state
  worked; all three nodes were ancestral, so removal of off-lineage nodes was
  not demonstrated. No dense graph or expansion frontier existed in this fixture.
- **Other consumers:** Dashboard listed both saved private recipes and no drafts;
  Search my recipes used recipes/mine and returned both cards. Settled public
  profile rendering used `user-332d` and its initials/fallback copy, with zero
  public recipes/collections. Explore did not expose the private fixtures.

### Geometry, privacy and review artifacts

The accepted populated matrix covered parent collection, child collection,
recipe detail, v1 snapshot and fork graph at **1440×810, 820×1180 and 390×844**:
15 route/viewport combinations, all with expected headings, no horizontal
document overflow and zero broken images among loaded images. Desktop/phone
captures were inspected; long titles wrapped on phone and graph cards/edges
retained their composition. The recipe editor was not included in this new
responsive matrix. An initial override targeted the temporary reauthentication
tab instead of the tested tab; those 1280×720 measurements were discarded and
the matrix rerun at the verified sizes. The earlier `fork-graph-desktop.png` is
also 1280-wide, not evidence for the 1440-wide matrix.

A populated dark fork graph retained all three nodes without horizontal overflow;
its screenshot was inspected and light mode restored. These checks do not close
all responsive interactions, dark states, screen-reader or composited-contrast
acceptance.

After sign-out, private recipe detail showed Recipe not found; both collections
showed Collection not found; private graph showed Version graph unavailable
with a generic no-access/not-found explanation. Private v1 denied content but
displayed an internal server-function error wrapper (see follow-up below).
No private fixture content appeared in these denials. An authenticated non-owner
account was not exercised; this is not a comprehensive authorization audit.

Temporary screenshots and raw matrices are outside the repository at
`/tmp/noms020-checkpoint8-fixtures-UDNCXD`: `populated-matrix.json`,
`privacy-matrix.json`, `graph-{1440,820,390}.png`,
`recipe-{1440,820,390}.png`, `graph-comparison-desktop.png` and
`fork-graph-dark.png`. These local artifacts may expire and are not permanent
baseline assets. Temporary tabs were closed, viewport overrides reset and the
session left signed out; user tabs were not changed. The app remained healthy.

### Findings and current queue

1. **Existing should-fix issues remain:** encoded-query round trips and the
   client-side sign-in gate's return destination, described in the initial pass.
2. **Error presentation should improve before acceptance:** signed-out private
   snapshots display `error running server function: Recipe not found (details:
   None)`. Expired-session image uploads similarly display internal Not
   authenticated wording. Denial/retry behavior worked; use appropriate user
   messages without exposing internal wrappers or weakening access checks.
3. **Early collection inputs need investigation:** the child creation form
   visibly held the filled name/description in the DOM after rapid navigation,
   but submission persisted Untitled Collection and an empty description. A
   settled Edit form saved the intended values correctly. The earlier audit had
   a similar observation, but cause and introduction revision are unestablished;
   source inspection found no obvious reset effect. Reproduce with actual form
   events and initialization before declaring a hydration regression or fix.
4. Complete dense/paginated data, graph expansion/concurrent/stale requests,
   remaining editor/image controls and authenticated non-owner/failure cases.
   Finish per-page responsive/accessibility acceptance and whole-app user review.
   Existing deferred product/artwork work remains separately tracked.

No fresh CI run is claimed: application source is unchanged from the initial
pass's `cf0696c` green local CI and separate 11-test search integration run.
This follow-up changes only the two planning records in one cohesive evidence
commit; `git diff --check` is the documentation gate. Second-pass self-review
checked evidence against the recorded limits: **validation record PASS; overall
checkpoint 8 acceptance OPEN**. No independent-agent review claimed. Code
rollback remains as documented above; reverting this documentation commit does
not remove the private fixtures or uploads.

## Search routing and sign-in return fixes — 2026-09-29

**Scoped fixes validated; overall checkpoint 8 acceptance OPEN.** Continued in
the existing checkpoint worktree from `5a2190b`. The ordered code commits are:

1. **`0ef66ba` — encoded Search query boundary.** A small application routing
   adapter keeps the single route inventory and delegates display, site map,
   path matching and layout rendering to Dioxus's derive. After matching Search,
   it replaces the decoded spread query with the original encoded substring,
   excluding the fragment. Search's existing parameter parser remains responsible
   for splitting parameters and decoding each value exactly once. Other routes,
   query canonicalization, validation and backend search behavior are unchanged.
   No dependency upgrade or vendored framework patch was introduced.
2. **`514fced` — client sign-in return context.** AuthRequired reads the router's
   subscribed raw current URL, uses existing same-origin/length normalization,
   and encodes the entire target into the native login link. Invalid/overlong
   targets and standalone rendering fall back to plain `/login`. Default login
   return `/home`, OAuth providers, server redirects and authorization remain
   unchanged. This applies to shared protected-page gates, not just Search.
3. **`a54b5dc` — native Search dropdown restoration.** Browser reload exposed a
   separate presentation mismatch: the advanced URL contained Exact phrase,
   and the select had `value="exact"`, but its actual native value was `all`.
   A select value attribute alone does not select an option in initial HTML.
   Explicit option selection now covers advanced match modes, each scope's sort,
   recipe visibility, total time and servings. Existing labels, values, event
   handling and styling are retained; no new search behavior was added.

### Regression and full-gate evidence

All commands used `SQLX_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`
from the checkpoint worktree.

- Three new actual-router regressions failed before the boundary fix: encoded
  text became decoded separators; advanced text containing a percent sign
  became malformed encoding; encoded duplicate parameter names were rewritten
  before validation. They now cover ampersand/plus/percent/equal/question/hash
  characters, literal encoding-like text, Unicode, advanced criteria with other
  parameters, invalid UTF-8/encoding and duplicate state. Additional tests retain
  nested collection query IDs, settings redirect, encoded path segments,
  trailing slash handling and NotFound routing.
- The routed AuthRequired regression failed with a plain `/login` href, then
  passed with the encoded complete target. Tests include query-bearing Search
  and collection creation, edit/dashboard paths, unsafe encoded path forms,
  overlong targets, unauthenticated child suppression and standalone fallback.
  A separate SSR test renders the **real application Router**, shared navbar,
  main/outlet and footer, and verifies the actual Search gate return href.
- Two dropdown SSR regressions failed before explicit option selection, then
  passed. They cover all three advanced match modes and non-default recipe,
  collection and user sorts, plus recipe visibility/range selections.
- `cargo test --features server --bin noms pages::search::tests -- --nocapture`
  on final code: **21 passed**. All eight AuthRequired tests also passed in the
  final full suite.
- Final **`just ci` at `a54b5dc`: exit 0; 808 passed, 0 failed, 12 ignored**,
  test execution 13.63s. Native/WASM checks, both Clippy gates with `-D warnings`,
  formatting, six schema-plan tests, pinned schema workflow, security policy,
  tracked-secret/deployment checks and pinned dependency audit passed. Audit
  retained **eight policy-allowed warnings**; no exceptions were added.
  An earlier sandboxed run passed build/lint/tests but could not write Cargo's
  advisory-cache lock; the complete gate was rerun with that cache access.
- `git diff --check` passed. No fresh ignored ParadeDB/real-storage integration
  run or remote CI is claimed; the earlier separate 11-test search integration
  evidence remains historical, not a run of this revision.

### Integrated browser evidence and limits

Fast-forwarded the three code commits into NOMS-020, preserving the existing
generated `assets/tailwind.css` edits. Restarted only the already-running app
container `noms-5063d4a16468023f-app-1` after the first pair and the dropdown
follow-up; no new deployment, backend restart, schema or fixture changes.

- Baseline browser submission reproduced `tomato & basil` → `q=tomato` and the
  plain client login link. After rebuilding, guest landing submission retained
  `/search?q=tomato%20%26%20basil`; the gate and both provider links retained that
  target. Mock Google `testuser` sign-in returned to the same URL, with the
  input displaying the complete text and the settled No results found state.
  GitHub's link was inspected, not used for another login/link action.
- Signed-in input `C++ & 100%` retained
  `/search?q=C%2B%2B%20%26%20100%25` through submission and direct reload.
  Recipes scope switching retained that text. Applied title criteria
  `100% & C++ #1` / Exact phrase; encoded JSON survived navigation/reload and
  Back/Forward restored the prior scope and advanced state.
- Final `a54b5dc` fresh-tab load showed Exact phrase selected and actual native
  value `exact`. Set Quickest, Mine only, 30–60 min and 3–4 servings; after reload
  the controls, filter chips and advanced mode all displayed the saved state.
  These searches had no matching fixture; populated result/pagination acceptance
  is not implied by a successful empty result.
- After sign-out on final code, navbar submission of `湯麵 🥣` retained its
  percent-encoded URL and complete client-gate return target. Full Search still
  required sign-in; the fix did not change guest access.
- A stale development tab initially retained the old client behavior after
  restarting. Those samples were discarded; a fresh tab exercised the rebuilt
  code. Temporary cache bypass was restored. Dev rebuild messages appeared in
  snapshots and are not treated as a production defect. An immediate scope
  click before hydration had no effect; after the page settled it worked. This
  does not resolve the separately recorded early collection-input observation.

Raw CI logs are temporary local artifacts in
`/tmp/noms020-search-return-cAz5MW/ci-final.log`; the earlier green pair-only
run is `ci.log`. The browser evidence is the inspected in-session state, not
new permanent design screenshots. Routine sign-in/sign-out did not change
profiles, provider linkage or private fixtures. No responsive/dark redesign,
full screen-reader audit or whole-app release acceptance is claimed.
All agent-created tabs were closed; the browser was left signed out with no
temporary cache/viewport overrides, and the existing app remained healthy.

Second-pass self-review checked the complete ordered diff, macro-generated
route mappings, URL/validation boundaries, SSR selection and browser evidence.
It identified the dropdown issue and required the third regression-backed fix.
**Scoped result PASS; overall checkpoint 8 OPEN.** No independent-agent review
claimed. Current queue is user-facing error presentation, early collection-input
investigation, then the remaining dense-data, failure/permission, responsive and
accessibility acceptance. Dashboard functional expansion and deferred artwork
deliverables remain separate.

Rollback of this follow-up is the three code commits in reverse order on a
review branch. No migrations, content or uploads require rollback. Do not reset
the integration worktree, generated CSS or retained fixtures.

## Error presentation and collection submission — 2026-09-29

Continuation base `d2efbea`; implementation in the existing isolated
`codex/NOMS-020-checkpoint-8` worktree, then fast-forwarded into
`codex/NOMS-020-visual-design-refinement`. The existing development deployment
at `http://127.0.0.1:32768` was reused and only its app container restarted.
Database, object storage and mock providers were not recreated. User-generated
`assets/tailwind.css` remained uncommitted and byte-identical across integration
and restart. Main was not modified.

### Contract and ordered changes

1. **`6187e21` — clarify historical snapshot and image upload failures.**
   Historical snapshot load failures render Recipe version unavailable and a
   generic no-access/not-found/load explanation, retaining Back to current recipe.
   No server-function wrapper or backend detail is rendered, regardless of the
   failure. Authentication, visibility, API and snapshot/comparison behavior are
   unchanged. This is a scoped load-error change, not global error normalization.
   The shared ImageUpload batch and retry paths map structured server errors to
   allowlisted sign-in, quota, rate, size, processing and storage copy; unknown
   errors get a generic retry message. Authentication copy asks for sign-in in
   another tab while keeping the editor open, since pending bytes are in memory.
   Failure still marks the image Failed, retains bytes/edits and leaves per-image
   Retry available; deferred-save success/failure behavior is unchanged.
2. **`3ba9b9a` — snapshot visible collection fields on submit.**
   Name/description now have native form names. The shared CollectionForm copies
   the submitted text into its signals before calling the existing parent save
   handler. Missing fields become empty, not stale defaults; existing required,
   length and whitespace validation remains authoritative. An already-submitting
   form does not dispatch again. Create/edit parents, visibility/parent selection,
   image upload orchestration and API contracts are unchanged. A test-only
   `dioxus-html/serialize` feature enables actual VirtualDom form-event dispatch;
   no dependency version or production dependency changed.

### Automated evidence

The initial error tests failed to compile before their new helpers existed;
the historical render harness then needed a parent router, as expected by Link.
With that test harness corrected, the historical unavailable-state SSR test and
all 30 ImageUpload tests passed, including error-copy and retained-byte checks.

The collection regression dispatched a real component submit event **without
preceding input events**, with `Typed collection` and `Typed description` in
the submitted fields. Before the fix, its callback read `Untitled Collection`
and an empty description. The empty/missing-field regression also dispatched
the default name. Both assertions failed on the original implementation; after
the fix all five form tests passed, including the already-submitting guard and
existing field validation. This proves the missed-input-event submission defect,
not the original browser observation's hydration timing.

Commands ran from the checkpoint worktree with `SQLX_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`:

```sh
cargo test --features server historical_version_error_tests -- --test-threads=8
cargo test --features server components::base::image_upload::tests -- --test-threads=8
cargo test --features server collection_form::tests -- --test-threads=8
just ci
```

Final `just ci` at `3ba9b9a`: **exit 0; 814 passed, 0 failed, 12 ignored**;
test execution 13.61s. Native/WASM checks and Clippy, formatting, six schema-plan
policy tests, pinned schema workflow, security policies, tracked-secret and
deployment-artifact checks and dependency audit passed. Eight existing audit
warnings remain policy-allowed; no new exceptions. The ignored ParadeDB and
real-storage integration tests were not rerun in this follow-up. Raw CI evidence
is a temporary local artifact: `/tmp/noms020-errors-forms-J5vWWj/ci.log`.

### Existing-deployment browser evidence

- Baseline signed-out private v1 reproduced the raw server-function wrapper.
  A normal collection creation with filled fields passed **before** the fix;
  the original rapid-input failure was not independently reproduced live.
- After restart, the first navigation raced startup and failed. Once healthy,
  a fresh tab still showed old nameless fields until a cache-disabled reload.
  Those stale samples were discarded; the rebuilt client's actual name field
  was verified as `collection_name` before accepting results. Dev rebuild banners
  appeared in DOM snapshots; they are not treated as a production defect.
- Fixed collection Edit rejected a whitespace-only name with Collection name is
  required. Filling name/description and pressing Enter from the name field saved
  successfully; reload retained the edited description, parent and Private state.
- Fixed New subcollection immediately filled/Enter-submitted a Unicode title and
  description. Its detail page and reload retained both, the expected parent and
  Private state. These are normal hydrated browser interactions, not a forced
  pre-hydration/autofill test.
- The owner still loaded private historical v1, its original method, photo and
  comparisons. After sign-out, the same route showed the new generic unavailable
  state and no private content; its desktop screenshot was inspected. Back to
  current recipe reached the existing generic Recipe not found denial. A missing
  nil-ID history route showed the same unavailable copy. No API/access relaxation.
- No new browser image upload/expired-session/retry run was performed: this
  revision's copy, byte retention and existing upload-state helpers have automated
  coverage, while the earlier real upload/retry remains historical evidence. The
  previously slow file-chooser operation was not repeated. Broader comparison,
  collection-save and other route errors are not certified by this scoped fix.

Two additional private UI-created fixtures remain for reproduction under the
existing approved local-test workflow; neither was deleted or made public:

| Fixture | ID | Current state |
| --- | --- | --- |
| NOMS020 CP8 form timing 2026-09-29 | `e59735c4-cddb-4121-aa66-4453f499d485` | Private child of the existing CP8 parent; description Private fixture: submitted details retained after reload. |
| NOMS020 CP8 submitted fields 湯麵 | `969ce48e-71a1-4f7d-94fb-2c6257813e22` | Private child of the same parent; description Private fixture: new collection values captured at submission. |

A scoped read-only database query confirmed both titles, descriptions, parent
`7f535c8e-d0a1-4052-87a5-65b203479917` and private visibility. Existing recipe,
version and membership fixtures were not edited. No profile/provider/access
changes were made. Temporary tabs closed, cache bypass restored and session
left signed out; user tabs and viewport/theme settings were not changed.
The app remained healthy at the existing port.

### Review, limitations and next gate

Second-pass self-review checked the complete two-commit diff, unchanged API and
deferred-upload boundaries, allowlisted messages, signal snapshot ordering,
test-only feature/lock change, full CI and accepted browser observations.
No blocking/should-fix finding remained in this scoped change. **Scoped result
PASS; overall checkpoint 8 OPEN.** No independent-agent review claimed.

The original early-input browser trigger/introduction revision remains
unconfirmed. Native submitted values now address the reproduced missed-event
case, but do not prove no pre-submit hydration overwrite or timing issue remains.
Follow that gate if reproduced, and finish dense/paginated data, graph
expansion/concurrent/stale requests, remaining editor/image interactions,
cross-route failures and authenticated non-owner cases. Full per-page responsive,
accessibility and whole-app acceptance remain open. Deferred dark redesign,
Dashboard functional expansion and artwork deliverables remain separate.

Rollback: revert `3ba9b9a` then `6187e21` on a review branch and revalidate; no
migrations or content cleanup are required. Preserve the integration CSS and
retained fixtures. Do not reset or clean the integration worktree.

## Version-graph request lifecycle — 2026-09-29

Continuation base `d515da9`; implementation in the existing isolated
`codex/NOMS-020-checkpoint-8` worktree and fast-forwarded to the NOMS-020
integration branch as `6eb11e0`. The existing development app at
`http://127.0.0.1:32768` was restarted after integration. No schema/API,
permission or fixture changes. The integration worktree's user-generated
`assets/tailwind.css` remained byte-identical and uncommitted.

### Acceptance contract and change

On graph route changes, an expansion or snapshot comparison requested for an
earlier visit must not update the current graph. Matching the recipe ID alone
is insufficient when navigation returns A → B → A before an old A response
arrives. A route-scoped monotonically advancing generation now identifies each
visit; expansion and both comparison completion paths discard mismatched
responses. A route change resets comparison loading with selection/result/error
state, so an old request cannot leave the new route's comparison controls
disabled. Expansion's existing cached-page merge, independent frontiers,
retry/continuation and collapse semantics remain unchanged. This is frontend
request-lifecycle hardening, not a change to the server's bounded graph/query
or ownership logic.

### Automated evidence

Tests were added first; the initial focused run failed to compile because the
request-scope and expansion-completion helpers did not yet exist. After
implementation, all **51 version-graph tests** passed. New tests cover the
A → B → A generation mismatch, a stale expansion response leaving the new
record untouched, a fresh response completing it, two independent frontiers
finishing in reverse order, continuation merging with a consumed boundary,
and a retryable error preserving the cached first page. A source-level guard
assertion supplements those state tests by checking comparison loading is reset
in the route effect.

From the checkpoint worktree with `SQLX_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`:

```sh
cargo test --features server pages::recipe_version_graph::tests -- --test-threads=8
just ci
```

Final `just ci` at `6eb11e0`: **exit 0; 818 passed, 0 failed, 12 ignored**,
14.05s test execution. Native/WASM checks, Clippy, formatting, schema-plan and
pinned schema-workflow tests, security policies, dependency audit, secret and
deployment-artifact checks passed. Eight existing audit warnings remained
policy-allowed; no exceptions added. The 12 ignored integration tests were not
rerun. Temporary raw log: `/tmp/noms020-graph-race-CCF8wR/ci.log`.

### Browser scope and remaining work

Before integration, signed-out private graph access showed the generic
unavailable state. After local mock sign-in, the owner graph loaded its two
published soup snapshots and one private fork; comparison v1 → v2 returned the
expected method change. Setting the fork as main recipe cleared comparison
selection/result on route change. The populated graph had three nodes and no
expansion frontier, so it cannot establish live expansion-page behavior. This
small-fixture limitation remains despite passing in-memory state/merge tests.

After integration and app restart, the owner graph again loaded the same
three-node topology. `Compare with current` on soup v1 produced the expected
v1 → v2 instructions change. In client-side navigation soup → fork → soup,
each destination displayed its correct heading and lineage counts, while the
comparison selection/result returned to the empty state and no pending
comparison spinner remained. The submit action was correctly disabled until
two snapshots were selected. Cache was disabled for the verification tab so the rebuilt
client bundle was used. No browser-level delayed-response interception was
completed; the A → B → A stale-response guarantee is established by the new
state tests, not by that normal-speed route smoke test. No expansion button
appeared because this graph has no frontier. Full dense/paginated graph,
all graph interactions and whole-app acceptance remain OPEN.

### Review and rollback

Self-review checked that both comparison request entry points use the same
route-generation guard, expansion completion checks the captured scope before
mutating records, and the route effect resets pending comparison state. No
independent-agent review was performed in this slice. To roll back this
frontend-only change, revert `6eb11e0` on a review branch, rerun the focused
graph tests and `just ci`, and smoke-test soup/fork route switching. No data
rollback or fixture deletion is required; preserve the integration worktree's
uncommitted CSS.

## Dense private graph fixture — 2026-09-29

The [insert-only local fixture script](../../scripts/dev-fixtures/noms-020-dense-version-graph.sql)
was run against the existing `noms-5063d4a16468023f` Docker development
instance after checking its `com.noms.dev-environment=true` label and the
private mock-account template. It requires that exact mock owner and template;
it fails rather than seeding a different environment. It was rerun once and
reported that the fixture already existed, with no rows inserted. The data is
retained for follow-up inspection; no existing recipes or test fixtures were
modified or deleted.

The new root is private recipe `aef72aa2-2c1c-4576-b253-14e0d531e634`,
owned by the local mock `testuser` / `@user-332d`. It has 32 immutable
versions, plus 30 private one-version forks attached to root version 32. The
script copies the prior private fixture's non-media recipe content but removes
images and varies only synthetic titles/descriptions/notes. Direct local SQL
seeding is test setup, **not** proof that the application publication workflow
can create this volume. Readback confirmed 32 root versions (1–32), 30 fork
relationships attached to v32, and 30 private child recipes. The root graph
is `/recipes/aef72aa2-2c1c-4576-b253-14e0d531e634/version-graph` on the
running local app.

### Browser observations

- Signed out, the private graph returned the generic unavailable state. After
  local mock sign-in, its initial page showed 25 snapshots (v8–v32), lineage
  counts of 30 forks/descendants, and two independent controls: older
  versions at v8 and child forks at v32. The displayed `+ 6` counts are the
  API's capped frontier-count hint, not the total hidden node count.
- The older branch returned five nodes (v7–v3), then its continuation returned
  v2 and v1. The child branch returned six successive five-node pages. The
  graph then contained all 62 distinct snapshots and no further expansion
  controls; the accessible outline could operate an offscreen older frontier.
- With both requests paused at the browser network boundary, the child-fork
  response was released first. The older request remained pending and its
  control disabled while 30 cards (25 initial + five forks) were present.
  Releasing older next produced 35 cards (v3–v32 plus five forks), preserving
  both independent batches and their continuation controls.
- A simulated network failure on older expansion showed an inline alert and
  left its control retryable. Clicking it again returned five nodes, cleared
  the error, and exposed the v3 continuation.
- With an A expansion paused, client-side navigation A → private fork B → A
  loaded each graph. After the old A response completed, the new A visit
  still had only its initial 25 cards and an unexpanded older frontier.
  A fresh click then returned 30 cards. This is direct browser evidence for
  discarding a response from an earlier visit to the same recipe.

Browser controls were restored after each interception; no request remains
paused. No production data, public recipes, schema or application code changed
in this fixture slice. The pre-existing `just ci` result at `6eb11e0` remains
the code validation baseline; no code was changed here and CI was not rerun.
The automatic fit makes a long chain's cards very small at desktop width;
pan/zoom and the outline remain functional, but dense-layout readability,
keyboard/focus and responsive acceptance still require a separate visual and
accessibility pass. The fixture script was reviewed for exact mock-only scope,
transactional insertion, idempotence and privacy; no independent agent review
was performed. To remove the fixture later, identify the exact root and child
IDs and use an approved cleanup plan; this task intentionally retains them.

## Dense graph readability and focus pass — 2026-09-29

The 25-card initial page previously fit its 12,848px-wide chain into a 1,361px
canvas, leaving cards about 28px wide. The `Current` control preserved that
unreadable zoom. On desktop, Tab could focus a card hundreds of pixels outside
the canvas. Resizing to tablet or phone retained the stale desktop transform.

The graph now opens on the current snapshot at 1× when fitting would fall
below 0.7×; smaller graphs still fit. `Fit` remains an intentional overview,
while toolbar/context-menu `Current` returns to at least 1×. Canvas resize
preserves the graph point at the viewport center and the user's zoom. When
keyboard focus reaches a snapshot's action button, link or static card, the
canvas brings that snapshot into view. The canvas uses `overflow: clip` so
browser focus navigation cannot silently scroll its transformed stage and
counteract that pan. These changes are limited to the graph; the private
fixture remains in place. Commits: `2289c07`, `e7e3f28`, `310aa02`.

Focused graph tests passed (55), including new dense/default, resize and
offscreen-focus geometry cases. The final `just ci` gate passed web/server
checks, Clippy, 822 tests (12 ignored), schema/security-policy scripts and
the dependency audit with the repository's existing allowed warnings. An
earlier audit attempt could not write its Cargo-cache lock under the sandbox;
the final full run used the necessary access. The updated client was checked with
browser cache disabled after rebuilding the running local app. Initial dense
cards rendered at 304px/1×; keyboard Tab through the action button and link
for versions 8, 9 and 10 kept each complete card in the canvas. At 820×1180
and 390×844, the focused version 10 remained 304px wide, centered inside the
canvas, with no document-level horizontal overflow. The 390px toolbar wrapped
into two rows without hiding Fit, Current or lineage controls.

This is a focused dense-graph pass, not whole-graph or whole-app acceptance.
Further interaction checks should still cover touch gestures, long sequences
of expansion plus focus, narrow viewports below 390px, reduced-motion/contrast
and the other paginated routes. No independent-agent review was performed.
Rollback is the three graph commits above; the local fixture is not removed
by rollback. Preserve the integration worktree's unrelated CSS edit.

## Graph interaction acceptance follow-up — 2026-09-29

**Scoped desktop/keyboard/narrow/motion checks: PASS. Touch-device and overall
checkpoint 8 acceptance: OPEN.** Work continued in
`codex/NOMS-020-checkpoint-8`, using the retained private 62-snapshot fixture
and the existing integration deployment at `http://127.0.0.1:32768`.

### Findings and fixes

- A native disabled loading button sent keyboard focus to the page body.
  Expansion controls now retain focus with guarded `aria-disabled` and
  `aria-busy` states. The continuation stays mounted at completion.
- Prepending older nodes still replaced the focused outline control. Keys now
  belong to the loop's outer fragment, preserving snapshot DOM identity.
  Nearby surviving snapshots anchor layout changes so newly discovered older
  columns do not shift the user's view. The layout effect subscribes to the
  completed resource before checking the non-subscribing `pending()` value.
- Older-history batches can expose a new frontier at another snapshot rather
  than continue from the same anchor. Their disabled completion label says
  “Batch loaded; more snapshots available”, not “All older versions loaded”.
- At 320px the old 304px card exceeded the canvas. Center/focus and resize now
  cap scale to the available canvas, including focus-ring margins. Requested
  readable zoom remains 1× where space permits; narrow screens use a smaller
  complete card instead of clipping it.
- The light-mode solid connector measured only 2.31:1. Both connector types,
  arrows and legend lines now use existing `--accent-ink`; solid/dashed styles
  continue to convey the different relationships.
- The expansion spinner inherited continuous rotation despite reduced motion.
  Its scoped override removes that animation while retaining loading text and
  busy status.
- Second-pass review identified that automatic layout anchoring must not
  counteract the explicit Fit performed by a lineage-mode toggle. A mode-aware
  guard and regression test preserve that existing navigation contract.

Ordered code commits: `3c42c0a`, `05e670f`, `44a9dae`, `c1c43b9`, `bdd38fc`,
`6cb6a6d`. These were fast-forwarded into
`codex/NOMS-020-visual-design-refinement`. No schema, public content, fixture
data, shared theme tokens or unrelated page changes were made. The existing
uncommitted integration `assets/tailwind.css` edit was preserved byte-for-byte.

### Browser evidence

- Fresh child-fork requests retain focus while pending and after completion.
  All six fork pages were loaded by keyboard; terminal completion retained
  focus and repeated activation made no additional change.
- The outline expanded v8's older history, then the new v3 frontier. Focus
  remained on each initiating control after prepending. Combined with all
  30 forks, the graph reached **62 cards**. Older-history collapse reduced
  this to **55** without removing the independent forks; cached reopen restored
  **62** and retained focus. At 1×, stage X moved by −2,600px then −1,040px
  to compensate for the added columns, preserving existing screen positions.
- A keyboard menu on the last expanded fork supports End and Escape; the menu
  closes and focus returns to the snapshot link. At **320×740**, the complete
  focused card measured **247×185.25px** inside a **273px** canvas; there was
  no document horizontal overflow. The menu stayed within the narrow viewport,
  with 44px menu items; toolbar controls wrapped and remained reachable.
- Rendered connectors now measure **6.29:1 light / 6.67:1 dark** against the
  opaque canvas. The light keyboard ring is a visible 3px darker-coral outline.
  Representative text/badge/Before/After samples passed 4.5:1 in both themes
  (lowest sampled text 4.83:1 light / 5.48:1 dark). This is sampled graph
  contrast, not a whole-app composited-contrast audit.
- Reduced-motion emulation showed no graph-card animation and a 0s stage
  transition. A paused fixture expansion verified the rendered 13px spinner's
  animation was `none`, focus remained on the busy control, and releasing the
  request produced 30 cards with the same focused control. Interception, cache,
  theme, motion and viewport overrides were restored; temporary tabs closed.
- The in-app browser does not support direct touch-event injection. A
  touch-to-mouse emulation attempt opened the action menu but its input command
  timed out; that is **not reliable touch acceptance evidence**. Long press,
  movement cancellation and physical-device scrolling remain open, alongside
  actual screen-reader testing. Static geometry and keyboard tests do not
  substitute for those checks.

Temporary screenshots (not tracked release assets):
`/tmp/noms-020-graph-acceptance-desktop.jpg` and
`/tmp/noms-020-graph-acceptance-phone.jpg`. These paths are local evidence only
and can disappear when temporary storage is cleared.

### Automated validation and review

Using `SQLX_OFFLINE=true` and the existing integration target directory:

```sh
cargo test --features server pages::recipe_version_graph::tests -- --test-threads=8
just check
just ci
```

Focused graph tests: **61 passed**. The added geometry regressions cover 320px
fit, narrow resize, prepend anchoring, unrelated layouts and explicit lineage
Fit. The reduced-motion regression failed before its CSS override, then
passed; loading/focus and terminal behavior also have the browser evidence
above. Final **`just ci` at `6cb6a6d`: exit 0; 828 passed, 0 failed, 12
ignored**, with formatting, native/WASM checks, Clippy, schema/security policy
scripts and the dependency audit. Eight pre-existing audit warnings remain
allowed. The local log is `/tmp/noms-020-graph-acceptance-ci.log`.
No fresh ignored real-storage/search integration run is claimed. A second-pass
self-review covered the ordered diff, route-scope guards, expansion visibility,
keys, focus, view-mode navigation and these evidence limits; no independent
agent review was performed. Rollback is the six scoped code commits above;
rollback does not remove the fixture.

Next: real-device touch and screen-reader follow-up, other routes' populated
pagination/dense states, remaining editor/image controls, cross-route failure
and permission checks, then whole-app acceptance. This pass does not authorize
a release or mark all of checkpoint 8 complete.

## 2026-09-29 — populated Search, collection and Dashboard pagination

Continued the approved acceptance work from `3e14b4d` in
`codex/NOMS-020-checkpoint-8`, reusing the existing isolated development
deployment at `http://127.0.0.1:32768`. No production data or public visibility
was changed. The integration worktree's user-owned `assets/tailwind.css` edit
was preserved byte-for-byte (SHA256
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`).

### Confirmed issues and fixes

- Search removed its pager while loading. Keyboard Next therefore lost focus
  to the body; native disabling at the last page also prevented stable focus.
  `94c4f59` moves the pager outside the loading/error/result branches and keeps
  its controls mounted, with `aria-disabled` and guarded handlers. Single-page
  results still hide the pager, with an explicit CSS rule that overrides its
  flex display. Previous cards remain hidden during loading.
- Collection Load more used native disabling during loading and disappeared
  after completion, also losing focus. `ad174a4` retains the control through
  loading, failure and completion, guards additional requests and shows
  “All recipes loaded” at the end. A failed request now presents readable retry
  copy instead of a server-function/network exception. Owner/reordering guards
  and page-size limits are unchanged.
- Search pagination measured about 40px high at 320px. `8b8afa2` applies the
  existing shared `touch-target` class to both buttons: the final browser
  measurement is **44px**, without a new visual token or palette.

### Private fixture and browser evidence

The existing mock owner `user-332d` has 33 private recipes, including 30 graph
forks. Those rows were sufficient for Search and Dashboard; no new recipes
were needed. After confirming `com.noms.dev-environment=true` on the existing
development Postgres container, the insert-only script
`scripts/dev-fixtures/noms-020-pagination-collection.sql` created one private
collection with the graph root and its 30 private forks:

`305e078e-e1ae-dd69-a6b1-a4b5baa943e6` —
`NOMS020 CP8 pagination collection 2026-09-29`.

The script checks the known owner/private recipe shape, detects conflicting
fixture IDs and verifies all memberships. `b8a1f46` corrects its repeat-run
name check for this database's two BM25 name aliases. A repeat run passed
with “already exists; no rows inserted”; 31 memberships were retained.
This is fixture setup, not evidence of a bulk collection-creation UI workflow.
No recipes, collections or memberships were deleted.

Live browser checks on the combined code:

- **Search / Recipes:** the query `NOMS020 CP8 graph fork`, sorted Title A–Z,
  returned 20 recipes on page 1 and 10 on page 2: 30 distinct titles, zero
  overlap. Query/sort/page URLs and Back to page 1 were checked.
- **Search delayed/failure states:** pausing the page-2 request retained focus
  on Next while showing skeletons, not old cards. A repeated actual click
  while paused produced only one request. At the terminal page Next remained
  focusable/inactive and activation did not change the page. A failed request
  retained the query/page and Next focus; Try again recovered the 10 results.
- **Search scope replacement:** switching to Collections while a recipe-page
  request was paused showed the new scope immediately. Releasing the old
  request did not restore recipe cards; the empty collection search's pager
  was hidden. This does not prove dense collection/cook Search pagination.
- **Collection pagination:** 12 → 24 → 31 distinct cards. Requests use a
  maximum batch of 12, with a seven-item final batch. While paused, focus
  stayed on Loading… and an additional actual click produced no second
  request. A forced failure retained the first 12 cards, readable feedback
  and Load more focus; retry cleared the error and continued correctly.
- **Collection completion:** focus stayed on All recipes loaded; further
  activation left 31 cards and no loading state. Recipe reordering stayed
  disabled until the complete list loaded, then the non-boundary movement
  controls enabled. No reorder/remove/delete action was performed.
- **Collection route isolation:** navigating to the collection index while
  Load more was paused and then releasing it did not insert cards into that
  route or produce console warnings/errors. Fresh entry/reload reset the
  detail view to the first 12 cards.
- **Dashboard:** its existing list view returned 12 + 12 + 9 distinct recipes
  with ranges 1–12, 13–24 and 25–33 of 33. Next retained focus on the final
  page; Previous returned to 13–24. Dashboard required no code changes.
- **320×740 geometry:** populated Search page 2 and the complete collection
  had no horizontal document overflow; both measured document width 305px
  inside the 320px viewport with its scrollbar. Search and collection paging
  controls measured 44px high and stayed within the container. This is
  geometry/keyboard evidence, not real-device touch acceptance.

Browser request interception and cache disabling were cleared; the temporary
viewport override was reset. The existing app remains healthy. Temporary
screenshots (not tracked release assets):

- `/tmp/noms-020-search-pagination-desktop.jpg`
- `/tmp/noms-020-search-pagination-phone.jpg`
- `/tmp/noms-020-collection-pagination-acceptance.jpg`
- `/tmp/noms-020-collection-pagination-phone.jpg`

### Automated validation, review and remaining gates

Using `SQLX_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`:

```sh
cargo test --features server pages:: -- --test-threads=8
just ci
```

The focused page run passed **202 tests**. Added Search SSR/source-boundary
regressions cover focusable controls and persistent pager placement; the
collection source regression and paging helper cover retained controls,
readable failures and the seven-item final batch. The added regressions failed
before the corresponding fixes. Runtime focus/guard behavior is supported by
the delayed-request browser checks above, not by source assertions alone.

Final **`just ci` on `8b8afa2`: exit 0; 831 passed, 0 failed, 12 ignored**,
including formatting, native/WASM checks, Clippy, schema/security policy gates
and dependency audit. Eight existing audit warnings remain allowed. Log:
`/tmp/noms-020-pagination-acceptance-ci.log`. No fresh ignored search or
real-storage integration run is claimed.

A second-pass self-review covered the ordered four commits, guarded
aria-disabled actions, stable pager placement, hidden-state CSS, private
fixture repeatability, route scope and validation limits. No independent-agent
review was performed. No blocking or should-fix finding remains in this slice.
Rollback is the four scoped commits above; it does not remove the private
fixture data.

Next: remaining editor/image controls and cross-route failure/permission
checks, then broader accessibility and whole-app review. Public Explore
pagination, dense collection/cook Search and other dense route states remain
open; this private fixture does not populate public pages. Physical graph
touch/long-press and screen-reader testing also remain open. This pass does
not close all of checkpoint 8 or authorize release.

## Editor history, avatar save and non-owner access — 2026-09-29

### Scope and fixes

Continued the remaining editing acceptance in the existing private local
deployment, `http://127.0.0.1:32768`, with mock OAuth and local storage services.
Implementation and tests used `codex/NOMS-020-checkpoint-8`; both scoped fixes
were fast-forwarded into `codex/NOMS-020-visual-design-refinement` and the
existing app was restarted. The integration worktree's unrelated generated
`assets/tailwind.css` was preserved byte-for-byte.

- **`4598435` — scope image history shortcuts to an open editor.** Closed
  image-editor components remain mounted and were intercepting document
  Ctrl/Cmd+Z/Y, preventing native recipe-text undo. The listener now leaves
  those keys alone unless that image editor is open; active image-history
  shortcuts retain their existing mapping. Two new unit tests cover closed
  and active shortcut mappings. Before the fix Ctrl+Z did not undo recipe
  notes with no image dialog open; afterwards text undo and redo worked.
- **`db9e0a0` — clarify unavailable recipe editor states.** Non-owner editing
  was correctly denied, but displayed a raw error and recipe identifier.
  Recipe and draft load errors now share a raised, generic alert with
  “Recipe editor unavailable” and a correctly labelled Back to Dashboard
  exit. A new SSR regression covers the copy, alert and destination. No
  authorization, API or visibility behavior was changed.

Tests were added before each production change, failed with the corresponding
helper absent, and passed after implementation. These regressions complement,
not replace, the browser checks below.

### Browser and persisted-state evidence

- **Recipe text history:** the owner opened the private soup recipe editor,
  appended temporary notes, then used Ctrl+Z and Ctrl+Y to undo/redo. Original
  notes were restored and the editor reported Saved. No recipe was published
  or draft discarded in this pass.
- **Avatar controls:** a dedicated local mock Google subject,
  `noms020-editor-reviewer` (`@user-62c8`), uploaded a repository test PNG.
  The ellipse/1:1 avatar editor exposed all ten paired adjustment controls.
  Crop, X/Y, rotation, width/height, saturation, three channel contrasts and
  both flips were exercised. Reset restored defaults; active Ctrl+Z/Ctrl+Y
  and the Undo button restored/reset the complete adjustment state.
- **Avatar apply/save/reopen:** OK completed processing/upload and showed
  “Avatar updated successfully!” with Edit Photo/Change Photo controls.
  Reopening, then reloading the page and reopening, restored crop 80%,
  displayed X 16px/Y −12px, rotation 25°, width 90%, height 85%, saturation
  70%, red/green/blue 120/80/110% and both flips pressed. Read-only database
  inspection confirmed persisted ellipse/1:1 edit metadata and transforms.
  Cancel returned focus to Edit Photo. Native Bio text undo also worked
  after closing the modal; the temporary unsaved text was cleared.
- **Authenticated non-owner access:** the dedicated account could not open
  the owner's private recipe editor or draft editor; both showed the new
  generic alert with no recipe content or editable form. Back to Dashboard
  worked and the new account had no private drafts/library entries.
  Recipe detail showed Recipe not found; v1 showed Recipe version unavailable;
  the graph showed Version graph unavailable; the private pagination
  collection showed Collection not found. No private recipe fields, graph
  nodes or collection cards were displayed.
- **Session cleanup:** the original local `testuser` owner was restored;
  owner editor access was rechecked. The mock reviewer and saved avatar are
  retained for reproducibility. No deletion, provider change, public sharing
  or modification to the owner's avatar/profile was performed.

The file-chooser automation took unusually long despite its requested timeout;
the expired mock session was refreshed before saving. Subsequent checks used
Edit Photo rather than another picker. The final rebuilt editor UI was verified
after reload and in a fresh tab; build success alone was not treated as proof.
Temporary browser cache overrides were cleared and created tabs closed.
Screenshots are local evidence, not tracked brand/release assets:

- `/tmp/noms-020-avatar-adjustments.jpg` — saved edit reopened after reload.
- `/tmp/noms-020-editor-unavailable.jpg` — final non-owner editor state.

### Validation and second-pass review

Using `SQLX_OFFLINE=true` and the integration worktree's `CARGO_TARGET_DIR`:

```sh
cargo test --features server components::base::image_editor_modal::crop_mode_tests
cargo test --features server pages::recipe_edit::
just check
just ci
```

Focused image-editor tests: **8 passed**. Focused recipe-editor tests:
**2 passed**. Native/WASM checks passed. Final **`just ci` on `db9e0a0`:
exit 0; 834 passed, 0 failed, 12 ignored**, including formatting, Clippy,
schema/security policy checks and dependency audit. Eight existing allowed
audit warnings remain. Raw log: `/tmp/noms-020-editor-acceptance-ci.log`.
No fresh ignored search or real-storage integration run is claimed.

An explicit second-pass self-review checked the ordered code diffs, shortcut
listener lifecycle, preserved active key mapping, generic error privacy,
unchanged authorization, tests and browser evidence. No independent-agent
review was performed. No blocking or should-fix issue remains in these two
scoped fixes; rollback is the two commits above and does not remove fixtures.

This does **not** close checkpoint 8: image-upload failure/retry in the avatar
caller, pointer crop/transform behavior, remaining editor failure states,
collection-edit and wider permission/failure presentation, public/dense
pagination and broader per-page accessibility still need acceptance. Native
text history was browser-tested with Ctrl keys; Cmd mappings have unit-test
coverage, not a macOS runtime check. Graph real-device touch/long-press and
actual screen-reader checks remain open. Dark-mode redesign remains deferred.

## Editor failure recovery fixes — 2026-09-30

### Scoped implementation

Continued in the existing `codex/NOMS-020-checkpoint-8` acceptance worktree,
based on `6d2b946`, and fast-forwarded the following four tested fixes into
`codex/NOMS-020-visual-design-refinement`. The existing mock/local deployment
at `http://127.0.0.1:32768` was rebuilt rather than replaced. Unrelated
generated `assets/tailwind.css` remained byte-identical (SHA-256
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`).

- **`2267b40` — recoverable collection editor errors.** A denied editor was
  browser-confirmed to show a server-function wrapper. Load failures now use
  a generic raised alert, Try again and Back to Collections. Detail-update,
  move, sharing-update and sharing-preview errors use readable copy rather
  than backend details. Partial save messages still distinguish details
  saved/move failed from details-and-location saved/sharing failed.
- **`e472420` — retained avatar retry.** A blocked upload was browser-confirmed
  to lose its selected image/edit and offer no retry. Confirmed source blobs
  and edits now survive modal close and failures in page-local state, with
  Retry photo upload. A synchronous claim blocks overlapping upload/removal
  requests. Pending retry stays focusable with `aria-disabled`; success clears
  retry data, and a newly confirmed image can replace a failed idle attempt.
  Error copy is allowlisted; no upload API or ownership rules changed.
- **`aa76056` — recipe autosave recovery feedback.** Autosave errors are
  separate from validation/publish errors and clear after a successful
  current-generation save. Draft-save failures retain entries with actionable
  feedback. Publish/discard feedback avoids backend details and accounts for
  an ambiguous lost response instead of claiming the operation could not
  have committed. Error regions now expose `role="alert"`. Existing revision
  guards, publication, visibility and discard behavior remain unchanged.
- **`496a5b5` — shared draft-save recovery.** Second-pass review found that a
  manual-save failure still used form validation state and could survive a
  successful background save. Manual and automatic draft saves now share their
  own error state; successful saving clears that state without clearing
  validation or publish/discard errors.

New regressions were added before their production helpers and failed to
compile while those helpers were absent, then passed after implementation.
Coverage includes collection alert/navigation, partial-save copy, exact avatar
source/edit retention, request guarding/replacement, safe feedback, and an
autosave source-boundary regression. Source assertions are not runtime proof;
the browser failure checks below establish recovery behavior. The manual-save
source-boundary assertion was strengthened before the fourth fix and failed
for the expected missing shared state, then passed in the final CI run.

### Browser acceptance

Used only the approved private local fixtures and existing mock accounts.
Request failures were injected before the target request reached the server.
No recipe was published, discarded, deleted or made public/unlisted.

- **Avatar:** as `noms020-editor-reviewer` (`@user-62c8`), Edit Photo reopened
  the existing test avatar without another file picker. A failed upload showed
  readable feedback and Retry photo upload while retaining its blob and full
  edit. Retrying did not require reselection or re-editing. While the retry
  request was paused, focus remained on Uploading photo… with `aria-disabled`;
  another actual click produced zero additional requests. Releasing the
  request produced Avatar updated successfully! and removed the retry state.
  Database inspection retained rotation 25°, scales .9/.85 and both flips.
  This tests the already-confirmed edit path; a fresh file selection and
  deliberately invalid image processing were not repeated.
- **Pointer crop:** dragging the avatar crop changed displayed X/Y from
  16/−12px to 76/33px (source-image coordinates). Undo restored 16/−12px;
  Cancel closed the modal and returned focus to Edit Photo. No pointer edits
  were saved. Rectangular resize handles, touch and pixel-level output
  correctness are separate checks.
- **Collection permission:** non-owner editor access showed the generic alert
  and no private form/data. Try again remained denied; Back to Collections
  reached that account's empty index. The original `testuser` owner was then
  restored for the remaining checks and retained as the final session.
- **Collection load retry:** a forced owner load failure reached the same
  generic alert. Try again restored the correctly populated form. Initial
  document-reload interception produced stale IDs; fresh-tab/client-side
  Edit navigation provided the verified failure/retry evidence.
- **Collection save:** a temporary description survived a failed update with
  readable feedback. Restoring the original description and retrying saved
  successfully and returned to detail. A separately blocked move reported
  that details had saved while the move failed, and reset Location to top
  level. Read-only database inspection confirmed the original description,
  private visibility, no parent and all 31 memberships. Sharing-update and
  preview failure copy changed in code but was not freshly exercised live;
  no sharing mutation was performed.
- **Recipe autosave:** both save attempts failed after temporary notes were
  entered. The notes remained on the page with the new autosave alert. A
  subsequent edit autosaved successfully and cleared that alert, rather than
  leaving an error beside Saved.
- **Manual draft save:** a forced failure retained local notes and showed
  “Could not save the draft…” feedback. Retrying after restoring the original
  notes reached Saved with zero alerts. A final check failed one manual save,
  then allowed background saving: the temporary notes survived, Saved appeared
  and the prior error cleared without another manual click. After restoring
  original notes, read-only database inspection of draft revision 10 contained
  exactly `Validation fixture only; keep private.` No publication/discard
  request was performed; their new copy has automated, not fresh live, evidence.

Temporary interception and cache overrides were cleared and all created tabs
closed. Screenshots (local, not tracked release assets):

- `/tmp/noms-020-avatar-upload-retry.jpg` — retained retry control and image.
- `/tmp/noms-020-collection-editor-unavailable.jpg` — generic non-owner state.

### Validation, review and remaining gates

With `SQLX_OFFLINE=true` and the integration worktree's target directory:

```sh
cargo test --features server pages:: -- --test-threads=8
just ci
```

The focused page tests passed **209 tests** before the final recovery follow-up.
Final `just ci` on the code committed as `496a5b5`:
**exit 0; 840 passed, 0 failed, 12 ignored**. Native/WASM compilation, Clippy,
formatting, schema/security policy gates and dependency audit passed. The
first CI attempt caught a collapsible condition, which was corrected. An
interrupted attempt and a later read-only advisory-cache lock failure were
not counted as passes; the final run had cache access and completed. Eight
existing audit warnings remain allowed. Log:
`/tmp/noms-020-recovery-ci-final.log`. No fresh ignored search or real-storage
integration run is claimed.

Explicit second-pass self-review: **PASS for this scoped series**. Checked the
ordered diffs, blob ownership and modal cleanup, synchronous retry guard,
partial-write copy, separate draft-save/validation feedback, revision guards,
tests and observed limits. The stale manual-save error finding was fixed and
browser-rechecked before this verdict. No independent-agent review was
performed; no blocking or should-fix finding remains in these four changes.
Rollback is the four commits above; retained fixtures are not removed by rollback.

Checkpoint 8 remains open for wider cross-route failure/permission and
accessibility acceptance, public/dense pagination, rectangular crop resizing,
device/touch interaction, actual screen-reader testing and whole-app sign-off.
Pending avatar retry data is deliberately page-local, not persisted across
reload/navigation. Dark-mode redesign remains deferred.

## Sharing, draft action failures and rectangular crop acceptance — 2026-09-30

### Scope and implementation

Continued in the existing isolated `codex/NOMS-020-checkpoint-8` worktree from
`75b9967`. Used the existing mock/local deployment at
`http://127.0.0.1:32768`, not a new deployment. The owner session had expired;
normal mock Google sign-in restored the existing `testuser` account. No new
account or fixture was created.

Acceptance criteria: readable sharing-preview/update and publish/discard
failure feedback; retained local entries; usable recovery controls; no new
competing draft action while a write is pending; rectangular crop mouse
resizing with bounds and history; private fixture restoration.

Browser testing found **Publish Changes remained enabled during a pending
discard**. Fix **`6a434ee` — prevent competing recipe draft actions during
discard** introduces one shared write-in-progress predicate for publication,
manual draft saving and discard. Action handlers recheck it synchronously,
controls disable consistently, and both initial and delayed autosave guards
observe discard. Manual save/cancel/discard also respect pending step-image
uploads. This does not change server authorization, publication semantics,
discard semantics or revision checks, and does not add cancellation of requests
already in flight before a user action.

Two regressions were added first: all eight pending-write flag combinations
and source-boundary checks for the action/autosave guards and publication
control. The first test run failed to compile because the new helper did not
exist. Tests passed after implementation. Source assertions supplement, not
replace, the browser checks below.

### Browser acceptance and fixture safety

Sharing-update, publication and discard failures were intercepted **before
the target request reached the server**. Normal private detail/draft saves
were allowed. No visibility change, publication or deletion committed.

- **Sharing preview:** selecting Unlisted locally and failing its preview
  request displayed “Could not preview sharing…” with retry guidance. Selecting
  Private and Unlisted again recovered the preview, cleared the alert and
  correctly reported that all 31 private recipes would be hidden from visitors.
  This selection alone did not save sharing settings.
- **Sharing update:** unchanged collection details saved, then the intercepted
  sharing update failed. The alert accurately distinguished saved details and
  location from sharing that could not be updated. Save changes became usable
  again. Returning the selection to Private and saving returned to detail.
  This checks pre-server failure, not an ambiguous response lost after a
  sharing change committed.
- **Publish failure/retry:** pre-publication draft refresh/save completed,
  but each publication request was blocked. The editor retained original
  notes and displayed the cautious “Could not confirm publication…” copy.
  Another publication attempt was possible after failure and was also blocked.
  On the rebuilt code, pending publication disabled discard/cancel and another
  actual click on Publishing… produced zero additional publication requests.
- **Discard failure/retry:** blocked discard retained the draft and entries,
  displayed “Could not confirm draft removal…” and restored its control. A
  second blocked attempt was possible. Before the fix, publication was still
  enabled while this request was paused; after rebuild it was disabled along
  with Cancel and manual saving. Additional actual clicks on publish, discard
  and cancel plus a temporary notes edit produced **zero additional write
  requests during the 3-second pending interval**, beyond the 2-second autosave
  debounce. Failing the paused request released the guard; restoring original
  notes and manual saving reached Saved with zero alerts.
- **Rectangular crop:** staged repository `assets/brand/page-icons/editor-v1.png`
  in the recipe image picker, without uploading it. On the 16:9 rectangle,
  each edge separately changed the crop from 80% to approximately 72%. The
  rectangle retained its aspect ratio. Undo restored 80% after each gesture.
  Right-edge drags beyond the wrapper clamped to 100% and 10%; Undo/Redo
  restored those states in one history step. A 72% crop screenshot was visually
  inspected. Cancel restored focus to Choose recipe images; the staged image
  remained Ready with its original/default edit, so it was removed from the
  local queue without uploading. The pre-existing saved image was untouched.
  This proves mouse preview/history behavior, not real touch resizing or
  pixel-level exported-image correctness.

After reopening the final rebuilt recipe editor, original notes and its one
saved image remained. Read-only database inspection confirmed:

- Recipe `490fa456-22d9-4f99-83f2-a7721184bcca`: private, latest published
  version still **2**.
- Draft `db986e55-2df0-43c4-ba7e-5d272b4d4fee`: retained, revision **17**,
  exactly `Validation fixture only; keep private.`, one image.
- Collection `305e078e-e1ae-dd69-a6b1-a4b5baa943e6`: private, original name and
  description, no parent, all **31 memberships**.

Interception/cache overrides were cleared and the two created tabs closed.
Normal viewport dimensions were used, without a sizing override. Screenshots
are local temporary evidence, not tracked release assets:

- `/tmp/noms-020-pending-discard-guard.jpg` — disabled publication during discard.
- `/tmp/noms-020-rectangular-crop-resize.jpg` — 72% rectangular crop preview.

### Validation and review

With `SQLX_OFFLINE=true` and the integration worktree target directory:

```sh
cargo test --features server pages::recipe_edit -- --test-threads=8
just ci
```

Focused editor tests: **13 passed, 0 failed**. Final full CI on the code committed
as `6a434ee`: **exit 0; 842 passed, 0 failed, 12 ignored**, with native/WASM
checks, Clippy, formatting, schema/security policy and dependency audit gates.
Eight existing audit warnings remain allowed. Raw log:
`/tmp/noms-020-sharing-discard-ci.log`. Ignored search/storage integration
checks were not freshly run.

Explicit second-pass self-review: **PASS for this scoped fix and acceptance**.
Reviewed synchronous claims, disabled controls, delayed autosave checks,
unchanged revision/authorization boundaries, tests and raw browser/database
evidence. No independent-agent review was performed. No blocking or should-fix
finding remains in the changed code. One behavior commit is appropriate for
the single shared action-exclusion concern; acceptance documentation is a
separate commit. The fix was fast-forwarded into the design integration branch;
unrelated generated CSS retained SHA-256
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

Checkpoint 8 remains open for public/dense pagination, remaining cross-route
permission/failure states, per-page phone/tablet and accessibility acceptance,
real-device touch and screen-reader checks, and final whole-app review.
Dark-mode redesign remains deferred. Rollback of this behavior slice is
`6a434ee`; it does not remove the retained fixtures.

## Public discovery and dense Search acceptance — 2026-09-30

### Scope and reproducible data

Continued the established isolated acceptance worktree
`codex/NOMS-020-checkpoint-8`, starting from `83b2859`, against the existing
local deployment `noms-5063d4a16468023f` at `http://127.0.0.1:32768`.
The user explicitly approved additional test data after the lack of public
recipes and dense collection/cook results blocked these checks. No production
environment or external account was involved.

Fixture commit **`391c65a`** adds
`scripts/dev-fixtures/noms-020-discovery-catalog.sql`. Before seeding, verified
`com.noms.dev-environment=true` on the exact Postgres container. The transactional
script inserts 25 synthetic public recipes with published v1 snapshots, 25
synthetic cook profiles (no OAuth identities), and 25 private collections owned
by the existing mock owner `da1ff4ae-c7b8-4ab5-96cb-ea982ddcad1f`. Names/descriptions
clearly mark the data as local/synthetic; addresses use `example.invalid`.
IDs are deterministic UUIDs from `md5('NOMS020 CP8 discovery catalog 2026-09-30'
|| ' cook/recipe/collection/version ' || ordinal)` with the respective entity
word substituted and ordinals 1–25. Collision and count checks abort unsafe
runs. Nothing is updated or deleted; fixtures remain for reproduction.
The current `CP8Catalog` recipe tag is added after the immutable v1 snapshot.

Reproduce only against the verified local test container:

```sh
docker inspect --format '{{ index .Config.Labels "com.noms.dev-environment" }}' noms-5063d4a16468023f-postgres-1
docker exec -i noms-5063d4a16468023f-postgres-1 psql -U noms -d noms < scripts/dev-fixtures/noms-020-discovery-catalog.sql
```

The first attempted seed rolled back on a PL/pgSQL ambiguous variable in an
`ON CONFLICT` target. Renamed it to `fixture_recipe_id` and corrected the
instruction JSON key to `sub_steps` before the successful seed. A successful
rerun confirmed idempotence: totals remained **25 public / 33 private recipes,
30 private collections, 27 profiles**. The prior 33 recipes and five collections
were not made public or otherwise changed.

### Browser evidence and fix

- **Explore:** initial 12 cards, retry to 24, final 25 unique recipe links.
  The original pagination button dropped focus when disabled, when replaced
  after failure, and when removed at completion. Added tests first (expected
  compilation failure for missing new helpers), then implemented **`0931dca`**:
  one stable, focusable control with guarded `aria-disabled` states. A generic
  error alert remains; initial empty-load errors retain their separate Retry.
  After fast-forwarding/rebuilding the existing deployment, actual Enter
  activation retained button focus through Loading recipes…, Retry, Load more
  and All recipes loaded. A failed second-page request retained all 12 cards;
  retry requested the failed offset once and reached 24. A forced extra click
  while pending and at completion each produced zero additional requests.
  No skipped or duplicate recipe IDs across the final 25 cards.
- **Explore query replacement:** from the completed catalog, paused `Recipe 01`,
  changed to `Recipe 02`, released the newer response before the older one.
  Pending cards cleared; the settled page contained only Recipe 02, with the
  input still Recipe 02. Prior page offsets/results did not leak into the query.
- **Collection Search:** `scope=collections&q=CP8Catalog&sort=name_ascending`
  returned **20 + 5 = 25 unique collection links**, names 01–25. Failed Next
  showed readable generic feedback, retained filters and Next focus, and Retry
  recovered the same page. Rapid additional clicks issued no duplicate request;
  Previous restored 20 cards. Pending stale cards were not shown.
- **Cook Search:** `scope=users&q=CP8Catalog&sort=username_ascending`
  returned **20 + 5 = 25 unique profile links**. Failed Next and successful
  retry retained paging focus; pending duplicate and completed-page clicks
  made no extra requests. Actual Enter activation on ready Previous/Next
  passed. An attempted locator keypress on `aria-disabled` Next was blocked by
  browser actionability; forced clicks tested its handler guard separately.
- **Scope/query/history:** switching from page 2 Cooks to Collections reset
  page and incompatible sort, showing 20 collections and no cook cards.
  A paused collection Next released after a newer Cooks response could not
  overwrite the cook scope. From cook page 2, changing/submitting `cp8catalog-01`
  reset pagination and returned one profile. Browser Back restored the broad
  query's first page (20 cards); Forward restored the one-profile query.
- **Visibility:** signed-out Explore loaded all 25 public synthetic recipes;
  the original private recipe was absent. The existing non-owner reviewer
  `user-62c8` searched Collections for CP8Catalog and received **0 found**,
  with no private collection links. Restored the mock owner session afterward.
- **Narrow geometry:** explicitly confirmed `innerWidth=320` for completed
  Explore and collection/cook page 2. Document width was 305px, with no
  horizontal overflow; all paging buttons were 44px high and within the page.
  The browser-level viewport override did not change observed width, so these
  checks used per-tab device-metrics emulation, cleared afterward. This is
  paging geometry only, not full page/mobile or real-touch acceptance.

All injected failures occurred before server handling, via local browser
request interception; all interception/emulation overrides were cleared.
Screenshots were visually inspected; temporary evidence, not release assets:

- `/tmp/noms-020-explore-pagination-complete.jpg`
- `/tmp/noms-020-explore-pagination-320.jpg`
- `/tmp/noms-020-collection-search-pagination-320.jpg`
- `/tmp/noms-020-cook-search-pagination-320.jpg`

The in-app browser exported the emulated narrow views as scaled 305×172
previews rather than a full-height device capture. Their geometry claims rely
on the measured live DOM, not screenshot pixel dimensions; the normal desktop
completion capture is 1265×712. Initial empty-load Retry focus and exhaustive
screen-reader announcements were not newly checked in this slice.

Read-only post-check inspection confirmed the original recipe remains private
at published v2; its draft remains revision 17 with exactly
`Validation fixture only; keep private.` and one image. The original pagination
collection remains private, top-level, with its original name/description and
all 31 memberships. No existing profile was edited.

### Validation, review and remaining work

With `SQLX_OFFLINE=true` and the integration worktree's existing target cache:

```sh
cargo test --features server pages::explore -- --test-threads=8
just ci
```

Focused Explore tests: **6 passed, 0 failed**. Full CI for code committed as
`0931dca`: **exit 0; 844 passed, 0 failed, 12 ignored**, including native/WASM,
Clippy, formatting, schema/security policy and audit gates. Eight existing
allowed audit warnings remain. Logs: `/tmp/noms-020-discovery-focus-tests.log`
and `/tmp/noms-020-discovery-ci.log`. No fresh ignored search/real-storage
integration run is claimed; the live tests used the running ParadeDB database.

Explicit second-pass self-review: **PASS for this scope**, covering stable
render identity, synchronous request claims, action guards, retry offsets,
query-generation protection, fixture visibility/idempotence and unchanged
private records. No blocking/should-fix finding remains. No independent-agent
review was performed. Fixture and behavior commits are separate concerns;
acceptance documentation is a third commit. Both code commits were fast-forwarded
into `codex/NOMS-020-visual-design-refinement`. Unrelated generated CSS retained
SHA-256 `63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

Public Explore and dense collection/cook pagination are no longer blocked or
open checklist items. Checkpoint 8 remains open for broader cross-route
failure/permission checks, other dense states, remaining editor/image
interactions, per-page populated phone/tablet and accessibility acceptance,
real-device touch, screen-reader and pixel-level export checks, and final
whole-app review. Dark-mode redesign remains deferred. Rollback of the behavior
slice is `0931dca`; reverting code does not delete the retained local fixtures.

## Collections and Dashboard load recovery — 2026-09-30

### Scope and acceptance criteria

Continued the existing isolated `codex/NOMS-020-checkpoint-8` worktree from
`cb50837`; reused the established local deployment on port 32768, with fresh
browser navigation, injected pre-server read failures and the retained test
fixtures. Acceptance for this slice: readable failure copy without transport
details; visible pending retry announcements; retained retry focus through
pending/repeated failure; no competing retry requests; independent Dashboard
sections; preserved collection filter; usable guest/non-owner boundaries and
320px retry geometry. No production effects, content writes or new fixtures.

### Reproductions and implementation

- Collections failed-load alert exposed
  `error reaching server to call server function: error sending request:
  TypeError: Failed to fetch`. This was reproduced by blocking
  `list_root_collections` during SPA navigation.
- Dashboard drafts and collection tree could fail while all 12 current recipe
  rows remained usable. Repeated Retry drafts clicks while its request was
  paused produced **one original plus one extra request** before the fix.
  Its button did retain focus in that baseline test; lost focus is not claimed
  as the observed bug in this flow.
- Source review found `Resource::pending()` peeks without subscribing. Retry
  controls lacked live pending guards; a non-embedded CollectionSidebar returned
  nothing on load failure. Only the embedded sidebar currently has a production
  caller, so standalone disappearance is source evidence, not a live route test.

Behavior commit **`fb884ad`** adds a shared `LoadRecovery` component and integrates
it into Collections, Dashboard drafts/library and CollectionSidebar. It renders
one focusable 44px retry button with `aria-disabled`, safe literal messages,
an alert when failed and a status/Retrying… label when pending. Callers subscribe
to `resource.state().read()` and check the live `pending()` value before restart,
so a second click before rendering cannot start a competing request. Recovery
remains mounted while the resource retains its previous error. Successful data
replaces the recovery panel; no post-success focus-transfer implementation is
claimed. Previous page in a failed Dashboard library is also guarded during
retry. Standalone sidebar loading/empty/error states now remain visible.

Tests preceded the relevant changes. The component tests first failed to
compile because `LoadRecovery` was missing. After implementing it, page contract
tests failed on the old raw collection copy and unsubscribed Dashboard state.
Two SSR tests check the stable focusable target and mutually exclusive
alert/status semantics; three source-contract regressions check integration
and pending guards. These source checks are not asynchronous event simulations;
the browser checks below provide request/focus behavior evidence.

### Rebuilt browser checks

Fast-forwarded the fix into the design branch and restarted its existing app.
All injected failures were read requests intercepted before server handling.

- **Collections:** failed load displayed only “Your collections couldn’t be
  loaded. Please try again.” Enter on Try again paused one request, showed
  Trying again… with no simultaneous alert, and retained button focus. A forced
  additional click made **0 extra requests**. Failing that retry restored
  Try again with focus still on it. A successful Enter retry restored
  **25 of 27 top-level collections** matching the retained CP8Catalog filter;
  alerts cleared. The two other top-level collections and three nested records
  remained untouched. An initial filter fill during rebuild/navigation did not
  stick; after the page settled, refill and failure/retry retention passed.
  The original early-input timing issue remains unconfirmed, not closed here.
- **Dashboard drafts:** failed drafts and collection-tree reads left **12 recipe
  rows** intact. Enter Retry drafts issued one request; extra clicks during
  Retrying… issued **0**, retained focus and did not disturb the library. A
  repeated failure restored Retry drafts with focus retained. Success restored
  the one existing draft while the collection tree still displayed its own error.
- **Dashboard collections:** the same pending/extra-click/repeated-failure focus
  checks passed. Successful retry restored **27 root navigation links**, with
  drafts and recipe rows intact. One over-specific link-name wait was corrected
  using the fresh DOM (“0 recipes”, not “0”); recovered tree had zero alerts.
- **Dashboard library:** failed Next retained independent draft/tree sections.
  Enter Retry recipes issued one request; extra retry and Previous page clicks
  while pending made **0 extra requests**, and Previous page was aria-disabled.
  A later successful retry retained pending button focus and restored
  **Showing 13–24 of 33 saved recipes**, 12 rows and zero alerts. Previous returned
  to **Showing 1–12 of 33**. An all-three-sections failure pass recovered all three
  through their own retry buttons without route reload.
- **Guest gates:** signed-out Dashboard and Collections redirected to
  `/login?redirect_uri=%2Fdashboard` and `/login?redirect_uri=%2Fcollections`,
  respectively, with zero private content links. Mock sign-in preserved each
  destination.
- **Authenticated non-owner:** reviewer `user-62c8` saw an empty Dashboard,
  no draft links, no collection-tree links, and Your first album starts here
  on Collections with zero album links. Owner content was not rendered.
  Restored the owner session and normal Dashboard data afterward.
- **Narrow recovery geometry:** per-tab metrics confirmed `innerWidth=320`,
  document width **305px**, and no horizontal document overflow for Collections
  failure and Dashboard's all-three failure state. All four tested retry controls
  were **44px high**, with both edges inside the viewport. This is recovery-state
  geometry, not complete populated mobile/tablet or real-touch acceptance.

Interception and device emulation were cleared; the temporary browser tab was
closed. Normal-desktop screenshots were saved and visually inspected:

- `/tmp/noms-020-collections-safe-recovery.jpg`
- `/tmp/noms-020-dashboard-independent-recovery.jpg`

Read-only database checks confirmed totals still **25 public / 33 private
recipes and 30 private collections**. Original draft revision **17**, exact
commentary, one image, published recipe v2 and **31 collection memberships**
were unchanged. No account linking/deletion, visibility changes or uploads.

### Validation, review and next order

With `SQLX_OFFLINE=true` and the existing integration target cache:

```sh
cargo test --features server load_re -- --test-threads=8
just ci
```

Focused filter: **10 passed, 0 failed** (includes the component tests, two page
contracts and unrelated substring matches). Full CI on `fb884ad`: **exit 0;
849 passed, 0 failed, 12 ignored**, including all five new tests, native/WASM
checks, Clippy, formatting and repository policy gates. Eight existing allowed
audit warnings remain; ignored search/storage integration checks were not
freshly run. Logs: `/tmp/noms-020-load-recovery-tests.log` and
`/tmp/noms-020-workspace-recovery-ci.log`.

Explicit second-pass self-review: **PASS for this scoped fix**, including complete
diff, stable pending recovery branches, live resource guards, API/authorization
boundaries, truthful source-vs-runtime evidence and browser recovery. No blocking
or should-fix finding remains in changed code. No independent-agent review was
performed. One cohesive shared-retry/integration behavior commit and a separate
acceptance-documentation commit; both fast-forward into the existing design
branch. User-owned generated CSS retains SHA-256
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

**Next:** public-profile load recovery and populated recipe/collection pagination.
Source review still finds raw profile/recipe/collection load errors, no retry
actions and incomplete pending/failed-offset guards. Username/route replacement
also needs runtime verification; no new privacy failure is inferred from source
alone. Settings/account and recipe-detail action failures remain later checks.
Post-success focus placement, full screen-reader/device acceptance, pixel-level
exports and final whole-app review remain open. Dark-mode redesign remains
deferred. This slice's rollback is `fb884ad`; no fixture cleanup is needed.

## Public-profile recovery and dense pagination — 2026-09-30

Continued checkpoint 8 in `codex/NOMS-020-checkpoint-8`, fast-forwarding into
the existing design worktree and rebuilding only its existing loopback deployment
at `http://127.0.0.1:32768` (Compose project `noms-5063d4a16468023f`).
Implementation-loop tests-first workflow and explicit second-pass self-review;
no new deployment, push or independent-agent review.

### Fixtures, reproduced defect and implementation

User explicitly approved additional local test data. Commit `4bf0637` adds
`scripts/dev-fixtures/noms-020-public-profile.sql`: transactional, insert-only,
idempotent data under namespace `NOMS020 CP8 public profile 2026-09-30`.
Verified the exact Postgres container's development label before insertion.
New `cp8profile` has no OAuth identity and is clearly labelled synthetic. Its
27 recipes and 27 collections each contain **25 public, one private and one
unlisted** entry, with deterministic IDs. Initial seed and rerun both succeeded;
no existing content or visibility was changed.

Browser reproduced a failed second recipe page hiding all 12 loaded cards,
exposing technical error text and providing no Retry. Commit `147b27f` replaces
raw errors with safe header/section feedback and independently guarded section
state. Requests claim their offset/generation synchronously; failures retain
cards and retry the same offset. Stale/duplicate completions and pending/terminal
extra actions are ignored. Controls remain mounted and focusable across loaded,
pending, failure and completion states. A keyed fragment replaces all profile
hooks/tasks when the requested username changes. No API/auth/backend changes.

Five new tests cover offset/retry transitions, pending/terminal/overflow guards,
stale responses, SSR feedback/control semantics and actual Dioxus keyed-scope
reconciliation. The lifecycle test substitutes a state probe for the loader;
it proves same-username preservation and A → B → A remounting, not network API
behavior. Tests were added before implementation and initially failed because
the new helpers were absent.

Narrow-screen browser checking found status text and the button competing in
the shared horizontal `.load-more` row. A new SSR class assertion failed first;
`5c22c33` then adds profile-scoped stacked feedback/button layout. Shared Explore
styles are unchanged.

### Rebuilt browser acceptance

All failure injections intercepted read requests before server handling.

- **Both public sections:** independently loaded **12 → 24 → 25 unique links**.
  Second-page failure retained 12 cards and the other section's content.
  Retry resumed the failed offset without gaps or duplicates. Actual Enter
  activation retained focus through pending, repeated failure, recovery and
  terminal completion. Forced extra pending/terminal clicks made **0 extra
  requests**. Recipe failure did not disturb 12 collections; collection failure
  did not disturb 25 recipes.
- **Initial section failures:** failed both initial section reads, producing two
  safe alerts and independent Retry controls. Recipe Retry restored 12 cards
  while Collections remained failed; its own Retry then restored 12 collections.
  Recipe focus became Load more recipes on successful recovery.
- **Header failure:** Profile unavailable used generic copy, Retry profile and
  Browse public recipes. Enter Retry showed Trying again…/Retrying… without
  simultaneously announcing an alert. One pending request, **0 extra requests**;
  repeated failure retained Retry profile focus. Success restored header and
  both 12-card sections. Header retry is replaced after success; intentional
  post-success focus transfer remains unimplemented and not accepted here.
- **Delayed route replacement:** paused profile A's second recipe page, navigated
  through Explore to `cp8catalog-01`, then released A's old request. B retained
  its own heading, one recipe, zero collections and zero alerts, with no A cards.
  Returning via Explore to A started at 12 recipes/12 collections, not its old
  offset. One over-specific heading wait was corrected using fresh DOM evidence
  (`CP8Catalog Cook 01 – Synthetic`). Browser evidence is route-away/re-entry;
  direct same-component username replacement is covered by the keyed-scope test.
- **Public-only visibility:** signed-out dense profile reported **25 recipes
  shared publicly**, rendered 25 recipes/25 collections, and excluded fixture
  names 26/27 (private/unlisted). Private-only `user-332d` showed zero public
  recipes/collections and both empty states. No fresh signed-in non-owner profile
  flow was performed in this slice; earlier non-owner checks remain historical.
- **Missing profile:** unique nonexistent username used the same safe unavailable
  presentation; retry remained unavailable and Browse public recipes reached
  Explore. No raw server error or private content rendered.
- **320px geometry:** `innerWidth=320`, document width 305px and no horizontal
  overflow. Loaded, terminal and retained-card failure controls were **44px
  high**, inside both viewport edges, with feedback above the button. This is
  sampled recovery/paging geometry, not complete mobile/tablet or real-touch
  acceptance.

Normal desktop screenshot saved and visually inspected:
`/tmp/noms-020-public-profile-retained-cards.jpg` (12 existing recipe cards and
safe retry feedback). Temporary interception and device emulation were cleared.

Read-only database verification: **50 public / 34 private / one unlisted recipes;
25 public / 31 private / one unlisted collections; 28 users**. Original private
recipe still has v1/v2; original draft revision **17**, exact commentary
`Validation fixture only; keep private.`, one image and **31 collection
memberships** remained unchanged. Synthetic fixtures remain local for later
acceptance; no cleanup, account linking/deletion or upload performed.

### Validation and review

With `SQLX_OFFLINE=true` and the existing integration target cache:

```sh
cargo test --features server user_profile -- --test-threads=8
just ci
```

Focused profile filter: **15 passed, 0 failed**, including eight page-local tests
and seven matching auth/DB tests. Narrow-layout SSR red/green evidence is in
`/tmp/noms-020-profile-layout-red.log` and
`/tmp/noms-020-profile-layout-tests.log`. Earlier recovery evidence:
`/tmp/noms-020-profile-recovery-red.log` and
`/tmp/noms-020-profile-recovery-tests.log`.

Final full CI on `5c22c33`: **exit 0; 854 passed, 0 failed, 12 ignored**, plus
native/WASM checks, Clippy, formatting and policy gates. Initial final run hit
a read-only Cargo advisory lock; rerunning with filesystem access passed.
Eight existing allowed audit warnings remain; ignored search/storage integration
checks were not freshly run. Final log:
`/tmp/noms-020-public-profile-final-ci.log`.

Explicit second-pass self-review **PASS for this scope**: checked complete ordered
diff, fixtures, request/offset ownership, key reconciliation, stable controls,
route-owned CSS, tests and raw runtime outcomes. No blocking or should-fix issue
remains in changed code. No independent-agent review performed. Ordered commits:
`4bf0637` (fixture), `147b27f` (state/recovery), `5c22c33` (narrow layout), followed
by this acceptance documentation. User-owned generated CSS retains SHA-256
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

**Next:** Settings/account load/save failure presentation, then recipe-detail
action failures and remaining cross-route/accessibility acceptance before
whole-app review. Full screen-reader/device checks, post-success header/empty
focus placement, post-commit lost-response handling and exported-image pixel
checks remain open. Dark-mode redesign remains deferred. Reverting the three
code/fixture commits restores this slice; inserted fixture data is not removed
by a Git revert and must not be mistaken for production content.

## Settings save and linked-account load recovery — 2026-09-30

Continued the established isolated `codex/NOMS-020-checkpoint-8` worktree at
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-checkpoint-8`, integrating by
fast-forward into the design worktree and rebuilding only existing deployment
`noms-5063d4a16468023f` at `http://127.0.0.1:32768`.
Implementation-loop skill: tests first, coherent local commits, browser acceptance
and explicit second-pass self-review. No independent-agent review, push, new
deployment or new fixture seed.

### Reproduced findings and changes

- A failed profile save restored old committed fields, discarding entered name
  and Bio, and displayed the raw TypeError/transport wrapper. Button focus became
  BODY when the old native-disabled save state was rendered.
- Linked-account retry did not subscribe to resource state. A paused retry still
  showed the previous alert and enabled Try again rather than pending feedback.
  No baseline focus-loss claim: that captured retry still had button focus.
- Reload/early-fill checking exposed initialization/hydration timing: entered
  text before handlers were ready could be replaced by committed profile data.
  One attempted save during startup generated **zero requests**, so it was not
  counted as a successful failure-injection check. A later hydrated navigation
  generated one request and proved recovery; final hydration gating was checked
  separately below.

`fb5f1bb` retains entered text—including surrounding whitespace—until a successful
save, instead of optimistic trimming and rollback. Unknown server/transport
errors map to safe retry copy; a strict allowlist keeps known username/name/Bio
validation, with sign-in-again guidance for an expired session. Parent handler
checks the live pending signal before spawning. ProfileSaveControl stays mounted
and focusable with aria-disabled pending/invalid guards and a named status.
Fields are read-only during a save so its completion cannot overwrite newer
edits. Editing/retrying clears stale success feedback; auth/navbar data updates
only after success. No save API, schema or authorization changes.

`eac415b` preserves linked-account read Results instead of swallowing errors,
subscribes to resource state, names the initial loading state, and uses the
shared LoadRecovery with a live pending guard. Failure/retry does not infer an
empty account list or offer Connect links.

`91bfe6f` initializes form signal values from available auth before first render,
guards uninitialized edits/saves and adds actual click-handler coverage.
It also removes a redundant loader closure caught by WASM Clippy.
`cd1065a` keeps populated SSR fields read-only and Save aria-disabled until the
client initialization effect has run. Available auth data remains readable,
but native pre-hydration edits cannot be accepted and then overwritten.

### Tests and rebuilt browser evidence

Six new tests: two profile feedback contracts, SSR stable save-control states,
actual ready/pending/invalid click dispatch, named account loading/guest
presentation, and real account-resource failure without a Fullstack session
(fails before DB access). Extended the existing profile SSR test to require
populated initial values and all three read-only fields before hydration.

Initial feedback/control and account tests were written before implementation:
missing new helpers failed compilation; account loading and safe-recovery tests failed; initial-field
and read-only assertions failed before their fixes. Initial test-draft syntax
and lifetime issues were corrected, not counted as behavior evidence. Actual
click-dispatch guard coverage was added during the follow-up review.

Used existing mock subject `noms020-editor-reviewer` / `user-62c8`, not the owner.
All injected failures intercepted requests **before server handling**.

- **Profile pending/repeat failure:** Enter Save issued one request and showed
  Saving profile…/Saving… with all three fields read-only. A forced extra click
  made **0 extra requests**. Failure and repeated failure retained name/Bio text
  exactly, restored editability, used safe copy and kept Save changes focus.
- **Real validation:** attempted already-existing `cp8profile` username. Server
  returned That username is already taken. Please choose another.; attempted
  username, name and Bio were retained. No successful username change performed.
  A later locator interaction was blocked because an inspected handle had been
  masked; explicitly refilling the known reviewer handle restored validity.
  That blocked interaction is not counted as a transport-failure test.
- **Success:** retry with the reviewer's unchanged handle successfully saved
  synthetic `CP8 Settings Review` and Bio, applied trimmed authoritative values,
  cleared the error, retained Save changes focus and updated the account label.
  Subsequent hydrated Profile navigation showed the stored name/Bio. Early
  post-reload observation was not counted as persistence proof until hydration
  settled.
- **Final-build initialization:** on reload, all three fields were read-only and
  Save had aria-disabled=true. Normal field fill waited until handlers were ready.
  Enter generated exactly one intercepted save; pending kept focus/read-only
  state. Failure retained `CP8 Final Settings Test` and the exact spaced Bio
  `  Final retained local input.  `, restored editability and Save focus.
  This is direct final-build startup/failed-save evidence; success/conflict and
  account retries were exercised earlier in the same integrated code series.
- **Account load/retry:** initial pause showed Loading linked accounts…. Failure
  produced safe copy and Retry linked accounts. Enter retry issued one request,
  retained focus as Retrying…, showed Trying again… with zero simultaneous
  alerts/Connect links. Forced extra activation made **0 extra requests**.
  Repeat failure retained retry focus; success restored the single Google card,
  zero alerts, disabled last-account Unlink and the missing-provider Connect
  GitHub link. No connect/unlink action was performed. Post-success account retry
  focus placement remains unimplemented; the retry panel is replaced by data.
- **320px geometry:** profile failure and linked-account failure each had
  innerWidth=320, document width 305px, no horizontal overflow and **44px-high**
  action controls inside the viewport. Not complete populated phone/tablet or
  real-touch acceptance.
- **Guest gates:** after normal reviewer sign-out, Settings Profile/Accounts
  redirected to `/login?redirect_uri=%2Fsettings%2Fprofile` and
  `/login?redirect_uri=%2Fsettings%2Faccounts`; zero profile fields, account rows
  or Unlink controls rendered. Existing mock sign-in at the start of this slice
  returned to Accounts. No owner settings were modified.

Final-build desktop screenshot saved and visually inspected:
`/tmp/noms-020-settings-retained-save.jpg`, showing retained Bio, safe alert and
focused Save changes. Temporary interception/emulation cleared and tab closed.

### Data preservation, validation and review

After successful-save acceptance, restored only the reviewer's temporary
name/Bio to its blank-name/no-Bio baseline using a conditional update guarded
by exact user ID, unchanged handle and the exact values this check had saved
(UPDATE 1). Existing saved avatar edit remained present. Final injected edits
never reached the server. Both original Google links remain one each for owner
and reviewer; no linking, unlinking, account deletion or avatar removal/upload.

Read-only DB checks: **28 users; 50 public / 34 private / one unlisted recipes;
25 public / 31 private / one unlisted collections**. Original draft revision
**17**, exact commentary `Validation fixture only; keep private.`, one image
and **31 collection memberships** unchanged. No recipe/collection data writes.
User-owned generated CSS retains SHA-256
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

With `SQLX_OFFLINE=true` and the existing integration target cache:

```sh
cargo test --features server pages::settings -- --test-threads=8
just ci
```

Focused Settings filter **22 passed, 0 failed**. Final full CI on `cd1065a`:
**exit 0; 860 passed, 0 failed, 12 ignored**, plus native/WASM compilation,
Clippy, formatting and policy gates. Eight existing allowed audit warnings
remain; ignored search/storage integration checks not freshly run.
Logs: `/tmp/noms-020-settings-recovery-red.log`,
`/tmp/noms-020-settings-accounts-red.log`,
`/tmp/noms-020-settings-initialization-red.log`,
`/tmp/noms-020-settings-hydration-red.log`,
`/tmp/noms-020-settings-recovery-tests.log`,
`/tmp/noms-020-settings-recovery-ci.log`.

Explicit second-pass self-review **PASS for this scoped change**: complete diff
and ordered commits, safe feedback allowlist, retained inputs, live pending
guards, hydration gate, stable controls, real resource failure tests, no API/auth
changes and browser/data evidence. No blocking/should-fix issue remains in changed
code; no independent-agent review claimed. Local commit sequence `fb5f1bb` →
`eac415b` → `91bfe6f` → `cd1065a`, followed by this documentation, fast-forwarded
into the existing design branch. Prior scoped baseline: `811f74b`.

**Next:** recipe-detail action failure acceptance, then remaining Settings mutation
and cross-route recovery. Source still exposes raw errors in unchanged unlink
and avatar-removal handlers; those are queued source findings, not newly proven
runtime/privacy failures. Global auth-bootstrap load failure, successful username
replacement, post-commit lost-response/partial-save handling, screen-reader,
real-device and complete per-page accessibility checks remain open. No complete
Settings-page or whole-checkpoint sign-off; dark-mode redesign remains deferred.

## Recipe mutation recovery — 2026-09-30

### Scope and environment

Acceptance for recipe-detail **visibility save, Fork and Restore**: retained
selection/content on failure, safe actionable feedback, guarded pending/retry
controls, focus continuity, successful local mutations and private-page guest
gating. History read/comparison recovery and Delete are not accepted here.

Used existing task branch `codex/NOMS-020-checkpoint-8` at
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-checkpoint-8`, and fast-forwarded
the existing design branch for browser testing. No new branch/worktree was
created: the user’s established integration workflow is retained.

The app at `http://127.0.0.1:32768` was a **new** local Compose instance,
`noms-c799915fcf93e034`, mounted from the design worktree. Its database was empty
before the normal mock-owner login. Previous evidence belongs to the stopped
`noms-5063d4a16468023f` instance, not this database; prior fixtures were not
recreated, deleted or modified. Verified the current Postgres container’s
`com.noms.dev-environment=true` label before test writes.

Added approved, labelled synthetic private recipe
`1b2dcf65-f1a0-4971-7d17-8de25203500b` under this instance’s mock owner
`6b1c56b4-945d-44d5-9f1b-eeb9525b8519` (`user-e13c`). Insert-only transactional
seed `scripts/dev-fixtures/noms-020-recipe-actions.sql` requires an explicit
existing owner ID. Applied twice: one recipe, two immutable snapshots, no
duplicate rows. No account creation is performed by the script.

### Reproduced issues and changes

Baseline pre-server interception reproduced raw transport errors in all three
actions, lost Save visibility focus, visibility selection resetting from
Unlisted to Private on failure, and Restore with no pending feedback/guard.

`bde5b52` fixes this scoped mutation behavior:

- Visibility synchronization observes recipe-result changes, not save-pending
  completion. Failed selections remain editable and retryable. Success uses
  the API’s authoritative recipe without a loading-page remount/refetch.
- Save/Restore share a local stable, focusable 44px control with `aria-disabled`,
  `aria-busy` and pending status. UI and live handler guards block repeated
  requests. One pending Restore also blocks other versions’ Restore actions.
- Fork retains the existing action menu and its normal trigger-focus return;
  pending status and an alert with a Dashboard-drafts link remain visible after
  the menu closes. Restore alerts are scoped to the attempted history row.
- Unknown server/transport failures never render internal error text. Copy
  says **could not confirm**, with check-drafts/reload guidance before retry,
  because a missing response cannot prove that a write did not commit.
  Known unauthenticated errors advise signing in in another tab.

### Browser evidence on the final build

Fresh reload after the existing app-container restart; real requests observed
with per-tab CDP interception, no injected application calls or DOM mutations.

- **Visibility:** one held request; Enter/Space while pending produced zero
  additional requests. Failed request retained Unlisted, title/history and
  focused Save visibility; keyboard retry issued one request and failed safely
  again. 320px viewport: document/scroll width 305px, button 44px high and fully
  inside the viewport. Temporarily set only this synthetic recipe to Unlisted
  in the DB to test a real UI save **back to Private**: one request, authoritative
  private badge/select, success status and focused Save visibility remained;
  completed/unchanged activation issued zero requests. No recipe made public.
- **Fork:** one held request, named pending status and unavailable menu action.
  Reopening the menu and pointer-activating the pending item produced zero
  extra requests. Failure gave safe alert/check-drafts link and returned focus
  to Recipe actions. Retry issued one request. A later successful real fork
  opened draft `ef64122d-ec6b-4494-92a8-f997178584bd` in the editor with source
  attribution and Private publishing selection; no source recipe mutation.
- **Restore:** one held request, stable focused button/status, zero duplicate
  requests from Enter/Space, then safe alert beside v1 and retryable Restore.
  320px geometry: document/scroll width 305px and 44px-high control. Successful
  retry appended v3 from v1, preserving old history. A separately loaded stale
  v2 tab then received real conflict feedback without appending a version.
  With v3 loaded, holding a v2 Restore blocked another version’s Restore too;
  zero additional requests, failure alert on v2 only. A subsequent successful
  v2 restore appended v4, returning the fixture’s content to its original v2
  snapshot. Success retains the existing full-page reload behavior; no
  post-success Restore-focus continuity is claimed.
- **Guest:** normal sign-out and fresh private-recipe reload rendered Recipe
  not found, no fixture title, history rows, visibility control or recipe menu.
  Expired-session mutation copy has unit coverage, not a browser session-expiry
  walkthrough. Public/non-owner mutation presentation was not re-audited here.

Captured and visually inspected desktop Restore failure:
`/tmp/noms-020-recipe-restore-recovery.jpg`. Additional evidence:
`/tmp/noms-020-recipe-visibility-recovery-mobile.jpg`,
`/tmp/noms-020-recipe-visibility-success.jpg`,
`/tmp/noms-020-recipe-fork-success.jpg`. Temporary interception and device
overrides cleared; agent-created tabs closed and mock owner signed out.

Final current-instance DB: **one user, one private recipe, four versions, one
owner-only fork draft at revision 0, zero collections**. The draft references
source v4 and has no published recipe ID. Snapshot equality checks confirmed
v3=v1 and v4=v2. No deletion, real content modification or old-instance writes.
User-owned generated CSS SHA-256 remains
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

### Validation and review

Tests first: focused red log records missing planned helper/control definitions
before implementation. Six new tests cover safe/uncertain-outcome and session
copy, rendered focusable states, actual event-handler pending/unavailable
guards, visibility-effect subscription and live handler/row-local contracts.

With `SQLX_OFFLINE=true` and `CARGO_TARGET_DIR` set to the existing integration
worktree’s target cache:

```sh
cargo test --features server pages::recipe_detail -- --test-threads=8
just ci
```

Focused recipe-detail tests: **38 passed, 0 failed**. Final `just ci` on
`bde5b52`: **exit 0; 866 passed, 0 failed, 12 ignored**, including native/WASM
checks, Clippy, formatting and policy gates. Eight existing allowed audit
warnings remain; ignored integration checks were not freshly exercised.
Logs: `/tmp/noms-020-recipe-mutations-red.log`,
`/tmp/noms-020-recipe-mutations-tests.log`,
`/tmp/noms-020-recipe-mutations-ci.log`.

Explicit second-pass self-review **PASS for this slice**: complete ordered diff,
test/event evidence, safe error copy, pending guards, unchanged API authorization,
stable focus, isolated restore feedback and synthetic-only data changes. No
independent-agent review claimed. No blocking/should-fix finding remains in the
changed scope. Commit sequence: `823a4f2` fixture → `bde5b52` behavior/tests →
this acceptance documentation. Prior scoped baseline: `c7d597e`; integration
fast-forward only, no push or new deployment.

**Next:** recipe-detail initial history/lineage and older-page/difference load
recovery, then remaining Settings mutations/global-auth recovery and broader
cross-route failure/permission/accessibility acceptance. Unchanged Delete still
has raw error presentation and no live pending guard; no delete confirmation or
request was attempted. Post-commit lost-response/idempotency, navigation-failure
fallbacks, recipe route replacement, screen-reader and real-device checks remain
open. This is not full recipe-page or whole-checkpoint acceptance; dark-mode
redesign remains deferred.

## Recipe history, lineage and inline difference recovery — 2026-09-30

### Accepted scope and implementation

Current recipe detail now distinguishes initial history/lineage loading,
failure and successful empty results. Each read has independent safe feedback
and a guarded retry; failed optional reads no longer silently disappear. The
owner's History link and section remain available during loading/failure.
Successful empty lineage retains its existing presentation (no lineage panel).

Older-history failures retain the loaded snapshots and failed cursor. Pending,
retry and terminal controls remain focusable; live guards prevent duplicate
requests. Successful exhaustion leaves an inert **All versions loaded** control
in place. Inline differences provide direct **Retry differences** feedback,
named pending status and disclosure relationships, rather than raw transport
errors or requiring collapse/reopen to retry.

Recipe contents use a keyed visit scope to reset captures, history cursors,
comparison state and errors across recipe replacement, including A → B → A.
The existing mutation control was renamed to `RecipeActionControl` and reused;
its mutation behavior and regression assertions remain intact. A populated
320px comparison exposed clipped fixed-minimum columns; columns now shrink to
the available width, and field headers wrap. This shared comparison-panel
layout correction also applies to historical snapshots, but their custom
comparison recovery is **not** accepted by this slice.

No API, authorization, sharing, deletion or global design-token changes.
The established checkpoint worktree was reused, not a new branch/deployment:
`codex/NOMS-020-checkpoint-8` in `.worktrees/NOMS-020-checkpoint-8`, fast-forwarded
into `codex/NOMS-020-visual-design-refinement` in
`.worktrees/NOMS-020-visual-design-refinement`.

### Approved fixture and browser evidence

Reused the existing local instance **noms-c799915fcf93e034**, app
`http://127.0.0.1:32768`, after verifying its development label. The older
stopped instance was not used. Insert-only transactional fixture
`scripts/dev-fixtures/noms-020-recipe-history.sql` requires an explicitly supplied
existing mock owner; no new account or real content was introduced. It adds a
private 30-version root, a private one-version fork and one published lineage
edge. Collision checks and a repeat application passed without duplicates.
The first ambiguous-variable attempt rolled back before the corrected script
was applied; the prior recipe/draft remained unchanged.

Browser checks used normal mock-owner sign-in and UI navigation. Failures were
injected before server execution using tab-scoped request interception; no
fabricated successful API payloads or DOM mutations were used.

- **Initial reads:** reproduced the old silent history/lineage failures, then
  verified named loading states and two independent safe retry panels. Retrying
  history recovered 25 snapshots while lineage retained its own failure;
  retrying lineage recovered the root/fork edge without restarting history.
  Enter/Space during either pending retry produced one request only and retained
  the native retry button's focus. At 320px, both retry targets were 44px high,
  inside a 305px content viewport with no document overflow.
- **Older history:** failure retained v30 through v6 and focused **Retry older
  versions**. Retry recovered v5 through v1: 30 unique, descending snapshots.
  Pending Enter/Space produced one request; terminal activation produced zero.
  Focus remained on **All versions loaded**. The failed retry target at 320px
  was 233×44px and retained all 25 initial rows without overflow.
- **Inline differences:** held requests blocked repeated keyboard activation
  and another version's forced pointer activation. Failure retained focus on
  **Retry differences**, `aria-expanded=true` and
  `aria-controls=recipe-difference-30`; no native disabled attribute or raw error.
  Direct retry recovered the real v29 → v30 description changes and retained
  **Hide differences** focus. Collapse/reopen requested a new comparison.
  At 320px the retry target was 142×44px; the populated panel's client and scroll
  widths both measured 201px after the correction (previously 201/245px).
- **Route isolation:** direct lineage links exercised root/fork navigation,
  with the fork showing only v1 and no inherited comparison/error/cursor.
  These lineage links are normal document links, not SPA-only proof. The
  router-aware fork attribution link followed by browser Back/Forward then
  exercised same-document replacement. After releasing a delayed root
  comparison following Back, Forward restored a fresh root with 25 snapshots
  and no expanded comparison. An old root paging request released
  after Back → Forward did not append to the new root visit: it retained its
  fresh 25 snapshots, idle **Load older versions**, and no stale errors/panel.
- **Guest:** normal sign-out and fresh private-root navigation rendered Recipe
  not found, without fixture title, history, lineage or owner controls. This is
  private guest-gate evidence, not fresh public/non-owner or session-expiry
  acceptance.

Screenshots captured and visually inspected:
`/tmp/noms-020-history-lineage-recovery.jpg` (history retry panel),
`/tmp/noms-020-history-pagination-recovery.jpg`,
`/tmp/noms-020-history-differences-recovery.jpg`,
`/tmp/noms-020-history-differences-success.jpg`, and
`/tmp/noms-020-history-differences-mobile.jpg` (actual 320×900 viewport).
Browser handles were reconnected after the browser service changed; no new
deployment or test identity was needed. Interception/device overrides cleared,
normal viewport restored, mock owner signed out and agent-created tab closed.

Final current-instance DB: **one user, three private recipes, 35 versions,
one unchanged owner-only draft, zero collections and one published lineage
edge**. No mutation/deletion was exercised in this slice. User-owned generated
CSS SHA-256 remains
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

### Validation, review and remaining work

Tests-first failures are retained in
`/tmp/noms-020-recipe-reads-red.log`,
`/tmp/noms-020-recipe-history-controls-red.log` and
`/tmp/noms-020-recipe-comparison-width-red.log`. Eight new tests cover rendered
read feedback, independent resource subscriptions/guards, actual keyed visit
remounting, actual continuation cursor/state invariants, empty/terminal guards,
difference retry/collapse/pending decisions, handler contracts and the narrow
comparison layout. Source-contract/SSR tests do not replace the runtime evidence
above; no full live server-component harness is claimed.

With `SQLX_OFFLINE=true` and `CARGO_TARGET_DIR` set to the existing integration
worktree's target cache:

```sh
cargo test --features server pages::recipe_detail -- --test-threads=8
just ci
```

Final focused suite: **46 passed, 0 failed**. Final `just ci` on `60f4eb3`:
**exit 0; 874 passed, 0 failed, 12 ignored**, including native/WASM checks,
Clippy, formatting and policy gates. Eight existing allowed audit warnings
remain; ignored integration checks were not freshly exercised. Logs:
`/tmp/noms-020-recipe-reads-tests.log`, `/tmp/noms-020-recipe-reads-ci.log`.

Explicit second-pass self-review **PASS for this slice**: reviewed the complete
ordered diff, tests, actual cursor/route/focus evidence, unchanged authorization
and mutation contracts, fixture boundaries and final narrow layout. The clipped
comparison finding was fixed and revalidated; no blocking/should-fix finding
remains in the changed scope. No independent-agent review claimed. Ordered
commits: `6d14bcd` fixture → `fa547af` independent reads/keyed visits → `51c5f8e`
paging/difference recovery → `60f4eb3` narrow comparison → this documentation.
Baseline `35942cd`; integration fast-forward only, no push.

**Next:** remaining Delete and Settings mutation/global-auth recovery, plus
historical-snapshot custom-comparison failure states and broader cross-route
failure/permission/accessibility acceptance. Post-success focus placement for
initial read retries, full screen-reader/real-touch checks, post-commit
lost-response/idempotency and navigation-failure fallbacks remain open. This
does not close the full recipe page or checkpoint 8; dark-mode redesign remains
deferred.

## Recipe Delete safeguards and shared confirmation — 2026-09-30

### Implemented scope

Recipe deletion now requires a separate, recipe-named confirmation in the
existing M4 `SharedModal`. Opening the owner action menu's Delete item does not
call the API. **Cancel** receives initial focus; Cancel, Escape and backdrop
dismissal never confirm deletion. A live owner/open-dialog guard and synchronous
pending claim prevent duplicate requests. Pending controls remain mounted and
focusable, with guarded handlers and `aria-disabled`/`aria-busy`, and dismissal
is blocked while a request is pending.

Pending status and safe error alerts replace raw server/transport errors.
Uncertain-outcome copy does not promise that the recipe survived a failed
response; **Check your Dashboard** opens another tab without discarding the
current page. Authentication-error copy preserves the recipe and asks the user
to sign in in another tab. Failure releases pending and preserves the dialog
for an explicit retry. The existing Delete API, server authorization and
successful Dashboard navigation contract are unchanged. No shared modal,
theme or API implementation was modified.

### Native-dialog test incident and bounded fixture recovery

The preliminary implementation (`76f7369`) retained a native confirmation.
During an intended open-and-cancel browser check, the native prompt blocked
automation: click/dialog/focus operations timed out, and the agent-created tab
was closed to recover browser access. No explicit confirmation acceptance was
issued by the agent. Nevertheless, the local API deletion ran; its exact
browser/tool cause was not conclusively established. This attempt is **not**
successful cancellation evidence.

The deleted data was the agent-created private **CP8 Recipe actions – Local
test only** fixture (`1b2dcf65-f1a0-4971-7d17-8de25203500b`) and its four versions.
Its preserved owner-only fork draft remained, but its source foreign key was
nulled by the deletion. No real recipe was affected; the two synthetic history
recipes and their lineage remained intact. The user was informed of the
incident and recovery limits during the work.

A guarded transaction on this exact local test deployment reconstructed the
same recipe UUID and private content using the retained revision-0 draft and
checked-in seed. Versions 1/3 reproduce the first method, versions 2/4 the
second; the known draft's source link to version 4 was restored without
changing its snapshot or revision. Original recipe/version timestamps and
API-generated v3/v4 UUIDs were unavailable and were regenerated. Recovery is
therefore **content-equivalent, not exact metadata restoration**. The one-off
recovery script is `/tmp/noms-020-recover-test-fixture.sql`, not a production
migration. Native confirmation was then replaced by the shared modal in
`41c933b`; no destructive action was confirmed during its final browser checks.

### Final shared-modal browser evidence

Used only the existing development instance **noms-c799915fcf93e034**, app
`http://127.0.0.1:32768`, with the established mock owner and synthetic fixture.
Defensive interception of non-document Fetch/XHR/Other requests recorded
**zero requests** throughout the final cancellation paths; no fabricated
successful API responses or DOM mutations were used.

- Opening Delete showed the correct recipe title and irreversible warning,
  with initial focus on **Cancel**. Shift+Tab/Tab wrapped between the two
  controls. Clicking Cancel restored focus to the recipe-action trigger.
- At an actual **320×900** viewport, page client/scroll widths were both 320px;
  the modal card was 304×242px at x=8. Cancel measured 77×44px and Delete
  116×44px. The warning wrapped and both controls remained within the viewport.
- Enter on Cancel and Escape after reopening dismissed without a request.
  Closing left no open dialog or body scroll lock. Recipe versions v4 through
  v1 remained available.
- Normal sign-out and fresh private-recipe navigation showed Recipe not found,
  without fixture title/history or owner/Delete controls. This is private
  guest-gate evidence, not public non-owner or session-expiry acceptance.

Screenshots captured and visually inspected:
`/tmp/noms-020-recipe-delete-confirmation.jpg` and
`/tmp/noms-020-recipe-delete-confirmation-mobile.jpg`.
Interception and viewport overrides cleared, mock owner signed out and
agent-created tab closed. No other deployment or user browser tab was changed.

Final read-only recovery check: **one user, three private recipes, 35 versions,
one owner-only draft at revision 0, zero collections and one lineage edge**.
The recovered draft points to the original recipe UUID/version 4; snapshot
equality checks confirm v1=v3 and v2=v4. App health returns HTTP 200.

### Validation, review and remaining work

Tests-first failures are retained in `/tmp/noms-020-recipe-delete-red.log` and
`/tmp/noms-020-recipe-delete-dialog-red.log`. Six new tests cover actual signal
claim/cancellation/exclusivity, named warnings and safe uncertain outcomes,
owner/dialog/handler contracts, rendered status/error feedback, rendered modal
controls, and actual Cancel/Confirm event dispatch including pending guards.
Existing mutation authentication tests now include Delete. The menu workflow
contract was updated for explicit shared confirmation, not removed or skipped.
Unit/SSR evidence does not constitute a live Delete API acceptance harness.

With `SQLX_OFFLINE=true` and `CARGO_TARGET_DIR` pointing to the existing
integration worktree's target cache:

```sh
cargo test --features server pages::recipe_detail -- --test-threads=8
just ci
```

Final focused suite: **52 passed, 0 failed**. Final `just ci` on `41c933b`:
**exit 0; 880 passed, 0 failed, 12 ignored**, including native/WASM checks,
Clippy, formatting and policy gates. Eight existing allowed audit warnings
remain; ignored integration checks were not freshly exercised. Logs:
`/tmp/noms-020-recipe-delete-tests.log`, `/tmp/noms-020-recipe-delete-ci.log`.

Explicit second-pass self-review **PASS for the final implementation and
scoped cancellation evidence**: reviewed the complete ordered diff, actual
event/signal tests, default-safe focus, pending guards, safe feedback,
unchanged server authorization and API contracts, and fixture recovery limits.
No blocking/should-fix finding remains in the changed scope. No independent
agent review claimed. Ordered commits: `76f7369` guards/feedback → `41c933b`
shared confirmation → this documentation; baseline `52da873`. Existing task
worktree reused and integration fast-forward only, no push. The integration's
user-owned generated CSS was preserved; SHA-256 remains
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

**Next:** Settings mutation/global-auth recovery and historical-snapshot custom
comparison failures, followed by broader cross-route permission/failure and
per-page accessibility acceptance. Final shared-modal Delete pending/error,
retry and success/navigation were **not browser accepted**; a separately
approved expendable-fixture destructive check remains open. Post-commit lost
responses/idempotency, navigation-failure fallback, initial-read retry success
focus, full screen-reader and real-touch checks also remain open. This does
not close the whole recipe page or checkpoint 8; dark-mode redesign is deferred.

## Global session-refresh recovery — 2026-10-01

### Scope and implementation

The hydrated `/api/user_profile` check previously ignored transport, HTTP and
JSON failures. `6ddb93d` moves that single check into an `AuthRefresh` component
in the persistent shared layout, retaining the root SSR-provided auth context.
A failed check no longer silently strands the user: a raised, shared-token
recovery panel offers **Retry sign-in check** without reloading the page or
discarding its route/form state. Initial successful checks remain invisible.

A synchronous live pending claim prevents overlapping requests. A retry keeps
the same focusable, 44px-minimum action through failure, pending and completion,
with safe alert/status copy and guarded `aria-disabled`/`aria-busy` handlers.
Success changes that control to **Dismiss**, rather than immediately unmounting
the focused button. Explicit dismissal returns focus to the shared navigation
menu trigger. No error body, transport detail or raw endpoint response is shown.

Only structurally consistent authenticated/guest responses replace the context.
Failures preserve it; a valid guest result still updates the existing protected
page gate. A delayed result is rejected if the context changed in flight, so a
recent profile save or identity replacement cannot be overwritten by an older
read. That rejection remains explicitly retryable. This does not add periodic
session polling, cross-tab synchronization or new authorization authority:
server/session/API contracts and the serialized context fields are unchanged.
`AuthContext` only gains `PartialEq` for response/context comparison.

### Existing-instance browser acceptance

Reused only development instance **noms-c799915fcf93e034**, app
`http://127.0.0.1:32768`, via the normal Google mock flow for the established
`testuser` owner. Task worktree reused; implementation fast-forwarded into the
design worktree and only that app container restarted. No new deployment,
identity, account link, avatar, deletion or publication was needed.

Interception was scoped to non-document Fetch/XHR requests for this exact
instance's `/api/user_profile`. Failed requests were blocked before reaching
the server; successful retries continued to the real endpoint. No fabricated
successful payloads, page-state injection or DOM mutations were used.

- **Failure/retention:** the failed initial check kept owner navigation and
  Profile settings available. Unsaved synthetic Display name
  `CP8 Unsaved session check` and Bio `Local unsaved recovery check; do not
  submit.` remained intact across a failed retry and real successful retry.
  These entries were never saved.
- **Pending/exclusivity:** a held retry rendered Checking sign-in status and
  retained focus on `auth-refresh-action`. Return/Space sent to the focused
  button produced **zero additional session requests**. Failure returned to
  Retry sign-in check with focus retained. The high-level locator's pending
  key press was refused; actual focused-key input supplied this runtime proof.
- **Success/focus:** continuing a real retry showed Sign-in status checked,
  kept the unsaved entries and focused the same recovery action. Dismiss
  removed the panel and focused `navigation-menu-trigger`.
- **Shared route lifetime:** after another injected initial failure, the normal
  Profile → Linked accounts link retained one recovery panel and generated
  **zero automatic session checks** during same-document navigation. The linked
  account resource loaded independently.
- **Narrow layout:** at a real **320×900 viewport**, document client and scroll
  widths both measured **305px** (15px scrollbar), without horizontal overflow.
  The retry target was **156×44px**, x=32, right=188px; the panel/copy fit and
  wrapped within the container.
- **Verified guest transition:** a second agent-created tab used normal Sign
  Out. The first tab retained its old owner context until an explicit real
  session retry returned guest. It then showed Sign in to continue, without
  linked-account details or owner navigation, and retained the return link
  `/login?redirect_uri=%2Fsettings%2Faccounts`. Dismiss again returned navigation
  focus. This is normal sign-out/verified-guest proof, not forced session-expiry
  or every cross-route permission state.

The first post-restart browser attempt encountered a dev-client rendering
exception. A cache-disabled reload then rendered and exercised the checks
above successfully; the exact cause of that first transient was not proven.
It is not counted as acceptance evidence. Temporary interception, cache and
viewport overrides were cleared; mock owner signed out and both agent-created
tabs closed. Other deployments and user tabs were untouched.

Screenshots captured and visually inspected:
`/tmp/noms-020-auth-refresh-failure.jpg` (desktop with retained unsaved name) and
`/tmp/noms-020-auth-refresh-mobile.jpg` (actual narrow viewport).

Read-only final DB check: **one user, three private recipes, 35 versions, one
draft, zero collections, one lineage edge and one OAuth account**. Owner Display
name remains empty, Bio/avatar URL remain NULL. The draft remains revision 0
with the restored recipe-action source UUID/version 4. No domain-content writes
were performed; normal sign-in/sign-out changed session bookkeeping only.
User-owned generated CSS SHA-256 remains
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

### Automated validation, review and remaining work

Seven new tests cover exclusive pending/failure/retry state, successful verified
guest completion, rejection of delayed identity/profile replacements, malformed
or inconsistent response validation, SSR feedback/control semantics, actual
pending click dispatch guards and shared-shell wiring. The delayed profile-edit
case has automated state evidence, not a browser profile-save race. Tests-first
missing-behavior failures are recorded in `/tmp/noms-020-auth-refresh-red.log`;
initial module/harness compile errors were corrected during the implementation
cycle, not hidden by skipping coverage.

With `SQLX_OFFLINE=true` and the integration worktree's existing target cache:

```sh
cargo test --features server auth::refresh -- --test-threads=8
just ci
```

Focused suite: **7 passed, 0 failed**. The first full CI run had **886 passed,
1 failed, 12 ignored**: the existing small-upload test timed out connecting to
its temporary PostgreSQL pool (`PoolTimedOut`). Tests were unchanged. Full
rerun on the final implementation: **exit 0; 887 passed, 0 failed, 12 ignored**,
with native/WASM checks, Clippy, formatting and policy gates. Eight existing
allowed audit warnings remain; ignored integration checks not freshly run.
Final logs: `/tmp/noms-020-auth-refresh-tests.log`,
`/tmp/noms-020-auth-refresh-ci.log`.

Explicit second-pass self-review **PASS for this scope**: complete ordered diff,
state/event tests, identity consistency/staleness guards, stable success focus,
real retry and guest gate, narrow geometry, unchanged server contracts and
preserved test data reviewed. No blocking/should-fix issue remains in changed
scope; no independent-agent review claimed. Baseline `7942aa6`; implementation
`6ddb93d` followed by this documentation, local fast-forward integration only,
no push.

**Next:** remaining Settings unlink/avatar-removal/account-deletion recovery,
historical-snapshot custom-comparison failures and broader per-page acceptance.
Destructive browser flows still require a separately approved expendable
fixture. A profile-save race and malformed HTTP body were not browser tested;
their scoped state/parser guards have automated coverage. Indefinitely stalled
requests, periodic/cross-tab session refresh, screen-reader/real-touch checks and
whole-app accessibility remain outside this acceptance. Checkpoint 8 and
dark-mode redesign are not closed by this slice.

## Settings mutation UI recovery — 2026-10-01

### Scope and implementation

Implementation `38d8dc8` adds recovery for unlinking a sign-in method, removing
an avatar and deleting an account. It reuses the existing checkpoint task
worktree and is locally fast-forwarded into the design integration worktree;
no push or new deployment. Server mutation/authentication contracts, global
button styling and the user-owned `assets/tailwind.css` are unchanged.

Task branch: `codex/NOMS-020-checkpoint-8`, worktree
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-checkpoint-8`.
Integration: `codex/NOMS-020-visual-design-refinement` at
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement`.

- Unlink confirmation now stays open while pending and after failure. A live
  exclusive claim blocks duplicate requests, cancellation and provider changes
  while pending. Both opening and submitting recheck the current loaded list;
  the sole remaining method cannot be unlinked. Successful completion closes
  the dialog and refreshes the list.
- Scoped focusable controls retain their nodes while pending, announce busy
  state and guard actual event dispatch. Avatar removal shares the existing
  exclusive upload authority and keeps selected image edits on failure.
- Account deletion retains its three-stage confirmation and phrase on failure.
  Submission checks the live authenticated username rather than a captured
  value; Back/final-action controls stay mounted and guarded while pending.
- Unknown failures use safe uncertain-outcome wording, not raw errors or a
  promise that the mutation rolled back. Guidance advises checking the account
  in another tab before retrying. Known expired-session/last-account errors
  receive specific recovery advice. Existing irreversible success/navigation
  behavior is not redesigned in this slice.
- OAuth linking-conflict feedback is provider-allowlisted, persistent until
  dismissed and no longer recommends deleting another account. Dismissal
  removes the query with History replacement, not a page reload, and focuses
  the Linked accounts heading. The old timed dismissal/reload is removed.

### Automated evidence and review

Ten new tests cover safe feedback, retained/exclusive unlink state, current
candidate/last-method guards, avatar-removal failure retaining image edits,
pending/error SSR semantics and actual control/dialog click dispatch. Existing
account-deletion state tests cover phrase, pending and retry authority. The
tests-first missing-behavior failures are in
`/tmp/noms-020-settings-mutations-red.log`; implementation/harness issues were
fixed without skipping coverage. The dialog event harness resolves real named
button IDs instead of assuming child-component mounting order.

With `SQLX_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`:

```sh
cargo test --features server pages::settings -- --test-threads=8
just ci
```

Focused Settings suite: **32 passed, 0 failed**. Full CI: **exit 0;
897 passed, 0 failed, 12 ignored**, including formatting, native/WASM checks,
Clippy and repository policy gates. Eight existing allowed dependency audit
warnings remain; ignored integration tests were not freshly executed. Logs:
`/tmp/noms-020-settings-mutations-tests.log` and
`/tmp/noms-020-settings-mutations-ci.log`.

Explicit second-pass self-review **PASS for the implemented UI scope**:
complete diff, state/event tests, live submission guards, safe uncertain-outcome
copy, stable pending nodes, unchanged server contracts and preservation of
unrelated work reviewed. No independent-agent review claimed. Ordered series:
baseline `cb659a4`, implementation `38d8dc8`, then this evidence/plan update.

### Non-destructive browser acceptance

Reused the named local test deployment `noms-c799915fcf93e034` at
`http://127.0.0.1:32768`, with only its app container restarted after local
integration. Normal mock Google sign-in used the existing owner `user-e13c`;
the unrelated deployment on port 32773 was untouched.

- Sole Google method: the new control has `aria-disabled=true`, no native
  disabled attribute and remains focusable. Forced pointer, Enter and Space
  attempts opened no dialog and sent **zero API requests**.
- A synthetic `account_already_linked&provider=github` query exercised the
  conflict presentation, not an actual linking attempt. Safe feedback remained
  beyond the former ten-second timeout. At 320px viewport width,
  document client/scroll widths were both 305px. Manual dismissal removed the
  query and alert and focused `linked-accounts-title` without reloading.
- Account-deletion initial focus was Cancel; Enter cancelled and restored the
  Delete Account trigger. An incorrect phrase disabled Continue. The correct
  synthetic-owner phrase advanced to the final screen, initially focused on
  Go Back; Back retained the exact phrase and focused its input. At 320px,
  document client/scroll widths were both 320px and final controls were 44px
  high. Escape closed the final dialog and restored trigger focus. These
  confirmation/cancellation checks sent **zero API requests**.
- No final Delete My Account, permitted unlink, avatar-removal or new-provider
  action was submitted. Non-document API request interception was used as a
  defensive cancellation check, not as authority to click destructive actions.

Screenshots (temporary, not committed):
`/tmp/noms-020-settings-account-recovery-mobile.jpg`,
`/tmp/noms-020-settings-account-confirmation.jpg`,
`/tmp/noms-020-settings-account-confirmation-mobile.jpg`.
Viewport/cache/interception overrides were cleared, the mock owner normally
signed out and the agent-created tab closed. Read-only post-check: **1 user,
3 recipes, 35 versions, 1 draft, 0 collections, 1 OAuth account**; owner name
remains empty, bio/avatar remain NULL, and the original fork draft remains
revision 0 linked to version 4 of its source. User-owned CSS hash is still
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

**Next:** historical-snapshot custom-comparison failure states, then broader
per-page acceptance. Unlink/removal/deletion pending/failure/retry/success have
automated evidence only, not full browser acceptance: this owner has one
provider and no avatar. Destructive end-to-end checks need a separately approved
expendable fixture and action-time confirmation where required. Post-success
focus, post-commit lost responses, account-deletion navigation failure,
identity replacement during mutations, real-touch and full screen-reader
acceptance remain open. This does not close checkpoint 8 or dark-mode redesign.

## Historical snapshot and custom-comparison recovery — 2026-10-01

Implementation `13b112d`, following baseline `6718c5b`, adds a real in-page
historical snapshot retry, safe comparison feedback, synchronous exclusive
comparison authority and keyed recipe/version visits. Failure keeps the chosen
Before/After pair; selectors cannot change it while pending. Successful snapshot
retry focuses its read-only summary; the comparison action retains focus through
pending/failure/success. Inconsistent response pairs are rejected. Narrow
selector columns shrink to their container. Server/history contracts unchanged.

The browser reproduced the baseline's raw transport error. Rebuilt acceptance
used only existing private owner/history fixtures in the named local deployment
on port 32768, normal mock-owner login and non-document API interception:

- Initial snapshot failure and repeated failed retry showed safe feedback.
  Rapid Enter attempts during a held retry produced one request. The real
  successful retry focused `historical-snapshot` and restored version v5.
- Comparison v4 → v2 retained both selections through two failures and then
  real successful retry. Pending action remained focused and aria-disabled;
  Enter/Space duplicates produced one request. Pending selectors were disabled.
  Success cleared the alert and retained Generate comparison focus.
- v2 → v2 produced an explicit no-content-differences panel. At 320px,
  document client/scroll widths were both 305px and the action was 44px high.
- A held v3 → v2 response was released after current-recipe navigation and
  same-document Back to v5. The returned visit retained fresh defaults v1/v5
  and no custom panel; the old response did not populate it. Direct recipe and
  snapshot-number replacement, including return visits, also has keyed-scope
  automated evidence (not a separately exercised direct browser replacement).
- Normal sign-out and guest retry showed only the generic unavailable state,
  no private title/content and no comparison selectors.

Five new state/SSR/keyed-scope tests pass. Two existing source assertions were
updated for renamed live pending state and keyed contents without removing
their subscription/selector-lock contracts. Tests-first expected failures:
`/tmp/noms-020-historical-red.log`; initial implementation compile/test issues
were repaired, including WASM-only scaler imports. Final
`SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci`
passed: **902 passed, 0 failed, 12 ignored**, eight existing allowed audit
warnings. Log: `/tmp/noms-020-historical-ci.log`. Ignored integrations not run.
Explicit second-pass self-review **PASS for this scope**, no independent agent.

Temporary screenshots: `/tmp/noms-020-historical-comparison-failure.jpg` and
`/tmp/noms-020-historical-comparison-mobile.jpg`. Interception and viewport
overrides cleared; cache override remains temporary for subsequent acceptance
in this same turn. Counts unchanged: 1 user, 3 recipes, 35 versions, 1 draft,
0 collections, 1 OAuth account. Local commit/fast-forward only; user-owned CSS
hash unchanged. Existing checkpoint task/integration worktrees are reused.

**Next:** graph recovery, broader route/permission presentation and populated
phone/tablet checks. Real-device touch, actual screen-reader operation,
destructive end-to-end outcomes and post-commit/navigation recovery remain
unaccepted; this slice does not close checkpoint 8.

## Version graph recovery follow-up — 2026-10-01

Implementation `4879025` follows `35c5cf3` in the existing checkpoint task and
design integration worktrees. Initial graph load now offers a live-guarded
in-page retry with stable pending focus and successful focus on the graph
content. Comparison submission uses a synchronous exclusive claim on both
menu and panel paths, rejecting missing/equal pairs. Selection, swap and clear
handlers guard live pending state too. The panel action is focusable and busy
while pending. Unknown expansion/comparison errors no longer expose transport
or database details; privacy-safe unavailable classification remains intact.
Server contracts, topology, transforms and route-generation guards unchanged.

Three new tests cover safe copy, actual synchronous claims and handler/resource
wiring. Existing pending/subscription and expansion-cache assertions are kept;
the stale-expansion failure test now requires safe copy rather than raw error.
Tests-first failures: `/tmp/noms-020-graph-recovery-red.log`. Focused graph
suite **64 passed, 0 failed**. Final full CI with `SQLX_OFFLINE=true` and the
integration target cache: **905 passed, 0 failed, 12 ignored**, exit 0; eight
existing allowed audit warnings. Commands: `cargo test --features server
pages::recipe_version_graph -- --test-threads=8`, then `just ci`. Logs:
`/tmp/noms-020-graph-recovery-tests.log` and
`/tmp/noms-020-graph-recovery-ci.log`. Ignored integrations not freshly run.
Second-pass self-review **PASS for this scope**, no independent-agent review.

Browser acceptance on the existing port-32768 test deployment:

- Initial graph read and first retry failed before reaching the server. Pending
  retry retained focus and ignored Enter/Space duplicates (one request). Real
  successful retry restored the populated graph and focused
  `version-graph-content`.
- v29's keyboard-opened menu started Compare with current (v30). A forced
  panel activation and Enter while held did not duplicate the request. Safe
  failure retained v29 → v30; panel retry returned real description differences
  and kept focus on Compare selected snapshots, with no remaining alert.
- At 320px, document client/scroll widths were both 305px. Before/After cards,
  result and focused action fit; screenshot
  `/tmp/noms-020-graph-comparison-mobile.jpg`. The initial screenshot
  `/tmp/noms-020-graph-recovery-mobile.jpg` shows only the upper portion of the
  comparison section and is not evidence of the error message itself.
- One child-fork expansion failed with safe copy; retry loaded the existing
  synthetic child without a content write. Normal sign-out and guest graph
  retry showed generic unavailable feedback and zero private title/nodes.

No destructive action or content write. Subsequent responsive fixture additions
are separate acceptance work, not graph mutations. Interception/viewport were
cleared; temporary cache override remains for later checks this turn. Local
commit/fast-forward only, user-owned CSS unchanged. Real-device touch, actual
screen-reader operation and whole-app contrast remain open.

**Next:** populated responsive routes, cross-route/permission presentation and
remaining editor interactions; keep destructive checks separately gated.

## Initial recipe and collection detail recovery — 2026-10-01

Implementation `1f885b1` repairs a gap discovered during the permission sweep:
current-recipe and collection-detail failures previously had no in-page retry.
Both now show a safe level-one unavailable heading, shared load-recovery
feedback and a live pending guard. The last failure remains mounted during a
retry, preserving keyboard focus. A successful retry focuses the recipe content
or collection heading. Missing, private and transport failures remain
indistinguishable; server authorization and route-keyed lifetimes are unchanged.

Four new tests cover actual SSR failure/pending output with router context and
resource/handler/focus wiring. Tests-first missing-helper failures are recorded
in `/tmp/noms-020-detail-load-red.log`; test scaffolding was corrected to provide
the router required by real Links. Final full CI: **909 passed, 0 failed,
12 ignored**, exit 0, eight existing allowed audit warnings. Command used the
integration target cache, `SQLX_OFFLINE=true`, and
`PGOPTIONS='-c log_min_error_statement=panic' just ci`. Log:
`/tmp/noms-020-detail-load-final-ci.log`.

The grants test initially stalled because expected rejection statements filled
its temporary PostgreSQL logging pipe (backend wait channel
`anon_pipe_write.cold`). Only this turn's identified stalled jobs were stopped.
The session-only psql logging override prevents logging those long statements;
it does not skip the grants test, change app database settings or weaken its
assertions. The final complete gate passed, including that regression test.

Browser on port 32768:

- Collection root initial read and first retry failed before the server.
  Rapid Enter/Space during a held retry produced one collection API request;
  focus remained on the busy Retrying action. A real retry restored the populated
  collection and focused `collection-detail-content`.
- Guest public-recipe initial read and first retry failed before the server.
  The same duplicate-key check produced one public-recipe API request. Real
  retry restored the recipe and focused `recipe-detail-content`, with no alert.
  This isolates the public read, not the signed-in fallback sequence.
- Both retry flows fit at 320px (client and document scroll widths 305px).
  Private guest routes retained generic unavailable states with no title or
  editable controls. Screenshots:
  `/tmp/noms-020-collection-initial-recovery-phone.jpg` and
  `/tmp/noms-020-recipe-initial-recovery-phone.jpg`.

Explicit second-pass self-review **PASS for this scope**, no independent agent.
Local commit/fast-forward only; existing design-worktree CSS preserved. These
are read-recovery checks, not destructive or post-commit acceptance.

## Populated light responsive and permission sweep — 2026-10-01

The existing approved local deployment remains `noms-c799915fcf93e034`, app port
32768; its Postgres dev-environment label was verified before each fixture
setup. The unrelated port-32773 deployment was not changed.

Data setup was insert-only and transactional:

- Reused `scripts/dev-fixtures/noms-020-public-profile.sql`: a clearly synthetic
  profile with 25 public recipes/collections plus one private and one unlisted
  of each. No OAuth identity was added.
- Added `scripts/dev-fixtures/noms-020-responsive-collections.sql`: private
  parent/child collections, the original recipe-action fixture in both, and
  the history root in the child. One newly seeded public collection contains
  all 25 public fixture recipes. The script takes an existing `fixture_owner`,
  validates prerequisite ownership/visibility/names and refuses ID collisions.
  Initial ParadeDB ambiguous text predicates rolled back; selected text is now
  compared outside SQL predicates. Successful replay left counts unchanged.
- Prepared `scripts/dev-fixtures/noms-020-disposable-recipe.sql`: one separate
  private recipe and one snapshot for future final Delete acceptance. It refuses
  collection, draft or fork dependencies. Replay is idempotent. Creation is
  **not deletion approval**; no final Delete action has been performed.
  Target: `b3b3cf67-6c74-2764-9148-d4d690b9d16f`,
  “CP8 Disposable recipe – Local acceptance only”.

Before the disposable recipe, counts were 2 users, 30 recipes, 62 versions,
1 draft, 29 collections and 28 memberships. After preparation: **2 users,
31 recipes, 63 versions, 1 draft, 29 collections, 28 memberships, 1 OAuth
account**. The original mock owner's display name remains empty, bio/avatar
NULL, and the original draft remains revision 0. Normal mock sign-in/sign-out
was used; no existing recipe, draft or profile content was edited.

Geometry/basic semantics: **75 route/viewport combinations** at 390×900,
820×1180 and 1280×800. Each completed with document client width equal to scroll
width, no missing image alt attributes and no unnamed visible main form inputs.
The owner sweep additionally checked unnamed visible main buttons (none).
Owner captures were retaken after the asynchronous content markers loaded;
early loading captures were replaced, not claimed as populated acceptance.

- Guest: landing, `/home` sign-in gate, About, How to, sign-in, Explore,
  populated public profile/collection/current recipe/snapshot/graph and 404.
  The guest `/home` capture is explicitly not the community layout.
- Owner: actual Community Home, Dashboard with recipes/draft/collection tree,
  Collections, private parent and child, collection editor/new form, existing
  draft editor/new recipe form, own profile, profile/account settings and
  populated own-recipe Search. Opening forms did not save or publish anything.
- Main-agent visual inspection of phone/tablet public sheets and owner
  phone/tablet/desktop sheets found no overlaps in the captured views. Geometry
  covers the document width, but viewport screenshots are not a visual review
  of every below-fold section or every overlay state.

Permission/read checks:

- Eleven guest-gated workspace/edit/settings/search routes showed sign-in and
  preserved their exact return path, including the Search query string.
- Guest private current recipe, historical snapshot, private parent/child and
  synthetic profile's private recipe/collection revealed no private titles or
  editable controls. Signed-in non-owner private reads also remained denied;
  direct public recipe/collection editor URLs showed safe unavailable forms.
- Direct unlisted recipe/collection links worked for guest and signed-in
  non-owner without owner edit controls. The complete public-profile catalog
  paged to 25 recipes and 25 collections and excluded private/unlisted items
  26/27. These are presentation/permission observations, not a security audit.

Keyboard scope:

- Dashboard collection tree expanded to its nested child. The picker displayed
  both existing memberships as checked/disabled, with breadcrumb-qualified
  names and zero new selections. Escape restored Add to Collection focus;
  no membership write was submitted.
- Navigation used a native modal dialog (`dialog:modal`, `aria-modal=true`).
  Shift-Tab wrapped Close to the final theme control; Tab wrapped back to Close.
  Escape restored the menu trigger. Native dialog semantics, not only explicit
  `[role=dialog]` selectors, were checked.
- Recipe Notes navigation changed the hash and focused its heading. The parent
  selector for the collection root excluded itself and its child (no cycles).
  These observations do not substitute for actual screen-reader operation.

Temporary evidence:
`/tmp/noms-020-responsive-acceptance.json`,
`/tmp/noms-020-permission-acceptance.json`,
`/tmp/noms-020-public-phone-review.png`,
`/tmp/noms-020-public-tablet-review.png`,
`/tmp/noms-020-owner-phone-review.png`,
`/tmp/noms-020-owner-tablet-review.png`,
`/tmp/noms-020-owner-desktop-review.png`,
`/tmp/noms-020-nested-picker-phone.jpg`. Individual route screenshots use
`/tmp/noms-020-acceptance-<route>-<width>.jpg`.

## Semantic text contrast follow-up — 2026-10-01

Implementation `f854042` separates text ink from decorative fills. Inspection
found the editor attribution link still using light coral as 16px text.
Global links, hovered text links, selected Search tabs/chips, small status copy,
graph descendant badges and inline error/success messages now use semantic ink
roles. Existing coral/green fills, strokes, gradients, shadows, fonts and artwork
are unchanged. Text-link hover retains ink and provides underline feedback;
contained buttons/navbar links are excluded from that new underline rule.

Two tests were added before implementation. Foundation suite **18 passed,
0 failed**; final `just ci` **911 passed, 0 failed, 12 ignored**, exit 0, eight
existing allowed audit warnings. Same target/SQLX/temporary psql logging setup
as above. Logs: `/tmp/noms-020-text-ink-red.log`,
`/tmp/noms-020-text-ink-hover-red.log`,
`/tmp/noms-020-text-ink-focused.log`,
`/tmp/noms-020-text-ink-final-ci.log`.

Token tests require at least 4.5:1 for accent/error/success/warning inks against
both themes' base surfaces and each gradient stop. They reject decorative fill
tokens in shared text declarations. Existing brand/button contracts still pass.
This is a bounded contrast contract, not a complete composited-pixel WCAG audit.
Browser confirmed attribution ink `rgb(149,67,47)` while Publish retained coral
fill `rgb(217,115,90)` and dark on-accent text. About rendered the same palette
without overflow. Screenshots:
`/tmp/noms-020-editor-final-contrast-desktop.jpg`,
`/tmp/noms-020-contrast-about-desktop.jpg`.

Browser developer-media checks on About: reduced motion stopped background
animation and smooth scrolling; forced colours removed shadows and provided
high-contrast control borders/text. Both emulations were cleared immediately.
Evidence: `/tmp/noms-020-media-acceptance.json`,
`/tmp/noms-020-forced-colors-about.jpg`. This is an About/foundation smoke check,
not a claim of system-wide assistive-technology acceptance.

Second-pass self-review **PASS for these scoped changes**, no independent-agent
review. Existing checkpoint task worktree and design integration worktree reused
under the established workflow. Local semantic commits/fast-forwards only; no
push, PR or production deployment. The integration worktree's user-owned
`assets/tailwind.css` SHA-256 remains
`63fb925365998338ce8dec1e725c097e5648f820c9674767880e910db80d7e46`.

**Gates remaining at the end of this preceding slice:** final recipe Delete browser outcomes need action-time
confirmation for the separate disposable target. Settings unlink/avatar removal/
account deletion need a separately approved expendable identity and media, not
the sole-provider/no-avatar original mock owner. Post-commit lost-response and
navigation/identity replacement outcomes are still unaccepted. Actual screen
reader, real-device touch and pixel-level image export checks remain open.
Whole-app sign-off must not be inferred from geometry, token contrast or passing
unit tests. Dark-mode redesign and favicon/single-colour/3D deliverables remain
explicitly separate/deferred work.

## Approved disposable recipe Delete acceptance — 2026-10-01

The user explicitly approved testing the final confirmation previously shown
for “CP8 Disposable recipe – Local acceptance only”,
`b3b3cf67-6c74-2764-9148-d4d690b9d16f`. That approval was used only for this
isolated local recipe and its one snapshot, on app port 32768. Before acting,
the exact Postgres container's dev-environment label, recipe owner/title/private
visibility, one snapshot and absence of collection/draft/fork dependencies were
verified. No original acceptance recipe or account was used as a deletion target.

Browser results:

- The first final confirmation was held before reaching the server. Its request
  body contained only the approved recipe ID. The modal announced “Deleting
  recipe…”, exposed busy/aria-disabled controls and retained focus inside the
  native modal. Repeated keyboard activation and Escape neither submitted a
  second request nor dismissed the modal.
- Failing that held request produced safe uncertain-outcome feedback, a separate
  Dashboard link and default-safe Cancel focus. A second retry was held and
  failed in the same way. Native keyboard input and forced pointer clicks on
  the pending Cancel/Deleting controls produced no extra request or dismissal.
  The initial locator-level attempt to press an aria-disabled control was
  refused by the browser driver; actual native input and forced-click guard
  checks, not that refusal alone, provide the runtime evidence.
- At 320×900, document client and scroll width were both 320px; the failure
  panel fit from x=8 to x=312 and all feedback/actions remained readable.
- The first un-intercepted retry encountered a naturally expired mock session
  after the pause between turns. It did not delete the recipe and displayed
  “Sign in again in another tab…”. The same existing `testuser` mock account
  was signed in through normal Google mock OAuth in a separate tab, without
  accepting terms or creating/linking an account. Database readback confirmed
  the recipe and snapshot still existed before retrying.
- Retrying the existing confirmation after reauthentication succeeded and
  navigated to `/dashboard`. The populated Dashboard showed the original three
  saved recipes and revision-0 draft, no disposable-recipe link, no alert and
  no open modal. Focus was BODY after full-document navigation; this is not a
  claim of focused-heading or screen-reader success announcement acceptance.

Database read-only verification: the approved recipe and its versions each
went from one row to zero. Totals went from **31 recipes/63 versions** to
**30 recipes/62 versions**. Collections (29), memberships (28), drafts (1) and
users (2) were unchanged. Ordered row-JSON hashes for every non-target recipe
and version, plus all collections, memberships, drafts and users matched
exactly before/after the successful retry. Routine OAuth/session refresh state
is excluded from this content comparison. The deletion is permanent through
the app; the clearly synthetic fixture can be recreated with its tracked
insert-only setup script, which does not authorize another Delete action.

Focused regression command:

```sh
SQLX_OFFLINE=true \
CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target \
cargo test --features server deletion
```

Result: **14 passed, 0 failed, 0 ignored**, 909 filtered out. This includes
recipe confirmation/live-claim guards, shared account-deletion state tests and
database lineage/privacy regression tests; it does not prove destructive
Settings browser outcomes. An initial `--lib` invocation was rejected because
this package has no library target; a source-filename filter matched zero tests.
Neither is counted as a passing test run. No runtime code changed in this
acceptance slice, so full CI was not rerun; the previously recorded complete
gate remains 911 passed/12 ignored.

Temporary screenshots:
`/tmp/noms-020-delete-pending.jpg`,
`/tmp/noms-020-delete-failure.jpg`,
`/tmp/noms-020-delete-failure-phone.jpg`,
`/tmp/noms-020-delete-session-expired.jpg`,
`/tmp/noms-020-delete-success-dashboard.jpg`.
Temporary viewport and request interception overrides were cleared; the
separate mock sign-in tab was closed. The existing task/design worktrees were
reused under the established workflow. Explicit second-pass self-review
**PASS for this scope**; no independent agent, push or production deployment.
A single documentation commit is appropriate because this slice adds only
one acceptance record and its matching master-plan checkbox.

**Next gates remain open:** separately approved expendable Settings identity/
media for unlink, avatar removal and account deletion; post-commit lost-response,
navigation-failure and identity-replacement recovery; pixel-level image export,
actual screen-reader/real-device touch and final whole-app visual/behavior
acceptance. The safe failure text and pre-server tests do not prove a committed
write whose response is lost. This scoped Delete pass is not checkpoint-8
sign-off or authorization to delete another fixture/account.

## Disposable Settings fixture preparation — 2026-10-01

The user approved preparation of a separate expendable Settings identity and
avatar. This is setup evidence, not approval or acceptance of the subsequent
final unlink/removal/account-deletion actions. Only the existing local project
`noms-c799915fcf93e034` was used: app **32768**, Postgres **32769**, MinIO public
base `http://127.0.0.1:32770/noms-media`. The exact database container's
`com.noms.dev-environment=true` label was reverified; no production deployment
or unrelated local instance was changed.

Tracked sources:
[insert-only Settings fixture](../../scripts/dev-fixtures/noms-020-settings-disposable.sql)
and [synthetic avatar](../../scripts/dev-fixtures/noms-020-settings-avatar.svg).
Namespace: `NOMS020 CP8 Settings disposable 2026-10-01`.

- User `5c4c58f4-bc23-6fbf-e2e0-3c4302d93e56`: username `cp8settings`, display
  name `CP8 Settings – Disposable`, email `cp8settings@example.invalid`.
  It owns **zero recipes, drafts and collections**; it is not a real person.
- Google identity `c66405cf-fd75-e244-44be-364ac563693a`, mock subject
  `cp8settingsdisposable`; GitHub identity
  `bff65377-fec8-e9ad-2b92-cd3589fba3d7`, subject `424242` from the repository's
  fixed mock GitHub user-info response. No credentials/provider tokens seeded.
- Avatar asset `aaa5ec53-62dd-fb06-a4b0-47503257f862`: retained managed avatar
  owned by this user, one `avatar_media_refs` row, no attachment capability or
  expiry. Legacy avatar fields remain NULL. Canonical objects under
  `avatars/5c4c58f4-bc23-6fbf-e2e0-3c4302d93e56/` are
  `aaa5ec53-62dd-fb06-a4b0-47503257f862-original` (512×512 PNG, 24,704 bytes),
  `aaa5ec53-62dd-fb06-a4b0-47503257f862.webp` (256×256 WebP, 2,620 bytes) and
  `aaa5ec53-62dd-fb06-a4b0-47503257f862-thumb.webp` (96×96 WebP, 1,114 bytes).
  The exact namespace was empty before uploading these three objects; original
  MIME is `image/png`, derivative MIME is `image/webp`. No objects overwritten.

Reproduce rendering from the task worktree with the existing ImageMagick:

```sh
magick scripts/dev-fixtures/noms-020-settings-avatar.svg /tmp/noms-020-settings-avatar-original.png
magick /tmp/noms-020-settings-avatar-original.png -resize 256x256 /tmp/noms-020-settings-avatar-full.webp
magick /tmp/noms-020-settings-avatar-original.png -resize 96x96 /tmp/noms-020-settings-avatar-thumb.webp
magick identify /tmp/noms-020-settings-avatar-original.png /tmp/noms-020-settings-avatar-full.webp /tmp/noms-020-settings-avatar-thumb.webp
```

For a newly approved local instance, verify its label and empty fixture/object
namespace first. Copy those generated files into its MinIO container and use
the existing `mc cp --attr Content-Type=...` with the three exact keys above,
using only that container's local credentials (never print them). Then run:

```sh
docker exec -i noms-c799915fcf93e034-postgres-1 \
  psql -U noms -d noms -X \
  -v fixture_media_base=http://127.0.0.1:32770/noms-media \
  -v fixture_allow_create=true \
  < scripts/dev-fixtures/noms-020-settings-disposable.sql
```

Initial creation and intact replay succeeded transactionally. The subsequent
guard refinement makes ordinary replay default to `fixture_allow_create=false`:
omit the create flag when validating this existing fixture. That replay passed
without changing counts; a non-local media base and an absent namespace each
failed with expected errors/exit 3 before insertion. The absent test substituted
only the namespace with `NOMS020 CP8 Settings absent-guard 2026-10-01` in the
input stream; no new fixture was created. Partially consumed fixtures fail
validation instead of resetting removed providers/avatar state. After complete
deletion, ordinary replay refuses recreation. Do not deliberately opt into
recreation/re-upload after consumption without fresh approval.

Totals after setup: **3 users, 3 OAuth identities, 1 managed media asset,
1 avatar reference, 30 recipes, 62 versions, 1 draft, 29 collections and
28 memberships**. Only one user, two identities, one asset/reference and the
three synthetic image objects were added. Ordered `row_to_json` hashes using
`string_agg(..., '|' ORDER BY id)` (membership order: collection/recipe ID)
matched before/after for all recipes, versions, collections, memberships and
drafts, plus all non-fixture user/OAuth rows. This comparison was repeated after
mock login and confirmation staging. Routine fixture OAuth/session updates are
excluded; original account rows are not.

Browser: normal Google mock login returned the existing fixture identity,
without account creation/terms acceptance. Profile fields/public-profile link
identify `@cp8settings`; the synthetic avatar loaded at natural width 256.
Linked accounts showed both Google and GitHub. GitHub login itself was not
tested. The GitHub unlink modal is staged with default Cancel focus, avatar
Remove photo is available in a separate profile tab, and account deletion's
non-final steps reached the final modal with Go Back focus. No final Unlink,
Remove photo, Save changes or Delete My Account action was submitted.
Temporary browser overrides are clear. Screenshots inspected:
`/tmp/noms-020-settings-avatar-ready.jpg`,
`/tmp/noms-020-settings-unlink-ready.jpg`,
`/tmp/noms-020-settings-delete-ready.jpg`.

No runtime code changed. Validation consisted of SQL creation/replay and
negative guards, exact row-hash isolation, image dimensions and browser setup
checks; full CI was not rerun (prior complete gate: 911 passed/12 ignored).
Explicit second-pass self-review **PASS for fixture preparation**; it corrected
the initial script's full-deletion replay ambiguity with the opt-in create guard.
No independent-agent review claimed. The established checkpoint task and design
integration worktrees are reused. One local semantic commit is appropriate for
this indivisible fixture/preparation-evidence slice; no push or deployment.

**Next:** obtain action-time confirmation for the three scoped mutation tests
on this fixture, including disposal of its synthetic avatar objects. Settings
mutation outcomes, post-commit lost-response/identity/navigation recovery and
the other whole-app/accessibility/image-export gates remain open.

## Approved disposable Settings mutation acceptance — 2026-10-01

The user's action-time **“yes proceed”** approved unlinking GitHub, removing
the avatar and permanently deleting only `@cp8settings`
(`5c4c58f4-bc23-6fbf-e2e0-3c4302d93e56`) and its three synthetic avatar files.
Target: existing local project `noms-c799915fcf93e034`, app **32768**, PostgreSQL
**32769**, MinIO **32770**. This is not approval to change the original mock
account, create/relink another identity or mutate production/other deployments.

Following the computer restart, the exact stopped project's services were
restarted. A transient PostgreSQL port collision was resolved by starting the
app before PostgreSQL; the app's original Compose network/alias was restored
after a failed start left it disconnected. No volume/data was recreated or
deleted and no network recipient or permission scope was expanded. The app was
rebuilt after the focus fix; all UI testing used this deployment.

### Browser outcomes and focus correction

- **GitHub unlink:** held the exact fixture identity's XHR before server
  execution. Pending status remained visible; forced duplicate submit, Cancel
  and Escape produced no extra API request or dismissal. Two failed attempts
  retained the confirmation and safe uncertain-outcome/check-account guidance.
  A fresh account page and database confirmed both providers before the real
  retry. Real success removed GitHub, refreshed Google as the sole provider and
  displayed success. Forced pointer/Return on the remaining disabled Unlink
  produced zero requests. Only identity
  `bff65377-fec8-e9ad-2b92-cd3589fba3d7` was unlinked.
- **Found/fixed success focus:** real unlink success left focus on `BODY`
  because its initiating provider button disappeared. Commit `e916455` focuses
  the mounted `#unlink-account-success` result and similarly focuses
  `#profile-save-success` only after avatar removal, not ordinary profile saves.
  A source-contract regression test failed before implementation (missing
  unlink-result ID) and passed after it. Since GitHub was already consumed,
  after-fix unlink focus was verified using an explicitly **UI-only replay**:
  fulfill one intercepted provider-list read with the fixture's former GitHub
  card, then fulfill its intercepted unlink with unit JSON `null`. No unlink
  request reached the server and no identity was recreated/relinked. The
  subsequent real provider refresh returned Google only and focus was on
  `#unlink-account-success`. This is not a second real unlink success.
- **Avatar removal:** two pre-server failures retained the actual avatar and
  safe uncertain-outcome guidance. During pending removal, Edit/Change were
  unavailable, Remove remained focusable and forced duplicate click/Return
  emitted no extra request. A fresh profile confirmed the avatar was intact
  before the actual retry. After the fix, real removal displayed **“Avatar
  removed successfully!”**, replaced the image with initials/Choose Photo and
  focused `#profile-save-success`. The durable avatar reference became zero.
- **Account deletion:** default Cancel, identity-specific phrase
  `delete cp8settings` and final Go Back focus were verified. Before the final
  real action, database reads reconfirmed zero owned recipes/drafts/collections.
  Held pending submission showed Deleting feedback; forced final/Back clicks
  and Escape produced no second request or dismissal. Two pre-server failures
  kept the final confirmation open, restored Go Back focus and advised checking
  sign-in in another tab without promising rollback. A fresh profile still
  identified the disposable account. The real retry deleted it, revoked its
  sessions and navigated to `/` with Sign In visible and no account menu.

All injected failures above happened **before server execution**, not after a
committed operation with a lost response. Interception was limited to Fetch/XHR
API patterns and cleared before each real retry; temporary viewport overrides
were also cleared. No new GitHub login, profile save, Terms acceptance or
original-account mutation was performed.

Inspected temporary evidence: `/tmp/noms-020-settings-unlink-pending.jpg`,
`/tmp/noms-020-settings-unlink-failure.jpg`,
`/tmp/noms-020-settings-unlink-failure-phone.jpg` (usable 320×875 capture),
`/tmp/noms-020-settings-unlink-success.jpg` (real, before focus fix),
`/tmp/noms-020-settings-unlink-focus-fixed.jpg` (controlled replay),
`/tmp/noms-020-settings-avatar-pending.jpg`,
`/tmp/noms-020-settings-avatar-failure.jpg`,
`/tmp/noms-020-settings-avatar-success.jpg`,
`/tmp/noms-020-settings-delete-pending.jpg`,
`/tmp/noms-020-settings-delete-failure.jpg` and
`/tmp/noms-020-settings-delete-success.jpg`. Delete feedback geometry measured
`innerWidth = clientWidth = scrollWidth = 320`. Its raw narrow capture was
blank and the avatar narrow capture was incorrectly scaled; neither is counted
as visual phone acceptance. No actual touch/screen-reader acceptance claimed.

### Exact disposal and original-data isolation

The original user/account and all original content remain unchanged. Ordered
row-JSON MD5 hashes (delimiter `|`; membership order collection/recipe ID)
matched the pre-action baseline exactly after deletion:

| Rows | Before = after |
| --- | --- |
| Recipes | `d7298c2c29ade897d2e39535462d7db3` |
| Versions | `0f91d540a069a9ae0ab0495253a3f4ae` |
| Collections | `917c84e8447402c9b3bfcb6e3d7b8075` |
| Memberships | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Drafts | `eeb6a9cba9018a65ee8e13b4bf625d52` |
| Non-fixture users | `3af5cd3524ea80707ea54f1961ef1864` |
| Non-fixture OAuth | `522c33b63155f208e47f4146c8bcd0bf` |

Final totals: **2 users, 1 OAuth identity, 1 media asset, 0 avatar references,
30 recipes, 62 versions, 1 draft, 29 collections, 28 memberships**. The target
user, its OAuth rows and sessions are absent. Ordinary replay without
`fixture_allow_create=true` fails with exit 3 and **“Settings fixture absent;
replay must not recreate a deleted account”**; no account was recreated.

The hourly orphan worker had not run following avatar removal/deletion.
Within the explicitly approved disposal scope, `mc rm` deleted the three exact
fixture keys listed in the preparation section, with no recursive/glob delete;
`mc ls` of that exact prefix returned empty. No other objects were removed.
This **direct test cleanup is not background-worker runtime evidence**. The
service-owned media record remains `retained`, with NULL uploader and no avatar
reference, awaiting normal worker reconciliation. Its retention is intentional
durable cleanup authority, not a retained avatar shown to users. Product-normal
recovery cannot restore the deleted account/provider or the disposed objects.

### Validation, review and remaining gates

Established task branch/worktree `codex/NOMS-020-checkpoint-8` was reused; code
commit `e916455` was fast-forwarded into `codex/NOMS-020-visual-design-refinement`.
User-owned `assets/tailwind.css` stayed unchanged at SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
Separate code and acceptance-evidence commits keep concerns reviewable; no push.

Validation from the task worktree used `SQLX_OFFLINE=true` and shared
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`:

```sh
cargo test --features server successful_removals_focus_a_mounted_result -- --test-threads=1
cargo test --features server pages::settings -- --test-threads=4
PGOPTIONS='-c log_min_error_statement=panic' just ci
```

Red regression run: 1 expected failure before implementation. Focused Settings:
**33 passed**. Final full CI: **912 passed, 0 failed, 12 ignored**, formatting,
checks/Clippy, schema/security policy, dependency audit (8 existing allowed
warnings), tracked-secret and deployment-artifact gates passed. The first CI
attempt passed tests but failed advisory-cache lock access in the sandbox;
rerunning with approved cache access passed. Logs:
`/tmp/noms-020-settings-success-focus-ci.log` and
`/tmp/noms-020-settings-success-focus-final-ci.log`.

Explicit second-pass self-review **PASS for this scoped mutation/focus slice**:
reviewed code/test diff, exact approval/targets, real versus simulated outcomes,
original-data hashes and capture limitations. No independent-agent review.
Post-commit lost responses, identity replacement, account-deletion navigation
failure, observed automatic media cleanup, usable remaining narrow captures,
actual screen-reader/real-device touch, image-export outcomes and final
whole-app visual/behavior acceptance remain open. This is not checkpoint-8 or
WCAG sign-off.

## Avatar export pixel acceptance — 2026-10-01

Scope: verify the actual browser-generated avatar bytes, framing, colour and
alpha without changing a saved account. Reused local project
`noms-c799915fcf93e034` (app **32768**, PostgreSQL **32769**, MinIO **32770**) and
the established checkpoint task/design integration worktrees. No new fixture
identity, account link, destructive action or persisted image was needed.

The original mock owner signed in through the normal mock flow. A synthetic
colour-grid PNG was staged using Choose Photo, with only the exact
`/api/upload_avatar*` Fetch/XHR request paused at **Request**, before server
execution. Each capture saved `edit`, `original_bytes`, `full_bytes` and
`thumb_bytes` under `/tmp`, then failed the held request. Nothing was uploaded
successfully. Retry feedback is expected in this harness, not a server failure.
Afterwards, interception and the temporary cache override were cleared and
reload discarded the staged photo; Choose Photo/initials remained, with no
Retry action or saved avatar. Original account fields were never submitted.

### Findings and implementation

- **Transform/crop order:** the exporter cropped the source first and rotated
  that fragment around the output centre, while the preview transforms the
  full image around its original centre inside a fixed crop. A 50% offset crop
  with 90° rotation and horizontal flip exported a different quadrant.
  `496cc16` maps that fixed crop to the output canvas, preserves the original
  image centre/dimensions and draws the transformed full image. The circular
  clip, colour processing, WebP sizes/quality and server contract are unchanged.
  Two geometry tests were written first; their initial run failed because the
  helper was not yet implemented. Focused Settings tests then passed **35/35**.
- **Preview colour space:** SVG defaults to linearRGB, whereas avatar pixel
  processing uses sRGB channels. At saturation 50%, red contrast 50%, green
  120%, blue 150%, an interior preview sample `(122,79,72)` versus output
  `(89,74,68)` showed an RGB error of 33. `8c706e5` explicitly uses sRGB for
  elliptical avatar previews, preserving linearRGB for rectangular recipe
  previews/non-destructive edits. A new source-contract test initially selected
  its own assertion string, failing even with correct production code;
  `5f667ad` confines it to the rendered ImageWithCrop filter. This test-selector
  correction is not a successful test-first red/green claim. Its focused run
  passed **1/1**; runtime colour evidence is recorded below.

The first after-fix byte capture was identical to the old result: the dev app's
stable WASM URL remained cached after restart. It is **not counted as a pass**.
A tab-local cache override plus reload loaded the corrected client; both actual
pixel checks and framing then passed. That override has been restored. Only
the existing app container was restarted for rebuilds; data volumes and other
deployments were untouched.

### Independent pixel oracle and results

Tracked source: `scripts/dev-fixtures/image-export-grid.svg`, final 512×512
four-colour grid with corner markers. Black grid lines in the initial fixture
were removed to avoid lossy thumbnail boundary halos. A read-only stdlib Python
checker, `scripts/tests/check-avatar-export.py`, uses the existing ImageMagick
CLI to decode local files, independently invert the full-image CSS transform
and crop, and calculate contrast/saturation channels. It checks **400×400** and
**100×100**, all four transparent corners, opaque unambiguous interior points
and maximum channel error **15** for lossy WebP. It requires at least eight
valid samples. Boundary exclusion uses an inverse-transformed four-output-pixel
footprint, including rotation/stretch; the tolerance was not relaxed. This is
sampled interior evidence, not exhaustive alpha/antialiasing edge equality.

The final checker still rejects the before-fix combined capture: pixel
`116,104`, actual `(140,126,56)` versus expected `(220,200,40)`, error **80**.
Observed successful captures:

| Case | Edits/input | Full samples / max RGB error | Thumbnail samples / max RGB error |
| --- | --- | --- | --- |
| `combined-fixed` | Initial grid; crop x=.1328125, y=.318359375, w=h=.5; 90°, flip X | 148 / 2 | 134 / 7 |
| `color-final` | Final grid; full crop; saturation .5, contrast R=.5/G=1.2/B=1.5 | 364 / 1 | 275 / 4 |
| `stretch-final` | Final grid; crop x=.1640625, y=.095703125, w=h=.75; 45°, width .8/height .6, flip Y | 343 / 3 | 178 / 14 |
| `portrait-final` | 512×1024; default square crop x=0, y=.25, w=1, h=.5 | 356 / 1 | 303 / 4 |
| `landscape-final` | 1024×512; crop x=.4140625, y=.25, w=.25, h=.5; 90°, both flips | 337 / 2 | 291 / 5 |

Every row passed dimensions, transparent-corner and interior-alpha checks.
Captured original bytes are untouched: source/capture SHA-256 pairs matched
for the original grid, final grid, portrait and landscape inputs. The final
grid pair is `90a59f1922d5839dab719ca84ae945c81225b5f186077f8f578e5f44f4d0b9f2`.

After the colour-space fix, the actual DOM filter reads
`color-interpolation-filters="sRGB"`. Four stable screenshot/output samples:

| Quadrant | Preview RGB | Export RGB |
| --- | --- | --- |
| Upper left | 89,76,70 | 89,74,68 |
| Upper right | 94,192,145 | 93,192,145 |
| Lower left | 57,94,174 | 57,94,173 |
| Lower right | 155,220,130 | 156,221,130 |

Maximum difference is **2** channel levels, including screenshot/WebP loss.
The inspected preview captures are
`/tmp/noms-020-avatar-export-color-final-preview.jpg` and
`/tmp/noms-020-avatar-export-landscape-final-preview.jpg`.
Other captures use the same case prefixes above. Files are temporary, not
tracked artwork. The initial lined fixture is retained only in its `/tmp`
snapshot for the red/control capture; use each capture's exact source file.

Reproduce the final synthetic sources and validate a captured case:

```sh
magick scripts/dev-fixtures/image-export-grid.svg /tmp/noms-020-export-solid-grid.png
magick /tmp/noms-020-export-solid-grid.png -resize '512x1024!' /tmp/noms-020-export-portrait-grid.png
magick /tmp/noms-020-export-solid-grid.png -resize '1024x512!' /tmp/noms-020-export-landscape-grid.png
# Through the visible editor, stage edits; pause the avatar API before execution,
# save its JSON byte arrays/metadata locally, and fail the held request.
python3 scripts/tests/check-avatar-export.py \
  /tmp/noms-020-export-solid-grid.png \
  /tmp/noms-020-avatar-export-color-final-edit.json \
  /tmp/noms-020-avatar-export-color-final-full_bytes.webp \
  /tmp/noms-020-avatar-export-color-final-thumb_bytes.webp
```

### Data isolation, validation and review

Read-only post-check row hashes exactly match the prior approved-disposal
baseline: recipes `d7298c2c29ade897d2e39535462d7db3`, versions
`0f91d540a069a9ae0ab0495253a3f4ae`, collections
`917c84e8447402c9b3bfcb6e3d7b8075`, memberships
`fd3bc9401f1389c7fa08cff816e7b44b`, drafts
`eeb6a9cba9018a65ee8e13b4bf625d52`, users
`3af5cd3524ea80707ea54f1961ef1864`. Counts remain 30 recipes, 62 versions,
29 collections, 28 memberships, 1 draft, 2 users, 1 OAuth identity, 1 media
asset and **0 avatar references**. Routine original mock sign-in refreshes OAuth
usage/session state; an unchanged full OAuth-row hash is not claimed.

A later read now observes the former disposable avatar media record as
`deleted`, with NULL uploader and no reference. It had been `retained` in the
previous section. Its three objects were already manually disposed within that
earlier approval; this metadata reconciliation is **not evidence that a worker
physically deleted existing objects**. No media was created in this slice.

Validation from the task worktree used `SQLX_OFFLINE=true` and the established
shared `CARGO_TARGET_DIR`:

```sh
cargo test --features server avatar_canvas -- --test-threads=1
cargo test --features server pages::settings -- --test-threads=4
cargo test --features server avatar_preview_uses_export_srgb -- --test-threads=1
cargo check --target wasm32-unknown-unknown --no-default-features --features web
PGOPTIONS='-c log_min_error_statement=panic' RUST_TEST_THREADS=4 just ci
```

Final CI: **915 passed, 0 failed, 12 ignored**, 6 security-policy script checks,
format/native/WASM/Clippy/schema checks, dependency audit (8 existing allowed
warnings), tracked-secret and deployment-artifact gates passed.
`/tmp/noms-020-avatar-export-bounded-ci.log` is the final raw log. The initial
framing-only full run passed 914 tests; a later run failed the new test selector
described above. After that correction, the unbounded run hit the existing
temporary database upload-test `PoolTimedOut`; the repeat passed without
disabling or weakening tests. **Concurrency correction recorded in the next
slice:** although the command set `RUST_TEST_THREADS=4`, `just ci` explicitly
passes `--test-threads=8`, overriding that environment variable. The raw log
confirms eight threads; the previous four-thread explanation was incorrect.
Earlier logs remain under the
`/tmp/noms-020-avatar-export-*ci.log` prefixes. The final Python checker was
rerun on all five captures and still rejects the old framing result.

Explicit second-pass self-review **PASS for this sampled avatar slice**:
reviewed transform order/centre, circular mask, colour-space scope, regression
tests, checker independence/threshold, before-fix rejection, actual captured
pixels, original-data hashes and cleanup. No independent-agent review claimed.
Code commits `496cc16`, `8c706e5`, `5f667ad` were fast-forwarded into the design
integration branch. Its user-owned `assets/tailwind.css` remains SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
The final checker/evidence update is a separate local acceptance commit; no push.

**Next:** recipe-image variants/additional input formats and remaining dense
image/editor interactions. This does not accept persisted avatar upload/reopen
again, all image formats, pixel-perfect edges, post-commit lost responses,
navigation/identity replacement, real-device touch, actual screen readers or
whole-app visual/behaviour acceptance. Any further destructive/account-linking
tests require a new isolated fixture and exact fresh action-time approval;
the consumed `@cp8settings` account must not be recreated. Checkpoint 8 and
full WCAG sign-off remain open.

## Recipe image variants and input-format acceptance — 2026-10-01

Scope: actual client-generated recipe WebP variants and additional JPEG/WebP
avatar inputs. No runtime code changed. Reused the established checkpoint task
branch/worktree and existing port-32768 development app; fresh browser state
and synthetic sources prevent old uploads from concealing processing defects.
The original account's saved content/profile remained read-only.

### Environment recovery before testing

The app health endpoint returned 200, but its database reported recovery mode.
Read-only Docker logs showed a repeated checkpoint panic, **No space left on
device**, on Docker's separate 98 GB filesystem (host workspace had free space).
This is environment failure, not an image-processing regression.

Within the user's earlier authorization to remove oldest disposable test
deployments, resolved labels, creation time, stopped status and volume users
for the oldest NOMS deployment `noms-ca6962a21bc5bc35` (created September 27).
Removed only its stopped `noms-ca6962a21bc5bc35-app-1` container and the
4.625 GB `noms-ca6962a21bc5bc35_cargo-target` rebuildable compile-cache volume.
The cache had no other container users. This freed **5.2 GB**; the current
PostgreSQL returned to accepting connections without restarting/recreating it.
All database/media volumes, other app instances and unrelated Docker projects
were preserved. Deleted cache/container files cannot be undeleted, but the app
and compiled outputs can be recreated from source. Old deployment registration
may still list the removed app; no whole-deployment teardown is claimed.
Current database/content hashes exactly matched the previous baseline after
recovery, before any browser edits.

### Browser processing and no-write harness

Normal Google mock login used the original existing `testuser` identity, with
no account creation/linking or Terms acceptance. In a fresh `/recipes/new`
editor, intercepted Fetch/XHR requests for `upload_image*`, `create_recipe*`,
`save_recipe_draft*` and `publish_recipe_draft*` at **Request**, before server
execution. Staged synthetic title/ingredient/instruction text solely to enable
the normal Publish action and capture generated image variants. Two autosave
draft-create attempts were failed before execution. Every captured upload was
also failed before execution; no recipe, draft, media object or profile save
was submitted successfully. Publication stopped on the guarded image failures.

Source-derived synthetic inputs were selected through the actual file chooser
(multiple selection supported). All six reached Ready. First PNG edits were
crop 60%, X=40 px, rotation 45°, horizontal flip and red contrast 80%. After OK,
the gallery rendered `rotate(45.0000deg) scaleX(-1.000000)` and the edited crop.
Its captured upload retained the full unadjusted image, as expected: recipe
edits are separate metadata, unlike baked circular avatar outputs. This checks
local preview/non-destructive variants, **not a persisted edit-metadata save or
reload round trip**. Generated crop edges were not compared pixel-for-pixel.

First batch captured four pairs; second batch captured only the two newly Ready
small/transparent inputs. Former failed items were not reuploaded by the next
Publish, and still contributed to the visible six-image failure summary.
Each upload purpose was `Recipe`. Per-item Retry and safe failure copy retained
selected images; global Publish/Save/Cancel recovered after the failed batch.
No successful storage-upload or publication acceptance is claimed here.

### Recipe file/pixel results

Read-only checker `scripts/tests/check-recipe-image-export.py` (`1d80544`) uses
stdlib Python plus the existing ImageMagick CLI. It verifies RIFF/WebP headers,
integer-rounded aspect-preserving bounds (**1920** full/**800** thumbnail), no
upscaling and flat interior colours/alpha against the exact decoded source.
Lossy RGB tolerance is 15, alpha tolerance 1, minimum eight samples; a
four-output-pixel footprint excludes boundary/resampling halos. All rows had
**176 valid samples per variant**, **zero alpha error** and the following max
RGB errors:

| Input | Source size | Full size / error | Thumbnail size / error |
| --- | --- | --- | --- |
| Landscape PNG (16-bit sRGB) | 2560×1440 | 1920×1080 / 1 | 800×450 / 2 |
| Landscape JPEG | 2560×1440 | 1920×1080 / 1 | 800×450 / 1 |
| Landscape WebP | 2560×1440 | 1920×1080 / 0 | 800×450 / 1 |
| Portrait PNG (16-bit sRGB) | 1200×2400 | 960×1920 / 1 | 400×800 / 3 |
| Small opaque PNG | 512×512 | 512×512 / 2 | 512×512 / 2 |
| Small translucent PNG (alpha 128) | 512×512 | 512×512 / 2 | 512×512 / 2 |

Small full/thumbnail pairs were identical bytes (no upscaling). The checker
rejects negative controls: rotated wrong framing at pixel `76,76` produced RGB
error **39**; using an 800×450 thumbnail as a full-size variant rejected the
size mismatch against 1920×1080. These are checker controls, not product bugs.
The final checker rerun passes all six pairs. Sampled interiors do not establish
whole-image perceptual equality, every edge pixel or complete format coverage.

Generate deterministic sources from the tracked grid and check one captured
case (all output files remain under `/tmp`):

```sh
magick scripts/dev-fixtures/image-export-grid.svg -resize '2560x1440!' /tmp/noms-020-recipe-landscape.png
magick /tmp/noms-020-recipe-landscape.png -quality 90 /tmp/noms-020-recipe-landscape.jpg
magick /tmp/noms-020-recipe-landscape.png -quality 90 /tmp/noms-020-recipe-landscape.webp
magick scripts/dev-fixtures/image-export-grid.svg -resize '1200x2400!' /tmp/noms-020-recipe-portrait.png
magick scripts/dev-fixtures/image-export-grid.svg -depth 8 /tmp/noms-020-recipe-small.png
magick /tmp/noms-020-recipe-small.png -alpha set -channel A -evaluate multiply 0.5 +channel /tmp/noms-020-recipe-transparent.png
# In the visible editor, stage input; capture/fail the API before execution.
python3 scripts/tests/check-recipe-image-export.py \
  /tmp/noms-020-recipe-landscape.png \
  /tmp/noms-020-recipe-export-noms-020-recipe-landscape.png-full.webp \
  /tmp/noms-020-recipe-export-noms-020-recipe-landscape.png-thumb.webp
```

Each capture uses `/tmp/noms-020-recipe-export-<source-filename>-full.webp`
and `-thumb.webp`. Inspected whole-page screenshots:
`/tmp/noms-020-recipe-format-ready.jpg` (four Ready cards, first edited),
`/tmp/noms-020-recipe-format-failed.jpg` (four pre-server failures).
These are desktop state evidence, not new phone/tablet acceptance.

### Additional avatar formats

On Profile, staged the same 512×512 source encoded as JPEG and WebP. Replaced
the interception patterns with only `upload_avatar*` Fetch/XHR and failed each
capture before execution. JPEG used crop 60%, X=30 px, rotation 90°, flip X;
WebP used saturation 50%, red contrast 80%, full crop. Both passed the existing
independent avatar checker, circular corner transparency and interior alpha:

| Input | Full samples / RGB error | Thumbnail samples / RGB error |
| --- | --- | --- |
| JPEG | 300 / 1 | 321 / 4 |
| WebP | 325 / 1 | 240 / 6 |

Original bytes match exact source SHA-256 pairs: JPEG
`cc553aa229e611b8c6a74fb8ae7ac30ebf8fc9b7256e743c9a6227aa384eaec9`,
WebP `89ee3219991e1c9178797ae254f000269be296495edcaa70b2dd3e514043354f`.
Captures: `/tmp/noms-020-avatar-format-jpeg-*` and
`/tmp/noms-020-avatar-format-webp-*`; inspected preview:
`/tmp/noms-020-avatar-format-webp-preview.jpg`.
Interception was cleared; reload removed staged Retry photo upload and restored
unchanged initials/Choose Photo. The temporary browser tab was closed. No cache
or viewport overrides were added during this slice.

### Isolation, validation and review

Post-check hashes exactly match both the pre-check and previous slice for all
30 recipes, 62 versions, 29 collections, 28 memberships, 1 draft and 2 users.
The exact hashes are in the previous section; no saved row was altered.
Media remained one `deleted` record and zero avatar references, with no new
uploads. Normal OAuth login usage/session refresh is excluded from unchanged
account-row claims.

Focused regression command from the established task worktree, using
`SQLX_OFFLINE=true`, `PGOPTIONS='-c log_min_error_statement=panic'` and the
integration target cache:

```sh
cargo test --features server components::base::image_upload -- --test-threads=4
```

**30 passed, 0 failed**, 897 filtered out. Both Python checkers passed the actual
captures; the recipe checker's two negative controls failed as expected.
`git diff --check` passed. No new full CI was run for this checker/evidence-only
change: the last complete runtime gate is still **915 passed/12 ignored**.
Corrected its concurrency explanation above after inspecting the exact raw log
and `justfile`: explicit `--test-threads=8` overrides `RUST_TEST_THREADS=4`.
The prior pass remains valid, but it was not a four-thread mitigation.

Explicit second-pass self-review **PASS for this scoped format/variant slice**:
reviewed the no-write guard, source/variant pixel contract, original-data hashes,
negative controls, cleanup targets and limitations. No independent-agent review.
Checker and evidence are separate semantic local commits on
`codex/NOMS-020-checkpoint-8`, fast-forwarded into the design integration branch;
its user-owned CSS remains SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
No push or production deployment.

**Next:** malformed/unsupported/oversized image failure recovery, remaining
image-editor interactions and persisted edit round trips. EXIF orientation,
wide-gamut/ICC, animation, every alpha edge, collection/step-photo-specific
flows, real-device touch and actual screen-reader checks are not accepted by
these samples. Post-commit lost responses/navigation/identity replacement and
whole-app acceptance remain open; fresh destructive/account-linking checks
still need a new isolated fixture and exact action-time approval.

## Image input failure recovery — 2026-10-01

### Contract and observed failures

Continued the explicitly requested checkpoint-8 checks in the established task
worktree, against the same isolated local deployment `noms-c799915fcf93e034`
at `http://127.0.0.1:32768`. No original recipe/profile save, image upload,
account mutation or new database fixture was needed in this slice.

Acceptance: rejected files must leave the picker usable; a malformed image
must exit Processing/Uploading with safe feedback; bad recipe images must not
block subsequent valid selections or expose an upload retry for empty bytes;
desktop and narrow error messages must fit their panels.

Before the fix, a 44-byte text file named `.png` left a recipe photo indefinitely
in **Processing…**, with Publish/Cancel disabled. The broken DOM image already
reported `complete=true`, `naturalWidth=0`; the shared decoder observed only
`onload`, which never fired for this input. Confirming the same malformed avatar
left **Uploading…**/Choose Photo disabled without reaching the upload API.
A valid PNG padded to 10,485,761 bytes also opened the avatar crop editor instead
of being rejected before preview. Recipe extension/size guards already rejected
the GIF/oversized inputs, but avatars had no equivalent early guard.

Red evidence: `/tmp/noms-020-malformed-before.jpg` and
`/tmp/noms-020-malformed-avatar-before.jpg`. Synthetic files were generated under
`/tmp`: malformed PNG, empty PNG, valid unsupported GIF and oversized PNG;
the existing deterministic 512×512 test grid supplied valid recovery input.
No personal images were used.

### Changes and test-first evidence

`9325dc6` adds shared case-insensitive JPEG/PNG/WebP original-input validation:
nonempty, at most `10 * 1024 * 1024` bytes. Both recipe and avatar selection use
it before preview allocation; avatars reset the file input after rejection so
the same file can be selected again. Extensions are an early user-input check,
not proof of the content's format; actual decoding and existing server
validation remain necessary.

The shared browser decoder now checks both completed and broken-image states,
returns actionable decode errors, removes its leaked onload callback and
revokes its object URL on all result paths. Polling is bounded to 3,000 ten-ms
waits (roughly 30 foreground seconds, not a strict background wall-clock bound).
Recipe failure revokes its preview URL and closes only that image's crop modal,
not a newer selection's modal. Decode-failed items show a live alert and require
Remove/reselection; upload Retry still requires both processed byte variants.

Four new native tests cover supported formats/exact size boundary, rejected
input, avatar guard ordering and decoder source contracts. The first focused
run failed to compile because the helper did not yet exist. After implementation,
an initial native run identified an unused re-export; gating the re-export to
WASM fixed it. The final native and WASM checks passed. Source-contract tests do
not substitute for the browser decode observations below.

A later 320px browser check found the error's existing `white-space: nowrap`
expanded the document to **514px**. A new regression test failed on the shared
status's nonshrinking minimum width. `fb06df6` allows the upload row/status to
wrap and shrink, gives failed status a full row, and wraps long error tokens.
The test then passed; this changes shared `assets/main.css`, not user-owned
`assets/tailwind.css`.

### Browser acceptance and cleanup

Fresh client reloads bypassed cache after integrating/restarting the app.
Checks used the existing authenticated original owner, but only selected local
files; no recipe text was entered and autosave was not triggered.

- Avatar oversized, GIF and empty originals: clear size/format/empty errors,
  no crop modal, Choose Photo enabled. Reselecting the same oversized file
  remained usable and was rejected again.
- Malformed avatar: the invalid preview still opens a crop dialog; OK now exits
  promptly with **Could not process your photo. Try again or choose another
  image.** Choose Photo/Retry photo upload are enabled; a second retry fails
  promptly with the same safe feedback. This does not claim malformed content
  is decoded before opening the avatar editor.
- Recipe oversized, GIF and empty originals: skipped with explicit alerts,
  without queuing/processing them. Malformed PNG: crop modal closes, failed
  placeholder/alert replaces the broken preview, no empty-byte Retry appears,
  and Publish/Cancel are enabled again.
- Mixed recipe selection `[malformed PNG, valid 512×512 PNG]`: the newer valid
  editor stays open with decoded 512×512 preview and full controls. OK leaves
  the valid item **Ready** alongside the failed item. Removing the failed item
  retains the valid Ready item. This is processing evidence, not an upload or
  persisted-edit round trip.
- Final recipe decode error wraps at desktop and 320px. At 320px, document
  width was 305px (available width after scrollbar), and alert bounds were
  x=46–259; no horizontal overflow. Malformed-avatar recovery also fits 320px
  with document width 305px. Valid avatar reselection opens its decoded 512×512
  preview; Cancel returns focus to Choose Photo without upload.

Final inspected captures:
`/tmp/noms-020-malformed-recipe-final.jpg`,
`/tmp/noms-020-malformed-recipe-final-320.jpg`,
`/tmp/noms-020-malformed-avatar-after.jpg`,
`/tmp/noms-020-malformed-avatar-final-320.jpg` and
`/tmp/noms-020-valid-after-malformed.jpg`.

Write-only API Fetch/XHR interception guarded uploads and recipe/draft writes;
observed matching API request count was **zero**. Internal already-handled
Document navigation events were excluded. One attempted failure of such a
Document event returned an invalid interception ID; it was not an API failure
or content write. All patterns were cleared, cache override disabled, viewport
reset, staged local state discarded by reload and the temporary tab closed.

Docker disk filled during rebuilding. Under the standing test-deployment cleanup
authorization, verified the stopped `noms-5063d4a16468023f-app-1` container and its
sole-use `noms-5063d4a16468023f_cargo-target` volume, then removed exactly those.
The cache was about 6.734GB; free Docker space rose to 8.6GB (8.4GB after rebuild).
Rebuildable outputs/container cannot be undeleted, but can be recreated from
source. All current/old database and uploaded-media volumes, cargo-home caches
and unrelated applications were preserved. Only the current app container was
restarted; database/media services stayed in place.

### Validation, isolation and second-pass review

Focused command with `SQLX_OFFLINE=true` and the established integration target
cache:

```sh
cargo test --features server components::base::image_upload -- --test-threads=4
```

**35 passed, 0 failed**, 897 filtered out. WASM check and formatting passed.
Full `just ci`, using `PGOPTIONS='-c log_min_error_statement=panic'`,
`SQLX_OFFLINE=true` and
`CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target`,
passed first with **919 passed/12 ignored**, then after the wrapping follow-up
with **920 passed, 0 failed, 12 ignored**. The repository's explicit test command
uses eight threads. Raw logs: `/tmp/noms-020-image-failure-ci.log` and
`/tmp/noms-020-image-failure-final-ci.log`; formatting, native/WASM check/clippy,
audit (eight existing allowed warnings), secret scan and deployment checks
completed successfully. `git diff --check` passed.

Post-check hashes match pre-check and previous acceptance exactly:

| Rows | Count | Hash |
| --- | ---: | --- |
| Recipes | 30 | `d7298c2c29ade897d2e39535462d7db3` |
| Versions | 62 | `0f91d540a069a9ae0ab0495253a3f4ae` |
| Collections | 29 | `917c84e8447402c9b3bfcb6e3d7b8075` |
| Memberships | 28 | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Drafts | 1 | `eeb6a9cba9018a65ee8e13b4bf625d52` |
| Users | 2 | `3af5cd3524ea80707ea54f1961ef1864` |

Media remained one previously deleted record, zero avatar references; no new
upload was created. Normal session reads/refresh are not unchanged-session
claims. The original account and consumed disposable `@cp8settings` fixture
were not modified/recreated.

Explicit second-pass self-review: **PASS for this scoped failure-recovery slice**.
Reviewed the complete code/test diff, ordered commits, no-write boundary,
zero-byte retry guard, URL/error cleanup, newer-dialog isolation, responsive
regression and raw CI/browser outcomes. The narrow overflow finding was fixed
and rechecked; no independent-agent review is claimed. Code and evidence are
separate semantic local commits on `codex/NOMS-020-checkpoint-8`, fast-forwarded
into `codex/NOMS-020-visual-design-refinement`. User-owned CSS remains SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
No push or production deployment. Runtime rollback baseline for this slice:
`92284c7`.

**Next:** persisted image-edit round trips and remaining image-editor,
collection-cover and step-photo flows. Browser-stalled decode timeout,
FileReader/canvas encoding failures, EXIF/ICC/animation, every alpha edge,
real-device touch and actual screen-reader checks are not accepted here.
Post-commit lost-response/navigation/identity-replacement recovery and whole-app
acceptance remain open. Further destructive/account-linking checks still need
a new isolated fixture and exact action-time approval.

## Persisted recipe image-edit round trips — 2026-10-01

**Contract:** a saved recipe gallery image can be reopened with its existing
crop, transform and colour settings, edited without re-uploading its bytes,
cancelled without leaking changes, and retained through autosave, publication,
reload and historical snapshots. This slice covers recipe gallery images only;
avatar persistence, collection covers and method-step photos remain open.

Reused the isolated checkpoint task worktree and the existing local test
deployment `noms-c799915fcf93e034` (app `http://127.0.0.1:32768`). Only its app
was restarted to serve the fix; database/media volumes were retained. No Docker
cleanup or production deployment occurred in this slice. Normal mock Google
sign-in reused the existing synthetic test identity; no profile/avatar or
provider-link mutation was submitted, and consumed `@cp8settings` was not
recreated. New private synthetic content is within the previously approved
test-data scope, not an authorization for destructive account checks.

### Finding and fix

After publishing and reopening the initial image, the saved gallery preview
was a static `div`: clicking could not reopen the editor. Source inspection
also showed no `initial_edit` binding, so local preview reopening could reset
settings; this second issue was source-confirmed, not a separate red browser
observation. New regression tests were added before production changes; the
first focused run failed to compile because the new helpers were missing.

`70ad0d4` makes both preview types named native buttons, passes the selected
image's edit into the modal, uses the saved full variant rather than thumbnail,
selects the `rectangle:16:9` template by name, and updates only the chosen edit
template. Asset identity, file metadata, other templates and upload status are
preserved. Multiple-template preservation is unit-test evidence; the actual
browser fixture has one rectangle template. Focus-visible styling preserves
the existing image layout and exposes a keyboard focus indicator.

### Fixture and browser results

Created through the UI and retained:

- Private recipe `6e906248-532b-40a3-960b-e99af9ae3f59`,
  **CP8 Image round trip — Local test only**, owner
  `6b1c56b4-945d-44d5-9f1b-eeb9525b8519` (`@user-e13c`).
- Source `/tmp/noms-020-recipe-landscape.png`, 2560 × 1440, derived from
  `scripts/dev-fixtures/image-export-grid.svg`.
- Retained managed asset `d424b194-b0a1-4951-a8ff-d1f435b53fe2`, shared by
  published v1, v2 and the unchanged follow-up edit draft
  `629489c5-0100-499b-90d3-1d2678868d42`. Drafts consumed by normal publication
  were not manually discarded. No fixture was deleted.

Initial v1 used crop 70%, original offsets X 120/Y −45, rotation 25°, width
90%, height 85%, horizontal flip, saturation 70% and RGB contrast 120/80/110%.
Its stored normalized crop is X `0.196875`, Y `0.11875000000000004`, width and
height `0.7`. The saved full image is 1920 × 1080, so reopening correctly scales
pixel offsets to X 90/Y −33.75 (the numeric control displays rounded −34).
The new resizing regression test checks normalized crop invariance and all
transform/colour settings, with offset tolerance `1e-9`.

Browser acceptance:

1. Real initial upload/publication, detail reload and keyboard Enter on the
   saved preview opened the full 1920 × 1080 image with all v1 settings restored;
   Undo was initially disabled.
2. Temporary crop 30%, rotation −90°, grayscale and horizontal-flip changes
   were cancelled. Focus returned to the edit-preview button; reopening
   restored v1 settings with no cancelled changes leaking.
3. Saturation 50% and vertical flip were applied, autosaved, reloaded and
   reopened from the private edit draft. Other settings remained intact.
4. A single publication request was failed **before server execution** using
   local XHR/Fetch interception. Pending publication controls were disabled,
   then re-enabled with safe uncertain-outcome feedback. The image edit remained
   available and intact. Clearing interception and retrying produced real v2.
   Observed retry requests contained draft save/publication/read operations,
   not another `upload_image` request. Asset URLs and identity stayed unchanged.
   This is not post-commit lost-response acceptance.
5. Reloaded v2 and reopened its saved image: saturation 50%, both flips and the
   retained crop/rotation/stretch/contrast were present. Cancelling retained
   those values and returned focus to the image button.
6. At 320 × 800, keyboard Space opened the saved image with restored v2 values.
   The page measured 320px wide while open; visual inspection showed the image,
   internally scrollable controls and footer actions fit. Escape closed the
   native dialog and restored focus to `Edit Saved recipe image`; after close
   the document measured 305px within the 320px viewport. This is browser
   responsive/keyboard evidence, not physical-device or screen-reader sign-off.
7. Read-only historical v1 still rendered saturation `0.700000`, positive
   vertical scale `0.850000`, horizontal scale `-0.900000`, rotation 25° and RGB
   slopes 1.2/0.8/1.1. Its comparison with v2 showed the intended saturation
   and flip changes. It also displays the negligible Y difference
   `0.11875000000000004` → `0.11875`; that floating-point comparison noise is an
   informational display-polish limitation, not a visible crop shift.

The development hot-reload banner appeared in accessibility snapshots after
CI, but fresh page reloads still served the verified edit button and current
controls; no claim of a banner-free development session is made. Cache was
temporarily disabled for the fresh build check, then restored. Interception
was cleared, network observation disabled, viewport override reset and the
agent-created tab closed. The deployment remains running for manual testing.

### Stored bytes, regression and review evidence

Read the actual saved full/thumbnail variants through their observed media
elements and checked them independently:

```sh
python3 scripts/tests/check-recipe-image-export.py \
  /tmp/noms-020-recipe-landscape.png \
  /tmp/noms-020-persisted-image-full.webp \
  /tmp/noms-020-persisted-image-thumb.webp
```

Full **1920 × 1080**, 176 uniform interior samples, maximum RGB error **1**;
thumbnail **800 × 450**, 176 samples, maximum RGB error **2**; maximum alpha
error **0** for both. This confirms non-destructive metadata rather than baked
or double-applied crop/colour edits in the stored variants. It does not prove
every rendered CSS transform/crop edge pixel. SHA-256:

- Full: `81011877c9a16c52e5539b16fe96e806e96e0aff4fba4ba0100a8d70f62474a0`.
- Thumbnail: `f5eb709b6878c9af0cd10c87a476c76f8be2169d443a3fe79edfbb0062c666e0`.

The two synthetic downloads were moved out of Downloads into exact named
`/tmp/noms-020-persisted-image-download-{full,thumb}.webp` evidence files;
both remain recoverable and their hashes match. No other Downloads files were
touched. An initial sandboxed cross-filesystem move copied but could not remove
the source; the approved exact-path move subsequently completed cleanup.

Focused image-upload tests: **38 passed, 0 failed**; WASM `cargo check --target
wasm32-unknown-unknown --no-default-features --features web` passed. Final
`just ci`, using `PGOPTIONS='-c log_min_error_statement=panic'`,
`SQLX_OFFLINE=true` and the integration worktree's `CARGO_TARGET_DIR`, passed
with **924 passed, 0 failed, 12 ignored**. The repository's test cap is eight
threads. Native/WASM check/clippy, formatting, audit (eight existing allowed
warnings), tracked-secret scan and deployment checks passed. Raw log:
`/tmp/noms-020-persisted-image-ci.log`. `git diff --check` passed.

Original row hashes match exactly, excluding only the new recipe and its linked
versions/draft from the comparison:

| Original rows | Count | Hash |
| --- | ---: | --- |
| Recipes | 30 | `d7298c2c29ade897d2e39535462d7db3` |
| Versions | 62 | `0f91d540a069a9ae0ab0495253a3f4ae` |
| Collections | 29 | `917c84e8447402c9b3bfcb6e3d7b8075` |
| Memberships | 28 | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Drafts | 1 | `eeb6a9cba9018a65ee8e13b4bf625d52` |
| Users | 2 | `3af5cd3524ea80707ea54f1961ef1864` |

Total rows now include the intended fixture: 31 recipes, 64 versions, two
drafts and two media records (the old deleted record and this retained asset).
Recipe/version/draft media-reference counts are 1/2/1. Normal session/OAuth
reads are not unchanged-session claims.

Temporary browser evidence:

- `/tmp/noms-020-persisted-image-before.jpg` — initial original-image settings.
- `/tmp/noms-020-persisted-image-v1-detail.jpg` — first saved result.
- `/tmp/noms-020-persisted-image-v1-reopened.jpg` — post-fix restored v1 modal;
  not the pre-fix static-preview screenshot.
- `/tmp/noms-020-persisted-image-draft-reopened.jpg` — restored autosaved edit.
- `/tmp/noms-020-persisted-image-publish-failure.jpg` — retained failure state.
- `/tmp/noms-020-persisted-image-v2-detail.jpg` and
  `/tmp/noms-020-persisted-image-v2-reopened.jpg` — real second publication.
- `/tmp/noms-020-persisted-image-320.jpg` — narrow saved-edit reopening.
- `/tmp/noms-020-persisted-image-v1-history.jpg` — retained historical photo.

Explicit second-pass self-review: **PASS for this scoped recipe-image slice**.
Reviewed the complete production/test diff, saved/local URL choice, named-edit
selection/preservation, missing-image handling, metadata-only upload plan,
keyboard semantics, cancellation boundaries, actual browser/SQL/pixel outcomes
and ordered commits. No independent-agent review is claimed. The tiny numeric
history comparison difference is informational and remains recorded above;
no blocking or should-fix finding remains within this contract. Code and this
evidence are separate local semantic commits on `codex/NOMS-020-checkpoint-8`,
fast-forwarded into `codex/NOMS-020-visual-design-refinement`. User-owned CSS
remains SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
No push. Runtime rollback baseline for this slice: `3fac4d8`.

**Next:** persisted avatar edits using an isolated non-destructive fixture,
then remaining collection-cover and step-photo flows. Post-commit lost-response,
navigation/identity-replacement recovery, full image edge/EXIF/ICC/animation
checks, physical-device touch, screen readers and whole-app acceptance remain
open. Further destructive/account-linking actions still require a new isolated
fixture and exact action-time approval.

## Persisted avatar edit round trips — 2026-10-01

**Contract:** save an adjusted circular avatar, reopen the saved original with
the same metadata, cancel changes without persistence, recover from a failed
upload without losing the selected image/edit, then replace successfully and
reopen after reload. This is a non-destructive avatar-edit slice, not removal,
unlink, account deletion or post-commit lost-response acceptance.

Reused the isolated `codex/NOMS-020-checkpoint-8` worktree and existing local
deployment `noms-c799915fcf93e034`, app port 32768. Verified the exact Postgres
container's `com.noms.dev-environment=true` label. No app restart, Docker cleanup,
volume removal, direct database mutation or object deletion occurred. Normal
mock Google sign-in created one separate synthetic identity through the UI:

- Subject `cp8avatarroundtrip20261001`, email
  `cp8avatarroundtrip20261001@example.invalid`.
- User `89f18aba-5a00-47d1-9d86-ea7330fe9a2c`, public handle
  `@cp8-avatar-round-trip-lo-6894`.
- Initial avatar asset `c0c31528-db1f-4790-8b39-d9943034a1f8`; successful edited
  replacement `d620ae2f-cb7d-4a5f-b238-26ce55fec355`, now the sole avatar
  reference. Both records were observed as retained after this test; the first
  is unreferenced and eligible for the normal lifecycle worker. No permanent
  retention or worker object-cleanup claim is made for that superseded asset.

The test account and its current avatar remain for reproducible checks. The
consumed disposable Settings account was not recreated. Original profiles and
content were not edited. Sign-in had no Terms acceptance/provider linking;
the original `testuser` session was restored through normal mock sign-in at
the end, verified as `@user-e13c`, and both temporary browser tabs were closed.

### Browser and storage outcomes

1. Selected `/tmp/noms-020-recipe-landscape.png` (2560 × 1440) through the
   actual Choose Photo/file-chooser flow. Applied crop 70%, X 120/Y −45,
   rotation 25°, width 90%, height 85%, horizontal flip, saturation 70% and
   RGB contrast 120/80/110%. Real upload succeeded and the new avatar appeared.
2. Reloaded and reopened Edit Photo. The editor used the **2560 × 1440 saved
   original**, not the baked 400px avatar; all numeric controls and horizontal
   flip were restored, with initial Undo disabled. An Enter attempt immediately
   after reload hit the pre-hydration page and timed out; inspecting state and
   activating the now-hydrated control worked. Subsequent hydrated Enter/Space
   checks passed. The development rebuild banner remained visible in AX
   snapshots; a banner-free or pre-hydration interaction contract is not claimed.
3. Changed crop to 30%, rotation to −90° and toggled horizontal flip, then
   Cancel. Focus returned to Edit Photo. Keyboard reopening restored every
   saved value; the temporary changes did not leak.
4. Changed saturation to 50% and enabled vertical flip. Held the actual
   `upload_avatar*` XHR before server execution: uploading status appeared,
   Edit/Change/Remove and upload actions were disabled. Failed that request;
   safe retry feedback appeared and the selected source/edit stayed available.
   Read-only SQL still showed the initial asset, saturation 0.7 and no vertical
   flip. Duplicate submission/identity replacement is not newly accepted here.
5. Keyboard Retry photo upload produced exactly the same edit JSON and
   original/full/thumbnail bytes as the failed attempt. Continued the actual
   held retry to the server: successful replacement appeared. Reload and
   keyboard reopening retained saturation 50%, both flips and all remaining
   settings. Unlike recipe-gallery metadata updates, avatar editing correctly
   regenerates and uploads baked circular variants from the saved original.
6. At 320 × 800, the native dialog bounds were X 0/width 320/height 800 and
   document scroll width 320. Restored settings, preview, internally scrolling
   controls and footer actions fit in the visually inspected settled capture.
   Escape closed the dialog and returned focus to Edit Photo. Viewport was
   reset and normal 1280px sizing confirmed before cleanup.
7. Failed `get_avatar_edit_source*` before execution: safe “Could not load the
   saved avatar edit” feedback, no modal. Then failed only the observed original
   image GET: “Could not load the current avatar for editing,” no modal.
   Clearing interception and retrying reopened the correct second edit at
   320px. These are read-failure recoveries, not stalled/overlapping read tests.

Stored normalized ellipse crop: X `0.35`, Y `0.11875000000000004`, width
approximately `0.39375`, height `0.7`, with aspect ratio 1:1. SQL confirmed only
the intended saturation/vertical-flip replacement. Tiny normalized floating
differences do not alter the displayed crop.

Captured actual stored original/full bytes from their observed browser network
responses and the actual stored thumbnail from its exact registry URL in a
temporary browser tab. All three are byte-identical to the submitted retry;
the original also equals the initially selected PNG. SHA-256:

- Original: `27d2e37f687b25f107645fff6b5b41720f0db3dcf06d125aeee12fa5f9ea22d2`.
- Full: `d99ecb71dfd296aad5d25d6977450d018747f25ccee32b4b2b208e54ab06b398`.
- Thumbnail: `c19e5baf464d293c70d18d6543701a9f1b3297d0754604b5bb6686a5ad19a864`.

Independent pixel verification:

```sh
python3 scripts/tests/check-avatar-export.py \
  /tmp/noms-020-avatar-persisted-stored-original.png \
  /tmp/noms-020-avatar-persisted-second-edit.json \
  /tmp/noms-020-avatar-persisted-stored-full.webp \
  /tmp/noms-020-avatar-persisted-stored-thumb.webp
```

**PASS:** full 400 × 400, 357 unambiguous interior samples, max RGB error 4;
thumbnail 100 × 100, 235 samples, max RGB error 11. Both have transparent
corners. The same check passed against captured pre-server retry files. This
accepts sampled transformed/coloured pixels and corners, not every antialias
edge, EXIF/ICC/animation input or rendered preview/export pixel.

No runtime change or new test was needed. Existing focused regression command
`SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target cargo test
--features server settings_profile -- --test-threads=4` passed **22 tests,
0 failures**, 914 filtered out. Raw log:
`/tmp/noms-020-avatar-persisted-tests.log`. Latest unchanged-runtime full CI
remains **924 passed, 0 failed, 12 ignored** from the preceding recipe-image
slice; no new full-CI run is claimed. Documentation diff checks passed.

Original content hashes match the start of this slice exactly:

| Rows | Count | Hash |
| --- | ---: | --- |
| Recipes | 31 | `ca156ae94adb947875212ab97b627946` |
| Versions | 64 | `f216416ebd2518de1dac32d51f466f11` |
| Collections | 29 | `917c84e8447402c9b3bfcb6e3d7b8075` |
| Memberships | 28 | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Drafts | 2 | `d860a7535dfc63331592ca6512a1c27e` |
| Original users, excluding only new avatar fixture | 2 | `3af5cd3524ea80707ea54f1961ef1864` |

Totals are now three users, four media records, one avatar reference and two
OAuth accounts. Session/provider last-use writes from routine sign-in/sign-out
are not unchanged-session/provider-row claims. Temporary interception was
cleared, cache restored, network observation disabled and viewport reset.

Browser captures in `/tmp`, with the prefix `noms-020-avatar-persisted-`:
`initial-edit.jpg`, `first-reopened.jpg`, `upload-pending.jpg`,
`upload-failure.jpg`, `second-saved.jpg`, `second-reopened.jpg`, `320.jpg`,
`source-read-failure.jpg` and `original-read-failure.jpg`. The settled 320px
and second-reopened captures were visually inspected. The thumbnail's native
image-document reload was unexpectedly slow in browser tooling; successful
response, decoded 100px image and byte verification followed. That is not
evidence of app upload slowness or a passed performance budget.

Explicit second-pass self-review: **PASS for this scoped avatar round trip**.
Reviewed source hydration/original selection, cancellation/upload-state
boundaries, byte comparisons, database ownership/reference outcomes, pixel
checker scope, focused tests and cleanup. No independent reviewer is claimed.
No code fix or additional implementation commits were warranted: one local
evidence/plan commit is sufficient for this documentation-only slice, followed
by fast-forward integration. User-owned integration CSS is unchanged, SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
No push or production deployment; runtime baseline remains `70ad0d4`.

**Informational follow-up:** OAuth accepted the synthetic provider's 36-character
display name, while Profile settings displays 36/30 and a 30-character editing
limit. This existing cross-flow inconsistency did not affect avatar edits; it
is recorded for separate profile-name normalization acceptance, not silently
fixed or considered closed here.

**Next:** remaining collection-cover and step-photo edit flows. Post-commit
lost responses, navigation/identity replacement, physical-device touch, screen
readers and whole-app acceptance remain open. Destructive/account-linking
actions still require a new isolated fixture and action-time approval.

## Collection-cover and step-photo edit round trips — 2026-10-01

**Contract:** reopen adjusted saved covers and method photos with their existing
metadata, isolate cancelled edits, persist successful replacements, retain
edits after pre-server save/publication failure, and support native keyboard
reopening at desktop and 320px. No removal, deletion, account/provider mutation,
post-commit lost-response or initial step-upload-failure acceptance is claimed.

Used existing isolated local deployment `noms-c799915fcf93e034`, app port
32768, PostgreSQL 32769, MinIO 32770/32771 and mock OAuth 32772. Original
`testuser` / `@user-e13c` session remained signed in. Runtime started at
`70ad0d4`, with evidence baseline `9a2ec9f`. Implementation used the existing
`codex/NOMS-020-checkpoint-8` task worktree and fast-forwarded into the design
worktree. No push, production deployment or unrelated deployment cleanup.

Only new private synthetic content was changed:

- Collection `ba957378-d935-4a52-a123-ad8e49fd3854`, **CP8 Cover round trip —
  Local test only**, cover asset `e97193cd-1ea7-4470-99ae-253f77001403`;
  no memberships or subcollections.
- Recipe `a5a7ee84-28df-41d1-bd0e-44e3d554776d`, **CP8 Step photo round trip —
  Local test only**, one ingredient and one method photo, no gallery photos;
  asset `afa4cff4-e352-4ea6-8317-ed2398652f15`, v1/v2 and unchanged follow-up
  draft `872997d1-b6be-410f-9ebb-22ace1d797ef` retained.
- New unpublished draft `ba3917ec-1033-4e19-83aa-a4977109a297`, **CP8 New
  editor step-photo reopen — Local test only**, one method photo, asset
  `69af30a3-23b7-4d1e-9348-6eadae1231e5`; retained without publishing/discarding.

All assets belong to mock user `6b1c56b4-945d-44d5-9f1b-eeb9525b8519` and
are retained. The cover has one collection reference; the published step photo
has one current-recipe and two historical-version references, plus its
follow-up draft reference. Draft-only media remains attached to its draft.

Selected `/tmp/noms-020-recipe-landscape.png`, the existing 2560 × 1440
synthetic grid, SHA-256
`27d2e37f687b25f107645fff6b5b41720f0db3dcf06d125aeee12fa5f9ea22d2`.
Step-photo filename/size remain `noms-020-recipe-landscape.png` / 44989 bytes.

### Collection cover

1. New collection: applied crop 70%, X 120/Y −45, rotation 25°, saturation
   70% and horizontal flip. Enter reopened the pending selection with those
   values; Cancel left it unchanged. Real create/upload succeeded.
2. Saved preview reopened the full 1920 × 1080 source, with normalized crop
   displayed as 70%, X 90/Y −34 and all remaining settings restored. Changing
   rotation to −90° then cancelling did not leak into the next reopening.
3. Saved saturation 50%/vertical flip, then tested two actual pre-server
   `update_collection*` failures, including a changed saturation of 40%.
   Pending Save was disabled; safe failure feedback retained all entries and
   re-enabled actions. Reopening after failure retained 40%.
4. Cleared interception and retried: successful save/reload, no `upload_image`
   request, same asset and URL. At 320 × 800, Enter reopened 70%, X 90/Y −34,
   rotation 25°, saturation 40%, both flips; Escape returned focus to the
   preview button and document width stayed 320px.

The first attempted interception used a regex-like `[0-9]*` in a CDP glob and
did **not** intercept; that synthetic save actually succeeded. Only the two
later correctly matched failures are failure evidence. The generic shared
preview name remains “Edit Saved recipe image”; collection-specific naming is
an informational follow-up, not a change in this slice.

### Step-photo reopening fix and acceptance

Browser red evidence: after initial upload/edit and after v1 publication,
method previews had no edit action; clicking the photo did not open a modal.
Three tests were added before production changes. The corrected red run failed
for missing production helpers/component; initial test scaffolding also needed
a qualified `VirtualDom` import. Raw red log:
`/tmp/noms-020-step-photo-red.log`.

Local code commit `ecd04cc`, **NOMS-020: reopen saved step photos with their
edits**, adds shared `StepPhotoEditButton` in both editors: a named native
button, disabled during editor writes/uploads. It selects the saved
`rectangle:16:9` metadata (or its default), edits the full source rather than
the thumbnail and replaces only that named view. Asset/URL/file metadata and
other named views remain unchanged. Upload-in-flight hand-off is only queued
for a local blob without a completed photo index. Modal close clears its
selection/initial metadata. The code/tests are one indivisible behavior commit;
acceptance documentation is a separate commit.

Fresh-build browser green evidence:

1. Enter opened v1's saved photo with crop 70%, X 90/Y −34, rotation 25°,
   saturation 70% and horizontal flip. Changed rotation −90°, Cancel, Space
   reopening restored 25°.
2. Applied saturation 50% plus vertical flip, waited for autosave and reloaded.
   Reopening restored all settings, including both flips and saturation 50%.
3. Held `publish_recipe_draft*` before server execution: Publishing and the
   photo-edit action were disabled. Failed the request; safe uncertain-outcome
   publication feedback appeared and reopening retained saturation 50%.
   Cleared interception; real retry published v2 without any image upload.
4. Reopened the unchanged follow-up draft at 320 × 800 with Space; settings
   survived, document width was 320px, and Escape returned focus to
   “Edit step photo: noms-020-recipe-landscape.png”.
5. New-recipe flow uploaded another synthetic photo, applied rotation 15° /
   saturation 80% and reopened it. Normal autosave transitioned to its named
   draft route; cancelling rotation 60° restored 15°. Reload/reopening retained
   15°/80%. This flow includes the normal New → Draft editor transition, not a
   claim that it remained on the New component throughout autosave.
6. SQL confirms v1 retained saturation 70% / no vertical flip while v2 has 50%
   / vertical flip, with the exact same asset and file metadata. Browser v1
   renders `saturate(0.700000)` and `scaleY(1.000000)` from its original full
   image. Historical snapshots were not overwritten.

The first restart and ordinary build reused stale Dioxus artifacts. Cleaned
only NOMS package artifacts in the actual `wasm-dev` / `server-dev` custom
profiles, preserving dependencies, DB and media; explicit build compiled both
targets and the subsequent restart served the new edit button. Fresh build
log: `/tmp/noms-020-step-photo-deployment-fresh-build.log`. Dioxus regenerated
the already user-modified `assets/tailwind.css` with one extra generated rule;
restored its exact pre-build bytes from a hash-verified cached asset using a
narrow patch. Final hash matches the starting hash:
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
The dev-tool rebuilding banner still appeared in DOM snapshots; it did not
prevent the observed app interactions and is not performance acceptance.

### Stored bytes, tests and integrity

Captured actual stored cover/step full and thumbnail bytes from observed
successful browser responses. Both use the same non-destructive export
pipeline and identical source, so their variant hashes match:

- Full: `81011877c9a16c52e5539b16fe96e806e96e0aff4fba4ba0100a8d70f62474a0`.
- Thumbnail: `f5eb709b6878c9af0cd10c87a476c76f8be2169d443a3fe79edfbb0062c666e0`.

`python3 scripts/tests/check-recipe-image-export.py` with the original PNG and
each `/tmp/noms-020-{cover,step}-stored-{full,thumb}.webp` pair passed: full
1920 × 1080 and thumbnail 800 × 450, 176 flat interior samples each, maximum
RGB error 1/2 and alpha error 0. This checks sampled non-destructive resizing,
not all interpolation edges or a rendered-preview pixel equivalence.

Validation:

- `cargo fmt --all`, `git diff --check`: passed.
- `SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target cargo test
  --features server step_photo -- --test-threads=4`: **5 passed, 0 failed**;
  `/tmp/noms-020-step-photo-final-tests.log`.
- `SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target cargo check
  --target wasm32-unknown-unknown --no-default-features --features web`: passed;
  `/tmp/noms-020-step-photo-web.log`.
- `PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true
  CARGO_TARGET_DIR=<integration-worktree>/target just ci`: **927 passed,
  0 failed, 12 ignored**, all remaining gates passed; raw final log
  `/tmp/noms-020-cover-step-ci-final.log`. First sandboxed run passed tests but
  could not lock the read-only Cargo advisory cache; authorized rerun completed
  with the existing eight allowed advisory warnings.

Excluding only this slice's new collection/recipe (and their versions/drafts)
and new draft-only fixture, original content matches its starting hashes:

| Rows | Count | Hash |
| --- | ---: | --- |
| Recipes | 31 | `ca156ae94adb947875212ab97b627946` |
| Versions | 64 | `f216416ebd2518de1dac32d51f466f11` |
| Collections | 29 | `917c84e8447402c9b3bfcb6e3d7b8075` |
| Memberships | 28 | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Drafts | 2 | `d860a7535dfc63331592ca6512a1c27e` |
| Users | 3 | `7bc139f2aea02af7488ad90b2d8f0cca` |

Temporary screenshots: `/tmp/noms-020-cover-retained-after-failure.jpg`,
`/tmp/noms-020-cover-reopened-320.jpg`, `/tmp/noms-020-step-photo-before.jpg`,
`/tmp/noms-020-step-photo-reopened.jpg`,
`/tmp/noms-020-step-photo-publication-failure.jpg` and
`/tmp/noms-020-step-photo-320.jpg`. Settled cover/step 320px captures and the
pre-fix method area were visually inspected. Interception cleared, cache
restored, network observation disabled, viewport reset and temporary tab closed.

Explicit second-pass self-review: **PASS for this scoped round trip**. Reviewed
selection/reset behavior, full-source use, named-edit preservation, busy guards,
upload hand-off, regression tests, stored media, historical integrity and
bounded evidence claims. Removed unnecessary hand-off entries for completed
local uploads during review. No independent agent review is claimed.

**Next:** initial step-photo processing/upload failure recovery, local blob URL
lifecycle and multi-file/reorder interactions. Their current log-only error
paths are not covered by the gallery input-failure acceptance. Cancelling the
initial step modal cancels edits, not an upload already underway. Post-commit
lost responses, navigation/identity replacement, full accessibility and
physical-device touch remain open; destructive/account-linking actions still
need their separate authorization and fixtures.

## Initial step-photo upload recovery, preview lifecycle and ordering — 2026-10-01

Continued the next scoped acceptance gate on the existing task worktree
`codex/NOMS-020-checkpoint-8`, based on `f75bd30`. Source commits, in order:
`703441e` (recovery, local preview ownership and batch/ordering guards),
`5f49899` (client-only callback guards) and `ab4d386` (native file-selection
stub consumption). These are fast-forward integrated into
`codex/NOMS-020-visual-design-refinement`; nothing was pushed. Reused the
established local project `noms-c799915fcf93e034` on port 32768, rebuilding and
restarting only its app. Its existing database and media were retained.

Contract and changes:

- Both recipe editors validate original step files using the shared
  JPEG/PNG/WebP, non-empty and 10 MB constraints before decoding. Safe,
  filename-bounded inline alerts distinguish validation, processing and upload
  failures; technical details remain in logs. The native file picker resets so
  the same file can be chosen again.
- One synchronous claim protects the entire selection batch. Pending uploads
  disable and live-guard structural step/photo controls, including reorder,
  drag, removal, another selection and Add Step. Publication/cancel guards
  remain in place. Method text stays editable. The upload destination cannot
  change through those controls between asynchronous files.
- A single file opens the existing adjustment modal. Multi-file selection
  uploads sequentially without replacing an unconfirmed modal; users can edit
  each uploaded photo through its preview button. Valid files keep selection
  order even when invalid files are skipped.
- Each local modal URL has a single owned lease in the component signal.
  Closing the modal, processing/upload failure or component teardown drops
  that lease and revokes the URL. Upload tasks retain only string keys.
  Failed pending-edit handoffs are removed, leaving other photos unchanged.
  This does not change initial-modal cancellation into upload cancellation.

Tests were added before implementation. The initial focused run failed on
missing production helpers/components. Final focused command:
`SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target cargo test
--features server step_photo -- --test-threads=4`: **8 passed, 0 failed**,
`/tmp/noms-020-step-upload-green.log`. Coverage includes safe bounded messages,
SSR status/alert semantics and source-contract checks for both upload paths;
source checks are not a substitute for browser execution.

`SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target cargo check
--target wasm32-unknown-unknown --no-default-features --features web` passed;
`/tmp/noms-020-step-upload-web.log`. Final
`PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true
CARGO_TARGET_DIR=<integration-worktree>/target just ci`: **930 passed,
0 failed, 12 ignored**, all remaining gates passed, with the eight existing
allowed advisory warnings; `/tmp/noms-020-step-upload-ci-final.log`. Earlier CI
attempts caught native-only needless-return/unused-file warnings; the two
follow-up commits fix those rather than suppressing the checks. Formatting and
`git diff --check` passed.

Browser acceptance used the existing private unpublished draft
`ba3917ec-1033-4e19-83aa-a4977109a297`, titled
**CP8 New editor step-photo reopen — Local test only**, owned by the original
mock account. Only this synthetic draft and three new recipe-image assets
were durably changed; nothing was published or deleted.

1. Before rebuilding, an empty PNG left the old initial modal open with no
   visible failure alert (`/tmp/noms-020-step-upload-before.jpg`). After the
   fix, empty/unsupported selections report actionable alerts without opening
   a modal or allocating preview URLs. A malformed PNG closes its failed modal
   and restores controls. Both local URLs observed for that decode were
   revoked.
2. Held a portrait upload before server execution. The existing photo,
   Add Step, photo editing/reorder and both steps' structural controls were
   disabled; Publish/Cancel/Discard were disabled. Confirmed temporary
   rotation 35/saturation 85 while held, then failed the request. The one
   existing photo stayed unchanged, the alert offered reselection, and
   controls recovered.
3. Retried the identical portrait file. Its modal started at rotation 0 and
   saturation 100, not the failed handoff's values. Confirmed rotation 15 and
   saturation 90; a real upload succeeded. Reopening restored those values.
4. Selected JPEG, empty PNG and WebP together. While the first request was
   held, there was no adjustment modal and the batch lock stayed active.
   Resuming uploaded both valid files in selection order and retained the
   empty-file warning. Moved WebP left then right; moved the photo-bearing
   step down then up. Autosave/reload preserved the final order and associated
   method text.
5. At 320px, the malformed-photo alert wrapped within its panel (left 32,
   right 273, width 241; viewport 320, document width 305). Four photos and
   both method rows remained usable without document overflow. Screenshot
   `/tmp/noms-020-step-upload-recovery-320.jpg` was visually inspected. This
   checks narrow layout, not physical-device touch or screen-reader speech.
6. On `/recipes/new`, malformed-file processing recovered with the same safe
   alert. Then opened a valid portrait's local modal with its upload held;
   normal same-document browser Back returned to the existing draft and
   revoked the component-owned preview. Both held upload and draft-create
   requests were aborted before execution, so no additional draft was made.

Temporary diagnostic wrappers logged URL allocation/revocation in this test
tab only. For the first edit-route sequence (malformed decode, failed single
upload, successful retry and two valid batch files), eight unique local URLs
were allocated and all eight revoked. The new-route teardown allocated two
more and revoked both. Raw events:
`/tmp/noms-020-step-upload-preview-events.json`. Reload removed the wrappers;
interception was cleared, Runtime/Network observation disabled, viewport reset
and the temporary tab closed. This is scoped URL-lifecycle evidence, not a
claim that every image-processing callback or browser resource is leak-free.

Saved draft provenance after reload:

| Step 1 photo order | Asset ID | Rectangle edit |
| --- | --- | --- |
| Original landscape PNG | `69af30a3-23b7-4d1e-9348-6eadae1231e5` | Original rotation 15, saturation 0.8 preserved |
| Retried portrait PNG | `70a18b88-6365-480f-9691-39e90eea33f0` | Rotation 15, saturation 0.9; crop y 0.359375, height 0.28125 |
| Batch landscape JPEG | `755bb27f-dbcb-40a3-ab47-78c8e1d9fc2e` | Default full rectangle |
| Batch landscape WebP | `0ec1cadf-097e-42fe-aaf3-34c4e5a1353f` | Default full rectangle |

The second method row contains only **Synthetic destination guard step.**
No crop adjustment was baked into upload pixels by this slice; earlier export
acceptance remains the pixel evidence. No new stored-pixel comparison is
claimed here.

Final read-only database hashes match the start of this slice, excluding only
the named synthetic draft:

| Rows | Count | Hash |
| --- | ---: | --- |
| Recipes | 32 | `e8933d2d64c0b6bd5553833c8a0dab20` |
| Versions | 66 | `ddeec72b8bfa497a60690729e3b65bac` |
| Collections | 30 | `9f2f90802adafb72eca3ea15fc0bbacb` |
| Memberships | 28 | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Other drafts | 3 | `56ecca403e09c456247b908907485805` |
| Users | 3 | `7bc139f2aea02af7488ad90b2d8f0cca` |

The rebuild regenerated a `.grayscale` utility in the integration worktree's
user-owned dirty `assets/tailwind.css`. Removed only that generated block;
the file's original SHA-256 is restored exactly:
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
It remains uncommitted; main's unrelated roadmap changes are untouched.

Explicit second-pass self-review: **PASS for this scoped gate**. Reviewed both
route diffs, native/WASM compilation, bounded safe feedback, whole-batch/live
guards, pending-edit failure cleanup, URL lease ownership, real retry/order
persistence, original-row integrity and evidence boundaries. No independent
review agent was used. Cross-step photo dragging is not newly implemented or
accepted; existing same-step arrow reordering is the tested interaction.

**Next:** post-commit lost-response/navigation/identity-replacement recovery,
remaining image-format/encoding edge cases, full accessibility/real-device
checks and whole-app visual/behavior acceptance. Pre-server failure tests do
not establish behavior after a committed mutation whose response is lost.
Further destructive/account-linking tests still require separate authorization
and isolated fixtures. Checkpoint 8 remains open.

## Committed draft response loss and new-recipe outcome recovery — 2026-10-01

Continued in the established task worktree `codex/NOMS-020-checkpoint-8`, based
on `f40d4e9`, without touching main's unrelated roadmap edits. Reused the
existing `noms-c799915fcf93e034` deployment on port 32768 and the synthetic
fixtures; a fresh temporary browser tab initially reached the sign-in gate.
Normal Google mock sign-in as `testuser` restored the existing `@user-e13c`
owner. A preliminary GitHub provider page was opened but no GitHub sign-in or
account link was submitted. No identity was created, replaced or deleted.

This slice distinguishes a **response-stage** failure after an independently
observed database commit from earlier pre-server failure checks. Original
account/content rows remain unchanged, except for the named synthetic fixtures
below. No Delete/Discard/provider-linking action was submitted. Two new private
recipes were intentionally published as local test data; none was made public.

### Existing-draft save and navigation evidence

Used the unpublished step-photo fixture
`ba3917ec-1033-4e19-83aa-a4977109a297`, initially revision 9 with empty Notes.
The four existing photo records and method rows were retained. Browser and
read-only SQL observations:

- Autosave A reached HTTP 200 at the response-stage pause; SQL proved revision
  10 with the submitted note before the response was failed. Existing
  refresh/retry reconciled the revision and returned to **Saved**.
- Autosave B committed, then a newer C note was entered before B's response
  was failed. The newest generation won; SQL proved revision 13 containing C,
  not B. No stale completion overwrote the local field.
- Explicit Save Draft D committed revision 14 before its response was failed.
  Pending controls were disabled. Safe draft-save feedback appeared, entries
  stayed visible and the existing autosave reconciliation returned to Saved.
  Reload restored D. This does not claim the original failed response itself
  reached the client successfully.
- Autosave E committed while its response was held. Native Dashboard
  navigation showed revision 16. Reopening the draft loaded E. A new F note
  was entered, then the old held response was failed; the new visit stayed
  isolated and autosaved revision 17 with F. This tests leaving/returning
  during a write, not a failed final browser navigation after success.

Final note: **CP8 returned visit F is isolated — synthetic only.** Published
recipes/versions and unrelated drafts were unchanged by these checks.

### Confirmed issues and fixes

1. On `/recipes/new`, the first creation committed draft
   `a4e2d8f5-1514-458b-b429-e86a76840a13` at revision 0. After dropping its HTTP
   200 response, the old UI exposed a raw transport exception. A subsequent
   edit issued another creation request. That second request was held and
   failed **before execution**, so no duplicate draft was actually inserted.
2. Added an explicit unconfirmed-creation state. It preserves visible entries
   while blocking autosave, pending debounce tasks, manual Save/Publish and
   new step uploads. It renders safe feedback even when the failed creation
   belongs to an older editor generation. Generic creation errors are treated
   conservatively; this is a UI recovery guard, not server-side idempotency or
   automatic content matching.
3. Shared the existing safe recipe-write feedback with the new editor rather
   than retaining raw creation/save/publication exceptions. Retain the returned
   draft ID/revision **before** issuing publication, including the initial
   publish-without-an-earlier-autosave path. A lost publication response must
   never send the new editor back through another creation call.
4. Review caught a local-entry loss risk: the recovery link must not leave the
   current unsaved page. **Check Dashboard** opens a separate tab. Users can
   inspect/reopen the saved draft while the original entries remain available
   for manual reconciliation. It does not silently merge unsaved changes.
5. The first publication response-loss run created exactly one private recipe
   and v1, and a retry issued no duplicate creation/publication. However, a
   later autosave error replaced the useful publication warning. Added an
   explicit unconfirmed-publication state: once publication was attempted
   and failed, keep the warning, pause further writes and expose the same
   separate-tab Dashboard recovery. A failure before publication is attempted
   does not claim that publication may have executed. This guard is also
   deliberately conservative for server-reported publication failures.

Ordered local source commits:

- `d04e2cf` — creation guard, shared safe feedback and retained publish identity.
- `63d3bd8` — separate-tab Dashboard recovery preserves local entries.
- `fd6a232` — persistent uncertain-publication feedback and write pause.

They are fast-forward integrated into `codex/NOMS-020-visual-design-refinement`;
nothing was pushed. Only the existing test app was rebuilt/restarted; its
database/media volumes and other deployments were not cleaned or restarted.

### Final browser acceptance

- Guarded autosave creation committed
  `b232b960-1d24-4ae4-b1ee-4d2572a46922`. Entered a newer description while its
  response was held, scheduling another debounce, then dropped the committed
  response. Safe alert remained visible even though the failed request was
  from the older generation. Observed zero new creation requests through that
  debounce and another subsequent Notes edit. Save/Publish stayed disabled;
  local description and note remained visible.
- Native Check Dashboard opened a second tab without navigating the original.
  Its draft card reopened the original ID. Manually transferred the retained
  description/note and saved revision 1; no automatic merge is claimed.
- At 320px, the creation alert spans x16–289 within document width 305 and
  viewport 320. The wrapped warning and keyboard-focused recovery link were
  visually inspected: `/tmp/noms-020-unconfirmed-draft-320.jpg`.
- Final publication test began on a fresh new-recipe form. Filled a title,
  synthetic ingredient and method, then clicked Publish before the autosave
  debounce. Publication consumed draft
  `afe7b3e0-761e-4ecb-85df-2454bb2bab27` and committed private recipe
  `84e712e9-4088-4b7b-9599-e64c874d02a9` with exactly one v1; SQL proved this
  before its HTTP 200 response was dropped.
- Save/Publish remained disabled, Cancel usable, and the safe publication
  warning stayed visible through another Notes edit. Observed **zero** further
  create/save/publish requests. Native recovery opened Dashboard separately,
  showing the committed recipe; reopening it created the normal edit draft.
  Manually transferred the retained note, autosaved and reloaded successfully.
  Published v1 remained unchanged; the note is private draft-only data.
- Publication alert also fits x16–289 at 320px without document overflow;
  `/tmp/noms-020-unconfirmed-publication-320.jpg` was visually inspected.
  Narrow geometry and native focus are not physical-device or screen-reader
  speech acceptance.

New synthetic records retained for replay:

| Fixture | ID | Final state |
| --- | --- | --- |
| Pre-fix first creation | `a4e2d8f5-1514-458b-b429-e86a76840a13` | Private draft, revision 0; duplicate request prevented before execution |
| Guarded first creation | `b232b960-1d24-4ae4-b1ee-4d2572a46922` | Private draft, revision 1 with manually recovered description/note |
| First lost-publication run | `640216dd-222e-4b56-8cf3-65ba77943dc1` | Private recipe, exactly one v1; source draft `6ee3f9ed-bf56-448f-8d58-389b00dda957` consumed by publication |
| Final publication guard | `84e712e9-4088-4b7b-9599-e64c874d02a9` | Private recipe, exactly one v1 |
| Continued edit after final publication | `35a5b54d-bae2-4df8-b1f0-480564056609` | Private edit draft, revision 1 with retained note; v1 not changed |

### Automated evidence, integrity and review

New tests preceded implementation. Initial run failed on missing shared helper;
the later publication test failed on the absent publication state. Final
`SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target cargo test
--features server draft_save -- --test-threads=4`: **5 passed, 0 failed**;
`/tmp/noms-020-lost-response-green-final.log`. These include message and
source-contract tests; browser/network/SQL observations, not those source
checks alone, prove the runtime outcomes.

Final `PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true
CARGO_TARGET_DIR=<integration-worktree>/target just ci`: **933 passed,
0 failed, 12 ignored**, native/WASM check and Clippy, formatting, schema/policy,
secret/artifact checks and dependency audit all passed. The eight existing
allowed advisory warnings remain; `/tmp/noms-020-lost-response-ci-final.log`.
Final app build: `/tmp/noms-020-lost-response-deployment-accepted.log`.
Scoped response/request-pause metadata (no cookies/headers):
`/tmp/noms-020-lost-response-pauses.json`. The earlier 932-test gate is
superseded by the final publication-guard revision.

Final hashes equal this turn's baseline after excluding only the newly named
fixtures and the existing synthetic step-photo draft whose Notes were edited:

| Original rows | Count | Hash |
| --- | ---: | --- |
| Recipes | 32 | `e8933d2d64c0b6bd5553833c8a0dab20` |
| Versions | 66 | `ddeec72b8bfa497a60690729e3b65bac` |
| Collections | 30 | `9f2f90802adafb72eca3ea15fc0bbacb` |
| Memberships | 28 | `fd3bc9401f1389c7fa08cff816e7b44b` |
| Other original drafts | 3 | `56ecca403e09c456247b908907485805` |
| Users | 3 | `7bc139f2aea02af7488ad90b2d8f0cca` |

No images were uploaded in this slice. Rebuild-generated `.grayscale` utility
was removed narrowly from the integration worktree's already-dirty
`assets/tailwind.css`; its starting SHA-256 is restored exactly:
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
The user's CSS remains uncommitted. Interception was cleared, Network
observation disabled, viewport reset and both recovery tabs/main temporary
test tab closed. The normal original mock-owner session remains signed in.

Explicit second-pass self-review: **PASS for this scoped gate**, with the
same-tab recovery risk and autosave-warning overwrite fixed and retested.
Reviewed live/deferred guards, older-generation feedback, retained draft
identity, native/WASM compilation, actual commit-versus-response evidence,
manual reconciliation, original-row integrity and honest evidence limits.
No independent review agent was used.

**Remaining:** other mutations' post-commit lost responses, failed final
navigation after successful operations, identity replacement across open
pages, broader image edge cases, full accessibility/real devices and final
whole-app acceptance. UI-only guards do not supply server-side idempotency,
cross-tab concurrency guarantees or automatic recovery of unsaved changes
after reload. Destructive/account-linking replay still requires its separate
authorization and fresh isolated fixtures. Checkpoint 8 remains open.

## Confirmed recipe-write navigation and identity-switch audit — 2026-10-01

### Contract and implementation

After a confirmed recipe publication or save-and-return, preserve the app
shell when the destination cannot be read. Recover by retrying the read, not
by repeating the completed mutation. Keep the existing destinations,
authorization, draft revision rules and publication/history contracts.
Discard and the new editor's ordinary Cancel remain unchanged.

Before the fix, the new editor published the private synthetic fixture below
and received a real HTTP 200 response, held at response stage. SQL confirmed
v1 existed before connectivity was disabled and the response continued.
Its subsequent `location.set_href` discarded the app document and reached
the browser's `ERR_INTERNET_DISCONNECTED` page instead of app recovery.
The browser's URL policy prevented inspection of that generated error page;
no safety-policy bypass or document-request interception was attempted.

`c004795` replaces the two editors' confirmed-publication redirects with
typed Dioxus router transitions. Save-and-return now retains an
`Option<Route>` destination and pushes it only in the successful save branch.
Published recipes still return to their detail page; unpublished drafts
return to Dashboard. The destination's existing load recovery remains usable
without a new document request. This is one indivisible runtime concern, with
tests in the same commit; evidence is recorded separately.

### Existing-deployment browser evidence

Reused only `noms-c799915fcf93e034`, app `http://127.0.0.1:32768`, owner
`testuser` / `@user-e13c`. Only the app container was rebuilt/restarted;
database, media and other deployments were retained. Existing isolated task
worktree was reused and integration remained fast-forward-only.

For each failure check, a real successful mutation response was held using
response-stage Fetch/XHR interception, its committed state checked with
read-only SQL, and the browser made offline before continuing the response.
No synthetic success body or document interception was used.

| Confirmed operation | Failed destination | Recovery observed |
| --- | --- | --- |
| Existing editor publication to v2 | Recipe detail | App shell and `Recipe unavailable` retained; offline keyboard retry remained focused; online retry loaded v2 and focused `recipe-detail-content`. |
| Existing editor save-and-return | Recipe detail | Saved draft remained durable; `Retry recipe` recovered the published v2 view, without publishing the unpublished notes. |
| New editor publication | Recipe detail | New editor (still mounted after its initial draft URL replacement) reached app recovery and online keyboard retry loaded its one private recipe/v1. |
| Unpublished editor save-and-return | Dashboard | Shell retained independent draft, library and collection retry controls; online retries recovered the library and the saved draft reopened with its latest notes. |

Network event observations contained exactly one mutation for each observed
save/publication and no new `Document` request after its response. Read retries
did not repeat publication or saving. For the existing-publication check,
`performance.timeOrigin` stayed `1790905778898.5` across publication, failed
read and successful retry, corroborating the retained document.
Offline ancillary history/lineage reads exposed their existing independent
retry controls; retrying them restored the actual two-version history.

At 320×720, measured `innerWidth=320`, document `scrollWidth=305`, keyboard
focus remained on `Retry recipe`, and screenshot review showed bounded copy,
reachable controls and no horizontal overflow. Evidence:
`/tmp/noms-020-navigation-offline-320.jpg`.

Retained synthetic fixtures, all owned by the existing owner:

- Recipe `5a6ceeff-cbc0-48fc-8ced-2a0f8e431fc2`, **CP8 Navigation recovery —
  Local test only**: private, exactly two published versions. v2 notes are
  `Synthetic confirmed publication before destination failure.` Its follow-up
  draft `6bab2d64-693c-4545-a6fc-c6294b54ea6e` is revision 1, notes
  `Synthetic save-and-return survives failed destination load.`
- Recipe `fdc3842b-a7b3-40fe-9106-81dd3da128bd`, **CP8 New navigation recovery —
  Local test only**: private, exactly one published version; its initial draft
  `91f57f8d-3e63-4eae-8c05-7408f5393517` was consumed by publication. The first
  rapid entry hit incomplete hydration; the retained initial draft was
  completed rather than creating another one. No before-autosave claim is
  made for this final new-editor navigation check.
- Unpublished draft `c4021f56-c3ef-4785-9528-898fc5068ecf`, **CP8 Dashboard return
  navigation — Local test only**: revision 1, no linked/published recipe, notes
  `Synthetic unpublished return with a failed dashboard read.`

Before/after row hashes match after excluding only those two new recipes,
their versions and two retained drafts:

```text
recipes     34  1f871abe478874372b39ff13d0049cc6
versions    68  5c4aae0621f38f2818f2a68c05b09b03
drafts       7  3fa0afdc23298fa289888dc6fb9269b9
collections 30  9f2f90802adafb72eca3ea15fc0bbacb
users        3  7bc139f2aea02af7488ad90b2d8f0cca
```

No media upload, collection write, profile save, account link, discard or
deletion was performed. The synthetic fixtures remain for replay.

### Identity-switch audit: failed acceptance, not implemented

Kept the owner's hydrated Settings tab open. In a second tab, used native
Sign Out and normal mock Google sign-in to the **existing** retained avatar
identity, `cp8avatarroundtrip20261001` / `@cp8-avatar-round-trip-lo-6894`.
No new account or provider link was created.

The original Settings tab still linked to `/u/user-e13c`, displayed the
original profile, showed no sign-in-status notice, and allowed the valid local
display-name entry `CP8 stale form` with **Save changes enabled**. It was not
submitted. Fresh Settings in the second tab showed the avatar identity.
Navigating the original tab to Dashboard fetched the replacement identity's
empty library while its **My profile** link still pointed to the old identity.
Screenshot evidence: `/tmp/noms-020-identity-stale-settings.jpg`.

A fresh private-recipe read under the replacement identity correctly showed
`Recipe unavailable`, with zero original-title or Edit links. This confirms
the tested server read denial, **not** safe identity binding for mutations.
Source inspection corroborates the write risk: `save_profile` accepts name,
bio and optional username, then selects its target from the current request's
session; it does not receive the identity that initialized the form.
An unintended cross-account write is an inference from that contract,
not a tested overwrite. No profile data changed; the user-row hash matches.

This gate is **not accepted**. A safe follow-up needs explicitly scoped
identity binding for form/creation/account writes, rejection when the current
session differs from the form's initiating identity, and useful retained-entry
feedback. Client focus refresh alone leaves an action-time race; silently
reinitializing a form could lose entries. Audit the related avatar/new-recipe/
new-collection paths and delayed callbacks, with server-side tests before
changing their contracts. Request scope approval rather than treating an
authorization/API change as a styling adjustment.

Original `testuser` session restored through normal mock sign-in. The
unsubmitted local Settings text was abandoned by navigation; no profile save
occurred. Saved synthetic draft notes reopened under the restored owner.

### Validation and review

Tests-first: the two new wiring regressions failed against the old redirects,
then all seven draft-save tests passed after implementation. These are source
wiring checks, not an automated browser navigation harness; the browser,
network and SQL observations above provide the live behavioral proof.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=<integration-worktree>/target \
  cargo test --features server draft_save -- --test-threads=4
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true \
  CARGO_TARGET_DIR=<integration-worktree>/target just ci
```

Full CI on `c004795`: **935 passed, 0 failed, 12 ignored**, exit 0, including
native/WASM checks, Clippy, formatting and repository policy gates. Eight
existing allowed advisory warnings remain; no fresh ignored storage/search
integration run claimed. Logs: `/tmp/noms-020-navigation-red.log`,
`/tmp/noms-020-navigation-green.log`, `/tmp/noms-020-navigation-ci.log`.
Existing deployment package artifacts were invalidated before the build;
client/server builds succeeded in 22.01s/43.04s. Build log:
`/tmp/noms-020-navigation-build.log`.

Explicit second-pass self-review: **PASS for confirmed recipe-write navigation**;
destinations, successful-save ordering, no-repeat-write evidence, durable
snapshots, narrow keyboard layout and unchanged server contracts reviewed.
No independent review agent was used. **FAIL/open for the separate identity
gate**, as documented; whole-app acceptance is not implied.

Successful-test tab interception/network overrides were cleared and viewport
override reset. The preliminary generated offline-error tab was blocked by
browser URL policy even for explicit reset/close; it remained unmarked for
the product's automatic temporary-tab cleanup. No policy bypass was used.
Other temporary tabs were closed explicitly. Existing integration CSS edits
were preserved byte-for-byte by removing only the build-generated grayscale
utility; SHA-256 `74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.

**Remaining:** scope/resolve the confirmed identity-binding gap, other
mutations' post-commit outcomes and final-navigation paths, image edge cases,
full accessibility/real-device checks and final whole-app acceptance.
Checkpoint 8 remains open.

## Account-bound form mutation acceptance — 2026-10-01

The user approved expanding the preceding identity audit into form/API write
guards. `38c233e` supersedes the earlier **unimplemented/unaccepted form-binding**
finding, but not the separate mixed navigation/session-display finding.

### Contract and implementation

- Capture the initiating authenticated account once per mounted form; never
  rebind its retained entries to a replacement account. A form mounted before
  authentication can latch the first authenticated identity once.
- Require that captured ID to match the **verified request session** before
  budgets, storage access or mutation queries. The supplied ID is a consistency
  check, never authorization. Signed-out requests fail as unauthenticated;
  mismatches fail with controlled `Account changed` feedback. Older callers
  omitting the expected ID fail closed, so this is an API contract change.
- Cover all 27 UI server-function mutations: 11 recipe, nine collection, two
  upload, four Settings profile/avatar/deletion and one account unlink. Bind
  `get_recipe_draft_api` and `get_avatar_edit_source` editing reads too.
  Existing ownership, revision, quota and media-lifecycle checks remain.
- Preserve entered text, selected files and image edits on rejection. Known
  pre-write account rejection is not an uncertain draft-creation/publication
  outcome. Collection picker mismatch handling must not reconcile against the
  replacement account's memberships and report false success.
- Profile/avatar callbacks check initiating, current client and returned
  account IDs before replacing local/auth data. Successful account deletion
  no longer posts a follow-up logout that could clear a replacement session;
  deletion already invalidates the original account's server sessions.

The legacy direct multipart `POST /api/upload` handler and normal OAuth
login/link/logout flows are **not** covered by this server-function guard.
Cross-tab synchronization of cached profile links, auth gates and navigation
is not implemented by this change. Bound requests protect writes, not every
client display or post-commit interaction.

### Automated acceptance and review

Eight new tests cover verified identity matching/missing sessions, one-time
form identity capture, stale profile completions, all guarded server-function
entry points, direct unauthenticated draft/upload/profile rejection, safe
profile mismatch feedback and absence of deletion's follow-up logout.
Source-wiring assertions establish guard placement; they are not runtime
exercise of all 27 mutations.

An explicit old-behavior replay temporarily removed only the profile mismatch
feedback branch. Its focused test failed because the old message advised a
blind retry instead of explaining a definitive rejection. The branch was
restored before commit and validation. Early test setup/compiler failures
are not claimed as this intentional red result.

```sh
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true \
  CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target \
  just ci
```

Final full CI on `38c233e`: **943 passed, 0 failed, 12 ignored**, exit 0;
native/WASM checks, Clippy with warnings denied, formatting, schema/workflow
and repository policy/advisory gates passed. Eight existing allowed advisory
warnings remain; ignored storage/search integration tests were not freshly
run. Logs: `/tmp/noms-020-account-binding-red.log` and
`/tmp/noms-020-account-binding-ci-final.log`.

Explicit second-pass self-review: **PASS for the scoped bound UI mutation
contract**. Fixed findings included generational read borrows held across
awaits, definitive rejection incorrectly entering uncertain-outcome handling,
replacement-account picker reconciliation, stale profile completion and the
deletion follow-up logout race. The deletion API comment was corrected in the
documentation follow-up. No independent review agent was used; whole-app
acceptance and destructive runtime acceptance are not implied.

### Existing-deployment browser acceptance

Reused only `noms-c799915fcf93e034` at `http://127.0.0.1:32768`, mounted from the
integration worktree, to preserve the established fixture state. Invalidated
only this deployment's NOMS client/server package artifacts, then built and
restarted its app; no database/media/dependency cleanup. Client/server builds
succeeded in 20.02s/41.61s; `/tmp/noms-020-account-binding-build.log`.

Prepared hydrated forms under original mock `testuser`
(`6b1c56b4-945d-44d5-9f1b-eeb9525b8519`), then switched the shared session using
normal mock sign-out/sign-in to existing synthetic account
`cp8avatarroundtrip20261001` (`89f18aba-5a00-47d1-9d86-ea7330fe9a2c`). No new
identity/provider link was created.

| Acceptance criterion | Direct observation |
| --- | --- |
| Guest and replacement identity rejection | Original Settings Save first failed unauthenticated, then returned actual server `Account changed` under the replacement cookie. Sanitized network observation confirmed the submitted expected ID stayed the original account ID. |
| Retained profile entries and usable feedback | Synthetic unsaved name/bio remained unchanged after rejection; Save retained focus. At actual 320×720 viewport, document width 305px stayed within the viewport; Enter, Tab and Shift+Tab remained usable. |
| No replacement-account collection write | Original New Collection retained its name/description after rejected Create. |
| Retained upload and edits | Original collection-cover upload and Retry rejected; reopening its editor still showed the selected image and 15° rotation. Closing returned focus to Edit. No upload succeeded. |
| Definitive draft rejection | Original New Recipe's autosave and manual Save Draft rejected under the replacement account; title/notes remained and no initial draft was created there. |
| Original-account recovery | Restored `testuser` normally; the still-mounted recipe and collection forms saved successfully. Reloaded the draft and autosaved a notes edit to revision 1 under its initiating owner. |

Desktop evidence: `/tmp/noms-020-account-binding-profile.jpg`; narrow evidence:
`/tmp/noms-020-account-binding-profile-320.jpg`. Temporary local screenshots
are not tracked assets. The narrow capture is at the tested scroll position,
not a full-page screenshot. No unlink, avatar removal, recipe deletion or
account deletion browser replay was performed; those paths have source/unit
coverage only in this pass. The consumed `@cp8settings` fixture was not recreated.

### Integrity and retained fixtures

Read-only SQL hashes used ordered complete row JSON. Rejected requests left
original users, recipes, versions, drafts and collections byte-for-byte
unchanged. Initial media hash changed during app restart because the normal
lifecycle worker updated cleanup metadata and retired a pre-existing,
unreferenced synthetic avatar asset `c0c31528-db1f-4790-8b39-d9943034a1f8`.
All ten media records predated this test; this is not a rejected upload or
new asset, and no direct S3 byte/deletion verification is claimed here.

After establishing the post-restart baseline, replayed the wrong-account
profile/collection/upload/draft requests. **All six table hashes matched that
baseline**, including media. After legitimate same-account recovery, original
rows still matched, excluding only the two deliberately added private fixtures:

| Original rows | Count | Final MD5 of ordered row JSON |
| --- | ---: | --- |
| Recipes | 36 | `5e44d4edd748ba5eb5ed6c240ce50f57` |
| Versions | 71 | `7de62836d36d6aa66878fa55b8e50e90` |
| Drafts, excluding new fixture | 9 | `2e4e0e0034eee5b24668415da1cdca78` |
| Collections, excluding new fixture | 30 | `8878d8e670a274587bc83821644a7d46` |
| Users | 3 | `f414496eebb3dc762dc189b668b7f52b` |
| Media, post-restart baseline | 10 | `36b9c4ea3d2675f00c15741744d25855` |

New fixtures, both owned by original `testuser`, are retained:
unpublished draft `c082c066-26cc-47f9-943e-c48b862184e5`, revision 1,
and private collection `8ac29d43-3c4c-4969-8cd9-213023f73954`, each titled
`CP8 Account binding — Local test only`. Totals are ten drafts and 31
collections; recipes/versions did not increase. No successful profile save,
accepted cover upload or publication occurred. Unsaved synthetic Settings
entries and the rejected cover selection were abandoned only when their
temporary test tabs were closed.

Original `testuser` session restored. All five created tabs closed, network
observation disabled, viewport/device overrides cleared. Other deployments
and native browser tabs were untouched.

### Integration and remaining gates

Reused task branch `codex/NOMS-020-checkpoint-8` in
`.worktrees/NOMS-020-checkpoint-8` and fast-forwarded the established integration
branch `codex/NOMS-020-visual-design-refinement`. Runtime code/tests are one
indivisible contract commit (`38c233e`): server signatures and every caller
must change together. Evidence/plan/comment cleanup is a separate follow-up.
No push. Root roadmap dirt and integration user CSS are outside this task;
remove only the build-generated grayscale utility to preserve starting CSS
SHA-256 `74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.

**Remaining:** mixed cross-tab navigation/session display, direct multipart
and OAuth flow scope/audit, other mutations' post-commit outcomes and
final-navigation paths, remaining image edge cases, full accessibility and
real-device checks, and final whole-app acceptance. Any further destructive
test needs a new isolated fixture and action-time approval. Checkpoint 8
remains open.

## Session refresh and protected-page lifecycle — 2026-10-01

User approved the next planned session/navigation consistency step. Reused
the dedicated checkpoint-8 worktree and local fast-forward integration flow;
root roadmap dirt and integration user CSS remain outside this task.

### Contract and ordered implementation

1. `d7b3013`: recheck the verified session at hydration, client route changes,
   window focus/pageshow and visible-document activation. Events during a
   pending check coalesce into one follow-up, so a cookie replaced mid-request
   is not silently missed. Failed/malformed checks retain the last context
   and the existing explicit recovery control; local profile updates still
   invalidate stale refresh responses.
2. Separate read views from retained editors. Dashboard/library, collections,
   Search and other read-only route trees reset on authenticated identity
   change; profile-only updates do not reset them. New/edit recipe and
   collection routes and both Settings pages retain their mounted state,
   but hide/inert the original editor behind an account-change notice.
   Returning to the initiating account reveals it without an automatic reload.
   A native new-tab link lets the user start fresh without discarding the old tab.
3. Preserve native modal state while suspending background inertness and scroll
   locking. Restore the existing modal and its pending/focus stage when the
   original account returns. Hidden top-layer dialogs/popovers cannot remain
   visually exposed; account menus/navigation drawers close on identity change.
   Fresh guests never mount protected children; an already-mounted form stays
   hidden across sign-out. A replacement avatar cannot initialize the old form.
4. `37935f4`: browser review found that a key on a single static Dioxus outlet
   did **not** remount its resource scope. The first build updated profile links
   but kept the original library. Fixed this using intentional one-item keyed
   fragments for read identities and retained route visits. Added an actual
   mount/cached-data regression, rather than relying on key-string wiring alone.
   Added a 30-second visible-tab fallback for concurrently visible windows and
   embedded browsers that do not emit activation on automated tab selection.
   Hidden tabs do not poll; listeners and interval registrations are owned and
   removed on shell teardown. No identity/token broadcasting or storage was added.

This is **bounded eventual consistency**, not an immediate cookie-change
notification mechanism. Failed checks cannot establish a new identity; their
last-known context remains until recovery. Every request's server permission
checks and the preceding account-bound write guards remain authoritative.
Previously loaded original entries remain in the local browser's hidden DOM/
component state; this preserves work, not erasure of cached private information.
Every in-flight read/identity race and native OS/browser activation is not newly
accepted by these tests. Direct multipart upload and OAuth contracts are unchanged.

### Tests and review

Eight added tests cover activation/route/listener wiring and cleanup, event
coalescing, route policy, hidden/inert/top-layer behavior, one mounted form
through original → guest → replacement → original, retained-modal suspension/
scroll/pending focus, and actual data-scope remount through replacement, guest
and return visits. Existing auth/permission/profile/overlay tests still pass.

The initial route-wiring regression failed against the old shell. The browser
cache-remount bug was then reproduced by a focused test: mount count stayed
one when it should be two. It passed after the keyed-fragment change. Logs:
`/tmp/noms-020-session-page-red.log`, `/tmp/noms-020-session-read-red.log`.
Missing-method compile failures during initial test setup are not claimed as
behavioral red results. The inert assertion was corrected to Dioxus's actual
boolean serialization, without removing the inert requirement.

```sh
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true \
  CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target \
  just ci
```

Full CI for the reviewed `37935f4` runtime: **951 passed, 0 failed, 12 ignored**,
exit 0. Native/WASM checks, Clippy, formatting, schema/workflow and repository
policy/advisory gates passed. Eight existing allowed advisory warnings remain;
ignored storage/search integration tests were not freshly exercised.
Raw log: `/tmp/noms-020-session-ci-reviewed.log`.

Explicit second-pass self-review fixed the ignored static key, native modal
inertness/scroll trap, omitted raw empty-string inert attribute and possible
overlapping signal-write guards in the queued-refresh loop. No independent
agent was used. Browser acceptance and cleanup results follow below; this
section does not close checkpoint 8 or final accessibility acceptance.

### Runtime setup and completed early checks

Rebuilt only existing `noms-c799915fcf93e034` at `http://127.0.0.1:32768`.
Only its NOMS client/server package artifacts were invalidated; no DB/media
or dependency cleanup. Final build succeeded in 23.27s/49.38s:
`/tmp/noms-020-session-build-reviewed.log`. First cached development assets
caused a hydration error; target-scoped cache bypass and a fresh reload
resolved it. Hydrated readonly-state and native controls were checked before
testing. The always-present hidden rebuild overlay is not an app failure.

Original mock `testuser` and existing `cp8avatarroundtrip20261001` identities
were switched using normal mock sign-out/sign-in. No new identity/provider
link was created; no content/profile save, publication or upload was submitted.
Under the first build, simulated focus events were used because the embedded
browser reported all tabs focused/visible and did not emit real activation
when automation selected a tab. Real verified guest/replacement responses
hid the original Settings form while retaining the synthetic name/bio.
The open collection-cover image editor suspended with no native open dialog,
released body scroll locking and focused the visible account notice.
Restoring the original account resumed that same open editor with its unsaved
15° rotation and restored the original profile text. These lifecycle paths
are unchanged by the read-scope/fallback correction; final-build checks are
recorded separately below.

### Final-build browser acceptance and integrity

Final `37935f4` was tested in fresh Dashboard/Settings tabs with no staged
entries, leaving the earlier retained test tabs untouched. No simulated focus
event or page reload was used to trigger these identity transitions:

- Original Dashboard showed **9 saved recipes, 10 drafts** and `/u/user-e13c`.
  After native sign-out elsewhere, the visible-tab fallback replaced its
  workspace with the guest sign-in gate and retained `/dashboard` return context.
- Native sign-in to the existing replacement account produced **0 recipes,
  0 drafts, no collections**, and `/u/cp8-avatar-round-trip-lo-6894` in that
  same Dashboard. No original library/collection scope survived the remount.
- The original Settings route showed the account-change notice with its editor
  hidden/inert and the notice heading focused. Tab moved to the safe native
  new-tab link, not into original fields; zero original inputs/buttons were
  visibly laid out. Actual 320×720 geometry: document width **305px**, no
  horizontal overflow. The viewport override was cleared afterward.
- Native mock sign-in restored original `testuser`; no fixture/profile/content
  mutation was submitted by this pass. The same final Dashboard returned to
  nine recipes, ten drafts and `/u/user-e13c`; its original Settings editor
  became visible again with unchanged initial fields. Timer detection can take
  up to its interval plus request latency; no stronger instantaneous guarantee
  is claimed.

Screenshots, local temporary evidence rather than tracked assets:
`/tmp/noms-020-session-account-notice.jpg`,
`/tmp/noms-020-session-account-notice-320.jpg`,
`/tmp/noms-020-session-replacement-dashboard.jpg`.
Desktop/narrow captures were visually inspected. Earlier hydration console
errors are from cached assets before fresh loading, not the accepted hydrated
flow. Real activation events and hidden-tab timer behavior have source/unit
coverage here, not independent OS/browser runtime acceptance.

Read-only ordered-row JSON hashes matched before and after native session
switching, including the two prior account-binding fixtures; no new fixtures
were added. All content/account rows and the post-restart media baseline match:

| Table | Count | MD5 of ordered complete row JSON |
| --- | ---: | --- |
| Recipes | 36 | `5e44d4edd748ba5eb5ed6c240ce50f57` |
| Versions | 71 | `7de62836d36d6aa66878fa55b8e50e90` |
| Drafts | 10 | `99d240e9afcfe8672c286906c1bb7111` |
| Collections | 31 | `dfb9250a2fc39efe4473234d2f7614d2` |
| Users | 3 | `f414496eebb3dc762dc189b668b7f52b` |
| Media | 10 | `66704ecf395a30cfdf2b593ec94ded0b` |

Media baseline is after normal restart lifecycle processing, not the previous
turn's pre-restart hash. No S3 byte/deletion proof is claimed. Authentication
session rows naturally changed through the normal login/logout flows and are
not included in these content-integrity hashes. No destructive browser flow
or direct database write was performed; consumed `@cp8settings` was not recreated.

Auto-review blocked reloading tabs with non-trivial unsaved state. Requested
approval to discard only the synthetic profile entries/cover selection;
continued final verification in fresh unmodified tabs instead. Until approved,
the two earlier retained test tabs remain handed off, not silently closed.
Temporary cache/network/viewport overrides were reset; all four unmodified
test tabs were closed. Other native tabs/deployments remain untouched.
Integration user CSS retains its exact starting SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`;
only the build-generated grayscale utility was removed. Final verification
confirmed HTTP 200, a clean task worktree and only the pre-existing user CSS
edit in integration. Root roadmap changes remain untouched.

Second-pass review verdict: **PASS for scoped session refresh, identity-keyed
read reset and retained-editor/modal lifecycle**, including the browser-found
keying bug corrected before acceptance. Ordered runtime commits are
`d7b3013` → `37935f4`; evidence/plan updates are a separate follow-up. Reused
task branch `codex/NOMS-020-checkpoint-8` in `.worktrees/NOMS-020-checkpoint-8`,
fast-forwarded `codex/NOMS-020-visual-design-refinement`; no push.

**Next:** other mutations' post-commit response-loss/final-navigation outcomes,
remaining identity races/direct multipart/OAuth scope, image edge cases and
full accessibility/real-device/final whole-app acceptance. Checkpoint 8 is
still open; destructive tests require fresh isolated fixtures and action-time
approval. Preserving local test tabs pending cleanup approval is not whole-app
acceptance or authority to discard any user-owned work.

## Early collection input investigation

On 2026-10-01, investigated the remaining collection-input question at
`4bb28b5` on the existing `http://127.0.0.1:32768` deployment. This was diagnosis
and documentation only: no application source change, rebuild, content save,
upload, sign-in change or direct database write. The investigation identified
two current editor lifecycle defects requiring remediation. It does not prove
the exact cause of the earlier rapid-navigation observations.

### Historical evidence and source boundary

The first recorded observation, during the 2026-09-27 subcollection check,
said an immediate fill was not retained during initialization. The later
populated-fixture check described filled DOM fields followed by a saved default
name and empty description. Neither record includes a timestamped event trace
or exact client initialization boundary. A normal browser creation passed before
the submit fix, so neither observation supports an unconditional save failure.

The original shared CollectionForm in `c1d5e1f` already used controlled signals,
input-event updates and a submit callback that did not read submitted fields.
CollectionNew initialized the name to Untitled Collection and the description
to empty. `2981abf` changed layout and pending guards, not that signal-only
contract. Therefore the missed-event vulnerability exists in the original
collection implementation, rather than being introduced by the visual overhaul.
This is a source-level introduction boundary, not runtime proof of the historical
browser trigger. `3ba9b9a` added named-field submission capture, with previously
recorded behavioral red/green regressions; those tests do not cover node removal
or loss of text before submission.

### Controlled browser comparison

Paused only the observed `/wasm/noms.js` Script request in a fresh temporary tab,
with tab-scoped cache bypass. The server-rendered name and description were
editable while the client hydration callback was absent. Filled synthetic text
through the normal field controls, then released the script. Never pressed
Create collection or submitted a mutation.

| Case | Server route in hydration payload | Browser route | Result after client initialization |
| --- | --- | --- | --- |
| Top-level form, first run | Not decoded in this run | `/collections/new` | Filled name/description replaced by defaults; enabled-looking form had inert ancestors. |
| Nested form | `/collections/new?parent=7f535c8e-d0a1-4052-87a5-65b203479917` | Same route | Original input nodes stayed connected and retained both synthetic entries. Native account-menu interaction confirmed the client was interactive. |
| Top-level form, repeat | `/collections/new?` | `/collections/new` | Original filled nodes disconnected; replacement nodes held Untitled Collection and an empty description. |

The repeat retained references to the original nodes and temporarily traced
their native value setters without changing setter behavior. No value assignment
to those nodes was recorded; the nodes were removed and replaced. It is therefore
inaccurate to describe this particular failure as a hydration setter overwriting
the original inputs. The current failure is a form remount.

The raw route key in SessionPage was introduced by `d7b3013`:
`form-{router().full_route_string()}`. Installed Dioxus 0.7.9 FullstackHistory
uses its serialized initial route during hydration, then reads browser history
and schedules a rerender. The top-level route's empty trailing query is present
in the serialized server route but absent from browser `location.search`.
Together with the key and observed node replacement, this establishes the
current top-level startup trigger. The valid nested parent route has no such
difference and did not remount in this comparison. This newer regression cannot
explain observations recorded before `d7b3013` existed.

### Active editor inert defect

After the top-level remount, both RetainedFormPage and AuthRequired wrappers
had `inert="false"` while the original account was still authenticated, no
account-change notice existed, and neither wrapper was hidden. Native browser
inspection reported `element.inert === true` for both wrappers; the form
disappeared from the accessibility tree despite remaining visually present.

The installed Dioxus interpreter's boolean-attribute removal list does not
include inert; a false value is written as an attribute instead of removed.
Both wrappers were added in `d7b3013`. Enabled editors must omit the attribute,
not serialize false. The nested form did not acquire these attributes in this
run, which further shows why an initial SSR snapshot or unit renderer alone
is not adequate acceptance. This finding reopens the earlier scoped lifecycle
acceptance; it is not a fix or a claim that every editor route was checked.

### Verification and bounded next action

Reran from the checkpoint worktree:

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server collection_form::tests -- --test-threads=1
```

Result: **5 passed, 0 failed, 0 ignored**. No fresh full CI run or new historical
red run is claimed. The unchanged submit-capture tests remain valid but do not
certify startup or native inert behavior.

Next, fix stable startup form identity and native inert omission, then verify
top-level/nested early input, original-account keyboard editing, and account
mismatch/recovery without losing text or image state. Before closing this gate,
also exercise missed-event submission on the stable form. The historical exact
event sequence remains a recorded limitation, not a reason to claim this newer
root cause or indefinitely repeat unguided historical tests.

Temporary local evidence:
`/tmp/noms-020-collection-early-before.jpg`,
`/tmp/noms-020-collection-nested-early-retained.jpg`,
`/tmp/noms-020-collection-top-level-early-lost.jpg` and
`/tmp/noms-020-collection-top-level-early-trace.json`. Screenshots were inspected;
the trace distinguishes disconnected original fields from current defaults.
These temporary artifacts are not permanent baseline assets.

Removed temporary setter instrumentation and cleared script interception and
cache/network overrides. Closed the repeat top-level tab after confirming its
synthetic entries had already been lost and only default values remained. The
nested synthetic unsaved entries are retained pending cleanup approval; the
two earlier profile/cover test tabs remain preserved and were not reloaded.
Other deployments and user tabs were not changed. Integration user CSS retains
SHA-256 `74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.

Second-pass self-review: **diagnostic evidence PASS; editor lifecycle acceptance
FAIL pending the two fixes; overall checkpoint 8 OPEN**. No independent-agent
review claimed. Reused the existing checkpoint task branch for this single
cohesive documentation update; no runtime changes to roll back.

## Collection startup and native editor recovery acceptance 2026-10-02

The two defects from the preceding investigation are fixed in `34c5bfe`.
Browser follow-up found and fixed one related handoff defect in `dfddc11`:
retaining the input nodes alone preserved early text only until a later edit
rerendered the form. CollectionForm now captures both live fields at client
mount, so Rust signals match the values entered before hydration.

### Scope and implementation

- Retained-form keys omit only empty query/fragment delimiters. Nonempty raw
  query and fragment text stays significant; a real parent change still starts
  a new form scope. The original path remains available for context and links.
- RetainedFormPage and AuthRequired emit inert only when blocked, and remove
  the attribute when enabled. Native HTML treats even `inert="false"` as inert.
- The collection form reads its own mounted input and textarea using typed Rust
  browser APIs, capturing both values before updating either signal. The same
  helper snapshots submitted named fields before parent validation. No global
  DOM query or production JavaScript event bridge was added.
- Reused the isolated checkpoint worktree and fast-forwarded the existing
  integration branch. Main, unrelated roadmap edits and other deployments were
  not changed. The running app at `http://127.0.0.1:32768` was rebuilt/restarted.

### Automated evidence

The route-key test reproduced replacement of the actual collection input node
on `/collections/new?` → `/collections/new`. The inert test reproduced native
renderer `Bool(false)` mutations. Both failed before `34c5bfe` and passed after.
Seven session-page tests include real mount retention and genuine route resets.

The mount-listener regression failed before the handoff fix; all seven
collection-form tests then passed. Signal tests cover capturing both early
values and editing either sibling. Native tests do not execute browser DOM
hydration; the controlled browser checks below provide that evidence.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server auth::session_page::tests -- --test-threads=1
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server components::base::collection_form::tests -- --test-threads=1
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
```

Full CI on the final runtime changes: **956 passed, 0 failed, 12 ignored**;
native/WASM checks, Clippy, formatting, schema/workflow/policy gates,
tracked-secret scan, deployment artifacts and dependency audit passed. Eight
existing warnings remain allowed. The advisory-cache check required reviewed
access outside the worktree. No fresh ignored search/storage integration run is
claimed. Logs: `/tmp/noms-020-startup-inert-red.log`,
`/tmp/noms-020-startup-inert-green.log`,
`/tmp/noms-020-hydration-handoff-red.log`,
`/tmp/noms-020-hydration-handoff-green.log` and
`/tmp/noms-020-hydration-handoff-ci.log`.

### Browser acceptance

Fresh forms used the rebuilt deployment, with only the client script paused
while entering synthetic unsaved name/description text. Script interception was
released before ordinary keyboard testing.

1. Top-level early name `CP8 fixed startup text 湯麵` and its description survived
   hydration in the same connected native nodes. Typing into the description
   retained the name; typing into the name retained the description.
2. The nested route with parent `7f535c8e-d0a1-4052-87a5-65b203479917` retained its
   original nodes and early Unicode text, including subsequent description and
   original-account name edits.
3. A synthetic local cover image remained in its unsaved editor with 15°
   rotation. Normal mock sign-out and sign-in as the existing alternate account
   hid the original editor, set native inert true and suspended its dialog.
   Both collection fields remained connected with unchanged values.
4. Restoring the original account removed inert entirely, restored the same
   fields and reopened the same image dialog at 15°. Native ArrowUp changed the
   rotation to 16°, demonstrating resumed keyboard input rather than just a
   visible DOM value. No image was applied or uploaded and no collection saved.
5. On a separate stable form, a diagnostic DOM value change cleared the name
   without emitting input. Temporarily bypassing native required validation let
   the real submit handler run; it displayed `Collection name is required.`
   rather than submitting the stale nonempty signal. Native validation was
   restored afterward. This probe made no content write.

Screenshots: `/tmp/noms-020-fixed-startup-handoff.jpg`,
`/tmp/noms-020-fixed-session-image-resume.jpg` and
`/tmp/noms-020-fixed-missed-event-validation.jpg`. These temporary files show
the focused runtime states, not a new visual baseline or whole-page acceptance.

### Integrity, cleanup and limits

Before/after counts and hashes using the same ordered JSONB SQL query match:

| Table | Rows | MD5 of ordered rows |
| --- | ---: | --- |
| recipes | 36 | 88e95c4d50e9b790acdc6828cefd592e |
| recipe_versions | 71 | 98f8d09f235988b761228e6833b3ba8e |
| recipe_drafts | 10 | 885e549be7306095015bc7acb012c876 |
| collections | 31 | b9657f26c904b1b594740a0a3e0ccfec |
| users | 3 | cbbdc47cfa8398755dd099086eca1572 |
| media_assets | 10 | 735c0a80a3576ac89c8efc91c6d85a21 |

Normal session rows changed through sign-in/out. Original account restored.
All script interception, cache/network overrides and temporary DOM references
were cleared. Closed only the clean sign-in tab and empty validation form.
Earlier unsaved profile/cover/nested probes and the new unsaved startup/image
forms remain preserved pending cleanup approval; no reload or deletion of
those entries. Integration CSS retains SHA-256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e` after removing
only the build-generated grayscale rule introduced by this rebuild.

Second-pass self-review: **PASS for the two fixes and bounded startup/recovery
acceptance**. No independent-agent review claimed. The exact historical rapid
navigation sequence remains unconfirmed and must not be attributed to these
newer regressions. Real-browser/OS activation, every identity race, full device
and screen-reader checks, and whole-app acceptance remain open. This closes
the first remaining checkpoint-8 group, not the entire checkpoint; five groups
remain. Rollback is a scoped revert of `dfddc11` and `34c5bfe`, not a workspace
reset or restoration over user edits.

## Graph gesture cancellation and accessibility follow-up 2026-10-02

**Synthetic gesture and native keyboard checks PASS; real-device and screen-reader
acceptance OPEN.** The next graph gate was revisited on the existing private
30-version history fixture, recipe `1c6173d6-505c-92d7-7493-97e30be37b85`, at
`http://127.0.0.1:32768`. This is not the older deployment's 62-snapshot fixture.
No new fixture or publication was needed.

### Finding and implementation

A synthetic two-finger touch held for more than the 550ms delay opened a
snapshot context menu. The handlers inspected only the first touch, so a
stationary multi-finger gesture could trigger the long-press action.
`3d9e731` restricts candidates to exactly one finger and cancels the existing
candidate on invalid starts or multi-finger movement. The token guards before
and after asynchronous geometry lookup remain unchanged, as do early-release
cancellation, the delay, and the 12px movement tolerance. The change does not
prevent default scrolling/pinch behavior for rejected gestures.

The coordinate helpers were first extracted without changing first-touch
behavior. Tests using Dioxus serialized touch data then failed for both a
two-finger start and a second finger joining. After the fix, those tests and
the single-finger boundary/diagonal/empty-touch tests passed.

### Browser and accessibility evidence

Native `Input.dispatchTouchEvent` is unsupported in the selected in-app browser.
The checks instead dispatched explicitly synthetic TouchEvents to the real
rendered graph card and exercised the Rust handlers/timer. This does not test
trusted compatibility clicks, native gesture arbitration or physical scrolling.

After rebuilding/restarting the same app container and reloading the clean
graph test tab:

- Two-finger start, another finger joining through touchstart or touchmove,
  movement of 13px, early release and touchcancel all left the menu closed
  after 650ms. The first multi-case debugger request timed out before returning
  its receipt; it was not counted as passing evidence. A fresh run with an
  adequate timeout returned all six closed-menu results.
- A single finger moved exactly 12px still opened the correct v30 snapshot
  menu after 700ms and focused View snapshot. Ending the synthetic touch and
  pressing native Escape returned focus to the v30 snapshot link without
  navigating. Native Shift+F10 reopened the menu; End/Home moved to Zoom to
  snapshot/View snapshot, and Escape restored link focus again.
- The accessibility tree exposed named graph controls, labeled snapshot links,
  menu roles and the current/outline state. The native outline disclosure
  opened by click, collapsed/reopened by Return, and its older-history control
  expanded the graph from 25 to 30 cards by Enter. Focus stayed on the same
  control, now labeled Collapse older versions. The outline includes explicit
  textual relationships instead of requiring interpretation of connector art.
- The graph expansion status is a polite, atomic live region. This confirms
  DOM semantics only, not whether a particular screen reader announces it at
  the intended time. No screen-reader executable or connected device was found
  in the available environment, and native app control is disabled.

Temporary inspected screenshot: `/tmp/noms-020-graph-longpress-fixed.jpg`.
The test tab was closed after recording results. Six earlier tabs with unsaved
synthetic profile/collection/image entries remain preserved; they were not
reloaded. No viewport, touch-emulation, interception or cache override was used
or left behind by this follow-up. Local mock sign-in restored the existing
fixture owner, without creating an account or changing profile content.

### Validation and integrity

On `codex/NOMS-020-checkpoint-8`, using the existing integration target:

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server pages::recipe_version_graph::tests -- --test-threads=1
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
```

Focused tests: **64 passed**. Full CI: **959 passed, 0 failed, 12 ignored**;
native/WASM checks, Clippy, formatting and all repository/audit gates passed.
Eight existing audit warnings remain allowed. No fresh ignored storage/search
integration run is claimed. Logs: `/tmp/noms-020-graph-gesture-red.log`,
`/tmp/noms-020-graph-gesture-green.log`, `/tmp/noms-020-graph-gesture-ci.log` and
`/tmp/noms-020-graph-gesture-build.log`.

The ordered JSONB content checks taken after initial probes/before rebuilt-client
testing and again after that testing match: 36 recipes, 71 versions, 10 drafts,
31 collections, 3 users and 10 media assets. Recipe/version/draft/collection/user
hashes also match the preceding acceptance record. Media's within-follow-up
hash is `d0a508add61d319d45f45fe1c393fbed`; it differs from the earlier record,
so no unchanged-media claim across the two turns is made. This slice performed
no content/media upload, save or delete. User integration CSS was preserved,
removing only the build-generated grayscale rule after restart.

Second-pass self-review: **PASS for the scoped fix and stated evidence**;
no independent-agent review. The fix was fast-forwarded into the existing
NOMS-020 integration worktree; main and other deployments were not changed.
Rollback is a scoped revert of `3d9e731`. Five acceptance groups remain open.

### Required manual graph handoff

- On a physical touch device, verify short tap navigation, deliberate long press,
  release without accidental navigation, scroll/movement cancellation, two-finger
  gestures and operating-system cancellation. Verify ordinary page/outline
  scrolling and that graph controls remain reachable at phone/tablet sizes.
- With an actual screen reader, verify snapshot/action identification, current
  state, outline relationships, expanded/busy announcements, comparison direction
  and focus restoration after dismissing menus. Confirm private/redacted content
  stays unavailable. Accessibility-tree observations alone do not complete this.

The next executable acceptance slice is the direct multipart upload/OAuth
identity-edge audit. Keep the graph manual gate open until the physical-device
and assistive-technology checks are recorded; do not repeat synthetic checks as
if they could certify those environments.

## Multipart and OAuth identity boundary acceptance 2026-10-02

The scoped admission checks pass. Commit `88b7a5c` adds two regression tests;
no production behavior change or deployment rebuild was required. The broader
identity acceptance group stays open for later races and real-browser activation.

### Contract and automated evidence

The direct `/api/upload` handler verifies its incoming cookie once before reading
the multipart body and keeps that account as the upload owner. It is a direct API,
not a retained form: repository search found no current UI call to this route.
The shared image UI uses the typed `upload_image` server function with its latched
form account instead. No new form-account parameter was added to the direct API.

- The new multipart test waits for a mocked S3 PUT, then revokes account A's
  session and creates account B's session while storage is delayed. A database
  assertion proves the upload has not completed at that point. The completed
  asset and key belong to A, B owns zero assets, and the response sets no cookie.
  A subsequent request with A's revoked token returns 401; B's valid token
  reaches multipart validation and returns 400 for the missing full image.
  This accepts original ownership, not cancellation of an already admitted write.
- The new OAuth route-wrapper test supplies an original session revoked before
  callback admission, plus the correct browser transaction binding. It returns
  401, makes zero token/user-info requests, adds no provider links to either
  account and consumes the state. Its sole Set-Cookie expires that transaction's
  cookie, not the session cookie; the replacement account's session remains valid.
- Existing OAuth tests cover a callback with a different account, missing
  sessions, active-session login rejection, transferred/malformed/mismatched
  browser bindings, expiry, replay, concurrent state consumption and successful
  matching linking/login in isolated temporary databases with mock providers.

Tests were written against the existing behavior and passed without a production
fix; no failing-before/fixed-after result is claimed. Temporary databases and
mock storage isolate the success/linking mutations from the running deployment.

### Existing deployment browser evidence

Used the existing deployment at `http://127.0.0.1:32768`, without restarting it.
Same-origin diagnostic fetches exercised the real endpoints; these were API
probes from the browser, not UI upload/link completion tests:

- Guest multipart request: 401, Authentication required.
- Guest OAuth link start: 401, Account linking requires a valid session.
- Normal Google mock sign-in to the existing `testuser` identity returned to
  Linked accounts. The page showed its existing Google connection and disabled
  Unlink because it is the only sign-in method.
- Authenticated multipart request missing `full_image`: 400, Missing full_image.
- OAuth login start with that active session: 409, OAuth login cannot complete
  with an active session.

The multipart probes contained only synthetic invalid thumbnail bytes and never
reached storage. No Connect/Unlink action, new account creation, valid live upload
or content save/delete was submitted. The new clean test tab was closed; six
earlier tabs with unsaved synthetic entries were preserved without reload.
No interception, viewport, cache or emulation override was introduced.
Inspected screenshot: `/tmp/noms-020-identity-linked-accounts.jpg`.

### Validation and integrity

From `codex/NOMS-020-checkpoint-8`, with the existing integration target:

```sh
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server api::upload::tests
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server multipart_upload_keeps_request_owner
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server auth::oauth::tests
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
```

Baseline upload tests: 25 passed. The initial new ownership test passed; the final
version adds explicit proof that session replacement precedes upload completion
and passes in full CI. OAuth tests: 57 passed. Final full CI: **961 passed,
0 failed, 12 ignored**; native/WASM checks, Clippy, formatting, schema/policy,
secret/deployment and dependency-audit gates passed. Eight existing allowed
audit warnings remain. No fresh ignored real-storage/search run is claimed.
Logs: `/tmp/noms-020-multipart-baseline.log`, `/tmp/noms-020-multipart-owner.log`,
`/tmp/noms-020-oauth-acceptance.log`, and
`/tmp/noms-020-multipart-oauth-ci-final.log`.

Before/after ordered JSONB hashes match for 36 recipes, 71 versions, 10 drafts,
31 collections, 3 users and 10 media assets. Media's within-turn hash is
`ebacf2f0a9b4072e4142de9d2efa1f8e`; it differs from the previous turn, so no
unchanged-media claim across turns is made. OAuth account count stayed at two;
its full-row hash changed after normal existing-account sign-in, so unchanged
OAuth rows are not claimed. No credentials or raw cookies were logged.
The user integration CSS remains byte-identical to its pre-turn checksum.

Second-pass self-review: **PASS for these scoped checks and stated limits**;
no independent-agent review. Test coverage and evidence are fast-forwarded into
the existing NOMS-020 integration worktree; main and other deployments remain
untouched. Removing the regression tests requires only reverting `88b7a5c`.

### Remaining checks and next slice

OAuth authority is checked at callback admission. These tests do not accept
revocation during later provider exchange, competing delayed login/refresh
response cookies, real-browser/OS activation, or every in-flight read race.
They also do not prove that an admitted multipart write is aborted after logout.
Those limitations stay explicit rather than changing authentication contracts
as part of this visual acceptance slice. Five broad acceptance groups remain.

Next: dense editor/image-state checks, including callback lifetimes and
cross-step interactions. Physical touch and screen-reader graph checks remain
a manual handoff, and final whole-app acceptance is still open.

## Gallery preview lifecycle and delayed-result acceptance 2026-10-02

The shared gallery now releases its local preview URLs on component teardown,
including ordinary SPA navigation. This closes one scoped image-lifecycle gap,
not the entire dense editor/image acceptance group.

### Change and regression evidence

The baseline browser created two object URLs for a picked image: its gallery
preview and the decoder's temporary URL. After processing and navigating to
Explore, only the decoder URL had been revoked. The gallery preview remained
allocated despite its DOM being removed.

`5d4cada` replaces raw preview strings with a shared ownership lease whose final
Drop revokes the URL. Render snapshots can clone the lease without revoking an
image still in use. Existing failure/removal paths drop their ownership; saved
remote image URLs are not leases and are never revoked. Upload/save behavior and
the visual design are unchanged.

Three regression tests cover last-copy ownership, component signal teardown and
removing a child gallery while its parent VirtualDom stays alive. The initial
tests failed to compile before the lease existed; the realistic child-removal
test passes on the lease implementation and asserts no duplicate revocation
when the remaining parent is subsequently dropped.

Post-build probes initially appeared to fail. Browser resource timing exposed
the cause: normal reloads still executed a 14,393,681-byte older WASM module,
even while an uncached diagnostic fetch retrieved the rebuilt asset. A forced
CDP `Page.reload` with `ignoreCache: true` loaded the final 14,393,597-byte module;
its asset SHA-256 is
`d86fc7fc0ee0de46141fa36f1f5a3c2a896b66f1c96a7688178c02ee0dd0a286`.
Temporary lifecycle markers were removed. Experimental explicit-cleanup and
weak-registry changes in `1d85e98` and `40f1afc` were removed by `c58ba7c`; they
are not part of the final implementation or evidence for a signal-order defect.

### Existing deployment browser checks

Rebuilt and restarted only the existing app container on
`http://127.0.0.1:32768`. Used blank new-recipe forms and synthetic local files;
no title/content edits, upload, draft save, publication or deletion was submitted.
URL wrappers counted calls without changing normal creation/revocation behavior.

- Valid landscape image: modal opened, Cancel retained a usable Ready preview.
  While mounted: 2 URLs created, 1 revoked, 1 outstanding. Ordinary navbar
  navigation to Explore: 2 created, 2 revoked, **0 outstanding**.
- Delayed encoder: a diagnostic wrapper held the first real canvas `toBlob`
  result. The gallery showed Processing and disabled Publish, Cancel and Add
  Step. Removing the item cleared the row and re-enabled those controls. After
  restoring the encoder and delivering the held result, no photo or modal
  returned and no client error was observed. This tests stale-result handling,
  not closure-memory reclamation or a permanently stalled encoder.
- Malformed PNG: the visible decode error appeared, the modal closed, and both
  URLs were revoked (2 created, 2 revoked, 0 outstanding). Removing that row and
  choosing a valid portrait PNG reopened the editor and reached Ready.
- A bounded synthetic DragEvent probe in the existing two-step private draft
  dropped the first photo onto the second step's text region. All four source
  photo labels/order stayed unchanged, the destination stayed empty, and
  drag-end cleared the dragging state. The source contract only supports
  within-step reordering. This is not native drag/touch acceptance or evidence
  of cross-step transfer support; no such feature was added.

Inspected screenshots: `/tmp/noms-020-gallery-preview-ready.jpg`,
`/tmp/noms-020-gallery-recovery-modal.jpg`, and
`/tmp/noms-020-gallery-recovery-ready.jpg`. The last shows the valid gallery's
Ready status after failure recovery. Numeric lifetime evidence is recorded in
`/tmp/noms-020-gallery-preview-acceptance.json`.

### Validation, integrity and review

Final focused command and result:

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server components::base::image_upload::tests
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
```

**41 focused tests passed; final full CI: 964 passed, 0 failed, 12 ignored.**
Native/WASM checks, Clippy, formatting, schema/policy, secret/deployment and
dependency-audit gates passed; eight existing allowed audit warnings remain.
An earlier CI run had one OAuth database setup PoolTimedOut failure. Its focused
rerun and subsequent full runs passed; no OAuth behavior regression is claimed.
Final logs: `/tmp/noms-020-gallery-final-green.log`,
`/tmp/noms-020-gallery-final-ci.log`, `/tmp/noms-020-gallery-final-build.log`.
No fresh ignored real-storage/search run is claimed.

Before/after ordered full-row JSONB hashes match for 36 recipes, 71 versions,
10 drafts, 31 collections and 3 users. Media count stays 10, but its full-row
hash changed during this turn; unchanged media rows are not claimed. No live
upload was submitted. Temporary diagnostic wrappers and cache override were
removed, and the clean test tab was closed. Six earlier unsaved test tabs were
marked for retention and were not intentionally reloaded or closed. A temporary
startup-error tab could not be explicitly closed because the browser policy
blocked its error-page URL; it was not marked for retention.

The integration CSS is byte-identical to its pre-turn checksum after removing
only the build-generated grayscale utility. Root main, unrelated working-tree
edits and other deployments remain untouched. Second-pass self-review: **PASS
for this scoped lease change and these stated limits**, with no independent
agent review. Rollback is the image-upload file's pre-slice version at `8706ef6`;
no schema/data migration was introduced.

### Remaining checks and next slice

Five broad checkpoint-8 groups remain open. Next, inspect and test the image
pipeline's encoder/FileReader callback ownership, rejection, timeout and
cancellation paths. Current `blob_from_canvas` and `blob_to_bytes` explicitly
forget their closures and poll without a bound; the decoder revokes after its
await rather than using cancellation-safe ownership. This slice does not accept
those lifetimes, EXIF/wide-gamut/animation behavior, all cross-step interactions,
physical-device gestures or screen-reader/whole-app sign-off.

## Image processing callback and cancellation acceptance 2026-10-02

The shared image pipeline now has bounded decoder, encoder and blob-read waits,
and decoder URLs are released when their future is cancelled. This closes the
specific lifetime gap recorded above, not all editor or checkpoint-8 acceptance.
Implementation remains on `codex/NOMS-020-checkpoint-8` in its existing isolated
worktree, fast-forwarded into `codex/NOMS-020-visual-design-refinement` for testing.

### Implementation and automated evidence

`0d1ddba` replaces forgotten encoder callbacks and unbounded FileReader polling.
Each asynchronous stage races a 30-second browser timer. The native Blob promise
has explicit success/rejection handlers; both hold weak completion references.
Canvas encoding uses a one-shot callback with the same weak ownership. Dropping
the waiting future drops its channel owner, so late callbacks discard their
payload instead of retaining or resuming a cancelled form. A decoder URL lease
revokes on success, failure or future cancellation. Dimensions, WebP quality,
save/upload behavior and visual styling are unchanged.

Native encoding/Blob work itself cannot be aborted through these APIs. Callback
captures are released on invocation or browser garbage collection; immediate
GC and compatibility with browsers lacking the wasm-bindgen weak-reference GC
mechanism are not claimed. The tested browser exposes WeakRef and
FinalizationRegistry. Timers can be delayed in suspended/background pages.

Tests-first logs show the missing lifecycle helper and forgotten callback wiring
failing before implementation. Seven native lifecycle tests cover completed and
timed-out waits, cancellation of both operation and timer, decoder URL release,
late payload disposal and first-result-only completion. A source-wiring regression
checks the actual WASM pipeline uses the bounded operations rather than forgotten
callbacks/FileReader. The existing broken-image decoder contract remains tested.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server utils::image_processing_lifecycle
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server image_encoder_and_blob_reader_use_bounded_owned_operations
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
```

Focused results: **7 and 1 passed**. Final full CI: **972 passed, 0 failed,
12 ignored**, including native/WASM checks, Clippy, formatting, schema, secret,
deployment and dependency-audit gates. Eight existing allowed audit warnings
remain. The initial CI run found a Rustdoc list-spacing lint; adding the missing
paragraph separator resolved it. No new ignored real-storage run is claimed.
Logs are `/tmp/noms-020-image-callback-red.log`,
`/tmp/noms-020-image-pipeline-wiring-red.log`,
`/tmp/noms-020-image-completion-green.log`,
`/tmp/noms-020-image-pipeline-wiring-green.log` and
`/tmp/noms-020-image-pipeline-ci-final.log`.

### Browser acceptance on the existing deployment

Rebuilt and restarted only the existing app container on
`http://127.0.0.1:32768`. Forced an uncached client reload and verified the
executed WASM resource length was **14,403,796 bytes**, matching the rebuilt
asset; SHA-256:
`b606f64418f7f1efff3170003d7cced290f708050614320bbd28a3bafa4e8a21`.
The subsequent documentation-only change does not alter pipeline behavior.
Used blank forms, the existing mock `testuser` identity and synthetic local PNGs.
No recipe title/content edit, server upload, draft save or publication was sent.

- Normal landscape selection opened the editor and reached Ready on closing it:
  43 KB to 5 KB, 86% smaller. One gallery URL remained while mounted; the decoder
  URL had already been revoked.
- A null encoder result showed the WebP error. A rejected native Blob promise
  showed the read error, with no observed unhandled rejection.
- Holding a real encoder callback past its deadline showed the encode timeout
  and re-enabled Publish, Cancel and Add Step. Delivering that result afterward
  threw no error. All eight URLs created up to that point had been revoked.
- Holding a native Blob resolution past its deadline showed the read timeout.
  Late resolution produced no error or Ready row. Ten URLs had been created and
  revoked; none remained outstanding.
- Forcing Blob-backed image `complete` to remain false, then navigating to
  Explore while decoding was pending, revoked all twelve URLs created so far,
  with no duplicate revocation.
- Navigating to Explore during a held encode, then delivering its callback,
  caused no error or revived editor. No URLs remained outstanding.
- A separate forced-pending decoder reached its decode timeout and restored the
  disabled controls. All sixteen URLs created so far had been revoked.
- After restoring all native APIs, a valid landscape image again reached Ready.
  A subsequent section-link probe cleared the form, described below. Final
  counts were **18 created, 18 revoked, 0 outstanding, 0 duplicate revocations**;
  the tab had no captured client error logs.

Inspected screenshots: `/tmp/noms-020-image-reader-timeout.png` and
`/tmp/noms-020-image-pipeline-recovered.png`. The former shows the recoverable
timeout; the latter shows Ready after restoring native processing. Temporary
diagnostic prototype wrappers and cache overrides were removed and the clean
verification tab closed. The six earlier unsaved tabs were marked for retention
and were not intentionally reloaded or closed.

### Integrity review and next fix

Before/after ordered full-row JSONB hashes match for 36 recipes, 71 versions,
10 drafts, 31 collections, 3 users and 10 media assets. Raw snapshots are
`/tmp/noms-020-image-pipeline-db-before.txt` and
`/tmp/noms-020-image-pipeline-db-after.txt`. The integration stylesheet is
byte-identical to its pre-turn checksum after removing only the generated
grayscale utility. Root main, unrelated edits and other deployments are untouched.

Second-pass self-review: **PASS for this scoped pipeline change**, with no
independent agent review. No schema migration is involved; the pre-change
pipeline is available at `0b284aa` if rollback is needed. No exact browser-GC
timing, EXIF/wide-gamut/animation, native gesture or whole-app sign-off is claimed.

A separate existing defect was reproduced twice: clicking the Photos section
link from `/recipes/new` changes the fragment and remounts the retained form,
clearing its local photo row. The second reproduction used a successfully
processed Ready photo, not a failing operation. Source inspection shows
`retained_form_visit_key` includes nonempty fragments in the component key.
Screenshot: `/tmp/noms-020-editor-section-link-clears-photo.png`. This is the
next fix: preserve same-page fragment navigation without losing local work,
while retaining reset behavior for genuine route/query changes. The five broad
checkpoint-8 acceptance groups remain open.

## Editor section navigation acceptance 2026-10-02

`a7bbcdf` fixes the fragment-navigation defect recorded above. The retained-form
key now ignores URL fragments and normalizes an empty query delimiter, while
preserving nonempty queries exactly. Section jumps no longer remount local form
state; a different route or parent query remains a new form. Authentication,
upload/save behavior and styling are unchanged.

The existing isolated worktree is
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-checkpoint-8`, branch
`codex/NOMS-020-checkpoint-8`. The code was fast-forwarded into
`codex/NOMS-020-visual-design-refinement` to test the existing deployment.

### Tests and browser evidence

Tests first reproduced two failures: a section fragment changed the key, and an
actual routed collection form replaced its input node. The final regression
dispatches a real serialized input event, verifies the edited text and original
node survive section changes and Back/Forward, then checks that a changed parent
query resets the form. Key cases retain encoded query characters and Unicode;
the existing startup, account-retention and read-scope tests still pass.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server auth::session_page::tests
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
```

**8 focused tests passed; full CI: 973 passed, 0 failed, 12 ignored.** Native/WASM
checks, Clippy, formatting, schema, secret, deployment and audit gates passed;
eight existing allowed audit warnings remain. Logs:
`/tmp/noms-020-section-navigation-red.log`,
`/tmp/noms-020-section-navigation-green-final.log`,
`/tmp/noms-020-section-navigation-ci.log` and
`/tmp/noms-020-section-navigation-build.log`.

Rebuilt and restarted only the existing app on `http://127.0.0.1:32768`.
A forced uncached reload executed the rebuilt **14,403,161-byte WASM** resource,
SHA-256 `cbd4206fed9e244dd1f6be71ae9f877080f9095ee8986bfc5a38a04e2137a4e2`.
Normal mock sign-in used the existing `testuser` identity. A blank recipe form
and synthetic PNGs supplied these browser checks without saving or uploading:

- A Ready landscape photo survived Photos/Notes jumps and browser Back/Forward.
  The retained-form DOM node was unchanged and its preview URL was not revoked.
- A locally confirmed 10° rotation was still shown when the image editor reopened
  after those jumps, and later after the gallery order changed.
- A held native encoder callback left a second photo Processing across Notes and
  Basics jumps. Publish, Cancel and Add Step stayed disabled. Restoring the
  encoder and delivering the callback completed processing; both photos were
  Ready and the controls re-enabled. The callback threw no error.
- Enter on the second photo's Move image up control placed it first. Its order,
  both Ready states and the landscape photo's rotation remained associated with
  the right images after another section/history round trip.
- Removing the portrait locally and jumping Notes to Photos retained the remaining
  Ready landscape. Leaving for Explore revoked the remaining preview: **4 URLs
  created, 4 revoked, 0 outstanding, 0 duplicate revocations**. No client error
  logs were captured.

Inspected screenshots: `/tmp/noms-020-section-retained-rotation.png`,
`/tmp/noms-020-section-retained-gallery.png` and
`/tmp/noms-020-section-navigation-fixed.png`. The rotation image shows the restored
editor value after navigation. The last page capture includes the retained Ready
row but is scrolled below the Photos heading; it is not a full-page visual audit.

### Integrity review and remaining acceptance

No recipe title/content edit, draft save, publication or server upload was sent.
Before/after ordered full-row hashes match for 36 recipes, 71 versions, 10 drafts,
31 collections and 3 users. Media count stays 10, but its full-row hash changed;
unchanged media metadata is not claimed. Snapshots:
`/tmp/noms-020-section-navigation-db-before.txt` and
`/tmp/noms-020-section-navigation-db-after.txt`. Diagnostic wrappers and the cache
override were removed, and the clean test tab was closed. Six earlier unsaved
tabs were marked for retention without intentional reload or closure. Root main,
unrelated edits and other deployments remain untouched; the integration stylesheet
is restored to its pre-turn checksum after removing the generated grayscale utility.

Second-pass self-review: **PASS for the scoped form-retention fix**; no independent
agent review was used. The preceding key implementation at `34addeb` is the
rollback baseline; no schema/data migration is involved.

The follow-up keyboard reorder test exposed a separate existing gap: the order
changes correctly, but the focused control is lost and `document.activeElement`
becomes the body. The moved first row's upward control is disabled. The current
gallery reorder handlers do not restore focus; no specific renderer/keying cause
is claimed. Restore a useful focus target and check boundary/non-boundary moves
next. Full keyboard, screen-reader, real-device and whole-app acceptance remain
open; this fix does not close any of the five broad checkpoint-8 groups.

## Gallery reorder focus acceptance 2026-10-02

The gallery focus gap recorded above is fixed by `e21c559`. Keyboard reordering
now keeps focus on the moved image's enabled reorder control. Middle moves retain
the requested direction; reaching the first or last row switches focus to the
opposite action on the same image. The effect runs after the DOM commit, scopes
button IDs to each uploader, avoids disabled/inert/hidden targets and does not
take focus from a different active control. Drag-and-drop does not request this
focus restoration. Upload, save, image-processing and visual contracts are unchanged.

Work used the clean existing isolated acceptance worktree
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-checkpoint-8`, branch
`codex/NOMS-020-checkpoint-8`, based on `715f6a4`. The runtime commit was
fast-forwarded into `codex/NOMS-020-visual-design-refinement` to rebuild the
existing deployment at `http://127.0.0.1:32768`; other deployments were untouched.

### Automated validation

Tests-first checks reproduced the missing boundary fallback: one passed and two
failed. Four final regressions cover both boundaries, middle moves, missing and
single-item galleries, invalid directions, twelve unique controls across two
uploaders, and actual rendered button-handler reordering. Native tests verify
the focus decision and wiring; browser checks below prove actual DOM focus.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server components::base::image_upload::tests
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
docker exec noms-c799915fcf93e034-app-1 dx build --platform web --force-sequential true --debug-symbols false
docker restart noms-c799915fcf93e034-app-1
```

Focused gallery tests: **46 passed**. Final full CI: **977 passed, 0 failed,
12 ignored**, including native/WASM checks, formatting, Clippy, schema, security,
secret and deployment gates. Eight existing allowed audit warnings remain.
The first run failed while an upload test created its temporary user because
PostgreSQL terminated the connection; the unchanged rerun passed all tests but
could not update the read-only dependency-advisory cache. A final reviewed
permission rerun passed the entire gate. Logs:
`/tmp/noms-020-gallery-focus-red.log`,
`/tmp/noms-020-gallery-focus-green-final.log`,
`/tmp/noms-020-gallery-focus-ci.log`,
`/tmp/noms-020-gallery-focus-ci-rerun.log`,
`/tmp/noms-020-gallery-focus-ci-final.log` and
`/tmp/noms-020-gallery-focus-build.log`.

### Browser acceptance

A fresh temporary tab used the existing mock `testuser` login and synthetic
landscape/portrait images; no recipe title/content or Save/Publish action was
submitted. An uncached reload executed the rebuilt **14,431,506-byte** WASM,
SHA256 `7d0d65701c958b6bbf6c4c83b4ed93a2047f2ee1ef1331eca1a7ce453b9f20e1`.

- Enter moved the second image to first, then focused its enabled Down action.
  Space on that restored control moved it back and focused its enabled Up action.
- With four Ready rows, Up and Down middle moves retained the same direction and
  stable image ID. Tab from Up reached Down; Space then reordered that same image.
- Notes navigation and browser Back retained all rows. Reopening the moved image
  still showed its 10° rotation; another image's adjustment was not substituted.
- At 320px, repeated Enter moves reached the first row and switched to its enabled
  Down action. Document scroll width was 305px within the 320px viewport.
- No captured browser warnings/errors appeared. Returning to Explore succeeded.

Inspected screenshots: `/tmp/noms-020-gallery-focus-middle.png` shows the focused
Down action on a rotated Ready image; `/tmp/noms-020-gallery-focus-mobile-full.png`
shows the same image and enabled focused boundary action at 320px. These are
interaction-state captures, not complete page audits. The browser's first narrow
screenshot was scaled incorrectly; the full-height CDP capture replaced it for
evidence. Temporary emulation/cache overrides were cleared and the new tab closed;
existing tabs were not reloaded or closed.

Before/after full-row counts and hashes for recipes, versions, drafts,
collections and users match exactly in
`/tmp/noms-020-gallery-focus-db-before.txt` and
`/tmp/noms-020-gallery-focus-db-after.txt`. No media-store integrity comparison is
claimed. The integration worktree's pre-existing Tailwind edits remain byte-for-byte
unchanged at SHA256 `74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`;
only the build-generated grayscale rule was removed. Unrelated root work was preserved.

Second-pass self-review: **PASS for scoped gallery-button focus**, with no remaining
blocking or should-fix findings. No independent-agent review was used. Rollback
baseline is `715f6a4`; no schema/data migration is involved. Continue multi-image
and cross-step drag interaction checks, then remaining mutation/identity gates.
Native drag gestures, real touch, screen-reader announcements and full accessibility
are not accepted by these keyboard checks; all five broad checkpoint-8 groups
remain open.

## Gallery and method photo drag acceptance 2026-10-02

The desktop drag slice passed. Gallery reorder keeps the complete image record
together while encoding finishes. Method photos reorder only inside their own
step; dropping onto a different step intentionally leaves both steps unchanged.
This records the existing no-transfer contract, not a new cross-step feature or
a previously reproduced data-loss defect.

Runtime commit `53a34e4` adds a shared tested method-photo reorder helper to both
recipe editor routes, replacing their duplicate drop logic. It rejects missing
steps and out-of-range photo positions without indexing panics. Gallery runtime
behavior is unchanged; its additions are regression tests. Work used the clean
isolated `codex/NOMS-020-checkpoint-8` worktree at
`/home/samko/GitRepos/Noms/.worktrees/NOMS-020-checkpoint-8`, based on `1c955c1`.
The runtime commit was fast-forwarded into the design integration branch and
rebuilt in the existing port-32768 deployment. Other deployments were untouched.

### Automated validation

Five new regressions cover complete gallery records during Processing/Failed
states, self/missing gallery targets, step-photo metadata and nested step-tree
preservation, cross-step/out-of-range/self drops, and both routes' source wiring
to the shared helper with live upload locks and drag cleanup. The wiring check
inspects actual handler source; it is not a mounted-browser test. The first
tests-first run failed to compile because the new shared helper did not yet exist;
this is not evidence of a behavioral failure in the original implementation.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server drag
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server step_photo
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
docker exec noms-c799915fcf93e034-app-1 dx build --platform web --force-sequential true --debug-symbols false
docker restart noms-c799915fcf93e034-app-1
```

The drag filter passed **4 tests**, including one existing image-editor test.
The step-photo filter passed **10 tests**. Full CI passed **982 tests, 0 failed,
12 ignored**, with formatting, native/WASM, Clippy, schema, security, secret and
deployment checks successful. Eight existing allowed dependency-audit warnings
remain. Logs are `/tmp/noms-020-step-drag-contract-red.log`,
`/tmp/noms-020-drag-green-final.log`,
`/tmp/noms-020-step-photo-regressions.log`, `/tmp/noms-020-drag-ci.log` and
`/tmp/noms-020-drag-build.log`.

### Browser acceptance

Two fresh temporary tabs used the existing mock `testuser` session and synthetic
landscape/portrait files. No Save or Publish action was submitted. The gallery
checks executed the previous unchanged gallery runtime at `1c955c1`, with uncached
WASM size **14,431,506 bytes**. The method-photo checks executed the rebuilt
**14,431,740-byte** WASM, SHA256
`204e7e50d76c276cb1c144ae06c00daa3cd571cd5ef6fd500a631e9be1dddd10`.

- A trusted native gallery drag moved the second Ready image first. Its 12°
  rotation remained attached to that image when reopened.
- A temporary diagnostic held one real canvas encoder callback while a third
  image remained Processing. Dragging that row changed order from `1,0,2` to
  `1,2,0`. Releasing the original callback made the moved image Ready at its new
  position, with all three images present and disabled processing controls
  reenabled. No fabricated encoding result was injected.
- Gallery self-drop left order unchanged. An outside drop produced dragstart/end
  without a drop event. Both cleared the drag indicator.
- Two method steps held two photos and one photo respectively. A trusted native
  same-step drag moved the portrait photo first; reopening it still showed 12°.
- Cross-step drops in both directions produced trusted dragstart/drop/dragend
  events, left the two photo lists unchanged and cleared the dragging class.
  The high-level fast vertical drag initially released before a final target
  dragover, producing only dragstart/end. Stepped native CDP mouse input then
  held the pointer over the observed target before release and verified the
  actual drop handler. No JavaScript-dispatched DragEvent was used.
- Method self-drop and outside cancellation retained all three photos and
  cleared the dragging class. Controls remained usable afterward.

Adding method steps triggers autosave even with an empty title. In the verification
tab only, 2-second autosave timers were held as 10-minute timers before adding
the fixture. This isolates drag state and does **not** accept normal autosave
or drag-plus-persistence behavior. The captured request interval was complete
and contained exactly three POSTs, all to the real step-photo upload endpoint;
no draft save or publication request occurred. Media asset count increased from
10 to 13; these unreferenced synthetic uploads were retained, not deleted. No
media-store integrity or worker-cleanup claim is made.

Before/after counts and full-row hashes match for recipes (36), versions (71),
drafts (10), collections (31) and users (3), recorded in
`/tmp/noms-020-drag-db-before.txt` and `/tmp/noms-020-drag-db-after.txt`.
Sanitized trusted-event evidence is in `/tmp/noms-020-gallery-drag-events.json`
and `/tmp/noms-020-step-drag-events.json`; method POST paths are in
`/tmp/noms-020-step-drag-network.json`. Inspected screenshots
`/tmp/noms-020-gallery-drag-ready.png` and `/tmp/noms-020-step-drag-ready.png`
show Ready gallery rows and the reordered rotated method photo alongside the
unchanged second step. These are desktop interaction captures, not full-page
or device audits.

The encoder prototype, autosave timer override and diagnostic listeners were
restored/removed; pending held timers were cleared after navigating away. Cache
overrides were cleared and both temporary tabs closed. Existing browser tabs
were not reloaded or closed. The integration worktree's user-owned Tailwind
edits retain exact SHA256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`;
only the generated grayscale rule was removed after the build. Root work remains
untouched.

Second-pass self-review: **PASS for the scoped desktop drag contract**, with no
remaining blocking or should-fix findings. The helper comment was narrowed to
missing/out-of-range positions: index-based method dragging does not promise
stable identity during simultaneous structural edits. No independent-agent
review was used. Rollback baseline is `1c955c1`; no schema migration is involved.
Native edit-route gestures are not newly accepted, though shared behavior and
both handlers' wiring are tested. Real touch, screen readers, general accessibility,
normal autosave, EXIF orientation, wide-gamut/animated files and remaining mutation
or identity gates stay open. Next: image orientation and export edge cases. All
five broad checkpoint-8 groups remain open.

## Image orientation and export acceptance 2026-10-02

Sampled orientation, resize and transparency checks passed after fixing a real
thin-image defect in `03095e9`. Before the fix, valid 4096×1 and 1×4096 PNGs
produced 1920×0 and 0×1920 canvases. Native canvas encoding returned null and
both gallery rows showed the safe WebP error. The resized short axis now keeps
at least one pixel. Ordinary sizes, no-upscale behavior and the 1920/800 bounds
remain unchanged; aspect ratio necessarily quantizes at the one-pixel boundary.

The sizing helper moved from the WASM-only processing module into
`src/utils/image_dimensions.rs` so native CI tests the production calculation.
The original eight WASM-module sizing tests remain intact. Seven new native
regressions cover thin axes, common landscape/portrait/square output, small and
exact-boundary sources, maximum u32 inputs, 147 positive size/bound combinations,
empty inputs and actual full/thumbnail pipeline source wiring. Maximum u32 tests
are arithmetic checks, not claims that the browser decodes such large images.

### Validation and deployment

The tests-first run reported **2 passed and 4 failed**: three failures reproduced
zero-pixel output and one identified the not-yet-connected shared helper. After
the fix, **7 focused tests passed**. Full CI passed **989 tests, 0 failed,
12 ignored**, including formatting, Clippy, native/WASM, schema, security, secret
and deployment gates; eight existing allowed audit warnings remain.

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server image_dimensions
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
docker exec noms-c799915fcf93e034-app-1 dx build --platform web --force-sequential true --debug-symbols false
docker restart noms-c799915fcf93e034-app-1
/home/samko/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 /tmp/noms-020-check-image-exports.py /tmp/noms-020-image-format-green.json
```

Logs: `/tmp/noms-020-image-dimensions-red.log`,
`/tmp/noms-020-image-dimensions-green.log`, `/tmp/noms-020-image-format-ci.log`
and `/tmp/noms-020-image-format-build.log`. The Python pixel/dimension assertion
script passed. Temporary fixture generation uses bundled Pillow, not a new
dependency. Work used the clean existing isolated `codex/NOMS-020-checkpoint-8`
worktree, based on `02ed58e`, and fast-forwarded the runtime fix into the design
integration branch. Only the existing port-32768 deployment was rebuilt/restarted.

### Browser evidence and limits

A fresh temporary browser tab used the normal mock `testuser` login. Initial
thin-image failures were captured on the previous runtime. After rebuilding,
an uncached reload executed **14,431,923-byte** WASM, SHA256
`f8085fb72c7c13ad8aa739a58566a99aeb23e213cfc334f39d8833eb51096ef3`.
Twelve synthetic images completed as Ready through the real gallery pipeline:

- Eight JPEG files shared asymmetric coloured corners with EXIF orientation
  values 1–8. Both full and thumbnail WebP exports matched independently generated
  Pillow orientation results: 480×320 for 1–4 and 320×480 for 5–8. All 64 sampled
  opaque corner pixels matched their expected RGB values within 8 channel levels,
  allowing lossy WebP compression. Rotation and mirroring were therefore checked
  by pixels, not dimensions alone. The oriented crop preview was also inspected.
- The repaired thin sources exported 1920×1 / 800×1 and 1×1920 / 1×800. All four
  blobs were nonempty `image/webp` and decoded successfully.
- A 3072×2048 PNG exported 1920×1280 and 800×533. Small EXIF and alpha fixtures
  were not enlarged.
- A transparent PNG retained alpha 0 at four corners and alpha 128 at its centre
  in both exports; sampled centre RGB remained within the same tolerance.

A temporary canvas diagnostic recorded native encoder dimensions, MIME type and
blob size, then independently decoded the returned blobs to sample pixels. It
passed the original blobs straight through to the application; no replacement
result was injected. Raw red/green observations are
`/tmp/noms-020-image-formats-red.json`,
`/tmp/noms-020-exif-export-observed.json` and
`/tmp/noms-020-image-format-green.json`. The fixture manifest and generation script
are `/tmp/noms-020-image-formats/expected.json` and
`/tmp/noms-020-image-format-fixtures.py`.

Inspected screenshot `/tmp/noms-020-image-format-ready.png` shows the repaired
tall-strip Ready row and correctly oriented large-image Ready row in the app;
`/tmp/noms-020-exif-6-preview.png` shows the oriented crop preview. They are sampled
interaction captures, not complete editor/layout acceptance. The post-fix request
interval was complete and contained **zero POSTs**, recorded in
`/tmp/noms-020-image-format-network.json`. No title/step/content edits, Save or
Publish were submitted, and no autosave timer override was needed.

Recipes (36), versions (71), drafts (10), collections (31) and users (3) retain
the exact full-row hashes from the previous drag baseline, checked before and
after the post-fix browser pass. Media asset count remains 13; no media-row hash
or object-store integrity claim is made. The canvas diagnostic and cache override
were removed, the test navigated to Explore and the temporary tab closed. Existing
tabs and other deployments were untouched. User-owned integration Tailwind edits
remain byte-identical at SHA256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`;
only the generated grayscale rule was removed. Unrelated root work was preserved.

Second-pass self-review: **PASS for sampled image orientation and resize/export**;
no remaining blocking or should-fix findings in this contract. No independent
agent review was used. Rollback baseline is `02ed58e`; no schema/data migration is
involved. This accepts the tested browser's decoder/encoder samples, not all
browser engines, all EXIF containers, ICC/wide-gamut conversion, animation/frame
policy, persisted upload delivery or real-device behavior. Next: wide-gamut and
animated-image handling, then remaining mutation/identity gates. All five broad
checkpoint-8 groups remain open.

## Image colour and animation audit 2026-10-02

Sampled ICC colour conversion passed without a code change. The animation audit
found a visible mismatch: the editor displays the original animated source while
the full-size and thumbnail exports are first-frame stills. The subsequent user
scope decision excludes animation handling from this editor overhaul. The mismatch
remains a documented limitation; no processing or input-rejection change is required.

### Colour conversion evidence

Six 480×320 fixtures embed the installed Artifex sRGB, A98 and ROMMRGB ICC
profiles, each in PNG and JPEG. The independent reference uses Pillow ImageCms
to convert each decoded source to the installed sRGB profile. JPEG expectations
use the decoded JPEG pixels rather than the pre-compression source. Four solid
quadrants distinguish profile conversion from simply ignoring the profile.

The real recipe-gallery pipeline produced 12 nonempty `image/webp` exports.
All stayed 480×320, preserving no-upscale behavior. All 48 sampled corner pixels
matched the independent sRGB reference within **2 channel levels**, below the
predeclared tolerance of 8 for browser conversion and lossy WebP encoding. Full
and thumbnail results matched each other at every sampled corner. This accepts
these embedded profiles' conversion to sRGB in the tested browser, not retention
of out-of-sRGB colours, HDR, all profiles or every browser engine. Editor colour
adjustment fidelity was not newly tested by these export samples.

### Animation finding

Two-frame PNG and lossless WebP fixtures contain a red first frame for 200 ms
and a blue second frame for 2000 ms. Both were admitted normally and became Ready.
Both full and thumbnail exports sampled red `[220, 41, 40, 255]`, within one
channel level of the source first frame. Reopening the WebP editor showed the
blue later frame; inspected screenshot `/tmp/noms-020-animation-editor.png`
records the mismatch. The original source preview remains visible after
processing, explaining why Ready does not imply preview/export agreement.

A separate controlled diagnostic delayed delivery of each native full-size
encoder callback by 700 ms before thumbnail generation. The original blobs were
passed unchanged to the application; no success or error result was fabricated.
Both formats still exported the red first frame in both variants. The four
captured delayed-run blobs were also independently decoded by Pillow as
single-frame, non-animated images. Thus this browser did **not** exhibit a
full/thumbnail frame race in the tested cases. The delay was a tab-local diagnostic,
not natural encoder latency. Preview agreement for animated sources is not accepted
or required under the subsequent still-image scope decision. Retaining animation
would require a separately scoped feature.

### Reproduction and preservation

Fixture generator: `/tmp/noms-020-colour-animation-fixtures.py`; manifest:
`/tmp/noms-020-colour-animation/expected.json`. Reference profiles come from
`/usr/share/ghostscript/iccprofiles/`, with names read directly from the profiles.
The existing bundled Python/Pillow runtime was used; no dependency was installed.
Raw observations: `/tmp/noms-020-colour-observed.json` and
`/tmp/noms-020-colour-animation-observed.json`. Reproduction of the assertions:

```sh
/home/samko/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 /tmp/noms-020-colour-animation-fixtures.py
/home/samko/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 /tmp/noms-020-check-colour-animation.py
```

Assertions passed for 12 ICC exports, 48 corner samples, eight animated-input
exports and four independently decoded delayed blobs. A fresh temporary tab
used the existing test session and an uncached reload on port 32768. The existing
application was not rebuilt or restarted. No source code changed and full CI was
not rerun; the latest code baseline remains `03095e9` with 989 passed, 0 failed,
12 ignored. The final page showed ten Ready images: six ICC, two animation and
two renamed delayed-run fixtures. Reselecting the identical animation files
produced no new rows or encoder calls, so renamed files were used for the delayed
pass; that picker behavior is not a new acceptance claim.

The complete captured interval had 24 requests, **zero POSTs**, no truncation or
remaining pages, recorded in `/tmp/noms-020-colour-animation-network.json`.
No title/content edits, Save or Publish were submitted. Recipe, version, draft,
collection and user counts/full-row hashes match the preceding recorded baseline;
media count remains 13. No object-store integrity claim is made from that count.
Canvas instrumentation and cache override were restored, the test navigated away
and its temporary tab closed. Existing user tabs and other deployments were not
changed. User-owned integration CSS retains SHA256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.

Second-pass self-review: **PASS for sampled ICC export conversion**. Animated-source
preview agreement was not established and is now out of scope. No independent-agent
review was used. This documentation-only slice uses one coherent evidence commit in the existing
isolated `codex/NOMS-020-checkpoint-8` worktree, based on `4d3a385`; no runtime,
schema or saved-content change requires deployment. All five broad checkpoint-8
groups remain open. Next: remaining mutation and still-image acceptance gates.

## Still image editor scope decision 2026-10-02

The user confirmed that the editor does not need to handle animations. The
acceptance contract covers still images; it does not require animated playback,
frame selection/editing, preservation or matching animated-source previews to
exports. Existing first-frame conversion is retained. The observed later-frame
preview is recorded above as a limitation, not a blocker or evidence of supported
animation editing. No animation detection, rejection, preview flattening or
processing change was implemented.

This documentation-only clarification uses the existing isolated checkpoint-8
worktree and one coherent scope commit. Diff whitespace and a second-pass
self-review passed; runtime tests were not rerun because application code did
not change. All five broad acceptance groups remain open. The next work returns
to remaining mutation outcomes and still-image checks, not animation support.

## Collection creation outcome recovery 2026-10-02

Collection creation now preserves local entries and prevents a blind second
creation after an uncertain response. The real pre-fix insert committed before
its HTTP 200 response was dropped. The old form exposed a raw transport error and
allowed another Create click. That second request was blocked at request stage
before execution, so no duplicate collection was actually inserted.

`e8a7382` adds an unconfirmed-creation state, safe feedback and a Check Collections
link that opens a separate tab. Create and implicit form submission stay disabled
in this state, while local fields and Cancel remain usable. The parent handler
also checks live state before beginning a deferred upload. The shared form's new
submission-only flag defaults to false, leaving collection editing unchanged.
Known account-change rejection preserves its existing safe retry feedback because
it occurs before writes. Other create failures conservatively require checking
Collections; the client does not infer rollback from a transport/server failure.
This is a visit-local guard, not server-side idempotency or cross-tab protection.

### Tests and deployment

Two tests-first source-contract regressions failed for the missing state and
submission-only guard. After implementation, five focused collection-creation
tests passed, including the existing API/route tests and a new real serialized
form-submit regression. That regression confirms no parent submission is
dispatched while unconfirmed, the saved field value is retained, input and Cancel
are enabled, Create is disabled and the form does not pretend it is still saving.
Three new regressions were added in total.

Full CI passed **992 tests, 0 failed, 12 ignored**, including formatting, Clippy,
native/WASM compilation, schema, security policy, secret and deployment gates.
Eight existing allowed dependency warnings remain. Commands:

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server collection_creation_recovery
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server collection_creation
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
docker exec noms-c799915fcf93e034-app-1 dx build --platform web --force-sequential true --debug-symbols false
docker restart noms-c799915fcf93e034-app-1
```

Logs: `/tmp/noms-020-collection-create-red.log`,
`/tmp/noms-020-collection-create-green.log`,
`/tmp/noms-020-collection-create-ci.log` and
`/tmp/noms-020-collection-create-build.log`. Only the existing port-32768 app
was rebuilt/restarted. The rebuilt WASM is 14,439,034 bytes, SHA256
`e80e5aab5f218437c8c288de43bc4396bf903bd7f99fa9474e743477d4522066`.
An uncached reload and the new runtime feedback/disabled behavior verified that
the browser exercised the fix. Normal mock Google sign-in restored the existing
`testuser` account; no user or provider link was created.

### Browser and database observations

The local fixture owner is `6b1c56b4-945d-44d5-9f1b-eeb9525b8519` / `@user-e13c`.
All three collections are private, top-level, empty and have no cover:

| Fixture | ID | Purpose |
| --- | --- | --- |
| CP8 collection response-loss red | `f67d7f0e-0cfd-4767-9a2a-373aa5d8c627` | Pre-fix committed response loss; duplicate attempt failed before execution. |
| CP8 collection response-loss green | `9d6595fe-7e22-42ca-ad59-e170716a1fe1` | Post-fix committed response loss and separate-tab recovery. |
| CP8 collection confirmed destination | `5cadc14b-c8cc-4c96-a0ad-a2644c2fa6d0` | Confirmed success followed by offline destination reads. |

Response-stage Fetch/XHR interception held each real HTTP 200 creation response.
Read-only SQL independently proved the named row, private visibility and correct
owner existed before the response was failed or continued. No replacement success
response was fabricated. Browser-managed Document interception was not used.

Post-fix lost-response observations:

- Safe uncertainty feedback appeared; Create stayed disabled. Name/description,
  sharing selection and Cancel remained available. Editing the name afterward
  and pressing Enter produced **zero POSTs** in the complete observed interval.
- Check Collections had `target="_blank"`; clicking opened a separate read tab.
  Filtering the library found exactly one matching collection, and opening it
  loaded the committed description. The original form retained its modified
  local name and warning. Cancel subsequently returned to the library normally.
- A temporary raw device-metrics probe measured 305px document width at 320px,
  but the screenshot API changed that override. The resulting narrow form capture
  is not claimed as full 320px recovery-layout acceptance. The destination test
  below used the supported viewport capability instead.

For the confirmed-success fixture, SQL again proved the commit before connectivity
was set offline and the held response continued. Typed navigation retained the
app shell at the new detail URL, with Collection unavailable, Retry collection,
Back to collections and sign-in-check recovery. An Enter retry at the actual
320×720 viewport remained usable; document width was 305px. Restoring connectivity
and retrying sign-in/collection reads loaded the exact created collection. The
complete interval contained **one create POST and zero Document requests**;
destination retries did not issue another creation.

Inspected captures: `/tmp/noms-020-collection-create-green.png` shows the safe
warning, disabled Create, enabled Cancel and Check Collections. The actual narrow
destination state is `/tmp/noms-020-collection-create-destination-narrow.png`;
desktop offline and recovered states are
`/tmp/noms-020-collection-create-destination-offline.png` and
`/tmp/noms-020-collection-create-destination-recovered.png`. Request metadata,
without credentials or request bodies, is saved in
`/tmp/noms-020-collection-create-no-retry.json` and
`/tmp/noms-020-collection-create-destination-network.json`.

A preliminary attempt clicked Create before the uncached client had hydrated,
causing native GET form navigation with named fields in the query, not an API
creation. After hydration those entries reset; no collection was inserted by that
attempt. The real post-fix acceptance occurred after hydration, and the final
confirmed-success run explicitly verified a reactive sharing-hint update before
submission. Pre-hydration submission is a separate remaining startup gate, not
accepted by these checks. Startup navigation briefly failed while the rebuilt
app was not ready; normal navigation succeeded after the endpoint was available.

### Preservation and review

Before/after SQL records are `/tmp/noms-020-collection-outcome-before.txt` and
`/tmp/noms-020-collection-outcome-after.txt`. Recipes (36), versions (71), drafts
(10), users (3) and the **31 original collection rows** retain their exact previous
full-row hashes. Excluding only the three explicit new fixture IDs reproduces the
original collection hash. Total collection count is now 34. Media count remains
13; no upload, deletion, publication, move or sharing change was submitted. No
object-store integrity claim is inferred from the media count.

Interception, offline emulation, cache and both temporary viewport overrides were
cleared; all four agent-created test/recovery tabs were closed. Existing user tabs
and other deployments were not reloaded or closed. The generated grayscale rule
was removed narrowly after build/restart, restoring the user's integration CSS
byte-for-byte at SHA256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
Unrelated root roadmap edits remain untouched.

Second-pass self-review: **PASS for hydrated collection creation response loss
and confirmed destination recovery**, with no blocking or should-fix findings in
that bounded contract. No independent-agent review was used. The production fix
and regressions form one coherent runtime commit; evidence is a separate commit,
both in the existing isolated `codex/NOMS-020-checkpoint-8` worktree. Rollback
baseline is `c36486b`; no schema migration is involved. Cover-upload response loss,
collection editing/moving/sharing, unhydrated submits, cross-tab duplicate
prevention, real-device/screen-reader checks and whole-app sign-off are not newly
accepted. All five broad checkpoint-8 groups remain open. Next: collection
edit/move/sharing response-loss and final-navigation recovery.

## Graph device and screen reader checklist 2026-10-03

**Supporting automated and browser checks PASS; physical touch and actual
screen-reader acceptance remain OPEN.** The user requested all checks in the
first remaining group. Its outstanding contract requires real gesture arbitration
and spoken assistive-technology behavior, neither of which the available browser
tools can certify. No production code was changed or application rebuild needed.

### Checks performed

On code baseline `73217d1` in the existing isolated checkpoint-8 worktree:

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server pages::recipe_version_graph -- --test-threads=1
```

**67 passed, 0 failed, 0 ignored**, including long-press candidate cancellation,
menu navigation, focus positioning, outline markup and request recovery. This is
a focused run, not a new whole-repository CI result.

The existing app at `http://127.0.0.1:32768` was tested in one clean in-app tab
using retained private recipe `1c6173d6-505c-92d7-7493-97e30be37b85`, titled
CP8 History root – Local test only. Normal mock Google sign-in restored the
existing `testuser` account. An initial mistaken `/graph` URL returned the normal
404; the actual `/version-graph` route was then reached through Recipe actions →
View version graph. The 404 attempt is not graph evidence.

- Shift+F10 on v30 opened its snapshot menu and focused View snapshot. End moved
  focus to Zoom to snapshot; Escape closed the menu and restored the v30 link.
- At 390×844, Enter opened the native Accessible graph outline disclosure. Its
  text identified current v30 and directional version relationships. Enter on
  Expand 5 older versions loaded 25 → 30 cards and 30 outline links; focus remained
  on the same control, now Collapse older versions, with `aria-expanded=true`.
- The expansion status exposed `role=status`, `aria-live=polite` and
  `aria-atomic=true`. Comparison text retained explicit Before → After direction.
  These observations verify markup, not spoken output or announcement timing.
- At 320×720, all eight pan/zoom/Fit/Current controls were 44px high and at least
  45px wide, entirely within the page. The document width was 305px, including
  the browser's scrollbar allowance. At 820×1180, those controls were at least
  44×44px and the document width was 805px. At 390×844 the document width was
  375px. These are viewport/keyboard observations, not physical-device acceptance.

Inspected screenshots: `/tmp/noms-020-graph-phone-controls-2026-10-03.png`
(comparison and opened outline at 390px) and
`/tmp/noms-020-graph-320-2026-10-03.png` (page heading and wrapped toolbar at 320px).
Temporary viewport overrides were reset and the test tab closed. Existing user
tabs and other deployments were untouched. No content mutation, upload, restore,
publication, sharing change or deletion was submitted. No storage-integrity hash
audit is newly claimed.

### Environment limits

A fresh `Input.dispatchTouchEvent` touch-cancel capability probe returned
unsupported in the in-app browser. Native computer APIs are disabled and the
enabled-surface inventory exposes no native app. Read-only environment discovery
found no `orca`, `speech-dispatcher` or `adb` executable and no `/dev/input`
directory. There is no connected physical test device or operable screen reader
available through these tools. No software was installed and no phone access,
network exposure or security setting was changed to work around those limits.

### Physical touch walkthrough still required

Use the private fixture in an authorized browser on a real phone and portrait
tablet. Record device, OS, browser/version and viewport alongside each result.
Do not publish, restore or delete a recipe as part of this walkthrough.

- [ ] A short snapshot tap opens the intended historical snapshot once; Back
  returns to a usable graph.
- [ ] Holding one finger beyond 550ms opens the correct snapshot menu. Releasing
  does not accidentally navigate. Dismissing restores the snapshot context.
- [ ] Releasing before 550ms does not leave a delayed menu. Movement beyond 12px
  while holding cancels the menu; ordinary scrolling continues.
- [ ] Starting with two fingers or adding a second finger cancels the candidate;
  native pinch/scroll behavior is not blocked and no stale menu appears.
- [ ] OS/browser gesture cancellation and leaving the page during a pending
  hold do not leave a delayed menu or navigate a replacement page.
- [ ] Page and outline scrolling work on phone and tablet; toolbar, expansion,
  comparison and menu controls remain reachable without accidental activation.

### Screen reader walkthrough still required

Use an actual screen reader with the fixture owner signed in, recording the
screen reader/version, OS and browser. Accessibility-tree snapshots do not count
as completing these items.

- [ ] Heading/landmark navigation reaches the graph and outline; snapshot links
  identify recipe and version, and current/main/ancestor/descendant states can
  be understood without connector colors or images.
- [ ] Snapshot actions and keyboard menu invocation are discoverable; menu items
  and unavailable states are spoken correctly. Arrow/Home/End navigation works,
  and Escape returns focus without accidental navigation.
- [ ] Outline disclosure announces expanded/collapsed state. Version and fork
  relationships are understandable in reading order.
- [ ] Expansion loading/busy/completion and a controlled read failure/retry are
  announced at useful times without unwanted repetition; focus remains usable.
- [ ] Before and After selections, swap direction and comparison results are
  spoken in the intended order, including when selection changes.
- [ ] Signed-out and non-owner private/redacted states do not expose protected
  snapshot content in spoken output or the accessible outline.

Second-pass self-review: PASS for the bounded test record and explicit limits;
no independent-agent review. This documentation update is one coherent concern.
The first remaining group is not complete, and all five broad checkpoint-8
groups remain open. Completing this group requires a device/screen-reader
walkthrough supplied by the user or an authorized environment exposing those
capabilities, not another synthetic-event pass.

## Collection edit outcome recovery 2026-10-03

**Scoped details, move, sharing and confirmed destination recovery PASS.** The
existing save sequence uses separate requests, not one atomic transaction. A lost
acknowledgement must not claim a definite failure, reset the intended location,
issue later writes or invite a blind retry. This slice reused empty synthetic
collections at `http://127.0.0.1:32768`, without changing existing user content.

### Reproduction and fix

On baseline `7692350`, the red fixture
`f67d7f0e-0cfd-4767-9a2a-373aa5d8c627` was moved inside fixture
`5cadc14b-c8cc-4c96-a0ad-a2644c2fa6d0`. The real HTTP200 move response was held;
SQL independently confirmed its description and parent had committed. Failing
that response made the old UI claim the move failed, reset location to top level
and enable Save changes. No second save was submitted in this red probe.

`62e7b20` adds stage-specific uncertainty feedback, a submission-only block and a
live submit guard. Details/location/sharing failures stop the sequence immediately
and preserve intended values. Only the explicit pre-write account-change
rejection stays retryable; later-stage account feedback acknowledges preceding
confirmed details. Other errors are conservative unconfirmed outcomes, without
raw backend details. A Check saved collection link opens the known collection ID
in a separate tab. Local edits and Cancel remain available. No automatic retry,
server idempotency, transaction, schema change or automatic reconciliation is
introduced. The old definite-failure test was updated to this explicit contract.

Two new wiring regressions failed before implementation. After implementation,
four focused recovery tests passed, including stage feedback and account-rejection
handling; broader collection coverage passed **107 tests, 0 failed, 2 ignored**.
The existing shared-form real-submit regression also passed within that run.
There are three net new tests; source-wiring checks alone are not runtime proof.

### Browser and database observations

The green fixture `9d6595fe-7e22-42ca-ad59-e170716a1fe1` was reused, with no cover,
recipes or subcollections. Each probe held a real response at the Fetch/XHR
response stage, confirmed database state before failing that response, and then
observed the actual Rust UI. No response body was fabricated. Interception was
restricted to mutation endpoints and never to Document requests.

- **Details:** one update committed the name CP8 collection edited recovery and
  description. Its acknowledgement was lost before any move/sharing request.
  The UI retained the desired parent and Unlisted selection while SQL still
  showed top-level/private. Save was disabled, local fields editable, and Enter
  issued no further POST. A further local name edit remained in the original
  tab while the Check saved collection link opened the committed name,
  description and Private badge in a second tab. Cancel was usable.
- **Location:** fresh editing loaded confirmed state. Details and a move into
  the destination fixture committed, then the move acknowledgement was lost.
  The intended parent remained selected, not reset; desired Unlisted remained
  local while stored visibility stayed Private. The request interval contained
  one details and one move mutation, no sharing write and no Enter retry.
  Cancel and fresh Edit loaded the committed parent/description.
- **Sharing:** a fresh save confirmed details and a move back to top level,
  then SQL proved Unlisted committed before the sharing acknowledgement was
  lost. The UI correctly reported unconfirmed sharing, kept its selection and
  enabled Cancel but not Save. The interval contained exactly one request per
  stage, without an Enter retry. Fresh Edit loaded stored Unlisted.
- **Final destination:** another save restored Private. After SQL confirmed it,
  the held successful sharing response was continued unchanged with the tab
  offline. Typed navigation reached the same collection URL and displayed
  Retry collection plus sign-in-check recovery, not a browser error page. Enter
  retry while offline remained usable. Restoring connectivity and using the
  two read-retry controls loaded the saved description/Private badge and focused
  collection-detail-content. The interval contained one details and one sharing
  mutation, three collection reads and **zero Document requests**. Read retry
  did not repeat either save.

The final recovery viewport measured **400×900**, document width **381px**. A
requested 320×720 override did not apply to this tab; these results and screenshot
are not claimed as 320px acceptance. The existing 320px collection-create check
is separate evidence. Full-page uncertainty screenshots were inspected at the
default desktop size. Temporary files:

- `/tmp/noms-020-collection-edit-uncertain-details.png`
- `/tmp/noms-020-collection-edit-uncertain-move.png`
- `/tmp/noms-020-collection-edit-destination-offline.png`
- `/tmp/noms-020-collection-edit-destination-recovered.png`
- `/tmp/noms-020-collection-edit-{details,move,sharing,destination}-network.json`

Network records contain only request sequence, URL, method and resource type;
no headers, credentials or request/response bodies were saved. All response-loss
and retry intervals were read without event truncation. The fixture ended
top-level/private; the red fixture remains private inside the destination fixture.
No publication, deletion, recipe restore or upload was submitted. The two
temporary tabs were closed; interception, offline mode, cache disabling and the
attempted viewport override were cleared.

### Validation and preservation

On the existing isolated `codex/NOMS-020-checkpoint-8` branch:

```sh
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server collection_edit_recovery -- --test-threads=1
SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target cargo test --features server collection_ -- --test-threads=1
PGOPTIONS='-c log_min_error_statement=panic' SQLX_OFFLINE=true CARGO_TARGET_DIR=/home/samko/GitRepos/Noms/.worktrees/NOMS-020-visual-design-refinement/target just ci
docker exec noms-c799915fcf93e034-app-1 dx build --platform web --force-sequential true --debug-symbols false
```

Full CI **995 passed, 0 failed, 12 ignored**; native/WASM compilation, Clippy,
formatting, schema, policy, secret and deployment gates passed with eight existing
allowed audit warnings. No fresh ignored storage/search integration run claimed.
CI log: `/tmp/noms-020-collection-edit-ci.log`. The first explicit build was
interrupted by a premature app restart after client compilation (exit137), so it
is not counted as successful. A second build completed both client/server with
exit0; log `/tmp/noms-020-collection-edit-build-complete.log`. The reloaded client
transferred 14,443,226 decoded WASM bytes (14,443,526 including transfer overhead),
matching the built file size. The built file's SHA256 is
`06a5b00b9fa86c477b28598b1a1842bbe8c0e36bf34193e2b912a89f2ee37c7d`.

Within this follow-up, ordered full-row hashes match for 36 recipes, 71 versions,
10 drafts, 3 users and the original 31 collections (excluding the three explicit
synthetic collection IDs). Version rows were ordered by recipe ID/version number:
`f5dcc0816a77295b76897942a28866fd`; other hashes match the preceding records.
Media count remains 13, which is not an object-store integrity audit. Only the
named synthetic collection edits/moves and temporary Unlisted visibility changed.
The user's integration CSS was restored byte-for-byte by removing only the
build-generated grayscale rule; SHA256
`74ca12aca7f2fc6409c26eda25a669a522dcb188915fafb111037529227e598e`.
Main's unrelated roadmap edits and other deployments were untouched.

Second-pass self-review: **PASS for the scoped outcome contract**, no blocking or
should-fix findings; no independent-agent review. The runtime fix/tests are one
coherent commit and this evidence is separate. The fix was fast-forwarded into
the existing NOMS-020 integration worktree. Rollback baseline `7692350`; no
migration. Cover upload response loss, concurrent external edits, in-flight route
or identity replacement, real-device/screen-reader acceptance and whole-app signoff
are not closed by this slice. All five broad groups remain open. Next: prevent
native collection form submission before hydration without losing early values.
