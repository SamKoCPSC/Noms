# NOMS-010: Recipe Collections — Implementation Plan

**Issue:** [NOMS-010-recipe-collections.md](../issues/NOMS-010-recipe-collections.md)
**Created:** 2026-06-10
**Depends on:** NOMS-008 (Recipe CRUD), NOMS-009 (Versioning & Drafts)
**Approach:** Bottom-up by dependency, 7 incremental checkpoints matching the issue spec

---

## Pre-requisites

Before starting NOMS-010, NOMS-008 and NOMS-009 must be complete:
- `recipes` table with: id, owner_id, title, description, is_public, is_draft, prep/cook/total_time_min, servings, ingredients JSONB, steps JSONB, created_at, updated_at
- `recipe_tags` table exists
- `recipe_versions` and `fork_relationships` tables exist
- Rust types: Recipe, RecipeTag, RecipeVersion, ForkRelationship in src/db/mod.rs
- Query functions: insert_recipe, get_recipe_by_id_and_owner, get_recipes_by_owner, update_recipe, delete_recipe
- Server functions: create_recipe, get_recipe, update_recipe, delete_recipe, list_my_recipes
- Pages: recipe_new.rs, recipe_detail.rs, recipe_edit.rs, dashboard.rs working
- Components: RecipeCard, RecipeForm working

---

## Checkpoint 1: Database schema + Rust types + queries

**Files:** `migrations/schema.sql`, `src/db/mod.rs`, `src/test_utils.rs`

### 1a. Schema additions (append to schema.sql, additive-only)

```sql
-- User-owned collections (folders for recipes)
CREATE TABLE IF NOT EXISTS collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_collections_owner_id ON collections(owner_id);
CREATE INDEX IF NOT EXISTS idx_collections_updated_at ON collections(updated_at DESC);

-- Junction table: recipes in collections (many-to-many with ordering)
CREATE TABLE IF NOT EXISTS collection_recipes (
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collection_id, recipe_id)
);

CREATE INDEX IF NOT EXISTS idx_collection_recipes_recipe_id ON collection_recipes(recipe_id);
```

