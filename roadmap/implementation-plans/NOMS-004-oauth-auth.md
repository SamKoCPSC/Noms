# NOMS-004: OAuth Authentication — Implementation Plan

**Issue:** [NOMS-004-oauth-auth.md](../issues/NOMS-004-oauth-auth.md)
**Created:** 2026-05-17
**Approach:** Bottom-up by dependency, 7 incremental checkpoints

---

## Checkpoint 1: Dependencies compile clean

Add server-only crates to `Cargo.toml`:

| Crate | Purpose |
|-------|---------|
| `oauth2` | OAuth 2.0 client (code flow, token exchange) |
| `jsonwebtoken` | Sign and verify JWTs |
| `axum-extra` | HTTP-only cookie management |
| `reqwest` | HTTP client (GitHub REST API calls) |

All gated behind `server` feature flag.

**Verify:**
- `cargo check --target wasm32-unknown-unknown` — zero errors/warnings
- `cargo check --features server` — zero errors/warnings

**Risk:** Low. Just dependency management.

---

## Checkpoint 2: Session management (pure logic, no DB)

**File:** `src/auth/session.rs`

- JWT struct with claims: `sub: Uuid`, `exp: usize`, `iat: usize`
- `create_session(user_id) -> String` — signs JWT with `SESSION_SECRET`, HS256
- `verify_session(token) -> Result<Uuid, Error>` — verifies signature + expiry
- `build_session_cookie(token) -> Cookie` — HttpOnly, Secure, SameSite=Lax, Max-Age=900
- `clear_session_cookie() -> Cookie` — deletion cookie
- `should_refresh(token) -> bool` — for rolling refresh logic

**Verify:**
- `cargo test` — unit tests for sign, verify, expiry, rolling refresh
- No infra needed, runs instantly

**Risk:** Low. Pure crypto, well-tested crates.

---

## Checkpoint 3: DB layer + migration runner

**File:** `src/db/mod.rs`

**SQLx connection pool:**
- Async pool setup, gated behind `server` feature
- Rust types mirroring schema: `AuthState`, `OauthAccount`, `User`

**Query functions:**
- `insert_auth_state()`, `get_auth_state()`, `delete_auth_state()`
- `get_oauth_account_by_provider()`, `get_oauth_account_by_email()`
- `insert_user()`, `insert_oauth_account()`, `update_oauth_last_used()`
- `get_user_by_id()` — for building UserProfile

**Schema migration:**
- Use pgschema (existing tooling) — call `pgschema apply` on app startup before serving
- Apply `migrations/extensions.sql` via `psql` first (pgcrypto, pg_cron), then `migrations/schema.sql` via pgschema
- Idempotent: safe on every startup, declarative schema is always applied
- Same behavior for local dev, staging, and production

**Startup sequence:**
1. Create SQLx connection pool
2. Run `psql < extensions.sql` then `pgschema apply` against the pool's database
3. Start serving — if migration fails, app refuses to start (fail-fast)

**Verify:**
- `cargo test --features server` — insert/select/delete against local Postgres
- App starts and runs pgschema cleanly on fresh DB

**Risk:** Medium. Schema exists but queries are new.

---

## Checkpoint 4: Account linking (DB + logic)

**File:** `src/auth/linking.rs`

- `resolve_user(provider, provider_user_id, email, display_name) -> (user_id, oauth_account_id, is_new)`
- Single DB transaction (atomic):
  1. Query existing by `provider + provider_user_id` → return if found
  2. Query existing by `email` → insert new oauth_account row, return existing user
  3. No match → insert new user + oauth_account, return new IDs

**Username generation (new users only):**
- Derive from display name: lowercase, strip non-alphanumeric, hyphenate words, truncate to 22 chars
- Append 4-char random hex suffix: `alice-smith-3f2a`
- If that collides (unlikely), retry with a new suffix (max 3 attempts)
- If all attempts collide, fall back to `user-{8hex}`: `user-a1b2c3d4`
- User can change username later via Settings (deferred to NOMS-005+)

**Verify:**
- `cargo test --features server` — three test cases:
  - Existing provider login → no new rows
  - New provider, same email → oauth_account linked to existing user
  - New provider, new email → new user + new oauth_account
- Unit tests for username generation:
  - "Alice Smith" → `alice-smith-XXXX`
  - "José García" → `jose-garcia-XXXX` (non-ASCII stripped)
  - Very long name → truncated to 22 chars + suffix
  - Collision retry produces different username

**Risk:** Medium. Transaction logic must be atomic.

---

## Checkpoint 5: OAuth handlers (server routes)

**File:** `src/auth/oauth.rs`

**Auth Start (`/auth/:provider/start`):**
- Extract provider from route param (`google` / `github`)
- Extract + validate `redirect_uri` query param (same-origin relative path only)
- Generate UUID auth state, store in DB with redirect_uri
- Build provider authorization URL with proper scopes, redirect user

**Auth Callback (`/auth/:provider/callback`):**
1. Extract state + code from query params
2. Verify state against DB, delete after use
3. Exchange code for tokens via `oauth2` crate
4. **Google:** extract ID token from response, manually verify JWT claims (iss, aud, exp, sub)
5. **GitHub:** call `GET /user` + `GET /user/emails` with access token, extract login/email/avatar_url
6. Call `resolve_user()` for account linking
7. Create session cookie, redirect to stored `redirect_uri`

**Scopes:**
- Google: `openid email profile`
- GitHub: `read:user user:email`

