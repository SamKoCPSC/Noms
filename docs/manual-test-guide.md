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
- Success toast/notification appears
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
- Shows provider name, email, and link status
- "Unlink" button available for each account
- No console errors

---

### TC-13: Unlink Account

**Steps:**
1. Navigate to `/settings/accounts`
2. Click "Unlink" on a provider
3. Confirm unlink dialog

**Expected:**
- Confirmation dialog appears
- After confirmation, provider is removed from list
- Success notification appears
- If last provider, warn about account access loss

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
