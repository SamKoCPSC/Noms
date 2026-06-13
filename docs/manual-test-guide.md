# Noms Manual Test Guide

> **Purpose:** Comprehensive manual testing checklist for AI agents and developers to verify all Noms features and regression-check known issues.
> **Environment:** App at `http://localhost:8080`, Mock OAuth2 at `http://localhost:8082`

---

## Prerequisites

1. Ensure the application is running: `cargo run` on `localhost:8080`
2. Ensure the mock OAuth2 server is running: `cargo run --package mock-oauth2-server` on `localhost:8082`
3. Open Chrome DevTools and clear cookies/storage before each test run

---

## Test Matrix

| # | Feature | Status | Notes |
|---|---------|--------|-------|
| 1 | Landing page (unauthenticated) | | |
| 2 | Mock OAuth2 Google login | | |
| 3 | Mock OAuth2 GitHub login | | |
| 4 | Navbar authenticated state | | |
| 5 | User dropdown toggle | | |
| 6 | User dropdown close on outside click | **FIXED** | Issue #2 |
| 7 | Profile page load | | |
| 8 | Profile save (valid data) | | |
| 9 | Profile save (invalid data) | | |
| 10 | Profile save (network failure rollback) | | |
| 11 | Username validation rules | | |
| 12 | Linked accounts page | | |
| 13 | Unlink account | | |
| 14 | Account deletion (3-layer confirmation) | | |
| 15 | Logout clears session cookie | **FIXED** | Issue #1 |
| 16 | Logout redirects to home | **FIXED** | Issue #1 |
| 17 | Protected routes show login prompt when unauthenticated | **FIXED** | Issue #3 — AuthRequired component |
| 18 | Server functions blocked when unauthenticated | | |
| 19 | Session token refresh | | |
| 20 | Concurrent request handling | | |
| 21 | Rate limiting on OAuth endpoints | | |
| 22 | Account linking flow (2nd provider) | | |
| 23 | Mock OAuth issuer separation (distinct identities) | | |
| 24 | Account linking with distinct provider identities | | |
| 25 | Unlink one provider (keep other linked) | | |
| 26 | Linking same provider twice (blocked) | | |
| 27 | Link provider owned by different account (collision) | | |
| 28 | Provider available after account deletion | | |
| 29 | OAuth callback URL verification (issuer-prefixed) | | |
| 30 | Cross-tab session revocation | | |
| 31 | Logout redirect URI validation | | |
| 32 | HTTP method enforcement on APIs | | |
| 33 | Theme toggle (dark/light mode) | | |
| 34 | Responsive navbar (mobile) | | |
| 35 | OAuth connect buttons from settings | | |
| 36 | 404 handling for unknown routes | | |
| 37 | Settings tabs navigation | | |
| 38 | Create recipe (minimal valid data) | | NOMS-008 AC2 |
| 39 | Create recipe (full data) | | NOMS-008 AC2 |
| 40 | Create recipe validation (missing title) | | NOMS-008 AC2 |
| 41 | View recipe detail page | | NOMS-008 AC3 |
| 42 | Dashboard recipe list | | NOMS-008 AC4 |
| 43 | Edit recipe (creates new version) | | NOMS-008 AC5 + NOMS-009 AC2 |
| 44 | Edit recipe auto-save | | NOMS-009 AC5 |
| 45 | Delete recipe | | NOMS-008 AC6 |
| 46 | Recipe ownership gating | | NOMS-008 AC3 |
| 47 | Recipe UUID route handling | | NOMS-008 |
| 48 | Dashboard empty state | | NOMS-008 AC4 |
| 49 | Recipe detail loading state | | NOMS-008 AC3 |
| 50 | Recipe detail error state | | NOMS-008 AC3 |
| 51 | Version history timeline (single version) | | NOMS-009 AC3 |
| 52 | Version history timeline (multiple versions) | | NOMS-009 AC3 |
| 53 | Version reconstruction (reverse diff chain) | | NOMS-009 AC3 |
| 54 | Restore version | | NOMS-009 AC4 |
| 55 | Restore version (cancel) | | NOMS-009 AC4 |
| 56 | Draft creation (new recipe) | | NOMS-009 AC5 |
| 57 | Draft auto-save (edit page) | | NOMS-009 AC5 |
| 58 | Publish draft recipe | | NOMS-009 AC5 |
| 59 | Draft toggle (dashboard) | | NOMS-009 AC5 |
| 60 | Fork recipe (same user — variant) | | NOMS-009 AC6 |
| 61 | Fork recipe (cross-user) | | NOMS-009 AC6 |
| 62 | Fork attribution display | | NOMS-009 AC6 |
| 63 | Version API no polling loop | | NOMS-009 (bug fix) |
| 64 | Switch between Details and History tabs | | NOMS-009 AC3 |
| 65 | Version select and diff display | | NOMS-009 AC3 |
| 66 | Draft recipe edit page | | NOMS-009 AC5 |
| 67 | Published recipe edit page | | NOMS-008 AC5 |
| 68 | Version history after multiple edits | | NOMS-009 AC2 |
| 69 | Restore creates new version (not overwrite) | | NOMS-009 AC4 |
| 70 | Recipe list API query parameters | | NOMS-009 (bug fix) |

---

## Detailed Test Cases

### TC-01: Landing Page (Unauthenticated)

**Steps:**
1. Clear all cookies and local storage
2. Navigate to `http://localhost:8080`

**Expected:**
- Page loads without errors
- Navbar shows "Sign In" button (not user avatar/dropdown)
- No console errors
- No network errors

---

### TC-02: Mock OAuth2 Google Login

**Steps:**
1. Click "Sign In" on the landing page
2. Select "Continue with Google"
3. Complete mock OAuth2 flow at `localhost:8082`

**Expected:**
- Redirect to mock OAuth2 server
- Mock consent screen appears
- After authorization, redirect back to app
- Navbar shows user avatar/dropdown
- Session cookie is set
- User is redirected to dashboard or home

---

### TC-03: Mock OAuth2 GitHub Login

**Steps:**
1. Click "Sign In" on the landing page
2. Select "Continue with GitHub"
3. Complete mock OAuth2 flow at `localhost:8082`

**Expected:**
- Same as TC-02 but with GitHub provider
- User profile shows GitHub as linked provider

---

### TC-04: Navbar Authenticated State

**Steps:**
1. Log in via any provider
2. Observe navbar

**Expected:**
- User avatar or initials displayed
- Dropdown menu available with options: Dashboard, Settings, Sign Out
- No "Sign In" button visible

---

### TC-05: User Dropdown Toggle

**Steps:**
1. Click user avatar in navbar
2. Click avatar again

**Expected:**
- First click: dropdown opens, shows menu items
- Second click: dropdown closes
- No console errors

---

### TC-06: User Dropdown Close on Outside Click (**FIXED - Issue #2**)

**Steps:**
1. Click user avatar to open dropdown
2. Click anywhere outside the dropdown (e.g., main content area)

