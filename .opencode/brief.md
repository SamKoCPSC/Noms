# Task Brief

## Task Description
Implement 4 quick fixes from NOMS-006 (Auth Security Hardening), each with its own architect → implement → review workflow:
1. AC7: Redirect URI length validation (max 2048 chars)
2. AC4: Error message sanitization (generic messages for INTERNAL_SERVER_ERROR)
3. AC8: Method enforcement on user_profile endpoint (reject non-GET with 405)
4. AC9: Cookie Domain attribute from COOKIE_DOMAIN env var

## Phase 0: Implementation Blueprint — AC7: Redirect URI Length Validation

### Objective
Add a maximum length check (2048 characters) to the `validate_redirect_uri` function so that over-length URIs are rejected with `InvalidRedirectUri` error, which already maps to HTTP 400.

### Research Findings

| Item | Location | Details |
|------|----------|---------|
| `validate_redirect_uri` function | `src/auth/oauth.rs:130-135` | Currently checks: starts with `/`, not `//`, no `://`. No length check. |
| `OAuthError::InvalidRedirectUri` variant | `src/auth/oauth.rs:69` | Already exists, carries the offending URI string. |
| `IntoResponse` for `InvalidRedirectUri` | `src/auth/oauth.rs:101` | Already maps to `StatusCode::BAD_REQUEST` (400). **No change needed.** |
| Caller of `validate_redirect_uri` | `src/auth/oauth.rs:207` | Called in `start_handler` — the only call site. |
| Existing redirect URI tests | `src/auth/oauth.rs:444-462` | 4 tests: valid, absolute URL, no leading slash, protocol-relative. |
| Constants section | `src/auth/oauth.rs:192` | `CSRF_STATE_TTL_SECS` constant lives here; new constant should go nearby. |

### Changes Required

#### 1. Add constant for max redirect URI length

**File:** `src/auth/oauth.rs`
**Location:** Add after line 192 (after `CSRF_STATE_TTL_SECS`, before the `// ── Handlers ──` comment on line 194)

```rust
/// Maximum allowed length for redirect_uri parameter.
const REDIRECT_URI_MAX_LEN: usize = 2048;
```

#### 2. Add length check in `validate_redirect_uri`

**File:** `src/auth/oauth.rs`
**Location:** Lines 130-135

**Before (current):**
```rust
fn validate_redirect_uri(uri: &str) -> Result<(), OAuthError> {
    if !uri.starts_with('/') || uri.starts_with("//") || uri.contains("://") {
        return Err(OAuthError::InvalidRedirectUri(uri.to_string()));
    }
    Ok(())
}
```

**After:**
```rust
fn validate_redirect_uri(uri: &str) -> Result<(), OAuthError> {
    if uri.len() > REDIRECT_URI_MAX_LEN {
        return Err(OAuthError::InvalidRedirectUri(uri.to_string()));
    }
    if !uri.starts_with('/') || uri.starts_with("//") || uri.contains("://") {
        return Err(OAuthError::InvalidRedirectUri(uri.to_string()));
    }
    Ok(())
}
```

**Rationale:** The length check is placed first because it's a cheap `usize` comparison that can reject maliciously long inputs before any string operations.

#### 3. Add tests for the new validation

**File:** `src/auth/oauth.rs`
**Location:** After line 462 (after `test_validate_redirect_uri_invalid_protocol_relative`, before `test_build_oauth_clients`)

Add two test functions:

```rust
#[test]
fn test_validate_redirect_uri_too_long() {
    let long_uri = format!("/{}", "a".repeat(2048));
    assert!(matches!(
        validate_redirect_uri(&long_uri),
        Err(OAuthError::InvalidRedirectUri(_))
    ));
}

#[test]
fn test_validate_redirect_uri_at_max_length() {
    let exact_uri = format!("/{}", "a".repeat(2047));
    assert!(validate_redirect_uri(&exact_uri).is_ok());
}
```

### Verification Checklist

- [x] `OAuthError::InvalidRedirectUri` already maps to HTTP 400 (`src/auth/oauth.rs:101`) — no change needed
- [x] Error message includes the offending URI via `to_string()` — already the case (line 84)
- [x] Length check is placed before other validation — avoids unnecessary string ops on oversized input
- [x] Boundary test included (exactly 2048 chars = OK, 2049 chars = rejected)

### Implementation Order

1. Add `REDIRECT_URI_MAX_LEN` constant (line ~193)
2. Add length check as first condition in `validate_redirect_uri` (line 131)
3. Add two test functions after existing redirect URI tests (after line 462)
4. Run `cargo test validate_redirect_uri` to verify all 6 redirect URI tests pass

### Dependencies / Build

- No new crate dependencies required.
- All types (`OAuthError`, `StatusCode`) already imported.

### Risks / Trade-offs

- **Truncation in error message:** The `InvalidRedirectUri` variant stores the full URI string. For a 2049+ char input, this means the error response body will include the full oversized URI. This is acceptable for a 400 response (the client is at fault), but could be considered a minor information disclosure. No action needed for this fix.
- **Byte length vs character length:** `.len()` on `&str` returns byte length, not character count. Since UTF-8 bytes >= characters, this is conservative (rejects even earlier for non-ASCII). This is the desired security posture.

### Gaps / Follow-up

- None. This is a self-contained, single-function change with no cross-module impact.

---

## Phase 0 (continued): Implementation Blueprint — AC8: Method Enforcement on user_profile Endpoint

### Objective
Add handler-level HTTP method enforcement to `handle_user_profile` in `src/auth/user_profile.rs` so that non-GET requests are rejected with `405 Method Not Allowed` and an `Allow: GET` header. Currently, the handler accepts any HTTP method because it has no internal method check — it relies solely on Axum's router-level `.get()` binding. This fix adds defense-in-depth by enforcing the constraint at the handler level as well.

### Research Findings

| Item | Location | Details |
|------|----------|---------|
| `handle_user_profile` function | `src/auth/user_profile.rs:27-73` | Handler for `/api/user_profile`. Takes `_req: Request<Body>` but **never uses it** (underscore prefix). |
| Handler signature | `src/auth/user_profile.rs:27-31` | `pub async fn handle_user_profile(State(state): State<UserProfileState>, jar: CookieJar, _req: Request<Body>) -> Result<Json<AuthContext>, (StatusCode, String)>` |
| Route registration | `src/main.rs:89-91` | `axum::routing::get(auth::user_profile::handle_user_profile)` — router-level `.get()` filter is already in place. |
| Return type | `src/auth/user_profile.rs:31` | `Result<Json<AuthContext>, (StatusCode, String)>` — error arm is a tuple `(StatusCode, String)`. |
| Existing imports | `src/auth/user_profile.rs:6-13` | `axum::body::Body`, `axum::extract::State`, `axum::http::Request`, `axum::http::StatusCode`, `axum::Json`, `axum_extra::extract::cookie::CookieJar`, `sqlx::PgPool`, `tracing` |
| Existing 405 patterns | N/A | **None.** Zero instances of `METHOD_NOT_ALLOWED`, `405`, or handler-level method checks in the entire codebase. |
| `HeaderMap` usage | `src/auth/logout.rs:16-23` | Pattern: `let mut headers = axum::http::HeaderMap::new(); headers.insert(axum::http::header::SET_COOKIE, ...);` |
| Test pattern (logout.rs) | `src/auth/logout.rs:26-118` | Uses `tower::ServiceExt::oneshot` with `axum::Router` for handler-level integration tests. |
| Test utilities | `src/test_utils.rs` | `setup_test_db()` provides temp PostgreSQL with schema for integration tests. |
| `tower` dev dependency | `Cargo.toml:59` | `tower = { version = "0.5", features = ["util"] }` — `ServiceExt` available for tests. |
| `axum::http::Method` | axum re-export | `axum::http::Method::GET` — same as `http::Method::GET`. |

### Why Handler-Level Enforcement?

The route is already registered with `.get()` in `main.rs:90`, which means Axum's router will reject non-GET methods with 405 at the routing layer. However, handler-level enforcement provides:

1. **Defense-in-depth:** If the route registration is ever changed (e.g., someone adds `.post()` or switches to `.route()` with method-agnostic routing), the handler still protects itself.
2. **Self-documenting:** The handler explicitly states its contract — it only handles GET requests.
3. **Testability:** The method check can be tested in isolation without depending on router configuration.

### Changes Required

#### 1. Add method check at the start of `handle_user_profile`

**File:** `src/auth/user_profile.rs`
**Location:** Lines 27-31 (handler signature) and line 32 (first line of function body)

**Before (current):**
```rust
pub async fn handle_user_profile(
    State(state): State<UserProfileState>,
    jar: CookieJar,
    _req: Request<Body>,
) -> Result<Json<crate::auth::context::AuthContext>, (StatusCode, String)> {
    // Check for valid session
```

