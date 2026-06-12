# NOMS-009: Recipe Versioning, Drafts & Branching — Implementation Plan

**Issue:** [NOMS-009-versioning-drafts-branching.md](../issues/NOMS-009-versioning-drafts-branching.md)
**Created:** 2026-06-08
**Depends on:** NOMS-008 (Recipe CRUD — must be implemented first)
**Approach:** Bottom-up by dependency, 7 incremental checkpoints matching the issue spec

---

## Pre-requisites

Before starting NOMS-009, NOMS-008 must be complete:
- `recipes` table with: id, owner_id, title, description, is_public, prep/cook/total_time_min, servings, ingredients JSONB, steps JSONB, created_at, updated_at
- `recipe_tags` table exists
- Rust types: Recipe, RecipeTag in src/db/mod.rs
- Query functions: insert_recipe, get_recipe_by_id_and_owner, get_recipes_by_owner, update_recipe, delete_recipe
- Server functions: create_recipe, get_recipe, update_recipe, delete_recipe, list_my_recipes
- Pages: recipe_new.rs, recipe_detail.rs, recipe_edit.rs working

---

## Checkpoint 1: Schema migration + backfill

**Files:** migrations/schema.sql, migrations/002_versioning.sql (new), src/test_utils.rs

### 1a. Add `is_draft` column to `recipes`

Since schema.sql is additive-only CREATE TABLE, ALTER goes into separate migration:

migrations/002_versioning.sql:
```sql
ALTER TABLE recipes ADD COLUMN IF NOT EXISTS is_draft BOOLEAN NOT NULL DEFAULT FALSE;
```

If NOMS-008 not yet deployed, add `is_draft BOOLEAN NOT NULL DEFAULT FALSE` to recipes CREATE TABLE in schema.sql instead.

### 1b. Create `recipe_versions` table

Append to schema.sql:
```sql
CREATE TABLE IF NOT EXISTS recipe_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,
    title VARCHAR(200),
    description TEXT,
    prep_time_min INTEGER,
    cook_time_min INTEGER,
    total_time_min INTEGER,
    servings INTEGER,
    ingredients JSONB,
    steps JSONB,
    reverse_diff JSONB,
    notes TEXT,
    is_latest BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(recipe_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe ON recipe_versions(recipe_id, version_number DESC);
CREATE INDEX IF NOT EXISTS idx_recipe_versions_latest ON recipe_versions(recipe_id, is_latest) WHERE is_latest = TRUE;
```

### 1c. Create `fork_relationships` table

Append to schema.sql:
```sql
CREATE TABLE IF NOT EXISTS fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_recipe_id UUID NOT NULL REFERENCES recipes(id),
    forked_recipe_id UUID NOT NULL REFERENCES recipes(id),
    forked_by UUID NOT NULL REFERENCES users(id),
    forked_version_number INTEGER,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id);
CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id);
```

### 1d. Backfill existing recipes as v1

In 002_versioning.sql:
```sql
INSERT INTO recipe_versions (recipe_id, version_number, title, description, prep_time_min, cook_time_min, total_time_min, servings, ingredients, steps, is_latest, created_at)
SELECT id, 1, title, description, prep_time_min, cook_time_min, total_time_min, servings, ingredients, steps, TRUE, updated_at
FROM recipes
WHERE id NOT IN (SELECT recipe_id FROM recipe_versions);
```