**Expected:**
- Dropdown closes immediately
- No console errors
- Click handler properly cleaned up on component unmount

**Verification:** Check `src/components/navbar.rs` for `use_effect` + `use_drop` with `web_sys` document click listener.

---

### TC-07: Profile Page Load

**Steps:**
1. Navigate to `/settings/profile`

**Expected:**
- Page loads with user data populated
- Display name, email, and username fields visible
- No console errors
- Profile photo/avatar displayed if available

---

### TC-08: Profile Save (Valid Data)

**Steps:**
1. Navigate to `/settings/profile`
2. Change display name to a valid name (e.g., "Test User")
3. Click "Save Changes"

**Expected:**
- Optimistic UI update shows immediately
- Inline success message appears (green background, "Profile updated successfully")
- Data persists on page reload
- No console errors

---

### TC-09: Profile Save (Invalid Data)

**Steps:**
1. Navigate to `/settings/profile`
2. Enter invalid username (e.g., "ab" - too short, or "user@name" - invalid chars)
3. Click "Save Changes"

**Expected:**
- Validation error message displayed
- Form does not submit
- Original data preserved
- No network request sent for invalid data

---

### TC-10: Profile Save (Network Failure Rollback)

**Steps:**
1. Navigate to `/settings/profile`
2. Open Chrome DevTools Network tab
3. Set network to "Offline"
4. Change a field and click "Save Changes"

**Expected:**
- Error toast/notification appears
- UI rolls back to original values
- Form state restored to pre-edit values
- No data loss

---

### TC-11: Username Validation Rules

**Rules:**
- Length: 3-30 characters
- Allowed: alphanumeric, hyphens, underscores
- No leading/trailing hyphens or underscores
- Must be unique

**Test Cases:**
| Input | Expected |
|-------|----------|
| `abc` | Valid (min length) |
| `ab` | Invalid (too short) |
| `a` repeated 31 times | Invalid (too long) |
| `valid-user_1` | Valid |
| `-invalid` | Invalid (leading hyphen) |
| `invalid-` | Invalid (trailing hyphen) |
| `user@name` | Invalid (special char) |
| `user name` | Invalid (space) |

---

### TC-12: Linked Accounts Page

**Steps:**
1. Navigate to `/settings/accounts`

**Expected:**
- Lists all linked OAuth providers
- Shows provider name, associated email, and "Last used" timestamp (relative: "Just now", "X minutes ago", etc.)
- "Unlink" button available for each account
- "Connect additional accounts" section shows buttons for unlinked providers
- No console errors

---

### TC-13: Unlink Account

**Steps:**
1. Navigate to `/settings/accounts`
2. Click "Unlink" on a provider
3. Confirm unlink dialog

**Expected:**
- Confirmation dialog appears with provider name and "You will need to sign in again with your remaining accounts."
- After confirmation, provider is removed from list
- Inline success message appears (green background, "{Provider} account unlinked successfully")
- If last provider, action is blocked entirely with error: "You must have at least one linked account"

---

### TC-14: Account Deletion (3-Layer Confirmation)

**Steps:**
1. Navigate to `/settings/profile`
2. Scroll to danger zone
3. Click "Delete Account"
4. Confirm in first dialog
5. Type "delete `<username>`" in confirmation field
6. Confirm final warning

**Expected:**
- Layer 1: Initial confirmation dialog
- Layer 2: Type-to-confirm field (must match "delete `<username>`")
- Layer 3: Final irreversible warning
- After completion: account deleted, redirected to home, session cleared

---

### TC-15: Logout Clears Session Cookie (**FIXED - Issue #1**)

**Steps:**
1. Log in
2. Open Chrome DevTools Application tab > Cookies
3. Click "Sign Out"
4. Check cookies after redirect

**Expected:**
- Session cookie is cleared (deleted or expired)
- No session token remains in cookies
- No session token in localStorage/sessionStorage

**Implementation:** Full-page navigation to `/auth/logout` via `window.location().set_href()`. Server responds with `Set-Cookie` header to expire token.

---

### TC-16: Logout Redirects to Home (**FIXED - Issue #1**)

**Steps:**
1. Log in
2. Click "Sign Out"

**Expected:**
- Redirect to `http://localhost:8080/`
- Navbar shows "Sign In" button
- User avatar/dropdown gone
- No authenticated UI elements visible

---

### TC-17: Protected Routes Show Login Prompt When Unauthenticated (**FIXED - Issue #3**)

**Steps:**
1. Log out
2. Clear cookies (ensure clean state)
3. Navigate directly to `/dashboard`, `/settings/profile`, `/recipes/new`, `/collections`, `/explore`, or any other protected route

**Expected:**
- The page does NOT render empty forms or protected content
- Instead, a centered login prompt appears with:
  - A lock icon
  - "Sign in to continue" heading
  - A brief description explaining the page requires authentication
  - A "Sign In" button linking to `/login`
  - A "Go Home" button linking to `/`
- No console errors
- No network errors for protected API calls

**Implementation:** `AuthRequired` component wraps all protected pages, checking `use_auth().is_authenticated` before rendering children. Applied to 8 protected pages: dashboard, recipe_new, recipe_detail, collection_list, collection_detail, explore, settings_profile, settings_accounts.

---

### TC-17b: AuthRequired Component Behavior

**Steps:**
1. Log out and clear cookies
2. Visit each protected route individually:
   - `/dashboard`
   - `/recipes/new`
   - `/recipes/1` (or any valid recipe ID)
   - `/collections`
   - `/collections/1` (or any valid collection ID)
   - `/explore`
   - `/settings/profile`
   - `/settings/accounts`

**Expected (for each route):**
- The login prompt appears with consistent styling across all routes
- No protected page content is visible behind or around the prompt
- The "Sign In" link navigates to `/login`
- The "Go Home" link navigates to `/`

**Steps (authenticated):**
1. Log in
2. Visit the same protected routes

**Expected (for each route):**
- The full page content renders normally (no login prompt visible)
- Navigation and functionality work as expected

---

### TC-18: Server Functions Blocked When Unauthenticated

**Steps:**
1. Log out and clear cookies
2. Open Chrome DevTools Network tab
3. Try to call server functions via console or direct API request:
   - `POST /api/profile/save`
   - `POST /api/account/delete`
   - `GET /api/accounts`

**Expected:**
- All requests return 401 Unauthorized or 302 redirect
- No data returned
- Server functions properly check authentication

**Note:** This works correctly even though TC-17 shows SSR rendering issue. The server-side protection is intact; the issue is purely SSR rendering protected page HTML.

---

### TC-19: Session Token Refresh

**Steps:**
1. Log in
2. Wait for token to approach refresh threshold (10 minutes of 15-minute lifetime)
3. Make an API request

**Expected:**
- Token refreshed automatically
- New expiration time set
- No logout or re-authentication required
- Seamless experience

**Config:** Token lifetime = 15 minutes, rolling refresh threshold = 10 minutes.

---

### TC-20: Concurrent Request Handling

