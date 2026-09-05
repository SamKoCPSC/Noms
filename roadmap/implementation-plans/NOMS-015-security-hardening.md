# NOMS-015: Security Hardening to a Good-Enough Baseline — Agentic Implementation Plan

**Status:** In progress — CP8D–CP8F are implemented; the CP9 final integrated adversarial re-review remains
**Created:** 2026-07-29
**Issue:** [NOMS-015-security-hardening.md](../issues/NOMS-015-security-hardening.md)
**Source:** Authentication, authorization, and application security review completed 2026-07-28
**Depends on:** NOMS-004 OAuth authentication, NOMS-006 auth security hardening, NOMS-008 recipe CRUD, NOMS-009 image uploads, NOMS-010 recipe versions/drafts/forks, NOMS-011 collections, NOMS-012 search, NOMS-014 isolated development environments
**Approach:** Write attack-regression tests first, remove the confirmed account-takeover and cross-tenant deletion paths, restore effective OAuth rate limiting, migrate provider identities toward stable subjects, extend bounded abuse controls across the integrated versioning/collection/search surface, and require an independent adversarial review before release.

---

## Outcome and release verdict

This plan is complete only when the application can be reassessed as **good enough for general consumer usage**.

The original review identified the following release blockers:

1. An OAuth login flow started without a session can become an account-link flow when its callback is opened by a signed-in victim.
2. A user can cause another user's recipe or avatar object to be deleted by persisting that public object URL in their own recipe and deleting the recipe.
3. OAuth rate limiting groups all production requests under the fallback `0.0.0.0` address because the server does not attach Axum `ConnectInfo`.

The stable-provider-identity work is also required before declaring completion for deployments that support organization-managed, changeable, or recyclable email addresses. If migration constraints prevent that work from shipping with the three blockers, the release decision must explicitly exclude those environments and track the remaining risk.

The integrated CP7 review on 2026-08-20 confirmed that the original blockers
were materially hardened, but found seven additional, realistically reachable
issues that keep the release verdict at `NOT GOOD ENOUGH`:

1. A logged-out OAuth callback is transferable to a browser that did not start
   the authorization transaction.
2. The public recipe-image deletion server function can remove an object still
   retained by another tenant's valid fork or immutable history.
3. Redacted version-graph nodes and edges expose stable UUIDs for adjacent
   unlisted recipes, turning graph access into an unlisted-resource discovery
   channel.
4. Initial and expansion graph requests build the complete graph before
   selecting their 25- or 5-node response, permitting close to 1 GiB of valid
   snapshot materialization per request at the configured limits.
5. Version-summary reads fetch, decode, and diff all 250 permitted snapshots,
   allowing roughly 250 MiB of valid snapshot processing per request.
6. Fork attribution returns the stored title and UUID of a source recipe after
   that source becomes private.
7. `GET /auth/logout` revokes the current session without same-origin intent
   validation and can be chained before the transferable-login attack.

CP7A–CP7D below are release-blocking remediation checkpoints derived from that
review. CP8 repeats the integrated adversarial gate after all four land.

The integrated CP8 review on 2026-09-02 verified the exact release candidate at
revision `1ec3766`, including the CP8A account-media cleanup, CP8B bounded media
cleanup, and CP8C bounded version-comparison changes. It found two additional
medium-severity release blockers:

1. Recipe-media retention walks at most 32 fork ancestors and treats a
   truncated ancestry search as proof that no owner-authorized reference
   remains. A 33-plus-generation fork chain can therefore allow deletion of
   media that a surviving recipe still renders.
2. A public collection page can return 100 complete recipes. Because each valid
   recipe snapshot may approach 1 MiB, one anonymous request can materialize
   roughly 100 MiB before JSON overhead without an aggregate byte or shared
   concurrency budget.

CP8D and CP8E below are the release-blocking remediation checkpoints derived
from that review. Both are now implemented. Because the application remains in
pre-production with no user data to preserve, CP8F replaces cleanup-time media
ownership inference with explicit durable asset references before CP9 repeats
the integrated adversarial gate. No legacy backfill or compatibility rollout is
required for that clean cutover.

Passing ordinary happy-path tests is not sufficient. Completion requires tests that reproduce each attack and prove that it is rejected without damaging the victim's session, linked accounts, database records, or storage objects.

NOMS-010, NOMS-011, and NOMS-012 materially expand the integrated review
surface. Their authorization and resource-boundary contracts are part of this
release gate even where the feature implementation already contains focused
tests. This expansion does not reopen the completed OAuth checkpoints; it makes
the remaining abuse-control and adversarial-review work match the application
that will actually ship.

### Progress summary

- **Implemented and independently reviewed:** CP0 baseline reliability, CP1 OAuth intent/session binding and same-origin redirects, CP2 integrated media ownership and orphan retention, CP3 trusted-proxy client identity and per-client OAuth limits, CP4 stable provider identities without legacy email reconciliation, CP4A completed-router and collection-media alignment, CP5 integrated resource-abuse bounds, CP5A typed media-upload unification, and CP6 dependency/secret/deployment verification.
- **Completed reviews:** CP7 at revision `cb57bdd` returned `NOT GOOD ENOUGH` with five medium- and two low-severity findings; CP8 at revision `1ec3766` returned `NOT GOOD ENOUGH` with two medium- and seven low-severity findings.
- **Implemented since CP7:** CP7A browser-bound OAuth and logout intent, CP7B reference-aware media deletion, CP7C hidden graph/lineage privacy, CP7D byte-bounded history and graph reads, CP8A account-media cleanup authority, CP8B bounded media cleanup, CP8C bounded version comparisons, CP8D fail-closed deep-fork media retention, CP8E bounded collection recipe projections, and CP8F explicit durable media references.
- **Remaining before the final consumer-security verdict:** CP9 final integrated adversarial re-review. The seven low-severity CP8 findings remain documented residual follow-up unless their aggregate risk is reassessed upward.

---

## Security contract

### OAuth flow intent

Every OAuth authorization state has an explicit, immutable purpose:

- `login`: authenticate as the provider identity;
- `link`: attach a provider identity to the already-authenticated user who initiated the flow.

Flow purpose must never be inferred from whether a callback request happens to contain a session cookie.

The required state/session combinations are:

| Purpose | Stored user | Callback session | Result |
|---|---:|---:|---|
| `login` | none | none | Continue login |
| `login` | none | any user | Reject; do not link or replace the session |
| `link` | user A | user A | Continue account link |
| `link` | user A | none | Reject |
| `link` | user A | user B | Reject |

A product decision may later introduce an explicit account-switch flow, but it must be distinct from linking and is outside this plan.

### Resource ownership

Possession of a public media URL is not proof of ownership.

- Recipe and avatar object keys remain namespace-scoped storage details, but the
  canonical identity and lifetime authority for managed media is its asset ID.
- Only server-derived, ownership-validated object keys may be sent to `delete_object`.
- An external image URL may be displayed or referenced if product behavior permits it, but the application must never translate an external URL into a storage deletion.
- A client-supplied managed URL or asset ID never proves attachment or retention
  authority. Direct attachment is permitted only for an uploader-owned asset or
  an asset already authorized for that exact holder; cross-owner references are
  created only by a server-authorized fork or explicit sharing operation.
- Nested step photos follow the same rules as top-level recipe images.

### Client identity for rate limiting

- Direct peer information must be attached by the actual production server stack.
- Forwarded headers are considered only when the direct peer is a configured trusted proxy.
- Missing client identity must not collapse all clients into a shared public bucket.
- OAuth throttling must work correctly in a production-equivalent TCP test, not only in unit tests that manually insert `ConnectInfo`.

### Provider identity

- Google accounts are identified by `sub`.
- GitHub accounts are identified by the stable numeric `id` or an equivalently stable provider subject.
- Email is profile and recovery metadata, not a provider account identifier.
- Automatic reconciliation by email requires a verified provider email and an explicit migration policy.
- A new OAuth login must never attach to an existing local user solely because an unverified email string matches.

---

## Agent operating model

### Roles

Use three logical roles for every checkpoint:

1. **Coordinator/test author**
   - owns the acceptance criteria;
   - writes the failing attack-regression tests before production changes;
   - records the expected failure mechanism;
   - does not weaken tests to accommodate an implementation.
2. **Implementer**
   - changes production code only after the regression tests exist;
   - makes the smallest coherent change;
   - may add implementation-specific tests but may not remove, skip, or weaken coordinator tests.
3. **Security reviewer**
   - independently traces the complete request and data flow;
   - reviews the full diff and test evidence;
   - attempts bypasses and adjacent variants;
   - returns `PASS` only with no blocking or should-fix security findings.

If only one agent is available, execute these roles in separate passes and review the final diff from a clean context. Do not combine implementation and approval into one unexamined pass.

### Worktrees and sequencing

Use NOMS-014 isolated worktrees and development environments:

```text
security-oauth       CP1 and CP4, serialized
security-storage     CP2
security-rate-limit  CP3
security-integration CP4A after the integrated rebase
security-abuse       CP5 after CP4A controls settle
security-media       CP5A after CP5 upload/quota controls settle
security-release     CP6–CP7 integration and review
security-auth-tx     CP7A browser-bound OAuth and logout intent
security-retention   CP7B reference-aware media deletion
security-graph       CP7C graph and lineage privacy, then CP7D bounded reads
security-review      CP8 integrated review of the first remediated candidate
security-deep-forks  CP8D fail-closed media retention
security-collections CP8E bounded collection recipe projections
security-media-refs  CP8F explicit media assets and durable references
security-re-review   CP9 final integrated adversarial release gate
```

CP2 and CP3 may run in parallel after CP0. CP1 must land before CP4 because both alter OAuth state, user-info extraction, account linking, schema, and tests. CP4A runs after the feature integration and before CP5 so the final router and every media-bearing feature share the intended controls. CP5 starts after those interfaces stabilize. CP5A then unifies the now-bounded upload implementations without changing their product behavior. CP6 and CP7 run on the integrated branch only. After CP7, CP7A and CP7B may proceed independently. CP7C and CP7D are serialized because both change version-graph/history contracts and tests. CP8 reviewed the exact integrated candidate. CP8D and CP8E may proceed independently and are now integrated. CP8F follows them as a clean pre-production schema and media-model cutover; CP9 reviews the exact candidate after all three checkpoints land.

Each implementation branch uses the `codex/` prefix unless the operator selects another repository convention. Agents must not share a source checkout, database, MinIO bucket, browser cookie name, or Dioxus cache.

### Checkpoint handoff packet

Every implementer and reviewer receives:

- the original security finding and exploit sequence;
- the checkpoint's acceptance criteria;
- the exact failing tests and their pre-fix output;
- relevant schema and compatibility constraints;
- all files changed in earlier checkpoints;
- the commands already run and their results;
- known unrelated dirty-worktree changes or baseline failures.

Every checkpoint ends with:

- production diff;
- new and existing test results;
- manual or TCP-level evidence where required;
- reviewer findings classified as blocking, should-fix, or informational;
- explicit `PASS` or `FAIL`.

---

## Checkpoint 0: Baseline, regression harness, and test reliability

**Goal:** Establish trustworthy security tests and a clean-enough baseline before changing behavior.

**Files:** existing auth, upload, database, and middleware test modules; optional focused integration tests under `scripts/tests/`; `justfile` only if a focused security recipe is needed.

### Coordinator tasks

- Record the current revision, working-tree state, Rust version, and resolved dependency versions.
- Run `just ci` and preserve the output.
- Reproduce the two known upload-test failures caused by an uninitialized global storage client.
- Stabilize test storage initialization without weakening assertions or making tests order-dependent.
- Add a focused `just security-test` recipe only if it provides a meaningful, deterministic subset beyond existing commands.
- Ensure tests use isolated temporary PostgreSQL and mock storage/OAuth services.
- Do not use real provider accounts, production tokens, or production storage.

### Required baseline evidence

- Full server test suite can run repeatedly without ordering-dependent failures.
- Security tests can inspect:
  - OAuth account rows before and after callbacks;
  - session identity and validity;
  - storage `DELETE` requests and object keys;
  - resolved client IP and rate-limit buckets.
- Existing unrelated work in the checkout is preserved.

### Validation

```bash
just fmt
just check
just test
just lint
just ci
```

**Gate:** Do not begin behavior changes until failures in the relevant test harness are fixed or documented as a concrete external blocker.

**Risk:** Medium. A flaky global singleton or shared environment can make security regressions appear fixed when tests merely ran in a favorable order.

---

## Checkpoint 1: Separate OAuth login from account linking

**Goal:** Eliminate login-CSRF account takeover by making flow intent explicit and binding link callbacks to the initiating user.

**Primary files:** `migrations/schema.sql`, SQLx offline metadata as required, `src/db/mod.rs`, `src/auth/oauth.rs`, `src/auth/linking.rs`, `src/pages/login.rs`, `src/pages/settings/settings_accounts.rs`.

