# NOMS-006: Auth Security Hardening

**Status:** ⚪ Backlog  
**Phase:** Phase 1.5 (security fixes before production)  
**Depends on:** NOMS-004 (OAuth authentication), NOMS-005 (user profile)

## Overview

Address HIGH and MEDIUM severity findings from the Phase 2 security audit of the authentication flow. These fixes harden the auth system against common attack vectors and bring it to production-grade security standards.

## Context

A thorough security audit of the auth flow identified 14 findings across 4 severity levels. This issue covers the HIGH and MEDIUM severity items. The two CRITICAL findings (TOCTOU race in OAuth state consumption, CSRF-vulnerable GET logout) are included here as well since they're in the same code paths.

| Severity | Count | Summary |
|----------|-------|---------|
| CRITICAL | 2 | TOCTOU race in OAuth state, CSRF-vulnerable GET logout |
| HIGH | 4 | No JWT `jti`, error message leakage, no rate limiting, no PKCE |
| MEDIUM | 5 | No redirect_uri length limit, no method enforcement, no cookie Domain, no auth_states cleanup, OAuth tokens not revoked on deletion |

## Acceptance Criteria

### AC1: OAuth state consumption is atomic (CRITICAL-1)

- [ ] `delete_auth_state` uses `DELETE ... RETURNING *` for atomic consumption
- [ ] `callback_handler` restructured: delete state first, then validate returned state
- [ ] Concurrent requests with the same `state` parameter cannot both succeed
- [ ] Existing tests updated to reflect new flow
- [ ] No regression in normal OAuth callback flow

### AC2: GET logout CSRF protection (CRITICAL-2)

- [ ] Logout endpoint requires `redirect_uri` query parameter on GET requests
- [ ] `redirect_uri` is validated against allowed paths (same validation as OAuth start)
- [ ] Unauthorized redirect targets default to `/`
- [ ] Full-page navigation logout still works (cookie clearing via `Set-Cookie`)
- [ ] POST logout remains unchanged (for programmatic use)

### AC3: JWT tokens have `jti` for revocation (HIGH-1)

- [ ] `SessionClaims` includes `jti: String` field (UUIDv4)
- [ ] `create_session` generates and embeds `jti` in every new token
- [ ] In-memory revocation list: `Arc<DashSet<String>>` with TTL-based cleanup
- [ ] `verify_session` checks revocation list before accepting token
- [ ] Logout adds the current token's `jti` to the revocation list
- [ ] Revocation list cleanup runs on a timer (e.g., every 5 minutes, remove entries older than 15 minutes)
- [ ] Stolen tokens become invalid immediately after logout

### AC4: Error messages are sanitized (HIGH-2)

- [ ] All `INTERNAL_SERVER_ERROR` responses return generic message: "An internal error occurred. Please try again later."
- [ ] Full error details are logged server-side with `tracing::error!()`
- [ ] `OAuthError` variants: `TokenExchange`, `UserInfoExtraction`, `DbError`, `SessionError`, `LinkError` all sanitized
- [ ] `SessionError::MissingSecret` no longer exposes "SESSION_SECRET not set" to clients
- [ ] `Display` impl retains detailed messages for logging purposes
- [ ] No regression in server-side error visibility

### AC5: Rate limiting on OAuth endpoints (HIGH-3)

- [ ] Rate limiting middleware applied to `/auth/{provider}/start` and `/auth/{provider}/callback`
- [ ] Limits: 10 starts/minute per IP, 5 callbacks/minute per IP
- [ ] Exceeded limit returns `429 Too Many Requests` with `Retry-After` header
- [ ] Implementation uses sliding window (`Arc<DashMap<IpAddr, Vec<Instant>>>`) or `governor` crate
- [ ] Rate limit state is cleaned up periodically to prevent memory growth
- [ ] No impact on legitimate user flows

### AC6: PKCE for OAuth flow (HIGH-4)

