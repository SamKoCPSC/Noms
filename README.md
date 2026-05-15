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
| [Dioxus CLI](https://dioxuslabs.com/) (`dx`) | Dev server + hot reload | `cargo install dioxus-cli` |
| [just](https://just.systems/) | Dev commands (`just up`, `just lint`, etc.) | `cargo install just` · Windows: `winget install just` |
| [pgmold](https://crates.io/crates/pgmold) | Declarative database schema management | `cargo install pgmold` |
| [Docker](https://docker.com/) or [Rancher Desktop](https://rancherdesktop.io/) | Local Postgres, MinIO, Mock OAuth | Windows: Docker Desktop installer |
| `jq` | JSON parsing (used by health checks) | Arch: `sudo pacman -S jq` · Ubuntu: `sudo apt install jq` · macOS: `brew install jq` |

After installing Rust, add the WASM compilation target:

```bash
rustup target add wasm32-unknown-unknown
```

### 2. Configure Environment Variables

Copy the example environment file and adjust if needed:

```bash
cp .env.local.example .env.local
```

The defaults work out-of-the-box with the local Docker services (Postgres, MinIO, and a mock OAuth server).

### 3. Start the Development Environment

```bash
just up
```

This command will:
1. Check that Docker is running (start Docker Desktop or Rancher Desktop first if needed)
2. Launch local infrastructure services (Postgres, MinIO, Mock OAuth)
3. Wait for all services to become healthy
4. Apply the database schema using `pgmold`
5. Start the Dioxus dev server with hot reload

Stop everything with `Ctrl+C` (this also tears down the Docker containers).

## Available Commands

| Command | Description |
|---|---|
| `just up` | Start infrastructure + apply schema + launch dev server |
| `just down` | Tear down Docker containers |
| `just migrate` | Apply database schema changes (run after modifying `migrations/schema.sql`) |
| `just dev` | Start the Dioxus dev server only (infrastructure must already be running) |
| `just fmt` | Format code |
| `just lint` | Run fmt + clippy |
| `just check` | Check compilation without building |
| `just test` | Run tests |

## Local Infrastructure

The `docker-compose.yml` file provides three services for local development:

| Service | Image | Port | Purpose |
|---|---|---|---|
| `noms-postgres` | `postgres:17-alpine` | 5432 | Local database |
| `noms-minio` | `minio/minio:latest` | 9000 | Local S3-compatible storage |
| `noms-mock-oauth` | `ghcr.io/navikt/mock-oauth2-server` | 8082 | Mock OAuth2 provider for Google/GitHub/Apple |

## Database Schema

The database schema is managed declaratively with `pgmold`. The source of truth is `migrations/schema.sql`.

To add a new table or column:
1. Edit `migrations/schema.sql`
2. Run `just migrate` (or `just up` to restart everything)

`pgmold` will automatically detect the diff and apply the necessary `ALTER` statements.