### Tests first

Add end-to-end handler tests for these exact cases:

1. Attacker starts `login` without a session, authorizes the attacker's provider account, and the callback is opened with victim B's valid cookie:
   - callback is rejected before linking;
   - no OAuth account is added to victim B;
   - no new session replaces victim B's session;
   - the OAuth state is consumed or invalidated safely.
2. User A starts `link`, but callback has no cookie:
   - callback is rejected;
   - no account is created or linked.
3. User A starts `link`, but callback has user B's cookie:
   - callback is rejected;
   - neither user changes.
4. User A starts and completes `link` with user A's cookie:
   - the provider identity is linked to A exactly once;
   - A's existing session remains valid.
5. An unauthenticated `login` completes without a session:
   - normal login still succeeds.
6. Reuse, provider mismatch, expiry, and invalid PKCE continue to fail.
7. Concurrent consumption still permits at most one callback.
8. Every OAuth and logout redirect remains same-origin after URL parsing and browser normalization:
   - reject protocol-relative paths and absolute URLs;
   - reject raw and encoded backslash forms such as `/\\evil.example`, `/%5cevil.example`, and equivalent mixed slash/backslash inputs;
   - reject encoded authority-confusion inputs such as `/%2fevil.example`;
   - verify accepted relative paths resolve beneath the application's own origin using WHATWG-compatible URL semantics, not only a string-prefix check.

The attack test must exercise the real callback handler and account-linking transaction. A unit test that merely recomputes a boolean mismatch is insufficient.

### Implementation tasks

- Add an explicit OAuth state purpose, preferably a constrained value such as `login` or `link`.
- Make login and account-link entry points pass purpose explicitly.
- Require a valid current session before creating a `link` state.
- Store the initiating user ID for every `link` state.
- In the callback, validate the complete `(purpose, stored_user_id, current_user_id)` tuple before token exchange or linking.
- For `login` callbacks carrying an active session, reject with a client-safe response and require logout before switching accounts.
- For `link` callbacks, call a linking-only function that requires a concrete authenticated user ID.
- For `login` callbacks, call a login-only resolver that cannot attach a new provider to an arbitrary current session.
- Preserve one-time state consumption, TTL, provider matching, PKCE, sanitized errors, and safe relative redirects.
- Centralize same-origin redirect validation for OAuth login, callback, and logout flows. Validate the parsed/normalized destination and fail closed on backslashes, encoded separators, authority confusion, or ambiguous decoding.
- Do not return a request cookie as a new `Set-Cookie` value. Only emit cookies built through the hardened cookie builder.

### Schema and migration

- Add the purpose column without silently classifying old states as link states.
- Existing short-lived states may be invalidated during deployment rather than migrated.
- Update SQLx offline metadata after schema/query changes.
- Inspect the schema plan for destructive operations.

### Validation

```bash
just migrate-plan
just schema-check-test
just sqlx-prepare
just fmt
just check
just test
just lint
```

Manual verification in a fresh isolated instance:

- ordinary Google and GitHub mock login;
- linking from settings;
- callback opened in a different signed-in browser context;
- callback replay;
- link flow after session expiry.

**Gate:** The checkpoint fails if any callback can choose link behavior based only on the callback cookie, or if any OAuth/logout redirect can normalize to a different origin.

**Risk:** Critical. This changes the core account identity boundary and must be reviewed before other OAuth refactoring.

---

## Checkpoint 2: Enforce media ownership at persistence and deletion boundaries

**Goal:** Prevent cross-tenant recipe and avatar deletion through attacker-supplied media URLs.

**Primary files:** `src/api/recipe.rs`, `src/api/upload.rs`, `src/storage.rs`, `src/types.rs`; database code only if server-owned keys are persisted separately.

### Tests first

Create tests covering top-level images and recursively nested step photos:

1. User A creates or updates a recipe with user B's internal recipe-media URL:
   - request is rejected with no database change.
2. User A supplies user B's avatar URL as recipe media:
   - request is rejected.
3. User A deletes a legacy recipe record containing user B's URL:
   - recipe deletion may proceed;
   - no storage deletion is sent for B's object.
4. User A deletes a recipe containing A's valid uploaded objects:
   - only A's full and thumbnail objects are deleted.
5. An external HTTPS image URL:
   - follows the documented product policy;
   - it is never sent to storage deletion.
6. Prefix-confusion inputs are rejected:
   - `recipes/{A-id}-suffix/...`;
   - encoded traversal;
   - duplicate slashes;
   - a public URL with a misleading host or path prefix;
   - thumbnail/full-name substitutions outside A's namespace.
7. The direct `delete_image` server function continues to reject cross-user keys.
8. Collection covers require the same exact owner, canonical full/thumbnail pairing, and the expected namespace for their media kind; CP4A moves new custom-cover uploads out of the recipe namespace.
9. New drafts, edit drafts, draft autosaves, publication, restore, and fork publication cannot introduce a new foreign internal media URL.
10. A server-created fork may continue referencing the exact media copied from its accessible source, but cannot use that exception to add a different foreign object.
11. Orphan cleanup retains canonical objects referenced only by a draft, immutable version, collection cover, published fork ancestry, or an unpublished fork whose source was deleted.
12. A forged cross-user reference in any live document is neither a deletion grant nor a retention grant for the victim's object.

Use a mock S3 endpoint that records exact delete requests. Asserting only on a helper's returned string is insufficient for the cross-tenant deletion regression.

### Implementation tasks

- Centralize parsing of public storage URLs into a typed, validated object reference.
- Compare URL origin/base exactly; do not use a loose textual prefix that accepts a sibling host or path.
- Validate the namespace segment as an exact UUID equality with the authenticated user.
- Restrict recipe objects to the expected filename forms generated by the server.
- Traverse all nested recipe steps with one shared validator.
- Validate internal media references during both recipe creation and update.
- Apply the same validation to new/edit draft creation, draft autosave, draft publication, restore, fork publication, and collection cover create/update paths.
- Permit inherited fork media only through a server-established provenance path; possession of an arbitrary foreign internal URL remains insufficient.
- Before deletion, validate every stored reference again and skip/log legacy or foreign references.
- Keep database ownership checks on the recipe itself.
- Prefer persisting a server-issued object key or opaque upload ID in addition to display URLs so future deletion does not derive authority from client-controlled URLs.
- Make orphan cleanup scan live recipes, nested steps, drafts, immutable versions, collection covers, and surviving fork references across every managed namespace while retaining objects only under an exact owner or server-established inheritance proof.

### Validation

```bash
just fmt
just check
just test
just lint
```

Manual verification in a fresh isolated instance:

- create users A and B;
- upload an image for each;
- attempt the attack using B's publicly visible URL from A's recipe;
- repeat it through a collection cover, draft autosave, and nested fork draft;
- verify B's object still returns successfully;
- delete A's legitimate recipe and verify only A's objects are removed.

**Gate:** Every `delete_object` call reachable from a user request must have a preceding server-side ownership proof for that exact key.

**Risk:** High. Incorrect cleanup can destroy another tenant's content even when database authorization is otherwise correct.

---

## Checkpoint 3: Restore per-client OAuth rate limiting

**Goal:** Prevent a small number of unauthenticated requests from disabling OAuth login for every user.

**Primary files:** `src/main.rs`, `src/middleware/rate_limit.rs`; deployment documentation/configuration if trusted proxy addresses must be supplied.

### Tests first

- Start the actual application router behind a real TCP listener using the production serving path.
- Prove the request contains direct peer information without manually adding `ConnectInfo`.
- Through a configured trusted proxy simulation, prove two forwarded client IPs receive independent start and callback budgets.
- Prove an untrusted direct client cannot select its bucket by spoofing `X-Forwarded-For`.
- Prove five invalid callback attempts from attacker A do not block client B.
- Prove sustained requests from attacker A receive `429` with bounded `Retry-After`.
- Prove missing client identity does not map unrelated clients into `0.0.0.0`.
- Retain cleanup tests so attacker-controlled IP cardinality does not cause unbounded memory growth.

### Implementation tasks

- Replace the server launch path or make-service construction so Axum attaches `ConnectInfo<SocketAddr>` in production.
- Keep trusted-proxy parsing anchored to the direct peer.
- Configure Railway or other production proxy networks explicitly; do not treat arbitrary private networks as trusted without deployment evidence.
- Choose an explicit missing-IP behavior:
  - reject the rate-limited endpoint temporarily; or
  - use a narrowly scoped fallback derived from authenticated platform metadata.
- Never use one public global IP bucket as the fallback.
- Keep rate-limit middleware before OAuth handlers so invalid states and token-exchange attempts are throttled.
- Document that in-memory limits are per application replica. If production uses multiple replicas or requires strong global abuse control, move the limiter to a shared store or enforce an upstream limit.

### Validation

```bash
just fmt
just check
just test
just lint
```

Run the TCP integration test in both debug and release-equivalent server configurations because Dioxus uses different serving paths between them.

**Gate:** A reviewer must demonstrate independent buckets through the shipped server entry point, not a synthetic middleware-only router.

**Risk:** High. Incorrect proxy trust creates either global denial of service or a spoofable rate-limit bypass.

---

## Checkpoint 4: Use stable provider subjects and verified email

**Goal:** Remove email reassignment and email-change ambiguity from OAuth identity.

**Primary files:** `src/auth/oauth.rs`, `src/auth/linking.rs`, `src/db/mod.rs`, `migrations/schema.sql`, SQLx metadata, provider mock configuration.

### Required design decision before coding

Inventory existing production OAuth accounts and choose one migration policy:

1. **Secure re-link migration — recommended**
   - existing sessions continue to work;
   - users must explicitly re-link provider identities while authenticated;
   - legacy email identifiers are never accepted as proof of identity.
2. **Just-in-time verified-email migration**
   - on first login, a verified provider email may claim one unambiguous legacy row;
   - the stable subject replaces the legacy identifier atomically;
   - this retains a one-time reassigned-email race and must be time-bounded, monitored, and explicitly accepted.
3. **Administrative backfill**
   - use a provider-authorized, auditable batch process to obtain stable subjects;
   - never infer stable subjects from email alone.

Do not silently implement a fallback that preserves the existing vulnerability indefinitely.

### Tests first

- Google email changes while `sub` stays constant: same local account.
- Two Google subjects present the same email over time: they do not authenticate as each other.
- GitHub public email changes while numeric `id` stays constant: same local account.
- Missing or false `email_verified`: no email-based reconciliation.
- Recycled organization email with a new stable subject: no takeover under the selected post-migration policy.
- Existing linked account conflict remains isolated to the correct user.
- Empty or missing stable subject is rejected; it must not create an account with an empty identifier.
- Provider user-info non-success status is rejected before JSON claim extraction.

### Implementation tasks

- Use Google `sub` as `provider_user_id`.
- Use GitHub numeric `id` as `provider_user_id`.
- Add `email_verified` to the provider user-info model and persist it correctly.
- For GitHub, request the documented user/email scopes and obtain a verified email from the appropriate endpoint when email reconciliation is needed.
- Require non-empty stable identifiers.
- Check user-info HTTP success status before accepting claims.
- Separate:
  - lookup by stable provider subject;
  - optional legacy migration;
  - explicit authenticated account linking;
  - new-user creation.
- Remove indefinite automatic account linking based only on `users.email`.
- Make migration updates transactional and conflict-safe under concurrent logins.
- Add metrics or structured audit events for legacy migrations and linking conflicts without logging access tokens, refresh tokens, authorization codes, or full raw user-info responses.

### Validation

```bash
just migrate-plan
just schema-check-test
just sqlx-prepare
just fmt
just check
just test
just lint
```

**Gate:** Stable provider subject is the primary identity for all newly created OAuth accounts. Any temporary legacy fallback has an owner, expiry date, telemetry, and removal task.

**Risk:** High. A careless migration can either lock out existing users or preserve an account-takeover path.

---

## Checkpoint 4A: Align post-merge router and media controls

**Goal:** Make the integrated NOMS-010/011/012 routes and collection-cover media use the already-reviewed authentication and typed media boundaries before beginning broader abuse-control work.

The post-merge review found no currently exploitable bypass in CP1–CP4. This
checkpoint closes two defense-in-depth inconsistencies that could become
security regressions as new endpoints or media types are added:

1. the Dioxus router attaches `handle_auth` before adding the routes it is
   intended to wrap, so the middleware does not currently supply redirects,
   request identity extensions, or rolling session refresh to those later
   routes;
2. collection covers enforce ownership with a collection-specific textual URL
   parser instead of the canonical typed parser used by recipe and avatar
   media.

Implement and review these as two independent commits.

### CP4A.1: Apply authentication middleware to the completed router

