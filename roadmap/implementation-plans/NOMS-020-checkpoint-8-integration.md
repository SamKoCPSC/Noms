# NOMS-020 — Integrated browser review

**Date:** 2026-09-27

**Status:** Page implementations integrated; release acceptance OPEN.

**Tested source:** `74c420035ce29ccbb09817f4325252f56b4c5a80` on `codex/NOMS-020-visual-design-refinement`.

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

### Clarify: guest-search promise and destination

- [ ] Reconcile Home/sign-in copy and destination with existing access rules.
  Do not remove authentication from Search without explicit scope.

Sign-in says guests can browse and search public recipes; Home Search navigates
to authenticated Search and prompts sign-in. Explore has public search, so the
promise may be intentional but its entry path needs clarification. This is not
established as a new regression or authorization bug.

## Remaining gates and next order

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