**Steps:**
1. Log in
2. Open multiple tabs
3. Make simultaneous requests (e.g., save profile in one tab while loading accounts in another)

**Expected:**
- All requests complete successfully
- No race conditions
- No token conflicts
- Consistent state across tabs

---

### TC-21: Rate Limiting on OAuth Endpoints

**Config:** 10 `/start` requests per IP per 60s; 5 `/callback` requests per IP per 60s.

**Steps (Start):**
1. Open Chrome DevTools Network tab
2. Repeatedly navigate to `/auth/google/start` (or use `curl` / console)
3. Send 11 requests within 60 seconds

**Expected (Start):**
- First 10 requests return 302 (redirect to OAuth provider)
- 11th request returns 429 Too Many Requests
- Response includes `Retry-After` header (value between 1 and 60 seconds)
- Response body is plain text: "Too Many Requests"

**Steps (Callback):**
1. Wait for rate limit window to reset (60 seconds)
2. Repeatedly navigate to `/auth/google/callback?code=fake&state=fake`
3. Send 6 requests within 60 seconds

**Expected (Callback):**
- First 5 requests process normally (may show error page, but not 429)
- 6th request returns 429 Too Many Requests

**Steps (Non-OAuth):**
1. Send 20+ requests to `/auth/logout`
2. Send 20+ requests to `/api/user_profile`

**Expected (Non-OAuth):**
- All requests process normally — no rate limiting on non-OAuth endpoints

**Verification:** Check `src/middleware/rate_limit.rs` for sliding-window implementation with `DashMap` per-IP tracking.

---

### TC-22: Account Linking Flow (2nd Provider)

**Steps:**
1. Log in with Google (TC-02)
2. Navigate to `/settings/accounts`
3. Verify only Google is listed
4. Click "Connect GitHub" in the "Connect additional accounts" section
5. Complete mock OAuth2 flow for GitHub at `localhost:8082`

**Expected:**
- Redirect to GitHub OAuth flow via mock server
- After authorization, redirect back to `/settings/accounts`
- Both Google and GitHub appear in the linked accounts list
- "Connect additional accounts" section shows no buttons (both linked)
- User remains logged in — no session interruption
- Navbar still shows user avatar/dropdown

**Steps (Email Match):**
1. Log out
2. Log in with GitHub (same email address)

**Expected:**
- User is logged into the same account (not a new account)
- Profile data is preserved (display name, username from original Google login)
- Both providers still listed on `/settings/accounts`

**Verification:** Check `src/auth/linking.rs` for `link_or_create` function — handles existing provider login, email match, and new user creation in a single transaction.

---

### TC-23: Mock OAuth Issuer Separation (Google vs GitHub Distinct Identities)

**Background:** The mock OAuth2 server uses issuer-prefixed URLs (`/google/*`, `/github/*`) to return different user identities per provider. This test verifies the separation works correctly.

**Steps (Verify Google Identity):**
1. Open Chrome DevTools Network tab
2. Navigate to `http://localhost:8082/google/.well-known/openid-configuration`
3. Check the response JSON

**Expected (Google):**
- `issuer`: `http://localhost:8082/google`
- `authorization_endpoint`: `http://localhost:8082/google/authorize`
- `token_endpoint`: `http://localhost:8082/google/token`
- `userinfo_endpoint`: `http://localhost:8082/google/userinfo`

**Steps (Verify GitHub Identity):**
1. Navigate to `http://localhost:8082/github/.well-known/openid-configuration`

**Expected (GitHub):**
- `issuer`: `http://localhost:8082/github`
- `authorization_endpoint`: `http://localhost:8082/github/authorize`
- `token_endpoint`: `http://localhost:8082/github/token`
- `userinfo_endpoint`: `http://localhost:8082/github/userinfo`

**Steps (Verify Distinct User Claims):**
1. Log in with Google, note the email shown in profile (should be `google-user@example.com`)
2. Log out, clear cookies
3. Log in with GitHub, note the email shown in profile (should be `github-user@example.com`)

**Expected:**
- Google login creates user with `google-user@example.com`
- GitHub login creates user with `github-user@example.com`
- These are two different accounts (different `provider_uid` values: `google-user-123` vs `github-user-456`)

**Verification:** Check `docker-compose.yml` `JSON_CONFIG` — `tokenCallbacks` defines separate claims per issuer:
```json
{
  "tokenCallbacks": {
    "google": { "sub": "google-user-123", "email": "google-user@example.com" },
    "github": { "sub": "github-user-456", "email": "github-user@example.com" }
  }
}
```

---

### TC-24: Account Linking with Distinct Provider Identities

**Steps:**
1. Clear cookies and storage
2. Log in with Google
3. Navigate to `/settings/accounts`
4. Verify Google account is listed with email `google-user@example.com`
5. Click "Connect GitHub"
6. Complete mock OAuth2 flow for GitHub (enter any username, click Sign-in)
7. Observe redirect back to `/settings/accounts`

**Expected:**
- Both Google and GitHub appear in linked accounts
- Google shows `google-user@example.com`
- GitHub shows `github-user@example.com`
- Different emails confirm distinct provider identities are linked to the same Noms account
- "Connect additional accounts" section is empty (no buttons)

**Steps (Verify Linking in Database):**
1. Open Chrome DevTools Network tab
2. Refresh `/settings/accounts`
3. Find the `GET /api/user_profile` request
4. Check the response JSON

**Expected:**
- Response contains both linked providers with distinct `provider_uid` values
- `provider_uid` for Google starts with or contains `google-user-123`
- `provider_uid` for GitHub starts with or contains `github-user-456`

---

### TC-25: Unlink One Provider (Keep Other Linked)

**Prerequisites:** Both Google and GitHub linked (from TC-24).

**Steps:**
1. Navigate to `/settings/accounts`
2. Click "Unlink" on the GitHub account
3. Confirm unlink dialog

**Expected:**
- GitHub account is removed from the list
- Google account remains linked
- Success message: "GitHub account unlinked successfully"
- User remains logged in (Google account still active)
- "Connect additional accounts" section shows "Connect GitHub" button again

**Steps (Verify Can Still Login with Remaining Provider):**
1. Log out
2. Log in with Google
3. Navigate to `/settings/accounts`

**Expected:**
- Only Google is listed
- User is logged into the same account (profile data preserved)
- GitHub is shown as available to connect

---

### TC-26: Linking Same Provider Twice (Should Be Blocked)

**Steps:**
1. Clear cookies and storage
2. Log in with Google
3. Navigate to `/settings/accounts`
4. Verify "Connect Google" button is NOT shown (already linked)
5. Try to directly navigate to `/auth/google/start?redirect_uri=/settings/accounts`

**Expected:**
- "Connect Google" button hidden on settings page (already linked)
- If navigating directly to `/auth/google/start`, the system recognizes the existing Google link
- User is either redirected back to settings, or the existing session is reused
- No duplicate provider link is created in the database

**Verification:** Check `src/auth/linking.rs` — `link_or_create` checks for existing `provider_uid` before creating new link. If user already exists with that provider, returns existing user instead of creating duplicate.

