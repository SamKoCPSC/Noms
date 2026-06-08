# NOMS-006: Auth Security Hardening

**Status:** ✅ DONE  
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
| MEDIUM | 6 | No redirect_uri length limit, no method enforcement, no cookie Domain, no auth_states cleanup, OAuth tokens not revoked on deletion, rate limit XFF spoofing bypass |

## Acceptance Criteria

### AC1: OAuth state consumption is atomic (CRITICAL-1) ✅ DONE

- [x] `delete_auth_state` uses `DELETE ... RETURNING *` for atomic consumption
- [x] `callback_handler` restructured: delete state first, then validate returned state
- [x] Concurrent requests with the same `state` parameter cannot both succeed
- [x] Existing tests updated to reflect new flow
- [x] No regression in normal OAuth callback flow

### AC2: GET logout CSRF protection (CRITICAL-2) ✅ DONE

- [x] Logout endpoint requires `redirect_uri` query parameter on GET requests
- [x] `redirect_uri` is validated against allowed paths (same validation as OAuth start)
- [x] Unauthorized redirect targets default to `/`
- [x] Full-page navigation logout still works (cookie clearing via `Set-Cookie`)
- [x] POST logout remains unchanged (for programmatic use)

### AC3: Refactor to server-side sessions (HIGH-1) ✅ DONE

- [x] Create `sessions` table: `id UUID`, `user_id UUID`, `created_at TIMESTAMPTZ`, `expires_at TIMESTAMPTZ`, `refreshed_at TIMESTAMPTZ`, `revoked BOOLEAN DEFAULT FALSE`
- [x] JWT token becomes a session ID reference (claims: `sub` = session_id, `exp`, `iat`)
- [x] `verify_session` looks up session in `sessions` table (checks exists, not revoked, not expired)
- [x] `create_session` inserts row into `sessions` table, returns JWT referencing the session
- [x] Logout sets `revoked = TRUE` on the session row (instant revocation, no in-memory list)
- [x] Session refresh updates `refreshed_at` and `expires_at` on the session row
- [x] Rolling refresh: if token is within last 10 minutes of expiry, issue new token with extended expiry
- [x] Cleanup: expired + revoked sessions purged by pg_cron job (every hour, delete older than 24 hours)
- [x] Remove in-memory revocation list entirely
- [x] Migration backfills active sessions from current JWT state (or accepts clean start)

#### Files requiring modification

| File | Change | Impact |
|------|--------|--------|
| **`session.rs`** | `create_session(user_id)` → `create_session(pool, user_id) async`; `verify_session(token)` → `verify_session(pool, token) async`; new `revoke_session()` and `refresh_session()` | **High** — core API change, ~18 existing tests need updating |
| **`middleware/auth.rs`** | Must become async to access `PgPool`; passes pool to `verify_session` and `create_session` (refresh path) | **High** — critical path, every protected request goes through here |
| **`logout.rs`** | Must revoke session in DB before clearing cookie: `verify_session` → extract session_id → `revoke_session` | **Medium** — new DB call required, otherwise token remains valid until expiry |
| **`user_profile.rs`** | Add `pool` parameter to `verify_session` call (already async, already has pool in State) | **Low** — one line change |
| **`oauth.rs`** | Add `pool` parameter to `verify_session` (line 330) and `create_session` (line 363) calls; update tests | **Low** — already async, already has pool |
| **`context.rs`** | `extract_user_from_request()` (line 219) calls `verify_session` — needs pool parameter; check if used in sync contexts (SSR) | **Unknown** — depends on SSR usage |

### AC4: Error messages are sanitized (HIGH-2) ✅ DONE

- [x] All `INTERNAL_SERVER_ERROR` responses return generic message: "An internal error occurred. Please try again later."
- [x] Full error details are logged server-side with `tracing::error!()`
- [x] `OAuthError` variants: `TokenExchange`, `UserInfoExtraction`, `DbError`, `SessionError`, `LinkError` all sanitized
- [x] `SessionError::MissingSecret` no longer exposes "SESSION_SECRET not set" to clients
- [x] `Display` impl retains detailed messages for logging purposes
- [x] No regression in server-side error visibility

### AC5: Rate limiting on OAuth endpoints (HIGH-3) ✅ DONE

- [x] Rate limiting middleware applied to `/auth/{provider}/start` and `/auth/{provider}/callback`
- [x] Limits: 10 starts/minute per IP, 5 callbacks/minute per IP
- [x] Exceeded limit returns `429 Too Many Requests` with `Retry-After` header
- [x] Implementation uses sliding window (`Arc<DashMap<IpAddr, Vec<Instant>>>`) or `governor` crate
- [x] Rate limit state is cleaned up periodically to prevent memory growth
- [x] No impact on legitimate user flows

### AC6: PKCE for OAuth flow (HIGH-4) ✅ DONE

