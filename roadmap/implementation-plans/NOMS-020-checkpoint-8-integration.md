# NOMS-020 — Integrated browser review

**Date:** 2026-09-27

**Status:** Page implementations integrated; release acceptance OPEN.

**Tested source:** `74c420035ce29ccbb09817f4325252f56b4c5a80` on `codex/NOMS-020-visual-design-refinement`.

This header and the original review below describe the 2026-09-27 baseline.
See **Checkpoint 8 close-out pass — 2026-09-29** at the end for current fixes,
CI results, newly observed issues and remaining acceptance gates.

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

Current queue:

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