**After:**
```rust
pub async fn handle_user_profile(
    State(state): State<UserProfileState>,
    jar: CookieJar,
    req: Request<Body>,
) -> Result<Json<crate::auth::context::AuthContext>, (StatusCode, String)> {
    // Enforce GET-only method
    if req.method() != axum::http::Method::GET {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::ALLOW,
            "GET".parse().expect("valid Allow header value"),
        );
        return Err((
            StatusCode::METHOD_NOT_ALLOWED,
            headers,
            "Method Not Allowed".to_string(),
        ).into_response());
    }

    // Check for valid session
```

Wait — the return type is `Result<Json<AuthContext>, (StatusCode, String)>`. We can't return `(StatusCode, HeaderMap, String).into_response()` because the error type is `(StatusCode, String)`. Let me reconsider.

**Corrected approach:** Change the return type to `axum::response::Response` to support custom headers on the 405 response.

**After (corrected):**
```rust
pub async fn handle_user_profile(
    State(state): State<UserProfileState>,
    jar: CookieJar,
    req: Request<Body>,
) -> axum::response::Response {
    // Enforce GET-only method
    if req.method() != axum::http::Method::GET {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::ALLOW,
            "GET".parse().expect("valid Allow header value"),
        );
        return (StatusCode::METHOD_NOT_ALLOWED, headers, "Method Not Allowed").into_response();
    }

    // Check for valid session
    let session_token = jar.get(session::COOKIE_NAME);
    let verified_user_id =
        session_token.and_then(|cookie| session::verify_session(cookie.value()).ok());

    match verified_user_id {
        Some(user_id) => {
            // Fetch user from database
            let user = crate::db::get_user_by_id(&state.pool, user_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to fetch user profile");
                    (StatusCode::INTERNAL_SERVER_ERROR, "An internal error occurred. Please try again later.".to_string())
                })?
                .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

            // Convert User to UserProfile
            let profile = crate::auth::context::UserProfile {
                id: user.id,
                username: user.username,
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                bio: user.bio,
            };

            let ctx = crate::auth::context::AuthContext {
                current_user_id: Some(user_id),
                current_user: Some(profile),
                is_authenticated: true,
            };

            Ok(Json(ctx))
        }
        None => {
            // No valid session - return unauthenticated context
            Ok(Json(crate::auth::context::AuthContext::default()))
        }
    }
    .into_response()
}
```

**Key changes:**
1. **Return type:** `Result<Json<AuthContext>, (StatusCode, String)>` → `axum::response::Response`
2. **Parameter rename:** `_req` → `req` (now used for method check)
3. **Method guard:** Added at the top of the function body, before any session/DB logic
4. **405 response:** Returns `(StatusCode::METHOD_NOT_ALLOWED, headers, "Method Not Allowed").into_response()` — the tuple form `(Status, HeaderMap, Body)` is supported by axum's `IntoResponse` impl
5. **Success/error paths:** Wrapped in `.into_response()` at the end — `Result<Json<T>, (StatusCode, String)>` implements `IntoResponse`, so `.into_response()` converts it to `axum::response::Response`

**Note on `.into_response()` chain:** The `match` expression returns `Result<Json<AuthContext>, (StatusCode, String)>`. Both the `Ok(Json(...))` and `Err((StatusCode, String))` arms implement `IntoResponse`. Calling `.into_response()` on the `Result` converts it to `axum::response::Response`. This is the idiomatic axum pattern.

#### 2. No new imports needed

All required types are already available:
- `axum::http::Method` — re-exported by axum (already have `axum::http::Request` and `axum::http::StatusCode`)
- `axum::http::HeaderMap` — re-exported by axum
- `axum::http::header::ALLOW` — re-exported by axum
- `axum::response::Response` — used as return type (already available via axum)
- `axum::response::IntoResponse` — trait used by `.into_response()` (already in scope via axum prelude or auto-import)

The only import that may need explicit addition is `axum::response::IntoResponse` if it's not already in scope. However, since `axum::response::Response` is used as the return type and `.into_response()` is called on multiple types, the trait should be in scope. If compilation fails, add:
```rust
use axum::response::IntoResponse;
```

### Alternative: Minimal Return Type Change

If changing the return type to `Response` is considered too invasive, an alternative is to keep the existing return type and return a 405 without the `Allow` header:

```rust
if req.method() != axum::http::Method::GET {
    return Err((StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed".to_string()));
}
```

**Trade-off:** This omits the `Allow: GET` header, which is required by [RFC 7231 §6.5.5](https://httpwg.org/specs/rfc7231.html#status.405). The `Allow` header tells the client which methods are valid for the resource. Without it, the response is technically non-compliant.

**Recommendation:** Use the full `Response` return type approach to include the `Allow` header. The change is minimal and follows axum best practices.

### Test Additions

#### 1. Add test module to `src/auth/user_profile.rs`

**File:** `src/auth/user_profile.rs`
**Location:** After line 74 (end of file, after the `handle_user_profile` function)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body as AxumBody;
    use axum::http::Request;
    use axum::Router;
    use tower::ServiceExt;

    /// Build a minimal router exposing the user_profile handler for testing.
    /// Uses a real PgPool from pgtemp to satisfy State extraction.
    async fn make_router() -> Router {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        Router::new()
            .route(
                "/api/user_profile",
                axum::routing::get(handle_user_profile),
            )
            .with_state(UserProfileState { pool })
    }

    #[tokio::test]
    async fn user_profile_get_returns_200_unauthenticated() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Without a valid session cookie, should return 200 with unauthenticated context
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn user_profile_post_returns_405() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        // Verify Allow header is present and contains GET
        let allow_header = response
            .headers()
            .get(axum::http::header::ALLOW)
            .expect("Allow header should be present on 405 response");
        assert_eq!(allow_header, "GET");
    }

    #[tokio::test]
    async fn user_profile_put_returns_405() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let allow_header = response
            .headers()
            .get(axum::http::header::ALLOW)
            .expect("Allow header should be present on 405 response");
        assert_eq!(allow_header, "GET");
    }

    #[tokio::test]
    async fn user_profile_delete_returns_405() {
        let app = make_router().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/user_profile")
                    .body(AxumBody::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let allow_header = response
            .headers()
            .get(axum::http::header::ALLOW)
            .expect("Allow header should be present on 405 response");
        assert_eq!(allow_header, "GET");
    }
}
```

**Notes on test design:**
- Uses `crate::test_utils::setup_test_db()` to create a real PgPool — required because `State<UserProfileState>` extraction needs a valid pool instance.
- The `make_router()` function is async because `setup_test_db()` is async. Each test calls `make_router().await` to get a fresh router with its own temp database.
- Tests verify both the status code (405) and the `Allow` header value ("GET").
- The GET test verifies the handler still works for valid requests (returns 200 with unauthenticated context when no session cookie is present).
- Tests cover POST, PUT, and DELETE as representative non-GET methods.

**Module visibility consideration:** `crate::test_utils` is declared as `#[cfg(all(feature = "server", test))] mod test_utils;` in `src/main.rs`. The test module in `user_profile.rs` will only compile when the `server` feature is enabled. This is correct — the handler itself is server-only (it uses `PgPool` and `CookieJar` from axum-extra).

### Verification Checklist

- [ ] Non-GET requests (POST, PUT, DELETE, PATCH) return HTTP 405
- [ ] 405 response includes `Allow: GET` header (RFC 7231 §6.5.5 compliance)
- [ ] GET requests continue to work normally (no regression)
- [ ] Method check runs before session verification and database queries (short-circuit)
- [ ] No new crate dependencies required
- [ ] All existing tests continue to pass
- [ ] New tests cover POST, PUT, DELETE methods + GET baseline

### Implementation Order

1. Rename `_req` to `req` in `handle_user_profile` signature (line 30)
2. Change return type from `Result<Json<...>, (StatusCode, String)>` to `axum::response::Response` (line 31)
3. Add method check block at the start of the function body (after line 31, before existing session check)
4. Add `.into_response()` to the end of the match expression (after line 73)
5. Add `#[cfg(test)] mod tests` block at end of file
6. Run `cargo test --features server user_profile` to verify all 4 new tests pass
7. Run `cargo test --features server` to verify no regressions

### Dependencies / Build

- **No new crate dependencies.** All required types (`Method`, `HeaderMap`, `ALLOW`, `StatusCode::METHOD_NOT_ALLOWED`, `Response`, `IntoResponse`) are re-exported by axum, which is already a dependency.
- `tower::ServiceExt` is available via dev-dependency (`Cargo.toml:59`).
- `crate::test_utils` is available in test builds with `server` feature (`src/main.rs:12-13`).

### Risks / Trade-offs

