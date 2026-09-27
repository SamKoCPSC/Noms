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
