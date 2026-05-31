# NOMS-005: User Profile & Account Management

**Status:** ⚪ Backlog  
**Phase:** Phase 1 (core user experience)  
**Depends on:** NOMS-004 (OAuth authentication)

## Overview

Complete the user profile and account management features that were deferred from NOMS-004. This makes the auth system usable end-to-end: users can see their real profile data, edit their settings, manage linked accounts, and sign out.

## Context

NOMS-004 delivered functional OAuth authentication but deferred several items:

| Deferred | Reason |
|----------|--------|
| `current_user` in AuthContext | Requires async DB fetch; synchronous `context_provider` can't do it |
| Settings pages (profile, accounts) | Placeholder UI only, not wired to data |
| Logout handler | No sign-out route or middleware handler |
| Profile editing | No server functions or API endpoints |
| Account linking UI | Settings page shows empty state |

## Acceptance Criteria

### AC1: Profile data loads on every authenticated request

- [ ] Async server function `get_current_profile(user_id)` fetches the user record from the database
- [ ] Server function is called during SSR and hydrates `current_user` in `AuthContext`
- [ ] Navbar displays the user's real username and avatar (from OAuth provider or uploaded)
- [ ] If profile fetch fails, navbar gracefully falls back to showing "User"

### AC2: Profile settings page is functional

- [ ] `/settings/profile` displays the user's current data: username, display name, email, bio, avatar
- [ ] User can edit display name and bio
- [ ] User can change username (with uniqueness validation and conflict messaging)
- [ ] Changes persist to the database via server function
- [ ] Optimistic UI updates with rollback on failure
- [ ] Toast or inline notification on success/error

### AC3: Linked accounts page is functional

- [ ] `/settings/accounts` shows all OAuth accounts linked to the user (fetched from `oauth_accounts` table)
- [ ] Each linked account displays: provider name, associated email, last used date
- [ ] User can link a new provider by clicking "Connect Google" / "Connect GitHub" (reuses OAuth flow)
- [ ] User can unlink an account (with confirmation dialog; blocked if it's their only linked account)
- [ ] Unlinking deletes the row from `oauth_accounts` via server function

### AC4: Logout works

- [ ] Sign-out button in navbar dropdown (or user menu)
- [ ] Clicking sign-out clears the session cookie and redirects to `/`
- [ ] Server function `logout()` returns a response with `Set-Cookie` to delete the session
- [ ] Auth context updates to unauthenticated state after logout

### AC5: Session refresh on expiry

- [ ] Auth middleware detects expired-but-valid session tokens
- [ ] Middleware auto-issues a refreshed token and sets a new cookie in the response
- [ ] User is not logged out mid-session due to token expiry

## Technical Details

### Database Schema (existing, no changes needed)

```sql
-- Users table (from NOMS-004 migration)
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    email           TEXT NOT NULL,
    avatar_url      TEXT,
    bio             TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- OAuth accounts table (from NOMS-004 migration)
CREATE TABLE oauth_accounts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider          TEXT NOT NULL,
    provider_user_id  TEXT NOT NULL,
    email             TEXT,
    email_verified    BOOLEAN NOT NULL DEFAULT false,
    profile_data      JSONB,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, provider_user_id)
);
```

### New DB queries needed

| Query | Purpose |
|-------|---------|
| `update_user(id, display_name, bio)` | Save profile edits |
| `update_username(id, new_username)` | Change username (with uniqueness check) |
| `get_oauth_accounts_by_user(user_id)` | List linked accounts |
| `delete_oauth_account(id, user_id)` | Unlink account |
| `count_oauth_accounts(user_id)` | Block unlink if it's the last account |

### New server functions

| Function | Method | Purpose |
|----------|--------|---------|
| `get_current_profile(user_id)` | GET | Fetch full user profile for AuthContext |
| `update_profile(display_name, bio)` | POST | Save display name and bio |
| `change_username(new_username)` | POST | Validate and change username |
| `get_linked_accounts(user_id)` | GET | Fetch OAuth accounts for settings page |
| `unlink_account(account_id)` | POST | Delete OAuth account link |
| `logout()` | POST | Clear session cookie |

### AuthContext changes

```rust
// Current state:
pub struct AuthContext {
    pub current_user_id: Option<Uuid>,
    pub current_user: Option<UserProfile>, // TODO: fetch via async server fn
    pub is_authenticated: bool,
}

// After NOMS-005:
pub struct AuthContext {
    pub current_user_id: Option<Uuid>,
    pub current_user: Option<UserProfile>, // Populated via server function
    pub is_authenticated: bool,
}
```

The tricky part: `context_provider` runs synchronously during SSR. Options:

1. **Server function with `use_server_future`** — call from a wrapper component that renders children once data loads. This is the Dioxus-idiomatic approach.
2. **Axum handler injection** — fetch the profile in the auth middleware and inject it into request extensions alongside `AuthUser`. This keeps the context provider synchronous.

**Recommendation:** Option 2 (middleware injection). It avoids a waterfall fetch during SSR, keeps the context provider simple, and reuses the DB connection pool already available in the middleware layer.

### Route protection changes

- Add `/settings/profile` and `/settings/accounts` to `PROTECTED_PATHS` (already done in NOMS-004)
- Add logout endpoint to OAuth router: `POST /auth/logout`
- Add account linking/unlinking endpoints or use Dioxus server functions

### Component changes

| Component | Change |
|-----------|--------|
| `Navbar` | Replace "User" fallback with real `current_user` data; add logout dropdown |
| `SettingsProfile` | Wire form to server functions; show current values; handle save/error states |
| `SettingsAccounts` | Fetch and display linked accounts; wire connect/unlink buttons |
| `Login` | (deferred) email/password form — out of scope for this issue |

## Out of Scope

- Email/password authentication (separate issue)
- Avatar upload (requires S3/R2 integration — Phase 2)
- Two-factor authentication
- Account deletion / GDPR data export
- Email verification flow
- Password reset flow
- Session management UI (active sessions list, revoke all)

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | Profile fetch in middleware | `current_user` populated in AuthContext on every authenticated request; navbar shows real username/avatar |
| 2 | Profile settings page | Editable display name and bio with server function persistence; optimistic UI |
| 3 | Username change | Username editing with uniqueness validation, conflict messaging, and DB update |
| 4 | Linked accounts display | Settings page fetches and renders OAuth accounts from database |
| 5 | Account linking & unlinking | Connect new providers via OAuth flow; unlink with confirmation; block last account removal |
| 6 | Logout | Sign-out button, session cookie clearing, redirect to home |
| 7 | Session refresh | Auto-refresh expired-but-valid tokens in middleware; no mid-session logouts |

## Success Metrics

- User can sign in via OAuth → see their real name in navbar → edit profile → see linked accounts → sign out
- All 7 checkpoints pass with tests (unit + integration)
- Zero clippy warnings on both wasm32 and x86_64 targets
- No unhandled error paths in server functions
