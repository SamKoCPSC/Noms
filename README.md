# Noms

Production deployments must satisfy the fail-closed configuration and
least-privilege requirements in
[`docs/production-security.md`](docs/production-security.md).

Recipe Management And Sharing App

## Summary
Noms is a full-stack recipe management and sharing platform that combines personal recipe organization with social discovery and collaboration features.

### Key Features

- **Version history** — Track how a recipe evolves over time with full history of changes
- **Fork model** — Iterate on other user recipes while tracking attribution
- **URL import** — Import recipe data from other websites
- **Collections** — Organize recipes in folders
- **Kitchen workspace** — Manage separate pantries and grocery lists in one place. Track ingredient amounts and availability, find owned and collected recipes with all ingredients available, check individual recipes against a selected pantry, or build independent shopping lists from accessible recipes and manual items with serving-aware quantities and recipe context. Optional ingredient-name suggestions support pantry and grocery entry alongside custom names and household supplies.
- **Community interactions** — Public profiles, shareable links, likes, and comments
- **Built-in calculator** — Easily scale recipes to specific proportions
- **Public informational pages** — Server-rendered Home, About, and task-oriented How-to pages

Built with **Rust** and **Dioxus**, Noms compiles to a single codebase that runs on the web, desktop, and mobile.

## Roadmap Status

**NOMS-013 — Marketing Pages & 3D Feasibility Experiment:** In progress. The conventional Home, About, and How-to implementation is merged. Editorial approval, visual/asset documentation, the custom icon family, the complete production release gate, and the isolated 3D feasibility experiment remain outstanding. See the [NOMS-013 implementation plan](roadmap/implementation-plans/NOMS-013-marketing-pages-3d-experience.md) for checkpoint-level evidence and remaining work.

## Local Development Setup

### 1. Install Prerequisites