- **Return type change:** Changing from `Result<Json<AuthContext>, (StatusCode, String)>` to `axum::response::Response` is a minor API change. The handler is only called from the router (no direct callers), so there's no external impact. The `Ok(Json(...))` and `Err((StatusCode, String))` arms both implement `IntoResponse`, so the conversion is seamless.
- **Router-level + handler-level redundancy:** The route is already `.get()`-only in `main.rs:90`. The handler-level check is redundant but intentional (defense-in-depth). Both layers will return 405 for non-GET requests, but the handler-level check produces a response with the `Allow` header, which is more informative.
- **Test database overhead:** Each test creates a temporary PostgreSQL database via `pgtemp`. This adds test startup time but ensures isolation. The existing codebase already uses this pattern (see `src/auth/oauth.rs` tests).

### Gaps / Follow-up

- None. This is a self-contained, single-handler change with no cross-module impact.
- **Observation:** The `_req: Request<Body>` parameter was already present but unused. This suggests the original developer anticipated needing the request object but never implemented the check. This fix completes that intent.

---

## Phase 0 (continued): Implementation Blueprint — AC9: Cookie Domain attribute

### Objective
Add support for a `COOKIE_DOMAIN` environment variable in `build_session_cookie` (and `clear_session_cookie` for consistency) in `src/auth/session.rs`. When set, the session cookie includes a `.domain(domain)` attribute, scoping the cookie to the specified domain and its subdomains. When not set, behavior is unchanged — no domain attribute is applied.

### Research Findings

| Item | Location | Details |
|------|----------|---------|
| `build_session_cookie` function | `src/auth/session.rs:138-147` | Builds session cookie using `CookieBuilder`. Already reads `NOMS_ENV` via `std::env::var` for Secure flag toggle. |
| `clear_session_cookie` function | `src/auth/session.rs:152-161` | Builds deletion cookie. **Must mirror `build_session_cookie` domain logic** — a cookie can only be cleared if the domain matches exactly. |
| `NOMS_ENV` env var pattern | `src/auth/session.rs:139,153` | `std::env::var("NOMS_ENV").ok() == Some("local".to_string())` — direct `std::env::var` call, no config struct. |
| `SESSION_SECRET` env var pattern | `src/auth/session.rs:64,72` | `std::env::var("SESSION_SECRET")` with `map_or` — same direct pattern. |
| `COOKIE_DOMAIN` in `.env.local.example` | N/A | **Not documented.** Needs to be added. |
| `COOKIE_DOMAIN` in `.env.local` | N/A | **Not set.** No existing usage. |
| `cookie` crate version | `Cargo.lock:1153` | `cookie = "0.18.1"` — `CookieBuilder::domain(&str)` is available. |
| `CookieBuilder::domain()` API | cookie crate docs | `pub fn domain<T: Into<String>>(&mut self, domain: T) -> &mut CookieBuilder` — returns `&mut CookieBuilder` for chaining. |
| Existing cookie attribute tests | `src/auth/session.rs:334-348` | `session_cookie_has_correct_attributes` — checks name, value, http_only, secure, path, max_age. Does NOT check domain (currently no domain). |
| Existing clear cookie tests | `src/auth/session.rs:350-360` | `clear_cookie_has_zero_max_age` — checks name, value, max_age, http_only, secure. |

### Design Decision: Env Var Reading Strategy

**Chosen approach:** Direct `std::env::var("COOKIE_DOMAIN").ok()` — same pattern as the existing `NOMS_ENV` check on line 139.

**Rationale:**
1. **Consistency:** The file already uses this pattern for `NOMS_ENV` (lines 139, 153) and `SESSION_SECRET` (lines 64, 72). No config struct or lazy initialization is used anywhere in this module.
2. **Simplicity:** `std::env::var().ok()` returns `Option<String>`, perfect for optional config. No allocation if not set.
3. **No caching needed:** The domain is read once per cookie build. Session cookies are created infrequently (login, refresh), so the overhead of `std::env::var` (which reads from the OS) is negligible.
4. **Testability:** Tests can set/unset the env var directly. The existing test infrastructure (`with_secret`, `without_secret`) already manipulates env vars.

### Changes Required

#### 1. Add domain logic to `build_session_cookie`

**File:** `src/auth/session.rs`
**Location:** Lines 138-147

**Before (current):**
```rust
pub fn build_session_cookie(token: &str) -> Cookie<'static> {
    let is_local = std::env::var("NOMS_ENV").ok() == Some("local".to_string());
    CookieBuilder::new(COOKIE_NAME, token.to_owned())
        .http_only(true)
        .secure(!is_local)
        .path("/")
        .max_age(TimeDuration::seconds(SESSION_LIFETIME_SECS as i64))
        .same_site(SameSite::Lax)
        .build()
}
```

**After:**
```rust
pub fn build_session_cookie(token: &str) -> Cookie<'static> {
    let is_local = std::env::var("NOMS_ENV").ok() == Some("local".to_string());
    let domain = std::env::var("COOKIE_DOMAIN").ok();
    let mut builder = CookieBuilder::new(COOKIE_NAME, token.to_owned())
        .http_only(true)
        .secure(!is_local)
        .path("/")
        .max_age(TimeDuration::seconds(SESSION_LIFETIME_SECS as i64))
        .same_site(SameSite::Lax);

    if let Some(d) = domain {
        builder = builder.domain(d);
    }

    builder.build()
}
```

**Key design notes:**
- The `CookieBuilder` is assigned to a `mut` binding so we can conditionally chain `.domain()`.
- `std::env::var("COOKIE_DOMAIN").ok()` returns `Option<String>` — `None` if not set, `Some(value)` if set.
- The `.domain()` method takes any `Into<String>`, so `String` from `std::env::var` works directly.
- No validation of the domain string is performed — the `cookie` crate handles malformed domains gracefully (the browser will simply ignore an invalid domain attribute).

#### 2. Add domain logic to `clear_session_cookie`

**File:** `src/auth/session.rs`
**Location:** Lines 152-161

**Before (current):**
```rust
pub fn clear_session_cookie() -> Cookie<'static> {
    let is_local = std::env::var("NOMS_ENV").ok() == Some("local".to_string());
    CookieBuilder::new(COOKIE_NAME, "")
        .http_only(true)
        .secure(!is_local)
        .path("/")
        .max_age(TimeDuration::ZERO)
        .same_site(SameSite::Lax)
        .build()
}
```

**After:**
```rust
pub fn clear_session_cookie() -> Cookie<'static> {
    let is_local = std::env::var("NOMS_ENV").ok() == Some("local".to_string());
    let domain = std::env::var("COOKIE_DOMAIN").ok();
    let mut builder = CookieBuilder::new(COOKIE_NAME, "")
        .http_only(true)
        .secure(!is_local)
        .path("/")
        .max_age(TimeDuration::ZERO)
        .same_site(SameSite::Lax);

    if let Some(d) = domain {
        builder = builder.domain(d);
    }

    builder.build()
}
```

**Why this must change:** Per RFC 6265 §5.3, a cookie can only be deleted if the Set-Cookie header matches the original cookie's name, path, and domain exactly. If `build_session_cookie` sets a domain, `clear_session_cookie` must set the same domain, or the browser will not delete the cookie.

#### 3. Add tests for domain attribute

**File:** `src/auth/session.rs`
**Location:** After line 360 (after `clear_cookie_has_zero_max_age`, before `fresh_token_does_not_need_refresh`)

Add two test functions:

```rust
#[test]
fn session_cookie_includes_domain_when_set() {
    with_secret(TEST_SECRET);
    std::env::set_var("COOKIE_DOMAIN", ".example.com");
    let token = create_session(test_user_id()).unwrap();
    let cookie = build_session_cookie(&token);

    assert_eq!(cookie.domain(), Some(&".example.com"));
    std::env::remove_var("COOKIE_DOMAIN");
}

#[test]
fn session_cookie_has_no_domain_when_unset() {
    with_secret(TEST_SECRET);
    std::env::remove_var("COOKIE_DOMAIN");
    let token = create_session(test_user_id()).unwrap();
    let cookie = build_session_cookie(&token);

    assert_eq!(cookie.domain(), None);
}
```

**Test design notes:**
- `session_cookie_includes_domain_when_set`: Sets `COOKIE_DOMAIN`, builds a cookie, verifies `cookie.domain()` returns `Some(".example.com")`. Cleans up env var after assertion.
- `session_cookie_has_no_domain_when_unset`: Ensures `COOKIE_DOMAIN` is removed, builds a cookie, verifies `cookie.domain()` returns `None`. This confirms backward compatibility.
- Both tests use `with_secret(TEST_SECRET)` to ensure `create_session` works.
- The `cookie.domain()` method on the `cookie` crate's `Cookie` type returns `Option<&str>`.

#### 4. Document `COOKIE_DOMAIN` in `.env.local.example`

**File:** `.env.local.example`
**Location:** After line 40 (after `SESSION_SECRET=...`, at end of file)