**Primary files:** `src/main.rs`, `src/middleware/auth.rs`, and production-router tests.

#### Tests first

- Build the same Dioxus router used by production and prove `handle_auth` runs for routes added by `serve_dioxus_application`.
- Prove an anonymous request to an owner-only edit, draft, collection-management, or settings route is redirected to login.
- Prove an authenticated request receives the expected identity extensions and a qualifying session receives the rolling-refresh cookie.
- Prove published recipe detail and historical snapshot pages are not blanket login-gated: anonymous access to public and URL-known unlisted history reaches the application authorization layer, while private history remains inaccessible there.
- Prove every state-changing server function still performs its own session and ownership checks; middleware is defense in depth, not the sole authorization boundary.
- Prove the upload body-limit layer and OAuth router rate-limit layer retain their current scopes after router reordering.

#### Implementation tasks

- Construct the Dioxus routes and direct API routes before applying `handle_auth`, so Axum applies the middleware to the completed router.
- Preserve the required pool and storage extensions and document the security-relevant layer order.
- Remove published historical snapshot routes from the blanket protected-path matcher. Continue to enforce public, unlisted, private, and owner access in the recipe/version server and database boundary.
- Keep edit routes, owner-only draft routes, collection-management routes, account settings, and other authenticated application pages protected.
- Do not move the OAuth start/callback/logout router behind the Dioxus authentication middleware; its explicit intent, callback, redirect, and rate-limit controls remain independent.

#### Gate

The production-composed router demonstrably executes authentication middleware
for its intended routes without narrowing intentional anonymous public/unlisted
recipe-history access or replacing server-function authorization checks.

**Risk:** Medium. Incorrect layer ordering can silently disable middleware; an indiscriminate correction can also break the intentional history-visibility policy.

### CP4A.2: Give custom collection covers a typed media lifecycle

**Primary files:** `src/api/upload.rs`, `src/api/collection.rs`, `src/components/base/image_upload.rs`, collection create/edit pages, `src/storage.rs`, `src/db/mod.rs`, `src/worker/orphan_cleanup.rs`, and focused media-ownership tests.

The current custom-cover flow reuses the generic recipe upload path, so cover
objects are stored as `recipes/{owner}/{object}_full.webp` and are protected
only because collection validation and orphan scanning treat them as recipe
media. This is currently owner-safe, but it conflates recipe and collection
lifecycles. New custom collection-cover uploads require their own typed object
kind and namespace while continuing to use the single canonical URL parser.

**Selected rollout:** No compatibility. Startup atomically clears collection
image references that are not canonical, owner-matched `CollectionFull`
objects before the dual-namespace cleanup worker is spawned. Historical
recipe-namespace covers are neither reinterpreted nor backfilled; their now
unreferenced recipe objects become eligible for ordinary orphan reclamation.

#### Tests first

- A custom-cover upload produces exactly one canonical pair under `collections/{owner}/{object}_full.webp` and `collections/{owner}/{object}_thumb.webp`; the ordinary recipe uploader continues to produce only `recipes/` objects.
- Collection creation and update accept only a canonical `CollectionFull` object owned by the authenticated user.
- An optional thumbnail must be the canonical `CollectionThumbnail` with the same owner and object ID as the full image.
- Recipe, avatar, foreign-owner, full/thumbnail substitution, sibling-host/path, credential, query/fragment, duplicate-slash, encoded-traversal, non-canonical UUID, and external URL inputs are rejected without a database write.
- A canonical custom cover is returned by the owner-aware reference scan and reaches the generalized retained-media-key collector as its exact full and thumbnail keys.
- Recipe cleanup cannot delete collection media, and collection cleanup cannot retain or delete recipe or avatar media merely because an object ID or owner matches.
- Removing, replacing, or deleting a collection removes its cover retention grant so the scheduled orphan worker may reclaim the old custom-cover objects, while another user's forged or legacy reference grants neither retention nor deletion authority.
- A custom cover uploaded before a failed/abandoned collection save is eventually reclaimed; partial upload failures synchronously remove only the server-generated keys from that attempt.
- Removing an uploaded-but-unsaved custom cover uses an owner-checked collection-media deletion path rather than the recipe-only `delete_image` path.
- The cleanup cycle loads the complete typed reference set and successfully lists both managed namespaces before issuing any deletion; failure of either prerequisite produces zero delete requests for the entire cycle.
- The selected legacy-cover rollout is exercised with cleanup running concurrently and cannot delete an object that is still displayed by a collection before the cutover transaction commits.
- Ordinary collection creation, editing, sharing, rendering, and cover replacement continue to work.

#### Implementation tasks

- Add `MediaObjectKind::CollectionFull` and `MediaObjectKind::CollectionThumbnail` to the canonical `StorageClient::parse_media_url` model with exact `collections/{owner}/{object}_{full|thumb}.webp` parsing and pairing.
- Add a dedicated collection-cover upload operation, or a closed server-side upload-purpose enum, that selects the collection namespace and object kinds. Do not infer purpose from a filename or accept a caller-supplied storage key.
- Make the shared `ImageUpload` component require an explicit upload purpose and route collection create/edit uploads through the collection-cover operation. Keep recipe uploads on the recipe operation.
- Replace the collection-specific `strip_prefix`/segment parser with the shared typed parser. Require exact owner, collection object kind, and full/thumbnail pairing at collection create and update boundaries.
- Keep custom collection covers restricted to application-owned collection media; do not silently widen them to arbitrary external HTTPS or recipe/avatar objects. Recipe-image mosaics remain derived display content rather than persisted custom-cover authority.
- Replace the recipe-specific retention representation with a typed owner-aware media reference, or equivalent separate recipe/collection reference sets, so the database source determines the expected object kinds. A client-provided URL must not select its own expected kind.
- Generalize `collect_owned_recipe_keys` into a typed retained-media-key collector that covers both `recipes/` and `collections/`, preserves the existing server-established fork exception only for recipe media, and never grants collection media inherited-recipe provenance.
- Preflight the complete typed reference set plus successful `recipes/` and `collections/` listings before deleting from either namespace. After that all-or-nothing preflight succeeds, compute and reclaim each namespace independently. Continue excluding `avatars/`, whose replacement/removal lifecycle remains separate unless a later checkpoint deliberately adds avatar orphan scanning.
- Add an owner-checked custom-cover deletion helper for uploaded-but-unsaved media. It may delete only an exact `CollectionFull` object and its canonical paired thumbnail.
- Keep deletion authority out of stored URLs. Cover replacement/deletion removes the database reference but does not directly delete storage objects; the collection orphan pass may reclaim canonical objects only after no authorized live collection reference remains.
- Select and document one rollout policy before implementation:
  - **No compatibility:** pause/disable cleanup, atomically clear every existing recipe-namespace collection-cover reference while cutting new uploads and validation to `CollectionFull`, verify that no collection retains a recipe-namespace custom-cover URL, then enable strict dual-namespace cleanup. This option performs no re-upload or backfill.
  - **Bounded read-only compatibility:** cut all new writes to `CollectionFull`, preserve existing recipe-namespace covers only as explicitly typed legacy display/retention references, and remove that compatibility after the last reference is replaced or cleared. Legacy references never authorize collection mutation or collection-object deletion against a recipe key.
- Keep cleanup disabled during the cutover until the selected policy's database state and code are both active. New writes must never create recipe-namespace collection covers after cutover, and strict cleanup must not start while a surviving reference lacks a recognized typed retention path.

#### Gate

Recipe images, nested step photos, custom collection covers, avatars, direct
deletion, and orphan retention all derive internal object identity from the
same canonical typed parser while retaining distinct object kinds and
namespaces. Every user-triggered storage deletion remains preceded by exact
ownership and kind proof for the key being deleted. Background orphan deletion
remains restricted to the controlled `recipes/` and `collections/` prefixes and
requires proof that no authorized live reference of the expected type retains
the canonical key.

**Risk:** Medium. The current recipe-namespace cover flow has no demonstrated ownership bypass, but parallel parsers and shared recipe/collection object kinds make later storage-format, cleanup, and quota changes prone to inconsistent enforcement.

### Validation

```bash
just fmt
just check
just test
just lint
```

Run the production-router tests through a real TCP listener and use a mock S3
endpoint to assert the exact collection-cover keys retained or deleted.

---

## Checkpoint 5: Bound realistic authenticated and unauthenticated abuse

**Goal:** Prevent inexpensive resource exhaustion without inventing enterprise-grade controls.

**Primary files:** recipe/version/draft server functions and validation helpers, collection services, search/autocomplete entry points, upload/avatar handlers, pagination functions, database query budgets, rate-limit middleware/configuration.

### Scope

Implement proportionate controls for the application's actual public surface:

- clamp public and authenticated pagination to a documented range such as `1..=100`;
- reject negative offsets;
- bound recipe title, description, commentary, tag count/length, ingredient count, instruction count/depth, equipment count, and image count;
- retain upload byte limits and ensure empty image variants are rejected across recipe, custom collection-cover, and avatar upload paths;
- reject oversized version-graph expansion state before loading or traversing the graph; bound loaded node IDs, requested depth, returned nodes/edges, and relationship traversal independently;
- bound collection hierarchy depth, total collections per user, direct children, memberships, and reorder-array lengths; paginate collection listings that can grow with user-controlled data;
- count custom-cover uploads, replacements, failed/abandoned saves, stored bytes, and object counts in proportionate per-user collection/upload budgets; separating the `collections/` namespace must not create an independent unbounded quota lane;
- bound active drafts per user and apply a documented publication/storage policy to immutable versions so repeated full-snapshot publication cannot cheaply exhaust database storage;
- retain NOMS-012's fixed search grammar, text/filter/page/facet/autocomplete limits, and maximum offset;
- apply proportionate per-user request limits and database execution budgets to expensive search, autocomplete, grouped/faceted search, graph traversal, mutation, and upload paths, with IP fallback only for unauthenticated endpoints;
- treat the 24 MiB server-function body ceiling as a final transport guard, not as the application-level limit for vectors, JSON trees, or graph state;
- ensure limits are applied server-side and return deterministic client errors;
- keep the limits high enough for ordinary recipe usage and document them in one place.

Do not introduce CAPTCHA, device fingerprinting, 2FA, enterprise audit infrastructure, or a new authorization-role system as part of this checkpoint.

### Selected consumer limits and enforcement policy

CP5 uses one server-only limit module (`src/security.rs`) and rejects requests
that exceed a bound rather than silently truncating mutations:

- legacy list pages are `1..=100` items with offsets `0..=10,000`; owner-wide
  collection pickers load at most five individually bounded 100-item pages,
  while root and direct-child queries have an explicit 100-row SQL ceiling.
  The public explore tag catalog is capped at 500 distinct tags;
- recipes allow a 255-character title, 10,000-character description and
  commentary, 50 tags of 100 characters, 200 ingredients (64-character amount
  and unit, 500-character name), 200 total steps to depth 8 with 5,000
  characters per step, 100 equipment names of 200 characters, and 32 total
  recipe/step images. Each image permits a 255-character filename, eight saved
  edits with 100-character names and 4 KiB serialized metadata each, and 16 KiB
  total serialized image metadata. The same metadata validators protect custom
  collection covers and avatar edits. The final serialized editor/snapshot
  ceiling is 1 MiB.
  Drafts may be incomplete, but visibility is always the closed
  private/unlisted/public set. Publication additionally requires title,
  ingredient, instruction, nonnegative times, and positive servings;
- version notes and fork messages are limited to 2,000 characters. A user may
  retain 50 active drafts, 250 immutable versions per recipe, and 5,000
  immutable versions across currently owned recipes. Draft/version allocation
  takes a user-row lock before counting and writing. Publication is also
  limited to 30 requests per user per rolling hour, independently of the
  broader mutation budget;
- collections are limited to 500 per user, depth 10, 100 root/direct siblings,
  and 1,000 recipe memberships per collection. Create, move, and membership
  insertion enforce these inside owner/collection-locked transactions. A move
  checks destination depth plus the moved subtree height;
- a graph expansion accepts at most 250 unique, positive-version loaded-node
  IDs and must include its frontier anchor before any pool lookup or graph
  query. Existing independent graph node, edge, traversal, relationship, and
  depth bounds continue to apply;
- aggregate media across the exact `recipes/{owner}/`, `collections/{owner}/`,
  and `avatars/{owner}/` prefixes is limited to 512 MiB and 2,048 S3 objects.
  The server lists and sums object sizes, then serializes quota-check plus
  upload per user within one replica. Recipe, collection-cover, and avatar
  uploads share this budget and a 60-upload rolling-hour limit. Failed partial
  uploads compensate only keys generated by that request. Successfully
  uploaded but abandoned recipe/collection objects continue to count until the
  hourly orphan scan reclaims them;
