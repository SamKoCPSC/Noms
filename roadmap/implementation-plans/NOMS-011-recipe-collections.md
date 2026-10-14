# NOMS-011: Recipe Collections — Implementation Plan

**Issue:** [NOMS-011-recipe-collections.md](../issues/NOMS-011-recipe-collections.md)
**Created:** 2026-06-10
**Revised:** 2026-08-11
**Depends on:** NOMS-008 published recipe model
**Coordination:** NOMS-010 may be implemented in parallel. NOMS-011 only references published `recipes`; it does not depend on `recipe_versions` or `recipe_drafts`.
**Approach:** Add private user-owned collections with a self-referential folder tree and ordered recipe membership, then integrate filesystem-like collection navigation into the existing recipe UI. After the private collection baseline is complete, add collection-level `private`, `unlisted`, and `public` visibility without allowing collection visibility to broaden a recipe's own visibility.
**Scope extension:** Checkpoints 8-11 formally extend the original NOMS-011 private-only scope with collection sharing. Checkpoint 12 adds one optional collection cover image. These checkpoints supersede the corresponding sharing, visibility, and cover-image exclusions from the original issue for this implementation plan.

---

## Implementation contract

### Current recipe model

The current `recipes` table uses `user_id`, `visibility`, `prep_time_minutes`, `cook_time_minutes`, `instructions`, `equipment`, and `images`. NOMS-011 must not introduce the older `owner_id`, `is_public`, `is_draft`, `steps`, or `total_time_min` recipe fields.

NOMS-010 now stores drafts separately in `recipe_drafts`. Collections reference `recipes` directly, so only published recipe rows can be added to collections. Drafts cannot be added to collections in this phase.

### Collection ownership, hierarchy, and visibility

Checkpoints 1-7 establish the private collection baseline:

- Collections are private and owned by one user.
- Collections form a tree: `parent_collection_id = NULL` means a root collection.
- A collection may contain child collections and recipe files.
- A child collection has exactly one parent; moving a collection changes its parent.
- A collection cannot be moved into itself or any of its descendants.
- Sibling collection names are case-insensitively unique for one owner.
- Only the collection owner can view, create, rename, delete, reorder, or modify membership.
- A user may add a recipe they can currently access: their own recipe, or another user’s public/unlisted recipe.
- A recipe becoming private later makes it unavailable in the collection view; the junction row may remain and become visible again if access is restored.
- Collections do not become public merely because they contain public recipes.
- Server functions derive the current user from the authenticated session. They must not accept a caller-supplied `user_id_str` for authorization.

Checkpoints 8-11 extend that baseline with collection sharing. Mutations remain owner-only, and collection visibility never grants access to a recipe that its own visibility would deny.

The shared-read visibility matrix is:

| Collection visibility | Non-owner recipe visibility |
|---|---|
| `private` | No collection or recipe access |
| `unlisted` | Direct-link access to `public` and `unlisted` recipes |
| `public` | Public access to `public` recipes only |

`unlisted` collections are accessible by UUID-based URL but are excluded from discovery. `public` collections may be listed and searched. The owner view continues to use the existing owner-aware recipe predicate: the owner's recipes plus other users' `public` or `unlisted` recipes.

Each collection has independent visibility. A shared child may exist below an inaccessible parent, but shared breadcrumbs must stop before the first inaccessible ancestor so private or unlisted parent metadata is not leaked. Shared child listings apply the same access context: a public page lists public children only, while an unlisted page may list public and unlisted children.

### Shared and server-only types

Client/server response types belong in `src/types.rs` and derive `Serialize`/`Deserialize`. Database-only row adapters and query functions belong in `src/db/mod.rs` under the server feature. Do not expose a server-only SQLx row type directly in a server-function response.

### Ordering

