# NOMS-015: Security Hardening to a Good-Enough Baseline — Agentic Implementation Plan

**Status:** Draft — ready for implementation
**Created:** 2026-07-29
**Issue:** [NOMS-015-security-hardening.md](../issues/NOMS-015-security-hardening.md)
**Source:** Authentication, authorization, and application security review completed 2026-07-28
**Depends on:** NOMS-004 OAuth authentication, NOMS-006 auth security hardening, NOMS-008 recipe CRUD, NOMS-009 image uploads, NOMS-014 isolated development environments
**Approach:** Write attack-regression tests first, remove the confirmed account-takeover and cross-tenant deletion paths, restore effective OAuth rate limiting, migrate provider identities toward stable subjects, add bounded abuse controls, and require an independent adversarial review before release.

---

## Outcome and release verdict

This plan is complete only when the application can be reassessed as **good enough for general consumer usage**.

The following are release blockers:

1. An OAuth login flow started without a session can become an account-link flow when its callback is opened by a signed-in victim.
2. A user can cause another user's recipe or avatar object to be deleted by persisting that public object URL in their own recipe and deleting the recipe.
3. OAuth rate limiting groups all production requests under the fallback `0.0.0.0` address because the server does not attach Axum `ConnectInfo`.

The stable-provider-identity work is also required before declaring completion for deployments that support organization-managed, changeable, or recyclable email addresses. If migration constraints prevent that work from shipping with the three blockers, the release decision must explicitly exclude those environments and track the remaining risk.

Passing ordinary happy-path tests is not sufficient. Completion requires tests that reproduce each attack and prove that it is rejected without damaging the victim's session, linked accounts, database records, or storage objects.

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