- rolling per-user per-operation budgets are 600 searches, 1,200 autocomplete
  requests, 240 graph requests, 1,200 mutations, 30 publications, and 60
  uploads per hour. Anonymous graph/history requests use the trusted-proxy
  resolved client IP. Limiter state is bounded and local to each application
  replica; these are consumer abuse controls, not a distributed global quota;
- expensive list, search, autocomplete, graph, and mutation operations use a
  five-second database execution budget and return sanitized timeout errors.
  The existing 24 MiB request-body ceiling remains a transport guard only.

These limits do not claim bot-proof, distributed-denial-of-service, coordinated
multi-account, or enterprise tenant-isolation protection. Multi-replica rate
limits can be multiplied by the number of replicas unless a later deployment
adds a shared limiter.

### Tests first

- boundary and one-over-boundary tests for every limit;
- deeply nested step payload terminates predictably;
- oversized requests do not write partial database rows or storage objects;
- one user exhausting a quota does not block another;
- extreme pagination cannot force an unbounded public query;
- a maximum-sized valid graph expansion succeeds, while one additional loaded node is rejected before database graph construction;
- oversized collection reorder vectors, excessive hierarchy depth, collection count, membership count, and active-draft count fail without partial writes;
- repeated custom-cover upload/replacement is constrained per user, abandoned objects remain eligible for timely reclamation, and exhausting that budget does not block another user or ordinary recipe uploads;
- repeated draft publication and version creation are constrained by the selected per-user rate/quota policy;
- one authenticated user saturating advanced/faceted search or autocomplete does not consume another user's budget, and database query timeouts fail with a controlled response;
- search counts, facets, grouped results, and autocomplete preserve the same authorization predicates under both boundary-sized and rejected requests;
- normal large recipes and avatar/recipe/custom-cover uploads continue to work.

### Validation

```bash
just fmt
just check
just test
just lint
```

**Gate:** Every attacker-controlled collection, vector, recursion/traversal depth, pagination value, search operation, upload, draft/version growth path, and persisted large text field has a documented server-side bound. Sustained expensive reads and mutations have per-user isolation plus a database execution budget.

**Risk:** Medium. Limits are security controls but also product behavior; choose values from real usage rather than arbitrary tiny thresholds.

---

## Checkpoint 5A: Unify typed media upload transactions

**Goal:** Remove parallel recipe/collection and avatar upload implementations by routing every application-owned media upload through one server-internal, typed transaction engine without weakening the distinct persistence rules for each media purpose.

**Primary files:** `src/api/upload.rs`, `src/pages/settings/settings_profile.rs`, `src/storage.rs`, `src/security.rs`, recipe/collection upload callers, and focused mock-storage tests.

The existing recipe and collection-cover flow shares `upload_media_variants`,
while avatar upload independently repeats key generation, quota locking, object
uploads, partial-failure compensation, and logging. Avatar upload is currently
owner-safe, but maintaining two implementations makes later fixes to upload
atomicity, quotas, validation, and observability easy to apply inconsistently.

The shared engine is internal infrastructure, not a new generic authorization
surface. Recipe and collection callers may continue selecting their closed
public `UploadPurpose`; avatar profile mutation remains available only through
the authenticated avatar operation, which supplies the avatar purpose to the
shared engine and updates the current user's profile itself.

### Tests first

- Every supported purpose produces only its canonical server-generated keys:
  - recipe: `recipes/{owner}/{object}_{full|thumb}.webp`;
  - collection cover: `collections/{owner}/{object}_{full|thumb}.webp`;
  - avatar: `avatars/{owner}/{object}-original`, `avatars/{owner}/{object}.webp`, and `avatars/{owner}/{object}-thumb.webp`.
- Purpose, expected variant set, content type, size policy, object kind, and namespace are selected by a closed server-side policy; filenames, URLs, and caller-supplied storage keys cannot change them.
- Recipe and collection uploads reject an avatar purpose at the generic client-callable boundary. Only the authenticated avatar profile operation may invoke the avatar policy and persist an avatar URL.
- Authentication is required before key generation or storage writes for all three purposes, and generated keys always use the authenticated user's canonical UUID.
- A failure while uploading any variant compensates every object successfully created by that attempt and no pre-existing or foreign object. Exercise failure at each position, including avatar original, full, and thumbnail uploads.
- A database failure after a complete avatar upload removes all three newly generated avatar objects and preserves the previous avatar reference and objects.
- Successful avatar replacement commits the new database references before best-effort deletion of the previous exact owned `AvatarFull`, paired thumbnail, and `AvatarOriginal` objects.
- Recipe, collection-cover, and avatar uploads share the CP5 per-user lock, aggregate quota, upload rate limit, byte/object accounting, and sanitized logging behavior; one purpose cannot obtain an independent quota lane.
- Existing WebP checks, avatar original/rendered size limits, image-edit metadata validation, recipe/collection byte limits, collection unsaved-cover cleanup, and ordinary upload behavior remain intact.
- Concurrent uploads for one user cannot both pass quota checks against the same starting usage, while another user's upload remains independent.

### Implementation tasks

- Extend the typed media-purpose model to include avatars and define one internal upload specification for namespace, canonical key shapes, required variants, content types, and per-variant validation policy.
- Replace the fixed two-variant helper with a shared transaction that accepts only validated variant bytes, generates one object UUID, uploads the specification's complete variant set, and records the exact keys created by the current attempt.
- Centralize reverse-order compensation so every failure path deletes only keys recorded for that attempt. Cleanup failures remain logged with structured key kind/purpose metadata and do not expose storage details to clients.
- Keep authorization and persistence orchestration outside the storage primitive: `upload_avatar` authenticates the user, validates avatar edit metadata, invokes the shared avatar transaction, updates the user's avatar fields, compensates on database failure, and only then retires the previous owned avatar.
- Keep recipe and collection persistence validation based on the canonical typed URL parser. Selecting an upload purpose never grants authority to persist, retain, or delete a media object under a different resource kind.
- Reuse the CP5 request budget, per-user upload lock, and aggregate quota calculation exactly once per logical upload transaction. Account for the correct number and byte size of variants before the first storage write.
- Return a typed internal upload result that exposes canonical objects/URLs by variant rather than relying on suffix replacement or positional string assumptions. Adapt the existing two-URL recipe/collection response at its boundary.
- Remove duplicated avatar key generation, object upload, compensation, and quota plumbing after the shared path has equivalent regression coverage.
- Do not migrate or backfill existing objects: current recipe, collection, and avatar key formats already match the canonical parser. This checkpoint changes implementation ownership, not stored URL formats or database schema.
- Keep avatar orphan scanning outside this refactor. Avatar replacement/removal retains its immediate, owner-checked lifecycle; expanding background retention or reclamation requires separate acceptance criteria.

### Validation

```bash
just fmt
just check
just test
just lint
```

Use a mock S3 endpoint to assert exact ordered PUT/DELETE behavior for every
variant and injected failure position. Review the final diff specifically for
authorization widening at the generic upload server-function boundary.

**Gate:** Recipe, collection-cover, and avatar uploads use one typed internal transaction and one quota/compensation path. Public callers cannot select avatar persistence authority, every returned object has a canonical kind and authenticated owner, and failures cannot delete an object not created by that attempt.

**Risk:** Medium. This is primarily technical-debt reduction, but an overly generic public API could accidentally let arbitrary callers mutate avatar state or mix media kinds. Preserve purpose-specific authorization and database boundaries while sharing only the upload mechanics and policy enforcement.

---

## Checkpoint 6: Dependency, secret, and deployment verification

**Goal:** Make the secure code path the deployed path and prevent known dependency drift from silently accumulating.

**Primary files:** CI configuration if present, `Cargo.toml`, `Cargo.lock`, `Dockerfile`, deployment documentation/configuration.

### Tasks

- Add a reproducible RustSec audit to CI.
- For every advisory that remains:
  - prove whether it is in the production server graph;
  - prove whether affected APIs/features are reachable;
  - record a narrow, expiring allowlist entry only when non-reachability is concrete.
- Update directly or transitively fixable advisories without broad unrelated upgrades.
- Pin production build tools currently installed from moving `latest` references.
- Pin the PostgreSQL/ParadeDB extension image and verify the production database exposes only the extensions and privileges required by the application role.
- Configure and test connection-pool, statement-timeout, and transaction-timeout budgets for search, graph, and mutation workloads; a single expensive request must not hold a database connection indefinitely.
- Verify production requires:
  - a high-entropy `SESSION_SECRET`;
  - real OAuth client IDs/secrets and HTTPS provider endpoints;
  - an HTTPS `BASE_URL`;
  - `NOMS_ENV` not set to `local`;
  - host-only cookies unless a broader `COOKIE_DOMAIN` is explicitly required;
  - least-privilege database and object-storage credentials.
- Verify TLS redirection/HSTS at the application or deployment platform boundary.
- Remove the raw Google user-info error log and confirm secrets, tokens, codes, cookies, PKCE verifiers, and refresh tokens are not logged.
- Keep local mock credentials and loopback endpoints confined to isolated development.
- Scan tracked files for real credentials; do not print discovered secret values into CI logs.

### Advisory baseline from the 2026-07-28 review

The lockfile audit reported six vulnerability advisories:

- three were not present in the production server graph;
- three `rustls-webpki` advisories were present through the AWS SDK but affected CRL or certificate-name-constraint behavior not configured by the application.

Re-evaluate rather than copying that conclusion indefinitely because the advisory database and dependency graph change over time.

### Validation

```bash
cargo audit
just ci
```

Build the production Docker image and inspect its runtime user, environment contract, exposed listener, and included artifacts. Do not embed `.env.local`, Cargo credentials, Git metadata, or development OAuth configuration.

**Gate:** CI fails on a new reachable vulnerability and production startup fails safely when security-critical configuration is missing or insecure.

**Risk:** Medium. Most work is preventative, but configuration drift can bypass otherwise correct cookie and OAuth behavior.

### Completion evidence (2026-08-20)

- Production configuration now fails closed for insecure or missing secrets,
  origins, provider endpoints, database role separation, and Railway-local mode.
- CI pins build inputs and rejects new reachable dependency advisories, tracked
  credential patterns, mutable deployment artifacts, and unpinned production
  dependencies. The reviewed RustSec exception is narrow and expires.
- The runtime database role is verified as a non-owning data-plane role with
  bounded pool and server-side statement/transaction timeouts.
- At revision `6f6f659`, the compile, lint, schema, security-policy, secret,
  deployment-artifact, and pinned dependency-audit gates passed with 656 active
  tests passing and 10 environment-dependent tests ignored.
- The production image built successfully from revision `6f6f659` as
  `noms-cp6-validation@sha256:923fbc0ff63c2898ec58a43ab0cb7166d91a49c207b076ab28b68d8a5274e499`.
  Artifact inspection confirmed UID/GID 999 (`noms`), a root-owned read-only
  application directory and entrypoint, port 8080 plus a health check, only
  `PORT` and `IP` application defaults, and no source, Git, Cargo credential,
  or local environment files. Startup without `DATABASE_URL` failed closed.
- Independent focused reviews returned `PASS` in
  `/tmp/noms-cp6-final-pass.html` and `/tmp/noms-cp6-docker-review.html`. CP7
  must still repeat the image/runtime inspection in its fresh release-candidate
  environment so the final gate validates the integrated release revision
  rather than this CP6 snapshot.

---

## Checkpoint 7: Integrated adversarial review and release gate

**Goal:** Reassess the integrated application, not isolated patches, against the original verdict.

**Environment:** Fresh NOMS-014 isolated worktree and application instance built from the integrated candidate revision. Do not reuse an implementer's running instance.

### Automated gate

```bash
just ci
cargo audit
```

Required focused evidence:

- login-CSRF attack rejected;
- cross-tenant recipe-image deletion rejected;
- cross-tenant avatar deletion rejected;
- independent OAuth rate-limit buckets through a real TCP listener;
- stable provider subject tests;
- session creation, refresh, revocation, and logout tests;
- recipe read/update/delete ownership tests;
- account unlink ownership and last-login-method tests;
- upload size/type/ownership tests;
- unified recipe, collection-cover, and avatar upload transaction tests, including per-variant compensation failures and rejection of avatar purpose at the generic upload boundary;
- pagination and structured recipe limit tests;
- current-visibility authorization tests across version lists, historical snapshots, comparisons, lineage, and version graphs;
- owner-only draft read/save/publish/discard tests, including stale and cross-user revisions;
- collection owner/hierarchy/membership/reorder authorization tests and collection/recipe visibility-intersection tests;
- search, grouped search, facets, counts, and autocomplete tests proving private and unlisted cross-user data cannot be inferred;
- graph, collection, draft/version, search, autocomplete, and reorder boundary/abuse tests;
- orphan-cleanup retention tests for drafts, versions, collection covers, and deleted-source forks.