- Child collections and recipe files have separate order sequences in this phase. Collection detail renders child folders first, followed by recipe files. This gives filesystem-like navigation without introducing a polymorphic `collection_entries` table; a future mixed folder/file order can add that if product requirements need it.
- Collection `sort_order` orders sibling folders; `collection_recipes.sort_order` orders recipe files within a folder.
- Both `sort_order` values are non-negative integers, normally normalized to `0..n-1`.
- Adding a recipe or child collection locks the parent collection row, calculates the next order, and appends atomically. Root-folder appends lock the owner row (or use an equivalent owner-scoped advisory lock), because roots have no parent row to serialize them.
- Reordering sends the complete ordered list of sibling IDs and normalizes that sequence in one transaction.
- Membership changes and reorders update `collections.updated_at` so the dashboard order reflects recent activity.

---

## Checkpoint 1: Schema, shared types, and ownership-safe queries

**Files:** `migrations/schema.sql`, `src/types.rs`, `src/db/mod.rs`, `src/test_utils.rs`

### 1a. Schema

Add the following objects to `migrations/schema.sql`:

```sql
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

CREATE TABLE IF NOT EXISTS collection_recipes (
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0 CHECK (sort_order >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collection_id, recipe_id)
);

CREATE INDEX IF NOT EXISTS idx_collection_recipes_order
    ON collection_recipes(collection_id, sort_order, created_at);

CREATE INDEX IF NOT EXISTS idx_collection_recipes_recipe_id
    ON collection_recipes(recipe_id);
```

The `NULLS NOT DISTINCT` index makes sibling names case-insensitively unique, including among root collections. The same name is allowed in different parent folders.

### 1b. Shared types

Add shared types in `src/types.rs`:

```rust
pub struct Collection {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub parent_collection_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CollectionSummary {
    pub id: Uuid,
    pub parent_collection_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub child_count: i64,
    pub recipe_count: i64,
    pub updated_at: DateTime<Utc>,
}

pub struct CollectionRecipeList {
    pub recipes: Vec<Recipe>,
    pub total_count: i64,
    pub has_more: bool,
}

pub struct RecipeOrder {
    pub recipe_id: Uuid,
    pub sort_order: i32,
}

pub struct CollectionOrder {
    pub collection_id: Uuid,
    pub sort_order: i32,
}

pub struct CollectionContents {
    pub collection: CollectionSummary,
    pub breadcrumbs: Vec<CollectionSummary>,
    pub child_collections: Vec<CollectionSummary>,
    pub recipes: CollectionRecipeList,
}
```

Add `Serialize`, `Deserialize`, `Clone`, `Debug`, and `PartialEq` as appropriate for server-function payloads. `CollectionRecipe` can remain server-only because it is an internal junction-row type.

### 1c. Ownership-safe query functions

Implement query functions that either include the owner ID in the SQL predicate or are used only inside an already owner-locked transaction:

```rust
insert_collection(executor, owner_id, parent_collection_id, name, description)
    get_collection_by_id_and_owner(executor, collection_id, owner_id)
get_root_collections(executor, owner_id)
get_child_collections(executor, parent_collection_id, owner_id)
get_all_collections_by_owner(executor, owner_id)
get_collection_breadcrumbs(executor, collection_id, owner_id)
get_collection_descendants(executor, collection_id, owner_id)
update_collection(executor, collection_id, owner_id, name, description)
move_collection(executor, collection_id, owner_id, new_parent_collection_id)
delete_collection(executor, collection_id, owner_id)

add_recipe_to_collection(executor, collection_id, owner_id, recipe_id)
remove_recipe_from_collection(executor, collection_id, owner_id, recipe_id)
get_collection_recipes(executor, collection_id, owner_id, offset, limit)
get_recipe_collections(executor, recipe_id, owner_id)
reorder_collection_recipes(executor, collection_id, owner_id, ordered_recipe_ids)
reorder_child_collections(executor, owner_id, parent_collection_id, ordered_collection_ids)
get_collection_recipe_count(executor, collection_id, owner_id)
```

`add_recipe_to_collection` must verify both:

1. The current user owns the collection.
2. The current user can access the recipe through the existing visibility rules: own recipe, `public`, or `unlisted`.

