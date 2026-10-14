# NOMS-010: Recipe Versioning, Drafts & Branching — Implementation Plan

**Issue:** [NOMS-010-versioning-drafts-branching.md](../issues/NOMS-010-versioning-drafts-branching.md)
**Created:** 2026-06-08
**Revised:** 2026-07-15
**Depends on:** NOMS-008 and NOMS-008b recipe CRUD/schema work
**Approach:** Store immutable full snapshots for published versions and keep in-progress edits in a separate owner-only draft row.

---

## Implementation contract

This plan targets the schema and APIs that currently exist in the repository. NOMS-010 must use the current recipe vocabulary: `user_id`, `visibility`, `prep_time_minutes`, `cook_time_minutes`, `instructions`, `equipment`, and `images`.

### Existing recipe model

The current `recipes` table contains:

- `user_id`, `title`, `description`, `commentary`
- `prep_time_minutes`, `cook_time_minutes`, `servings`
- `ingredients JSONB`, `instructions JSONB`, `equipment JSONB`, `images JSONB`
- `visibility` with values `private`, `unlisted`, or `public`
- `created_at`, `updated_at`

The shared `Recipe` type is defined in `src/types.rs` and re-exported by `src/db/mod.rs`. It includes nested `RecipeStep` values and image metadata, including step photos.

### Canonical recipe snapshot

Both published versions and drafts use the same server-only `RecipeSnapshot` payload:

```rust
struct RecipeSnapshot {
    title: String,
    description: Option<String>,
    commentary: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: Vec<RecipeIngredient>,
    instructions: Vec<RecipeStep>,
    equipment: Vec<RecipeEquipment>,
    images: Vec<RecipeImage>,
    visibility: String,
    tags: Vec<String>,
}
```

Presentation-only fields such as author username, avatar URL, and timestamps are not part of the snapshot. Image binaries are not stored in the snapshot; only their existing recipe image metadata is copied.

### Published version storage

- Every published version stores a complete immutable `snapshot JSONB` value.
- `recipe_versions.version_number` increases monotonically per recipe.
- The latest published version is the row with the greatest `version_number`; no `is_latest` flag is needed.
- Diffs for the history UI are computed between adjacent full snapshots when requested. They are not persisted as part of version history.
- The denormalized `recipes` row remains the current published snapshot for existing list/detail/search queries.

### Historical snapshot access policy

The live recipe's current visibility intentionally governs its complete published history:

- owners may read every version of their own recipe;
- anyone may read every version of a `public` recipe;
- anyone with the URL may read every version of an `unlisted` recipe;
- non-owners cannot list, read, compare, or traverse snapshots of a `private` recipe.

Changing the live recipe's visibility changes access to all of its immutable
snapshots. In particular, changing a private recipe to public or unlisted makes
older versions available under that new visibility, including content removed
from the current version. This is intentional product behavior rather than a
per-version visibility model. Drafts remain owner-only and never inherit the
published recipe's non-owner access policy.

Version lists, individual snapshots, comparisons, compact lineage views, and
the version graph must apply this policy consistently. Inaccessible adjacent
branches may preserve topology through redacted nodes, but must not expose
snapshot content or protected recipe metadata.

### Draft storage

- Drafts live in `recipe_drafts`, not in `recipes`.
- A draft for an existing recipe references `recipe_id` and starts from the current published snapshot.
- A new recipe draft has `recipe_id = NULL` until it is published.
- Autosaves update one mutable draft row and never create a `recipe_versions` row.
- Publishing an existing draft creates one new immutable version, updates the denormalized `recipes` row, updates tags, and deletes the draft in one transaction.
- Publishing a new draft creates the `recipes` row, creates version 1, inserts tags, and deletes the draft in one transaction.
- The last published recipe remains visible while its owner edits a draft.
- Drafts are visible only to their owner. Public queries never join or expose `recipe_drafts`.

### Concurrency policy

