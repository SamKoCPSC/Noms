# Staging test-session login

This feature lets authorized automation obtain an ordinary session without typing
an OAuth password. Recipe, collection, upload, account-binding, ownership and
logout checks still operate through the existing session machinery. There is no
public test-login button, password form or arbitrary-user impersonation parameter.

## Deployment boundary

The disabled-by-default `POST /auth/staging/session` endpoint returns 404 until
`NOMS_STAGING_LOGIN_ENABLED=true` is validated at startup. Enabling it requires
all of these exact, approved deployment identifiers:

| Setting | Approved value |
| --- | --- |
| `RAILWAY_PROJECT_ID` | `8f16a6e9-6089-4088-9307-8ca7c29b12e2` |
| `RAILWAY_SERVICE_ID` | `199b610b-f88e-4c4d-a9f1-6120cbf3d1ae` |
| `RAILWAY_ENVIRONMENT_ID` | `196c1b53-86cd-451f-81d0-aaf9cadc2e9a` |
| `BASE_URL` | `https://staging.nomsrecipes.com` (optional trailing slash) |

Use Railway's injected IDs. Do not redefine them as user variables. The expected
IDs are code-reviewed constants; copying only the flag or secret into another
environment cannot enable login. Recreating the service/environment or changing
its canonical hostname requires a reviewed code change. `NOMS_ENV` retains the
hosted production security configuration; local mode is rejected for this feature.
The normal startup validations, HTTPS cookies and HSTS remain active.

Only boolean `true` or `false` is accepted for the flag. An absent/empty flag is
disabled. An enabled flag outside the approved deployment, a missing/weak secret,
a secret equal to `SESSION_SECRET`, or a configured `COOKIE_DOMAIN` rejects
startup before the application's database/storage connections. Set the flag only on the staging
service, never as a shared project variable.

A public "Staging environment — test data only" banner identifies the approved
staging deployment even while the login feature is disabled. Only this boolean
marker is hydrated into the client; the login credential/digest is server-only.

The operator confirmed separate staging databases and media storage. The app
cannot prove data-store isolation from URLs alone: keep staging database/R2
credentials independent, and populate staging with synthetic data. The ordinary
OAuth client settings remain required by startup configuration.

## Credentials and requests

Configure `NOMS_STAGING_LOGIN_SECRET` as an independently generated random token
of 32 to 512 printable ASCII characters without spaces, tabs or line breaks.
Hexadecimal, Base64 (including `+`, `/` and `=` padding), and URL-safe tokens are
accepted as opaque strings; the app does not decode or normalize them. For example,
generate 32 random bytes with `openssl rand -base64 32` or `openssl rand -hex 32`
in a trusted secret-management workflow. Keep at least 12 distinct characters,
use a different value from `SESSION_SECRET`, and configure the exact same token in
Railway and Codex. Existing 64-character hexadecimal secrets continue to work.
Store it in the staging service and
Codex's approved secret bindings; do not paste its value into chat, source files,
URLs, shell arguments or logs. Keep the flag false until deployment and the Codex
credential binding are ready. Runtime/tool-required approvals still apply.

The endpoint requires `Authorization: Bearer <secret>`. Missing, wrong or duplicate
credentials receive 401. A supplied browser Origin must match the exact canonical
origin; backend requests may omit Origin because the bearer credential authorizes
them. Bodies and query parameters are rejected. There are no redirects or token
values in the response body. The existing OAuth-start IP rate limit applies,
including trusted-proxy client-IP resolution, before database work. Request Host
or forwarded headers never enable this feature; deployment configuration is the
authority, so Cloudflare origin-host rewriting does not grant login or break the
approved setup flow.

On success the response is 204 with one host-only, Secure, HttpOnly, SameSite=Lax
session cookie and `private, no-store` cache control. The dedicated test account
has reserved ID `38d6fd8a-1ec5-4791-b782-51ea46e11c7c`, initial username
`codex_staging_tester`, and email `codex-staging@noms.invalid`. The account is
created on the first authorized request. Account/session creation is atomic.
A username/email collision with an ordinary account fails instead of adopting
that account. Later requests reuse only the reserved identity. Treat its data as
shared synthetic test data; concurrent tests use the same account. It has ordinary
user permissions and no OAuth identity is fabricated or linked.

Sessions use the existing 15-minute expiry and normal rolling refresh/logout
behavior. This is an inactivity lifetime, not an absolute test-duration limit.
Disabling the flag prevents new sessions; previously issued sessions remain valid
until expiration/logout/revocation and can follow normal refresh behavior. Revoke
the test account's sessions as well when immediate access removal is required.
Rotating the login secret prevents further issuance with the old secret but does
not itself revoke session cookies.

## Browser setup

After the operator enables the deployed feature and the execution environment has
the staging secret binding, run:

```bash
python3 scripts/staging-session.py --output ../work/staging-auth.json
```

The helper uses the configured environment variable and the inherited HTTPS
proxy/CA trust. A configured proxy credential placeholder is passed through as
provided; the helper does not print or introspect it. Configure any proxy-based
credential injection for this exact staging destination. The helper identifies
itself as `Noms-Staging-Validation/1.0` because staging's Cloudflare edge rejects
urllib's default Python client with error 1010. If staging customizes
`NOMS_SESSION_COOKIE_NAME`, set the same non-secret cookie
name in the helper environment. Cloudflare cookies are ignored; only the app
session is exported. The helper requests only
`https://staging.nomsrecipes.com/auth/staging/session`, refuses all redirects and
validates the response cookie's scope, protections and expiry. It writes a
Playwright storage-state file atomically with mode 0600 and prints no credentials.
Do not enable HTTP wire tracing or Playwright API tracing during session setup.

Use that file when creating the browser context:

```python
context = await browser.new_context(storage_state="../work/staging-auth.json")
page = await context.new_page()
await page.goto("https://staging.nomsrecipes.com/recipes/new")
```

Keep the file outside the repository and delete it after testing. For a long test
run the browser's normal session refresh remains in effect; rerun setup after an
expired session. Normal logout uses `POST /auth/logout` with the canonical Origin.
The new helper never calls production, changes Railway variables or publishes data.
Only authenticated test actions explicitly performed afterwards can mutate staging.

## Local verification

The change passed 67 distinct focused native tests across staging login,
configuration, sessions, logout, request categorization and production router
regressions. Five Python helper tests cover exact destination/cookie scope,
redirect refusal, proxy placeholders, rejecting repository-local credential files,
and private atomic output. The helper tests are wired into CI. Configuration and
handler tests use synthetic credentials and disposable PostgreSQL databases; a
cookie issued by the staging handler was exercised against the actual SSR router and its
public banner, without serializing the login secret. The credential scanner also
recognizes the new secret key and reports filenames without values.

Server and WebAssembly checks, formatting and the dependency audit passed. Local
Rust 1.99 Clippy uses compatibility allowances for the unchanged
`chunks_exact_to_as_chunks` and server `result_large_err` findings. No package
versions or database schema changed; `subtle`, already in the lockfile, is now an
explicit server-only dependency for constant-time credential comparison.

Live Railway login and an interactive browser run are pending deployment and a
configured credential binding. Keep `NOMS_STAGING_LOGIN_ENABLED=false` until then.
