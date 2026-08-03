# Noms
Recipe Management And Sharing App

## Summary
Noms is a full-stack recipe management and sharing platform that combines personal recipe organization with social discovery and collaboration features.

### Key Features

- **Version history** — Track how a recipe evolves over time with full history of changes
- **Fork model** — Iterate on other user recipes while tracking attribution
- **URL import** — Import recipe data from other websites
- **Collections** — Organize recipes in folders
- **Community interactions** — Public profiles, shareable links, likes, and comments
- **Built-in calculator** — Easily scale recipes to specific proportions

Built with **Rust** and **Dioxus**, Noms compiles to a single codebase that runs on the web, desktop, and mobile.

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

For host-side Rust checks, add the WASM compilation target:

```bash
rustup target add wasm32-unknown-unknown
```

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

#### OAuth rate limiting behind a reverse proxy

OAuth start and callback requests are limited per client address. Forwarded
address headers are ignored unless the request's direct TCP peer is included
in the comma-separated `TRUSTED_PROXIES` setting. Entries may be individual IP
addresses or CIDR ranges. The default is empty; do not add loopback, private,
or platform ranges unless they are the verified addresses of the proxy that
connects directly to this application.

Railway currently documents its private proxy network as `100.0.0.0/8`, so a
Railway deployment can explicitly set:

```text
TRUSTED_PROXIES=100.0.0.0/8
```

The application accepts `X-Real-IP` or a strictly unwound
`X-Forwarded-For` chain from configured peers. If both are present, they must
identify the same client. Invalid configuration stops startup. A runtime with
Railway deployment metadata also refuses to start when `TRUSTED_PROXIES` is
missing or empty, preventing a silently shared proxy-wide bucket. Missing or
ambiguous request identity fails closed only on the rate-limited OAuth
endpoints.

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
