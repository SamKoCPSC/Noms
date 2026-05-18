# NOMS-004: OAuth Authentication (Google + GitHub)

**Status:** ⚪ Backlog
**Phase:** 1 — Foundation
**Created:** 2026-05-17

## Description

Implement OAuth 2.0 authentication with Google and GitHub providers. This is the first real backend work — everything before this was frontend scaffolding. Users can sign in, get a session, and the UI reflects their auth state.

Automatic account linking is built in from day one — if a user signs in with GitHub using an email that already exists on a Google-linked account, they are merged into a single Noms account seamlessly. No manual linking UI, no duplicate accounts.

After authentication, users are always redirected back to the page they were trying to access — not to a generic dashboard. If they hit a protected route, get bounced to login, authenticate, they land right back where they were headed.

## Scope

### 1. OAuth 2.0 Flow (Both Providers)

Implement the full OAuth flow per DESIGN.md § Authentication & Session Management:

**Auth Start (`/auth/:provider/start`)**
- Accept optional `redirect_uri` query parameter (the page the user was trying to access)
- If no `redirect_uri` provided, default to `/dashboard`
- Validate `redirect_uri` is a same-origin relative path (prevent open redirect attacks)
- Generate random auth state (UUID) for CSRF protection
- Store state + redirect URI in `auth_states` table
- Redirect user to provider's authorization URL with proper scopes

**Auth Callback (`/auth/:provider/callback`)**
1. Verify state parameter matches stored value (CSRF check)
2. Exchange authorization code for tokens (access + ID token)
3. Verify provider ID token (JWT signature, audience, issuer, expiry)
4. **Account linking logic:**
   - Check `oauth_accounts` for existing `provider + provider_user_id` → existing login, create session
   - Check `oauth_accounts` for matching `email` across any provider → link new provider to existing user, create session
   - No match → create new `users` record + `oauth_accounts` row, create session
5. Retrieve stored `redirect_uri` from `auth_states`
6. Set HTTP-only session cookie + redirect to stored `redirect_uri`

**Scopes:**
- Google: `openid email profile`
- GitHub: `read:user user:email`

### 2. Session Management

Per DESIGN.md strategy — JWT in HTTP-only cookie:

- **JWT claims:** `sub` (user UUID), `exp` (15 minutes), `iat` (issued at)
- **Signing:** HS256 with secret from environment variable (`SESSION_SECRET`)
- **Cookie attributes:**
  - `HttpOnly: true` — JS cannot read (XSS protection)
  - `Secure: true` — HTTPS only
  - `SameSite: Lax` — CSRF protection
  - `Path: /` — available to all routes
  - `Max-Age: 900` (15 minutes)
- **Rolling refresh:** Each authenticated request with a valid but expiring JWT gets a fresh one (extend session on active use)
- **Logout:** Delete cookie (JWT remains valid until expiry — acceptable trade-off for simplicity)

### 3. Database Layer

Implement the SQLx queries against the existing schema (`migrations/schema.sql`):

| Query | Purpose |
|-------|---------|
| `INSERT INTO auth_states` | Store OAuth state + redirect URI for CSRF |
| `SELECT FROM auth_states WHERE id = $1` | Verify callback state + retrieve redirect URI |
| `DELETE FROM auth_states WHERE id = $1` | Clean up after use |
| `SELECT FROM oauth_accounts WHERE provider = $1 AND provider_user_id = $2` | Check existing provider login |
| `SELECT FROM oauth_accounts WHERE email = $1` | Check for email-based account linking |
| `INSERT INTO users` | Create new user |
| `INSERT INTO oauth_accounts` | Link provider to user |
| `UPDATE oauth_accounts SET last_used_at = NOW() WHERE id = $1` | Track usage |

Add periodic cleanup of expired `auth_states` rows (via pg_cron or app-level task).

### 4. Auth Context (Dioxus)

Provide auth state to the UI:

```rust
// Global auth context
pub struct AuthContext {
    pub current_user: Option<UserProfile>,  // Set during SSR from cookie
    pub is_authenticated: bool,
}

// UserProfile (subset of users table + display data)
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email: String,
}
```

- **SSR:** On server render, validate JWT from cookie → populate `AuthContext` → no flash of unauthenticated state
- **Client:** Context available via Dioxus scope extension or provide/consume pattern
- **Navbar:** Replace mock user with real `AuthContext` data

### 5. Login Page Wiring

Update the existing `/login` page shell:

- "Continue with Google" button → redirects to `/auth/google/start?redirect_uri=<current_path>`
- "Continue with GitHub" button → redirects to `/auth/github/start?redirect_uri=<current_path>`
- Preserve `redirect_uri` from query params if already present (from route protection redirect)
- Remove email/password form (not in scope yet — defer to NOMS-005)
- Add "Back to home" link

### 6. Route Protection

Add middleware/guard pattern for authenticated routes. When an unauthenticated user hits a protected route, redirect to `/login?redirect_uri=<original_path>` so the full auth flow preserves their destination:

| Route | Auth Required? | Behavior |
|-------|---------------|----------|
| `/` | No | Public landing page |
| `/login` | No (redirect to `/dashboard` if already logged in) | Sign-in page |
| `/explore` | No | Public discovery |
| `/dashboard` | Yes | Redirect to `/login?redirect_uri=/dashboard` if unauthenticated |
| `/recipes/new` | Yes | Redirect to `/login?redirect_uri=/recipes/new` if unauthenticated |
| `/recipes/:id` | No | Public recipe view |
| `/collections` | Yes | Redirect to `/login?redirect_uri=/collections` if unauthenticated |
| `/settings/*` | Yes | Redirect to `/login?redirect_uri=/settings/<path>` if unauthenticated |