### Manual adversarial walkthrough

Use at least two users and two separate browser contexts:

1. Start OAuth as attacker A, capture the unconsumed callback, and open it as signed-in victim B.
2. Start linking as A and complete it with no session and with B's session.
3. Replay successful and failed callbacks.
4. Put B's recipe image and avatar URLs into A's recipe payload, including nested step photos, then update and delete A's recipe.
5. Exhaust A's OAuth callback budget and complete a login as B.
6. Change mock provider email while retaining the stable subject, then change the subject while reusing the email.
7. Exercise session expiry, rolling refresh, logout revocation, and account deletion.
8. Verify public, unlisted, and private recipe access remains consistent.
9. As a non-owner, read public and URL-known unlisted version history, snapshots, comparisons, and graph nodes; verify private history is inaccessible and adjacent private branches are redacted.
10. Change a recipe from private to public/unlisted and confirm that its older snapshots intentionally become readable; change it back to private and confirm non-owner access is removed.
11. Attempt cross-owner collection move, reorder, membership, cover-image, and visibility operations; verify collection sharing never widens a member recipe's own visibility.
12. Search as two users and verify recipe results, grouped sections, facets, counts, and autocomplete consistently exclude the other user's private/unlisted recipes and collections.
13. Exercise maximum and one-over-maximum graph expansion, collection reorder, hierarchy, draft/version, pagination, and advanced-search inputs, then verify another user remains unaffected.

### Reviewer questions

- Can any callback attach an identity without an explicit link state?
- Is every link state bound to the same live user at start and callback?
- Can any client-controlled URL become a storage deletion key?
- Can malformed URL encoding or prefix confusion escape a namespace?
- Does the deployed server, not just the unit-test router, supply trustworthy client identity?
- Can provider email change or reassignment alter account ownership?
- Are all authorization decisions repeated server-side at the data mutation boundary?
- Does current recipe visibility govern version history consistently without exposing private snapshot content or metadata through comparisons, graph expansion, or deleted lineage nodes?
- Can collection visibility, membership, breadcrumbs, counts, or cover media reveal a recipe or collection that the viewer could not access directly?
- Do search results, facets, counts, grouped sections, and autocomplete use identical authorization predicates?
- Can a caller turn graph state, collection reorder arrays, deep hierarchy, repeated full snapshots, or advanced/faceted search into disproportionate CPU, memory, database, or storage consumption?
- Did any fix create session fixation, cookie weakening, open redirect, token leakage, or user enumeration?

### Portable review report

Produce a self-contained HTML report under `/tmp` containing:

- original findings and acceptance criteria;
- candidate revision and environment;
- attack walkthroughs and outcomes;
- relevant test commands and results;
- screenshots where browser state helps prove user separation;
- existing-feature regression checks;
- remaining limitations and explicitly accepted risks;
- final `GOOD ENOUGH` or `NOT GOOD ENOUGH` verdict.

### Final release criteria

The security reviewer may return `GOOD ENOUGH` only when:

- CP1, CP2, and CP3 have no blocking or should-fix findings;
- CP4A production-router and typed collection-media lifecycle gates pass without weakening server-function authorization or intentional public/unlisted history access;
- CP4 is complete, or the deployment explicitly excludes recyclable-email environments with a time-bounded migration plan;
- `just ci` passes;
- no reachable unmitigated critical or high dependency advisory remains;
- the exact attacks from the review fail safely;
- ordinary login, linking, recipe CRUD, public sharing, uploads, profile settings, logout, and account deletion still work;
- the documented current-visibility policy is enforced consistently for public, unlisted, and private historical snapshots;
- drafts remain owner-only, collection visibility never overrides member-recipe visibility, and search cannot infer inaccessible entities through metadata or counts;
- integrated graph, collection, draft/version, search, and autocomplete resource controls meet CP5's per-request and sustained-abuse gates;
- recipe, collection-cover, and avatar uploads meet CP5A's shared typed-transaction, quota, compensation, and purpose-specific authorization gate;
- no security control relies only on client UI state;
- residual risks are operational or specialized rather than straightforward cross-user compromise.

### CP7 completion evidence (2026-08-20)

- The review ran from a fresh detached worktree at revision `cb57bdd2` with an
  isolated application, ParadeDB/PostgreSQL, MinIO, and Mock OAuth stack.
- `just ci` completed its compile, Clippy, 656-test, schema, policy, secret, and
  deployment checks. The dependency audit passed the repository policy with 17
  explicitly allowed warnings. All nine focused ParadeDB search authorization
  tests passed separately against the live pinned database.
- Ordinary browser OAuth login worked. Separate cookie-jar attack validation
  proved that a logged-out victim could consume an attacker-authorized callback
  and receive the attacker's session. Replay returned 401, an already signed-in
  victim received 409 and retained their session, and a state-changing GET
  logout revoked a valid session.
- Independent source-to-sink review confirmed the reference-blind recipe image
  deletion, unlisted graph-identifier disclosure, private fork-attribution
  leak, eager complete-graph construction, and unpaginated history-summary
  amplification paths.
- The canonical scan recorded seven high-confidence findings: five medium and
  two low. The final CP7 verdict was `NOT GOOD ENOUGH`.
- Review artifacts were written outside the repository at
  `/tmp/noms-cp7-security-review.html` and the canonical Codex Security scan
  directory for scan `4d7c78e2-2ce7-43e2-ad90-2274fccd29fa`.

---

## Checkpoint 7A: Bind OAuth transactions to the initiating browser

**Goal:** A valid OAuth callback must create or link authority only in the
browser context that initiated that exact transaction, and logout must require
an intentional same-origin state-changing request.

**Findings addressed:** Transferable logged-out OAuth login callbacks and
state-changing GET logout.

**Depends on:** CP7. May proceed independently of CP7B. Must land before CP8.

**Primary files:** `src/auth/oauth.rs`, `src/auth/logout.rs`,
`src/auth/session.rs`, `src/db/mod.rs`, `src/main.rs`, auth-state schema and SQLx
metadata, and OAuth/logout integration tests.

### Tests first

Add attack-regression tests that prove the pre-fix candidate fails for the
expected reason:

1. Browser A starts login and authorizes provider identity A; logged-out
   browser B attempts the unconsumed callback and receives no application
   session.
2. Browser A completes its own callback successfully exactly once.
3. A callback with a missing, malformed, expired, or mismatched browser
   transaction binding fails before token exchange and never returns a session
   cookie.
4. Existing state expiry, provider mismatch, replay, login-session conflict,
   link-session requirement, and link-user mismatch cases remain rejected.
5. Two explicitly supported concurrent authorization attempts behave according
   to a documented policy; one flow must not silently authorize the other.
6. `GET /auth/logout` does not revoke or clear a session.
7. Logout POST with a missing, duplicate, `null`, malformed, or non-matching
   `Origin` header returns 403 without changing the session. A POST whose
   parsed origin exactly matches the configured `BASE_URL` origin revokes the
   database session and clears the cookie.
8. A forced-logout navigation followed by an attacker callback still cannot
   swap the browser into the attacker account.

### Implementation tasks

- Generate a high-entropy browser transaction binding at OAuth start.
- Store only a digest or keyed representation with the auth-state row; do not
  store or log the raw browser secret.
- Deliver the browser value in a short-lived `HttpOnly`, `Secure` in
  production, `SameSite=Lax` cookie with the narrowest practical host/path
  scope and lifetime no longer than the auth-state lifetime.
- Require an exact binding match at callback before exchanging the provider
  code or issuing/linking authority. Preserve atomic one-time state handling.
- Clear the transaction cookie on success and terminal failure without
  weakening the application session cookie.
- Define and test the concurrent-flow policy rather than accidentally relying
  on one global cookie value.
- Remove state mutation from GET logout. Make revocation POST-only and require
  exactly one syntactically valid `Origin` header whose scheme, host, and
  effective port exactly match the configured `BASE_URL` origin. Reject
  missing, duplicate, `null`, malformed, or mismatched origins with 403; do not
  fall back to `Host`, `Referer`, forwarded headers, or a permissive no-header
  path.
- Preserve normalized same-origin redirects and sanitized OAuth error/log
  behavior.
- Add aggregate metrics for binding mismatches and rejected logout intent;
  never include state, code, verifier, cookie, token, or provider response
  values.

### Acceptance criteria

- Possession of an otherwise valid callback URL is insufficient to create or
  link an application session in another browser context.
- The correct initiating browser can complete ordinary Google and GitHub login
  and explicit account linking.
- A mismatched callback cannot replace, attach to, or clear an existing
  application session.
- PKCE, one-time state, provider, purpose, user, expiry, and redirect controls
  remain intact.
- No safe/idempotent HTTP method revokes a session, and no POST without the
  exact configured origin revokes a session.
- The forced-logout plus transferred-callback chain fails end to end.

### Validation and review gate

Run focused auth tests, `just ci`, and a fresh two-context Mock OAuth browser
walkthrough. Capture the exact Set-Cookie/status/profile outcomes for the
correct, missing-binding, wrong-binding, replay, signed-in conflict, explicit
link, GET logout, every rejected Origin category, and exact-origin POST logout
cases.

**Gate:** The security reviewer returns `PASS` only when no callback authority
can cross browser transaction boundaries and logout revocation requires an
exact configured-origin POST without regressing ordinary login/link/logout.

**Risk:** High. Cookie scope, concurrency, callback ordering, and state
consumption mistakes can create login failures, session fixation, or a new
account-linking path.

---

## Checkpoint 7B: Make recipe-media deletion reference-aware

**Goal:** An object must not be deleted while any authorized live recipe,
draft, immutable version, fork, or collection projection still retains it,
even when the deletion caller owns the storage namespace.

**Finding addressed:** The public `delete_image` server function can remove
media inherited by another tenant's valid fork and history.

**Depends on:** CP7. May proceed independently of CP7A. Must land before CP8.

**Primary files:** `src/api/upload.rs`, `src/db/mod.rs`,
`src/db/versioning.rs`, `src/worker/orphan_cleanup.rs`, `src/storage.rs`, shared
typed-upload components, and mock-storage integration tests.

### Tests first

Add mock-storage attack-regression tests covering:

1. User A uploads recipe media, user B forks A's public/unlisted recipe, and A
   attempts direct deletion; neither full nor thumbnail object is deleted.
2. Retention holds when the source recipe is later private or deleted and the
   surviving reference exists only in B's draft, live fork, immutable history,
   or a collection recipe preview.
3. A caller cannot use encoded, malformed, wrong-kind, wrong-owner, or external
   URLs to influence a deletion decision.
4. A genuinely unsaved, unreferenced upload can still be discarded immediately
   with its one-use attempt-bound capability, and only objects created by that
   upload attempt are removed.
5. A concurrent save/reference operation cannot race a successful discard into
   deleting a newly retained object.
6. Recipe, collection-cover, and avatar replacement/removal preserve their
   purpose-specific lifecycle and compensation behavior.

### Implementation tasks

- Replace the generic client-callable `delete_image(full_url)` authority with
  an opaque, short-lived, one-use discard capability issued by the server for
  one exact upload family. Retirement without an immediate server-managed
  discard replacement does not satisfy this checkpoint.
- Store only a digest of the discard capability with its authenticated owner,
  media purpose, exact full/thumbnail keys, expiry, and lifecycle state. A
  caller must not supply or change the URL, owner, kind, or key set at discard
  time.
- Serialize the upload family lifecycle under the same database transaction and
  per-owner/object coordination used by persistence: `uncommitted` may become
  either `retained` or `discarded` exactly once. Publish, fork, and persistence
  may retain only an eligible family; discard may claim only an uncommitted,
  unreferenced family. After `retained`, the attempt capability can never
  authorize storage deletion.
- Centralize the retained-reference decision used by normal recipe deletion,
  direct discard, and orphan cleanup. Include live recipes, drafts, immutable
  versions, authorized inherited fork provenance, and any collection view that
  depends on the recipe object.
- Make the final reference check and deletion authorization race-safe. Do not
  perform a stale read followed by an uncoordinated storage delete while a
  publish/fork operation can establish a new reference.
- Keep exact canonical URL, owner UUID, object kind, filename, full/thumbnail
  pairing, and external-URL rejection checks.
- Do not broaden inherited-owner exceptions to collection-cover or avatar
  namespaces.
- Ensure failed or partial deletion has explicit, observable behavior and does
  not report successful cleanup while leaving an inconsistent pair.

### Acceptance criteria

- Namespace ownership alone is never treated as proof that a recipe object is
  safe to delete.
- Another tenant's valid fork, draft, live recipe, immutable history, or
  collection display cannot be broken by the original object owner.
- The user can still immediately remove an abandoned upload without waiting for
  orphan cleanup, but only with a one-use capability bound to that exact
  unreferenced attempt and lifecycle transition.