- [x] `start_handler` generates `code_verifier` (43-128 chars, base64url) and `code_challenge` (S256)
- [x] `code_challenge` stored in `auth_states` table alongside CSRF state
- [x] Authorization URL includes `code_challenge` and `code_challenge_method=S256`
- [x] `callback_handler` verifies `code_verifier` against stored `code_challenge` before token exchange
- [x] Migration adds `code_challenge TEXT` column to `auth_states`
- [x] Existing tests updated to include PKCE flow

### AC7: Redirect URI length validation (MEDIUM-1) ✅ DONE

- [x] `validate_redirect_uri` enforces maximum length of 2048 characters
- [x] Over-length URIs return `InvalidRedirectUri` error with 400 status
- [x] Test covers boundary conditions (2047 OK, 2048 OK, 2049 rejected)

### AC8: User profile enforces GET method (MEDIUM-2) ✅ DONE

- [x] `handle_user_profile` rejects non-GET methods with `405 Method Not Allowed`
- [x] Route registration in `main.rs` already uses `.get()` only, but handler adds defense in depth

### AC9: Cookie Domain attribute (MEDIUM-3) ✅ DONE

- [x] `build_session_cookie` reads domain from `COOKIE_DOMAIN` environment variable
- [x] If `COOKIE_DOMAIN` is set, cookie includes `.domain(domain)` attribute
- [x] If `COOKIE_DOMAIN` is not set, behavior is unchanged (no domain attribute)
- [x] Document the env var in `.env.local.example`

### AC10: Auth states cleanup (MEDIUM-4) ✅ DONE

- [x] `pg_cron` job added to `migrations/extensions.sql`:
  ```sql
  SELECT cron.schedule(
      'cleanup-auth-states',
      '*/5 * * * *',
      'DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL ''15 minutes'''
  );
  ```
- [x] Fallback: application-level cleanup task on startup (tokio timer) if pg_cron unavailable
- [x] Old auth states are purged within 15 minutes of creation

### AC11: OAuth token revocation on account deletion (MEDIUM-5) ✅ DONE