Collection recipe queries must apply the same visibility predicate when returning recipes and counts. This prevents a recipe that later becomes private from leaking through a collection.

The add query must lock the collection row before calculating `MAX(sort_order) + 1`. Membership changes must update `collections.updated_at` in the same transaction.

`get_collection_recipes` returns only recipes directly inside the requested folder. Recursive browsing happens by entering child collections; it must not silently flatten the entire subtree.

`get_collection_breadcrumbs` uses a recursive CTE from the current folder to the root and returns an owner-checked, root-to-leaf path. `get_collection_descendants` uses a recursive CTE and is used by `move_collection` to reject the collection itself or any descendant as the new parent.

`move_collection` locks the affected collection and destination parent (when non-root), verifies both belong to the current owner, checks the cycle rule, and updates `parent_collection_id`, `sort_order`, and `updated_at` atomically. The destination sibling order is appended and the old parent’s order is normalized.

Hierarchy mutations acquire locks in a deterministic order (owner row, then parent/collection IDs) to avoid deadlocks when moving or reordering folders concurrently. Root reorders use the same owner-row serialization as root appends.

`reorder_collection_recipes` must verify that the submitted IDs are exactly the collection’s current membership, lock the collection, update all order values, and update `collections.updated_at`. Do not build dynamic SQL from untrusted recipe IDs.

`reorder_child_collections` must verify that the submitted IDs are exactly the direct children of the selected parent (or root), lock the affected rows, normalize `sort_order`, and update the parent’s `updated_at` when there is a parent.

### 1d. Errors and test helpers

Preserve existing `DbError` variants and add only the errors needed by the current implementation, for example:

```rust
CollectionNotFound(Uuid),
CollectionParentNotFound(Uuid),
CollectionNameTaken,
CollectionCycle,
RecipeNotAccessible(Uuid),
RecipeAlreadyInCollection,
InvalidCollectionOrder,
```

Map the sibling-name uniqueness violation to a controlled `CollectionNameTaken` error.

Update `apply_test_schema()` and add helpers for users, collections, published recipes, and collection membership.

**Verify:**

- Fresh `just migrate` creates both tables and indexes.
- Owner guards work for every collection read/write query.
- Public, unlisted, private, and inaccessible recipe cases are covered.
- Concurrent appends do not create ambiguous duplicate order values.
- Reordering normalizes order correctly and updates `updated_at`.
- `cargo test --features server` and `cargo check --features server` pass.

**Risk:** Medium. Ownership and visibility checks are more important than the table definitions.

---

## Checkpoint 2: Server functions and authorization

**Files:** `src/api/collection.rs` (new), `src/api/mod.rs`, `src/types.rs`

All server functions extract the authenticated user from the session. No function accepts a user ID from the client for authorization.

Implement:

```text
create_collection(name, description, parent_collection_id)
get_collection(collection_id) -> CollectionContents
list_root_collections()
list_collection_children(parent_collection_id)
list_my_collections() -> Vec<CollectionSummary>
move_collection(collection_id, new_parent_collection_id)
reorder_child_collections(parent_collection_id, ordered_collection_ids)
update_collection(collection_id, name, description)
delete_collection(collection_id)
add_recipe_to_collection(collection_id, recipe_id)
remove_recipe_from_collection(collection_id, recipe_id)
get_collection_recipes(collection_id, offset, limit)
reorder_collection_recipes(collection_id, ordered_recipe_ids)
get_recipe_collections(recipe_id) -> Vec<Uuid>
```

Each function should parse UUIDs, extract the session user, and map ownership/access failures to safe errors. `get_collection` and all membership reads are owner-only.

Use shared response types from `src/types.rs`. `get_collection` returns `CollectionContents`, including the current folder, root-to-leaf breadcrumbs, direct child folders, and direct recipe files. Root and child listing functions return `CollectionSummary`; `get_collection_recipes` returns `CollectionRecipeList`, using the existing shared `Recipe` type rather than an undefined `RecipeSummary` type. `get_recipe_collections` returns the IDs of the current user’s collections containing the recipe, which is sufficient for checkbox state.

