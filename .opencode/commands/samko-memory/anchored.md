# Anchored Summary — NOMS-008 Recipe CRUD

## Goal
- Implement NOMS-008 (Recipe CRUD) and perform comprehensive E2E testing of all features

## Constraints & Preferences
- No new crate dependencies (sqlx, uuid, chrono, serde_json, tokio, pgtemp already in Cargo.toml)
- All server-side code must be `#[cfg(feature = "server")]` guarded
- Follow existing code patterns (Dioxus 0.7.1 fullstack, Axum 0.8, SQLx 0.8, PostgreSQL, Tailwind CSS)
- E2E testing via Chrome DevTools on running dev server at `localhost:8080`

## Progress
### Done
- **Checkpoints 1-7**: All recipe CRUD features implemented and passing 183 tests
- **Dev server**: Running at `localhost:8080`, app loads correctly
- **E2E testing complete**:
  - ✅ Authentication: JWT session cookie (`noms_session`) works, user "finalusername" authenticated
  - ✅ Create recipe: Form at `/recipes/new` works, redirects to detail page
  - ✅ View recipe detail: `/recipes/{id}` shows title, tags, times, ingredients, steps, author
  - ✅ Edit recipe: `/recipes/{id}/edit` pre-populates all fields, saves changes, redirects to detail
  - ✅ Delete recipe: Confirm dialog → deletes recipe → redirects to dashboard
  - ✅ Dashboard: Shows recipe list / empty state with "No recipes yet"
  - ✅ Navigation: Sidebar links work (Home, Dashboard, Explore, New Recipe)

### In Progress
- None — all CRUD features verified E2E

### Blocked
- **Git commit**: `git add`/`git commit` blocked by bash permission rules (only `git branch`, `git switch`, `git checkout` allowed) — commit message ready for manual execution

## Key Decisions
- User ID extracted from session via `extract_user_id_from_fullstack()` in all server functions (prevents spoofing)
- Recipe ID as `String` parameter parsed to `Uuid` inside functions
- Duplicate form logic between create/edit pages (extraction deferred)
- Instructions stored as serialized text; `parse_instructions()` reverses the format for editing
- Delete uses `web_sys::window().confirm()` dialog (full modal deferred)
- Three-layer auth: middleware + `AuthRequired` component + server function ownership check
- Sessions use JWT with `sub = session_id` (not user_id), signed with HS256

## Bugs Fixed
- **Edit page stuck on "Loading recipe..."**: `use_resource` with server functions in Dioxus 0.7 fullstack mode never resolves `pending()` state despite API calls succeeding. Fixed by replacing `use_resource` with `use_effect` + `spawn(async move { ... })` pattern with manual `is_loading` signal.
- **Textarea values not bound**: Description and step textareas in edit form missing `value:` attribute — added `value: description().clone()` and `value: steps()[idx].text.clone()` respectively.

## Next Steps
- Manual git commit (bash blocked):
  ```
  git add src/pages/recipe_edit.rs
  git commit -m "fix: replace use_resource with use_effect+spawn in recipe edit page

  use_resource with server functions in Dioxus 0.7 fullstack mode never
  resolves the pending() state, leaving the edit page stuck on 'Loading
  recipe...' despite successful API responses. Switch to use_effect with
  spawn(async move { ... }) and manual is_loading signal.

  Also bind value attributes to description and step textareas so that
  pre-populated data is visible in the form."
  ```
- Optional: Create a helper to generate test sessions for future E2E testing
- Optional: Extract shared form component from create/edit pages

## Critical Context
- Branch: `NOMS-008-recipe-crud`
- 183 tests passing across all checkpoints
- Both `cargo check --features server` and `cargo check --target wasm32-unknown-unknown` compile clean
- Dev server at `localhost:8080` working — app loads with correct navigation
- OAuth-only authentication (Google/GitHub), no email/password login
- 5 test users exist in database; sessions created manually via SQL + JWT for E2E testing
- Session cookie name: `noms_session`
- JWT secret: `SESSION_SECRET` env var (from `.env.local`)
- **E2E testing pattern**: Create session row in DB → generate JWT token → set cookie via `document.cookie` → reload page
- Known suggestions (non-blocking): duplicate `format_relative_time()`, double-cloning in `use_resource` closures, loading flash on pagination, tags dirty-tracking in edit form, index-based keys in dynamic lists

## Relevant Files
- `src/db/mod.rs`: Recipe/RecipeTag structs, 8+ query functions, tests
- `src/api/recipe.rs`: 6 server functions including `get_recipe_tags`
- `src/pages/recipe_new.rs`: Create recipe form
- `src/pages/recipe_detail.rs`: Detail view with delete functionality
- `src/pages/recipe_edit.rs`: Edit recipe page (**BUG FIX: use_resource → use_effect+spawn**)
- `src/pages/dashboard.rs`: Dashboard with recipe list and pagination
- `src/components/base/recipe_card.rs`: Recipe card component
- `src/main.rs`: Routes including `RecipeDetail { id: String }` and `RecipeEdit { id: String }`
- `src/middleware/auth.rs`: UUID-aware recipe route matching
- `src/auth/session.rs`: Session creation (`create_session`), JWT signing, cookie building
- `migrations/schema.sql`: Recipe tables and indexes
- `src/test_utils.rs`: Test schema with recipe DDL
- `src/types.rs`: Shared `Recipe`/`RecipeListResponse` for client-server boundary
- `.opencode/brief.md`: Full task brief with blueprints and review verdicts
