# NOMS-001: Write Design Document Outlining the High-Level Plan

**Status:** ✅ Done
**Phase:** Pre-work
**Created:** 2026-05-09
**Resolved:** 2026-05-09

## Description

Produce a comprehensive design document (`DESIGN.md`) covering all aspects of Noms before writing any implementation code. Establish product vision, technical architecture, data model, and visual direction as a single source of truth.

## Scope Delivered

### Product
- [x] Product vision & core metaphor ("GitHub for Recipes")
- [x] Target user personas (home cooks, power users, businesses)
- [x] Feature roadmap — Phase 1–6 with implementation dependencies
- [x] Competitive landscape analysis (Paprika, CopyMeThat, Yummly, ChefTap)

### Technical Architecture
- [x] Full-stack Dioxus + Axum pattern (shared Rust codebase → WASM frontend + native backend)
- [x] Multi-platform strategy (web, desktop webview, mobile webview, future native GPU)
- [x] Infrastructure: Railway (app + Postgres), Cloudflare R2 (images)
- [x] CI/CD pipeline — GitHub Actions (CI) + Railway native deployments (CD), staging/prod split
- [x] Database migration strategy — declarative `schema.sql`, additive-only policy

### Data Model
- [x] 14 tables: Users, OAuth Accounts, Sessions, Auth States, Recipes, RecipeVersions, ForkRelationships, Collections, CollectionRecipes, Follows, Comments, Likes, Tags, RecipeTags, Pantry, Variations, Meal Plans, Shopping Lists, Notifications
- [x] Fork lineage DAG with recursive CTE traversal + materialized paths
- [x] Multi-provider OAuth identity model (Google + Apple + GitHub) with email-based automatic account linking

### Feature Deep Dives
- [x] Recipe URL import pipeline — two-stage parse flow, schema.org extraction, heuristic fallbacks
- [x] Authentication — server-side sessions over JWTs, rolling expiry, HTTP-only cookies, multi-provider callback logic
- [x] Fork graph visualization — Reingold-Tilford layout, SVG DAG rendering, lazy-loaded generations
- [x] Image storage — presigned URLs, direct browser-to-R2 uploads, on-the-fly resizing, CDN caching by visibility
- [x] Offline usability — SQLite WASM (`op-sqlite`), shared query layer with PostgreSQL (~75% SQL overlap), 3-phase sync lifecycle, conflict resolution

### Visual Design
- [x] Styling strategy: Tailwind CSS + `dioxus-components` library
- [x] Aesthetic blend: Neumorphic + Glassmorphic + Subtle 3D
- [x] Color system — warm earth tones, light/dark mode (15 tokens each), animated gradient background
- [x] Theme toggle plumbing designed in, UI deferred

## Outcome

`DESIGN.md` — 2,338 lines. Serves as the single source of truth for all subsequent implementation work. All future phases reference this document rather than duplicating decisions elsewhere.