Creating with `parent_collection_id = None` creates a root folder. Creating with a parent creates a child folder after verifying that the parent belongs to the current user. Moving to `None` moves a folder to the root. Server functions trim names before storing them and map self/descendant moves, cross-owner parents, and sibling name collisions to controlled errors. `list_my_collections` returns all owned folder summaries for the membership picker; primary navigation still uses root/child queries.

**Tests:**

- unauthenticated requests fail
- wrong-user requests cannot read or mutate collections
- users can add their own private recipes
- users can add accessible public/unlisted recipes
- users cannot add another user’s private recipe
- drafts are not accepted as collection members
- invalid UUIDs and duplicate membership return controlled errors
- root and nested collection creation returns the expected parent and sibling order
- root appends remain correctly ordered under concurrent creation
- same-name siblings are rejected, while the same name is allowed in separate folders
- moving a collection to root or another folder works; moving into itself or a descendant fails

**Risk:** Medium. The important behavior is that authorization is derived from the session and enforced twice: API boundary and SQL ownership predicate.

---

## Checkpoint 3: Collection CRUD and routes

**Files:** `src/pages/collection_list.rs`, `src/pages/collection_new.rs` (new), `src/pages/collection_detail.rs`, `src/pages/collection_edit.rs` (new), `src/components/base/collection_form.rs` (new), `src/main.rs`, `src/pages/mod.rs`, `src/middleware/auth.rs`

### 3a. Routes

Use UUID strings consistently:

```rust
#[route("/collections")] CollectionList {}
#[route("/collections/new")] CollectionNew {}
#[route("/collections/:id")] CollectionDetail { id: String }
#[route("/collections/:id/edit")] CollectionEdit { id: String }
```

Replace the existing placeholder collection pages. Keep `/collections` as a useful collection browser/list page rather than leaving a placeholder route behind.

Update `src/middleware/auth.rs` so `/collections/:id` and `/collections/:id/edit` use UUID validation. Coordinate this change with NOMS-010’s route updates instead of replacing the other agent’s work.

### 3b. Collection form

- Name required after trimming, maximum 100 characters.
- Description optional, maximum 500 characters.
- Default new name: `Untitled Collection`.
- Create and edit use the same form component.
- Server validation repeats the client validation.

### 3c. Pages

- `collection_list.rs`: owner’s root folders with counts and an empty state.
- `collection_new.rs`: create a root folder or a child folder when opened from a collection, then redirect to detail.
- `collection_detail.rs`: owner-only detail page with breadcrumbs, child-folder navigation, direct recipe files, recipe count, empty state, edit/delete controls, and “New subcollection”.
- `collection_edit.rs`: load, rename, validate, optionally move to another folder/root, and redirect back to detail.

**Verify:**

- UUID collection routes work.
- Create, view, edit, and delete flows work.
- Deleting a collection does not delete recipes.
- Non-owners receive a safe permission/not-found response.
- Existing placeholder pages are fully replaced.
- Breadcrumbs and child-folder links preserve the current hierarchy.
- Move controls cannot offer the current folder or one of its descendants as a destination.

---

## Checkpoint 4: Dashboard collection navigation

**Files:** `src/pages/dashboard.rs`, `src/components/base/collection_sidebar.rs` (new), `src/components/base/mod.rs`

Add `CollectionSidebar`:

- “All Recipes” link to `/dashboard`.
- Owner’s root collections with recipe count badges and expand/collapse controls.
- Lazily load or prefetch child collections as the user expands a folder; render the tree using each collection’s `parent_collection_id`.
- “New Collection” link to `/collections/new`, plus a “New subcollection” action on each expanded folder.
- Sibling folders ordered by `sort_order ASC`, with `updated_at` used only as a secondary display fallback.
- Hidden or collapsed empty state when no collections exist.

The sidebar is a navigation tree, not a recursive recipe listing: selecting a folder opens that folder and shows only its direct child folders and recipe files.

The sidebar must not fetch or display drafts directly; NOMS-010 owns draft presentation. Coordinate the final dashboard composition after the NOMS-010 dashboard changes are merged.

