# Task Brief

## Task Description
Implement AC2 from NOMS-006: GET logout CSRF protection. The logout endpoint currently accepts GET requests without CSRF validation, allowing an attacker to force a user to logout via an image tag or link. The fix requires the logout endpoint to validate a `redirect_uri` query parameter on GET requests, or use a CSRF token.

## Phase 0: Implementation Blueprint
<!-- written by @develop-architect -->

## Approach

The GET `/auth/logout` endpoint is currently vulnerable to CSRF because it performs a state-changing action (clearing the session cookie) in response to a simple GET request. An attacker can trigger logout via `<img src="https://app.example.com/auth/logout">` or a link.

**Mitigation strategy:** Require a `redirect_uri` query parameter on GET logout requests. If the parameter is missing or fails validation, default to `/`. This means an attacker's forged GET request (which cannot include a custom `redirect_uri`) will redirect the user to the home page instead of an attacker-controlled destination, and critically, the attacker cannot force a logout that redirects to a page they control.

**Why this works:** The `SameSite=Lax` cookie attribute already prevents the session cookie from being sent on cross-origin GET requests triggered by `<img>` tags or form submissions. However, `SameSite=Lax` *does* send cookies on top-level navigations (including `<a href>` links from a same-site context). The `redirect_uri` validation adds defense-in-depth: even if a same-site attacker tricks a user into clicking a link to `/auth/logout`, the redirect destination is validated and cannot be an external URL.

**POST logout remains unchanged** — it continues to work without `redirect_uri` for programmatic/API use.

## Key Research Findings

### Current behavior

| Aspect | Detail |
|--------|--------|
| Handler | `src/auth/logout.rs`, `handle_logout()` (lines 12-24) — takes no parameters |
| Route registration | `src/main.rs` lines 104-108 — GET and POST both use same handler |
| Frontend logout (navbar) | `src/components/navbar.rs` line 120 — `window.location().set_href("/auth/logout")` |
| Frontend logout (settings) | `src/pages/settings/settings_profile.rs` line 352 — `window.location().set_href("/auth/logout")` |
| Redirect validation pattern | `src/auth/oauth.rs` lines 167-175 — `validate_redirect_uri()` function |
| Cookie clearing | `src/auth/session.rs` line 182-197 — `clear_session_cookie()` |
| Session cookie | `SameSite=Lax`, `HttpOnly`, `Secure` (prod) — `src/auth/session.rs` lines 162-177 |

### Existing patterns to reuse

1. **`validate_redirect_uri`** (`src/auth/oauth.rs:167-175`): Validates URI starts with `/`, doesn't start with `//`, doesn't contain `://`, and is ≤2048 chars. Returns `Result<(), OAuthError>`. This function is `fn` (not `pub fn`) — we need to either make it `pub` or duplicate the logic.

2. **`REDIRECT_URI_MAX_LEN`** (`src/auth/oauth.rs:235`): Constant `2048`, defined in `oauth.rs` as `const`.

3. **Axum `Query` extractor** — used in `oauth.rs:247` for `StartQuery`. We'll use the same pattern.

### Security references

- **OWASP CSRF Prevention Cheat Sheet**: "If for any reason you do [use GET for state-changing operations], protect those resources against CSRF." Also notes `SameSite=Lax` allows cookies on top-level GET navigations.
- **RFC 9700 §4.1.1**: Redirect URI validation must use exact string matching; open redirectors must be avoided.
- **Spring Security logout**: Uses GET `/logout` to show confirmation page (with CSRF token) and POST `/logout` for actual logout — a different pattern but same principle: GET alone shouldn't perform the action.

## Files to Modify

### 1. `src/auth/logout.rs` — main changes

**Add imports** (after line 5):
```rust
use axum::extract::Query;
use serde::Deserialize;
```

