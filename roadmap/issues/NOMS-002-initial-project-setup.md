# NOMS-002: Initial Project Setup — Repository, Workspace & Infrastructure

**Status:** ⚪ Backlog
**Phase:** Pre-work
**Created:** 2026-05-09

## Description

Establish the foundational codebase and external infrastructure so that subsequent Phase 1 issues have something to build against. This is purely scaffolding — no product features, just the plumbing that makes development possible.

## Scope

### Rust Workspace & Package Structure
- [ ] Initialize Cargo workspace (`Cargo.toml` root) matching Dioxus conventions:
  - `packages/app/` — Dioxus frontend (WASM target) + shared types
  - `packages/server/` — Axum backend (native target)
  - `packages/db/` — Database layer (SQLx types, query functions, migrations)
- [ ] Add foundational dependencies to workspace Cargo.toml:
  - Dioxus fullstack (`dioxus`, `dioxus-fullstack`, `dioxus-router`)
  - Axum + tower-http for server
  - SQLx with PostgreSQL feature for database
  - `tracing` + `tracing-subscriber` (JSON structured logs)
  - `aws-sdk-s3` for Cloudflare R2 integration
- [ ] Configure `rustfmt` and `clippy` at workspace level (`Cargo.toml` `[workspace.lints]`)

### Railway Infrastructure
- [ ] Create Railway project with two services:
  - **App service** — Rust binary deployment (will host both Axum + Dioxus SSR)
  - **PostgreSQL database** — Railway managed Postgres instance
- [ ] Configure environment variables on Railway:
  - Database URL (`DATABASE_URL`)
  - R2 credentials (`R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_ACCOUNT_ID`)
  - Google OAuth client ID/secret placeholders (to be filled in Phase 1 auth work)
- [ ] Verify database connectivity from local dev environment

### Cloudflare Infrastructure
- [ ] Create R2 bucket (`noms-media`) with structured prefix layout:
  ```
  noms-media/
  ├── originals/{user_id}/{image_uuid}.{ext}
  └── avatars/{user_id}/avatar.{ext}
  ```
- [ ] Configure public access path for recipe images (needed for static pages + SEO)
- [ ] Enable Cloudflare Images on-the-fly transformation for the project zone

### CI/CD Skeleton
- [ ] Create `.github/workflows/ci.yml` with minimal pipeline:
  - `cargo check` — compile verification
  - `cargo clippy` — linting
  - `cargo test` — run tests (will be empty initially)
- [ ] Configure Railway deployment trigger on `main` branch pushes

### Development Ergonomics
- [ ] Add `.gitignore` tuned for Rust projects (`target/`, `.env.local`, IDE files)
- [ ] Create `justfile` or `Makefile` with common dev commands:
  - `just dev` — run local development server (Axum + Dioxus hot reload)
  - `just migrate` — run database migrations locally
  - `just lint` — fmt + clippy
- [ ] Add `docker-compose.yml` for local PostgreSQL (mirrors Railway Postgres version)

## Acceptance Criteria

- [ ] `cargo check --workspace` passes with zero errors and zero warnings
- [ ] `cargo test --workspace` runs (may have 0 tests initially, but command succeeds)
- [ ] Local PostgreSQL container starts via `docker-compose up` and accepts connections
- [ ] Railway project exists with app service + database provisioned
- [ ] R2 bucket exists and is accessible from local environment using test credentials
- [ ] Pushing to `main` triggers CI pipeline on GitHub Actions (green checkmark)

## Outcome

A green-field Rust workspace that compiles, a running local database for development, Railway infrastructure ready for deployment, and CI catching basic errors on every push. Every subsequent Phase 1 issue builds inside this skeleton.