- Existing recipe, collection-cover, and avatar upload/replacement UX continues
  to work with the shared typed transaction and quota accounting.
- Orphan cleanup and direct deletion use one consistent retention contract.

### Validation and review gate

Run the focused mock-storage suite with exact ordered storage-call assertions,
the fork/deleted-source retention tests, unified upload compensation tests, and
`just ci`. Manually exercise upload/cancel, publish/fork, source deletion, fork
rendering, collection preview, and eventual unreferenced cleanup in a fresh
two-user environment.

**Gate:** The security reviewer returns `PASS` only when every deletion sink
re-proves both namespace authority and absence of retained references, including
the concurrent save/delete boundary.

**Risk:** High. An incomplete reference inventory can cause irreversible
cross-tenant data loss, while an overly broad retention rule can leak storage
indefinitely.

---

## Checkpoint 7C: Remove stable identifiers from hidden graph and attribution projections

**Goal:** Public graph and fork-attribution responses may preserve explicitly
accepted topology, but must not reveal a stable locator or private metadata for
a recipe the viewer could not discover under current visibility rules.

**Findings addressed:** Redacted graph nodes/edges disclose unlisted recipe
UUIDs, and fork attribution discloses a private source title and UUID.

**Depends on:** CP7. Serialize before CP7D because both modify version graph
contracts, queries, wire types, and tests. Must land before CP8.

**Primary files:** `src/types.rs`, `src/api/recipe.rs`,
`src/db/versioning.rs`, `src/pages/recipe_detail.rs`, graph/lineage components,
and serialization/authorization tests.

### Tests first

Add response-level tests proving:

1. An anonymous viewer of a public recipe connected to an unlisted recipe
   receives no stable UUID in hidden nodes, edges, frontiers, counts, links, or
   serialized auxiliary fields that can be passed to `get_public_recipe`.
2. URL-known unlisted recipe pages and their intentionally readable history
   still work when the caller already possesses that recipe ID.
3. Adjacent private recipes reveal no ID, title, owner, image, timestamp,
   change summary, fork message, or deleted snapshot metadata.
4. A public/unlisted fork whose source later becomes private returns redacted
   attribution without source UUID or title.
5. Deleted-private source attribution remains redacted; public source
   attribution remains complete.
6. Graph expansion works with the chosen opaque/topology representation and
   does not accept a handle from a different recipe, viewer, response, or
   expired context.

### Implementation tasks

- Replace stable recipe/version IDs for inaccessible graph endpoints with
  either omitted hidden endpoints or opaque, scoped graph handles that cannot
  be reused as direct recipe locators.
- Apply the same rule to node IDs, fork-edge source/target values, frontiers,
  comparison affordances, links, and any client cache key serialized to the
  viewer.
- If opaque handles are retained across expansion requests, authenticate their
  selected recipe, viewer/anonymous context, direction, expiry, and integrity
  server-side. Do not expose a reversible UUID encoding.
- Resolve the source recipe and apply current visibility before returning fork
  attribution. Redact deleted-private sources using their captured visibility.
- Keep the product decision that non-owners may read public and URL-known
  unlisted history/snapshots. Do not reintroduce blanket owner-only history.
- Document whether hidden-branch existence/count topology remains intentionally
  visible; no content-bearing or reusable identifier may be included.

### Acceptance criteria

- Public graph access cannot be used to discover a related unlisted recipe's
  direct-access UUID.
- Private and inaccessible source metadata remains redacted after visibility
  changes and deletion.
- Known public/unlisted graph, history, comparison, and attribution workflows
  continue to work with deterministic expansion and no cross-context handles.
- No client UI state is relied on to hide a stable identifier already present
  in the server response.

### Validation and review gate

Run graph serialization snapshots, public/unlisted/private visibility matrices,
source visibility-transition tests, deleted-lineage tests, and `just ci`.
Inspect raw server-function responses rather than only the rendered UI. In a
fresh two-user walkthrough, attempt to extract and reuse every hidden node,
edge, frontier, attribution, DOM, and network identifier.

**Gate:** The security reviewer returns `PASS` only when an inaccessible graph
endpoint or attribution cannot be converted into a direct recipe locator or
private metadata read while intentional URL-known unlisted access still works.

**Risk:** High. Graph topology depends on stable internal identity; careless
redaction can either leak the same UUID elsewhere or break expansion,
comparison, caching, and accessibility behavior.

---

## Checkpoint 7D: Make history and graph work page- and byte-bounded

**Goal:** CPU, decoded bytes, database rows, and transient memory for history
and graph requests must scale with the requested response page, not the entire
tenant-controlled history or connected graph.

**Findings addressed:** Complete-graph construction before 25/5-node paging and
unpaginated 250-snapshot version-summary decoding/diffing.

**Depends on:** CP7C. Must land before CP8.

**Primary files:** `src/api/recipe.rs`, `src/db/versioning.rs`,
`src/db/diff.rs`, `src/security.rs`, recipe history/graph types and pages,
schema/SQLx metadata if summary projection is persisted, and resource-bound
tests.

### Tests first

Create large but non-exhaustive fixtures with instrumented query/decode counts
and prove the pre-fix behavior exceeds the intended bounds:

1. Initial graph returns at most 25 nodes while fetching/decoding at most the
   response page plus a small documented lookahead, not 1,000 full snapshots.
2. Expansion returns at most five new nodes and does not reconstruct already
   loaded or unrelated branches.
3. Version-summary reads use cursor pagination and fetch/decode at most one page
   plus the minimum adjacent data required for summaries.
4. Aggregate encoded and decoded bytes are capped independently of row count;
   one oversized page fails safely before large allocation.
5. Concurrent maximum-cost graph/history requests are bounded by a dedicated
   semaphore or equivalent work budget and do not starve ordinary reads for
   another user.
6. Stale, forged, duplicate, cross-recipe, and over-limit expansion cursors or
   loaded-node state fail without triggering complete-graph work.
7. Current-visibility authorization and CP7C opaque hidden-endpoint behavior
   remain identical across every page.

### Implementation tasks

- Replace `get_recipe_version_graph(...complete...)` followed by in-memory
  batching with bounded frontier queries that fetch only the requested nodes
  and a small lookahead needed to describe further frontiers.
- Push ordering, traversal limits, and lightweight projection into bounded SQL
  where practical. Do not load full snapshots for nodes whose response needs
  only title/image/change-summary metadata.
- Add a cursor/page contract for version summaries. Persist or derive bounded
  lightweight summary metadata at publish time instead of decoding all
  historical snapshots on every read.
- If schema-denormalized summary metadata is introduced, define and review its
  migration/backfill policy explicitly. Do not preserve old rows through an
  unbounded fallback that recreates the finding.
- Enforce an aggregate snapshot-byte budget before or during decoding, with
  checked accounting for encoded JSON, decoded structures, and diff inputs.
- Add a dedicated concurrency/work budget for graph and history operations in
  addition to sustained per-user/IP request rates.
- Preserve deterministic ordering, comparison direction, truncation metadata,
  and no-duplicate expansion behavior.
- Instrument bounded rows/bytes/duration/rejection counts without logging
  recipe content or identifiers.

### Acceptance criteria

- Initial and expansion work has a documented constant relationship to the
  25/5-node response size and lookahead; it cannot fetch/decode the 1,000-node
  maximum first.
- Version-summary work has a documented constant relationship to its page size
  and cannot decode all 250 snapshots in one request.
- Aggregate bytes and concurrent expensive operations have explicit server-side
  ceilings below process-memory and database-pool danger levels.
- A patient attacker who fills every valid quota cannot turn one anonymous read
  into hundreds of megabytes of snapshot allocation.
- History, graph, comparisons, visibility, and CP7C privacy behavior remain
  functionally correct.

### Validation and review gate

Run focused query-count/decoded-byte tests, maximum and one-over-maximum cursor
and page tests, concurrent work-budget tests, current-visibility matrices, and
`just ci`. Record SQL row counts and application byte counters for large valid
fixtures. Use a controlled load test with conservative host limits; do not prove
the fix by intentionally exhausting the workstation.

**Gate:** The security reviewer returns `PASS` only with measured evidence that
database rows, decoded snapshots, diff operations, peak bytes, and concurrent
work remain within the documented page-level bounds.

**Risk:** High. This changes core graph/history query shapes and may require
schema projection changes; incorrect cursors can omit, duplicate, reorder, or
de-authorize historical data.

---

## Checkpoint 8: Integrated review of the first remediated candidate

**Goal:** Repeat CP7 against the exact integrated candidate after CP7A–CP7D and
determine whether that candidate meets the consumer-security release gate.

**Environment:** A fresh detached worktree and isolated NOMS-014 application,
database, storage, Mock OAuth server, browser profiles, cookie name, ports, and
build cache. Rebuild and inspect the production image from the candidate
revision; do not reuse CP7 runtime state or an implementer's environment.

### Required gate

- Run `just ci`, the current dependency/secret/deployment policy, all focused
  search tests, and the production image/runtime inspection.
- Repeat every CP7 automated and manual adversarial case.
- Repeat the exact transferred-callback attack with logged-out and signed-in
  victims, the forced-logout chain, correct-browser completion, explicit link,
  replay, expiry, and mismatch variants.
- Repeat the fork-retained image deletion attack across live forks, drafts,
  immutable versions, deleted sources, and collection projections.
- Inspect raw graph/attribution responses for reusable hidden identifiers and
  private metadata, then attempt direct unlisted reads with every value exposed.
- Measure graph/history rows, decodes, diff operations, bytes, duration, and
  concurrent work on large valid fixtures.
- Re-run the ordinary login, linking, logout, recipe CRUD/history/fork,
  collection sharing/cover, unified upload/avatar, search, profile, and account
  deletion regression walkthroughs.
- Obtain a separate independent security review of the complete diff and exact
  candidate, not only one review per remediation checkpoint.

### Review acceptance criteria

All original CP7 release criteria still apply. In addition:

- another browser cannot redeem or link any OAuth transaction it did not start;
- logout state changes require a POST with exactly one valid `Origin` matching
  the configured `BASE_URL` origin;
- no deletion sink can remove a recipe object retained by an authorized
  reference, including inherited cross-tenant forks;
- no hidden graph or attribution response exposes a stable direct-access
  locator or private metadata;
- graph and history work is demonstrably page-, byte-, and concurrency-bounded;
- no CP7A–CP7D reviewer has an unresolved blocking or should-fix finding;
- the exact seven CP7 findings have regression tests and end-to-end evidence;
- remaining limitations are operational/specialized and do not provide a
  straightforward cross-user compromise or feasible single-actor service
  exhaustion path.

Produce a new self-contained HTML report under `/tmp` with the candidate
revision, threat model, all attack outcomes, measured resource evidence,
screenshots where browser identity matters, regression evidence, limitations,
and a final `GOOD ENOUGH` or `NOT GOOD ENOUGH` verdict.

**Gate result:** The independent CP8 reviewer returned `NOT GOOD ENOUGH` and
created CP8D and CP8E below. NOMS-015 completion now requires the CP9 re-review;
do not weaken the original CP8 release criteria.

---

### CP8 completion evidence (2026-09-02)

- The review ran against revision `1ec37667` in a fresh detached worktree and
  isolated application, ParadeDB/PostgreSQL, MinIO, and Mock OAuth stack.
- `just ci` passed 699 tests with 10 ignored. All nine ignored ParadeDB search
  integration tests passed separately against the live pinned database.
- The production image built successfully, ran as the unprivileged `noms` user,
  exposed no world-writable application files, and failed closed when required
  production configuration was absent.
- Focused runtime checks passed for ordinary Mock OAuth login, missing and
  mismatched browser bindings, callback replay, signed-in login rejection,
  cross-origin logout rejection, and exact-origin logout.
- The canonical scan recorded two medium- and seven low-severity findings. The
  two medium findings below remain release blocking; the final CP8 verdict was
  `NOT GOOD ENOUGH`.
- The self-contained review report is `/tmp/noms-cp8-security-review.html`; the
  canonical scan ID is `227e6e84-2f54-4f8a-aa17-22c8a2e95514`.

---

## Checkpoint 8D: Fail closed when media-reference ancestry is truncated

**Goal:** A bounded ancestry search must never turn incomplete knowledge into
authority to delete recipe media that a surviving fork still references.

**Finding addressed:** Depth-limited fork ancestry can authorize deletion of
still-referenced media.

**Depends on:** CP8B and the CP8 candidate at revision `1ec3766`. May proceed
independently of CP8E. Must land before CP9.

**Primary files:** `src/db/mod.rs`, `src/db/versioning.rs`,
`src/db/media_lifecycle_tests.rs`, `src/worker/orphan_cleanup.rs`, and focused
media-retention integration tests.

### Tests first

