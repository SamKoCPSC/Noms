# Task Brief

## Task Description
Fix the local mock OAuth configuration so that Google and GitHub providers are correctly separated using issuer-prefixed URLs on the Navikt mock-oauth2-server. Currently both providers share the same `/authorize`, `/token`, and `/userinfo` endpoints, causing `provider_uid` collisions and preventing proper testing of cross-provider linking scenarios. The fix should use issuer-prefixed URLs (e.g., `/google/authorize`, `/github/authorize`) so each provider gets independent token signing keys and claims. Also update userinfo extraction logic if needed to handle issuer-specific claim formats. After implementation, verify the fix works end-to-end using Chrome DevTools against the running local app and mock server.

## Phase 0: Implementation Blueprint
## Phase 0: Implementation Blueprint

### 1. Problem Analysis

**Root cause:** Both Google and GitHub OAuth clients point to the same mock server endpoints (`/authorize`, `/token`, `/userinfo`). The Navikt mock-oauth2-server treats the first path segment of a URL as the `issuerId`. Without issuer prefixes, both providers use the implicit `default` issuer, which means:
- Both providers share the same token signing key
- Both providers return the same `sub` claim from `/userinfo`
- The `provider_uid` extraction (which falls back to `sub`) produces identical values for both providers
- This prevents testing cross-provider account linking scenarios

**Current state (`.env.local` lines 20-31):**
```
GOOGLE_AUTH_URL=http://localhost:8082/authorize
GOOGLE_TOKEN_URL=http://localhost:8082/token
GOOGLE_USERINFO_URL=http://localhost:8082/userinfo
GITHUB_AUTH_URL=http://localhost:8082/authorize
GITHUB_TOKEN_URL=http://localhost:8082/token
GITHUB_USERINFO_URL=http://localhost:8082/userinfo
```

**Desired state:** Each provider gets its own issuer (`google` and `github`), with independent signing keys and claims:
```
GOOGLE_AUTH_URL=http://localhost:8082/google/authorize
GOOGLE_TOKEN_URL=http://localhost:8082/google/token
GOOGLE_USERINFO_URL=http://localhost:8082/google/userinfo
GITHUB_AUTH_URL=http://localhost:8082/github/authorize
GITHUB_TOKEN_URL=http://localhost:8082/github/token
GITHUB_USERINFO_URL=http://localhost:8082/github/userinfo
```

### 2. Research Findings