- [x] `oauth_accounts` table stores `refresh_token TEXT` (migration adds column)
- [x] On account deletion, call provider revocation endpoints:
  - Google: `POST https://oauth2.googleapis.com/revoke?token={refresh_token}`
  - GitHub: Document limitation (GitHub doesn't support token revocation API)
- [x] Revocation failures are logged but don't block account deletion
- [x] Timeout on revocation requests: 5 seconds max

### AC12: Account conflict warning on OAuth link (ENHANCEMENT)

- [ ] When user attempts to link a provider that is already linked to a different user account, the callback detects the conflict
- [ ] Callback redirects to `/settings/accounts?error=account_already_linked&provider={provider}` preserving the current user's session
- [ ] Frontend displays an error notification: **"This {provider} account is already linked to another user. That account will need to be deleted before you can link this provider."**
- [ ] User remains signed in as their current account (no session change)
- [ ] The OAuth flow is discarded (no new account created, no linking attempted)
- [ ] Linking the same provider to the current user still works when no conflict exists

### AC13: Rate limiting trusts only connection IP or trusted proxies (MEDIUM)

- [ ] `extract_client_ip` uses `ConnectInfo<SocketAddr>` (TCP connection IP) as the primary source
- [ ] `X-Forwarded-For` is only trusted when the TCP connection IP is in a configurable trusted proxy list
- [ ] Trusted proxy list is configurable via `TRUSTED_PROXIES` environment variable (comma-separated IPs or CIDRs)
- [ ] When `TRUSTED_PROXIES` is unset or empty, `X-Forwarded-For` is ignored entirely (secure default for direct deployment)
- [ ] When XFF is trusted, the leftmost non-proxy IP is used as the client IP (standard XFF unwinding)
- [ ] Loopback (`127.0.0.1`, `::1`) and Docker gateway (`172.17.0.1`) are trusted by default for local development behind a local proxy
- [ ] Test: spoofed `X-Forwarded-For` from direct connection does not bypass rate limit
- [ ] Test: valid `X-Forwarded-For` from trusted proxy IP is used correctly
- [ ] Test: multiple XFF entries are unwound correctly (leftmost non-proxy IP selected)
- [ ] No regression on existing rate limiting behavior (limits, sliding window, cleanup)

## Technical Details

### Database Migrations

```sql
-- Sessions table (for server-side session storage)
CREATE TABLE sessions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL,
    refreshed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked       BOOLEAN NOT NULL DEFAULT FALSE
);

-- Index for fast session lookups by token sub claim
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_revoked_expires ON sessions(revoked, expires_at) WHERE revoked = TRUE;

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

-- Cleanup expired/revoked sessions (pg_cron)
SELECT cron.schedule(
    'cleanup-sessions',
    '0 * * * *',  -- every hour
    'DELETE FROM sessions WHERE revoked = TRUE AND expires_at < NOW() - INTERVAL ''24 hours'''
);
```

### New Dependencies

| Crate | Purpose |
|-------|---------|
| `governor = "0.6"` | Rate limiting middleware (optional — can use custom implementation) |
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

### Server-Side Session Flow

```rust
// src/auth/session.rs — new flow

// JWT claims: session_id (sub), exp, iat
// The token itself carries no user data — just a reference to the DB row

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, SessionError> {
    let session_id = Uuid::new_v4();
    let expires_at = now_secs() + SESSION_LIFETIME_SECS;
    
    // Insert session row
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(session_id).bind(user_id).bind(expires_at)
        .execute(pool).await?;
    
    // Build JWT referencing the session
    let claims = SessionClaims {
        sub: session_id.to_string(),
        exp: expires_at,
        iat: now_secs(),
    };
    encode_jwt(&claims, &secret)
}

pub async fn verify_session(pool: &PgPool, token: &str) -> Result<Uuid, SessionError> {
    // 1. Verify JWT signature and expiry (fast fail)
    let claims = decode_jwt(token, &secret)?;
    
    // 2. Look up session in database
    let session = sqlx::query_as::<_, SessionRow>(
        "SELECT user_id, expires_at, revoked FROM sessions WHERE id = $1"
    ).bind(claims.sub).fetch_one(pool).await?;
    
    // 3. Check not revoked and not expired
    if session.revoked {
        return Err(SessionError::Revoked);
    }
    if now_secs() > session.expires_at {
        return Err(SessionError::Expired);
    }
    
    Ok(session.user_id)
}

pub async fn revoke_session(pool: &PgPool, session_id: Uuid) -> Result<(), SessionError> {
    sqlx::query("UPDATE sessions SET revoked = TRUE WHERE id = $1")
        .bind(session_id).execute(pool).await?;
    Ok(())
}

pub async fn refresh_session(pool: &PgPool, session_id: Uuid) -> Result<String, SessionError> {
    // Extend expiry by SESSION_LIFETIME_SECS from now
    let new_expires = now_secs() + SESSION_LIFETIME_SECS;
    sqlx::query("UPDATE sessions SET expires_at = $1, refreshed_at = NOW() WHERE id = $2")
        .bind(new_expires).bind(session_id).execute(pool).await?;
    
    // Return new JWT with extended expiry
    encode_jwt(&SessionClaims { sub: session_id, exp: new_expires, iat: now_secs() }, &secret)
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
| 6 | AC3 (server-side sessions) | Medium, refactoring — migration + session flow rewrite |
| 7 | AC5 (rate limiting) | Medium, new middleware |
| 8 | AC10 (auth_states cleanup) | Quick, cron job |
| 9 | AC2 (logout CSRF) | Medium, requires redirect validation |
| 10 | AC9 (cookie domain) | Quick, config-only |
| 11 | AC11 (OAuth token revocation) | Complex, provider API integration |

## Testing Plan

### Unit Tests
- [ ] `verify_session` rejects revoked sessions (revoked = TRUE)
- [ ] `verify_session` rejects expired sessions (expires_at < now)
- [ ] `create_session` inserts row into sessions table and returns valid JWT
- [ ] `revoke_session` sets revoked = TRUE on session row
- [ ] `refresh_session` extends expires_at and updates refreshed_at
- [ ] `validate_redirect_uri` rejects over-length URIs
- [ ] `handle_user_profile` rejects non-GET methods
- [ ] Rate limiter allows requests under limit
- [ ] Rate limiter blocks requests over limit
- [ ] PKCE code challenge generation and verification
- [ ] Atomic state delete returns correct data

### Integration Tests
- [ ] Full OAuth flow with PKCE (start → callback → session row created)
- [ ] Concurrent callback requests with same state (only one succeeds)
- [ ] Logout → session revoked in DB → token rejected by middleware
- [ ] Session refresh → new token issued with extended expiry
- [ ] Account deletion → cascading session deletion (ON DELETE CASCADE)
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
| 3 | Server-side sessions (AC3) | sessions table, DB-backed verify/create/revoke/refresh; tests pass |
| 4 | Rate limiting (AC5) | Rate limiter middleware on OAuth endpoints; tests pass |
| 4b | Rate limiting hardening (AC13) | Trusted proxy model for XFF; spoofed XFF rejected; tests pass |
| 5 | Cleanup and config (AC9, AC10) | Cookie domain env var, pg_cron cleanup jobs; verified |
| 6 | Logout CSRF fix (AC2) | Redirect validation on GET logout; tests pass |
| 7 | OAuth token revocation (AC11) | Provider API revocation on account deletion; tests pass |

## Success Metrics

- All 11 acceptance criteria pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- All existing auth tests pass (no regressions)
- Security audit re-check: 0 CRITICAL, 0 HIGH, 0 MEDIUM findings remaining
- OAuth flow works end-to-end with PKCE
- Logout immediately invalidates session (DB row revoked, token rejected)
- Session refresh extends expiry transparently
- Rate limiting blocks excessive requests with 429