**Verify:**
- Manual flow against mock-oauth on `:8082`:
  - Visit `/auth/google/start` → complete mock login → callback sets cookie + redirects
  - Visit `/auth/github/start` → complete mock login → callback sets cookie + redirects
  - Check DB: new user + oauth_account rows created correctly

**Risk:** High. First real integration point, provider quirks surface here.

---

## Checkpoint 6: Route protection + auth context

**Files:** `src/auth/context.rs`, `src/middleware/auth.rs`

**Auth Context (Dioxus):**
- `AuthContext` struct: `current_user: Option<UserProfile>`, `is_authenticated: bool`
- Dioxus provider + hook for consuming context
- SSR initialization: read session cookie → verify JWT → query user from DB → populate context

**Route Protection (Axum layer):**
- Implemented as an Axum `Layer`/`Middleware` that wraps the Dioxus fullstack router
- Read + verify session cookie
- If valid: inject user into request extensions, continue to Dioxus handler
- If invalid/missing on protected path: return 302 redirect to `/login?redirect_uri=<current_path>` (before Dioxus sees the request)
- If already authenticated visiting `/login`: return 302 redirect to `/dashboard`
- Protected path list is a `HashSet<&'static str>` in the middleware — kept in sync with the `Route` enum by convention (no programmatic coupling to Dioxus routes)

**Protected routes:**
- `/dashboard`, `/recipes/new`, `/collections`, `/settings/*`

**Public routes:**
- `/`, `/login`, `/explore`, `/recipes/:id`

**Verify:**
- Hit `/dashboard` unauthenticated → 302 to `/login?redirect_uri=/dashboard`
- Complete auth → land on `/dashboard`
- Hit `/login` while authenticated → 302 to `/dashboard`
- Public routes work without auth

**Risk:** Medium. Middleware + SSR integration can be tricky.

---

## Checkpoint 7: Login page + navbar polish

**Files:** `src/pages/login.rs`, `src/components/navbar.rs`

**Login Page:**
- "Continue with Google" button → `/auth/google/start?redirect_uri=<path>`
- "Continue with GitHub" button → `/auth/github/start?redirect_uri=<path>`
- Preserve `redirect_uri` from query params if present (from route protection redirect)
- Remove email/password form (defer to NOMS-005)
- "Back to home" link

**Navbar:**
- Replace mock user with `AuthContext` data
- Signed in: show avatar + username
- Signed out: show "Sign In" button

**Verify:**
- Full e2e: visit `/recipes/new` → bounce to `/login` → click Google → mock login → land on `/recipes/new`
- Navbar reflects auth state both ways (signed in/out)
- `cargo clippy` clean, zero warnings on both targets

**Risk:** Low. Glue work, everything else is proven.

---

## Mock OAuth Server

The Navikt mock-oauth2-server is already running on `:8082` via docker-compose.

**Issuer-prefixed URLs (separate issuers per provider):**
- Google: `http://localhost:8082/google/authorize`, `/google/token`, `/google/userinfo`
- GitHub: `http://localhost:8082/github/authorize`, `/github/token`

Each issuer gets its own token signing key. The mock server supports:
- Authorization code flow
- UserInfo endpoint (`/issuer/userinfo`) — returns claims from access token
- Interactive login form (username + claims textarea)
- Token callbacks — configure claims per issuer via `JSON_CONFIG` env var

**Production:** Same code paths, just different endpoint URLs (real Google/GitHub). The `oauth2` crate abstracts the flow.

---

## Local HTTPS Development

The session cookie is always built with `Secure: true` — no environment-specific branching. Local development runs over HTTPS so the cookie is actually sent by the browser.

**Self-signed certificate:**
- Generated once by `just up` if `dev-cert.pem` / `dev-key.pem` don't exist:
  ```
  openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 \
      -keyout dev-key.pem -out dev-cert.pem -days 365 -nodes -subj '/CN=localhost'
  ```
- Both files are `.gitignore`d — never committed
- `dx serve` is invoked with `--http2` and the cert/key paths
- First browser visit to `https://localhost:8080` shows a cert warning — user clicks "proceed" once

**Application code:** No `cfg`, no env var checks, no conditional logic. Cookie is always `HttpOnly + Secure + SameSite=Lax`.

---

## Dependencies Summary

| Crate | Feature | Purpose |
|-------|---------|---------|
| `oauth2` | `server` | OAuth 2.0 client |
| `jsonwebtoken` | `server` | JWT sign/verify |
| `axum-extra` | `server` (+ `cookie`) | Cookie jar extractor |
| `cookie` | `server` | Cookie types + builder |
| `reqwest` | `server` (+ `json`) | HTTP client (GitHub API) |
| `uuid` | `server` (+ `v4`, `serde`) | User ID generation |
| `serde` | `server` (+ `derive`) | JWT claims serialization |
| `time` | `server` | Cookie max-age durations |
| `sqlx` | `server` | Already present, no changes needed |

## File Structure

```
src/
├── auth/
│   ├── mod.rs                 # Re-exports
│   ├── oauth.rs               # OAuth flow handlers (start, callback)
│   ├── session.rs             # JWT creation, verification, cookie management
│   ├── linking.rs             # Account linking logic
│   └── context.rs             # AuthContext for Dioxus UI
├── db/
│   └── mod.rs                 # SQLx queries, connection pool, types, migration runner
├── middleware/
│   └── auth.rs                # Route protection middleware
├── pages/
│   └── login.rs               # OAuth buttons, redirect_uri preservation
└── components/
    └── navbar.rs              # Real auth state from AuthContext
```
