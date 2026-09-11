# NOMS-021: Production Readiness and Reliability Follow-ups

**Status:** 🔵 Ready
**Phase:** Production hardening
**Created:** 2026-09-08
**Depends on:** NOMS-015

## Objective

Resolve the realistic reliability, resource-bounding, and user-facing defects
identified during the final NOMS-015 code review without extending the scope of
the completed OAuth, authorization, media-ownership, or visibility hardening.

NOMS-015 may be reviewed and merged independently because no critical
authorization bypass or data-loss path was found. Findings 1–4 below remain a
release gate for public production deployment; findings 5–12 are lower-priority
correctness, resilience, and maintainability follow-ups.

## Findings and enhancement scope

1. Dashboard recipe and draft lists return complete snapshots where lightweight
   card projections are sufficient. At current quotas, one draft-list response
   can approach 50 MiB, and the owner recipe-list API accepts up to 100 complete
   recipes.
2. Advanced collection and user search validate individual criteria but do not
   enforce the aggregate 64-term ceiling used by recipe search. Accepted input
   can therefore expand into thousands of ParadeDB predicates.
3. Media cleanup always prioritizes `delete_claimed` rows. Five hundred
   persistently failing deletions can occupy every batch indefinitely and starve
   expired uploads and newly unreferenced assets.
4. The production Docker build and the ordinary GitHub test job omit the
   eight-thread test cap required by the repository's PostgreSQL-per-test
   fixtures, making builds prone to resource exhaustion or indefinite stalls.
5. Browser image processing waits only for success callbacks. Decode,
   `FileReader`, or canvas failures can leave an upload permanently stuck in its
   processing state.
6. Fork-attribution and lineage reads do not share the graph-history concurrency
   admission path, while multi-query version endpoints reset the database
   timeout for each sequential operation instead of using one aggregate
   deadline.
7. The ParadeDB-safe exact username expression `(username || '') = $1` avoids
   ambiguous aliases but prevents use of the unique username index, producing a
   sequential scan for public profile identity lookup.
8. Explore tag chips only change selected styling; they do not filter or refetch
   the displayed recipes.
9. `NOMS_ORPHAN_CLEANUP_INTERVAL_SECS=0` is accepted even though Tokio rejects a
   zero interval, causing the detached cleanup task to terminate.
10. As an additional maintainability enhancement, several older modules and
    workflows predate the repository's current readability guidance. Large
    multi-responsibility files, long endpoint and UI
    workflows, repeated boundary logic, and comments that describe mechanics
    instead of invariants make important behavior harder to review safely.
11. Route-specific document metadata is limited to the marketing pages. After
    client-side navigation from Home, About, or How To, application pages retain
    the previous page's title and metadata instead of describing the active
    route.
12. Search is intended to be available to anonymous visitors, but the page is
    wrapped in `AuthRequired` and every search server function requires a user
    session. The page also starts its search resources before rendering the
    authentication prompt, so anonymous navigation both hides Search and emits
    a misleading HTTP 500 for an expected signed-out request.

## Implementation checkpoints

### CP1: Bound dashboard projections

- Add lightweight owner-recipe and draft-card projections containing only the
  fields rendered by the dashboard.
- Paginate draft cards or otherwise keep the complete response within an
  aggregate byte ceiling.
- Apply one actor budget, shared concurrency permit, and aggregate database
  deadline to each dashboard read.
- Preserve draft ordering, recipe pagination, media resolution, and navigation.

### CP2: Complete aggregate search limits

- Count simple and advanced terms across collection and user search requests.
- Reject more than 64 aggregate terms before query construction.
- Cover boundary, exclusion, multi-field, and maximum-shape requests.
- Re-run the pinned ParadeDB integration suite.

### CP3: Make media cleanup fair and configuration-safe

- Prevent failed `delete_claimed` rows from monopolizing every cleanup batch.
- Record retry progress and introduce bounded backoff or a fair per-lifecycle
  allocation while preserving deletion priority.
- Reject or safely default a zero cleanup interval before spawning the worker.
- Test persistent object-specific failures, queues larger than one batch, and
  forward progress for expired and retained candidates.

### CP4: Stabilize production test execution

- Apply the repository's PostgreSQL-safe test-thread cap to the Docker build and
  ordinary GitHub test job.
- Keep the production image test gate enabled.
- Prove a clean image build completes and the resulting container becomes
  healthy with the production security contract enabled.

### CP5: Make browser image processing terminate

- Handle image decode, `FileReader`, and canvas failure callbacks.
- Add a bounded timeout and always revoke temporary object URLs.
- Surface a retryable error instead of leaving uploads in `Processing`.
- Exercise corrupt, unsupported, canceled, and valid image inputs in a browser.

### CP6: Finish graph-history work admission

- Route lineage and fork-attribution reads through the shared graph-history
  concurrency budget.
- Use one aggregate deadline for every database operation belonging to a single
  history, comparison, attribution, or lineage request.
- Preserve current visibility redaction, topology, pagination, and output limits.

### CP7: Restore indexed identity lookup

- Provide an unambiguous exact username lookup that can use an index alongside
  the ParadeDB aliases.
- Apply the same identity path consistently to profiles, collections, and
  username availability checks.
