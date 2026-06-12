# NOMS-010: Recipe Collections

**Status:** ⚪ Backlog  
**Phase:** Phase 2 (organization)  
**Depends on:** NOMS-008 (Recipe CRUD), NOMS-009 (Versioning & Drafts)

## Overview

Allow users to organize their recipes into named collections (folders). Collections are user-owned containers that hold references to recipes. A recipe can belong to multiple collections. Collections support reordering, renaming, and deletion.

This is a pure data organization feature — no storage backend, images, or external dependencies required.

## Context

After NOMS-008 and NOMS-009, users can create, edit, and version recipes. As their recipe count grows, the flat dashboard list becomes harder to navigate. Collections provide a way to group recipes by meal type, cuisine, season, or any user-defined category.

## Acceptance Criteria

### AC1: Database schema for collections

- [ ] Migration creates `collections` table: `id`, `owner_id`, `name`, `description`, `created_at`, `updated_at`
- [ ] Migration creates `collection_recipes` junction table: `collection_id`, `recipe_id`, `sort_order`, `created_at`
- [ ] Foreign keys: `collections.owner_id REFERENCES users(id) ON DELETE CASCADE`
- [ ] Foreign keys: `collection_recipes.collection_id REFERENCES collections(id) ON DELETE CASCADE`
- [ ] Foreign keys: `collection_recipes.recipe_id REFERENCES recipes(id) ON DELETE CASCADE`
- [ ] Unique constraint: `collection_recipes(collection_id, recipe_id)` — recipe can't be added twice to same collection
- [ ] Index: `collections(owner_id)` for listing user's collections
- [ ] Index: `collection_recipes(recipe_id)` for finding which collections contain a recipe
- [ ] Schema is additive-only (IF NOT EXISTS) and idempotent

### AC2: Create a collection

- [ ] "New Collection" button on dashboard creates empty collection
- [ ] Collection form collects: name (required, max 100 chars), description (optional, max 500 chars)
- [ ] Saving creates new `collections` row
- [ ] Default collection name is "Untitled Collection" — editable inline
- [ ] User is redirected to the new collection's detail page on success

### AC3: View collection (detail page)

- [ ] `/collections/:id` displays: collection name, description, recipe count
- [ ] Recipes displayed as a card grid (reuse `RecipeCard` from dashboard)
- [ ] Empty state: "This collection is empty" with "Add recipes" button
- [ ] Loading state while fetching collection data
- [ ] Error state if collection not found or user lacks permission
- [ ] "Edit" and "Delete" buttons in header (only visible to collection owner)

### AC4: List user's collections (dashboard sidebar)

- [ ] Dashboard sidebar shows list of user's collections
- [ ] Each item shows: name, recipe count badge
- [ ] Clicking navigates to collection detail page
- [ ] "All Recipes" link at top returns to unfiltered dashboard view
- [ ] "New Collection" button at bottom of list
- [ ] Collections sorted by `updated_at DESC` (most recently modified first)
- [ ] Empty state: no sidebar section shown if user has no collections

### AC5: Add/remove recipes from collection

- [ ] "Add to Collection" dropdown on recipe detail page shows user's collections with checkboxes
- [ ] Recipe can be added to multiple collections simultaneously
- [ ] Recipe removed from collection via "Remove" button in collection detail page
- [ ] Adding recipe increments collection recipe count
- [ ] Removing last recipe doesn't delete collection (collection remains empty)
- [ ] Recipe's `sort_order` in `collection_recipes` set to current max + 1 on add (appends to end)

### AC6: Edit and reorder collection

- [ ] "Edit" button on collection detail page opens rename/description form
- [ ] Saving updates collection name, description, and `updated_at`
- [ ] Recipes within collection can be reordered via drag-and-drop or up/down buttons
- [ ] Reorder updates `sort_order` in `collection_recipes` for affected recipes
- [ ] Reordered recipes displayed in `sort_order ASC` within collection

### AC7: Delete collection

- [ ] "Delete" button on collection detail page triggers confirmation dialog
- [ ] Confirmation: "Delete this collection? Recipes in the collection will not be deleted."
- [ ] Deleting removes the `collections` row; `collection_recipes` cascade via foreign key
- [ ] Recipes referenced by the collection remain untouched
- [ ] User is redirected to dashboard after deletion

### AC8: Collections DB queries and types