### 7. Environment Configuration

New required environment variables:

| Variable | Purpose | Example |
|----------|---------|---------|
| `SESSION_SECRET` | HMAC signing key for JWTs (reuses existing session secret) | Already generated |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | From Google Cloud Console |
| `GOOGLE_CLIENT_SECRET` | Google OAuth client secret | From Google Cloud Console |
| `GITHUB_CLIENT_ID` | GitHub OAuth client ID | From GitHub OAuth app settings |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth client secret | From GitHub OAuth app settings |
| `APP_URL` | Base URL for callback redirects | `https://noms.example.com` |

Update `.env.local.example` with these values. Update `docker-compose.yml` and `Dockerfile` to pass them through.

## Dependencies

New crates (server-only, gated behind `server` feature):

| Crate | Purpose |
|-------|---------|
| `oauth2` | Standard OAuth 2.0 client |
| `jsonwebtoken` | Sign and verify JWTs |
| `axum-extra` | HTTP-only cookie management |
| `uuid` | Auth state generation (already in deps) |
| `serde` / `serde_json` | ID token parsing (likely already pulled in) |

## Out of Scope

- ❌ Email/password authentication — defer to NOMS-005
- ❌ Apple Sign In — defer until iOS native app is considered
- ❌ Email verification — no self-reported emails in OAuth flow
- ❌ Password reset — no passwords yet
- ❌ Manual account linking UI — linking is automatic, no settings page needed
- ❌ Session list / "logged in devices" — defer
- ❌ Explicit logout endpoint — cookie expiry is sufficient for now

## Acceptance Criteria

### OAuth Flow
- [ ] Clicking "Continue with Google" initiates OAuth flow and redirects to Google consent
- [ ] Clicking "Continue with GitHub" initiates OAuth flow and redirects to GitHub authorization
- [ ] After successful OAuth, user is redirected back to the page they were trying to access (not always /dashboard)
- [ ] If no original destination, redirect defaults to `/dashboard`
- [ ] `redirect_uri` is validated as same-origin relative path (open redirect attacks are blocked)
- [ ] Auth state (CSRF nonce) is verified on callback — invalid state is rejected
- [ ] Provider ID token is verified (signature, audience, issuer, expiry)

### Account Linking
- [ ] Signing in with a different provider using the same email merges into existing account
- [ ] Existing user signing in with same provider creates session (no duplicate)
- [ ] New user with new email creates fresh account
- [ ] `oauth_accounts.last_used_at` is updated on each login

### Sessions
- [ ] JWT is set as HTTP-only, Secure, SameSite=Lax cookie
- [ ] JWT contains user ID and 15-minute expiry
- [ ] Rolling refresh extends session on active use
- [ ] Expired JWT results in redirect to `/login`

### Auth Context & UI
- [ ] `AuthContext` provides user data to components during SSR (no flash of unauthenticated state)
- [ ] Navbar shows real avatar + username when signed in
- [ ] Navbar shows "Sign In" button when not signed in
- [ ] Mock user code is removed from Navbar

### Route Protection & Redirect
- [ ] Unauthenticated users are redirected to `/login?redirect_uri=<original_path>` on protected routes
- [ ] After auth, user lands back on their original page seamlessly
- [ ] Authenticated users are redirected to `/dashboard` when visiting `/login` directly
- [ ] Public routes (`/`, `/explore`, `/recipes/:id`) work without auth

### Code Quality
- [ ] `cargo check` passes with zero errors and zero warnings (both `wasm32` and `server` targets)
- [ ] `cargo clippy` passes with zero warnings
- [ ] `cargo test` passes (auth logic has unit tests)
- [ ] Server-only code is gated behind `#[cfg(feature = "server")]`
- [ ] Secrets are never logged or exposed in error messages

### Tests
- [ ] Unit tests for JWT creation and verification
- [ ] Unit tests for account linking logic (merge, new, existing)
- [ ] Unit tests for auth state generation and validation
- [ ] Unit tests for redirect URI validation (same-origin, relative path only)
- [ ] Unit tests for route protection middleware

## File Changes

```
src/
├── main.rs                    # Add auth routes, cookie middleware
├── db/
│   └── mod.rs                 # SQLx queries, connection pool, types
├── pages/
│   └── login.rs               # Wire OAuth buttons, preserve redirect_uri
├── components/
│   └── navbar.rs              # Replace mock user with AuthContext
├── auth/
│   ├── mod.rs                 # Re-exports
│   ├── oauth.rs               # OAuth flow handlers (start, callback)
│   ├── session.rs             # JWT creation, verification, cookie management
│   ├── linking.rs             # Account linking logic
│   └── context.rs             # AuthContext for Dioxus UI
└── middleware/
    └── auth.rs                # Route protection, redirect_uri preservation
```

## Outcome

Users can sign in with Google or GitHub and get a working session. After authenticating, they land right back where they were headed. The navbar reflects real auth state. Protected routes require authentication. Automatic account linking prevents duplicate accounts. The foundation is in place for email/password auth, session management, and all user-facing features that depend on identity.