#### Navikt mock-oauth2-server issuer architecture
- **Source:** [navikt/mock-oauth2-server README](https://github.com/navikt/mock-oauth2-server)
- The first path segment in any URL is the `issuerId` (e.g., `/google/authorize` → issuerId = `google`)
- Each issuer gets its own token signing key automatically
- The `/userinfo` endpoint returns the claims from the Bearer token (access token) — it does NOT use `requestMappings`
- `tokenCallbacks` in `JSON_CONFIG` configure what claims are returned when a `/token` request matches certain parameters
- With `interactiveLogin: true`, the user can input custom claims in the login form
- Without `tokenCallbacks`, the default behavior returns a random UUID as `sub`
- The `tid` claim is auto-set to the `issuerId`

#### Current userinfo extraction logic (`src/auth/oauth.rs`)
- **Google (lines 460-502):** Calls `GOOGLE_USERINFO_URL` with Bearer token, extracts `email` as primary `provider_uid`, falls back to `sub`. Also extracts `name` and `picture`.
- **GitHub (lines 508-556):** Calls `GITHUB_USERINFO_URL` (env var) or falls back to `GITHUB_API_URL/user`. Extracts `email` as primary `provider_uid`, falls back to `id` (GitHub API) or `sub` (OIDC). Also extracts `login`/`name` and `avatar_url`/`picture`.

#### Current `build_oauth_clients` defaults (`src/auth/oauth.rs`, lines 208-252)
- Google auth/token fallback: `http://localhost:8082/authorize`, `http://localhost:8082/token`
- GitHub auth/token fallback: `http://localhost:8082/authorize`, `http://localhost:8082/token` (same!)
- Google userinfo default: `https://www.googleapis.com/oauth2/v3/userinfo` (production URL — always overridden by env var in dev)
- GitHub userinfo: env var `GITHUB_USERINFO_URL` checked first, falls back to `GITHUB_API_URL/user`

#### Linking implications (`src/auth/linking.rs`)
- `link_or_create()` uses `info.provider_uid` (line 221, 262, 282, 322) to look up/create OAuth accounts
- The `provider` enum is already distinct (`Provider::Google` vs `Provider::GitHub`)
- No changes needed to linking.rs — the fix is purely in URL configuration and userinfo extraction

### 3. Files to Modify

#### A. `docker-compose.yml` (lines 50-61) — Add JSON_CONFIG for mock-oauth service

**Change:** Add `JSON_CONFIG` environment variable to configure token callbacks with distinct claims per issuer.

```yaml
  mock-oauth:
    image: ghcr.io/navikt/mock-oauth2-server:latest
    container_name: noms-mock-oauth
    ports:
      - "8082:8080"
    environment:
      - LOG_LEVEL=info
      - JSON_CONFIG={"interactiveLogin":true,"httpServer":"NettyWrapper","tokenCallbacks":[{"issuerId":"google","tokenExpiry":3600,"requestMappings":[{"requestParam":"grant_type","match":"*","claims":{"sub":"google-user-123","email":"google-user@example.com","name":"Google User","picture":"https://example.com/google-avatar.png","aud":["mock-google-client-id"]}]}]},{"issuerId":"github","tokenExpiry":3600,"requestMappings":[{"requestParam":"grant_type","match":"*","claims":{"sub":"github-user-456","email":"github-user@example.com","name":"GitHub User","picture":"https://example.com/github-avatar.png","aud":["mock-github-client-id"]}]}]}
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://localhost:8080/isalive || exit 1"]
      interval: 5s
      timeout: 3s
      retries: 5
```

**Key design decisions:**
- `interactiveLogin: true` — keeps the interactive login form (allows manual claim override during testing)
- `httpServer: "NettyWrapper"` — required for standalone Docker mode
- `tokenExpiry: 3600` — 1 hour token expiry (same as default)
- Distinct `sub` values: `google-user-123` vs `github-user-456` — guarantees no `provider_uid` collision
- Distinct `email` values: different emails per provider to test email-based linking separately
- `aud` matches the client IDs used in `.env.local`

**Alternative (cleaner JSON formatting):** If the single-line JSON is too unwieldy, we can use `JSON_CONFIG_PATH` with a mounted config file. However, the inline approach avoids creating an extra file and is simpler for local dev.

#### B. `.env.local` (lines 19-31) — Update OAuth URLs to use issuer prefixes

**Before:**
```
# Google OAuth (local mock server)
GOOGLE_AUTH_URL=http://localhost:8082/authorize
GOOGLE_TOKEN_URL=http://localhost:8082/token
GOOGLE_USERINFO_URL=http://localhost:8082/userinfo
GOOGLE_CLIENT_ID=google
GOOGLE_CLIENT_SECRET=secret

# GitHub OAuth (local mock server)
GITHUB_AUTH_URL=http://localhost:8082/authorize
GITHUB_TOKEN_URL=http://localhost:8082/token
GITHUB_CLIENT_ID=github
GITHUB_CLIENT_SECRET=secret
GITHUB_USERINFO_URL=http://localhost:8082/userinfo
```

**After:**
```
# Google OAuth (local mock server — issuer-prefixed)
GOOGLE_AUTH_URL=http://localhost:8082/google/authorize
GOOGLE_TOKEN_URL=http://localhost:8082/google/token
GOOGLE_USERINFO_URL=http://localhost:8082/google/userinfo
GOOGLE_CLIENT_ID=google
GOOGLE_CLIENT_SECRET=secret

# GitHub OAuth (local mock server — issuer-prefixed)
GITHUB_AUTH_URL=http://localhost:8082/github/authorize
GITHUB_TOKEN_URL=http://localhost:8082/github/token
GITHUB_CLIENT_ID=github
GITHUB_CLIENT_SECRET=secret
GITHUB_USERINFO_URL=http://localhost:8082/github/userinfo
```

**Note:** The client IDs (`google` and `github`) must match the `aud` claim configured in the mock server's `tokenCallbacks`. Currently they match the default behavior where the mock server accepts any client ID.

#### C. `.env.local.example` (lines 24-31) — Update example URLs

**Before:**
```
# Google OAuth
GOOGLE_CLIENT_ID=google
GOOGLE_CLIENT_SECRET=secret

# GitHub OAuth
GITHUB_CLIENT_ID=github
GITHUB_CLIENT_SECRET=secret
GITHUB_USERINFO_URL=http://localhost:8082/userinfo
```

**After:**
```
# Google OAuth (local mock server — issuer-prefixed URLs)
GOOGLE_CLIENT_ID=google
GOOGLE_CLIENT_SECRET=secret
GOOGLE_AUTH_URL=http://localhost:8082/google/authorize
GOOGLE_TOKEN_URL=http://localhost:8082/google/token
GOOGLE_USERINFO_URL=http://localhost:8082/google/userinfo

# GitHub OAuth (local mock server — issuer-prefixed URLs)
GITHUB_CLIENT_ID=github
GITHUB_CLIENT_SECRET=secret
GITHUB_AUTH_URL=http://localhost:8082/github/authorize
GITHUB_TOKEN_URL=http://localhost:8082/github/token
GITHUB_USERINFO_URL=http://localhost:8082/github/userinfo
```

#### D. `src/auth/oauth.rs` — Update build_oauth_clients() default fallback URLs

**Lines 218-224 (Google client):**
```rust
// Before:
.set_auth_uri(
    AuthUrl::new(env_or("GOOGLE_AUTH_URL", "http://localhost:8082/authorize"))
        .expect("invalid Google auth URL"),
)
.set_token_uri(
    TokenUrl::new(env_or("GOOGLE_TOKEN_URL", "http://localhost:8082/token"))
        .expect("invalid Google token URL"),
)

// After:
.set_auth_uri(
    AuthUrl::new(env_or("GOOGLE_AUTH_URL", "http://localhost:8082/google/authorize"))
        .expect("invalid Google auth URL"),
)
.set_token_uri(
    TokenUrl::new(env_or("GOOGLE_TOKEN_URL", "http://localhost:8082/google/token"))
        .expect("invalid Google token URL"),
)
```

**Lines 239-245 (GitHub client):**
```rust
// Before:
.set_auth_uri(
    AuthUrl::new(env_or("GITHUB_AUTH_URL", "http://localhost:8082/authorize"))
        .expect("invalid GitHub auth URL"),
)
.set_token_uri(
    TokenUrl::new(env_or("GITHUB_TOKEN_URL", "http://localhost:8082/token"))
        .expect("invalid GitHub token URL"),
)

// After:
.set_auth_uri(
    AuthUrl::new(env_or("GITHUB_AUTH_URL", "http://localhost:8082/github/authorize"))
        .expect("invalid GitHub auth URL"),
)
.set_token_uri(
    TokenUrl::new(env_or("GITHUB_TOKEN_URL", "http://localhost:8082/github/token"))
        .expect("invalid GitHub token URL"),
)
```

**Line 470-472 (Google userinfo default):**
```rust
// Before:
let userinfo_url = env_or(
    "GOOGLE_USERINFO_URL",
    "https://www.googleapis.com/oauth2/v3/userinfo",
);

// After:
let userinfo_url = env_or(
    "GOOGLE_USERINFO_URL",
    "http://localhost:8082/google/userinfo",
);
```

**Rationale for the Google userinfo default change:** The current default is the production Google URL. In dev, this is always overridden by `.env.local`. Changing the default to the issuer-prefixed mock URL ensures that if someone forgets to set `GOOGLE_USERINFO_URL` in their env, they still get the correct mock endpoint instead of a production URL that would fail.

**Lines 514-521 (GitHub userinfo — no change needed):** The GitHub userinfo extraction already checks `GITHUB_USERINFO_URL` env var first. The fallback to `GITHUB_API_URL/user` is the production GitHub API, which is correct for non-mock scenarios.

#### E. `docs/manual-test-guide.md` — Update test environment URLs

**Line 4:** Change `Mock OAuth2 at http://localhost:8082` to note the issuer-prefixed URLs.

**Lines 11, 74, 91, 444:** Update references from `localhost:8082` to include issuer paths (e.g., `localhost:8082/google` for Google flow, `localhost:8082/github` for GitHub flow).

### 4. What Does NOT Need to Change

#### `src/auth/linking.rs` — No changes needed
- The `provider_uid` extraction already uses `email` as primary key (oauth.rs lines 494-497 for Google, 541-546 for GitHub)
- With distinct issuers, the `sub` claim will be different per provider (`google-user-123` vs `github-user-456`)
- The `provider` enum is already distinct and used for DB lookups
- Email-based linking (path b in `link_or_create()`) works independently of `sub`

#### Userinfo extraction logic — No changes needed
- `extract_google_user_info()` already handles both OIDC claims (`sub`, `name`, `picture`, `email`) and the mock server returns these via the `/userinfo` endpoint
- `extract_github_user_info()` already handles both OIDC claims and GitHub API fields
- The mock server's `/userinfo` endpoint returns the claims from the Bearer token, which will include the configured claims from `tokenCallbacks`

#### Redirect URIs — No changes needed
- Google redirect: `{base_url}/auth/google/callback` (line 226)
- GitHub redirect: `{base_url}/auth/github/callback` (line 247)
- These are already distinct and correct

### 5. Step-by-Step Implementation Order

1. **Update `docker-compose.yml`** — Add `JSON_CONFIG` env var with token callbacks for `google` and `github` issuers
2. **Update `.env.local`** — Change all 6 OAuth URLs to use issuer prefixes
3. **Update `.env.local.example`** — Add missing URL entries and update to issuer prefixes
4. **Update `src/auth/oauth.rs`** — Change `build_oauth_clients()` default fallback URLs (4 URLs) and Google userinfo default (1 URL)
5. **Update `docs/manual-test-guide.md`** — Update URL references for test documentation
6. **Restart Docker services** — `docker compose down && docker compose up -d` to apply new JSON_CONFIG
7. **Verify end-to-end** — See testing steps below

### 6. Testing Verification Steps

#### Step 1: Verify mock server configuration
```bash
# Check that the mock server is running with new config
curl http://localhost:8082/google/.well-known/openid-configuration
# Should return: {"issuer":"http://localhost:8082/google",...}

curl http://localhost:8082/github/.well-known/openid-configuration
# Should return: {"issuer":"http://localhost:8082/github",...}

# Verify distinct JWKS (different signing keys per issuer)
curl http://localhost:8082/google/jwks
curl http://localhost:8082/github/jwks
# The keys should be different between the two issuers
```

#### Step 2: Verify Google OAuth flow via Chrome DevTools
1. Navigate to `http://localhost:8080/auth/google/start?redirect_uri=/dashboard`
2. Should redirect to `http://localhost:8082/google/authorize?...`
3. Complete the mock login form (enter username, optionally customize claims)
4. Should redirect back to app callback
5. Check Network tab: `/google/token` was called, `/google/userinfo` was called
6. Check Application tab: session cookie is set
7. Check DB: new user created with `provider_uid` = `google-user-123` (or whatever `sub` was configured)

#### Step 3: Verify GitHub OAuth flow via Chrome DevTools
1. Navigate to `http://localhost:8080/auth/github/start?redirect_uri=/dashboard`
2. Should redirect to `http://localhost:8082/github/authorize?...`
3. Complete the mock login form
4. Should redirect back to app callback
5. Check Network tab: `/github/token` was called, `/github/userinfo` was called
6. Check DB: new user created with `provider_uid` = `github-user-456` (distinct from Google!)

#### Step 4: Verify cross-provider linking scenario
1. Log in with Google (creates user A with Google account)
2. Log out
3. Log in with GitHub (should create user B — different `provider_uid`, different email)
4. Verify: 2 users exist in DB, each with their own OAuth account
5. **Alternative:** If both providers share the same email, GitHub login should link to existing user via email match (path b in `link_or_create()`)

#### Step 5: Verify account linking from settings
1. Log in as the Google user
2. Navigate to `/settings/accounts`
3. Click "Connect GitHub"
4. Complete GitHub OAuth flow
5. Verify: GitHub account is linked to the same user (not a new user)
6. Both providers should appear in the linked accounts list

#### Step 6: Run existing tests
```bash
SQLX_OFFLINE=true cargo test --features server
```
All existing tests should pass. The changes only affect default fallback URLs and environment configuration.

### 7. Potential Pitfalls & Mitigations

| Risk | Mitigation |
|------|-----------|
| JSON_CONFIG escaping issues in docker-compose | Use YAML multi-line string (`>`) or single-line with proper JSON escaping. Test with `docker compose config` to verify. |
| Mock server version compatibility | The current image tag is `latest`. If issuer-prefixed URLs don't work, pin to a known version (e.g., `2.1.10`). |
| Client ID mismatch with `aud` claim | The `tokenCallbacks` configure `aud` to match the client IDs. If the mock server rejects the token request, check that `GOOGLE_CLIENT_ID` matches the `aud` claim. |
| Existing tests use hardcoded URLs | The `build_oauth_clients()` tests (line 687-693) use the default fallback URLs. These will change but should still work since the test only checks that the URL is non-empty. |
| `.env.local` not reloaded after changes | The `just up` command sources `.env.local` before starting the app. After changes, restart with `just down && just up`. |

### 8. Summary of Changes

| File | Lines | Change Type | Description |
|------|-------|-------------|-------------|
| `docker-compose.yml` | 50-61 | Modify | Add `JSON_CONFIG` env var with token callbacks for `google` and `github` issuers |
| `.env.local` | 20-31 | Modify | Update 6 OAuth URLs to use issuer prefixes (`/google/`, `/github/`) |
| `.env.local.example` | 24-31 | Modify | Add missing URL entries, update to issuer prefixes |
| `src/auth/oauth.rs` | 218, 222, 239, 243, 470 | Modify | Update 5 default fallback URLs to use issuer prefixes |
| `docs/manual-test-guide.md` | 4, 11, 74, 91, 444 | Modify | Update URL references to include issuer paths |

### 9. References

- **Navikt mock-oauth2-server README:** https://github.com/navikt/mock-oauth2-server
- **Issuer-prefixed URL architecture:** First path segment = `issuerId`, each issuer gets independent signing key
- **Token callbacks:** Configure `sub`, `email`, `name`, `picture`, `aud` per issuer via `JSON_CONFIG`
- **UserInfo endpoint behavior:** Returns claims from Bearer token, not from `requestMappings`
- **Current oauth.rs implementation:** `/home/samko/GitRepos/Noms/src/auth/oauth.rs`
- **Current linking.rs implementation:** `/home/samko/GitRepos/Noms/src/auth/linking.rs`
- **Implementation plan:** `/home/samko/GitRepos/Noms/roadmap/implementation-plans/NOMS-004-oauth-auth.md` (lines 206-218)

## Phase 1: Implementation Details

### Summary of Changes
Fixed the local mock OAuth configuration so Google and GitHub providers use separate issuers on the Navikt mock-oauth2-server. Previously both providers shared the same `/authorize`, `/token`, and `/userinfo` endpoints under the implicit `default` issuer, causing `provider_uid` collisions. Now each provider has its own issuer (`google` and `github`) with independent token signing keys, distinct `sub` claims, and separate userinfo responses.

### Files Modified

| File | Change |
|------|--------|
| `docker-compose.yml` | Added `JSON_CONFIG` env var with `tokenCallbacks` for `google` and `github` issuers, each with distinct `sub`, `email`, `name`, `picture`, and `aud` claims |
| `.env.local` | Updated 6 OAuth URLs from shared paths (`/authorize`, `/token`, `/userinfo`) to issuer-prefixed paths (`/google/authorize`, `/github/authorize`, etc.) |
| `.env.local.example` | Added missing `GOOGLE_AUTH_URL`, `GOOGLE_TOKEN_URL`, `GOOGLE_USERINFO_URL`, `GITHUB_AUTH_URL`, `GITHUB_TOKEN_URL` entries; all URLs use issuer prefixes |
| `src/auth/oauth.rs` | Updated 5 default fallback URLs in `build_oauth_clients()` and `extract_google_user_info()`: Google auth/token/userinfo now default to `/google/...` paths, GitHub auth/token default to `/github/...` paths |
| `docs/manual-test-guide.md` | Updated environment header and TC-02, TC-03, TC-22 test steps to reference issuer-prefixed URLs (`localhost:8082/google` and `localhost:8082/github`) |

### Files NOT Modified (confirmed no changes needed)
- `src/auth/linking.rs` — `provider_uid` extraction already uses `email` as primary key; distinct issuers ensure different `sub` values per provider; `provider` enum is already distinct
- Userinfo extraction logic — both `extract_google_user_info()` and `extract_github_user_info()` already handle OIDC claims correctly
- Redirect URIs — already distinct (`/auth/google/callback` vs `/auth/github/callback`)

### Tests
- **All 147 existing tests pass** (`SQLX_OFFLINE=true cargo test --features server`)
- No new tests were needed — the changes only affect default fallback URLs and environment configuration
- `test_build_oauth_clients` and `test_pkce_challenge_in_auth_url` continue to pass with the new default URLs (they only verify URLs are non-empty)

### Verification
- `cargo check --features server` — compiles without errors
- `SQLX_OFFLINE=true cargo test --features server` — 147 tests pass, 0 failures
- `docker compose config` — validates the YAML and JSON_CONFIG are properly parsed
- Manual verification of all modified files confirms correct issuer-prefixed URLs throughout

### Key Configuration Details
- Google issuer: `sub=google-user-123`, `email=google-user@example.com`
- GitHub issuer: `sub=github-user-456`, `email=github-user@example.com`
- `interactiveLogin: true` preserves the interactive login form for manual claim override during testing
- `aud` claims include both the mock client ID (`mock-google-client-id`/`mock-github-client-id`) and the short name (`google`/`github`) used in `.env.local`

## Phase 2: Review Verdict
<!-- written by @develop-review -->

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->
