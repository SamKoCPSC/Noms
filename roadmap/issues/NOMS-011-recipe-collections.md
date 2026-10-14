# NOMS-011: Recipe Collections

**Status:** ⚪ Backlog  
**Phase:** Phase 2 (organization)  
**Revised:** 2026-07-15
**Depends on:** NOMS-008 (Recipe CRUD); NOMS-010 dashboard/recipe-detail integration may land in parallel

## Overview

Allow users to organize their recipes into named collections (folders). Collections are user-owned filesystem-like containers that can contain child folders and references to recipe files. A recipe can belong to multiple collections. Collections support navigation, moving, reordering, renaming, and deletion.

This is a pure data organization feature — no storage backend, images, or external dependencies required.

## Context

After NOMS-008, users can create and access published recipes. NOMS-010 may add separate owner-only drafts in parallel, but collections reference published `recipes` rows only. As the recipe count grows, the flat dashboard list becomes harder to navigate. Collections provide a filesystem-like hierarchy for meal type, cuisine, season, or any user-defined category.

## Acceptance Criteria

### AC1: Database schema for collections

- [ ] Migration creates `collections` table: `id`, `owner_id`, `name`, `description`, `created_at`, `updated_at`
- [ ] `collections.parent_collection_id` is nullable for root folders and references another collection owned by the same user
- [ ] `collections.sort_order` orders sibling folders and has a non-negative check
- [ ] Self-referential deletion cascades to descendant folders; a collection cannot reference itself
- [ ] Sibling collection names are case-insensitively unique, including root siblings; the same name is allowed in different folders
- [ ] Migration creates `collection_recipes` junction table: `collection_id`, `recipe_id`, `sort_order`, `created_at`
- [ ] Foreign keys: `collections.owner_id REFERENCES users(id) ON DELETE CASCADE`
- [ ] Foreign keys: `collection_recipes.collection_id REFERENCES collections(id) ON DELETE CASCADE`
- [ ] Foreign keys: `collection_recipes.recipe_id REFERENCES recipes(id) ON DELETE CASCADE`
- [ ] Unique constraint: `collection_recipes(collection_id, recipe_id)` — recipe can't be added twice to same collection
- [ ] Index: `collections(owner_id)` for listing user's collections
- [ ] Index: `collection_recipes(recipe_id)` for finding which collections contain a recipe
- [ ] Indexes support owner/parent/sort traversal and breadcrumb/descendant queries
- [ ] Schema is additive-only (IF NOT EXISTS) and idempotent

### AC2: Create a collection

- [ ] "New Collection" button on dashboard creates an empty root collection
- [ ] "New subcollection" from a collection creates an empty child collection under that folder
- [ ] Collection form collects: name (required, max 100 chars), description (optional, max 500 chars)
- [ ] Saving creates new `collections` row
- [ ] Default collection name is "Untitled Collection" — editable inline
- [ ] User is redirected to the new collection's detail page on success
- [ ] Creating a child verifies that the selected parent belongs to the current user

### AC3: View collection (detail page)

- [ ] `/collections/:id` displays: collection name, description, recipe count, breadcrumbs, direct child folders, and direct recipe files
- [ ] Direct recipes are displayed as a card grid (reuse `RecipeCard` from dashboard); descendants are reached by entering child folders
- [ ] Empty state: "This collection is empty" with "Add recipes" button
- [ ] Loading state while fetching collection data
- [ ] Error state if collection not found or user lacks permission
- [ ] "Edit" and "Delete" buttons in header (only visible to collection owner)
- [ ] Collection edit supports moving the folder to another valid parent or to root

### AC4: List user's collections (dashboard sidebar)

- [ ] Dashboard sidebar shows a collapsible tree of the user's root and nested collections
- [ ] Each item shows: name, recipe count badge
- [ ] Clicking navigates to collection detail page
- [ ] "All Recipes" link at top returns to unfiltered dashboard view
- [ ] "New Collection" button at bottom of list
- [ ] Sibling folders are sorted by `sort_order ASC` (with a stable name fallback)
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
- [ ] User can move a collection to another owned folder or to the root
- [ ] Moving into the collection itself or any descendant is rejected
- [ ] Child folders within a parent can be reordered independently of recipe files
- [ ] Recipes within collection can be reordered via drag-and-drop or up/down buttons
- [ ] Reorder updates `sort_order` in `collection_recipes` for affected recipes
- [ ] Reordered recipes displayed in `sort_order ASC` within collection