1. Build a 33-plus-generation cross-owner fork chain, leave only the deepest
   exact holder live, and prove the original media family cannot be claimed or
   deleted.
2. Repeat the deep-chain case after the original recipe becomes private and
   after its source row is deleted; the surviving fork must continue rendering
   the media.
3. Exercise the exact depth boundary on both sides of the configured cap and
   prove a truncated result is distinguishable from a complete negative result.
4. Prove the scheduled worker and any direct lifecycle claim use the same
   fail-closed decision.
5. Keep a shallow, genuinely unreferenced media family reclaimable so the fix
   does not convert all bounded ancestry checks into permanent retention.
6. Preserve the existing fail-closed behavior when the holder-count cap,
   database timeout, malformed lineage, or another safety limit prevents a
   complete proof.

### Implementation tasks

- Change the retained-reference query contract to report whether the ancestry
  proof was complete. Reaching the recursion, holder, row, timeout, or decoding
  limit is `incomplete`, not `unreferenced`.
- Select the conservative rollout: retain the media family whenever the proof
  is incomplete. Do not silently increase or remove the recursion cap merely to
  make the test pass.
- Keep the complete negative path capable of granting a cleanup claim only
  after every exact holder has been checked against the original owner or a
  server-established inherited-authority relationship.
- Centralize the result so orphan cleanup, account-deletion cleanup, and any
  other recipe-media claim path cannot interpret truncation differently.
- Record aggregate, non-sensitive metrics for conservative retention caused by
  depth, holder, row, and timeout limits. Do not record recipe, owner, media, or
  lineage identifiers.
- Document durable reference-authority persistence as a possible later
  optimization, not as a prerequisite for the conservative correctness fix.

### Acceptance criteria

- A fork chain of any permitted depth cannot cause a still-rendered media object
  to be claimed for deletion merely because its authority lies beyond a search
  cap.
- Every incomplete safety proof retains the object and remains observable
  through aggregate metrics.
- A complete proof that no authorized reference remains still allows eventual
  reclamation.
- Existing shallow fork, draft, immutable-version, collection projection,
  account deletion, and ordinary orphan-cleanup behavior remains intact.

### Validation and review gate

Run the focused media lifecycle and cleanup suites, the 32/33-plus-generation
database integration cases, account-deletion cleanup tests, and `just ci`.
Manually verify that the deepest surviving fork still renders the image after
the source is deleted and cleanup runs.

**Gate:** The reviewer returns `PASS` only when a complete ancestry proof is
required before deletion authority is granted and every truncated/error path
retains conservatively.

**Risk:** High. A false negative can permanently break another tenant's live
content; an overly broad fail-closed result can retain unused objects until a
later complete scan.

**Implementation:** Completed at revision `5bc2939` with explicit incomplete
lineage states that retain conservatively at the holder, depth, cycle,
mismatch, and missing-authority boundaries.

---

## Checkpoint 8E: Bound collection recipe projections and anonymous read work

**Goal:** Collection pages return only the lightweight recipe-card data they
render, with endpoint-specific page, byte, rate, timeout, and shared concurrency
bounds.

**Finding addressed:** Public collection pages can materialize roughly 100 MiB
of recipe JSON per request.

**Depends on:** CP8C and the CP8 candidate at revision `1ec3766`. May proceed
independently of CP8D. Must land before CP9.

**Primary files:** `src/types.rs`, `src/security.rs`, `src/api/collection.rs`,
`src/db/mod.rs`, `src/pages/collection_detail.rs`, recipe-card components, and
collection authorization/resource-bound tests.

### Tests first

1. Publish a collection containing 100 maximum-size valid recipes and prove a
   page response omits ingredients, instructions, equipment, commentary, and
   every other full-snapshot field not rendered by a recipe card.
2. Assert the server independently caps each collection recipe page at the
   endpoint's existing 12-card display size even when a client requests 100 or
   supplies a forged larger value.
3. Measure selected rows, serialized bytes, duration, and peak decoded data for
   maximum-size fixtures; each request must remain under documented aggregate
   ceilings derived from the lightweight projection rather than the 1 MiB
   recipe snapshot limit.
4. Prove public anonymous and identifier-known unlisted collection reads acquire
   the same shared expensive-read permit and actor/client rate budget without
   changing visibility filtering or existence-hiding behavior.
5. Exercise concurrent requests above the shared limit and prove excess work is
   rejected or queued within the documented database execution budget without
   exhausting the connection pool.
6. Preserve stable pagination, owner-only removal/reordering, recipe-card
   rendering, visibility intersections, and loading every permitted membership
   over multiple bounded pages.

### Implementation tasks

- Introduce a serialized collection recipe-card projection containing only the
  fields the card and owner controls require: stable recipe identity, title,
  bounded description, timing/servings metadata, one display image with bounded
  edit metadata, visibility as required by owner controls, and bounded author
  display data.
- Define endpoint-specific constants of 12 cards, 500 Unicode scalar values of
  description per card, and 512 KiB of serialized card data per response. Full
  recipe detail remains available only through the individually authorized
  recipe-detail endpoint.
- Replace both owned and public collection membership queries that construct
  complete `Recipe` values with explicit projection queries. Do not fetch or
  decode ingredients, instructions, equipment, commentary, or complete image
  arrays and discard them after allocation.
- Enforce the 12-card endpoint limit inside the server/database boundary rather
  than trusting the client-provided limit. Preserve deterministic keyset or
  otherwise bounded pagination and `has_more` semantics.
- Enforce the 512 KiB serialized output ceiling with checked accounting. A
  ceiling breach must fail safely without returning a partial ambiguous page.
- Wrap collection recipe reads in the shared expensive-read concurrency budget
  and an anonymous-aware rate budget keyed by the existing trusted client
  identity contract. Keep the five-second database budget as a backstop, not
  the primary memory-control mechanism.
- Reuse the lightweight projection for the initial collection contents and
  subsequent pages so no alternate route retains the full-recipe amplification
  path.

### Acceptance criteria

- No collection list/read endpoint selects, decodes, or serializes full recipe
  snapshots merely to render cards.
- One request returns at most 12 bounded card projections, no description over
  500 Unicode scalar values, and no more than 512 KiB of serialized card data,
  regardless of valid recipe snapshot size.
- Repeated anonymous reads are constrained by both per-client rate policy and
  the shared expensive-read concurrency budget.
- Public, unlisted, private, owner, membership mutation, pagination, and reorder
  behavior remains functionally correct.

### Validation and review gate

Run focused collection database/API/UI tests, maximum-fixture byte and row
instrumentation tests, concurrent anonymous-read tests, visibility matrices,
and `just ci`. In a fresh isolated browser environment, load and paginate a
large public collection and verify cards, loading states, removal, and owner
reordering remain functional.

**Gate:** The reviewer returns `PASS` only with measured evidence that every
collection page performs work proportional to at most 12 lightweight cards and
that concurrent anonymous reads share the intended rate and concurrency limits.

**Risk:** Medium. Projection or cursor mistakes can omit card fields, duplicate
or skip memberships, break reordering, or accidentally widen recipe visibility.

**Implementation:** Completed at revision `e9527f8` with a 12-card database
projection, aggregate response validation, and shared anonymous-aware read
budgets.

---

## Checkpoint 8F: Replace cleanup-time media liveness inference with explicit durable references

**Goal:** Make managed-media lifetime depend on indexed, transactionally
maintained asset references rather than cleanup-time URL scans and fork-lineage
reconstruction.

**Architectural decision:** The application is pre-production and has no user
data that must survive this change. Implement a clean schema and model cutover;
do not add legacy backfills, dual writes, compatibility reads, or rollout flags.
Development databases and object-storage fixtures may be reset.

**Depends on:** The integrated CP8D and CP8E candidate. Must land before CP9.

**Primary files:** `migrations/schema.sql`, managed-media types and upload APIs,
`src/db/mod.rs`, `src/db/diff.rs`, `src/db/versioning.rs`, recipe/draft/fork,
collection-cover and avatar persistence, `src/worker/orphan_cleanup.rs`, SQLx
metadata, and focused media lifecycle integration tests.

### Tests first

1. Prove every successful recipe or collection-cover upload creates one typed
   media asset with its original, full, and thumbnail variants as applicable.
   The uncommitted asset has no durable holder reference but remains attachable
   until its upload-attempt expiry. An avatar flow may instead attach its asset
   and avatar reference atomically.
2. Prove recipe, recipe-version, draft, collection-cover, and avatar saves add
   and remove explicit asset references in the same transaction as the holder
   mutation, including both top-level recipe images and nested step photos; a
   failed mutation must leave neither stale content nor stale references.
3. Fork a recipe across owners and prove the server-authorized fork creates a
   durable reference to each inherited asset. After the source becomes private
   or is deleted, the fork owner can edit a draft, publish an immutable version,
   and restore or re-save the fork with those assets. That inherited authority
   must not permit attaching the assets to an unrelated recipe.
4. Prove an arbitrary user cannot pin another user's managed object by copying
   its URL or submitting its asset ID outside an authorized fork or explicit
   sharing operation.
5. Keep immutable versions and restorable drafts visually complete for as long
   as they exist, then prove deleting their final holder removes their asset
   references.
6. Resolve asset IDs safely and within existing response bounds on every read
   surface: recipe detail/editor, draft editor, immutable version/history/graph,
   collection cards/mosaics, search projections, managed avatars, and managed
   collection covers. Derived projections consume references but do not become
   durable holders themselves.
7. Race final-reference removal and a new authorized attachment. Both paths
   must lock the asset consistently so cleanup cannot delete an object after a
   new reference commits.
8. Prove an asset with any reference cannot be claimed. An uncommitted asset
   cannot be swept before its attempt expires; before then only its exact
   one-use attempt-bound discard capability may claim it. After expiry, or
   after the final durable reference is removed, cleanup can claim the asset
   exactly once and eventually delete every variant with retry-safe state.
9. Delete an uploader account in three cases: while a foreign authorized
   reference survives, while only the uploader's own holder survives, and while
   the asset is uncommitted. Holder deletion removes its references
   transactionally, the asset row survives without the user row, every
   surviving foreign reference remains renderable, and every resulting
   unreferenced asset remains reclaimable by ordinary cleanup.
10. Preserve explicitly external recipe images and OAuth/external avatars
    without treating them as managed asset references or allowing them to
    influence managed-object retention.
11. Prove cleanup work uses indexed exact asset references and never scans JSON
    documents or traverses fork lineage to decide liveness.

### Implementation tasks

- Replace user-owned `media_upload_families` as the lifetime authority with a
  first-class media-asset record. Keep uploader/custodian attribution for
  authorization, quotas, abuse response, and audit, with a nullable uploader
  foreign key that uses `ON DELETE SET NULL` so the asset survives account
  deletion.
- Represent original, full, and thumbnail objects as typed variants of one
  asset. An object URL must never be the authoritative asset identity.
- Store managed asset IDs in canonical recipe/draft/version media values and
  resolve their display URLs through bounded server projections. Keep external
  URLs, including OAuth avatar URLs, explicitly tagged and distinct from
  managed assets.
- Add strongly constrained references for every durable holder. Prefer
  holder-specific reference tables with foreign keys for recipes, immutable
  versions, drafts, collection covers, and avatars over an unenforced
  polymorphic holder ID. Extract both top-level recipe images and nested step
  photos into those references. Collection cards, mosaics, search results, and
  other derived projections resolve asset IDs as consumers; they are not
  additional durable holders.
- Create and delete references transactionally with holder writes. Lock the
  media asset before attaching it, and reject attachment when its lifecycle is
  already deletion-claimed or deleted.
- Keep each uncommitted recipe or collection-cover asset attachable until its
  upload-attempt expiry. Before expiry, only the exact one-use attempt-bound
  discard capability may claim it; the cleanup worker must lock and recheck
  both references and expiry before claiming it.
- During a server-authorized fork, copy asset references while source access is
  still authorized. Carry that scoped authority through the fork's drafts,
  immutable versions, and restore/re-save operations, but do not turn it into
  permission to attach the asset to unrelated recipes. Retention then depends
  on durable references, while fork lineage remains responsible only for
  attribution and version-graph history.
- Replace cleanup-time recipe JSONPath scans and recursive lineage proofs with
  an indexed `NOT EXISTS` reference check performed while the asset row is
  locked. Recheck liveness before transitioning to deletion-claimed.
- Preserve the existing claim/delete/finish lifecycle around object storage;
  database references remove lineage ambiguity but do not make database and
  object-storage operations atomic.
- Use this single asset lifecycle for account deletion. Remove managed-media
  `account_media_cleanup_tombstones` and any parallel cleanup authority: holder
  deletion removes its references transactionally, the uploader foreign key is
  cleared, a surviving reference outlives the uploader, and an unreferenced
  service-custodied asset remains reclaimable without the user row.