**Verify:**

- Collection counts update after membership changes.
- Sidebar navigation works on desktop and narrow layouts.
- Existing dashboard recipe pagination and draft UI remain intact.

**Risk:** Medium. `dashboard.rs` is a shared integration file with NOMS-010.

---

## Checkpoint 5: Add and remove recipes

**Files:** `src/pages/recipe_detail.rs`, `src/pages/collection_detail.rs`, `src/components/base/add_to_collection_dropdown.rs` (new)

### 5a. AddToCollectionDropdown

- Render only for an authenticated user.
- Load all of the user’s collections and the recipe’s current memberships.
- Show one checkbox per owned collection, labeled with its breadcrumb path so identically named folders in different branches are distinguishable.
- Add/remove through the server functions.
- Use optimistic state only after ownership and access checks are already enforced server-side.
- Refresh counts after successful changes.

### 5b. Recipe detail integration

Add “Add to Collection” to the recipe detail page for authenticated viewers. A user may add any currently accessible published recipe, including another user’s public/unlisted recipe.

### 5c. Collection detail integration

Add a Remove control next to each recipe card. Removal is owner-gated and must update the collection count and `updated_at`.

**Verify:**

- A recipe can belong to multiple collections.
- Duplicate membership is rejected by the primary key.
- Adding appends to the end.
- Removing the last recipe leaves the collection intact.
- Private or inaccessible recipes do not leak through collection results.

**Risk:** Medium. `recipe_detail.rs` is another shared integration file with NOMS-010.

---

## Checkpoint 6: Move and reorder folders and recipe files

**Files:** `src/pages/collection_detail.rs`, `src/components/base/reorder_handle.rs` (new)

Use up/down controls for the first implementation. Drag-and-drop can be added later.

- Recipe-file move controls update local order immediately, debounce the save after 500ms of inactivity, and send the complete ordered recipe ID list.
- Child-folder move controls send the complete ordered child collection ID list for the current parent.
- A move/reparent action sends the target parent ID or `None` for root. The UI excludes the current folder and all descendants from the destination choices.
- The server verifies membership or direct-child membership, locks the affected rows, normalizes sort values, and commits atomically.
- On failure or stale state, reload the collection order.

Do not use dynamic SQL string interpolation for the submitted IDs. Parameterize each update or use a safe transaction loop.

**Tests:**

- first/last boundary controls are disabled
- order persists after reload
- rapid moves produce one final save
- concurrent reorders do not create duplicate or negative order values
- other collections remain unchanged
- moving a folder preserves its descendants and memberships
- moving a folder into itself or a descendant is rejected without changing either tree

---

## Checkpoint 7: Edge cases, integration, and polish

**Files:** collection-related files plus shared integration files as needed

### Ownership and access

- All collection operations are owner-gated.
- Public/unlisted recipe membership is checked at add time and read time.
- Drafts cannot be added because collection membership references `recipes`, not `recipe_drafts`.
- Collection contents are never public in this phase.

### Cascade behavior

- Deleting a collection recursively cascades to child collections through the self-referential foreign key, and then to every descendant’s `collection_recipes` row.
- Deleting a recipe cascades its collection memberships.
- Deleting a user cascades collections and memberships.
- Deleting or hiding a source recipe does not delete unrelated collections.

### Hierarchy behavior

- Root and nested folders use the same collection detail route and expose root-to-leaf breadcrumbs.
- Direct folder listings show child folders and direct recipe files; descendants are not flattened into the parent view.
- Sibling names are case-insensitively unique, but names may repeat in different branches.
- Self moves, descendant moves, cross-owner parents, and invalid reorder lists fail atomically.
- Very deep trees must not overflow the UI; the first implementation should use bounded indentation and allow navigation through breadcrumbs.

### Shared-file integration

After NOMS-010 is integrated, verify the combined changes in:

- `src/main.rs`
- `src/middleware/auth.rs`
- `src/pages/dashboard.rs`
- `src/pages/recipe_detail.rs`
- `src/components/base/mod.rs`
- `src/api/mod.rs`
- `src/db/mod.rs`
- `src/test_utils.rs`
- `migrations/schema.sql`

---

## Checkpoint 8: Collection visibility schema, types, and owner mutations

**Files:** `migrations/schema.sql`, `src/types.rs`, `src/db/mod.rs`, `src/api/collection.rs`, `src/test_utils.rs`

### 8a. Schema and shared types

- Add `collections.visibility VARCHAR(20) NOT NULL DEFAULT 'private'` with a check constraint allowing only `private`, `unlisted`, and `public`.
- Add an index suitable for public collection discovery without weakening the existing owner and hierarchy indexes.
- Add a serializable `CollectionVisibility` enum and use it in collection response and form types. Do not refactor the existing recipe visibility strings as part of this checkpoint.
- Add visibility to `Collection` and `CollectionSummary` so owner pages, breadcrumbs, and edit forms can render the current state.
- Default all newly created collections to `private` unless the create request explicitly supplies another valid value.
- Preserve the development-phase migration assumption: no production backfill workflow is required. Existing development rows resolve to `private` through the schema default and development databases may be recreated.

### 8b. Owner-only visibility mutation

- Extend collection creation and update inputs with collection visibility.
- Add an owner-scoped database query and server function for changing visibility.
- Derive the owner from the authenticated session. Do not accept a caller-supplied owner/user ID.
- Keep rename, move, reorder, delete, and membership mutations owner-only regardless of visibility.
- Update `collections.updated_at` when visibility changes.

**Verify:**

- invalid visibility values are rejected at both the typed API boundary and database constraint
- another user cannot change a shared collection's visibility
- changing visibility does not modify recipe visibility or membership rows
- newly created collections remain private by default

**Risk:** Medium. Visibility is an access-control field and must not be represented as an unchecked client string.

---

## Checkpoint 9: Viewer-safe collection reads and recursive access

**Files:** `src/db/mod.rs`, `src/api/collection.rs`, `src/types.rs`

Add a viewer-aware read path while preserving the existing owner-only mutation paths.

### 9a. Viewer-aware contents

- Add a collection contents query that accepts an optional authenticated viewer derived by the server and returns an owner view when the viewer owns the collection.
- A non-owner or anonymous viewer may read `public` and `unlisted` collections by ID; a private or missing collection returns the same not-found result.
- Shared collection responses include the owner username and an explicit capability such as `can_edit`; do not expose mutation controls based only on client-side ownership guesses.
- Counts and pagination must use the same recipe predicate as returned rows.
- Private recipes are never returned to non-owners.
- Public collection views include only `public` recipes.
- Unlisted collection views include `public` and `unlisted` recipes.
- A recipe becoming less visible remains in `collection_recipes` but disappears from shared rows and counts immediately.

### 9b. Recursive hierarchy access

- Update breadcrumb CTE support so a shared response stops before the first ancestor that is inaccessible in the current visibility context.
- Shared child queries return only children accessible in that same context: public children on public pages, and public/unlisted children on unlisted pages.
- Do not expose inaccessible ancestor names, descriptions, counts, IDs, or existence through errors.
- Retain cycle protection and recursion bounds from the private hierarchy implementation.

### 9c. Server API separation

- Add a viewer-aware read server function using an optional authenticated session.
- Keep create, update, delete, move, reorder, add/remove membership, and visibility updates behind required authentication and owner-scoped queries.
- Use safe not-found responses for inaccessible collections; do not distinguish private from nonexistent resources.
- Continue referencing published `recipes` only; never query or expose `recipe_drafts` through collections.

**Tests:**

- anonymous, authenticated non-owner, and owner access across all three collection visibility values
- the full collection/recipe visibility matrix
- row/count/pagination predicate equivalence
- public child below private or unlisted ancestors without breadcrumb leakage
- hidden children omitted from shared child counts and listings
- recipe visibility changes reflected without deleting membership
- owner mutations rejected for every non-owner viewer

