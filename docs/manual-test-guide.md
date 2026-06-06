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

## Known Issues Summary

| Issue | Status | Description | Fix Location |
|-------|--------|-------------|--------------|
| #1 | FIXED | Logout cookie not cleared | `navbar.rs` + `logout.rs` |
| #2 | FIXED | Dropdown not closing on outside click | `navbar.rs` |
| #3 | FIXED | Protected routes show login prompt when unauthenticated | `auth_required.rs` — AuthRequired component wraps all protected pages |

---

## Quick Regression Checklist

Run these after any code change:

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