Add:
```ini
# Cookie domain (optional)
# Set to restrict the session cookie to a specific domain and its subdomains.
# Example: .example.com (note the leading dot for subdomain inclusion)
# Leave empty or unset to use the default behavior (cookie scoped to current host)
# COOKIE_DOMAIN=.example.com
```

### Verification Checklist

- [ ] `build_session_cookie` includes `.domain(d)` when `COOKIE_DOMAIN` is set
- [ ] `build_session_cookie` omits domain when `COOKIE_DOMAIN` is not set (backward compatible)
- [ ] `clear_session_cookie` mirrors the same domain logic (required for cookie deletion to work)
- [ ] Existing cookie attribute tests continue to pass (no regression)
- [ ] New tests verify both set and unset states
- [ ] `.env.local.example` documents the new env var with example and explanation
- [ ] No new crate dependencies required

### Implementation Order

1. Modify `build_session_cookie` (lines 138-147): add `domain` variable, refactor to mutable builder, conditional `.domain()` chain.
2. Modify `clear_session_cookie` (lines 152-161): same pattern as step 1.
3. Add two test functions after `clear_cookie_has_zero_max_age` (after line 360).
4. Append `COOKIE_DOMAIN` documentation to `.env.local.example` (after line 40).
5. Run `cargo test --features server session` to verify all session tests pass (existing + new).
6. Run `cargo test --features server` to verify no regressions.

### Dependencies / Build

- **No new crate dependencies.** The `cookie` crate (v0.18.1, already in `Cargo.toml:27`) provides `CookieBuilder::domain()`.
- `std::env::var` is from the standard library — no external dependency.
- All existing imports (`Cookie`, `CookieBuilder`, `SameSite`) remain unchanged.

### Risks / Trade-offs

- **No domain validation:** The implementation does not validate the domain string format. If a user sets `COOKIE_DOMAIN` to an invalid domain (e.g., missing dots for non-local domains), the `cookie` crate will still set the attribute, but the browser may ignore it. This is acceptable — the env var is an operator-controlled configuration, and invalid values will simply result in the cookie behaving as if no domain was set (browser-level fallback). Adding validation would increase complexity without meaningful security benefit.
- **Domain prefix convention:** The documentation recommends a leading dot (`.example.com`) for subdomain inclusion per RFC 6265 §5.2.3. However, modern browsers (Chrome 52+, Firefox 52+) ignore the leading dot and treat `example.com` and `.example.com` identically. The documentation notes this convention but does not enforce it.
- **Environment variable mutation in tests:** The tests use `std::env::set_var` and `std::env::remove_var` directly. Since Rust tests run in separate threads and env vars are process-global, there's a theoretical race condition if tests run in parallel. However, the existing test suite already uses this pattern (see `without_secret()` on line 258-262), and the test functions are not marked `#[parallel]`. This is consistent with existing practice.
- **Cookie clearing with mismatched domain:** If `COOKIE_DOMAIN` is set at login time but unset at logout time (or vice versa), the clear cookie will have a different domain and the browser will not delete the session cookie. This is a configuration consistency issue, not a code bug. The fix ensures both functions read the same env var, so as long as the env var is consistent across the application lifecycle, this cannot occur.

### Gaps / Follow-up

- None. This is a self-contained, config-driven change with no cross-module impact.
- **Observation:** The existing `session_cookie_has_correct_attributes` test (line 334) does not assert on `cookie.domain()`. This is correct — the test runs without `COOKIE_DOMAIN` set, so `domain()` returns `None`. The new test `session_cookie_has_no_domain_when_unset` explicitly verifies this behavior.

---

## Phase 1: Implementation Details

### Summary
Added redirect URI length validation (max 2048 bytes) to `validate_redirect_uri` in `src/auth/oauth.rs`. Over-length URIs are rejected with `OAuthError::InvalidRedirectUri`, which already maps to HTTP 400.

### Changes Made

**`src/auth/oauth.rs`:**
1. **Added constant** `REDIRECT_URI_MAX_LEN: usize = 2048` after `CSRF_STATE_TTL_SECS` (line 195).
2. **Added length check** as the first condition in `validate_redirect_uri` — a cheap `usize` comparison that rejects oversized input before any string operations.
3. **Added 2 test functions:**
   - `test_validate_redirect_uri_too_long` — 2049-char URI ("/" + 2048 "a"s) is rejected with `InvalidRedirectUri`.
   - `test_validate_redirect_uri_at_max_length` — 2048-char URI ("/" + 2047 "a"s) is accepted.

### Test Results
- All 6 redirect URI tests pass (4 existing + 2 new).
- Full test suite: **88 passed, 0 failed** (`cargo test --features server`).

### Verification
- Build: compiles without errors or warnings.
- Tests: `cargo test --features server validate_redirect_uri` — 6/6 pass.
- Full suite: `cargo test --features server` — 88/88 pass.

### Notes
- `Display` impl is **unchanged** — it continues to produce detailed messages for logging purposes.
- No new crate dependencies (`tracing` already in `Cargo.toml`).
- Generic message string is inlined in both `oauth.rs` and `user_profile.rs` (minor duplication, acceptable for short self-documenting string).

---

### AC8: Method Enforcement on user_profile Endpoint

**Summary**
Added handler-level HTTP method enforcement to `handle_user_profile` in `src/auth/user_profile.rs`. Non-GET requests are rejected with `405 Method Not Allowed` and an `Allow: GET` header, providing defense-in-depth beyond the existing router-level `.get()` binding.

**Changes Made**

**`src/auth/user_profile.rs`:**
1. **Added imports:** `axum::http::header`, `axum::http::HeaderMap`, `axum::http::Method`, `axum::response::IntoResponse`.
2. **Changed return type:** `Result<Json<AuthContext>, (StatusCode, String)>` → `axum::response::Response` to support custom headers on the 405 response.
3. **Renamed parameter:** `_req` → `req` (now actively used for method check).
4. **Added method guard** at the top of the function body: checks `req.method() != Method::GET`, returns `(StatusCode::METHOD_NOT_ALLOWED, headers, "Method Not Allowed").into_response()` with `Allow: GET` header.
5. **Restructured error handling:** Replaced `.map_err()?` and `.ok_or()?` with nested `match` on `get_user_by_id` result, since `?` cannot be used with `Response` return type.
6. **Added `.into_response()`** at the end of the outer match expression to convert `Result<Json<AuthContext>, (StatusCode, String)>` to `axum::response::Response`.

**Tests added** (in `src/auth/user_profile.rs` test module):
- `user_profile_get_returns_200_unauthenticated` — GET baseline, returns 200 with unauthenticated context when no session cookie is present.
- `user_profile_post_returns_405` — POST rejected with `Allow: GET` header.
- `user_profile_put_returns_405` — PUT rejected with `Allow: GET` header.
- `user_profile_delete_returns_405` — DELETE rejected with `Allow: GET` header.

Test router registers handler for all methods (GET, POST, PUT, DELETE) to bypass the router-level method filter and exercise the handler-level check directly.

**Test Results**
- All 4 new tests pass.
- Full test suite: **95 passed, 0 failed** (`cargo test --features server`).

**Verification**
- Build: compiles without errors or warnings.
- Formatting: `cargo fmt` applied, `cargo fmt --check` clean.
- Tests: `cargo test --features server user_profile` — 7/7 pass (4 new + 3 existing db tests matching the filter).
- Full suite: `cargo test --features server` — 95/95 pass.

**Notes**
- No new crate dependencies (`Method`, `HeaderMap`, `ALLOW`, `StatusCode::METHOD_NOT_ALLOWED`, `Response`, `IntoResponse` are all re-exported by axum).
- The `?` operator is incompatible with `Response` return type; replaced with explicit nested `match` for the DB query result.
- Router registers all methods in tests to reach the handler-level check; in production, the route is `.get()`-only in `main.rs`, so both layers enforce GET-only.

---

### AC9: Cookie Domain attribute

**Summary**
Added support for a `COOKIE_DOMAIN` environment variable in `build_session_cookie` and `clear_session_cookie` in `src/auth/session.rs`. When set, the session cookie includes a `.domain(domain)` attribute, scoping the cookie to the specified domain and its subdomains. When not set, behavior is unchanged — no domain attribute is applied (backward compatible).

**Changes Made**