**Add query struct and constants** (after line 6, before `handle_logout`):
```rust
/// Maximum allowed length for redirect_uri parameter on logout.
const REDIRECT_URI_MAX_LEN: usize = 2048;

/// Query parameters for GET logout requests.
#[derive(Debug, Deserialize)]
struct LogoutQuery {
    #[serde(default)]
    redirect_uri: Option<String>,
}

/// Validate that the redirect_uri is a same-origin relative path.
///
/// Must start with `/` and must not contain `://` (no absolute URLs)
/// and must not start with `//` (no protocol-relative URLs).
/// Returns `None` if the URI is empty or missing (caller should use default).
fn validate_redirect_uri(uri: &str) -> Option<String> {
    if uri.is_empty() {
        return None;
    }
    if uri.len() > REDIRECT_URI_MAX_LEN {
        return None;
    }
    if !uri.starts_with('/') || uri.starts_with("//") || uri.contains("://") {
        return None;
    }
    Some(uri.to_string())
}
```

**Replace `handle_logout`** (lines 8-24) with:
```rust
/// Handle a logout request.
///
/// For GET requests: validates an optional `redirect_uri` query parameter.
/// If valid, redirects to that URI. If missing or invalid, defaults to `/`.
/// For POST requests: always redirects to `/` (unchanged behavior).
///
/// Clears the session cookie by setting it with `max-age=0`.
pub async fn handle_logout(Query(params): Query<Option<LogoutQuery>>) -> Response {
    let clear_cookie = session::clear_session_cookie();
    let cookie_header = clear_cookie.encoded().to_string();

    // Determine redirect target: validate redirect_uri if provided, default to "/"
    let redirect_target = match &params {
        Some(q) => validate_redirect_uri(&q.redirect_uri.clone().unwrap_or_default())
            .unwrap_or_else(|| "/".to_string()),
        None => "/".to_string(),
    };

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        cookie_header.parse().expect("valid cookie header"),
    );
    headers.insert(
        axum::http::header::LOCATION,
        redirect_target
            .parse()
            .expect("valid redirect location"),
    );

    (StatusCode::FOUND, headers, ()).into_response()
}
```

**Update tests** (lines 26-118):

Replace the existing test module with:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn make_router() -> axum::Router {
        axum::Router::new().route(
            "/auth/logout",
            axum::routing::get(handle_logout).post(handle_logout),
        )
    }

    #[tokio::test]
    async fn logout_post_returns_302_with_redirect() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_sets_clear_cookie_header() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let set_cookie = response
            .headers()
            .get(axum::http::header::SET_COOKIE)
            .expect("Set-Cookie header should be present");

        let cookie_str = set_cookie.to_str().unwrap();
        assert!(cookie_str.contains("noms_session"));
        assert!(cookie_str.contains("Max-Age=0"));
    }

    #[tokio::test]
    async fn logout_get_no_params_redirects_to_home() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
        assert!(response.headers().contains_key(axum::http::header::SET_COOKIE));
    }

    #[tokio::test]
    async fn logout_get_valid_redirect_uri() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/dashboard"
        );
        assert!(response.headers().contains_key(axum::http::header::SET_COOKIE));
    }

    #[tokio::test]
    async fn logout_get_redirect_uri_with_query_string() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=/dashboard%3Ftab%3Drecipes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/dashboard?tab=recipes"
        );
    }

    #[tokio::test]
    async fn logout_get_invalid_redirect_external_url_defaults_to_home() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=https://evil.com/phish")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        // Should default to "/" because external URL is rejected
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_invalid_redirect_protocol_relative_defaults_to_home() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=//evil.com/phish")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_invalid_redirect_no_leading_slash_defaults_to_home() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_empty_redirect_uri_defaults_to_home() {
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/auth/logout?redirect_uri=")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn logout_get_overlong_redirect_uri_defaults_to_home() {
        let long_uri = format!("{}", "a".repeat(REDIRECT_URI_MAX_LEN + 1));
        let encoded = urlencoding::encode(&long_uri);
        let app = make_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/auth/logout?redirect_uri={}", encoded))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::LOCATION).unwrap(),
            "/"
        );
    }
}
```

