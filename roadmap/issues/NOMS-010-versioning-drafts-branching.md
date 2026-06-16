# NOMS-010: Recipe Versioning, Drafts & Branching

**Status:** ⚪ Backlog  
**Phase:** Phase 1 (core content creation)  
**Depends on:** NOMS-008 (Recipe CRUD)

## Overview

Add reverse-diff versioning, draft saving, and recipe forking on top of the basic CRUD from NOMS-008. Every edit creates an immutable version. Users can save drafts, browse history, restore old versions, and fork recipes — either someone else's recipe or their own — into an independent copy that evolves separately.

## Context

After NOMS-008, recipes are single rows that get overwritten on edit. NOMS-010 adds:

| Feature | What it solves |
|---------|----------------|
| **Reverse-diff versioning** | Immutable history, storage-efficient, O(1) read of latest version |
| **Draft saving** | Auto-save work in progress, explicit publish flow |
| **Recipe forking** | Copy a recipe (yours or another user's) into an independent recipe that evolves separately — serves as both cross-user sharing and same-user variant creation |

## Acceptance Criteria

### AC1: Versioning schema + backfill

- [ ] Migration creates `recipe_versions` table with reverse-diff columns
- [ ] Migration creates `fork_relationships` table for branching lineage
- [ ] Migration adds `is_draft BOOLEAN` column to `recipes` table
- [ ] Backfill: every existing recipe from NOMS-008 becomes version 1 (full snapshot, `is_latest = true`)
- [ ] Schema is additive-only (IF NOT EXISTS, ALTER TABLE ADD COLUMN) and idempotent

### AC2: Edit creates versions (reverse diff)

- [ ] Edit flow changes from "overwrite row" to "compute reverse diff + insert new version"
- [ ] Each save computes a JSON Patch (RFC 6902) diff between old and new recipe data
- [ ] New version stored as full snapshot with `is_latest = true`
- [ ] Old latest version updated: `is_latest = false`, `reverse_diff` populated (JSON Patch to apply to new version to reconstruct old)
- [ ] `recipes` row updated to match new version (denormalized latest)
- [ ] Original versions remain reconstructable via reverse diff chain

### AC3: View version history

- [ ] "History" tab on recipe detail page
- [ ] Shows timeline of versions with: version number, created date, change summary
- [ ] Clicking a version reconstructs and displays that version's data (title, description, times, servings, ingredients, steps)
- [ ] Reconstruction: start from latest snapshot, apply reverse diffs backwards
- [ ] "What changed" display: show added/removed/modified ingredients and steps between adjacent versions
- [ ] Current version clearly indicated

### AC4: Restore version

- [ ] "Restore" button on historical versions
- [ ] Confirmation: "Restore version {N}? This will create a new version with the data from version {N}."
- [ ] Restore reconstructs the target version, then saves it as a new version (N+1) with full snapshot
- [ ] Original versions remain unchanged
- [ ] User redirected to detail page showing restored data

### AC5: Draft saving

- [ ] `is_draft BOOLEAN` column on `recipes` table (default `false` for existing recipes)
- [ ] New recipes created as drafts (`is_draft = true`)
- [ ] Edit page auto-saves as draft on field changes (debounced, e.g., 2s)
- [ ] "Publish" button on edit page sets `is_draft = false`
- [ ] Dashboard shows draft indicator on draft recipes
- [ ] Dashboard filter toggle: show/hide drafts

### AC6: Recipe forking (cross-user and same-user variants)

- [ ] "Fork" button on recipe detail page (visible to anyone who can access the recipe, including the owner for creating variants)
- [ ] Fork creates a new recipe owned by the forking user (if forking your own recipe, you own both the original and the fork)
- [ ] Forked recipe starts as version 1 with a full snapshot copied from the source recipe's latest version
- [ ] `fork_relationships` row records: original recipe ID, forked recipe ID, forked by user, fork date
- [ ] Forked recipe is independent: edits to one don't affect the other
- [ ] Forked recipe shows "Forked from {original title}" attribution
- [ ] Forked recipe created as draft (`is_draft = true`)

### AC7: Versioning DB queries and types

- [ ] Rust types in `src/db/mod.rs`: `RecipeVersion`, `ForkRelationship`
- [ ] Query functions:
  - `insert_recipe_version()` — create new version (full snapshot, is_latest = true)
  - `update_previous_latest()` — set old latest to is_latest = false, store reverse_diff
  - `get_recipe_by_id_and_owner()` — updated: reads from `recipes` row (denormalized latest)
  - `get_recipe_versions()` — fetch all versions for a recipe ordered by version_number DESC
  - `reconstruct_version()` — fetch latest snapshot + all reverse diffs, return reconstructed data for version N
  - `restore_version()` — reconstruct target version, insert as new version
  - `fork_recipe()` — copy source recipe's latest data into new recipe + v1 snapshot
  - `insert_fork_relationship()` — record fork lineage
  - `get_fork_info()` — fetch fork source info for a recipe
  - `update_draft_status()` — update is_draft on recipes table
  - `get_recipes_by_owner_with_draft_filter()` — list user's recipes with optional draft filter
- [ ] All queries guard by `owner_id` where appropriate
- [ ] Tests for each query function, including reconstruction correctness

## Technical Details

### Database Schema (new tables + alterations)

```sql
-- Add draft column to existing recipes table
ALTER TABLE recipes ADD COLUMN IF NOT EXISTS is_draft BOOLEAN NOT NULL DEFAULT FALSE;

-- Recipe versions (reverse-diff chain, latest is anchor)
CREATE TABLE IF NOT EXISTS recipe_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    version_number INTEGER NOT NULL,

    -- Full snapshot data (populated for latest version only)
    title VARCHAR(200),
    description TEXT,
    prep_time_min INTEGER,
    cook_time_min INTEGER,
    total_time_min INTEGER,
    servings INTEGER,
    ingredients JSONB,
    steps JSONB,

    -- Reverse diff: JSON Patch (RFC 6902) to apply to NEXT version to get THIS version
    -- NULL for latest version, populated for all historical versions
    reverse_diff JSONB,

    notes TEXT,
    is_latest BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(recipe_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_recipe_versions_recipe ON recipe_versions(recipe_id, version_number DESC);
CREATE INDEX IF NOT EXISTS idx_recipe_versions_latest ON recipe_versions(recipe_id, is_latest) WHERE is_latest = TRUE;

-- Fork relationships (DAG of recipe lineage)
CREATE TABLE IF NOT EXISTS fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_recipe_id UUID NOT NULL REFERENCES recipes(id),
    forked_recipe_id UUID NOT NULL REFERENCES recipes(id),
    forked_by UUID NOT NULL REFERENCES users(id),
    forked_version_number INTEGER,  -- Version of original that was forked
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_forks_original ON fork_relationships(original_recipe_id);
CREATE INDEX IF NOT EXISTS idx_forks_result ON fork_relationships(forked_recipe_id);
```

### Reverse Diff Strategy

**Storage model:**
- Latest version: full snapshot, `reverse_diff = NULL`, `is_latest = TRUE`
- Historical versions: `reverse_diff` = JSON Patch array, snapshot columns are NULL, `is_latest = FALSE`

**Save new version (v4):**
1. Read current latest (v3) snapshot
2. Compute JSON Patch diff: `v3 → v4` using `json-patch` crate
3. Reverse the patch operations (add↔remove, swap from/value in replace)
4. Update v3: `is_latest = FALSE`, `reverse_diff = reversed patch`
5. Insert v4: full snapshot, `is_latest = TRUE`, `reverse_diff = NULL`
6. Update `recipes` row to match v4

**Reconstruct version N:**
1. Fetch latest version snapshot (v_latest)
2. Fetch all `reverse_diff` values for versions N through latest-1, ordered ascending
3. Apply each reverse_diff in sequence: `v_latest → v_latest-1 → ... → v_N`
4. Return reconstructed data

**"What changed" between v(N) and v(N+1):**
- v(N)'s `reverse_diff` already stores the patch from v(N+1) to v(N)
- Display: iterate patch operations, show added/removed/modified ingredients and steps

### Fork Flow

Works for both cross-user forking and same-user variant creation.

```
1. User clicks "Fork" on recipe R (owned by user A, may be same as current user)
2. Fetch R's latest version snapshot
3. Create new recipe R' owned by current user, is_draft = true
4. Insert R' v1: full snapshot copy of R's latest, is_latest = true
5. Insert fork_relationships row: original=R, forked=R', forked_by=current user
6. Redirect to R' edit page (as draft)
```

### Server functions (new/updated)

| Function | Purpose |
|----------|---------|
| `update_recipe` (modified) | Compute reverse diff, update old latest, insert new version, update recipes row |
| `get_recipe_versions(recipe_id, user_id)` | Fetch all versions (latest as snapshot, others as summaries with reverse_diff) |
| `reconstruct_version(recipe_id, user_id, version_number)` | Reconstruct historical version from reverse diff chain |
| `restore_version(recipe_id, user_id, version_number)` | Reconstruct target version, save as new version |
| `fork_recipe(source_recipe_id, source_user_id, message)` | Fork recipe into new independent recipe |
| `save_draft(recipe_id, user_id, ...)` | Auto-save as draft (same as update but keeps is_draft = true) |
| `publish_recipe(recipe_id, user_id)` | Set is_draft = false |
| `list_my_recipes(user_id, offset, limit, include_drafts)` | Paginated list with draft filter |

### Diff format

JSON Patch (RFC 6902) array. Each operation:
```json
[
  { "op": "replace", "path": "/title", "value": "New Title" },
  { "op": "add", "path": "/ingredients/3", "value": { "amount": "1", "unit": "tsp", "name": "salt", "note": "" } },
  { "op": "remove", "path": "/steps/2" }
]
```

Rust crate: `json-patch` (or `json_patch`) for computing and applying patches.

### AuthContext changes

No changes needed. Same ownership gating as NOMS-008.

### Route protection changes

No new routes. Existing `/recipes/:id` and `/recipes/:id/edit` routes gain versioning/draft/branching features.

### Component changes

| Component | Change |
|-----------|--------|
| `RecipeDetail` | Add "History" tab, "Fork" button (for non-owners), draft indicator |
| `RecipeEdit` | Auto-save draft on changes, "Publish" button, draft state indicator |
| `Dashboard` | Draft filter toggle, draft indicator on cards |
| New: `VersionTimeline` | Version history timeline with expand/collapse, restore button |
| New: `VersionDiff` | Show what changed between adjacent versions |

## Out of Scope

- Visual diff viewer (highlight changes in ingredients/steps side-by-side) — future enhancement
- Fork merging — not planned
- Git-like branching (parallel version chains within a single recipe) — not needed; same-user variants are handled by forking your own recipe
- Collaborative editing — not planned
- Conflict resolution for concurrent edits — not planned (first-write-wins for now)

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | Schema migration + backfill | `recipe_versions` + `fork_relationships` tables, `is_draft` column, existing recipes backfilled as v1 |
| 2 | Reverse diff library + queries | `json-patch` integration, diff computation, reconstruction logic, all query functions tested |
| 3 | Versioned edit flow | Edit creates versions via reverse diff, `recipes` row stays in sync |
| 4 | Version history UI | Timeline component, reconstruct and display historical versions |
| 5 | Restore version | Reconstruct + save as new version |
| 6 | Draft saving | Auto-save drafts, publish flow, dashboard draft filter |
| 7 | Recipe forking | Fork button, copy recipe, fork_relationships tracking, attribution |

## Success Metrics

- User edits recipe → v2 created with reverse diff, v1 reconstructable
- User views history → sees timeline, expands old version, data matches what was saved
- User restores v1 → v3 created with v1's data
- User saves draft → auto-saved, shows draft badge, publish makes it live
- User forks recipe → independent copy created, fork attribution shown
- All 7 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- Reconstruction correctness: reconstruct any version, compare against known data