Published operations use the draft’s `base_version_number` and lock the parent recipe row during publish. Draft autosaves use a mutable `revision` counter so out-of-order requests cannot overwrite a newer draft save.

---

## Checkpoint 1: Schema and clean development database

**Files:** `migrations/schema.sql`, `src/types.rs`, `src/db/mod.rs`, `src/test_utils.rs`

### 1a. Create `recipe_versions`

Add the table to `migrations/schema.sql`:

```sql
CREATE TABLE IF NOT EXISTS recipe_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL CHECK (version_number > 0),
    snapshot JSONB NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (recipe_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe
    ON recipe_versions(recipe_id, version_number DESC);
```

### 1b. Create `recipe_drafts`

Drafts use the same canonical snapshot shape but remain mutable:

```sql
CREATE TABLE IF NOT EXISTS recipe_drafts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    base_version_number INTEGER CHECK (base_version_number IS NULL OR base_version_number > 0),
    snapshot JSONB NOT NULL,
    revision BIGINT NOT NULL DEFAULT 0,
    forked_from_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,
    forked_from_version_number INTEGER CHECK (
        forked_from_version_number IS NULL OR forked_from_version_number > 0
    ),
    forked_from_title VARCHAR(255),
    fork_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipe_drafts_user
    ON recipe_drafts(user_id, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_recipe_drafts_existing_recipe
    ON recipe_drafts(user_id, recipe_id)
    WHERE recipe_id IS NOT NULL;
```

Multiple unsaved new-recipe drafts are allowed. Only one active draft per existing recipe/user pair is allowed.

### 1c. Create fork lineage tables

Fork lineage is recorded when a fork draft is published:

```sql
CREATE TABLE IF NOT EXISTS fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_recipe_id UUID REFERENCES recipes(id) ON DELETE SET NULL,
    forked_recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    forked_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    forked_version_number INTEGER CHECK (
        forked_version_number IS NULL OR forked_version_number > 0
    ),
    original_title VARCHAR(255) NOT NULL,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id);
CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id);
```

`original_title` preserves attribution if the source recipe is deleted. A fork draft stores equivalent source metadata until it is published.

### 1d. Clean-database prerequisite

This is an early development feature. Reset or recreate the development database before enabling NOMS-010; existing recipe data is intentionally disposable. No backfill migration or compatibility path for legacy recipe-only rows is required.

### 1e. Rust types and errors

Add server-only `RecipeVersion`, `RecipeDraft`, and `ForkRelationship` types. Preserve the existing `DbError::RecipeNotFound(Uuid)` variant; do not add a duplicate variant. Add appropriately shaped variants such as:

```rust
VersionNotFound { recipe_id: Uuid, version_number: i32 },
VersionConflict { recipe_id: Uuid, expected: i32, actual: i32 },
DraftNotFound(Uuid),
DraftConflict { draft_id: Uuid, expected: i64, actual: i64 },
ForkError(String),
```

### 1f. Test schema and helpers

Update `apply_test_schema()` to match `schema.sql`, including both new tables, constraints, and indexes. Add helpers that create a published recipe with v1, create a draft from a recipe, and create a new unsaved draft.

**Verify:**

- A recreated development database has all objects through `just migrate`.
- New recipe publication creates both the recipe row and version 1.
- Draft creation does not create a recipe version.
- `cargo test --features server` and `cargo check --features server` pass.
- `cargo check --target wasm32-unknown-unknown` passes for shared types.

**Risk:** Low. Existing data is explicitly out of scope; the important invariant is that newly published recipes begin with version 1.

---

## Checkpoint 2: Snapshot serialization, optional diff computation, and queries

**Files:** `Cargo.toml`, `Cargo.lock`, `src/db/diff.rs` (new), `src/db/mod.rs`

### 2a. Snapshot module

Implement server-only helpers:

- `recipe_to_snapshot(recipe, tags)`
- `snapshot_to_json(snapshot)` and `json_to_snapshot(json)`
- `snapshot_to_recipe(snapshot, recipe_metadata)`
- `compute_diff(old_snapshot, new_snapshot)` for history display only
- `change_summary(diff)` for timeline labels

The JSON Patch dependency is optional history-display tooling, not the persistence format. No reverse patch or reconstruction chain is needed.

Tests must cover round trips, scalar changes, array additions/removals, reordered ingredients/instructions, nested instruction arrays, image metadata, tags, null fields, empty arrays, and JSON Pointer escaping.

### 2b. Published-version queries

Add:

- `get_recipe_versions(pool, recipe_id)` — summaries ordered newest first
- `get_latest_recipe_version(pool, recipe_id)` — highest version number
- `get_recipe_version(pool, recipe_id, version_number)`
- `get_next_version_number(pool, recipe_id)`
- `insert_recipe_version(...)`

### 2c. Draft queries

Add ownership-gated queries:

- `insert_recipe_draft(...)`
- `get_recipe_draft(pool, draft_id, user_id)`
- `get_recipe_draft_for_recipe(pool, recipe_id, user_id)`
- `update_recipe_draft(pool, draft_id, user_id, expected_revision, snapshot)`
- `delete_recipe_draft(pool, draft_id, user_id)`
- `list_recipe_drafts(pool, user_id)`

Public recipe queries continue to read only from `recipes`; they do not need an `is_draft` filter or a join to drafts.

**Verify:**

- Every published version can be deserialized independently.
- History diffs are computed between any two adjacent snapshots.
- Draft queries cannot read or update another user’s draft.
- Stale draft revisions return `DraftConflict`.

**Risk:** Medium. The persistence model is simple, but the snapshot contract must remain identical for recipes and drafts.

---

## Checkpoint 3: Transactional publish and version creation

**Files:** `src/api/recipe.rs`, `src/db/mod.rs`

Replace the current direct overwrite path with explicit draft/publish operations.

### 3a. Create and save drafts

- `create_recipe_draft(initial_snapshot)` — creates a new owner-only draft.
- `create_edit_draft(recipe_id)` — copies the current published recipe and tags into a draft with `base_version_number`.
- `save_recipe_draft(draft_id, expected_revision, snapshot)` — updates only the mutable draft row.
- `discard_recipe_draft(draft_id)` — deletes an owner’s draft without affecting the published recipe or version history.

Draft saves must not insert into `recipe_versions` or update the published `recipes` row.

### 3b. Publish an existing-recipe draft

Inside one transaction:

1. Lock the parent recipe row with `SELECT ... FOR UPDATE` and verify ownership.
2. Lock and read the owner’s draft.
3. Verify the draft’s `base_version_number` still matches the current published version.
4. Validate the complete snapshot for publication.
5. Insert `version_number + 1` with the complete snapshot and optional notes.
6. Update every denormalized field on `recipes`, including `updated_at`.
7. Replace `recipe_tags` from the snapshot.
8. Delete the draft.
9. Commit and return the published recipe.

### 3c. Publish a new-recipe draft

Inside one transaction:

1. Lock and read the owner’s new draft.
2. Validate the complete snapshot.
3. Insert the `recipes` row.
4. Insert version 1 with the complete snapshot.
5. Insert tags.
6. Create a fork relationship if the draft originated from a fork.
7. Delete the draft and commit.

### 3d. Compatibility with existing API names

The existing `update_recipe` server function should either be changed to publish an existing draft or be replaced by an explicit `publish_recipe_draft` function. Do not retain a code path that overwrites `recipes` without inserting a version.

**Tests:**

- draft autosave changes no published data
- publishing an existing draft creates exactly one new version
- publishing a new draft creates v1
- tags and all recipe fields remain synchronized
- stale base version rejects publication without deleting the draft
- any transaction failure leaves the draft and published recipe unchanged
- concurrent publication leaves one consistent latest version

