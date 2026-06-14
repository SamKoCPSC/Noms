# NOMS-008: Recipe CRUD + Visibility & Discovery — Implementation Plan

**Issue:** [NOMS-008-recipe-crud.md](../issues/NOMS-008-recipe-crud.md)
**Created:** 2026-06-08
**Updated:** 2026-06-13 (expanded scope: visibility + discovery)
**Approach:** Bottom-up by dependency, 11 incremental checkpoints

---

## Checkpoint 1: Database schema + Rust types + queries

**Files:** `migrations/schema.sql`, `src/db/mod.rs`

**Schema additions** (append to `schema.sql`, additive-only):
- `recipes` table: `id UUID`, `owner_id UUID`, `title`, `description`, `is_public`, `prep_time_min`, `cook_time_min`, `total_time_min`, `servings`, `ingredients JSONB`, `steps JSONB`, `created_at`, `updated_at`
- `recipe_tags` table: `recipe_id UUID`, `tag TEXT`, `PRIMARY KEY(recipe_id, tag)`
- Indexes: `idx_recipes_owner_id`, `idx_recipes_updated_at`

**Rust types** (append to `src/db/mod.rs`):
- `Recipe { id: Uuid, owner_id: Uuid, title: String, description: Option<String>, is_public: bool, prep_time_min: Option<i32>, cook_time_min: Option<i32>, total_time_min: Option<i32>, servings: Option<i32>, ingredients: serde_json::Value, steps: serde_json::Value, created_at: DateTime<Utc>, updated_at: DateTime<Utc> }`
- `RecipeTag { recipe_id: Uuid, tag: String }`