- [ ] `start_handler` generates `code_verifier` (43-128 chars, base64url) and `code_challenge` (S256)
- [ ] `code_challenge` stored in `auth_states` table alongside CSRF state
- [ ] Authorization URL includes `code_challenge` and `code_challenge_method=S256`
- [ ] `callback_handler` verifies `code_verifier` against stored `code_challenge` before token exchange
- [ ] Migration adds `code_challenge TEXT` column to `auth_states`
- [ ] Existing tests updated to include PKCE flow

### AC7: Redirect URI length validation (MEDIUM-1)

- [ ] `validate_redirect_uri` enforces maximum length of 2048 characters
- [ ] Over-length URIs return `InvalidRedirectUri` error with 400 status
- [ ] Test covers boundary conditions (2047 OK, 2048 OK, 2049 rejected)

### AC8: User profile enforces GET method (MEDIUM-2)

- [ ] `handle_user_profile` rejects non-GET methods with `405 Method Not Allowed`
- [ ] Route registration in `main.rs` already uses `.get()` only, but handler adds defense in depth

### AC9: Cookie Domain attribute (MEDIUM-3)

- [ ] `build_session_cookie` reads domain from `COOKIE_DOMAIN` environment variable
- [ ] If `COOKIE_DOMAIN` is set, cookie includes `.domain(domain)` attribute
- [ ] If `COOKIE_DOMAIN` is not set, behavior is unchanged (no domain attribute)
- [ ] Document the env var in `.env.local.example`

### AC10: Auth states cleanup (MEDIUM-4)

- [ ] `pg_cron` job added to `migrations/extensions.sql`:
  ```sql
  SELECT cron.schedule(
      'cleanup-auth-states',
      '*/5 * * * *',
      'DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL ''15 minutes'''
  );
  ```
- [ ] Fallback: application-level cleanup task on startup (tokio timer) if pg_cron unavailable
- [ ] Old auth states are purged within 15 minutes of creation

### AC11: OAuth token revocation on account deletion (MEDIUM-5)