**Risk:** High. This checkpoint establishes the public authorization boundary and requires focused query and API review.

---

## Checkpoint 10: Visibility controls and read-only sharing UI

**Files:** `src/pages/collection_new.rs`, `src/pages/collection_edit.rs`, `src/pages/collection_detail.rs`, collection card/list components, shared form components as needed

### 10a. Owner controls

- Add a visibility selector to collection creation and editing with plain-language descriptions:
  - Private: only you can access it.
  - Unlisted: anyone with the link can access shareable contents.
  - Public: anyone can access it and it may appear in discovery.
- Show visibility badges on owner collection cards and detail pages.
- Before changing to `public` or `unlisted`, show how many memberships will be hidden from visitors by recipe visibility rules.
- Add Copy link and Share actions for public and unlisted collections. Use the Web Share API when available and provide a clipboard fallback.

### 10b. Shared collection detail

- Reuse the collection detail layout for shared reads, but render it as read-only when `can_edit` is false.
- Hide edit, delete, reorder, remove, add-membership, move, and subcollection-creation controls from non-owners.
- Keep safe read operations such as pagination available.
- Ensure recipe links use the existing public/unlisted recipe-detail access path.
- Distinguish a truly empty collection from a collection whose memberships are not visible to the viewer without revealing hidden recipe counts.
- Preserve loading, error, narrow-screen, keyboard, and screen-reader behavior.

**Verify:**

- copied unlisted and public links open in a logged-out session
- private links return the standard unavailable state
- no owner-only control appears or executes for a non-owner
- public pages never render unlisted/private recipes or inaccessible hierarchy metadata
- changing visibility updates badges and sharing controls after reload

**Risk:** Medium. The detail page must use server-provided capabilities and filtered data rather than recreating authorization rules in RSX.

---

## Checkpoint 11: Discovery, integration, and sharing security review

**Files:** collection pages and APIs, public profile/search integration files as available, collection-related tests

### Discovery

- Public collections may appear on public profiles and in NOMS-012 collection search.
- Unlisted collections must never appear in search, feeds, public profile listings, autocomplete, or public collection indexes.
- Public discovery queries return only public-safe counts and metadata.
- Discovery integration may be delivered after the direct-link sharing path if NOMS-012 is not yet integrated.

### Security and privacy review

- Confirm that collection visibility can restrict recipe exposure but can never broaden recipe visibility.
- Confirm private and nonexistent collections are indistinguishable to unauthorized viewers.
- Confirm shared breadcrumbs, child listings, counts, pagination, empty states, and errors do not leak inaccessible memberships or ancestors.
- Confirm every mutation still derives authorization from the authenticated session and remains owner-only.
- Confirm UUID parsing, pagination limits, recursive CTE cycle protection, and deletion cascades remain intact.
- Confirm visibility changes and recipe visibility changes take effect on the next read without cache leakage.

### Integration checks

- Test direct shared URLs in authenticated-owner, authenticated-non-owner, and anonymous browser sessions.
- Test public collection discovery separately from unlisted direct-link access.
- Run the full native server, WASM, formatting, and lint checks.
- Review the final diff specifically for owner predicates accidentally replaced by visibility predicates in mutation queries.

**Risk:** High. Treat any metadata, count, breadcrumb, or membership disclosure as an authorization defect.

---

## Checkpoint 12: Optional collection cover image

**Files:** `migrations/schema.sql`, collection types/queries/APIs, shared image upload component, collection create/edit/detail/list/profile UI, orphan cleanup query, collection tests

- Store one optional `RecipeImage` object in nullable `collections.image` JSONB; no development-data backfill is required.
- Pass the cover through the canonical owner-authorized create and update functions. Authorization remains session-derived and mutations remain owner-only.
- Include the image in owner and viewer summaries, recursive breadcrumb reads, child reads, and public discovery without changing collection or recipe visibility rules.
- Reuse the existing deferred image upload pipeline. Collection forms cap the gallery at one while recipe image galleries remain unlimited.
- Allow owners to add, replace, or remove the cover on edit. Do not save collection fields until any pending image processing/upload completes.
- Render the cover on collection detail, owner collection cards, child collection cards, and public-profile discovery cards. When no explicit cover is set, render a bounded mosaic of the first four recipe images visible to that viewer; use the stable folder fallback only when no usable recipe image exists.
- Include both full and thumbnail cover URLs in orphan-cleanup reference discovery.