---

### TC-27: Link Provider Already Owned by Different Account (Cross-Account Collision)

**Background:** If User A (logged in via Google) tries to link a GitHub account that's already the primary login for User B, the system must reject the link. Accounts should never be silently merged.

**Setup (Create Two Separate Accounts):**
1. Clear cookies and storage
2. Log in with Google → creates **Account A** (email: `google-user@example.com`)
3. Update display name to "Google User" so you can identify this account later
4. Log out, clear cookies
5. Log in with GitHub → creates **Account B** (email: `github-user@example.com`)
6. Update display name to "GitHub User"
7. Log out, clear cookies

**Steps (Attempt Cross-Account Link):**
1. Log in with Google (Account A)
2. Navigate to `/settings/accounts`
3. Verify only Google is listed, display name is "Google User"
4. Click "Connect GitHub"
5. Complete mock OAuth2 flow for GitHub

**Expected:**
- The system detects that the GitHub `provider_uid` (`github-user-456`) is already linked to Account B
- Linking is **rejected** — GitHub does NOT appear in Account A's linked accounts
- Error message displayed: account already belongs to another user, or similar warning
- Account A still only has Google linked
- Account B is unaffected (GitHub still linked to "GitHub User")

**Steps (Verify Account B Is Unchanged):**
1. Log out, clear cookies
2. Log in with GitHub
3. Navigate to `/settings/profile`

**Expected:**
- Display name is still "GitHub User" (Account B)
- Not merged with Account A
- `/settings/accounts` shows only GitHub linked

**Verification:** Check `src/auth/linking.rs` — `link_or_create` must check if the `provider_uid` exists and belongs to a **different** `user_id`. If so, return an error instead of creating a duplicate link or merging accounts. The key logic:
```
1. Look up provider_uid in oauth_accounts table
2. If found AND belongs to current user → return existing user (normal login)
3. If found AND belongs to different user → ERROR, reject link
4. If not found → create new link (normal linking)
```

---

### TC-28: Provider Becomes Available After Account Deletion

**Background:** When Account B (from TC-27) is deleted, its OAuth provider links are removed. Account A should then be able to link that provider since the collision no longer exists.

**Prerequisites:** Two separate accounts exist from TC-27 setup:
- **Account A**: Google only, display name "Google User"
- **Account B**: GitHub only, display name "GitHub User"

**Steps (Delete Account B):**
1. Log in with GitHub (Account B)
2. Navigate to `/settings/profile`
3. Scroll to danger zone, click "Delete Account"
4. Complete 3-layer confirmation (confirm dialog → type "delete githubuser" → final warning)
5. Verify redirect to home page, logged out

**Expected (Deletion):**
- Account B and all its data are deleted
- GitHub `provider_uid` (`github-user-456`) is removed from `oauth_accounts` table
- Session is cleared

**Steps (Link Freed Provider to Account A):**
1. Clear cookies
2. Log in with Google (Account A)
3. Navigate to `/settings/accounts`
4. Verify display name is "Google User", only Google is linked
5. Click "Connect GitHub"
6. Complete mock OAuth2 flow for GitHub

**Expected (Linking):**
- Linking **succeeds** — the GitHub provider is no longer blocked
- Both Google and GitHub appear in Account A's linked accounts
- GitHub shows `github-user@example.com`
- Account A now owns the GitHub `provider_uid` that previously belonged to deleted Account B
- "Connect additional accounts" section is empty

**Steps (Verify Deleted Account Cannot Login Again):**
1. Log out, clear cookies
2. Try to log in with GitHub

**Expected:**
- User is logged into **Account A** (not a new account, not Account B)
- Display name is "Google User"
- `/settings/accounts` shows both Google and GitHub linked

**Verification:** Check account deletion logic — must cascade-delete rows from `oauth_accounts` table when user is deleted. The `oauth_accounts` table should have a foreign key with `ON DELETE CASCADE` to the `users` table, or the deletion code must explicitly clean up OAuth links before deleting the user.

---

### TC-29: OAuth Callback URL Verification (Issuer-Prefixed)

**Steps:**
1. Clear cookies and storage
2. Open Chrome DevTools Network tab
3. Click "Sign In" > "Continue with Google"
4. Observe the redirect URL in the Network tab

**Expected (Google):**
- Authorization redirect goes to: `http://localhost:8082/google/authorize?...`
- Callback URL in query params: `redirect_uri=http://localhost:8080/auth/google/callback`
- After mock login, callback hits: `http://localhost:8080/auth/google/callback?code=...&state=...`

**Steps (GitHub):**
1. Log out, clear cookies
2. Click "Sign In" > "Continue with GitHub"
3. Observe the redirect URL

**Expected (GitHub):**
- Authorization redirect goes to: `http://localhost:8082/github/authorize?...`
- Callback URL in query params: `redirect_uri=http://localhost:8080/auth/github/callback`
- After mock login, callback hits: `http://localhost:8080/auth/github/callback?code=...&state=...`

**Verification:** Check `.env.local` — URLs must use issuer prefixes:
```
GOOGLE_AUTH_URL=http://localhost:8082/google/authorize
GOOGLE_TOKEN_URL=http://localhost:8082/google/token
GOOGLE_USERINFO_URL=http://localhost:8082/google/userinfo
GITHUB_AUTH_URL=http://localhost:8082/github/authorize
GITHUB_TOKEN_URL=http://localhost:8082/github/token
GITHUB_USERINFO_URL=http://localhost:8082/github/userinfo
```

---

### TC-30: Cross-Tab Session Revocation

**Steps:**
1. Log in via any provider
2. Open the app in a second browser tab
3. Verify both tabs show authenticated state (user avatar in navbar)
4. In Tab A, click "Sign Out"
5. In Tab B, refresh the page or navigate to `/settings/profile`

**Expected:**
- Tab A: redirected to home, shows "Sign In" button
- Tab B: after refresh, shows "Sign In" button (session revoked)
- Tab B: no protected content visible — AuthRequired login prompt appears on protected routes
- Session cookie in Tab B is still present in browser, but server rejects it (DB `revoked = TRUE`)

**Verification:** Check `src/auth/logout.rs` for `session::revoke_session()` call — sets `revoked = TRUE` on the session row in the database.

---

### TC-31: Logout Redirect URI Validation

**Steps (Valid):**
1. Log in
2. Navigate to `/auth/logout?redirect_uri=/dashboard`

**Expected (Valid):**
- Session cleared, cookie expired
- Redirected to `/dashboard` (which shows AuthRequired prompt since logged out)

**Steps (Invalid — External URL):**
1. Log in
2. Navigate to `/auth/logout?redirect_uri=https://evil.com/phish`

**Expected (Invalid):**
- Session cleared, cookie expired
- Redirected to `/` (default home, NOT the external URL)

**Steps (Invalid — Protocol-Relative):**
1. Log in
2. Navigate to `/auth/logout?redirect_uri=//evil.com/phish`

**Expected (Invalid):**
- Redirected to `/` (default home)