- [ ] Rust types in `src/db/mod.rs`: `Collection`, `CollectionRecipe`
- [ ] Query functions:
  - `insert_collection()` — create new collection
  - `get_collection_by_id()` — fetch collection by ID
  - `get_collection_by_id_and_owner()` — ownership-gated lookup
  - `get_collections_by_owner()` — list user's collections
  - `update_collection()` — update name/description, refresh `updated_at`
  - `delete_collection()` — delete collection (recipes cascade)
  - `add_recipe_to_collection()` — insert junction row with sort_order
  - `remove_recipe_from_collection()` — delete junction row
  - `get_collection_recipes()` — fetch recipes in collection with sort_order
  - `get_recipe_collections()` — fetch collections containing a recipe
  - `reorder_collection_recipes()` — bulk update sort_order for recipes in collection
  - `get_collection_recipe_count()` — count recipes in collection
- [ ] All queries guard by `owner_id` where appropriate
- [ ] Tests for each query function (using existing `test_utils` pattern)

## Technical Details

### Database Schema (new tables)

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

### Server functions

| Function | Purpose |
|----------|---------|
| `create_collection(owner_id, name, description)` | Insert collection row |
| `get_collection(collection_id, user_id)` | Fetch collection + recipe count (ownership-gated) |
| `list_my_collections(user_id)` | List user's collections with recipe counts |
| `update_collection(collection_id, user_id, name, description)` | Update name/description |
| `delete_collection(collection_id, user_id)` | Delete collection (ownership-gated) |
| `add_recipe_to_collection(collection_id, user_id, recipe_id)` | Add recipe to collection |
| `remove_recipe_from_collection(collection_id, user_id, recipe_id)` | Remove recipe from collection |
| `get_collection_recipes(collection_id, user_id, offset, limit)` | Paginated recipes in collection |
| `reorder_collection_recipes(collection_id, user_id, recipe_order[])` | Bulk reorder recipes |
| `get_recipe_collections(recipe_id, user_id)` | Collections containing a recipe |

### AuthContext changes

No changes. Collection pages use existing `current_user_id` from AuthContext for ownership checks.

### Route protection changes

- `/collections/new` added to `PROTECTED_PATHS`
- `/collections/:id` added to `PROTECTED_PATHS`
- `/collections/:id/edit` added to `PROTECTED_PATHS`

### Component changes

| Component | Change |
|-----------|--------|
| New: `CollectionSidebar` | Dashboard sidebar: "All Recipes" link, collection list with counts, "New Collection" button |
| New: `CollectionDetail` | Collection page: name, description, recipe grid, edit/delete, reorder |
| New: `CollectionForm` | Create/edit collection form (name + description) |
| New: `AddToCollectionDropdown` | Dropdown on recipe detail page: checkboxes for user's collections |
| `Dashboard` | Add sidebar slot for `CollectionSidebar` |
| `RecipeDetail` | Add "Add to Collection" button in header area |
| `RecipeCard` | No changes (reused in collection grid) |

### Reordering strategy

`sort_order` is an integer. On add, new recipe gets `MAX(sort_order) + 1`. On reorder, only affected recipes get new values. No floating-point sort keys — integer renumbering on drag is simpler and avoids precision issues.

For drag-and-drop: calculate target position, shift affected recipes up/down by 1, insert at target. O(n) update for affected rows, executed in a single transaction.

### Ownership model

- Collections are always owned by a single user
- Only the owner can edit, delete, or add/remove recipes
- A recipe can be in collections owned by different users (each user manages their own collections independently)
- If user A adds user B's public recipe to their collection, user A manages that reference; user B's recipe is unaffected

## Out of Scope

- Shared/public collections (Phase 3)
- Collection-level visibility (private/public toggle)
- Collection cover images (depends on NOMS-011 image uploads)
- Nested collections / sub-collections
- Collection import/export
- Auto-collections (e.g., "Recently Added", "Favorites")
- Recipe search within collection (covered by NOMS-012 global search)

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | DB schema + queries | Migration applied, all 12 query functions working, tests pass |
| 2 | Server functions | All 10 server functions compile and work end-to-end |
| 3 | Collection CRUD | Create, view, edit, delete collections working, redirects correct |
| 4 | Dashboard sidebar | Collections listed with counts, navigation works |
| 5 | Add/remove recipes | Dropdown on recipe detail, remove from collection detail, junction table correct |
| 6 | Reorder recipes | Drag-and-drop or up/down buttons, sort_order updated, display reflects order |
| 7 | Edge cases + polish | Empty states, error states, ownership guards, cascade deletes |

## Success Metrics

- User creates collection → adds recipes → sees them in collection grid
- User reorders recipes in collection → order persists on reload
- User adds same recipe to multiple collections → no duplicates within single collection
- User deletes collection → recipes remain intact, junction rows cascade
- User adds other user's public recipe to their own collection → works correctly
- All 7 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- No unhandled error paths in server functions