**Risk:** High. This is the main data-integrity boundary.

---

## Checkpoint 4: Draft UI and autosave

**Files:** `src/api/recipe.rs`, `src/pages/recipe_new.rs`, `src/pages/recipe_edit.rs`, `src/pages/dashboard.rs`, `src/components/base/recipe_card.rs`, `src/main.rs`, `src/middleware/auth.rs`

### 4a. New recipe flow

- New recipe page creates a draft on first meaningful input or on the first autosave.
- The client retains the draft ID while editing.
- Incomplete drafts are allowed; publish validation is stricter.
- Add an owner-protected route such as `/recipes/drafts/:id/edit` for drafts that do not yet have a recipe ID.

### 4b. Existing recipe edit flow

- Opening edit creates or loads the owner’s existing-recipe draft.
- The published recipe remains unchanged while the draft is edited.
- Autosave uses a two-second debounce and sends the expected draft revision.
- Out-of-order or stale responses do not replace the client’s newest state.
- Publish redirects to the published recipe detail page.

### 4c. Dashboard

Show published recipes and owner-only drafts as separate sections or filters. Draft cards link to the draft edit route and show a DRAFT badge. Public recipe cards never include drafts.

**Tests:**

- incomplete drafts persist across reloads
- draft autosave does not create versions
- publish rejects invalid content and accepts valid content
- discard removes only the draft
- another user cannot view, edit, publish, or discard the draft
- public/detail/explore/profile queries never expose drafts

**Risk:** Medium. The main risk is keeping draft IDs and revision numbers synchronized across autosave requests.

---

## Checkpoint 5: Version history UI

**Files:** `src/api/recipe.rs`, `src/pages/recipe_detail.rs`, `src/components/base/version_timeline.rs` (new), `src/components/base/version_diff.rs` (new)

Add:

- `get_recipe_versions_api(recipe_id)` — history access governed by the live recipe's current visibility
- `get_recipe_version_api(recipe_id, version_number)`
- `VersionSummary` with version number, title, notes, created date, and current marker
- `VersionTimeline` with all immutable published versions
- `VersionDiff` that computes changes between adjacent full snapshots
- History tab on recipe detail

Historical content is read directly from the selected full snapshot. No reconstruction chain is needed. Drafts do not appear in version history.

**Verify:**

- v1, v2, and v3 render exactly as published.
- History labels and diffs work for scalar, array, nested-step, image, and tag changes.
- Owners can access all history for their own recipes.
- Non-owners can access every historical snapshot of public and unlisted recipes.
- Non-owners cannot access version lists, snapshots, or comparisons for private recipes.
- Changing a private recipe to public or unlisted exposes its earlier snapshots; changing it back to private removes non-owner access again.
- Drafts never appear in history or become readable through the historical snapshot APIs.

---

## Checkpoint 6: Restore version

**Files:** `src/api/recipe.rs`, `src/db/mod.rs`, history components

Implement `restore_version(recipe_id, version_number, expected_latest_version)`:

1. Verify ownership and expected current version.
2. Load the target full snapshot directly.
3. Insert it as a new version through the same publish transaction.
4. Update the denormalized recipe row and tags.
5. Add notes such as `Restored from v{N}`.

Restoring does not delete or rewrite intermediate versions. It publishes immediately; restoring into a draft can be added later if needed.

**Tests:**

- v1 → v2 → v3, restore v1 → v4 exactly matches v1
- restore is atomic under failure
- historical versions remain unchanged
- stale restore requests return `VersionConflict`

---

## Checkpoint 7: Recipe forking

**Files:** `src/api/recipe.rs`, `src/db/mod.rs`, `src/pages/recipe_detail.rs`, `src/components/base/fork_attribution.rs` (new)

### 7a. Access policy

A user may fork:

- their own published recipe;
- another user’s public or unlisted recipe that they can access;
- never another user’s private recipe.