**Steps (Invalid — No Leading Slash):**
1. Log in
2. Navigate to `/auth/logout?redirect_uri=dashboard`

**Expected (Invalid):**
- Redirected to `/` (default home)

**Steps (Valid — Query String):**
1. Log in
2. Navigate to `/auth/logout?redirect_uri=/dashboard%3Ftab%3Drecipes`

**Expected (Valid):**
- Redirected to `/dashboard?tab=recipes`

**Verification:** Check `src/auth/logout.rs` for `validate_redirect_uri()` — rejects URLs containing `://`, starting with `//`, or not starting with `/`. Max length 2048 chars.

---

### TC-32: HTTP Method Enforcement on APIs

**Steps:**
1. Log in
2. Open Chrome DevTools Network tab
3. Send a `POST` request to `/api/user_profile`
4. Send a `PUT` request to `/api/user_profile`
5. Send a `DELETE` request to `/api/user_profile`

**Expected:**
- `POST` returns 405 Method Not Allowed
- `PUT` returns 405 Method Not Allowed
- `DELETE` returns 405 Method Not Allowed
- Only `GET` returns 200 with user profile JSON

**Verification:** Check `src/main.rs` — `/api/user_profile` route uses `axum::routing::get()` only.

---

### TC-33: Theme Toggle (Dark/Light Mode)

**Steps:**
1. Log in
2. Locate the theme toggle button in the navbar (🌙 for light mode, ☀️ for dark mode)
3. Click the toggle button

**Expected:**
- Icon changes (🌙 ↔ ☀️)
- Page background, text colors, and card styles switch between light and dark themes
- `document.documentElement` has `dark` class added/removed
- `localStorage.theme` is set to `"dark"` or `"light"`

**Steps (Persistence):**
1. Set theme to dark
2. Refresh the page

**Expected:**
- Dark theme persists after page reload
- Theme preference survives navigation between pages

**Steps (Mobile Drawer):**
1. Resize viewport to mobile width (< 768px)
2. Open hamburger menu
3. Tap "☀️ Light Mode" or "🌙 Dark Mode" in the drawer

**Expected:**
- Theme toggles correctly
- Drawer closes after theme change

**Verification:** Check `src/utils/theme.rs` for `use_theme()` hook — reads from `localStorage.theme`, syncs `<html>` class and localStorage on change.

---

### TC-34: Responsive Navbar (Mobile)

**Steps:**
1. Log in
2. Resize browser viewport to mobile width (< 768px)

**Expected:**
- Desktop nav links ("Dashboard", "Explore", "New Recipe") are hidden
- Hamburger menu button (3 horizontal lines) appears
- Theme toggle button remains visible

**Steps (Drawer):**
1. Click hamburger menu button

**Expected:**
- Slide-out drawer appears from the right
- Drawer contains: Dashboard, Explore, New Recipe, Settings, Sign Out
- Drawer has a close button (✕)
- Clicking outside drawer content closes it
- Clicking a nav link navigates and closes drawer

**Steps (Authenticated vs Unauthenticated):**
1. Log out
2. Open hamburger menu

**Expected:**
- Drawer shows "Sign In" instead of "Settings" + "Sign Out"

**Verification:** Check `src/components/navbar.rs` — `navbar-links` class for desktop, `navbar-hamburger` and `navbar-drawer` for mobile.

---

### TC-35: OAuth Connect Buttons from Settings

**Steps (No Accounts):**
1. Log in with Google
2. Navigate to `/settings/accounts`
3. Unlink Google (should be blocked — last provider)
4. Instead, from a fresh account with only Google linked:

**Steps (One Provider Linked):**
1. Log in with Google only
2. Navigate to `/settings/accounts`
3. Scroll to "Connect additional accounts" section

**Expected:**
- "Connect Google" button is NOT shown (already linked)
- "Connect GitHub" button IS shown
- Clicking "Connect GitHub" redirects to `/auth/github/start?redirect_uri=/settings/accounts`
- After completing GitHub OAuth, redirected back to `/settings/accounts`
- Both Google and GitHub now listed
- "Connect additional accounts" section shows no buttons

**Steps (Both Providers Linked):**
1. With both providers linked, verify "Connect additional accounts" section

**Expected:**
- No "Connect" buttons visible (both already linked)

**Verification:** Check `src/pages/settings/settings_accounts.rs` — `linked_providers` HashSet controls visibility of connect buttons.

---

### TC-36: 404 Handling for Unknown Routes

**Steps:**
1. Navigate to a non-existent route: `/nonexistent`
2. Navigate to `/some/random/deep/path`

**Expected:**
- Application does not crash
- Page shows either:
  - A 404 error page, OR
  - The home page (Dioxus default fallback behavior)
- Navbar is still visible and functional
- No console errors
- No JavaScript errors in console

**Verification:** Check `src/main.rs` — Dioxus routing handles unknown routes; `ErrorBoundary` wraps all routes to catch rendering errors.

---

### TC-37: Settings Tabs Navigation

**Steps:**
1. Log in
2. Navigate to `/settings/profile`

**Expected:**
- Tab bar shows "Profile" and "Accounts"
- "Profile" tab has active styling (different background/border)
- Profile form is displayed

**Steps:**
1. Click "Accounts" tab

**Expected:**
- Navigates to `/settings/accounts`
- "Accounts" tab has active styling
- Linked accounts list is displayed
- URL changes to `/settings/accounts`

**Steps:**
1. Click "Profile" tab

**Expected:**
- Navigates to `/settings/profile`
- "Profile" tab has active styling
- Profile form is displayed
- URL changes to `/settings/profile`

**Verification:** Check `src/components/base/settings_tabs.rs` — `SettingsTabs` component with `SettingsTab::Profile` and `SettingsTab::Accounts` variants, active class applied based on `active` prop.

---

---

## NOMS-008: Recipe CRUD — Test Cases

### TC-38: Create Recipe (Minimal Valid Data)

**Prerequisites:** Logged in user.

**Steps:**
1. Navigate to `/recipes/new`
2. Enter title: "Pancakes"
3. Leave description empty
4. Enter ingredients (one per line):
   ```
   2 cups flour
   2 eggs
   1 cup milk
   ```
5. Enter steps (one per line):
   ```
   Mix dry ingredients
   Add eggs and milk
   Cook on griddle
   ```
6. Leave all time fields empty
7. Leave servings empty
8. Click "Save Draft"

**Expected:**
- Recipe is created as a draft (`is_draft = true`)
- Redirected to recipe detail or edit page
- Recipe appears in dashboard with "DRAFT" badge when "Show drafts" is enabled
- Version 1 created in `recipe_versions` table

---

### TC-39: Create Recipe (Full Data)

**Prerequisites:** Logged in user.

**Steps:**
1. Navigate to `/recipes/new`
2. Enter title: "Chocolate Chip Cookies"
3. Enter description: "Classic homemade cookies"
4. Set prep time: 15
5. Set cook time: 12
6. Set total time: 27
7. Set servings: 24
8. Enter ingredients:
   ```
   2.25 cups all-purpose flour
   1 cup butter, softened
   3/4 cup sugar
   2 eggs
   1 tsp vanilla extract
   2 cups chocolate chips
   ```