**Verify:** nullable image round-trips through every collection read path; create/edit defer correctly; the one-image limit is enforced; remove/replace works; recipe-image mosaics preserve collection order, stop at four, and never reveal inaccessible recipe images; image-less collections retain a coherent folder fallback; private/unlisted/public access behavior is unchanged; recipe galleries still accept multiple images.

**Risk:** Medium. The central risks are dropping image metadata in a custom recursive query and orphan cleanup deleting a live cover object.

---

## Testing strategy

- Unit tests for collection validation and order normalization.
- Integration tests with the existing `pgtemp` test database helpers.
- Query tests for every ownership and visibility path.
- Access-matrix tests for private, unlisted, and public collections combined with private, unlisted, and public recipes.
- Shared hierarchy tests proving inaccessible ancestors and children do not leak through breadcrumbs, counts, or errors.
- Hierarchy tests for root/nested creation, breadcrumbs, same-owner parent enforcement, sibling-name uniqueness, cycle-safe moves, recursive deletion, and independent folder/file ordering.
- Transaction tests for concurrent appends, reorder, and rollback.
- Server-function tests for session-derived authorization and UUID parsing.
- UI tests/manual checks for collection CRUD, dashboard navigation, membership, and reorder.
- Logged-out and authenticated-non-owner UI checks for public/unlisted collection links and read-only controls.
- `cargo test --features server`
- `cargo check --features server`
- `cargo check --target wasm32-unknown-unknown`
- `cargo clippy --features server --all-targets -- -D warnings`

The WASM check validates shared client types; it does not replace native server tests.

---

## Migration and rollback

- `migrations/schema.sql` remains the declarative source of truth.
- The collection schema is additive and requires no data backfill.
- The visibility extension adds a non-null `collections.visibility` field with a `private` default; no separate development-data backfill is required.
- Apply through the existing `just migrate` flow.
- Existing development data may be discarded if needed while NOMS-010 and NOMS-011 are being integrated.
- Development rollback can recreate the database and rerun `just migrate`; do not add destructive rollback SQL to the plan.

---

## Dependencies and parallelization

| Checkpoint | Depends On | Parallelization |
|---|---|---|
| 1 Schema/types/queries | NOMS-008 published recipe model | Can proceed alongside NOMS-010 in a separate worktree; shared schema/db files need merge review |
| 2 Server functions | Checkpoint 1 | Can proceed before NOMS-010 finishes |
| 3 CRUD/routes | Checkpoint 2 | Mostly independent; auth/main files may conflict with NOMS-010 |
| 4 Sidebar | Checkpoint 2 | Defer final dashboard integration until NOMS-010 dashboard work is merged |
| 5 Membership | Checkpoint 2 + recipe detail | Defer final recipe-detail integration until NOMS-010 recipe-detail work is merged |
| 6 Reorder | Checkpoint 2 + collection detail | Independent of NOMS-010 |
| 7 Integration/polish | All previous + NOMS-010 integration | Must run after both feature branches are combined |
| 8 Visibility schema/types/mutation | Checkpoint 7 | Can begin independently of discovery work; shared schema and collection API files require merge review |
| 9 Viewer-safe reads | Checkpoint 8 | Must complete before any shared collection UI is exposed |
| 10 Sharing UI | Checkpoint 9 | Direct-link sharing can proceed without NOMS-012 discovery integration |
| 11 Discovery/security review | Checkpoints 9-10; NOMS-012 for search | Discovery may be deferred, but the authorization and privacy review is required before release |
| 12 Optional cover image | Checkpoints 7 and 9 | Reuses recipe image processing; collection reads and orphan cleanup require full-path review |