**`src/auth/session.rs`:**
1. **`build_session_cookie`:** Refactored from a single-expression chain to a mutable `CookieBuilder`. Domain is read via `read_cookie_domain()` helper, conditionally chains `.domain(d)` if set.
2. **`clear_session_cookie`:** Same pattern — reads domain via `read_cookie_domain()` and conditionally adds `.domain(d)`. Critical for RFC 6265 cookie deletion: the domain must match exactly between set and clear cookies.
3. **Added `read_cookie_domain()` helper** (dual `#[cfg(test)]`/`#[cfg(not(test))]` variants): In test builds, checks `TEST_COOKIE_DOMAIN` thread-local first, then falls back to `std::env::var("COOKIE_DOMAIN")`. Both variants filter out empty and whitespace-only values via `.filter(|d| !d.trim().is_empty())`.
4. **Added `TEST_COOKIE_DOMAIN` thread-local** (matching existing `TEST_SECRET` pattern): `thread_local! { static TEST_COOKIE_DOMAIN: std::cell::RefCell<Option<String>> }` — eliminates race condition when domain tests run in parallel.
5. **Added 6 test functions** (after `clear_cookie_has_zero_max_age`):
   - `session_cookie_includes_domain_when_set` — uses thread-local to set `.example.com`, verifies `cookie.domain() == Some("example.com")`.
   - `session_cookie_has_no_domain_when_unset` — clears thread-local, verifies `cookie.domain() == None`.
   - `clear_cookie_includes_domain_when_set` — verifies `clear_session_cookie` also picks up domain from thread-local, plus `max_age == ZERO`.
   - `clear_cookie_has_no_domain_when_unset` — verifies `clear_session_cookie` has no domain when unset.
   - `cookie_domain_empty_string_is_treated_as_unset` — empty string `""` is filtered out, cookie has no domain.
   - `cookie_domain_whitespace_only_is_treated_as_unset` — whitespace-only `"   "` is filtered out, cookie has no domain.
6. **Added `with_cookie_domain()` and `without_cookie_domain()` test helpers** — mirror the existing `with_secret()`/`without_secret()` pattern.

**`.env.local.example`:**
7. **Appended `COOKIE_DOMAIN` documentation** after `SESSION_SECRET` section: describes purpose, format (leading dot convention), and default behavior when unset.

**Review Fixes Applied (Phase 2 → Fix 4/4):**
- **BLOCKER (test race condition):** Replaced `std::env::set_var("COOKIE_DOMAIN", ...)` in tests with `thread_local!` pattern (`TEST_COOKIE_DOMAIN`), matching the existing `TEST_SECRET` approach. Both domain tests now run safely in parallel.
- **WARNING (no clear_session_cookie domain test):** Added `clear_cookie_includes_domain_when_set` and `clear_cookie_has_no_domain_when_unset` tests.
- **SUGGESTION (empty string edge case):** Added `.filter(|d| !d.trim().is_empty())` to `read_cookie_domain()` in both `#[cfg(test)]` and `#[cfg(not(test))]` variants. Empty and whitespace-only values are treated as unset.

**Test Results**
- All 6 new tests pass.
- All 25 session tests pass (19 existing + 6 new).
- Full test suite: **101 passed, 0 failed** (`cargo test --features server`), both parallel and single-threaded.

**Verification**
- Build: compiles without errors or warnings.
- Formatting: `cargo fmt --check` clean.
- Tests parallel: `cargo test --features server` — 101/101 pass.
- Tests single-threaded: `cargo test --features server -- --test-threads=1` — 101/101 pass.

**Notes**
- No new crate dependencies (`CookieBuilder::domain()` is available in `cookie` v0.18.1, already in `Cargo.toml`).
- The `cookie` crate strips the leading dot from domain values (`.example.com` → `example.com`), which is RFC 6265 compliant. Tests account for this normalization.
- Both `build_session_cookie` and `clear_session_cookie` read the same env var (via `read_cookie_domain()`), ensuring domain consistency between cookie creation and deletion.
- Thread-local isolation eliminates the need for `serial_test` dependency — consistent with the existing `TEST_SECRET` pattern in the same file.

## Phase 2: Review Verdict

**Verdict: PASS**

### Requirements Coverage

| Requirement | Status |
|---|---|
| Max 2048-char redirect URI validation | ✅ Implemented via `REDIRECT_URI_MAX_LEN` constant + `uri.len()` check |
| Rejected URIs return HTTP 400 | ✅ `InvalidRedirectUri` already maps to `StatusCode::BAD_REQUEST` (line 101) |
| Length check runs before other validations | ✅ First condition in `validate_redirect_uri` (line 131) |
| Boundary tests (2048 OK, 2049 rejected) | ✅ Two dedicated tests cover both boundaries |

### Issues

None. No blockers, warnings, or suggestions.

### Positive Findings

1. **Correct check ordering** — The `uri.len() > REDIRECT_URI_MAX_LEN` guard is the first condition in `validate_redirect_uri` (line 131), avoiding unnecessary string operations (`starts_with`, `contains`) on oversized input. This is the right defense-in-depth pattern.

2. **Conservative byte-length semantics** — `.len()` on `&str` returns byte count, not character count. Since UTF-8 bytes ≥ characters, this rejects non-ASCII input earlier than a character-count check would. This is the desired security posture and is well-documented in the Phase 0 brief.

3. **Clean constant placement** — `REDIRECT_URI_MAX_LEN` (line 198) sits next to `CSRF_STATE_TTL_SECS` (line 195), keeping configuration constants grouped logically.

4. **Test coverage is solid** — 6/6 `validate_redirect_uri` tests pass, including:
   - `test_validate_redirect_uri_at_max_length`: exactly 2048 bytes ("/" + 2047 "a"s) → **accepted** ✅
   - `test_validate_redirect_uri_too_long`: 2049 bytes ("/" + 2048 "a"s) → **rejected** ✅
   - 4 pre-existing tests for format validation remain passing.

5. **No regressions** — The full test suite (88 tests) passes with zero failures.

### Edge Case Analysis

- **Empty string**: Length 0 passes the length check, then correctly fails `starts_with('/')`. ✅
- **Single "/"**: Length 1, passes all checks. Valid. ✅
- **Non-ASCII characters**: Byte-length check is conservative (rejects earlier). ✅
- **Error response body size**: The full oversized URI is included in the 400 response body via `InvalidRedirectUri(uri.to_string())`. This is noted in Phase 0 as acceptable — the client is at fault, and the string is already in memory from deserialization. No additional allocation risk. ✅
- **Single call site**: `validate_redirect_uri` is called only from `start_handler` (line 213). No other entry point bypasses this check. ✅

### Summary

Clean, minimal, correct implementation. The length guard is well-placed, the constant is reasonable, and the tests cover the critical boundaries. No changes needed.

---

### AC4: Error Message Sanitization

**Verdict: PASS**

#### Requirements Coverage

| Requirement | Status |
|---|---|
| All 5 server error variants return generic message | ✅ `sanitized_message()` returns generic string for TokenExchange, UserInfoExtraction, DbError, SessionError, LinkError |
| Client error variants return detailed messages | ✅ InvalidProvider, InvalidRedirectUri, StateNotFound, StateExpired, ProviderMismatch all return `self.to_string()` |
| Detailed error logged via tracing before sanitization | ✅ `tracing::error!` for 5xx, `tracing::warn!` for 4xx, both with `%self` (Display) formatting |
| `Display` impl preserved for logging | ✅ Unchanged — still produces detailed messages (verified by `test_display_still_detailed_for_logging`) |
| `user_profile.rs` DB error sanitized | ✅ `.map_err()` logs via `tracing::error!` and returns generic message |
| No new crate dependencies | ✅ `tracing` already in `Cargo.toml` |

#### Issues

1. **SUGGESTION — Generic message string duplicated across two files**
   - **Location:** `src/auth/oauth.rs:119` and `src/auth/user_profile.rs:46`
   - **Description:** The string `"An internal error occurred. Please try again later."` is inlined in both files. If the message text ever needs to change, both locations must be updated.
   - **Recommended fix:** Define a shared constant (e.g., `const INTERNAL_ERROR_MSG: &str = "An internal error occurred. Please try again later.";`) in a common module and reference it from both files. Low priority — the string is short and self-documenting.

2. **SUGGESTION — `StateNotFound`/`StateExpired` warn-level logging may be noisy**
   - **Location:** `src/auth/oauth.rs:144`
   - **Description:** `StateNotFound` and `StateExpired` are logged at `warn!` level. During normal OAuth flows (e.g., user opens stale link, refreshes callback page), these can occur frequently and generate log noise.
   - **Recommended fix:** Consider downgrading these two specific variants to `tracing::debug!` in a follow-up if log volume becomes an issue. Not a blocker — `warn!` is reasonable for initial deployment.

#### Positive Findings

1. **Clean two-tier architecture** — The `sanitized_message()` method cleanly separates client-facing messages from log-facing messages. The `IntoResponse` impl delegates to it, keeping the separation of concerns clear.

2. **Correct logging before sanitization** — The `tracing::error!`/`tracing::warn!` calls happen *before* `self.sanitized_message()` is called, ensuring the detailed error is always logged regardless of what the client receives. The `%self` formatting correctly invokes the `Display` impl.

3. **Appropriate log level split** — 5xx errors use `tracing::error!` (unexpected failures requiring attention), 4xx errors use `tracing::warn!` (expected client mistakes). Both include structured fields (`error`, `status`) for log filtering.

