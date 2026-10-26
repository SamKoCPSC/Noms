# Production security configuration

Production startup is intentionally fail-closed. `NOMS_ENV=local` is the only
mode that permits loopback OAuth endpoints and development credentials. Any
other value, including an unset value, is treated as production.

## Railway deployment contract

Set `NOMS_ENV=production` in Railway for clarity and configure all of the
following variables. Do not copy the development values from
`.env.local.example`.

Railway-provided project/environment identifiers make `NOMS_ENV=local` a
startup error, so a deployment cannot accidentally enable the development
credential and loopback-endpoint path.

| Variable | Production requirement |
|---|---|
| `BASE_URL` | Public HTTPS origin only, with no path, query, fragment, or credentials. |
| `SESSION_SECRET` | At least 32 high-entropy bytes. Generate independently for each environment, for example with `openssl rand -base64 48`. |
| `DATABASE_URL` | Credentialed PostgreSQL URL for the restricted application runtime role. |
| `MIGRATION_DATABASE_URL` | Credentialed PostgreSQL URL for a different schema-owning role targeting the same database. |
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` | Real Google OAuth application credentials. |
| `GOOGLE_AUTH_URL` | `https://accounts.google.com/o/oauth2/v2/auth` |
| `GOOGLE_TOKEN_URL` | `https://oauth2.googleapis.com/token` |
| `GOOGLE_USERINFO_URL` | `https://openidconnect.googleapis.com/v1/userinfo` |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` | Real GitHub OAuth application credentials. |
| `GITHUB_AUTH_URL` | `https://github.com/login/oauth/authorize` |
| `GITHUB_TOKEN_URL` | `https://github.com/login/oauth/access_token` |
| `GITHUB_USERINFO_URL` | `https://api.github.com/user` |
| `GITHUB_EMAILS_URL` | `https://api.github.com/user/emails` |
| `R2_ENDPOINT`, `R2_PUBLIC_URL` | HTTPS R2 API and public-media URLs. |
| `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET` | A non-development credential pair restricted to the one media bucket. |

Leave `COOKIE_DOMAIN` unset for host-only session cookies. Set it only when
sessions deliberately need to span trusted subdomains; it must be a bare DNS
domain rather than a URL.

Client address handling defaults to `CLIENT_IP_POLICY=proxy-chain`.
`TRUSTED_PROXIES` must contain only verified direct-proxy IPs/CIDRs; do not use
`0.0.0.0/0` or `::/0`. Railway runtimes using that policy require a nonempty
value. Non-proxy local runtimes can use an empty value for direct peer identity.

`CLIENT_IP_POLICY=railway-edge` explicitly selects Railway's overwritten
`X-Real-IP` for client identity, requires Railway deployment metadata, and rejects
missing, malformed, or duplicate values. It does not require agreement with
`X-Forwarded-For` or use `TRUSTED_PROXIES` for identity. Enable it only when the
app has no public TCP proxy and every workload sharing its project/environment
(including databases and private-network sandboxes) is trusted to supply client
identity. Private callers bypass edge sanitization; no request-level mechanism
proves edge origin. Railway's ingress CIDRs are not a stable trust contract.
Review that network boundary whenever adding workloads or public ingress.
Unknown/empty policy values fail startup; `NOMS_ENV` does not select the policy.
See the README's OAuth rate limiting section for the explicit deployment setup.

Railway is configured to build the repository Dockerfile, use `/` as its
health check, and restart failed processes. The container runs as the
unprivileged `noms` user. The server, entrypoint, and migration files remain
root-owned and non-writable so a compromised runtime process cannot replace a
privileged restart artifact. Railway terminates public TLS; the application
adds a one-year HSTS header to every production response. Verify HTTP-to-HTTPS
redirection and HSTS on the deployed custom domain after every routing change.

Keep the application at one replica until the process-local OAuth request
windows and per-user media transaction lock are replaced by a shared database
or distributed implementation. Multiple replicas would weaken aggregate rate
and quota serialization even though tenant authorization remains intact.

## Database roles and time budgets

The migration role creates extensions and owns the declarative schema. The
entrypoint then applies `migrations/runtime-grants.sql`, granting the runtime
role only schema usage plus table/sequence data-plane privileges. It runs the
packaged vocabulary loader with `--if-changed`, supplying the migration URL as
`DATABASE_URL` only for that child process. The runtime role retains SELECT-only
access to maintained vocabulary. A matching latest committed ingredient/source
fingerprint skips the entry scan and all seed writes; failures prevent startup.
Migration credentials are removed from the server process environment before launch.
The entrypoint supplies only a non-secret completion marker to the server.

`entrypoint.sh --prepare-database` performs the same preparation and exits without
launching the server. Repository Railway configs use it as the pre-deploy hook;
image-backed services require the equivalent dashboard setting. Startup also
prepares the database so image deployments without that setting still seed.
The hook and startup share transaction-serialized checksum checks, so unchanged
catalogs do not load twice. The fingerprint assumes vocabulary is maintained via
this loader; omit `--if-changed` for an explicit database-drift check/repair.

The runtime role must be standalone: no inherited roles, `CREATEDB`,
`CREATEROLE`, superuser, replication, row-security bypass,
extension-management, schema-creation, or object-ownership privileges. The
entrypoint checks these properties before granting data-plane access and
aborts deployment if they are violated. It also removes public temporary-table
and schema-creation access for the application database, removes any direct
runtime-role database ACLs, and then restores only `CONNECT`.

Production requires PostgreSQL 17 or newer because the transaction-wide
budget uses `transaction_timeout`; an older server is rejected while opening
the pool. Every pooled connection receives these defaults:

| Variable | Default | Accepted range |
|---|---:|---:|
| `DB_MAX_CONNECTIONS` | 5 | 1–100 |
| `DB_ACQUIRE_TIMEOUT_SECS` | 10 s | 1–60 s |
| `DB_IDLE_TIMEOUT_SECS` | 300 s | 1–3600 s |
| `DB_STATEMENT_TIMEOUT_MS` | 15000 ms | 100–300000 ms |
| `DB_TRANSACTION_TIMEOUT_MS` | 30000 ms | 100–600000 ms |
| `DB_IDLE_TRANSACTION_TIMEOUT_MS` | 30000 ms | 100–600000 ms |

Transaction and idle-in-transaction limits may not be shorter than the
statement limit. Malformed, zero, or out-of-range overrides abort startup.

## Object-storage privilege boundary

The application R2 token should be scoped to the configured bucket and only
the object operations used by Noms: list, head/read, put, and delete. It should
not manage accounts, users, unrelated buckets, custom domains, lifecycle
rules, or access policies. Use a separate administrative credential outside
the application for bucket provisioning.

## Release verification

Run the same gates used by CI:

```bash
just ci
```

This includes the pinned RustSec audit, expiring-exception validation, tracked
credential scan (including project-specific OAuth, session, and R2 assignment
formats), immutable container/action checks, compilation, linting, tests, and
schema-plan regression tests. The PostgreSQL extension packages are exact
versions and the ParadeDB package is checksum-verified; update those pins only
as a reviewed dependency change. A production Docker build should also be
completed before release. Never put discovered secret values into CI logs or
review reports.