9. Enter steps:
   ```
   Preheat oven to 375°F
   Cream butter and sugar
   Beat in eggs and vanilla
   Mix in flour gradually
   Fold in chocolate chips
   Drop spoonfuls onto baking sheet
   Bake 9-11 minutes
   ```
10. Click "Save Draft"
11. Click "Publish Recipe"

**Expected:**
- Recipe saved as draft initially
- Publishing sets `is_draft = false`
- Recipe appears in dashboard without draft filter
- All fields persisted correctly in database

---

### TC-40: Create Recipe Validation (Missing Title)

**Steps:**
1. Navigate to `/recipes/new`
2. Leave title empty
3. Fill in other fields
4. Click "Save Draft"

**Expected:**
- Form validation prevents submission
- Error message displayed near title field
- Recipe is NOT created

---

### TC-41: View Recipe Detail Page

**Prerequisites:** At least one recipe exists.

**Steps:**
1. Navigate to `/dashboard`
2. Click on a recipe card

**Expected:**
- Redirected to `/recipes/{uuid}`
- Details tab shows:
  - Recipe title (h2 heading)
  - Description (if present)
  - Time cards: Prep Time, Cook Time, Total Time (if set)
  - Servings card (if set)
  - Ingredients list (bulleted)
  - Steps list (numbered, "Step N:" prefix)
- History tab shows version timeline
- Fork button visible
- No console errors

---

### TC-42: Dashboard Recipe List

**Prerequisites:** At least 2 recipes exist (1 published, 1 draft).

**Steps:**
1. Navigate to `/dashboard`
2. Observe recipe list with "Show drafts" unchecked

**Expected (Drafts Hidden):**
- Only published recipes appear
- Draft recipe is NOT visible
- Each card shows: title, description snippet, Edit button

**Steps:**
1. Check "Show drafts" checkbox

**Expected (Drafts Visible):**
- Both published and draft recipes appear
- Draft recipe has "DRAFT" badge
- Published recipe has no badge

**Steps:**
1. Uncheck "Show drafts"

**Expected:**
- Draft recipe disappears again
- Toggle works bidirectionally

---

### TC-43: Edit Recipe (Creates New Version)

**Prerequisites:** A published recipe exists.

**Steps:**
1. Navigate to recipe detail page
2. Click "Edit" button (or navigate to `/recipes/{uuid}/edit`)
3. Change title from "Pancakes" to "Fluffy Pancakes"
4. Add a new ingredient: "1 tsp baking powder"
5. Click "Save Draft" or "Publish Recipe"

**Expected:**
- Recipe updated in database
- New version (v2) created in `recipe_versions` table
- v1's `is_latest` set to false, `reverse_diff` populated
- v2's `is_latest` set to true, full snapshot stored
- `recipes` row updated to match v2
- `updated_at` timestamp refreshed
- Redirected back to detail page showing updated title

**Verification (History Tab):**
1. Click "History" tab
2. Two versions should appear: v2 (Current) and v1
3. Click v1 to reconstruct — should show original title "Pancakes"

---

### TC-44: Edit Recipe Page Auto-Save

**Prerequisites:** A recipe exists.

**Steps:**
1. Navigate to `/recipes/{uuid}/edit`
2. Change the title to "Auto-Save Test"
3. Wait 3 seconds (auto-save triggers after 2s debounce)
4. Open Chrome DevTools Network tab
5. Observe `POST /api/recipes/{uuid}/draft` request

**Expected:**
- Auto-save fires after ~2 seconds of inactivity
- Recipe saved as draft (or updated draft)
- No user action required
- Timer resets on each keystroke

**Steps (Multiple Edits):**
1. Immediately start typing again
2. Wait 3 seconds
3. Observe Network tab

**Expected:**
- Previous auto-save timer cancelled
- New auto-save fires after 2s from last keystroke
- Only ONE save request per burst of typing

---

### TC-45: Delete Recipe

**Prerequisites:** A recipe exists.

**Steps:**
1. Navigate to recipe detail page
2. Click "Delete" button (if visible — owner only)

**Expected:**
- Confirmation dialog appears: "Delete this recipe? This will permanently remove the recipe."
- Click "OK" → recipe deleted, redirected to `/dashboard`
- Recipe no longer appears in dashboard
- Recipe no longer accessible at `/recipes/{uuid}` (404 or error)
- Associated `recipe_tags` cascade-deleted
- Associated `recipe_versions` cascade-deleted
- Associated `fork_relationships` cascade-deleted (if any)

**Steps (Cancel):**
1. Create another recipe
2. Navigate to detail page
3. Click "Delete"
4. Click "Cancel" in confirmation dialog

**Expected:**
- Dialog closes
- Recipe still exists
- No navigation occurs

---

### TC-46: Recipe Ownership Gating

**Prerequisites:** Two accounts exist (Account A owns a recipe).