Only published recipes can be fork sources; drafts are owner-only and have no public fork path.

### 7b. Fork draft flow

1. Verify source access and load the source’s latest full snapshot.
2. Create an owner-only new `recipe_drafts` row with `recipe_id = NULL`.
3. Copy the complete snapshot, including tags and image metadata.
4. Store source recipe ID, source version number, original title, and optional message on the draft.
5. Redirect to the draft edit route.
6. When the user publishes, create the new recipe, version 1, tags, and `fork_relationships` row in one transaction.

Forked recipes are independent. Editing either recipe or abandoning the fork draft does not affect the source.

### 7c. Attribution UI

Show `Forked from {original title}` on the fork draft and published recipe. Link to the source when it still exists; retain text-only attribution after source deletion. The Fork button is available to owners and non-owners who satisfy the access policy.

**Tests:**

- cross-user public fork
- cross-user unlisted fork
- same-user variant fork
- private-recipe fork rejected
- fork draft is owner-only
- tags, images, and all recipe fields copied
- publishing a fork creates the relationship and v1
- deleting a fork removes its relationship
- deleting an original preserves the fork and text attribution

---

## Checkpoint 8: Dedicated recipe version graph

**Files:** `src/main.rs`, `src/types.rs`, `src/api/recipe.rs`, `src/db/versioning.rs`, `src/pages/recipe_detail.rs`, `src/pages/recipe_version_graph.rs` (new), `assets/main.css`

The version graph is a dedicated page, not a replacement for the existing Version history or Fork lineage sections on recipe details. Those existing sections remain the compact, linear views and accessible fallback. The graph presents the connected lineage family as immutable recipe snapshots joined by version and fork edges.

### 8a. Route and page shell

- Add a dedicated route such as `/recipes/:id/version-graph`.
- Add a "View version graph" action on recipe details.
- Implement page heading, back navigation, loading, empty, error, and access-denied states.
- Do not introduce graph traversal or a layout dependency in this slice.

**Verify:**

- The action opens the graph page for the selected recipe.
- Invalid, inaccessible, loading, and empty states render without panics.
- Existing recipe-details Version history and Fork lineage behavior is unchanged.

### 8b. Normalized graph data contract

Define client-facing graph types around immutable snapshot identity:

```rust
struct VersionGraphNodeId {
    recipe_id: Uuid,
    version_number: i32,
}

enum VersionGraphEdgeKind {
    Version,
    Fork,
}
```

- A graph node represents one `(recipe_id, version_number)` snapshot.
- A version edge connects consecutive snapshots of the same recipe.
- A fork edge connects `(source_recipe_id, forked_version_number)` to `(forked_recipe_id, 1)`.
- Include title, owner, 16:9 thumbnail, timestamp, change summary, current-state flags, deletion/redaction state, and truncation metadata as appropriate.
- Keep the API normalized as node and edge collections rather than nested UI-shaped trees.

**Verify:**

- Serialization round-trips preserve stable node identities and typed edges.
- Fork edges terminate at the exact recorded source snapshot and fork v1.
- Redacted nodes expose no private title, owner, image, or snapshot content.

### 8c. Current recipe version chain

- Return only the selected recipe's immutable snapshots initially.
- Render snapshot cards in chronological order with version connectors.
- Highlight the live/current snapshot.
- Use accessible HTML cards with SVG connectors rendered behind them.

**Verify:**

- Every stored version appears once and in the correct order.
- Snapshot thumbnails and image-less placeholders render at 16:9.
- Node links open the correct immutable snapshot page.

### 8d. Immediate fork connections

- Add the selected recipe's direct fork parent and direct fork children.
- Connect every fork edge to its recorded source version rather than the source recipe generally.
- Preserve deleted or redacted topology with synthetic/redacted cards where policy allows.

**Verify:**