### 1e. Rust types (append to src/db/mod.rs)

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecipeVersion {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub version_number: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub total_time_min: Option<i32>,
    pub servings: Option<i32>,
    pub ingredients: Option<serde_json::Value>,
    pub steps: Option<serde_json::Value>,
    pub reverse_diff: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub is_latest: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ForkRelationship {
    pub id: Uuid,
    pub original_recipe_id: Uuid,
    pub forked_recipe_id: Uuid,
    pub forked_by: Uuid,
    pub forked_version_number: Option<i32>,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

### 1f. Error type additions (append to DbError)

```rust
RecipeNotFound,
VersionNotFound,
ForkError,
DiffError(String),
```

### 1g. test_utils.rs changes

- Add recipe_versions table creation to apply_test_schema()
- Add fork_relationships table creation to apply_test_schema()
- Add is_draft column to recipes CREATE TABLE in test schema
- Add helper: insert_test_recipe(pool, owner_id, title) — creates recipe + v1 version

**Verify:**
- cargo test --features server — migration SQL applies cleanly
- Backfill query runs without error
- cargo check --target wasm32-unknown-unknown — types compile

**Risk:** Low. Straightforward SQL. Backfill is one-time.

---

## Checkpoint 2: Reverse diff library + query functions

**Files:** Cargo.toml, src/db/diff.rs (new), src/db/mod.rs

### 2a. Dependency (Cargo.toml)

```toml
json-patch = "4"
```

json-patch v4.2.0: pure Rust, no platform-specific deps, compiles for wasm32 and native.

### 2b. Diff utility module (src/db/diff.rs, new)

Functions:
- `recipe_to_json()` — serialize recipe fields to single JSON object
- `compute_diff(old, new)` — JSON Patch diff from old to new
- `reverse_patch(patch, old_doc)` — reverse patch so applying to new produces old
- `reconstruct_from_chain(latest_snapshot, reverse_diffs[])` — apply diff chain
- `json_to_recipe(json)` — deserialize back to recipe fields

Key design: reverse_patch takes old_doc to extract pre-replace values for replace/remove ops.

### 2c. Query functions (append to src/db/mod.rs)

- `get_recipe_versions(pool, recipe_id)` — all versions, newest first
- `get_latest_version(pool, recipe_id)` — latest snapshot only
- `get_recipe_version(pool, recipe_id, version_number)` — specific version
- `get_max_version_number(pool, recipe_id)` — max version number
- `insert_recipe_version(pool, recipe_id, version_number, snapshot?, reverse_diff?, notes?)` — new version
- `set_latest_version(pool, recipe_id, version_number)` — mark version as latest
- `get_fork_relationship(pool, forked_recipe_id)` — fork attribution
- `insert_fork_relationship(pool, original, forked, by, version?, message?)` — record fork

### 2d. Tests

- test_recipe_to_json_roundtrip — serialize/deserialize matches
- test_diff_and_reverse — forward diff + reverse + reconstruct = original
- test_reconstruct_chain_three_versions — chain of 3 versions reconstructs correctly

**Verify:**
- cargo test --features server diff_tests — all pass
- cargo check --target wasm32-unknown-unknown — compiles

**Risk:** Medium. Reverse patch for replace needs careful testing. Edge case: missing old value at path.

---

## Checkpoint 3: Versioned recipe edit flow

**Files:** src/api/recipe.rs, src/db/mod.rs

### 3a. update_recipe_versioned()

Steps:
1. Verify ownership via get_recipe_by_id_and_owner
2. Get current latest version
3. Serialize old and new snapshots via recipe_to_json
4. Compute forward diff, then reverse it
5. Mark current latest as historical (set is_latest=FALSE, store reverse_diff)
6. Insert new version as latest full snapshot (version_number+1)
7. Update recipe metadata (title, timestamps)
8. Return updated recipe

### 3b. Tests

- test_update_recipe_versioned: edit creates v2, reconstruction matches v1
- test_version_chain: three edits produce three versions, all reconstruct correctly

**Verify:**
- Edit creates v2 with correct reverse_diff
- Reconstruction of v1 from v2 + reverse_diff matches original
- Title and timestamps update on parent recipe row

**Risk:** Medium. Transaction safety critical — if step 6 fails, roll back step 5.

---

## Checkpoint 4: Version history UI

**Files:** src/api/recipe.rs, src/pages/recipe_detail.rs, src/components/base/version_timeline.rs (new), src/components/base/version_diff.rs (new)

### 4a. get_recipe_versions_api()

Returns Vec<VersionSummary> with: version_number, title, notes, is_latest, created_at.

### 4b. reconstruct_version()

Reconstructs any historical version by collecting reverse diffs from latest down to target+1, applying chain via reconstruct_from_chain.

### 4c. VersionTimeline component

Shows version list with: version number, date, title, notes. Clicking expands to show diff. Has "Restore" button per version.

### 4d. recipe_detail.rs integration

Add "History" tab alongside Ingredients and Steps. Show fork attribution bar if recipe was forked.

**Verify:**
- History tab loads versions correctly
- Diff display shows changes between consecutive versions

**Risk:** Low. Read-only UI feature.

---

## Checkpoint 5: Restore version

**Files:** src/api/recipe.rs

### 5a. restore_version()

1. Verify ownership
2. Reconstruct target version via reconstruct_version()
3. Create new version from restored data (same flow as update_recipe_versioned)
4. Auto-notes: "Restored from v{N}"

### 5b. Tests

- test_restore_version: create v1, v2, v3, restore v1 — produces v4 matching v1

**Verify:**
- Restoring creates new version (does not delete intermediate versions)
- Restored data matches target version exactly

**Risk:** Low. Reuses existing diff + version creation logic.

---

## Checkpoint 6: Draft saving

**Files:** src/api/recipe.rs, src/pages/recipe_new.rs, src/pages/recipe_edit.rs, src/pages/dashboard.rs

### 6a. Server functions

- save_draft(pool, user_id, recipe_id?, title, ...) — creates or updates draft, sets is_draft=TRUE
- publish_recipe(pool, user_id, recipe_id) — sets is_draft=FALSE

### 6b. Auto-save

Debounce 2-second timer using gloo-timers::callback::Timeout. First-write-wins on rapid edits.

### 6c. Dashboard draft filter

"Show drafts" checkbox toggle. DRAFT badge on draft recipe cards.

### 6d. Tests

- test_save_and_publish_draft: draft created with is_draft=TRUE, publish clears it

**Verify:**
- Draft saves persist and show on dashboard with filter
- Publish clears draft flag
- Auto-save debounce works

**Risk:** Low. Isolated feature.

---

## Checkpoint 7: Recipe forking (cross-user + same-user variants)

**Files:** src/api/recipe.rs, src/pages/recipe_detail.rs, src/components/base/fork_attribution.rs (new)

### 7a. fork_recipe()

Forking works for both cross-user sharing and same-user variant creation ("branching").

1. Verify source recipe is accessible (public, or owned by forker)
2. Get source latest version snapshot
3. Create new recipe (as draft) with copied data, owner = forker (may be same user as source owner)
4. Create v1 version for new recipe (full snapshot copy)
5. Insert fork_relationship row
6. Return new recipe

### 7b. ForkAttribution component

Shows "Forked from [Original Recipe]" bar on forked recipe detail page. If same-user fork, shows "Variant of [Original Recipe]".

### 7c. Tests

- test_fork_recipe_cross_user: User B forks User A's public recipe, fork_relationship recorded
- test_fork_recipe_self: User forks their own recipe to create a variant
- test_fork_private_other_user: forking private recipe of another user fails

**Verify:**
- Forked recipe is independent (edits don't affect source)
- Attribution displays correctly
- Same-user fork works (no ownership restriction)

**Risk:** Low. Read source, write new recipe.

---

## File Structure

```
src/
  db/
    mod.rs          — RecipeVersion, ForkRelationship types, 11 query functions, DbError additions
    diff.rs (new)   — recipe_to_json, compute_diff, reverse_patch, reconstruct_from_chain, json_to_recipe
  api/
    recipe.rs       — update_recipe_versioned, save_draft, publish_recipe, reconstruct_version, restore_version, fork_recipe
  pages/
    recipe_new.rs   — draft creation, auto-save, publish button
    recipe_edit.rs  — auto-save draft, publish button, version notes field
    recipe_detail.rs — History tab, Fork button, draft indicator, fork attribution
    dashboard.rs    — draft filter toggle, draft badges
  components/base/
    version_timeline.rs (new) — version history timeline with expand/restore
    version_diff.rs (new)     — diff display between versions
    fork_attribution.rs (new) — "Forked from" attribution bar
migrations/
  schema.sql        — recipe_versions, fork_relationships tables, is_draft column
  002_versioning.sql (new) — ALTER + backfill for deployed instances
```

---

## Dependencies Table

| Checkpoint | Depends On | Can Parallelize With |
|---|---|---|
| 1 (Schema) | NOMS-008 complete | — |
| 2 (Diff lib) | 1 (types defined) | — |
| 3 (Edit flow) | 2 (diff + queries) | — |
| 4 (History UI) | 2 (queries) + 3 (reconstruct) | 5, 6, 7 |
| 5 (Restore) | 3 (edit flow) | 4, 6, 7 |
| 6 (Drafts) | 1 (is_draft column) | 3, 4, 5, 7 |
| 7 (Forking) | 2 (queries) + 3 (version creation) | 4, 5, 6 |

---

## Testing Strategy

- Unit tests: diff.rs (serialize, diff, reverse, reconstruct roundtrips)
- Integration tests: each server function tested with sqlx::test macro
- Chain tests: create v1, v2, v3, verify all reconstruct from v3
- Fork tests: cross-user fork, same-user fork (variant), private recipe fork rejection
- Edge cases: empty ingredients/steps, null fields, large recipe diffs
- WASM check: cargo check --target wasm32-unknown-unknown after each checkpoint

---

## Rollback Plan

- Per-checkpoint git branches: cp1-schema, cp2-diff, cp3-edit, cp4-history, cp5-restore, cp6-drafts, cp7-fork
- Schema changes are additive (IF NOT EXISTS) — safe to apply, no destructive changes
- If diff library proves problematic, can fall back to full snapshot storage (no reverse_diff)
- Each checkpoint verified independently before proceeding to next