**Steps:**
1. Log in as Account A
2. Create a recipe, note the UUID from URL
3. Log out, clear cookies
4. Log in as Account B (different OAuth provider)
5. Navigate directly to `/recipes/{uuid}` (Account A's recipe)

**Expected:**
- Recipe detail page shows error: "Recipe not found" or "You don't have permission to view this recipe"
- Recipe data is NOT displayed
- Edit/Delete buttons NOT visible
- API call `GET /api/recipes/{uuid}` returns 403 or 404

---

### TC-47: Recipe UUID Route Handling

**Steps:**
1. Log in
2. Create a recipe
3. Note the UUID from the URL (e.g., `/recipes/d99afd00-6322-43e5-9832-65034ec01731`)
4. Navigate to `/recipes/invalid-uuid`

**Expected:**
- Application does not crash
- Shows "Recipe not found" error state
- No console errors

---

### TC-48: Dashboard Empty State

**Prerequisites:** User has zero recipes.

**Steps:**
1. Log in as a new user (no recipes)
2. Navigate to `/dashboard`

**Expected:**
- "No recipes yet" message displayed
- "Create your first recipe" link visible, points to `/recipes/new`
- No recipe cards shown
- "Show drafts" toggle present but has no effect

---

### TC-49: Recipe Detail Loading State

**Steps:**
1. Log in
2. Open Chrome DevTools Network tab
3. Set network to "Slow 3G"
4. Navigate to `/recipes/{uuid}`

**Expected:**
- Loading spinner displayed immediately
- "Loading recipe..." text shown
- No blank/empty page
- Content appears after API response arrives

---

### TC-50: Recipe Detail Error State

**Steps:**
1. Log in
2. Navigate to `/recipes/00000000-0000-0000-0000-000000000000` (non-existent UUID)

**Expected:**
- "Recipe not found" message displayed
- No crash, no console errors
- Page header still visible
- Navigation still functional

---

## NOMS-009: Recipe Versioning, Drafts & Branching — Test Cases

### TC-51: Version History Timeline (Single Version)

**Prerequisites:** A recipe with 1 version exists.

**Steps:**
1. Navigate to recipe detail page
2. Click "History" tab

**Expected:**
- "Version History" heading visible
- One version entry: "v1"
- "Current" badge on v1
- Recipe title displayed
- Timestamp shown (relative or absolute)
- "View" button to select version
- No polling loop in Network tab (only 1 request to `/api/recipes/{uuid}/versions`)

---

### TC-52: Version History Timeline (Multiple Versions)

**Prerequisites:** A recipe with 3+ versions exists.

**Steps:**
1. Create a recipe (v1)
2. Edit the recipe twice (v2, v3)
3. Navigate to recipe detail page
4. Click "History" tab

**Expected:**
- Three version entries: v3, v2, v1 (descending order)
- "Current" badge on v3 (latest)
- Each entry shows: version number, title, timestamp
- Clicking v2 reconstructs and displays v2's data in diff panel
- Clicking v1 reconstructs and displays v1's data in diff panel
- Clicking v3 shows current version data

---

### TC-53: Version Reconstruction (Reverse Diff Chain)

**Prerequisites:** A recipe with 3 versions exists.

**Setup:**
1. Create recipe: title "Original", ingredients: "flour", steps: "Mix"
2. Edit: title "Updated", ingredients: "flour, sugar", steps: "Mix, Bake"
3. Edit: title "Final", ingredients: "flour, sugar, eggs", steps: "Mix, Bake, Cool"

**Steps:**
1. Navigate to recipe detail page
2. Click "History" tab
3. Click "View" on v1

**Expected (v1 Reconstruction):**
- Diff panel shows reconstructed v1 data
- Title: "Original"
- Ingredients: "flour"
- Steps: "Mix"
- Reverse diff chain applied correctly: v3 → v2 → v1

**Steps:**
1. Click "View" on v2

**Expected (v2 Reconstruction):**
- Title: "Updated"
- Ingredients: "flour, sugar"
- Steps: "Mix, Bake"

---

### TC-54: Restore Version

**Prerequisites:** A recipe with 2 versions exists.

**Steps:**
1. Navigate to recipe detail page
2. Click "History" tab
3. Click "Restore" on v1
4. Confirm dialog: "Restore version 1? This will create a new version with the data from version 1."

**Expected:**
- New version (v3) created with v1's data
- v3 is marked as `is_latest = true`
- Original v1 and v2 unchanged
- Timeline reloads showing v3, v2, v1
- v3 has "Current" badge
- Details tab shows v1's data (restored)

**Verification:**
1. Click "Details" tab
2. Verify title and content match v1
3. Click "History" tab
4. Verify 3 versions listed

---

### TC-55: Restore Version (Cancel)

**Steps:**
1. Navigate to recipe detail page
2. Click "History" tab
3. Click "Restore" on v1
4. Click "Cancel" in confirmation dialog

**Expected:**
- No new version created
- Timeline unchanged
- No navigation

---

### TC-56: Draft Creation (New Recipe)

**Steps:**
1. Navigate to `/recipes/new`
2. Fill in title: "Draft Recipe"
3. Fill in some ingredients
4. Click "Save Draft"

**Expected:**
- Recipe created with `is_draft = true`
- Recipe appears in dashboard ONLY when "Show drafts" is checked
- "DRAFT" badge visible on recipe card
- Recipe detail page accessible to owner

---

### TC-57: Draft Auto-Save (Edit Page)

**Prerequisites:** A draft recipe exists.

**Steps:**
1. Navigate to `/recipes/{uuid}/edit`
2. Observe "DRAFT" badge on page
3. Change title to "Auto-Saved Title"
4. Wait 3 seconds
5. Open Chrome DevTools Network tab

**Expected:**
- `POST /api/recipes/{uuid}/draft` request visible
- Recipe saved automatically
- Timer resets on each keystroke
- No duplicate saves

**Steps (Verify Persistence):**
1. Refresh the page
2. Observe title field

**Expected:**
- Title shows "Auto-Saved Title" (persisted from auto-save)

---

### TC-58: Publish Draft Recipe

**Prerequisites:** A draft recipe exists.

**Steps:**
1. Navigate to `/recipes/{uuid}/edit`
2. Click "Publish Recipe"

**Expected:**
- `POST /api/recipes/{uuid}/publish` request sent
- Recipe's `is_draft` set to `false`
- Recipe appears in dashboard WITHOUT "Show drafts" filter
- "DRAFT" badge removed from recipe card
- Recipe visible in normal recipe list

**Verification:**
1. Navigate to `/dashboard`
2. Uncheck "Show drafts"
3. Recipe still visible (published)

---

### TC-59: Draft Toggle (Dashboard)

**Prerequisites:** User has 2 published recipes and 2 draft recipes.

**Steps:**
1. Navigate to `/dashboard`
2. "Show drafts" unchecked

**Expected:**
- Only 2 published recipes visible
- Draft recipes hidden

**Steps:**
1. Check "Show drafts"

**Expected:**
- All 4 recipes visible
- 2 drafts have "DRAFT" badge
- 2 published recipes have no badge

**Steps:**
1. Uncheck "Show drafts"

**Expected:**
- Back to 2 published recipes only

---

### TC-60: Fork Recipe (Same User — Variant)

**Prerequisites:** A published recipe exists.

**Steps:**
1. Navigate to recipe detail page
2. Click "Fork" button

**Expected:**
- New recipe created owned by same user
- New recipe is a draft (`is_draft = true`)
- New recipe has version 1 with full snapshot of original's latest version
- `fork_relationships` row created: original_recipe_id, forked_recipe_id, forked_by
- Redirected to `/recipes/{new_uuid}/edit`
- Fork attribution bar visible: "Forked from {original title}"

**Verification:**
1. Edit the forked recipe (change title)
2. Save
3. Navigate back to original recipe
4. Original recipe unchanged
5. Forked recipe shows modified title
6. Fork attribution still visible on forked recipe

---

### TC-61: Fork Recipe (Cross-User)

**Prerequisites:** Account A owns a recipe, Account B exists.

**Steps:**
1. Log in as Account A
2. Create a recipe titled "Account A's Recipe"
3. Log out, clear cookies
4. Log in as Account B
5. Navigate to Account A's recipe (if accessible — requires recipe to be accessible to Account B)

**Note:** Currently recipes are private by default. Cross-user forking requires the recipe to be accessible. This test may require making recipes public or sharing (future feature).

**Expected (if accessible):**
- Fork button visible
- Fork creates recipe owned by Account B
- Fork attribution shows Account A's name
- Forked recipe is independent of original

---

### TC-62: Fork Attribution Display

**Prerequisites:** A forked recipe exists.

**Steps:**
1. Navigate to forked recipe detail page

**Expected:**
- Fork attribution bar visible at top of page
- Shows: "Forked from {original title}"
- Original owner name displayed
- Fork message displayed (if provided during fork)
- Attribution persists across edits

---

### TC-63: Version API No Polling Loop

**Steps:**
1. Navigate to recipe detail page
2. Open Chrome DevTools Network tab
3. Click "History" tab
4. Wait 10 seconds
5. Count requests to `/api/recipes/{uuid}/versions`

**Expected:**
- Exactly 1 request to `/api/recipes/{uuid}/versions`
- No repeated polling
- No infinite request loop
- Network tab stable

---

### TC-64: Switch Between Details and History Tabs

**Steps:**
1. Navigate to recipe detail page
2. Click "Details" tab
3. Click "History" tab
4. Click "Details" tab
5. Click "History" tab

**Expected:**
- Tab switching works smoothly
- Details tab shows recipe content
- History tab shows version timeline
- No duplicate API calls on re-switch (versions cached)
- No console errors

---

### TC-65: Version Select and Diff Display

**Prerequisites:** Recipe with 2+ versions.

**Steps:**
1. Navigate to recipe detail page
2. Click "History" tab
3. Click "View" on v1

**Expected:**
- Diff panel (right side) shows reconstructed v1 data
- Title, description, times, servings, ingredients, steps all displayed
- "Loading" state during reconstruction
- No errors in diff panel

**Steps:**
1. Click "View" on v2 (current)

**Expected:**
- Diff panel updates to show v2 data
- Smooth transition, no flicker

---

### TC-66: Draft Recipe Edit Page

**Prerequisites:** A draft recipe exists.

**Steps:**
1. Navigate to `/recipes/{uuid}/edit`

**Expected:**
- "Edit Recipe" heading
- "DRAFT" badge visible
- All form fields pre-populated with draft data
- "Publish Recipe" button visible
- "Save Draft" button visible
- "Back" button visible

---

### TC-67: Published Recipe Edit Page

**Prerequisites:** A published recipe exists.

**Steps:**
1. Navigate to `/recipes/{uuid}/edit`

**Expected:**
- "Edit Recipe" heading
- No "DRAFT" badge (or "PUBLISHED" badge)
- All form fields pre-populated
- "Publish Recipe" button visible (saves as new version)
- "Save Draft" button visible
- "Back" button visible

---

### TC-68: Version History After Multiple Edits

**Prerequisites:** A recipe exists.

**Steps:**
1. Edit recipe 5 times, changing title each time: "v1", "v2", "v3", "v4", "v5"
2. Navigate to recipe detail page
3. Click "History" tab

**Expected:**
- 5 versions listed: v5, v4, v3, v2, v1
- v5 has "Current" badge
- Clicking each version reconstructs correct data
- v1 shows title "v1"
- v3 shows title "v3"
- v5 shows title "v5"

---

### TC-69: Restore Creates New Version (Not Overwrite)

**Prerequisites:** Recipe with 3 versions.

**Steps:**
1. Navigate to recipe detail page
2. Click "History" tab
3. Note v3 is current
4. Restore v1
5. Check version count

**Expected:**
- 4 versions now exist: v4 (restored v1 data), v3, v2, v1
- v4 is "Current"
- Original v1, v2, v3 unchanged
- Restore creates NEW version, does not modify existing versions

---

### TC-70: Recipe List API Query Parameters

**Steps:**
1. Log in
2. Open Chrome DevTools Network tab
3. Navigate to `/dashboard`
4. Observe `GET /api/recipes` request

**Expected (No Query Params):**
- Request: `GET /api/recipes` (no query string)
- Response: 200 OK
- Returns only published recipes

**Steps:**
1. Check "Show drafts"
2. Observe new `GET /api/recipes` request

**Expected (With Query Params):**
- Request: `GET /api/recipes?include_drafts=true`
- Response: 200 OK
- Returns published + draft recipes

---

## Known Issues Summary

| Issue | Status | Description | Fix Location |
|-------|--------|-------------|--------------|
| #1 | FIXED | Logout cookie not cleared | `navbar.rs` + `logout.rs` |
| #2 | FIXED | Dropdown not closing on outside click | `navbar.rs` |
| #3 | FIXED | Protected routes show login prompt when unauthenticated | `auth_required.rs` — AuthRequired component wraps all protected pages |

---

## Quick Regression Checklist

Run these after any code change:

### Auth & Profile (NOMS-004/005)
- [ ] Login works (Google + GitHub)
- [ ] Navbar shows correct auth state
- [ ] Dropdown opens/closes properly
- [ ] Profile page loads and saves
- [ ] Profile validation works
- [ ] Account deletion flow works
- [ ] Logout clears cookie and redirects
- [ ] AuthRequired prompt appears on protected routes when logged out
- [ ] Unauthenticated users cannot access server functions
- [ ] Account linking works (2nd provider)
- [ ] Google and GitHub return distinct user identities (TC-23)
- [ ] Linked accounts show different emails per provider (TC-24)
- [ ] Can unlink one provider, keep other (TC-25)
- [ ] Cannot link same provider twice (TC-26)
- [ ] Cannot link provider owned by different account (TC-27)
- [ ] Provider becomes linkable after account deletion (TC-28)
- [ ] Theme toggle persists across reload
- [ ] Mobile hamburger menu works

### Recipe CRUD (NOMS-008)
- [ ] Create recipe saves to database (TC-38)
- [ ] Recipe detail page displays all fields (TC-41)
- [ ] Dashboard shows recipe list (TC-42)
- [ ] Edit recipe updates database (TC-43)
- [ ] Delete recipe removes from database (TC-45)
- [ ] Recipe ownership gating works (TC-46)
- [ ] Dashboard empty state for new users (TC-48)
- [ ] Recipe detail loading/error states (TC-49, TC-50)

### Recipe Versioning, Drafts & Forking (NOMS-009)
- [ ] Version history timeline renders (TC-51)
- [ ] Multiple versions displayed correctly (TC-52)
- [ ] Version reconstruction works (TC-53)
- [ ] Restore version creates new version (TC-54)
- [ ] Draft creation works (TC-56)
- [ ] Auto-save triggers after 2s debounce (TC-57)
- [ ] Publish draft removes draft status (TC-58)
- [ ] Draft toggle filters dashboard (TC-59)
- [ ] Fork recipe creates independent copy (TC-60)
- [ ] Fork attribution displayed (TC-62)
- [ ] No API polling loops (TC-63)
- [ ] Tab switching works smoothly (TC-64)
- [ ] Version select shows diff (TC-65)
- [ ] API query params work correctly (TC-70)

---

## Testing Commands

```bash
# Start the application
cargo run

# Start mock OAuth2 server (separate terminal)
cargo run --package mock-oauth2-server

# Run existing tests
cargo test
```

---

## Notes for AI Agents

- Use Chrome DevTools MCP tools for browser testing
- Check Application > Cookies for session state
- Check Network tab for request/response codes
- Check Console for JavaScript errors
- Use `evaluate_script` to inspect auth context: `window.__authContext` (if exposed)
- Always clear cookies before testing auth flows
- Mock OAuth2 server handles both Google and GitHub providers