- Recipe media uploaded by user A belongs to the exact `recipes/{user-A-id}/` namespace.
- Avatar media belongs to the exact `avatars/{user-A-id}/` namespace.
- Only server-derived, ownership-validated object keys may be sent to `delete_object`.
- An external image URL may be displayed or referenced if product behavior permits it, but the application must never translate an external URL into a storage deletion.
- An internal storage URL belonging to another user must be rejected when a recipe is created or updated.
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
security-abuse       CP5 after CP1–CP3 interfaces settle
security-release     CP6–CP7 integration and review
```

CP2 and CP3 may run in parallel after CP0. CP1 must land before CP4 because both alter OAuth state, user-info extraction, account linking, schema, and tests. CP5 starts after the blocking interfaces stabilize. CP6 and CP7 run on the integrated branch only.

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

Use a mock S3 endpoint that records exact delete requests. Asserting only on a helper's returned string is insufficient for the cross-tenant deletion regression.

### Implementation tasks

- Centralize parsing of public storage URLs into a typed, validated object reference.
- Compare URL origin/base exactly; do not use a loose textual prefix that accepts a sibling host or path.
- Validate the namespace segment as an exact UUID equality with the authenticated user.
- Restrict recipe objects to the expected filename forms generated by the server.
- Traverse all nested recipe steps with one shared validator.
- Validate internal media references during both recipe creation and update.
- Before deletion, validate every stored reference again and skip/log legacy or foreign references.
- Keep database ownership checks on the recipe itself.
- Prefer persisting a server-issued object key or opaque upload ID in addition to display URLs so future deletion does not derive authority from client-controlled URLs.
- Ensure orphan cleanup cannot treat a cross-user reference as authorization to retain or delete another tenant's object.

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

## Checkpoint 5: Bound realistic authenticated and unauthenticated abuse

**Goal:** Prevent inexpensive resource exhaustion without inventing enterprise-grade controls.

**Primary files:** recipe server functions and validation helpers, upload/avatar handlers, pagination functions, rate-limit middleware/configuration.

### Scope

Implement proportionate controls for the application's actual public surface:

- clamp public and authenticated pagination to a documented range such as `1..=100`;
- reject negative offsets;
- bound recipe title, description, commentary, tag count/length, ingredient count, instruction count/depth, equipment count, and image count;
- retain upload byte limits and ensure empty image variants are rejected;
- limit mutation and upload request rates per authenticated user, with IP fallback only for unauthenticated endpoints;
- ensure limits are applied server-side and return deterministic client errors;
- keep the limits high enough for ordinary recipe usage and document them in one place.

Do not introduce CAPTCHA, device fingerprinting, 2FA, enterprise audit infrastructure, or a new authorization-role system as part of this checkpoint.

### Tests first

- boundary and one-over-boundary tests for every limit;
- deeply nested step payload terminates predictably;
- oversized requests do not write partial database rows or storage objects;
- one user exhausting a quota does not block another;
- extreme pagination cannot force an unbounded public query;
- normal large recipes and avatar/recipe uploads continue to work.

### Validation

```bash
just fmt
just check
just test
just lint
```

**Gate:** Every attacker-controlled collection, recursion depth, pagination value, upload, and persisted large text field has a documented server-side bound.

**Risk:** Medium. Limits are security controls but also product behavior; choose values from real usage rather than arbitrary tiny thresholds.

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
- pagination and structured recipe limit tests.

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

### Reviewer questions

- Can any callback attach an identity without an explicit link state?
- Is every link state bound to the same live user at start and callback?
- Can any client-controlled URL become a storage deletion key?
- Can malformed URL encoding or prefix confusion escape a namespace?
- Does the deployed server, not just the unit-test router, supply trustworthy client identity?
- Can provider email change or reassignment alter account ownership?
- Are all authorization decisions repeated server-side at the data mutation boundary?
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
- CP4 is complete, or the deployment explicitly excludes recyclable-email environments with a time-bounded migration plan;
- `just ci` passes;
- no reachable unmitigated critical or high dependency advisory remains;
- the exact attacks from the review fail safely;
- ordinary login, linking, recipe CRUD, public sharing, uploads, profile settings, logout, and account deletion still work;
- no security control relies only on client UI state;
- residual risks are operational or specialized rather than straightforward cross-user compromise.

---

## Rollout and rollback

### Deployment order

1. Apply additive schema changes.
2. Deploy code that understands both the old and new schema where compatibility is required.
3. Enable explicit OAuth purposes.
4. Enable storage ownership validation and safe deletion.
5. Enable corrected client-IP extraction and observe rate-limit metrics.
6. Execute the selected stable-identity migration.
7. Remove temporary legacy compatibility only after evidence shows it is unused.

### Monitoring

Track structured counts without sensitive values:

- OAuth state-purpose mismatches;
- rejected link callbacks by mismatch category;
- provider-subject migration successes/conflicts;
- rejected foreign storage references;
- skipped unsafe legacy deletions;
- OAuth rate-limit responses by hashed/anonymized client key;
- mutation-limit responses;
- session verification and revocation failures at aggregate level.

### Rollback constraints

- Do not roll back to code that treats a callback cookie as authorization to link.
- Do not roll back to unconditional URL-derived storage deletion.
- Additive schema may remain during rollback.
- If rate-limit client identity fails in production, disable only the faulty limiter through a controlled configuration while preserving OAuth state/PKCE checks; do not restore the global fallback bucket.
- Preserve audit evidence and rejected-event metrics during rollback analysis.

---

## Definition of done

- [ ] CP0 baseline is deterministic and `just ci` passes.
- [ ] OAuth login and linking have explicit purposes and complete state/session binding.
- [ ] The login-CSRF account-takeover regression test passes through the real callback handler.
- [ ] OAuth and logout redirects remain same-origin after browser-compatible parsing and normalization, including raw and encoded slash/backslash confusion cases.
- [ ] All media persistence and deletion paths enforce exact tenant ownership.
- [ ] Cross-tenant image and avatar deletion tests prove no victim storage request occurs.
- [ ] The production serving path attaches trustworthy connection information.
- [ ] OAuth rate limits are independent per client and cannot be globally exhausted by one attacker.
- [ ] New OAuth accounts use stable provider subjects and verified-email semantics.
- [ ] Existing identity migration follows an explicitly approved policy.
- [ ] Public queries, structured recipe inputs, uploads, and mutations have proportionate server-side bounds.
- [ ] Dependency audit and production security configuration are enforced in CI/deployment.
- [ ] Independent integrated security review returns `GOOD ENOUGH`.
- [ ] Remaining specialized risks, exclusions, owners, and expiry dates are documented.