- [ ] `oauth_accounts` table stores `refresh_token TEXT` (migration adds column)
- [ ] On account deletion, call provider revocation endpoints:
  - Google: `POST https://oauth2.googleapis.com/revoke?token={refresh_token}`
  - GitHub: Document limitation (GitHub doesn't support token revocation API)
- [ ] Revocation failures are logged but don't block account deletion
- [ ] Timeout on revocation requests: 5 seconds max

## Technical Details

### Database Migrations

```sql
-- Add code_challenge to auth_states (for PKCE)
ALTER TABLE auth_states ADD COLUMN IF NOT EXISTS code_challenge TEXT;

-- Add refresh_token to oauth_accounts (for token revocation)
ALTER TABLE oauth_accounts ADD COLUMN IF NOT EXISTS refresh_token TEXT;

-- Update delete_auth_state to return deleted row (for atomic consumption)
CREATE OR REPLACE FUNCTION delete_auth_state_atomic(state_id VARCHAR(64))
RETURNS TABLE (
    id VARCHAR(64),
    redirect_uri TEXT,
    provider TEXT,
    created_at TIMESTAMPTZ
) AS $$
    DELETE FROM auth_states WHERE id = state_id RETURNING id, redirect_uri, provider, created_at;
$$ LANGUAGE sql;
```

### New Dependencies

| Crate | Purpose |
|-------|---------|
| `governor = "0.6"` | Rate limiting middleware (optional — can use custom implementation) |
| `dashmap = "6"` | Thread-safe in-memory revocation list |
| `tracing` | Structured error logging (if not already present) |

### Rate Limiting Implementation

```rust
// src/middleware/rate_limit.rs
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use dashmap::DashMap;

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<DashMap<String, VecDeque<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self { ... }
    pub fn is_allowed(&self, key: &str) -> bool { ... }
}

pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let ip = extract_ip(&req);
    if !limiter.is_allowed(&ip) {
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }
    next.run(req).await
}
```

### JWT Revocation List

```rust
// src/auth/session.rs additions
use dashmap::DashSet;
use std::sync::Arc;

pub struct RevocationList {
    revoked: Arc<DashSet<String>>,
    cleanup_interval: tokio::task::JoinHandle<()>,
}

impl RevocationList {
    pub fn new(max_age_secs: u64) -> Self { ... }
    pub fn revoke(&self, jti: String) { ... }
    pub fn is_revoked(&self, jti: &str) -> bool { ... }
    // Background task: cleanup entries older than max_age_secs
}
```

### PKCE Flow Changes

```rust
// In start_handler:
let code_verifier = generate_code_verifier(); // 43-128 chars
let code_challenge = create_pkce_challenge(&code_verifier); // S256
db::insert_auth_state(&pool, &csrf_state, prov, &redirect_uri, &code_challenge).await?;

// In callback_handler:
let auth_state = db::delete_auth_state_atomic(&pool, &params.state).await?;
// Verify code_verifier against code_challenge before token exchange
```

## Implementation Order

| Order | AC | Reason |
|-------|----|--------|
| 1 | AC7 (redirect_uri length) | Quick, low risk, standalone |
| 2 | AC4 (error sanitization) | Quick, low risk, standalone |
| 3 | AC8 (method enforcement) | Quick, low risk, standalone |
| 4 | AC1 (TOCTOU fix) | Medium, requires migration + handler changes |
| 5 | AC6 (PKCE) | Medium, requires migration + flow changes |
| 6 | AC3 (JWT jti) | Medium, requires session changes + revocation list |
| 7 | AC5 (rate limiting) | Medium, new middleware |
| 8 | AC10 (auth_states cleanup) | Quick, cron job |
| 9 | AC2 (logout CSRF) | Medium, requires redirect validation |
| 10 | AC9 (cookie domain) | Quick, config-only |
| 11 | AC11 (OAuth token revocation) | Complex, provider API integration |

## Testing Plan

### Unit Tests
- [ ] `verify_session` rejects revoked `jti`
- [ ] `create_session` includes valid `jti` in claims
- [ ] `validate_redirect_uri` rejects over-length URIs
- [ ] `handle_user_profile` rejects non-GET methods
- [ ] Rate limiter allows requests under limit
- [ ] Rate limiter blocks requests over limit
- [ ] PKCE code challenge generation and verification
- [ ] Atomic state delete returns correct data

### Integration Tests
- [ ] Full OAuth flow with PKCE (start → callback → session)
- [ ] Concurrent callback requests with same state (only one succeeds)
- [ ] Logout → token revocation → token rejected by middleware
- [ ] Account deletion → OAuth token revocation calls
- [ ] Rate limit enforcement on OAuth endpoints

### Regression Tests
- [ ] All existing auth tests pass (session, linking, logout, OAuth)
- [ ] Login flow still works end-to-end
- [ ] Account linking still works
- [ ] Session refresh still works
- [ ] Protected route access still works

## Out of Scope

- LOW severity findings (naive cookie parser, SESSION_SECRET length check, usize timestamps)
- Comprehensive penetration testing
- Security headers (CSP, HSTS, etc.) — separate issue
- Audit logging — separate issue
- IP-based session binding — separate issue
- Two-factor authentication — separate issue
- GDPR data export — separate issue

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | Quick wins (AC4, AC7, AC8) | Error sanitization, redirect_uri length, method enforcement; tests pass |
| 2 | OAuth flow hardening (AC1, AC6) | Atomic state consumption, PKCE; migration applied; tests pass |
| 3 | Token revocation (AC3, AC11) | JWT `jti`, in-memory revocation list, OAuth token revocation on deletion; tests pass |
| 4 | Rate limiting (AC5) | Rate limiter middleware on OAuth endpoints; tests pass |
| 5 | Cleanup and config (AC9, AC10) | Cookie domain env var, pg_cron cleanup job; verified |
| 6 | Logout CSRF fix (AC2) | Redirect validation on GET logout; tests pass |

## Success Metrics

- All 11 acceptance criteria pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- All existing auth tests pass (no regressions)
- Security audit re-check: 0 CRITICAL, 0 HIGH, 0 MEDIUM findings remaining
- OAuth flow works end-to-end with PKCE
- Logout immediately invalidates session token
- Rate limiting blocks excessive requests with 429