- Fork edges originate from the correct version nodes.
- Direct parent and child branches remain connected after a recipe is deleted.
- Private branches do not leak protected metadata.

### 8e. Connected lineage family

- Traverse backward to the lineage root and forward through all forks in the connected component.
- Include siblings, cousins, and deeper branches even when they are not immediate ancestors or descendants of the selected recipe.
- Load recipe versions in batches for all discovered recipes.
- Enforce server-side node, edge, depth, and traversal limits.

**Verify:**

- A recipe's sibling and cousin branches appear when connected through a shared lineage.
- Cycles or malformed relationships cannot cause an infinite traversal.
- Limit and truncation metadata accurately identify omitted graph regions.

### 8f. Layered graph layout and navigation

- Use a directed acyclic graph layout with time/version progression flowing left to right and forks branching vertically.
- Render SVG edge paths separately from accessible HTML snapshot cards.
- Add pan, zoom, fit-to-screen, and center-on-current controls.
- Visually distinguish version edges from fork edges.
- Provide a simplified list/outline fallback when a full canvas is unsuitable or unavailable.

**Verify:**

- Edges terminate at the intended cards without crossing card content.
- Desktop and narrow viewports remain navigable without clipping controls.
- Keyboard users can reach every visible node and graph control.

### 8g. Progressive graph expansion

- Return a bounded initial graph rather than the entire connected component.
- Place expansion controls at every truncated graph frontier.
- Expand a predictable bounded batch from the selected frontier without discarding already loaded nodes.
- Allow branches or long version chains to be collapsed again.

**Verify:**

- Each expansion adds no more than the configured node/edge batch.
- Expanding one frontier does not unexpectedly expand or collapse another.
- Repeated expansion cannot duplicate nodes or edges.

### 8h. Snapshot interactions

- Open a node's immutable snapshot.
- Compare a selected node with the current snapshot.
- Select any two visible snapshots for directional comparison.
- Navigate from a snapshot node to its live recipe when accessible.
- Keep selection, loading, error, and disabled states explicit.

**Verify:**

- Comparison direction matches the user's node-selection order.
- Cross-recipe snapshot comparisons handle schema-compatible snapshots safely.
- Existing restore and comparison actions on recipe details remain unchanged.

**Risk:** High. Connected-component traversal, privacy filtering, graph-size limits, and layout performance must be validated independently at each sub-checkpoint. Do not begin the next slice until the current slice has focused server tests, layout tests, and browser evidence.

---

## Testing strategy

- Unit tests for snapshot serialization, snapshot equality, and computed adjacent-version diffs.
- Integration tests using the existing `pgtemp` test database helpers.
- Schema tests for fresh creation, constraints, indexes, and the one-draft-per-existing-recipe invariant.
- Transaction tests for rollback, stale draft saves, stale publication, concurrent publication, and exactly one consistent latest version.
- Authorization tests for owner, public, unlisted, private, and draft access.
- Visibility-transition tests proving that the current live visibility is applied consistently to version lists, snapshots, comparisons, lineage, and graph endpoints.
- UI/server tests for draft validation, autosave, publish, discard, restore, and fork flows.
- `cargo test --features server`
- `cargo check --features server`
- `cargo check --target wasm32-unknown-unknown`
- `cargo clippy --features server --all-targets -- -D warnings`

The WASM check validates shared client types; it does not replace native server compilation and integration tests.

---

## Development reset and rollback

- `migrations/schema.sql` remains the declarative schema source of truth.
- Existing development data may be discarded and the database recreated before enabling NOMS-010.
- Do not add a backfill migration or compatibility path for legacy recipe rows in this phase.
- Verify that every newly published recipe has version 1 and that every draft is owner-gated.
- Application rollback is safe in development by recreating the database and rerunning `just migrate`.

### Follow-up coordination

NOMS-012’s future search plan should treat published recipes and owner-only drafts as separate sources rather than filtering an `is_draft` column on `recipes`.
