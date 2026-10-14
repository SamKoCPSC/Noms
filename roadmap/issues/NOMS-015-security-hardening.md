# NOMS-015: Security Hardening to a Good-Enough Baseline

**Status:** 🔵 Ready
**Phase:** Security
**Created:** 2026-07-29
**Depends on:** NOMS-004, NOMS-006, NOMS-008, NOMS-009, NOMS-014
**Implementation plan:** [NOMS-015-security-hardening.md](../implementation-plans/NOMS-015-security-hardening.md)

## Objective

Move the application from the 2026-07-28 security review verdict of **not good enough** to **good enough for general consumer usage** by closing the confirmed cross-user attack paths and proving the fixes through adversarial regression tests.

## Confirmed release blockers

1. An unauthenticated OAuth login state can become an account-link operation when its callback is opened with a victim's valid session, allowing account takeover.
2. A user can persist another user's public media URL in their own recipe and cause the underlying victim object to be deleted during recipe cleanup.
3. The shipped OAuth rate limiter does not receive connection metadata, so all clients share the `0.0.0.0` fallback bucket and one attacker can continuously block login for everyone.

## Specialized identity risk

Google and GitHub account records currently prefer email over the provider's stable subject. Email changes or reassignment can cause account duplication or, in organization-managed/recyclable-email environments, account takeover. New identities must use stable provider subjects; existing records require an explicit migration policy.

## Acceptance criteria

### AC1: OAuth login and linking are distinct

- OAuth state stores explicit `login` or `link` purpose.
- Link state is created only for an authenticated user and is bound to that user.
- Link callback requires the same current user.
- Login callback cannot become linking behavior because a session cookie is present.
- The exact cross-browser login-CSRF attack is covered through the real callback handler.

### AC2: Media deletion proves ownership

- Internal media URLs are validated against the authenticated user's exact namespace during recipe creation and update.
- Top-level images and recursively nested step photos use the same validation.
- Every request-reachable storage deletion proves ownership of the exact object key.
- Legacy or foreign URLs never cause storage deletion.
- Cross-user recipe-image and avatar attack tests assert that no victim object is deleted.

### AC3: OAuth rate limiting is per client

- The production server attaches direct peer information.
- Forwarded IP headers are trusted only from configured proxies.
- Missing peer identity does not group the public into one bucket.
- A real TCP integration test proves one client cannot exhaust another client's budget.

### AC4: Provider identity is stable

- Google uses `sub`; GitHub uses stable numeric `id`.
- Provider email verification is captured and enforced where email reconciliation is allowed.
- Newly created accounts are never identified by email.
- Existing email-keyed records follow a documented, conflict-safe migration.

### AC5: Public inputs are proportionately bounded

- Pagination, structured recipe collections/depth, persisted text, images, uploads, and mutations have documented server-side limits.
- Boundary tests prove ordinary use works and excessive input fails without partial writes.

### AC6: Integrated security gate passes

- `just ci` passes.
- A current dependency audit has no reachable unmitigated critical or high advisory.
- Independent review replays the original attacks against a fresh isolated instance.
- Login, linking, recipe CRUD/sharing, uploads, settings, logout, and account deletion have no regressions.
- The reviewer returns an explicit `GOOD ENOUGH` verdict.

## Out of scope

- Enterprise roles or permissions;
- two-factor authentication;
- device fingerprinting;
- CAPTCHA;
- IP-bound sessions;
- comprehensive compliance certification;
- a general-purpose security operations platform.

These may be valuable later but are not required to close the confirmed general-usage risks.