- Verify correctness with the pinned ParadeDB image and verify the query plan is
  indexed rather than a sequential user-table scan.

### CP8: Make Explore tag filtering functional

- Define whether tag selection composes with or replaces text filtering.
- Pass the selected tag into bounded server-side discovery or apply a complete,
  pagination-correct alternative.
- Keep request-generation protection so stale responses cannot overwrite the
  latest text/tag selection.

### CP9: Improve code readability and maintainability

- Inventory readability hotspots in the code touched by NOMS-015 and CP1–CP8,
  prioritizing security boundaries, media lifecycle transitions, database query
  construction, and complex page-level state flows.
- Establish concise repository guidance for descriptive domain names, focused
  functions and modules, explicit error and state transitions, and comments that
  explain invariants and cross-component relationships rather than restating
  syntax.
- Extract repeated validation, admission, projection, and error-mapping logic
  where one shared abstraction makes the contract clearer without hiding
  authorization or lifecycle decisions.
- Split large modules only along stable responsibility boundaries; do not pursue
  file-size or function-length targets mechanically.
- Land refactors separately from behavior changes when practical, preserve the
  existing public API and persistence contracts, and use characterization tests
  before restructuring non-trivial behavior.
- Review each refactor as a behavior-preserving change and reject abstractions
  that reduce local clarity or make security-sensitive control flow implicit.

### CP10: Make SPA document metadata route-correct

- Define a shared metadata convention with a safe application default and
  route-specific titles, descriptions, canonical URLs, and social metadata.
- Inventory every routed page and apply the convention to Search, Explore,
  Login, Dashboard, collection, recipe, public-profile, settings, and fallback
  pages without allowing user-authored values to create unsafe or unbounded
  document metadata.
- Verify both server-rendered entry and client-side navigation so metadata is
  replaced when the active route changes instead of leaking from the previous
  page.
- Add focused tests for navigation from each marketing page into representative
  public, authenticated, dynamic, and not-found routes.

### CP11: Restore visibility-aware anonymous search

- Remove the unconditional authentication guard from the Search page and treat
  a missing session as an anonymous viewer rather than an exceptional request.
- Pass an optional viewer identity through search and autocomplete so anonymous
  callers can discover public recipes, public collections, and public profiles,
  while authenticated callers can additionally discover their own non-public
  resources where the existing product contract permits it.
- Give anonymous callers an abuse-resistant actor or request budget without
  weakening the existing global concurrency, database-deadline, input, or
  response-size bounds; derive any anonymous identity only from trusted request
  context and prevent callers from selecting or spoofing the budget key.
- Keep authorization in the database queries: every returned resource must be
  public or owned by the authenticated viewer, and anonymous results must never
  depend on client-side filtering.
- Preserve projection-level privacy for public collection results: truncate
  breadcrumbs before inaccessible ancestors and apply the collection/recipe
  visibility intersection to covers, members, counts, and other derived data.
- Ensure direct Search navigation, the Home search form, scoped searches,
  autocomplete, pagination, and empty results work without authentication and
  do not produce expected-authentication HTTP 500 responses.
- Cover anonymous and authenticated visibility matrices, public children below
  private or unlisted ancestors, mixed-visibility collection members, anonymous
  quota exhaustion and spoof attempts, and mixed public/private result sets
  with server and browser tests. Invalid or expired credentials must safely
  degrade to the anonymous result set without granting viewer-only visibility.

### CP12: Integrated release verification

- Run `just ci` and all pinned ParadeDB integration tests.
- Complete a clean production Docker image build and runtime health check.
- In a fresh isolated browser deployment, verify dashboard loading, text and tag
  discovery, corrupt-image failure feedback, graph/history reads, and cleanup
  forward progress.
- Recheck that NOMS-015 authorization, visibility, OAuth, logout, and media
  ownership behavior has not regressed.

## Acceptance criteria

- Dashboard and search endpoints have explicit input, output, concurrency, and
  aggregate-deadline bounds proportionate to their rendered data.
- Cleanup makes forward progress across lifecycle classes despite persistent
  deletion failures and cannot be disabled by a zero interval.
- Production CI and Docker builds complete reliably under the documented test
  concurrency limit.
- Every browser image-processing path resolves successfully or fails with usable
  feedback within a bounded time.
- Exact username identity lookup remains ParadeDB-safe and index-backed.
- Explore tag selection changes the recipe result set correctly across pages.
- Graph and lineage reads consistently share admission and deadline controls.
- Every route replaces document metadata correctly during SSR and client-side
  navigation.
- Anonymous Search returns only public results without an authentication prompt
  or server error; authenticated Search additionally returns resources visible
  to that viewer without exposing another user's non-public data.
- Anonymous Search remains bounded under abuse, and public collection results do
  not reveal inaccessible ancestors or member-derived metadata.
- Readability hotspots touched by this issue have clear ownership boundaries,
  descriptive names, explicit state transitions, and durable invariant-focused
  documentation without changing observable behavior.
- The integrated code, production image, and fresh-browser regression gate pass.

## Out of scope

- Reopening NOMS-015's completed OAuth intent, browser binding, media ownership,
  or hidden-lineage policy decisions.
- Enterprise security controls, compliance certification, CAPTCHA, or a general
  security-operations platform.
- New product features unrelated to the defects listed above.