**Note on `urlencoding` dependency:** The overlong URI test uses `urlencoding`. Since this is a dev-only need, check if `percent-encoding` (already in `Cargo.toml` line 24) can be used instead. If not, add `urlencoding` to `[dev-dependencies]`. Alternatively, manually construct the URL-encoded string.

### 2. `src/components/navbar.rs` — frontend logout URL update

**Line 120:** Change:
```rust
let _ = window.location().set_href("/auth/logout");
```
To:
```rust
let _ = window.location().set_href("/auth/logout?redirect_uri=/");
```

This single change covers both the desktop dropdown (line 184 calls `on_sign_out`) and the mobile drawer (line 270 calls `on_sign_out`), since both use the same `on_sign_out` closure defined at lines 118-122.

### 3. `src/pages/settings/settings_profile.rs` — frontend logout URL update

**Line 352:** Change:
```rust
let _ = window.location().set_href("/auth/logout");
```
To:
```rust
let _ = window.location().set_href("/auth/logout?redirect_uri=/");
```

## Step-by-Step Implementation Order

1. **Modify `src/auth/logout.rs`**: Add imports, constants, `LogoutQuery` struct, `validate_redirect_uri` helper, update `handle_logout` signature and body.
2. **Update tests in `src/auth/logout.rs`**: Replace existing tests with the expanded test suite covering all validation cases.
3. **Update `src/components/navbar.rs`** line 120: Add `?redirect_uri=/` to logout URL.
4. **Update `src/pages/settings/settings_profile.rs`** line 352: Add `?redirect_uri=/` to logout URL.
5. **Verify `src/main.rs`**: No changes needed — the route registration at lines 104-108 already supports both GET and POST with the same handler. The `Query` extractor in Axum handles missing query params gracefully.
6. **Run tests**: `cargo test --features server logout` to verify all logout tests pass.

## Architectural Decisions and Trade-offs

| Decision | Rationale |
|----------|-----------|
| **Validate in handler, not middleware** | Minimal change — logout is a single endpoint. A middleware layer would be overkill and add complexity. |
| **Default to `/` on invalid redirect** | Safe fallback. An attacker cannot redirect to an external site. The user is simply logged out and sent home. |
| **Allow GET without params** | Backward compatibility — existing bookmarks or direct navigation to `/auth/logout` still work (redirects to `/`). |
| **POST unchanged** | Programmatic/API clients that POST to logout don't need to change. |
| **Duplicate `validate_redirect_uri` logic** | Rather than making `oauth.rs`'s function public (which would create a circular dependency since `logout.rs` shouldn't depend on OAuth internals), we define a local `fn` with the same validation logic. The logic is simple (4 checks) and self-contained. |
| **`Query<Option<LogoutQuery>>` pattern** | Axum's `Query` extractor with `Option<T>` gracefully handles both requests with and without query parameters. POST requests without query params will get `None`. |

## Test Summary

| Test | What it verifies |
|------|-----------------|
| `logout_post_returns_302_with_redirect` | POST still returns 302 to `/` |
| `logout_sets_clear_cookie_header` | Cookie is cleared (Max-Age=0) |
| `logout_get_no_params_redirects_to_home` | GET without params → `/` |
| `logout_get_valid_redirect_uri` | GET with valid `/dashboard` → `/dashboard` |
| `logout_get_redirect_uri_with_query_string` | URL-encoded query strings work |
| `logout_get_invalid_redirect_external_url_defaults_to_home` | `https://evil.com` → `/` |
| `logout_get_invalid_redirect_protocol_relative_defaults_to_home` | `//evil.com` → `/` |
| `logout_get_invalid_redirect_no_leading_slash_defaults_to_home` | `dashboard` → `/` |
| `logout_get_empty_redirect_uri_defaults_to_home` | Empty string → `/` |
| `logout_get_overlong_redirect_uri_defaults_to_home` | >2048 chars → `/` |