4. **`user_profile.rs` fix is consistent** — The `.map_err()` pattern in `user_profile.rs` mirrors the `oauth.rs` approach: log detailed error, return generic message. Uses the same generic message string for consistency.

5. **Test coverage is well-structured** — Three tests cover the key invariants:
   - `test_sanitized_message_client_errors_preserved`: all 5 client variants return detailed messages ✅
   - `test_sanitized_message_server_errors_generic`: all 5 server variants return generic message ✅
   - `test_display_still_detailed_for_logging`: `Display` and `sanitized_message()` produce different output for the same error ✅

6. **No regressions** — Full test suite: **91 passed, 0 failed** (`cargo test --features server`).

#### Edge Case Analysis

- **`OAuthError::SessionError("SESSION_SECRET not set")`**: The sanitized response correctly hides this configuration detail from the client. The `tracing::error!` log preserves it for server-side debugging. ✅
- **`OAuthError::DbError` wrapping SQL details**: PostgreSQL error messages (e.g., relation names, constraint violations) are fully contained in the log and never reach the client. ✅
- **`OAuthError::TokenExchange` wrapping OAuth provider responses**: Provider API error details (e.g., Google/GitHub HTTP error bodies) are hidden from the client. ✅
- **`user_profile.rs` "User not found" (404)**: This is a legitimate user-facing message — not an internal error — and is correctly left unsanitized. No `tracing` call needed here. ✅
- **`SessionError` used outside OAuth context**: In `user_profile.rs`, `session::verify_session()` errors are handled via `.ok()` (silently returning `None`), not via `OAuthError`. No sanitization needed — the handler returns an unauthenticated context (200 OK with empty profile). ✅
- **No other error paths bypass sanitization**: Verified all `StatusCode::INTERNAL_SERVER_ERROR` usages in the codebase (6 total: 5 in `oauth.rs` `IntoResponse`, 1 in `user_profile.rs`). All are covered. ✅
- **`LinkError::Db(DbError)` chain**: The inner `DbError` details flow through `LinkError::Display` → `OAuthError::LinkError` → sanitized in `IntoResponse`. The full chain is logged via `%self`. ✅

#### Summary

Well-designed, thorough implementation. The two-tier message strategy (`sanitized_message` vs `Display`) cleanly separates client-facing and log-facing concerns. All error paths are covered, logging is correct, and test coverage validates the key invariants. No changes needed.

### AC8: Method Enforcement on user_profile Endpoint

**Verdict: PASS**

#### Requirements Coverage

| Requirement | Status |
|---|---|
| Non-GET methods (POST, PUT, DELETE) rejected with 405 | ✅ `req.method() != Method::GET` guard at line 37, returns `StatusCode::METHOD_NOT_ALLOWED` |
| 405 response includes `Allow: GET` header | ✅ `HeaderMap` with `header::ALLOW` inserted at line 38-42, verified in all 3 rejection tests |
| GET still works normally (returns 200 with JSON) | ✅ `user_profile_get_returns_200_unauthenticated` passes; success path (lines 56-93) unchanged |
| Method check runs BEFORE session/DB logic | ✅ Method guard (lines 37-49) is the first block in the function body, before session check (line 52) and DB query (line 58) |
| Tests cover multiple methods + verify Allow header | ✅ 4 tests: GET→200, POST→405+Allow, PUT→405+Allow, DELETE→405+Allow |
| No panics, proper response conversion | ✅ `.parse().expect()` on `"GET"` is provably safe; `into_response()` correctly converts all response types |

#### Issues

None. No blockers, warnings, or suggestions.

#### Positive Findings

1. **Correct short-circuit ordering** — The method check (line 37) is the very first operation in the handler body, before the session cookie extraction (line 52) and database query (line 58). A malicious POST/PUT/DELETE request is rejected with zero session validation overhead and zero database round-trips.

2. **RFC 7231 §6.5.5 compliance** — The 405 response includes the `Allow: GET` header (lines 38-42), which is required by the HTTP specification. This tells the client exactly which methods are valid for the resource, enabling proper client-side retry logic.

3. **Clean return type migration** — Changing from `Result<Json<AuthContext>, (StatusCode, String)>` to `axum::response::Response` is the right call. It enables attaching custom headers to the 405 response without introducing a custom error type. The `.into_response()` call on line 93 cleanly converts the existing `Result` pattern to the new return type.

4. **Test router design is intentional and correct** — The test router (lines 110-117) registers `.get().post().put().delete()` to deliberately bypass the router-level `.get()` filter. This is the correct approach for testing handler-level defense-in-depth: it exercises the method check in isolation, proving the handler protects itself regardless of how the route is registered.

5. **No new dependencies** — All types (`Method`, `HeaderMap`, `header::ALLOW`, `StatusCode::METHOD_NOT_ALLOWED`, `Response`, `IntoResponse`) are re-exported by axum, which is already a dependency. Zero `Cargo.toml` changes.

6. **Safe `expect()` usage** — The `"GET".parse().expect("valid Allow header value")` on line 41 can never fail at runtime. `"GET"` is a valid HTTP token and always parses successfully as a `HeaderValue`. This is idiomatic Rust for compile-time-known-valid values.

7. **Consistent error handling pattern** — The DB error path (lines 79-85) correctly uses `tracing::error!` with detailed error logging and returns a generic user-facing message, matching the AC4 sanitization pattern applied to `oauth.rs`.

#### Edge Case Analysis

- **`expect()` on `"GET"` parse**: `"GET"` is a valid HTTP token per RFC 7230 §3.2.6. The `HeaderValue::from_str` parse will never fail for this literal. No panic risk. ✅
- **Router-level + handler-level both return 405**: In production (`main.rs:90`), the route is `.get()`-only, so Axum's router rejects non-GET methods before reaching the handler. The handler-level check is defense-in-depth: if someone later changes the route registration (e.g., adds `.post()` or switches to `.route()`), the handler still protects itself. ✅
- **Authenticated GET path unchanged**: The `match verified_user_id` block (lines 56-92) is structurally identical to the original implementation, only the error handling was restructured from `?` operator to explicit `match` (required because `Response` return type is incompatible with `?`). The logic flow is preserved. ✅
- **Response body format on 405**: The 405 body is plain text (`"Method Not Allowed"`), not JSON. This is technically fine — the HTTP spec does not mandate a body format for 405 responses. The primary information is in the status code and `Allow` header. Minor stylistic inconsistency with the rest of the API's JSON responses, but not a functional issue. ✅
- **PATCH, HEAD, OPTIONS not tested**: The tests cover POST, PUT, DELETE as representative non-GET methods. The guard (`!= Method::GET`) catches ALL non-GET methods, so PATCH, HEAD, OPTIONS, etc. would also be correctly rejected. Testing the three most common "dangerous" methods is sufficient. ✅

#### Summary

Clean, minimal, correct implementation. The method guard is well-placed (first in function body), the response is RFC-compliant (includes `Allow` header), and the test strategy (bypassing router-level filter to exercise handler-level check) is sound. No changes needed.

---

### AC9: Cookie Domain attribute — Fix 4/4 Re-review

**Verdict: PASS**

#### Requirements Coverage

| Requirement | Status |
|---|---|
| `build_session_cookie` reads `COOKIE_DOMAIN` and conditionally adds `.domain()` | ✅ `read_cookie_domain()` helper at line 164, conditional `.domain(d)` at lines 172-174 |
| `clear_session_cookie` also reads `COOKIE_DOMAIN` | ✅ Same `read_cookie_domain()` helper at line 184, conditional `.domain(d)` at lines 192-194 — RFC 6265 cookie deletion consistency |
| Behavior unchanged when `COOKIE_DOMAIN` not set (backward compat) | ✅ `Option::None` path skips `.domain()` — cookie has no domain attribute, same as before |
| `.env.local.example` documented | ✅ Lines 42-46: purpose, format, leading dot convention, and default behavior all documented |
| Tests: domain set when env var present | ✅ `session_cookie_includes_domain_when_set` — uses thread-local `.example.com`, asserts `Some("example.com")` |
| Tests: no domain when env var absent | ✅ `session_cookie_has_no_domain_when_unset` — thread-local cleared, asserts `None` |
| Tests: `clear_session_cookie` domain behavior | ✅ `clear_cookie_includes_domain_when_set` + `clear_cookie_has_no_domain_when_unset` |
| Tests: empty/whitespace edge cases | ✅ `cookie_domain_empty_string_is_treated_as_unset` + `cookie_domain_whitespace_only_is_treated_as_unset` |
| Tests: parallel execution safety | ✅ All 101 tests pass in parallel mode (no race conditions) |

#### Issues

All three issues from the previous review have been resolved:

1. **BLOCKER (FIXED) — Test race condition** → Resolved via `TEST_COOKIE_DOMAIN` thread-local (line 62-64) + `read_cookie_domain()` dual `#[cfg(test)]`/`#[cfg(not(test))]` variants (lines 82-97). Tests use `with_cookie_domain()`/`without_cookie_domain()` helpers (lines 399-411) instead of direct `std::env::set_var`. Verified: 101/101 pass in parallel mode.

2. **WARNING (FIXED) — No `clear_session_cookie` domain test** → Two new tests added: `clear_cookie_includes_domain_when_set` (lines 435-443) and `clear_cookie_has_no_domain_when_unset` (lines 446-453). Both verify domain attribute AND `max_age == ZERO`.

3. **SUGGESTION (FIXED) — Empty string edge case** → Both `read_cookie_domain()` variants now apply `.filter(|d| !d.trim().is_empty())` (production: line 86, test: lines 92, 96). Two edge case tests verify empty string and whitespace-only values are treated as unset.

#### Positive Findings

1. **Thread-local pattern is correct and consistent** — `TEST_COOKIE_DOMAIN` (line 62-64) mirrors the existing `TEST_SECRET` pattern (line 57-59) exactly. The `read_cookie_domain()` test variant checks thread-local first, then falls back to `std::env::var`, matching the `read_secret()` pattern. This ensures full parallel test safety without adding any dependencies.

2. **`read_cookie_domain()` helper eliminates duplication** — Both `build_session_cookie` and `clear_session_cookie` call the same helper function, ensuring they always read the domain identically. This is cleaner than the inline `std::env::var` calls from the original blueprint, and makes the empty/whitespace filtering a single point of change.

3. **Dual `#[cfg(test)]`/`#[cfg(not(test))]` variants** — The production variant (lines 82-87) is a simple one-liner: `std::env::var("COOKIE_DOMAIN").ok().filter(...)`. The test variant (lines 89-97) adds thread-local precedence. Both share the same filtering logic. Zero runtime cost in production builds.

4. **Test helpers follow established conventions** — `with_cookie_domain()` and `without_cookie_domain()` mirror `with_secret()`/`without_secret()` in naming, signature, and behavior. `without_cookie_domain()` correctly clears both the thread-local AND the env var, matching `without_secret()`.

5. **Cleanup discipline in tests** — Tests that set a domain (`session_cookie_includes_domain_when_set`, `clear_cookie_includes_domain_when_set`, `cookie_domain_empty_string_is_treated_as_unset`, `cookie_domain_whitespace_only_is_treated_as_unset`) all call `without_cookie_domain()` at the end. This prevents test pollution even though thread-local isolation makes it technically unnecessary — good defensive practice.

6. **No new dependencies** — `CookieBuilder::domain()` is available in `cookie` v0.18.1. `thread_local!` and `RefCell` are standard library. Zero `Cargo.toml` changes.

#### Edge Case Analysis

- **`COOKIE_DOMAIN` not set**: `read_cookie_domain()` returns `None`, `.domain()` never called. Identical to pre-fix behavior. ✅
- **`COOKIE_DOMAIN=.example.com`**: Domain set, cookie crate normalizes to `example.com`. Both build and clear cookies match. ✅
- **`COOKIE_DOMAIN=""` (empty string)**: `.filter(|d| !d.trim().is_empty())` returns `None`. Treated as unset. ✅ (FIXED)
- **`COOKIE_DOMAIN="   "` (whitespace-only)**: Same filter catches this. Treated as unset. ✅ (FIXED)
- **`COOKIE_DOMAIN=example.com` (no leading dot)**: Works correctly. Cookie crate stores as-is. ✅
- **`COOKIE_DOMAIN=localhost`**: Sets `Domain=localhost`. Acceptable for local development. ✅
- **Domain mismatch between build and clear**: Both functions call the same `read_cookie_domain()` helper at call time. Impossible to mismatch as long as the env var/thread-local is consistent. ✅
- **Parallel test execution**: Thread-local isolation ensures no cross-thread pollution. 101/101 pass in both parallel and single-threaded modes. ✅ (FIXED)

#### Summary

All three issues from the initial review (BLOCKER, WARNING, SUGGESTION) are fully resolved. The thread-local pattern correctly eliminates the parallel test race condition, `clear_session_cookie` now has dedicated domain tests, and empty/whitespace values are properly filtered. The `read_cookie_domain()` helper is a clean abstraction that eliminates duplication between the two cookie-building functions. 101/101 tests pass in both parallel and single-threaded modes. Build is clean with no warnings.

## Phase 3: Synthesis

### User-Facing Summary

This release delivers four security hardening fixes from **NOMS-006 (Auth Security Hardening)**, each following the full architect → implement → review workflow. All four fixes passed review with no remaining blockers. The full test suite passes with **101 tests, 0 failures**.

| Fix | Acceptance Criteria | What Changed |
|-----|---------------------|--------------|
| **AC7** | Redirect URI max 2048 chars, 400 for over-length | Length guard added to `validate_redirect_uri` in `oauth.rs` |
| **AC4** | Generic messages for 5xx, detailed for 4xx, tracing preserved | `sanitized_message()` method + `IntoResponse` rewrite in `oauth.rs`; DB error sanitization in `user_profile.rs` |
| **AC8** | Reject non-GET on `/api/user_profile` with 405 + `Allow: GET` | Handler-level method guard in `user_profile.rs`; return type changed to `Response` |
| **AC9** | Cookie domain from `COOKIE_DOMAIN` env var, thread-safe tests | Conditional `.domain()` in `session.rs`; `read_cookie_domain()` helper; `TEST_COOKIE_DOMAIN` thread-local |

---

### Detailed Change Walkthrough

#### File 1: `src/auth/oauth.rs` — AC7 (Redirect URI Length) + AC4 (Error Sanitization)

**AC7 — Redirect URI Length Validation**

- **Constant added** (line 235): `const REDIRECT_URI_MAX_LEN: usize = 2048` — placed next to `CSRF_STATE_TTL_SECS` (line 232) for logical grouping of configuration constants.
- **Length check added** (line 168): `if uri.len() > REDIRECT_URI_MAX_LEN` is the **first** condition in `validate_redirect_uri`, before any string operations (`starts_with`, `contains`). This is intentional: a `usize` comparison is cheaper than string scanning, and it rejects maliciously long inputs before they can cause excessive CPU or memory usage.
- **Byte-length semantics**: `.len()` on `&str` returns byte count, not character count. Since UTF-8 bytes ≥ characters, this is conservative — non-ASCII input is rejected even earlier. This is the desired security posture.
- **Error mapping**: `OAuthError::InvalidRedirectUri` already maps to `StatusCode::BAD_REQUEST` (400) in the `IntoResponse` impl (line 129). No change needed.
- **Tests** (lines 507-520): Two boundary tests — 2049 bytes rejected, exactly 2048 bytes accepted. Combined with 4 pre-existing format tests, all 6 `validate_redirect_uri` tests pass.

**AC4 — Error Message Sanitization**

- **`sanitized_message()` method added** (lines 104-122): Two-tier message strategy. Client errors (4xx: `InvalidProvider`, `InvalidRedirectUri`, `StateNotFound`, `StateExpired`, `ProviderMismatch`) return their detailed `Display` message. Server errors (5xx: `TokenExchange`, `UserInfoExtraction`, `DbError`, `SessionError`, `LinkError`) return a generic `"An internal error occurred. Please try again later."` string.
- **`IntoResponse` impl rewritten** (lines 125-149): Now calls `tracing::error!` for 5xx and `tracing::warn!` for 4xx (both with `%self` Display formatting for structured logs), then delegates to `sanitized_message()` for the client-facing body. The `Display` impl (lines 81-95) is **unchanged** — it still produces detailed messages for logging purposes.
- **Tests** (lines 532-591): Three tests cover the key invariants:
  - `test_sanitized_message_client_errors_preserved` — all 5 client variants return detailed messages
  - `test_sanitized_message_server_errors_generic` — all 5 server variants return the generic string
  - `test_display_still_detailed_for_logging` — `Display` and `sanitized_message()` produce different output for the same error

#### File 2: `src/auth/user_profile.rs` — AC4 (DB Error Sanitization) + AC8 (Method Enforcement)

**AC4 — DB Error Sanitization**

- **Error handling restructured** (lines 78-85): The `.map_err()` pattern on `get_user_by_id` now logs the detailed error via `tracing::error!` and returns the generic `"An internal error occurred. Please try again later."` message. Matches the `oauth.rs` approach.
- **"User not found" (404)** is correctly left unsanitized — it's a legitimate user-facing message, not an internal error.

**AC8 — Method Enforcement**