### AC7: Delete collection

- [ ] "Delete" button on collection detail page triggers confirmation dialog
- [ ] Confirmation explains that child folders and memberships will be deleted recursively, while recipes themselves will not be deleted
- [ ] Deleting removes the `collections` row; `collection_recipes` cascade via foreign key
- [ ] Deleting a parent recursively removes descendant collection rows and their membership rows
- [ ] Recipes referenced by the collection remain untouched
- [ ] User is redirected to dashboard after deletion

### AC8: Collections DB queries and types

- [ ] Shared response types in `src/types.rs`: `Collection`, `CollectionSummary`, `CollectionContents`, `CollectionRecipeList`, and ordering payloads
- [ ] Database-only row types in `src/db/mod.rs`: `CollectionRecipe` and query adapters
- [ ] Query functions:
  - `insert_collection()` — create new collection
  - `get_collection_by_id_and_owner()` — ownership-gated lookup
  - `get_root_collections()` — list the user's root folders
  - `get_child_collections()` — list direct child folders for an owned parent
  - `get_all_collections_by_owner()` — return all owned folder summaries for the membership picker
  - `get_collection_breadcrumbs()` — return the root-to-leaf path using a recursive CTE
  - `get_collection_descendants()` — return descendants for cycle-safe moves
  - `update_collection()` — update name/description, refresh `updated_at`
  - `move_collection()` — move a folder to another owned parent or root
  - `delete_collection()` — delete collection (recipes cascade)
  - `add_recipe_to_collection()` — insert junction row with sort_order
  - `remove_recipe_from_collection()` — delete junction row
  - `get_collection_recipes()` — fetch recipes in collection with sort_order
  - `get_recipe_collections()` — fetch collections containing a recipe
  - `reorder_collection_recipes()` — bulk update sort_order for recipes in collection
  - `reorder_child_collections()` — bulk update sort_order for direct child folders
  - `get_collection_recipe_count()` — count recipes in collection
- [ ] All queries guard by `owner_id` where appropriate
- [ ] Parent lookups and moves enforce same-owner hierarchy membership
- [ ] Recursive queries reject cycles and never return another user's folders
- [ ] Tests for each query function (using existing `test_utils` pattern)

## Technical Details

### Database Schema (new tables)

```sql
-- User-owned collections (folders for recipes)
CREATE TABLE IF NOT EXISTS collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_collection_id UUID,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (id, owner_id),
    CHECK (parent_collection_id IS NULL OR parent_collection_id <> id),
    FOREIGN KEY (parent_collection_id, owner_id)
        REFERENCES collections(id, owner_id) ON DELETE CASCADE,
    CHECK (btrim(name) <> '')
);

CREATE INDEX IF NOT EXISTS idx_collections_owner_updated
    ON collections(owner_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_collections_parent_order
    ON collections(owner_id, parent_collection_id, sort_order, lower(name));
CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_sibling_name
    ON collections(owner_id, parent_collection_id, lower(name))
    NULLS NOT DISTINCT;

-- Junction table: recipes in collections (many-to-many with ordering)
CREATE TABLE IF NOT EXISTS collection_recipes (
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collection_id, recipe_id)
);

CREATE INDEX IF NOT EXISTS idx_collection_recipes_recipe_id ON collection_recipes(recipe_id);
```

### Server functions

| Function | Purpose |
|----------|---------|
| `create_collection(name, description, parent_collection_id)` | Insert a root or child folder; derive owner from the session |
| `get_collection(collection_id)` | Fetch folder contents, breadcrumbs, child folders, and direct recipe files |
| `list_root_collections()` | List the current user's root folders |
| `list_collection_children(parent_collection_id)` | List direct child folders for an owned parent |
| `list_my_collections()` | Return all owned folders for the membership picker; clients use parent IDs to display paths |
| `move_collection(collection_id, new_parent_collection_id)` | Move a folder to another owned folder or root, rejecting cycles |
| `reorder_child_collections(parent_collection_id, ordered_collection_ids[])` | Reorder direct child folders |
| `update_collection(collection_id, name, description)` | Update name/description |
| `delete_collection(collection_id)` | Delete a folder and recursively cascade descendants/memberships |
| `add_recipe_to_collection(collection_id, recipe_id)` | Add an accessible published recipe to a folder |
| `remove_recipe_from_collection(collection_id, recipe_id)` | Remove recipe from folder |
| `get_collection_recipes(collection_id, offset, limit)` | Paginated direct recipe files in a folder |
| `reorder_collection_recipes(collection_id, recipe_order[])` | Bulk reorder direct recipe files |
| `get_recipe_collections(recipe_id)` | Collections containing a recipe for the current user |