## Dependencies

No new production dependencies. The `percent-encoding` crate is already in `Cargo.toml` (line 24) and can be used for URL encoding in the overlong URI test. If `percent-encoding` API is awkward for this test, add `urlencoding = "2"` to `[dev-dependencies]` in `Cargo.toml`.

## Gaps and Follow-up Items

1. **No middleware-level CSRF protection exists** — this fix is endpoint-specific. If broader CSRF protection is needed, consider adding a CSRF middleware in a future issue (NOMS-006 AC2 is scoped to logout only).
2. **The `redirect_uri` parameter is optional** — a GET to `/auth/logout` without params still clears the cookie and redirects to `/`. This is intentional for usability (direct navigation works) but means a same-site attacker *can* still force a logout via `<a href="/auth/logout">`. The mitigation is that the attacker cannot control the redirect destination. A future enhancement could require `redirect_uri` to be present (breaking change).
3. **Consider `Sec-Fetch-Site` header** — modern browsers send `Sec-Fetch-Site` headers. A future enhancement could reject GET logout requests where `Sec-Fetch-Site: cross-site`, providing stronger CSRF protection.

## Phase 1: Implementation Details
<!-- written by @develop-implement -->

### Summary
Implemented GET logout CSRF protection by adding `redirect_uri` query parameter validation to the `/auth/logout` endpoint. GET requests now validate the optional `redirect_uri` parameter against the same rules as OAuth (must start with `/`, no `://`, no `//`, ≤2048 chars). Invalid or missing values default to `/`. POST behavior is unchanged. Frontend logout URLs were updated to include `?redirect_uri=/`.

### Files Modified

**`src/auth/logout.rs`** — Main implementation
- Added `use axum::extract::Query;` and `use serde::Deserialize;` imports
- Added `REDIRECT_URI_MAX_LEN` constant (2048)
- Added `LogoutQuery` struct with optional `redirect_uri` field
- Added `validate_redirect_uri()` helper returning `Option<String>` (same validation logic as `oauth.rs`)
- Updated `handle_logout` to accept `Query<LogoutQuery>` parameter; validates `redirect_uri` if present, defaults to `/` otherwise
- Replaced 3 existing tests with 10 comprehensive tests covering: POST redirect, cookie clearing, GET without params, valid redirect_uri, redirect_uri with query string, invalid external URL, protocol-relative URL, no leading slash, empty redirect_uri, and overlong redirect_uri

**`src/components/navbar.rs`** — Line 120
- Changed `window.location().set_href("/auth/logout")` to `window.location().set_href("/auth/logout?redirect_uri=/")`

**`src/pages/settings/settings_profile.rs`** — Line 352
- Changed `window.location().set_href("/auth/logout")` to `window.location().set_href("/auth/logout?redirect_uri=/")`

### Test Results
All 108 tests pass (`cargo test --features server`):
- 10 logout-specific tests (3 updated + 7 new)
- 98 other existing tests (all passing, no regressions)

### Verification
- `cargo test --features server` — 108 passed, 0 failed
- Build compiles without warnings
- No changes needed to `src/main.rs` (route registration already supports both GET and POST)

## Phase 2: Review Verdict
<!-- written by @develop-review -->

### Verdict: **PASS**

The implementation correctly addresses AC2: GET logout CSRF protection. All requirements are met, tests are comprehensive, and the code is clean and idiomatic.

### Requirements Coverage