### 1b. Rust types (append to src/db/mod.rs)

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Collection {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CollectionRecipe {
    pub collection_id: Uuid,
    pub recipe_id: Uuid,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CollectionWithCount {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub recipe_count: i64,
}
```

### 1c. Query functions (12 total, append to src/db/mod.rs)

```rust
// Collection CRUD
pub async fn insert_collection(pool: &PgPool, owner_id: Uuid, name: String, description: Option<String>) -> Result<Collection, DbError>
pub async fn get_collection_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Collection>, DbError>
pub async fn get_collection_by_id_and_owner(pool: &PgPool, id: Uuid, owner_id: Uuid) -> Result<Option<Collection>, DbError>
pub async fn get_collections_by_owner(pool: &PgPool, owner_id: Uuid) -> Result<Vec<CollectionWithCount>, DbError>
pub async fn update_collection(pool: &PgPool, id: Uuid, owner_id: Uuid, name: String, description: Option<String>) -> Result<Collection, DbError>
pub async fn delete_collection(pool: &PgPool, id: Uuid, owner_id: Uuid) -> Result<bool, DbError>

// Collection recipes (junction)
pub async fn add_recipe_to_collection(pool: &PgPool, collection_id: Uuid, recipe_id: Uuid) -> Result<bool, DbError>
pub async fn remove_recipe_from_collection(pool: &PgPool, collection_id: Uuid, recipe_id: Uuid) -> Result<bool, DbError>
pub async fn get_collection_recipes(pool: &PgPool, collection_id: Uuid, offset: i64, limit: i64) -> Result<Vec<Recipe>, DbError>
pub async fn get_recipe_collections(pool: &PgPool, recipe_id: Uuid, user_id: Uuid) -> Result<Vec<Collection>, DbError>
pub async fn reorder_collection_recipes(pool: &PgPool, collection_id: Uuid, recipe_orders: Vec<(Uuid, i32)>) -> Result<(), DbError>
pub async fn get_collection_recipe_count(pool: &PgPool, collection_id: Uuid) -> Result<i64, DbError>
```

### 1d. Error type additions (append to DbError)

```rust
CollectionNotFound,
CollectionAlreadyExists,  // For duplicate name prevention if needed
RecipeAlreadyInCollection,  // For unique constraint violation
```

### 1e. test_utils.rs changes

- Add collections table creation to apply_test_schema()
- Add collection_recipes table creation to apply_test_schema()
- Add helper: `insert_test_collection(pool, owner_id, name)` — creates collection
- Add helper: `insert_test_collection_recipe(pool, collection_id, recipe_id, sort_order)` — adds recipe to collection

**Verify:**
- `cargo test --features server` — all 12 query functions tested against local Postgres
- `cargo check --target wasm32-unknown-unknown` — zero errors (types are `#[cfg(feature = "server")]`)
- Migration applies cleanly on fresh DB via `just migrate`

**Risk:** Low. Straightforward SQL + SQLx pattern matching existing code.

---

## Checkpoint 2: Server functions (API layer)

**File:** `src/api/collection.rs` (new module)

**Server functions** (10 total, all `#[server]`):

| Function | Signature | Purpose |
|----------|-----------|---------|
| `create_collection` | `(user_id_str, name, description)` | Insert collection row, return Collection |
| `get_collection` | `(collection_id_str, user_id_str)` | Fetch collection + recipe count (ownership-gated) |
| `list_my_collections` | `(user_id_str)` | List user's collections with recipe counts |
| `update_collection` | `(collection_id_str, user_id_str, name, description)` | Update name/description, refresh updated_at |
| `delete_collection` | `(collection_id_str, user_id_str)` | Delete collection (ownership-gated) |
| `add_recipe_to_collection` | `(collection_id_str, user_id_str, recipe_id_str)` | Add recipe to collection with sort_order |
| `remove_recipe_from_collection` | `(collection_id_str, user_id_str, recipe_id_str)` | Remove recipe from collection |
| `get_collection_recipes` | `(collection_id_str, user_id_str, offset, limit)` | Paginated recipes in collection |
| `reorder_collection_recipes` | `(collection_id_str, user_id_str, recipe_orders)` | Bulk reorder recipes in collection |
| `get_recipe_collections` | `(recipe_id_str, user_id_str)` | Collections containing a recipe |

**Serialization types** (shared between client and server):

- `CollectionSummary { id: Uuid, name: String, recipe_count: i64 }` — sidebar item
- `CollectionDetail { collection: Collection, recipe_count: i64, recipes: Vec<RecipeSummary> }` — detail response
- `RecipeOrder { recipe_id: Uuid, sort_order: i32 }` — reorder payload

**Module registration:** Add `collection` module to `src/api/mod.rs`.

**Verify:**
- `cargo check --features server` — all server functions compile
- Manual test: call `create_collection` server function from test component, verify DB rows
- Ownership gating: calling `get_collection` with wrong `user_id` returns error

**Risk:** Low. Follows same pattern as recipe server functions.

---

## Checkpoint 3: Collection CRUD (create, view, edit, delete)

**Files:** `src/pages/collection_new.rs` (new), `src/pages/collection_detail.rs` (new), `src/pages/collection_edit.rs` (new), `src/components/base/collection_form.rs` (new), `src/main.rs` (new routes)

### 3a. Routes

```rust
#[route("/collections/new")] CollectionNew
#[route("/collections/:id")] CollectionDetail { id: String }
#[route("/collections/:id/edit")] CollectionEdit { id: String }
```

### 3b. Auth middleware

Add to protected paths:
- `/collections/new`
- `/collections/:id` (UUID pattern)
- `/collections/:id/edit` (UUID pattern)

Update `is_uuid_route` to handle `/collections/` paths alongside `/recipes/`.

### 3c. CollectionForm component

- Props: `initial_data: Option<CollectionFormData>`, `on_submit: Callback<CollectionFormData>`, `is_editing: bool`
- State: name (required, max 100 chars), description (optional, max 500 chars)
- Validation: name required, length constraints
- Submit: collect fields into `CollectionFormData`, call `on_submit` callback
- Cancel button: navigates to dashboard or collection detail (depending on context)

### 3d. collection_new.rs

- Render `CollectionForm { initial_data: None, on_submit: create_handler, is_editing: false }`
- `create_handler`: calls `create_collection` server function
- On success: redirect to `/collections/{id}`
- On error: show error message
- Loading state during save

### 3e. collection_detail.rs

- Fetch collection via `get_collection` server function on mount
- Fetch recipes via `get_collection_recipes` server function on mount
- Loading state: show `LoadingSpinner` while fetching
- Error state: "Collection not found" or "You don't have permission"
- Render:
  - Collection name, description, recipe count
  - "Edit" and "Delete" buttons in header (only visible to owner)
  - Recipe card grid (reuse `RecipeCard`)
  - Empty state: "This collection is empty" with "Add recipes" button

### 3f. collection_edit.rs

- Fetch collection on mount via `get_collection` server function
- Pre-populate `CollectionForm` with existing data
- `edit_handler`: calls `update_collection` server function
- On success: redirect to detail page
- On error: show error message

### 3g. Delete flow

- "Delete" button triggers confirmation dialog
- Confirmation: "Delete this collection? Recipes in the collection will not be deleted."
- On confirm: call `delete_collection` server function
- On success: redirect to `/dashboard`
- On error: show toast message

**Verify:**
- Create collection with name and description → appears in DB
- Navigate to collection detail → shows name, description, empty state
- Edit collection → name/description updated, updated_at refreshed
- Delete collection → removed from DB, redirect to dashboard
- Non-owner sees permission error on collection detail
- `cargo clippy --target wasm32-unknown-unknown` — zero warnings

**Risk:** Low. Standard CRUD pattern matching recipe pages.

---

## Checkpoint 4: Dashboard sidebar

**Files:** `src/pages/dashboard.rs`, `src/components/base/collection_sidebar.rs` (new)

### 4a. CollectionSidebar component

- Fetch collections via `list_my_collections` server function on mount
- Render:
  - "All Recipes" link at top (navigates to `/dashboard`)
  - Collection list: each item shows name + recipe count badge
  - "New Collection" button at bottom (navigates to `/collections/new`)
- Clicking collection navigates to `/collections/{id}`
- Empty state: no sidebar section shown if user has no collections
- Collections sorted by `updated_at DESC`

### 4b. dashboard.rs changes

- Add sidebar slot for `CollectionSidebar`
- Layout: sidebar on left, recipe grid on right
- Responsive: sidebar collapses to hamburger menu on narrow screens
- "All Recipes" is the default view (existing behavior)

**Verify:**
- Dashboard shows collections in sidebar with recipe counts
- Clicking collection navigates to collection detail page
- "All Recipes" link returns to unfiltered dashboard
- "New Collection" button navigates to create page
- Empty state: no sidebar shown when no collections exist
- Collections sorted by most recently modified first

**Risk:** Low. Read-only list with navigation.

---

## Checkpoint 5: Add/remove recipes from collection

**Files:** `src/pages/recipe_detail.rs`, `src/pages/collection_detail.rs`, `src/components/base/add_to_collection_dropdown.rs` (new)

### 5a. AddToCollectionDropdown component

- Props: `recipe_id: Uuid`
- Fetch user's collections via `get_recipe_collections` on mount (to know which are already checked)
- Fetch all user's collections via `list_my_collections` on mount
- Render: dropdown with checkboxes for each collection
- Checked state: collection already contains this recipe
- Toggle: add or remove recipe from collection
- Optimistic UI: update checkbox immediately, show error on failure

### 5b. recipe_detail.rs changes

- Add "Add to Collection" button in header area
- Button opens `AddToCollectionDropdown`
- Dropdown shows all user's collections with checkboxes

### 5c. collection_detail.rs changes

- Add "Remove" button next to each recipe card in collection grid
- Remove calls `remove_recipe_from_collection` server function
- Optimistic removal: remove card from UI immediately, re-fetch on error

### 5d. Sort order on add

- New recipe gets `MAX(sort_order) + 1` in `collection_recipes`
- If collection is empty, first recipe gets `sort_order = 0`

**Verify:**
- "Add to Collection" dropdown shows all user's collections
- Checkbox reflects whether recipe is already in collection
- Adding recipe to collection → appears in collection grid
- Removing recipe from collection → disappears from grid
- Recipe can be in multiple collections simultaneously
- Adding same recipe twice to same collection prevented (unique constraint)
- Recipe count badge updates correctly

**Risk:** Medium. Optimistic UI with rollback on error needs careful state management.

---

## Checkpoint 6: Reorder recipes in collection

**Files:** `src/pages/collection_detail.rs`, `src/components/base/reorder_handle.rs` (new)

### 6a. ReorderHandle component

- Props: `index: usize`, `total: usize`, `on_move_up: Callback`, `on_move_down: Callback`
- Render: up/down arrow buttons
- Disabled at boundaries (first item can't move up, last can't move down)
- Alternative: drag handle for drag-and-drop (future enhancement)

### 6b. collection_detail.rs changes

- Each recipe card in collection grid shows `ReorderHandle`
- Up/down buttons update local state immediately (optimistic)
- Debounced save: call `reorder_collection_recipes` after 500ms of no changes
- Reorder logic:
  - Moving item at index `i` to index `j`: swap sort_orders of affected items
  - Only affected rows updated (not full renumbering)
  - Single transaction for atomicity

### 6c. Reorder query

```sql
-- Bulk update sort_order for affected recipes
UPDATE collection_recipes
SET sort_order = CASE recipe_id
    WHEN $1 THEN $2  -- recipe_id -> new_sort_order
    WHEN $3 THEN $4
    -- ... more pairs
END
WHERE collection_id = $N AND recipe_id IN ($1, $3, ...);
```

### 6d. Display order

- Recipes in collection displayed in `sort_order ASC`
- Query: `ORDER BY cr.sort_order ASC, cr.created_at ASC` (tie-break by creation time)

**Verify:**
- Up/down buttons reorder recipes in collection grid
- Order persists on page reload
- Moving first item up is disabled
- Moving last item down is disabled
- Reorder only affects recipes in the collection
- Other collections are unaffected
- Debounced save doesn't fire during rapid reordering

**Risk:** Medium. Optimistic reordering with debounced save needs careful synchronization.

---

## Checkpoint 7: Edge cases + polish

**Files:** All collection-related files

### 7a. Ownership guards

- Verify all server functions check ownership before write operations
- Read operations: user can only view their own collections
- Write operations: user can only modify their own collections
- Recipe access: user can add any recipe they can view to their collections (own recipes + public recipes from others)

### 7b. Cascade behavior

- Deleting collection cascades to `collection_recipes` (FK ON DELETE CASCADE)
- Deleting recipe cascades from `collection_recipes` (FK ON DELETE CASCADE)
- Deleting user cascades to collections and collection_recipes

### 7c. Empty states

- Collection with no recipes: "This collection is empty" with "Add recipes" button
- User with no collections: sidebar section hidden
- Dashboard with no recipes in collection: empty grid with helpful message

### 7d. Error states

- Collection not found: "Collection not found" with link to dashboard
- Permission denied: "You don't have permission to view this collection"
- Network error: retry button, graceful degradation
- Duplicate recipe in collection: error message, no duplicate added

### 7e. URL state

- Collection detail URL: `/collections/{id}` — shareable
- Deep link to collection works for owner, shows error for non-owner

### 7f. Tests

- test_create_collection: collection created with correct owner
- test_get_collection_by_owner: returns collection, wrong owner returns None
- test_add_recipe_to_collection: recipe added, sort_order set correctly
- test_remove_recipe_from_collection: recipe removed, junction row deleted
- test_reorder_recipes: sort_orders updated correctly
- test_delete_collection_cascade: collection deleted, junction rows cascade, recipes intact
- test_duplicate_recipe_prevented: unique constraint prevents duplicate
- test_collection_with_count: recipe count accurate
- test_add_recipe_multiple_collections: recipe in multiple collections works
- test_add_other_users_public_recipe: can add public recipe from other user

**Verify:**
- All ownership guards work correctly
- Cascade deletes work as expected
- Empty states display correctly
- Error states display correctly
- All tests pass
- `cargo clippy` clean on both targets

**Risk:** Low. Verification and polish pass.

---

## Dependencies Summary

| Crate | Feature | Purpose |
|-------|---------|---------|
| `uuid` | both | Collection ID in URL, server function params |
| `chrono` | both | Timestamp serialization in responses |
| `sqlx` | `server` | Already present, no changes needed |
| `dioxus` | both | Already present, `#[server]` macro for API functions |
| `serde` | both | Already present, serialization for server functions |

No new crate dependencies required.

---

## File Structure

```
src/
├── api/
│   ├── mod.rs                    # + collection module re-export
│   └── collection.rs (new)       # Server functions + serialization types
├── db/
│   └── mod.rs                    # + Collection, CollectionRecipe, CollectionWithCount types + 12 query functions
├── components/
│   └── base/
│       ├── add_to_collection_dropdown.rs (new)  # Dropdown with checkboxes for collections
│       ├── collection_form.rs (new)             # Create/edit collection form
│       ├── collection_sidebar.rs (new)          # Dashboard sidebar with collection list
│       └── reorder_handle.rs (new)              # Up/down buttons for reordering
├── middleware/
│   └── auth.rs                   # Updated: UUID route matching for /collections/:id
├── pages/
│   ├── collection_detail.rs (new)  # Collection detail page with recipe grid
│   ├── collection_edit.rs (new)    # Edit collection page
│   ├── collection_new.rs (new)     # Create collection page
│   ├── dashboard.rs                # + CollectionSidebar integration
│   └── recipe_detail.rs            # + AddToCollectionDropdown button
└── main.rs                       # + CollectionNew, CollectionDetail, CollectionEdit routes
migrations/
└── schema.sql                    # + collections, collection_recipes tables
```

---

## Migration Notes

- Schema is additive-only: `CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`
- No `ALTER TABLE` or `DROP` statements
- Safe to apply on existing database (auth, recipe, and versioning tables unaffected)
- Applied via existing `just migrate` command (pgschema)

---

## Testing Strategy

Each checkpoint has self-contained verification:
- **Checkpoint 1:** Unit tests for all 12 query functions against test DB
- **Checkpoint 2:** Compile check + manual server function test
- **Checkpoint 3:** Manual E2E testing: create → view → edit → delete collection
- **Checkpoint 4:** Sidebar renders collections with counts, navigation works
- **Checkpoint 5:** Add/remove recipes from collections, junction table correct
- **Checkpoint 6:** Reorder recipes, sort_order persists, display reflects order
- **Checkpoint 7:** Edge cases, ownership guards, cascade deletes, empty/error states
- **All checkpoints:** `cargo clippy` clean on both `wasm32` and `x86_64` targets

---

## Rollback Plan

If any checkpoint reveals issues:
- Schema tables can be dropped manually: `DROP TABLE collection_recipes, collections CASCADE`
- Code changes are isolated to new files + minimal modifications to existing files
- Each checkpoint is independently reversible
- Dashboard sidebar can be hidden by reverting dashboard.rs changes
- Recipe detail page changes can be reverted without affecting collection pages

---

## Dependencies Table

| Checkpoint | Depends On | Can Parallelize With |
|---|---|---|
| 1 (Schema) | NOMS-008 + NOMS-009 complete | — |
| 2 (Server functions) | 1 (types defined) | — |
| 3 (CRUD pages) | 2 (server functions) | — |
| 4 (Sidebar) | 2 (list_my_collections) | 5, 6 |
| 5 (Add/remove) | 2 (add/remove functions) | 4, 6 |
| 6 (Reorder) | 2 (reorder function) + 3 (detail page) | 4, 5 |
| 7 (Edge cases) | All previous | — |