### AuthContext changes

No changes. Server functions use the existing authenticated session/current-user mechanism for ownership checks; clients never provide a user ID for authorization.

### Route protection changes

- `/collections/new` added to `PROTECTED_PATHS`
- `/collections/:id` added to `PROTECTED_PATHS`
- `/collections/:id/edit` added to `PROTECTED_PATHS`

### Component changes

| Component | Change |
|-----------|--------|
| New: `CollectionSidebar` | Dashboard sidebar: "All Recipes" link, collapsible root/nested collection tree with counts, "New Collection" button |
| New: `CollectionDetail` | Collection page: breadcrumbs, child-folder list, direct recipe grid, edit/delete, move, reorder |
| New: `CollectionForm` | Create/edit collection form (name + description + optional parent) |
| New: `AddToCollectionDropdown` | Dropdown on recipe detail page: checkboxes for user's collections labeled with breadcrumb paths |
| `Dashboard` | Add sidebar slot for `CollectionSidebar` |
| `RecipeDetail` | Add "Add to Collection" button in header area |
| `RecipeCard` | No changes (reused in collection grid) |

### Reordering strategy

`sort_order` is a non-negative integer. On add, a new recipe or child folder gets `MAX(sort_order) + 1` within its respective parent sequence. On reorder, the complete direct-child or direct-recipe list is verified and normalized to `0..n-1`. No floating-point sort keys — integer renumbering is simpler and avoids precision issues.

Folder order and recipe-file order are independent in this phase. Collection detail renders child folders first and recipe files second. A future requirement for arbitrary mixed folder/file interleaving would introduce a unified `collection_entries` table, but it is not needed for recursive navigation.

Moving a folder appends it to the destination sibling sequence and normalizes the old and new parent sequences in one transaction. A recursive CTE prevents moving a folder below itself or any descendant.

For drag-and-drop: calculate target position, shift affected folders or recipe files up/down by 1, insert at target. O(n) update for affected rows, executed in a single transaction.

### Ownership model

- Collections are always owned by a single user
- A collection's parent, when present, must be owned by the same user
- Only the owner can edit, delete, or add/remove recipes
- A recipe can be in collections owned by different users (each user manages their own collections independently)
- If user A adds user B's public recipe to their collection, user A manages that reference; user B's recipe is unaffected

## Out of Scope

- Shared/public collections (Phase 3)
- Collection-level visibility (private/public toggle)
- Collection cover images (depends on NOMS-009 image uploads)
- Collection import/export
- Auto-collections (e.g., "Recently Added", "Favorites")
- Recipe search within collection (covered by NOMS-012 global search)

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | DB schema + queries | Migration applied, hierarchy/membership query functions working, tests pass |
| 2 | Server functions | Collection CRUD, tree navigation, move, membership, and reorder functions compile and work end-to-end |
| 3 | Collection CRUD | Create root/child folders, view breadcrumbs/contents, edit/move, delete recursively, redirects correct |
| 4 | Dashboard sidebar | Root and nested collections listed with counts, expand/collapse navigation works |
| 5 | Add/remove recipes | Dropdown on recipe detail, remove from collection detail, junction table correct |
| 6 | Move and reorder | Folder moves, child-folder ordering, recipe-file ordering, cycle protection, persistence |
| 7 | Edge cases + polish | Empty states, error states, ownership guards, recursive cascade deletes |

## Success Metrics

- User creates collection → adds recipes → sees them in collection grid
- User reorders recipes in collection → order persists on reload
- User adds same recipe to multiple collections → no duplicates within single collection
- User deletes collection → recipes remain intact, junction rows cascade
- User nests folders, navigates breadcrumbs, and moves a folder without creating a cycle
- User deletes a parent folder → all descendant folders and memberships disappear, recipes remain intact
- User adds other user's public recipe to their own collection → works correctly
- All 7 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- No unhandled error paths in server functions