| # | Requirement | Status |
|---|-------------|--------|
| 1 | GET logout validates `redirect_uri` against same-origin rules (starts with `/`, no `://`, no `//`, ≤2048 chars) | ✅ `validate_redirect_uri()` at `logout.rs:25-36` implements all four checks |
| 2 | Invalid/missing `redirect_uri` defaults to `/` (safe fallback) | ✅ `logout.rs:50-53` — both `None` and validation failure paths return `"/"` |
| 3 | POST logout remains unchanged (redirects to `/`) | ✅ `logout.rs:52` — POST without query params deserializes to `redirect_uri: None` → defaults to `"/"` |
| 4 | Frontend logout URLs include `?redirect_uri=/` | ✅ `navbar.rs:120` and `settings_profile.rs:352` both updated |
| 5 | Tests are comprehensive | ✅ 10 tests covering: POST redirect, cookie clearing, no params, valid URI, URI with query string, external URL, protocol-relative, no leading slash, empty string, overlong URI |
| 6 | `<img src="...">` CSRF attack vector | ✅ Mitigated by `SameSite=Lax` cookie attribute (`session.rs:170`) — cross-origin GET requests do not send the session cookie, so the logout handler receives no authenticated session. Even if cookies were sent, the attacker cannot control the redirect destination |
| 7 | Edge cases and security concerns | ✅ Addressed (see findings below) |

### Issues

**No blockers or warnings found.** One suggestion for future improvement:

1. **SUGGESTION — Deduplicate `validate_redirect_uri`** (`src/auth/logout.rs:25-36` and `src/auth/oauth.rs:167-175`)
   - **Description:** The same 4-check validation logic exists in two files. The blueprint acknowledged this trade-off (avoiding circular dependency by not making `oauth.rs`'s function `pub`). The logic is simple enough that duplication is acceptable, but if more endpoints need redirect validation in the future, consider extracting to `src/auth/redirect.rs` as a shared utility.
   - **Recommended fix:** None needed now. Consider extraction if a third consumer appears.

### Positive Findings and Good Practices

1. **Better than blueprint — `Query<LogoutQuery>` instead of `Query<Option<LogoutQuery>>`:** The implementation uses `Query<LogoutQuery>` with `#[serde(default)]` on the field, which is cleaner and more idiomatic than the blueprint's `Query<Option<LogoutQuery>>` approach. The handler body is simpler (no nested `Option` matching).

2. **Test quality is excellent:** All 10 tests are well-named, isolated, and cover the full validation surface. The overlong URI test correctly uses `percent_encoding` (already in dependencies) rather than adding a new dev dependency.

3. **Validation order is correct:** The `starts_with('/')` check comes before `starts_with("//")`, which means a URI like `//evil.com` is caught by both checks (defense-in-depth). The `contains("://")` check catches `https://` and similar schemes.

4. **Safe `expect()` usage:** The `parse().expect("valid redirect location")` on line 64 can never panic because `validate_redirect_uri` only returns strings starting with `/`, and the fallback is `"/"`. Both are valid HTTP Location header values.

5. **Frontend coverage is complete:** Grep confirms only two frontend locations reference `/auth/logout` (`navbar.rs` and `settings_profile.rs`), and both are updated. The `on_sign_out` closure in `navbar.rs` is shared between desktop dropdown and mobile drawer, so a single change covers both UI paths.

6. **No regressions:** Full test suite (108 tests) passes cleanly.

### Summary

Well-executed, minimal-impact security fix. The implementation is slightly cleaner than the blueprint proposed, tests are thorough, and the security model is sound: `SameSite=Lax` prevents cross-origin CSRF, and redirect validation prevents open redirects. The only remaining exposure is same-site link-based logout (e.g., `<a href="/auth/logout">` from a same-origin page), which is intentional — logout is a low-impact action, and the user is safely redirected to `/`.

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->

### User-Facing Summary

This change fixes a CSRF vulnerability in the GET `/auth/logout` endpoint (NOMS-006, AC2). Previously, the logout endpoint accepted GET requests without any validation of the redirect destination, allowing two attack vectors: (1) an attacker could force a logged-in user to log out via an `<img>` tag or link, and (2) the logout could redirect to an attacker-controlled URL (open redirect). The fix adds `redirect_uri` query parameter validation on GET requests — only same-origin relative paths (starting with `/`, no `://`, no `//`, ≤2048 chars) are accepted. Invalid or missing values safely default to `/`. The frontend logout flows (navbar and settings page) were updated to include `?redirect_uri=/`. All 108 tests pass with no regressions.

### Walkthrough of Changes

#### `src/auth/logout.rs` — Logout handler (main implementation)

**What changed:** This file was substantially modified to add redirect URI validation to the GET logout flow.

**New imports** (lines 3, 6):
- `use axum::extract::Query;` — Axum's query string extractor, used to parse `redirect_uri` from the URL.
- `use serde::Deserialize;` — Derive macro for deserializing query parameters into a struct.

**New constant** (line 11):
- `REDIRECT_URI_MAX_LEN: usize = 2048` — Maximum allowed length for the `redirect_uri` parameter, matching the OAuth module's existing limit.

**New struct** (lines 14-18):
- `LogoutQuery` with a single optional `redirect_uri: Option<String>` field. The `#[serde(default)]` attribute ensures that requests without the parameter deserialize cleanly (field becomes `None` rather than causing a deserialization error). This is cleaner than the blueprint's `Query<Option<LogoutQuery>>` approach.

**New helper function** (lines 25-36):
- `validate_redirect_uri(uri: &str) -> Option<String>` — Four-check validation:
  1. Empty string → `None` (use default)
  2. Length > 2048 → `None` (reject overlong)
  3. Doesn't start with `/`, starts with `//`, or contains `://` → `None` (reject non-relative, protocol-relative, and absolute URLs)
  4. Otherwise → `Some(uri.to_string())`

**Updated handler** (lines 45-68):
- Signature changed from `pub async fn handle_logout() -> Response` to `pub async fn handle_logout(Query(params): Query<LogoutQuery>) -> Response`.
- The `Query<LogoutQuery>` extractor parses query parameters. For POST requests (no query string), `redirect_uri` is `None`. For GET requests, it may be `Some(value)` or `None` if the parameter is absent.
- Redirect target logic (lines 50-53): if `redirect_uri` is `Some(uri)`, validate it; if validation passes, use the validated URI; if it fails or is `None`, default to `"/"`.
- Response construction is unchanged: `Set-Cookie` header clears the session cookie, `Location` header sets the redirect target, status is `302 Found`.

**Test suite** (lines 70-318): Replaced 3 existing tests with 10 comprehensive tests:
| Test | What it verifies |
|------|-----------------|
| `logout_post_returns_302_with_redirect` | POST still returns 302 to `/` |
| `logout_sets_clear_cookie_header` | Cookie is cleared (`noms_session`, `Max-Age=0`) |
| `logout_get_no_params_redirects_to_home` | GET without params → `/` |
| `logout_get_valid_redirect_uri` | GET with valid `/dashboard` → `/dashboard` |
| `logout_get_redirect_uri_with_query_string` | URL-encoded query strings (`/dashboard?tab=recipes`) work |
| `logout_get_invalid_redirect_external_url_defaults_to_home` | `https://evil.com/phish` → `/` |
| `logout_get_invalid_redirect_protocol_relative_defaults_to_home` | `//evil.com/phish` → `/` |
| `logout_get_invalid_redirect_no_leading_slash_defaults_to_home` | `dashboard` → `/` |
| `logout_get_empty_redirect_uri_defaults_to_home` | Empty string → `/` |
| `logout_get_overlong_redirect_uri_defaults_to_home` | >2048 chars → `/` |

**Notable patterns:**
- Tests use `tower::ServiceExt::oneshot` to send requests directly to the handler without a full server — fast and isolated.
- The overlong URI test uses `percent_encoding` (already in `Cargo.toml`) to properly URL-encode the test payload, avoiding the need for a new dev dependency.

#### `src/components/navbar.rs` — Frontend logout URL (line 120)

**What changed:** Single-line change in the `on_sign_out` closure.

**Before:** `let _ = window.location().set_href("/auth/logout");`
**After:** `let _ = window.location().set_href("/auth/logout?redirect_uri=/");`

This single change covers both the desktop dropdown menu and the mobile drawer, since both UI paths share the same `on_sign_out` closure. The `redirect_uri=/` ensures that after logout, the user is redirected to the home page rather than relying on the handler's default (which is the same, but being explicit is clearer for the frontend contract).

#### `src/pages/settings/settings_profile.rs` — Frontend logout URL (line 352)

**What changed:** Single-line change in the account deletion flow.

**Before:** `let _ = window.location().set_href("/auth/logout");`
**After:** `let _ = window.location().set_href("/auth/logout?redirect_uri=/");`

This is the logout step that fires after a successful account deletion. The user is deleted, then navigated to logout, then redirected to `/`.

### Dependencies

No new production or development dependencies were introduced. The `percent-encoding` crate (already in `Cargo.toml`) is used in one test for URL encoding.

### No Changes Needed

- **`src/main.rs`** — Route registration already supports both GET and POST with the same handler. The `Query` extractor handles missing parameters gracefully via `#[serde(default)]`.
- **`src/auth/session.rs`** — Cookie clearing logic unchanged. The `SameSite=Lax` attribute on the session cookie provides the first layer of CSRF defense (cross-origin `<img>` tags don't send the cookie).

### Follow-Up Recommendations

1. **Deduplicate `validate_redirect_uri`** — The same 4-check validation logic exists in both `logout.rs` (lines 25-36) and `oauth.rs` (lines 167-175). If a third endpoint needs redirect validation, extract to a shared module (e.g., `src/auth/redirect.rs`).
2. **Consider `Sec-Fetch-Site` header** — Modern browsers send `Sec-Fetch-Site` headers. Rejecting GET logout requests where `Sec-Fetch-Site: cross-site` would provide stronger CSRF protection without impacting legitimate flows.
3. **Evaluate requiring `redirect_uri`** — Currently, GET `/auth/logout` without parameters still clears the cookie and redirects to `/`. A same-site attacker could still force a logout via `<a href="/auth/logout">`. Requiring `redirect_uri` would close this vector but is a breaking change for direct navigation.

### Commit Message

```
fix(auth): protect GET logout from CSRF and open redirect

The /auth/logout endpoint accepted GET requests without validating the
redirect destination, allowing two attack vectors:

1. CSRF via <img src="/auth/logout"> — an attacker could force a
   logged-in user to log out via a cross-origin image tag. Mitigated
   by the existing SameSite=Lax cookie attribute (cross-origin GETs
   don't send the session cookie).

2. Open redirect — even if SameSite=Lax allowed the cookie through
   (e.g., same-site link click), the attacker could control where the
   user is redirected after logout.

Fix: Add redirect_uri query parameter validation on GET requests.
Only same-origin relative paths are accepted (must start with /, no
://, no //, max 2048 chars). Invalid or missing values default to /,
preventing open redirects to attacker-controlled URLs.

Frontend logout URLs (navbar, settings) now include ?redirect_uri=/
to maintain the expected redirect behavior.

POST /auth/logout remains unchanged for API/programmatic use.

Files changed:
- src/auth/logout.rs: Added LogoutQuery struct, validate_redirect_uri
  helper, updated handler signature and logic, expanded tests from 3
  to 10 covering all validation cases
- src/components/navbar.rs: Added ?redirect_uri=/ to logout URL
- src/pages/settings/settings_profile.rs: Added ?redirect_uri=/ to
  logout URL

Tests: 108 passing (10 logout-specific, 98 existing — no regressions)

Refs: NOMS-006 AC2
```