| Tool | Why | Install |
|---|---|---|
| [Rust](https://rustup.rs/) | Language | Follow rustup.rs installer |
| [just](https://just.systems/) | Dev commands (`just up`, `just lint`, etc.) | `cargo install just` · Windows: `winget install just` |
| [Docker](https://docker.com/) or [Rancher Desktop](https://rancherdesktop.io/) | Isolated app, PostgreSQL, MinIO, and Mock OAuth | Docker Desktop supports macOS; Docker Engine and Rancher Desktop support Linux |
| Git | Source control and separate development worktrees | Included with most developer environments |

Rust, the WASM target, Dioxus CLI, PostgreSQL client, and `pgschema` are
installed in the development image used by `just up`. Install them on the host
only if you also want to run standalone commands such as `just check`,
`just migrate`, or `just dev`.

MinIO and its `mc` client are also built from pinned official source versions
in that image because the former public MinIO container images are unavailable.
The first build downloads Go modules and compiles these tools; subsequent
builds can reuse Docker's cached layers. No MinIO registry login is needed.
Go tools compile on the Docker builder's native architecture and cross-compile
for the runtime image, avoiding emulated Go/GCC builds on Apple Silicon.

For host-side Rust checks, add the WASM compilation target:

```bash
rustup target add wasm32-unknown-unknown
```

For managed Codex cloud sessions, use the [Codex cloud development guide](docker/codex-cloud/README.md), including headless browser checks and screenshots. The normal local and production workflows remain unchanged.

### 2. Configure Environment Variables for Standalone Commands

`just up` generates a private environment for every invocation and does not
read or rewrite `.env.local`. To use standalone host-side migration or
development recipes, copy the example and adjust it as needed:

```bash
cp .env.local.example .env.local
```

The fixed example URLs are intended only for manually managed standalone
services. An isolated environment's generated values are stored with mode
`0600` under its printed runtime path.

#### Local OAuth addresses

Local environments use the printed application URL as their canonical origin.
If you open the same port using `localhost` instead of `127.0.0.1` (or vice
versa), the browser switches to the configured address while preserving the
path, query and fragment. Sign-in and provider-connect links already target that
address before client hydration. Cookies remain host-only; they are not shared
between the aliases. Use these links, or the canonical URL, when starting OAuth
through `dx serve`, whose development proxy replaces the original Host header.

This behavior is limited to `NOMS_ENV=local` with a plain loopback `BASE_URL`.
Production routing, registered callbacks and exact-origin logout checks are
unchanged. Other hostnames and different ports are not treated as aliases.

#### OAuth rate limiting behind a reverse proxy

OAuth start and callback requests are limited per client address. Select the
client address policy explicitly with `CLIENT_IP_POLICY`:

- `proxy-chain` (default): forwarding headers are accepted only from direct
  peers configured in comma-separated `TRUSTED_PROXIES` IPs/CIDRs. The chain
  is unwound from the right; `X-Real-IP` must agree when both headers exist.
  Railway runtimes using this policy require nonempty `TRUSTED_PROXIES`.
- `railway-edge`: opt-in for Railway public HTTP ingress. Requires Railway
  deployment metadata and one valid `X-Real-IP`; missing, malformed, or duplicate
  values fail closed without falling back to another header or the peer IP.
  `X-Forwarded-For` does not determine identity in this policy. `TRUSTED_PROXIES`
  is not used for identity, and may be omitted. Invalid supplied CIDRs still
  reject startup. Unknown or empty policy values also reject startup.

For Railway staging, set this on the **app service**, then redeploy:

```text
CLIENT_IP_POLICY=railway-edge
```

This setting declares that **all workloads able to reach the app privately in
its project/environment are trusted**. Verify the app has no public TCP proxy
and review every service and private-network sandbox sharing that environment.
Private callers bypass HTTP edge header sanitization and can supply arbitrary
`X-Real-IP` values. The application cannot authenticate edge origin from headers
or a stable proxy CIDR. Do not enable this policy for an untrusted private caller
or raw public TCP ingress. Railway's proxy address ranges are not guaranteed
stable; do not treat `100.0.0.0/8` as a durable ingress authentication contract.
See [Railway's confirmed header and network trust model](https://station.railway.com/questions/fastapi-uvicorn-trusted-proxy-configura-9c06fa12).

The policy applies to OAuth rate limiting and the normalized client identity
passed to server functions. Rejection logs record the policy and reason without
raw IP addresses, headers, or credentials. Keep `NOMS_ENV` independent of this
setting; it does not enable the Railway policy automatically.

The limiter is stored in process memory and therefore applies independently
to each application replica. Deployments requiring a global limit across
multiple replicas should enforce an upstream shared limit or move this state
to a shared store.

### 3. Start the Development Environment

```bash
just up
```

Each invocation creates a disposable Compose project with its own application,
PostgreSQL database, MinIO storage, Mock OAuth provider, network, writable
volumes, build output, Dioxus cache, browser cookie name, and dynamically
allocated loopback ports. The app runs in a development container and
bind-mounts the current worktree for hot reload. Startup applies extensions and
the declarative schema, but deliberately does not regenerate `.sqlx`.

When startup succeeds, it prints the instance ID and usable endpoints:

```text
Noms instance: noms-7f3a91
Application:   http://127.0.0.1:41783
MinIO media:   http://127.0.0.1:41784/noms-media
MinIO console: http://127.0.0.1:41785
Mock OAuth:    http://127.0.0.1:41786
PostgreSQL:    127.0.0.1:41787
Runtime state: /tmp/noms-dev/noms-7f3a91
```

Use an explicit lowercase name when you want a recognizable environment:

```bash
just up instance=agent-a
```

The project name becomes `noms-agent-a`. A duplicate active name is rejected
instead of attaching to or replacing the existing project.

Container builds default to eight parallel Cargo jobs per instance. Container
builds also use single-threaded LLVM linking with debug info and incremental
compilation disabled by default, and Dioxus builds the server and client
sequentially. Set `NOMS_CARGO_BUILD_JOBS` lower when running many cold
worktrees on a constrained Docker engine, or higher when additional CPU and
memory are available. Profile defaults can be overridden at the same time:

```bash
NOMS_CARGO_BUILD_JOBS=4 NOMS_CARGO_PROFILE_DEV_DEBUG=2 \
  NOMS_CARGO_PROFILE_DEV_CODEGEN_UNITS=64 \
  NOMS_CARGO_PROFILE_DEV_INCREMENTAL=true just up instance=agent-a
```

Simultaneous cold starts use a short-lived host startup slot so only one
memory-intensive initial Dioxus compile runs at a time. The slot is released
as soon as an app is healthy; it does not share any application or
infrastructure state, and all completed instances run concurrently.

### 4. Use a Separate Worktree per Concurrent Session

Only one active environment may bind-mount a given source checkout. Run every
concurrent agent session from a separate Git worktree so source files and
generated checkout state cannot overlap:

```bash
git worktree add ../noms-agent-a -b agent-a
git worktree add ../noms-agent-b -b agent-b

cd ../noms-agent-a
just up instance=agent-a
```

If a second process calls `just up` from the same worktree, it fails before
creating containers and identifies the instance holding the lock.

Stop everything with `Ctrl+C` (this also tears down the Docker containers).
Normal teardown removes that instance's containers, network, data volumes,
build caches, runtime files, and worktree lock without touching another
instance.

To inspect or recover an environment from another terminal:

```bash
just dev-instances
just dev-status instance=agent-a
just dev-logs instance=agent-a
just down instance=agent-a
```

`just down` intentionally requires an exact, validated instance name. It never
uses a wildcard or an implicit Compose project, and it refuses to remove
containers that do not carry matching Noms ownership labels.

## CI and Railway deployment

GitHub Actions owns formatting, WASM/server compilation and linting, database
tests, search integration tests, and security checks. Its PostgreSQL test job
installs the server binaries needed by `pgtemp` and adds their directory to
`PATH`. Both schema jobs expose the pinned `pgschema` executable on `PATH`.

The production Dockerfile only builds and packages the release application;
it does not rerun CI checks. This avoids spending Railway's limited build
budget compiling a separate test profile. The runtime still includes migration
tools and retains its non-root user and healthcheck.

On `main`, CI's `publish-image` job waits for all six validation jobs, builds
the production Dockerfile for `linux/amd64`, and publishes
`ghcr.io/samkocpsc/noms:main` plus `sha-<full-commit-sha>`. It uses the job's
`GITHUB_TOKEN` with `packages: write`; no registry password needs to be saved
as a repository secret. BuildKit layers are cached in GitHub Actions.
The workflow can also be run manually on `main` to retry a failed build.

Staging's `Noms-App` should use the public Docker image
`ghcr.io/samkocpsc/noms:main` as its Railway source. Keep the HTTP healthcheck
at `/` with a 300-second timeout and preserve its runtime variables and domain.
Enable image auto-updates if the Railway plan supports them; otherwise redeploy
after each successful image publication. For a rollback, select a published
`sha-<full-commit-sha>` tag or the immutable digest recorded in the CI summary.
GHCR packages initially default to private: the `noms` package must be made
public for Railway's Free plan to pull it. Credentials are injected by Railway
at runtime, never passed as Docker build arguments.

The release image includes the maintained vocabulary and its seed executable.
Database preparation applies extensions, schema and runtime grants, then invokes
`seed-vocabulary --if-changed` with deployment credentials before the server starts.
The application still receives only its restricted runtime database URL.

Repository-backed Railway services use the configured pre-deploy command:
`/usr/local/app/entrypoint.sh --prepare-database`. For image-backed services, set
that command under **Settings → Deploy → Pre-deploy Command** to gate rollout on
database preparation. Startup also performs preparation as a fallback when no
hook is configured. A subsequent unchanged check skips the ingredient scan and
all seed writes; schema/grant preparation remains idempotent.

The fingerprint covers parsed ingredient and source JSON and is compared with
the latest committed `vocabulary_seed_runs` receipt while holding the seed-writer
lock. Whitespace/key-order changes do not trigger loading. Changed inputs load
transactionally; failures and dry runs never advance the receipt. Concurrent
deployments serialize, and rolling back to an older vocabulary is checked against
the latest load rather than any historical receipt. Omitted existing IDs still
fail; retirement must be explicit. For manual database-drift checks or repairs,
run the seed command without `--if-changed` using deployment credentials.

The PostgreSQL service continues using the repository source with root directory
`/docker/postgres` and config file `/docker/postgres/railway.json`. App HTTP
healthchecks must not be applied to that service.

Run the workflow regression check with:

```bash
bash scripts/tests/test-ci-deployment-workflow.sh
```

Railway now pulls an existing image, avoiding its cold-build time limit.

## Available Commands

| Command | Description |
|---|---|
| `just up [instance=<name>]` | Create an isolated app and infrastructure environment |
| `just down instance=<name>` | Tear down exactly one environment and all its writable volumes |
| `just dev-instances` | List active, failed, and abandoned environments |
| `just dev-status instance=<name>` | Show metadata and Compose state for one environment |
| `just dev-logs instance=<name>` | Follow service logs for one environment |
| `just isolated-dev-config-test` | Run fast orchestration and Compose configuration tests |
| `just isolated-dev-test` | Run the Docker-backed two-worktree concurrency test |
| `just migrate` | Apply database schema changes (run after modifying `migrations/schema.sql`) |
| `just dev` | Start the Dioxus dev server only (infrastructure must already be running) |
| `just fmt` | Format code |
| `just lint` | Run fmt + clippy |
| `just check` | Check compilation without building |
| `just test` | Run tests |

## Local Infrastructure and Isolation

The Compose project contains the hot-reloading app, PostgreSQL, MinIO, a MinIO
bucket initializer, Mock OAuth, and a schema initializer. Containers use
Compose DNS and container ports for server-to-server traffic. Only
browser/developer-facing listeners are published, always on `127.0.0.1` with
Docker-assigned ports.

PostgreSQL data, MinIO data, Cargo home and target output, and Dioxus session
data use project-scoped volumes. Dioxus CLI 0.7.9 configures its session path
through `DIOXUS_SESSION_CACHE_DIR`, the supported equivalent of a
`--session-cache-dir` option. Session cookies include the instance ID because
browsers scope `localhost` cookies by host rather than port.

Both environments still share the host Docker daemon, image/build layers, CPU,
memory, disk bandwidth, and network bandwidth. Those resources are immutable
cache or host capacity rather than mutable application state.

### Troubleshooting

- **Docker is unavailable:** Start Docker Desktop, Rancher Desktop, or Docker
  Engine, wait for `docker info` to succeed, and rerun `just up`.
- **An instance name already exists:** Inspect it with
  `just dev-status instance=<name>` or remove only that instance with
  `just down instance=<name>`.
- **The worktree is locked:** Use `just dev-instances` to find the owner. If the
  process was abandoned, run the targeted `just down` command. Use a separate
  Git worktree for another active environment.
- **Startup or schema initialization fails:** The error prints the instance ID
  and preserved `compose.log` path. Inspect the metadata/logs, then run the
  targeted `just down` command before retrying.
- **A port cannot be allocated:** Docker normally assigns all ports. Clean up
  the named failed instance and retry to create a fresh project.
- **Hot reload does not see changes:** Confirm you are editing the worktree
  printed by `just dev-status`, not another agent's checkout. On macOS, also
  confirm that Docker Desktop is allowed to share the worktree directory.
- **Files have unexpected ownership:** Build and Dioxus output live in Docker
  volumes rather than the bind-mounted checkout. Run targeted teardown to
  remove those volumes; do not delete another instance's volumes manually.

## Database Schema

The database schema is managed declaratively with `pgschema`, with
`migrations/schema.sql` as the source of truth. Because the schema uses
extension-provided types and indexes, `pgschema` validates it through the
extension-enabled target database rather than its embedded PostgreSQL instance.

To add a new table or column:
1. Edit `migrations/schema.sql`
2. Restart an isolated environment with `just up`, or run `just migrate`
   against a manually configured standalone database

`pgschema` will automatically detect the diff and apply the necessary changes.