- Remove superseded lineage-retention code and compatibility-only schema after
  the new tests pass. Do not retain a fallback URL/lineage cleanup path.

### Acceptance criteria

- Every managed object variant belongs to exactly one typed media asset, and
  every durable use is represented by an explicit database reference.
- Cleanup may claim an asset only after a locked, indexed check proves that no
  reference exists; reference creation and cleanup claims serialize on the
  same asset row.
- A legitimate cross-owner fork keeps inherited assets alive without any
  cleanup-time lineage traversal, while arbitrary foreign URLs or asset IDs
  cannot create retention authority.
- Recipe versions, drafts, collection covers, and avatars retain their assets
  for their documented lifetime, and removal of the final holder permits
  eventual reclamation.
- Uncommitted uploads remain attachable until expiry and cannot be claimed
  early except by their exact one-use attempt-bound discard capability.
- Account deletion neither breaks surviving foreign references nor removes the
  system's authority to reclaim the asset later; managed-media account
  tombstones and parallel deletion authority no longer exist.
- Fork lineage remains available for attribution and graph history but is not
  consulted for media liveness.
- The clean development cutover contains no backfill, compatibility read, dual
  write, or legacy fallback path.

### Validation and review gate

Run focused media lifecycle, upload, recipe/draft/version/fork,
collection-cover, avatar, account-deletion, race, and cleanup-worker suites;
then run `just migrate-plan`, `just sqlx-prepare`, `just schema-check-test`, and
`just ci`. The destructive schema plan is acceptable only for disposable
pre-production databases and must be recorded explicitly.

In a fresh isolated environment created from an empty database and bucket,
exercise upload, recipe save, fork, source deletion, account deletion, final
reference removal, and cleanup. Verify the object remains renderable until its
last authorized reference disappears and that every variant is removed only
afterward.

**Gate:** The reviewer returns `PASS` only when cleanup authorization derives
solely from explicit durable references, attachment and cleanup are race-safe,
and no legacy URL/lineage inference remains reachable.

**Risk:** High. This intentionally changes the canonical media identity and
every persistence path that can hold media. The lack of production data removes
backfill and compatibility risk, but incomplete reference maintenance could
still cause premature deletion or indefinite retention.

**Implementation:** Completed at revisions `694c349`, `7894f10`, and `26cd8c7`.
Managed object families now have first-class asset rows and holder-specific
recipe, version, draft, collection, and avatar references. Holder writes and
reference synchronization are transactional; attachment and cleanup serialize
on the asset row; cleanup uses only the indexed reference tables and stored
object keys. Registry-backed projections resolve managed media on ordinary
reads and client-facing mutation responses, while missing managed references
fail closed and explicitly external images and identity-provider avatars remain
readable.

The intentional disposable-database cutover added six schema objects, modified
four, and dropped the superseded upload-family and account-cleanup-tombstone
objects. SQLx metadata generation, the fresh-schema migration plan, schema-plan
tests, and `just ci` passed. Final CI reported 682 passing tests and 10 documented
integration-test ignores. A fresh empty database and bucket passed browser smoke
coverage for OAuth login, draft save, publication, detail projection, visibility
mutation, and search. The independent implementation review returned `PASS`
with no actionable findings.

---

## Checkpoint 9: Final integrated adversarial re-review and release verdict

**Goal:** Repeat the integrated release review against the exact candidate after
CP8D, CP8E, and the clean CP8F media-reference cutover without weakening any CP7
or CP8 acceptance criterion.

**Depends on:** CP8D, CP8E, and CP8F.

### Required gate

- Repeat the complete CP8 validation matrix and production image/runtime
  inspection against a fresh isolated environment.
- Reproduce the former 33-plus-generation retained-media exploit and prove an
  explicit surviving reference keeps the asset alive regardless of lineage
  depth; confirm that no deletion or cleanup authorization decision consults
  fork ancestry.
- Measure maximum-size public collection pages and concurrent anonymous reads,
  recording selected columns, rows, serialized bytes, duration, permits, and
  rate-limit outcomes.
- Verify every managed media holder creates an explicit reference, cleanup uses
  only the indexed reference proof, and a surviving cross-owner fork remains
  renderable without cleanup-time lineage traversal.
- Confirm the candidate starts from an empty schema without any CP8F legacy
  backfill, dual-write, or compatibility path.
- Re-run the seven low-severity CP8 findings to ensure neither remediation
  increases their reachability or impact; promote any aggregate or changed risk
  to an explicit checkpoint.
- Obtain an independent security review of the complete candidate and issue a
  new self-contained report with a `GOOD ENOUGH` or `NOT GOOD ENOUGH` verdict.

**Gate:** NOMS-015 is complete only if the independent CP9 reviewer returns
`GOOD ENOUGH` with no unresolved blocking or should-fix findings.

---

## Rollout and rollback

### Deployment order

1. Apply additive schema changes.
2. Deploy code that understands both the old and new schema where compatibility is required.
3. Enable explicit OAuth purposes.
4. Enable storage ownership validation and safe deletion.
5. Enable corrected client-IP extraction and observe rate-limit metrics.
6. Pause collection/recipe orphan cleanup, atomically apply the selected legacy-cover database policy and typed `collections/` upload cutover, verify every surviving cover has a recognized retention path, then re-enable strict dual-namespace cleanup.
7. Deploy the unified internal upload transaction; no media-object or schema migration is required because canonical key formats remain unchanged.
8. Execute the selected stable-identity rollout policy, including the approved no-compatibility path where applicable.
9. Deploy the browser-bound OAuth transaction schema and code together after clearing or expiring all pre-deployment auth-state rows; those rows are ephemeral and must not receive a compatibility bypass.
10. Deploy POST-only logout and the exact configured-Origin client/server change in the same release so users are not left without a working logout action.
11. Deploy the attempt-bound discard capability and reference-aware recipe-media deletion together; remove the generic URL endpoint only when the replacement client flow is active, and retain orphan cleanup as fallback reclamation rather than the normal immediate-discard path.
12. Deploy the opaque graph/attribution contract before the bounded graph/history query contract, then invalidate incompatible client caches or generated bundles.
13. Apply any CP7D summary-projection schema/migration policy before switching reads; do not enable an unbounded legacy fallback.
14. Record the CP8 `NOT GOOD ENOUGH` verdict and preserve its exact candidate and report artifacts as review evidence.
15. Deploy CP8D's fail-closed ancestry result before running any recipe-media cleanup worker built from the new candidate.
16. Deploy CP8E's collection card projection, endpoint page limit, and anonymous read budgets together so no public route retains the full-recipe fallback.
17. Before CP9, reset disposable development databases and buckets and deploy CP8F's media-asset schema, explicit references, and code as one clean cutover. Do not carry the superseded URL/lineage retention path into the candidate.
18. Complete CP9 against the exact remediated production candidate before general release.

### Monitoring

Track structured counts without sensitive values:

- OAuth state-purpose mismatches;
- OAuth browser-transaction binding mismatches by non-sensitive category;
- rejected GET logout and rejected POST logout attempts by missing, duplicate, `null`, malformed, or mismatched Origin category;
- rejected link callbacks by mismatch category;
- provider-subject migration successes/conflicts;
- rejected foreign storage references;
- rejected media-kind/namespace mismatches and orphan cleanup outcomes per managed namespace;
- deletion attempts blocked by retained live, draft, version, fork, or collection references;
- media-retention proofs that fail closed by depth, holder, row, or timeout category;
- skipped unsafe legacy deletions;
- OAuth rate-limit responses by hashed/anonymized client key;
- mutation-limit responses;
- graph/history rows, decoded bytes, duration, concurrency rejections, and stale/invalid cursor rejections;
- collection recipe-card rows and serialized bytes, plus anonymous rate and shared-concurrency rejections;
- redacted graph/attribution responses by visibility category, without recipe IDs;
- session verification and revocation failures at aggregate level.

### Rollback constraints

- Do not roll back to code that treats a callback cookie as authorization to link.
- Do not roll back to OAuth callbacks that accept a state without the matching initiating-browser transaction binding.
- Do not restore state-changing GET logout.
- Do not roll back to unconditional URL-derived storage deletion.
- Do not roll back to namespace-only deletion that ignores retained references.
- Do not treat a truncated or failed media ancestry search as a complete negative reference proof.
- Do not restore stable recipe UUIDs in inaccessible graph nodes, edges, frontiers, or attribution.
- Do not restore complete-graph or full-history materialization as a compatibility fallback.
- Do not restore full-recipe collection membership responses or a client-selectable 100-recipe collection page.
- Additive schema may remain during rollback.
- If rate-limit client identity fails in production, disable only the faulty limiter through a controlled configuration while preserving OAuth state/PKCE checks; do not restore the global fallback bucket.
- Preserve audit evidence and rejected-event metrics during rollback analysis.

---

## Definition of done

- [ ] CP0 baseline is deterministic and `just ci` passes.
- [ ] OAuth login and linking have explicit purposes and complete state/session binding.
- [ ] The login-CSRF account-takeover regression test passes through the real callback handler.
- [ ] Every OAuth callback is bound to the initiating browser transaction; a transferred callback cannot create or link authority even after forced logout.
- [ ] Logout revocation is POST-only and rejects missing, duplicate, `null`, malformed, or mismatched Origin headers; only the exact configured `BASE_URL` origin may revoke a session.
- [ ] OAuth and logout redirects remain same-origin after browser-compatible parsing and normalization, including raw and encoded slash/backslash confusion cases.
- [ ] The production-composed Dioxus router applies authentication middleware to intended routes while public/unlisted recipe history remains governed by application authorization rather than a blanket login redirect.
- [ ] All media persistence and deletion paths enforce exact tenant ownership.
- [ ] New custom collection covers use dedicated collection object kinds, upload handling, and the `collections/` namespace through the shared typed parser; exact pairing, owner-aware retention, unsaved-upload deletion, and orphan reclamation are verified.
- [ ] Recipe, collection-cover, and avatar uploads use one typed internal transaction with shared quota locking, exact variant accounting, attempt-scoped compensation, and purpose-specific authorization; generic upload callers cannot acquire avatar persistence authority.
- [ ] Cross-tenant image and avatar deletion tests prove no victim storage request occurs.
- [ ] Recipe-media deletion proves both exact namespace authority and absence of retained live, draft, version, fork, or collection references at a race-safe boundary; immediate abandoned-upload cleanup uses a one-use attempt-bound capability and lifecycle transition rather than a caller-supplied URL.
- [ ] The production serving path attaches trustworthy connection information.
- [ ] OAuth rate limits are independent per client and cannot be globally exhausted by one attacker.
- [ ] New OAuth accounts use stable provider subjects and verified-email semantics.
- [ ] Existing identity handling follows an explicitly approved migration or no-compatibility policy.
- [ ] Current live recipe visibility governs version lists, snapshots, comparisons, lineage, and graph access consistently; drafts remain owner-only and inaccessible branches reveal at most redacted topology.
- [ ] Hidden graph nodes, edges, frontiers, links, and attribution expose no stable direct-access recipe locator or private source metadata; URL-known unlisted access remains intentional and functional.
- [ ] Collection hierarchy, membership, sharing, cover media, and search projections preserve both collection ownership and member-recipe visibility.
- [ ] Search results, grouped sections, counts, facets, and autocomplete cannot reveal another user's private/unlisted entities.
- [ ] Graph expansion state, collection hierarchy/reorders, active drafts, immutable-version growth, and search/autocomplete workloads have proportionate per-request and sustained-abuse controls.
- [ ] Initial/expansion graph queries and version-summary reads are page-, aggregate-byte-, and concurrency-bounded with measured evidence; they never construct the complete 1,000-node graph or decode all 250 snapshots before returning a page.
- [ ] Public queries, structured recipe inputs, uploads, and mutations have proportionate server-side bounds.
- [x] Dependency audit and production security configuration are enforced in CI/deployment.
- [x] CP7 independent integrated security review completed and returned `NOT GOOD ENOUGH`; all seven findings have explicit remediation checkpoints.
- [x] CP8 independent integrated security review completed at revision `1ec3766` and returned `NOT GOOD ENOUGH` with two medium- and seven low-severity findings.
- [x] Incomplete or depth-truncated fork ancestry retains recipe media; only a complete negative authority proof permits cleanup.
- [x] Collection membership reads return at most 12 lightweight card projections under aggregate byte, anonymous rate, database timeout, and shared concurrency bounds.
- [x] Managed media uses first-class asset identity and explicit durable references for every holder; cleanup relies on a locked indexed no-reference proof rather than URL scans or fork-lineage traversal.
- [ ] CP9 independent integrated security re-review of the CP8D/CP8E/CP8F candidate returns `GOOD ENOUGH`.
- [ ] Remaining specialized risks, exclusions, owners, and expiry dates are documented.