**Query functions** (8 total):
- `insert_recipe()` — create new recipe row, return `Recipe`
- `get_recipe_by_id()` — fetch recipe by ID
- `get_recipe_by_id_and_owner()` — ownership-gated lookup (returns `None` if owner doesn't match)
- `get_recipes_by_owner()` — list user's recipes with `LIMIT`/`OFFSET` pagination
- `update_recipe()` — overwrite recipe row (title, description, times, servings, ingredients, steps), refresh `updated_at`
- `delete_recipe()` — delete recipe (tags cascade via FK)
- `insert_recipe_tags()` — delete old tags + insert new tags for a recipe
- `get_recipe_tags()` — fetch tags for a recipe

**Error type addition:**
- `DbError::RecipeNotFound` — for ownership-gated lookups where recipe exists but user isn't the owner

**Verify:**
- `cargo test --features server` — all 8 query functions tested against local Postgres
- `cargo check --target wasm32-unknown-unknown` — zero errors (types are `#[cfg(feature = "server")]`)
- Migration applies cleanly on fresh DB via `just migrate`

**Risk:** Low. Straightforward SQL + SQLx pattern matching existing code.

---

## Checkpoint 2: Server functions (API layer)

**File:** `src/api/recipe.rs` (new module)

**Server functions** (5 total, all `#[server]`):

| Function | Signature | Purpose |
|----------|-----------|---------|
| `create_recipe` | `(title, description, prep_time, cook_time, total_time, servings, ingredients_json, steps_json, tags)` | Insert recipe + tags atomically in a transaction |
| `get_recipe` | `(recipe_id_str, user_id_str)` | Fetch recipe (ownership-gated) |
| `update_recipe` | `(recipe_id_str, user_id_str, title, description, prep_time, cook_time, total_time, servings, ingredients_json, steps_json, tags)` | Overwrite recipe + upsert tags |
| `delete_recipe` | `(recipe_id_str, user_id_str)` | Delete recipe (ownership-gated) |
| `list_my_recipes` | `(user_id_str, offset, limit)` | Paginated recipe list for dashboard |

**Serialization types** (shared between client and server):
- `Ingredient { amount: Option<String>, unit: Option<String>, name: String, note: Option<String> }` — `Serialize + Deserialize`
- `Step { text: String, photo_url: Option<String> }` — `Serialize + Deserialize`
- `RecipeFormData { title, description, prep_time_min, cook_time_min, total_time_min, servings, ingredients: Vec<Ingredient>, steps: Vec<Step>, tags: Vec<String> }` — form payload
- `RecipeResponse { recipe: Recipe, tags: Vec<String>, owner: UserProfile }` — detail response
- `RecipeSummary { id, title, description, created_at, updated_at }` — list response

**Module registration:** Add `#[cfg(feature = "server")] mod api;` to `main.rs`, create `src/api/mod.rs` re-exporting `recipe`.

**Verify:**
- `cargo check --features server` — all server functions compile
- Manual test: call `create_recipe` server function from a test component, verify DB rows
- Ownership gating: calling `get_recipe` with wrong `user_id` returns error

**Risk:** Medium. First use of `#[server]` macro for recipe data. Serialization of JSONB fields needs careful handling.

---

## Checkpoint 3: Create recipe form

**Files:** `src/pages/recipe_new.rs`, `src/components/base/ingredient_row.rs` (new), `src/components/base/step_row.rs` (new), `src/components/base/recipe_form.rs` (new)

**`IngredientRow` component:**
- State: `amount`, `unit`, `name`, `note` (signals)
- Render: 4 input fields + "Remove" button
- Props: `ingredient: Signal<Ingredient>`, `on_remove: Callback`
- Validation: `name` is required (show error if empty on submit)

**`StepRow` component:**
- State: `text` (signal), `index` (prop)
- Render: textarea + "Remove" button + reorder buttons (up/down arrows)
- Props: `step: Signal<Step>`, `index: usize`, `total: usize`, `on_remove: Callback`, `on_move_up: Callback`, `on_move_down: Callback`
- Validation: `text` is required

**`RecipeForm` component (shared by create + edit):**
- Props: `initial_data: Option<RecipeFormData>`, `on_submit: Callback<RecipeFormData>`, `is_editing: bool`
- State: title, description, prep/cook/total time, servings, ingredients (Vec<Ingredient>), steps (Vec<Step>), tags (text input)
- Dynamic ingredient management: add/remove rows, each row is an `IngredientRow`
- Dynamic step management: add/remove rows, reorder via up/down buttons
- Tag input: comma-separated text, parsed on submit
- Form validation: title required, at least 1 ingredient and 1 step required
- Submit: collect all fields into `RecipeFormData`, call `on_submit` callback
- Cancel button: navigates to dashboard

**`recipe_new.rs` changes:**
- Replace empty form shell with `RecipeForm { initial_data: None, on_submit: create_handler, is_editing: false }`
- `create_handler`: calls `create_recipe` server function, handles success (redirect to detail) and error (show message)
- Loading state during save

**Verify:**
- Fill out form with title, 3 ingredients, 4 steps, save → recipe appears in DB
- Cancel button navigates to dashboard
- Validation: empty title blocked, empty ingredient name blocked, empty step text blocked
- Reorder steps works (up/down buttons)
- `cargo clippy --target wasm32-unknown-unknown` — zero warnings

**Risk:** Medium. Dynamic form state management in Dioxus can be tricky with nested signals.

---

## Checkpoint 4: Recipe detail view

**Files:** `src/pages/recipe_detail.rs`, `src/main.rs` (route change)

**Route change:** Change `RecipeDetail { id: i32 }` to `RecipeDetail { id: String }` in `main.rs`. The `id` is a UUID string from the URL.

**Auth middleware change:** Update `is_numeric_id_route` to `is_uuid_route` for `/recipes/` paths (check for UUID format instead of `i32`). Keep `is_numeric_id_route` for `/collections/` (unchanged for now).

**`recipe_detail.rs` changes:**
- Fetch recipe via `get_recipe` server function on mount (use `use_resource`)
- Loading state: show `LoadingSpinner` while fetching
- Error state: "Recipe not found" or "You don't have permission to view this recipe"
- Render:
  - Title, description, prep/cook/total time, servings
  - Ingredients as styled list (amount + unit + name + note)
  - Steps as numbered ordered list
  - Tags as chips below title
  - Author attribution: avatar + username + created date
  - "Edit" and "Delete" buttons in header (only visible to owner)

**Verify:**
- Navigate to `/recipes/{uuid}` → recipe renders correctly
- Ingredients display with amounts and units
- Steps display as numbered list
- Non-owner sees "You don't have permission" error
- Loading state shows spinner during fetch
- `cargo clippy` clean on both targets

**Risk:** Low. Standard data fetching and rendering.

---

## Checkpoint 5: Dashboard recipe list

**Files:** `src/pages/dashboard.rs`, `src/components/base/recipe_card.rs` (new)

**`RecipeCard` component:**
- Props: `recipe: RecipeSummary`
- Render: Card with title, description snippet (truncated to 120 chars), creation date
- Click handler: navigates to `/recipes/{id}`
- Styling: matches existing card design system

**`dashboard.rs` changes:**
- Fetch recipes via `list_my_recipes` server function on mount (use `use_resource`)
- Replace `EmptyState` with recipe grid when recipes exist
- Keep existing `EmptyState` for zero-recipe case
- Grid: responsive CSS grid (1 col mobile, 2 col tablet, 3 col desktop)
- "New Recipe" button in header (already exists)
- Pagination: "Load more" button at bottom (increments offset by 12)

**Verify:**
- Dashboard shows recipe cards for user's recipes
- Clicking card navigates to detail page
- Empty state shows when no recipes exist
- "Load more" loads next page of recipes
- Recipes sorted by `updated_at DESC`

**Risk:** Low. Standard list rendering.

---

## Checkpoint 6: Edit recipe

**Files:** `src/pages/recipe_edit.rs` (new), `src/main.rs` (new route)

**Route addition:** `#[route("/recipes/:id/edit")] RecipeEdit { id: String }`

**Auth middleware:** Add `/recipes/:id/edit` to protected paths (UUID pattern match).

**`recipe_edit.rs`:**
- Fetch recipe on mount via `get_recipe` server function
- Pre-populate `RecipeForm` with existing data: `RecipeForm { initial_data: Some(formData), on_submit: edit_handler, is_editing: true }`
- `edit_handler`: calls `update_recipe` server function (overwrites row + upserts tags)
- On success: redirect to detail page
- On error: show error message
- Loading state during fetch and save

**Verify:**
- Click "Edit" on detail page → edit form loads with existing data
- Modify title and ingredients, save → recipe row overwritten in DB
- `updated_at` timestamp refreshed on recipe
- Redirect back to detail page shows updated data

**Risk:** Low. Reuses RecipeForm from checkpoint 3.

---

## Checkpoint 7: Delete recipe

**Files:** `src/pages/recipe_detail.rs`

**Delete flow:**
- "Delete" button triggers confirmation dialog (built-in `window.confirm` or custom modal)
- Confirmation message: "Delete this recipe? This will permanently remove the recipe."
- On confirm: call `delete_recipe` server function
- On success: redirect to `/dashboard`
- On error: show toast message "Failed to delete recipe"

**Verify:**
- Click "Delete" → confirmation dialog appears
- Confirm → recipe deleted from DB, redirect to dashboard
- Cancel → dialog closes, recipe still exists
- Recipe tags are cascaded (FK ON DELETE CASCADE)
- Non-owner cannot see delete button

**Risk:** Low. Simple delete flow.

---

## Dependencies Summary

| Crate | Feature | Purpose |
|-------|---------|---------|
| `serde_json` | both | JSONB ingredient/step serialization, server function payloads |
| `uuid` | both | Recipe ID in URL, server function params |
| `chrono` | both | Timestamp serialization in responses |
| `sqlx` | `server` | Already present, no changes needed |
| `dioxus` | both | Already present, `#[server]` macro for API functions |

## File Structure

```
src/
├── api/
│   ├── mod.rs                    # Re-exports recipe module
│   └── recipe.rs                 # Server functions + serialization types
├── db/
│   └── mod.rs                    # + Recipe, RecipeTag types + 8 query functions
├── components/
│   └── base/
│       ├── ingredient_row.rs     # Single ingredient input row
│       ├── recipe_card.rs        # Recipe preview card for dashboard
│       ├── recipe_form.rs        # Shared create/edit form
│       └── step_row.rs           # Single instruction step input row
├── middleware/
│   └── auth.rs                   # Updated: UUID route matching for /recipes/:id
├── pages/
│   ├── dashboard.rs              # Recipe card grid + list_my_recipes
│   ├── recipe_detail.rs          # Full recipe view with edit/delete
│   ├── recipe_edit.rs            # Edit page (new)
│   └── recipe_new.rs             # Full create form with save logic
└── main.rs                       # + RecipeEdit route, UUID param for RecipeDetail
```

## Migration Notes

- Schema is additive-only: `CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`
- No `ALTER TABLE` or `DROP` statements
- Safe to apply on existing database (auth tables unaffected)
- Applied via existing `just migrate` command (pgschema)

## Testing Strategy

Each checkpoint has self-contained verification:
- **Checkpoint 1:** Unit tests for all 8 query functions against test DB
- **Checkpoint 2:** Compile check + manual server function test
- **Checkpoint 3-7:** Manual E2E testing in browser (create → view → edit → delete)
- **All checkpoints:** `cargo clippy` clean on both `wasm32` and `x86_64` targets

## Rollback Plan

If any checkpoint reveals issues:
- Schema tables can be dropped manually: `DROP TABLE recipe_tags, recipes CASCADE`
- Code changes are isolated to new files + minimal modifications to existing files
- Each checkpoint is independently reversible

---

## Checkpoint 8: DB + API for public access

**Files:** `migrations/schema.sql`, `src/db/mod.rs`, `src/api/recipe.rs`

**Schema changes** (append to `schema.sql`, additive-only):
- Add `visibility` column: `ALTER TABLE recipes ADD COLUMN visibility VARCHAR(20) NOT NULL DEFAULT 'private'`
- Add CHECK constraint: `ALTER TABLE recipes ADD CONSTRAINT valid_visibility CHECK (visibility IN ('private', 'unlisted', 'public'))`
- Add partial index for public recipes: `CREATE INDEX IF NOT EXISTS idx_recipes_visibility_created ON recipes(visibility, created_at DESC) WHERE visibility = 'public'`
- Update `insert_recipe()` to accept `visibility` parameter
- Update `update_recipe()` to accept `visibility` parameter

**New DB query functions** (4 total):
- `get_public_recipes_paginated()` — `WHERE visibility = 'public' ORDER BY created_at DESC LIMIT/OFFSET`
- `get_recipe_by_id_public()` — `WHERE id = $1 AND visibility IN ('public', 'unlisted')` (no ownership check)
- `get_user_public_recipes()` — `WHERE user_id = $1 AND visibility = 'public' ORDER BY created_at DESC LIMIT/OFFSET`
- `get_user_by_username()` — `WHERE username = $1` (for profile routing)

**New server functions** (3 total):
- `get_public_recipe(recipe_id)` — fetch public/unlisted recipe, no auth required
- `get_public_recipes(offset, limit)` — paginated public recipes for explore page
- `get_user_profile(username)` — user info + public recipe count

**Updated server functions:**
- `create_recipe()` — add `visibility` parameter
- `update_recipe()` — add `visibility` parameter
- `get_recipe()` — handle visibility: private requires ownership, public/unlisted allow any authenticated user

**Verify:**
- `cargo test --features server` — all new query functions tested
- Public recipe visible to non-owner; private recipe returns error
- Unlisted recipe only accessible via direct ID lookup
- `cargo check --target wasm32-unknown-unknown` — zero errors

**Risk:** Low. Straightforward query additions.

---

## Checkpoint 9: Explore page

**Files:** `src/pages/explore.rs`, `src/main.rs` (route already exists)

**`explore.rs` changes:**
- Fetch public recipes via `get_public_recipes` server function
- Render recipe card grid (reuse `RecipeCard` component)
- Tag filter: clickable chips that filter recipes by tag (client-side or server-side)
- Search input: filter by title (client-side for simplicity)
- Pagination: "Load more" button
- Empty state: "No public recipes yet. Be the first to share!"
- No auth required to browse

**Verify:**
- Navigate to `/explore` → public recipes displayed
- Private recipes do not appear
- Tag filtering works
- Search filtering works
- "Load more" paginates correctly

**Risk:** Low. Reuses existing card component and pagination pattern.

---

## Checkpoint 10: User profile page

**Files:** `src/pages/user_profile.rs` (new), `src/main.rs` (new route)

**Route addition:** `#[route("/u/:username")] UserProfile { username: String }`

**`user_profile.rs`:**
- Fetch user via `get_user_profile` server function on mount
- Fetch user's public recipes via `get_user_public_recipes` server function
- Render:
  - User avatar, display name, bio, join date
  - Public recipe grid (paginated)
  - Empty state if user has no public recipes
- 404 if username doesn't exist
- No auth required to view

**Verify:**
- Navigate to `/u/username` → user profile renders
- Only public recipes shown (unlisted/private hidden)
- Invalid username shows 404
- Recipe cards link to detail pages

**Risk:** Low. Standard profile page pattern.

---

## Checkpoint 11: Public recipe detail

**Files:** `src/pages/recipe_detail.rs`, `src/middleware/auth.rs`

**Auth middleware change:**
- Remove `/recipes/:id` from protected paths (allow public access)
- Keep `/recipes/:id/edit` protected
- Keep `/recipes/new` protected

**`recipe_detail.rs` changes:**
- Try `get_recipe()` first (ownership-gated); if fails, try `get_public_recipe()`
- Owner mode: show "Edit" and "Delete" buttons
- Non-owner mode: show "View Owner's Profile" link to `/u/:username`
- Private recipe accessed by non-owner: "Recipe not found" (no existence leakage)
- Unlisted recipe: renders normally when accessed via direct link
- Owner attribution: avatar + username (linked to `/u/:username`) + created date

**Verify:**
- Owner visits own recipe → sees edit/delete buttons
- Non-owner visits public recipe → sees view-only mode with profile link
- Non-owner visits private recipe → sees "not found"
- Unlisted recipe accessible via direct link only
- Owner username links to their profile page

**Risk:** Medium. Conditional rendering based on ownership requires careful state management.

---

## Updated File Structure

```
src/
├── api/
│   ├── mod.rs                    # Re-exports recipe module
│   └── recipe.rs                 # Server functions + serialization types
├── db/
│   └── mod.rs                    # Recipe, RecipeTag types + all query functions
├── components/
│   └── base/
│       ├── recipe_card.rs        # Recipe preview card for grids
│       └── visibility_selector.rs # Visibility radio/dropdown
├── middleware/
│   └── auth.rs                   # Updated: /recipes/:id is public
├── pages/
│   ├── dashboard.rs              # Recipe card grid + list_my_recipes
│   ├── explore.rs                # Public recipe discovery
│   ├── recipe_detail.rs          # Full recipe view (owner + public modes)
│   ├── recipe_edit.rs            # Edit page
│   ├── recipe_new.rs             # Create page
│   └── user_profile.rs           # User's public profile
└── main.rs                       # Routes including /u/:username
```