- **New imports** (lines 8-13): `axum::http::header`, `axum::http::HeaderMap`, `axum::http::Method`, `axum::response::IntoResponse`.
- **Return type changed** (line 35): `Result<Json<AuthContext>, (StatusCode, String)>` → `axum::response::Response`. This enables attaching custom headers (the `Allow` header) to the 405 response.
- **Parameter renamed** (line 34): `_req` → `req` — the parameter was already present but unused; now actively used for the method check.
- **Method guard** (lines 37-49): First block in the function body. Checks `req.method() != Method::GET`, returns `StatusCode::METHOD_NOT_ALLOWED` with `Allow: GET` header. This short-circuits before any session validation or database queries.
- **Error handling restructured** (lines 56-93): Replaced `?` operator with explicit nested `match` because `Response` return type is incompatible with `?`. The outer `match` on `verified_user_id` wraps an inner `match` on `get_user_by_id`. Both success and error arms are converted via `.into_response()` at the end.
- **Tests** (lines 96-205): 4 integration tests using `tower::ServiceExt::oneshot`:
  - `user_profile_get_returns_200_unauthenticated` — GET baseline works (200 with unauthenticated context)
  - `user_profile_post_returns_405` — POST rejected with `Allow: GET` header
  - `user_profile_put_returns_405` — PUT rejected with `Allow: GET` header
  - `user_profile_delete_returns_405` — DELETE rejected with `Allow: GET` header
- **Test router design** (lines 108-119): Registers `.get().post().put().delete()` to deliberately bypass the router-level `.get()` filter. This exercises the handler-level check in isolation, proving defense-in-depth.

#### File 3: `src/auth/session.rs` — AC9 (Cookie Domain)

- **`TEST_COOKIE_DOMAIN` thread-local** (lines 62-64): `thread_local! { static TEST_COOKIE_DOMAIN: std::cell::RefCell<Option<String>> }` — mirrors the existing `TEST_SECRET` pattern (line 57-59). Eliminates race conditions when domain tests run in parallel.
- **`read_cookie_domain()` helper** (lines 82-97): Dual `#[cfg(test)]`/`#[cfg(not(test))]` variants. Production variant (lines 83-87) is a simple `std::env::var("COOKIE_DOMAIN").ok().filter(|d| !d.trim().is_empty())`. Test variant (lines 90-97) checks thread-local first, then falls back to env var. Both filter out empty and whitespace-only values.
- **`build_session_cookie` refactored** (lines 162-177): Changed from single-expression chain to mutable `CookieBuilder`. Reads domain via `read_cookie_domain()`, conditionally chains `.domain(d)` if set.
- **`clear_session_cookie` refactored** (lines 182-197): Same pattern. Critical for RFC 6265 cookie deletion — the domain must match exactly between set and clear cookies.
- **Test helpers** (lines 398-411): `with_cookie_domain()` and `without_cookie_domain()` mirror the existing `with_secret()`/`without_secret()` pattern. `without_cookie_domain()` clears both the thread-local AND the env var.
- **Tests** (lines 413-475): 6 new tests:
  - `session_cookie_includes_domain_when_set` — thread-local `.example.com` → `cookie.domain() == Some("example.com")` (cookie crate strips leading dot)
  - `session_cookie_has_no_domain_when_unset` — thread-local cleared → `cookie.domain() == None`
  - `clear_cookie_includes_domain_when_set` — `clear_session_cookie` picks up domain + `max_age == ZERO`
  - `clear_cookie_has_no_domain_when_unset` — `clear_session_cookie` has no domain when unset
  - `cookie_domain_empty_string_is_treated_as_unset` — empty string filtered out
  - `cookie_domain_whitespace_only_is_treated_as_unset` — whitespace-only filtered out

#### File 4: `.env.local.example` — AC9 Documentation

- **Appended** (lines 42-46): Documents `COOKIE_DOMAIN` env var — purpose, format (leading dot convention), and default behavior when unset. Commented out by default.

---

### Dependencies

**No new crate dependencies were introduced.** All changes use existing dependencies:
- `tracing` (already in `Cargo.toml`) — used for structured error logging in AC4
- `axum` (already in `Cargo.toml`) — `Method`, `HeaderMap`, `header::ALLOW`, `StatusCode::METHOD_NOT_ALLOWED`, `Response`, `IntoResponse` all re-exported
- `cookie` v0.18.1 (already in `Cargo.toml`) — `CookieBuilder::domain()` available
- `tower` (already in dev-dependencies) — `ServiceExt` used in AC8 tests

---

### Non-Obvious Patterns & Language Features

1. **Byte-length vs character-length** (AC7): `.len()` on `&str` returns byte count. This is intentionally conservative for security — non-ASCII input is rejected earlier than a character-count check would.

2. **`?` operator incompatibility with `Response`** (AC8): Changing the return type to `axum::response::Response` means `?` can no longer be used. The error handling was restructured to explicit nested `match` blocks, with `.into_response()` at the end converting the `Result` to `Response`.

3. **`#[cfg(test)]`/`#[cfg(not(test))]` dual variants** (AC9): The `read_cookie_domain()` function has two compiled variants — a simple one-liner in production and a thread-local-aware version in tests. Zero runtime cost in production.

4. **Thread-local test isolation** (AC9): `TEST_COOKIE_DOMAIN` uses `thread_local!` + `RefCell<Option<String>>` to give each test thread its own domain value. This avoids the `serial_test` dependency and is consistent with the existing `TEST_SECRET` pattern.

5. **Cookie crate domain normalization** (AC9): The `cookie` crate strips the leading dot from domain values (`.example.com` → `example.com`), which is RFC 6265 compliant. Tests account for this normalization.

6. **Tuple `IntoResponse` patterns** (AC8): `(StatusCode::METHOD_NOT_ALLOWED, headers, "Method Not Allowed").into_response()` — axum's `IntoResponse` impl supports `(Status, HeaderMap, Body)` tuples for responses with custom headers.

---

### Follow-Up Recommendations

1. **Generic message string duplication** (AC4): The string `"An internal error occurred. Please try again later."` is inlined in both `oauth.rs` (line 119) and `user_profile.rs` (line 83). Consider defining a shared constant in a common module if the message text ever needs to change. Low priority — the string is short and self-documenting.

2. **`StateNotFound`/`StateExpired` warn-level logging** (AC4): These are logged at `warn!` level and may generate noise during normal OAuth flows (stale links, refreshed callback pages). Consider downgrading to `debug!` if log volume becomes an issue in production.

3. **405 response body format** (AC8): The 405 body is plain text (`"Method Not Allowed"`), not JSON. This is technically fine per the HTTP spec, but is a minor stylistic inconsistency with the rest of the API's JSON responses. Consider wrapping in `Json` if API consistency is a priority.

4. **Monitor `COOKIE_DOMAIN` configuration consistency** (AC9): If `COOKIE_DOMAIN` is set at login time but unset (or changed) at logout time, the clear cookie will have a different domain and the browser will not delete the session cookie. Ensure the env var is consistent across the application lifecycle.

---

### Commit Message

```
fix(auth): harden OAuth redirect URI, error messages, method enforcement, cookie domain

AC7 — Redirect URI length validation:
- Add REDIRECT_URI_MAX_LEN constant (2048 bytes) to oauth.rs
- Add length check as first condition in validate_redirect_uri,
  rejecting oversized input before any string operations
- Byte-length semantics are conservative for non-ASCII input
- Add boundary tests: 2049 bytes rejected, 2048 bytes accepted

AC4 — Error message sanitization:
- Add sanitized_message() method to OAuthError: client errors (4xx)
  return detailed messages, server errors (5xx) return generic message
- Rewrite IntoResponse impl: log detailed error via tracing::error!/warn!
  before returning sanitized message to client
- Preserve Display impl unchanged for server-side logging
- Sanitize DB error in user_profile.rs handler (same pattern)
- Add 3 tests: client errors preserved, server errors generic,
  Display still detailed

AC8 — Method enforcement on user_profile endpoint:
- Add handler-level GET-only check in handle_user_profile
- Return 405 Method Not Allowed with Allow: GET header (RFC 7231)
- Change return type to axum::response::Response to support custom headers
- Restructure error handling from ? to explicit match (Response incompatible
  with ? operator)
- Add 4 integration tests: GET→200, POST/PUT/DELETE→405+Allow header
- Test router registers all methods to bypass router-level filter

AC9 — Cookie domain from COOKIE_DOMAIN env var:
- Add read_cookie_domain() helper with dual #[cfg(test)]/#[cfg(not(test))]
  variants; filters empty and whitespace-only values
- Conditionally chain .domain() on CookieBuilder in both
  build_session_cookie and clear_session_cookie
- Add TEST_COOKIE_DOMAIN thread-local for parallel test safety
- Add with_cookie_domain/without_cookie_domain test helpers
- Add 6 tests: domain set/unset for build and clear, empty/whitespace edges
- Document COOKIE_DOMAIN in .env.local.example

All changes use existing dependencies (tracing, axum, cookie 0.18.1).
Full test suite: 101 passed, 0 failed.
```
