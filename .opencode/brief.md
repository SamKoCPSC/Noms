# Task Brief

## Task Description
Fix 3 issues found during NOMS-005 acceptance testing:

**Issue 1 (High): Logout doesn't clear session cookie**
- Navbar sign-out handler uses XHR POST to /auth/logout, but browser ignores Set-Cookie headers from XHR responses
- Session cookie persists after logout, user remains authenticated
- Affects: navbar.rs sign-out handler and settings_profile.rs account deletion flow
- Fix: Replace XHR POST with full-page navigation to /auth/logout (allow GET on logout endpoint)

**Issue 2 (Medium): Navbar dropdown doesn't close on outside click**
- dropdown_open signal has no document-level click listener
- Dropdown stays open when clicking anywhere in main content area
- Fix: Add use_effect with document-level click listener via web_sys to close dropdown

**Issue 3 (High): Protected routes accessible after logout**
- Cascading from Issue 1 — no code change needed, resolves when Issue 1 is fixed

## Phase 0: Implementation Blueprint

### Architecture Overview
Dioxus 0.7.1 fullstack app with SSR + hydration, Axum 0.8 backend, JWT session cookies.

### Issue 1 — Logout Cookie Fix
**Root Cause:** Browser ignores `Set-Cookie` from XHR/fetch (same-origin policy). The `window.location().set_href("/")` redirect doesn't help because the cookie is still in the jar.

**Fix Strategy:** Allow GET on `/auth/logout` endpoint (logout is idempotent — safe for GET per RFC 9110). Replace XHR POST with `window.location().set_href("/auth/logout")` for a full-page navigation that processes the `Set-Cookie` header.

**Files to Change:**
- `src/main.rs:104-107` — Route registration: change `.post()` to `.get().post()` on `/auth/logout`
- `src/auth/logout.rs` — Handler already works with both methods (it only reads the cookie jar, doesn't check method). Add GET test to `make_router()`.
- `src/components/navbar.rs:36-46` — Replace `gloo_net::http::Request::post("/auth/logout").send()` with `web_sys::window().unwrap().location().set_href("/auth/logout").unwrap()`
- `src/pages/settings/settings_profile.rs:327-345` — Same replacement in the account deletion final step

**Dependencies:** Add `web-sys` features: `Document`, `MouseEvent`, `Element`, `Node`, `Location`, `Window` (already partially present, add missing ones)

### Issue 2 — Dropdown Close on Outside Click
**Fix Strategy:** Add a `use_hook_with_cleanup` that registers a document-level "click" listener via `web_sys`. When the click target is outside the dropdown container, set `dropdown_open.set(false)`.

**Files to Change:**
- `src/components/navbar.rs:15` — Add a `use_ref()` for the dropdown container element
- `src/components/navbar.rs:68-109` — Wrap dropdown in a `div` with `id="user-dropdown"` for easy DOM lookup
- `src/components/navbar.rs` (new effect) — `use_hook_with_cleanup` to register/unregister `document.addEventListener("click", ...)` that checks if `event.target` is inside `#user-dropdown`

**Implementation Pattern:**
```rust
use_hook_with_cleanup(|| {
    let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let target = event.target().unwrap();
        let dropdown = web_sys::window().unwrap().document().unwrap().get_element_by_id("user-dropdown").unwrap();
        if !dropdown.contains(&target) {
            dropdown_open.set(false);
        }
    }) as Box<dyn FnMut(_)>);
    document.add_event_listener_with_callback("click", closure.refrence().unchecked_ref()).unwrap();
    // Cleanup: remove listener
    Box::new(move || {
        document.remove_event_listener_with_callback("click", closure.refrence().unchecked_ref()).unwrap();
        closure.forget(); // Prevent double-free
    });
});
```

### Issue 3 — Protected Routes After Logout
No code change needed. Resolves automatically once Issue 1 clears the cookie.

### Dependencies
- `Cargo.toml`: Add `wasm-bindgen` if not present, expand `web-sys` features to include `Document`, `MouseEvent`, `Element`, `Node`, `Location`, `Window`

### Testing Plan
1. `cargo check` — verify compilation
2. `cargo test` — verify all existing tests pass (especially logout tests in `src/auth/logout.rs`)
3. `just up` + Chrome DevTools:
   - Issue 1: Login → click Sign Out → check Application tab → `noms_session` cookie is gone → navigate to /dashboard → redirects to /
   - Issue 2: Click avatar → dropdown opens → click anywhere outside → dropdown closes
   - Issue 3: After logout → /settings/profile → redirects to /

## Phase 1: Implementation Details
<!-- written by @develop-implement -->

## Phase 2: Review Verdict
<!-- written by @develop-review -->

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->
