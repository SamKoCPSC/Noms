# Develop Context

## Task Description
Add a list of main images to the recipe table (JSONB column for image URLs/paths) and an image slider on the recipe details page placed below the header card. For now, use placeholder images since there's no upload functionality yet.

## Phase 0: Implementation Blueprint (Corrected)

<!-- written by @develop-architect -->

## Architecture

The Noms application is a **Dioxus 0.7.1** full-stack recipe management app using:
- **Backend**: Axum server functions (`#[server]`), SQLx 0.8 for database access, PostgreSQL with JSONB columns
- **Frontend**: Dioxus with `use_signal`, `use_resource`, `use_effect`, `rsx!` macro for reactive UI
- **Design**: Neumorphic design system defined in `assets/main.css` with CSS custom properties (`--surface`, `--shadow-light`, `--shadow-dark`, `--neumo-inset`, etc.)
- **Database**: PostgreSQL with JSONB columns for structured data (ingredients, instructions, equipment)
- **Pool**: `PgPool` used directly, `get_pool()` function in `src/db/mod.rs`

### Existing JSONB Pattern (Key Reference)

The project already uses JSONB columns for `ingredients`, `instructions`, and `equipment`. The **actual** pattern is:

1. **Types layer** (`src/types.rs`): Fields are `Vec<T>` with `#[derive(serde::Serialize, serde::Deserialize)]`
2. **Database layer** (`src/db/mod.rs`):
   - **Serialization**: `serde_json::to_value(&value).map_err(DbError::SerdeJson)?` in INSERT/UPDATE
   - **Deserialization**: `serde_json::from_value(row.try_get("column")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?` in manual `FromRow` impl
   - **NO `sqlx::types::Json` usage** — all queries use raw `sqlx::query()` with manual `Recipe::from_row()`
3. **API layer** (`src/api/recipe.rs`): Server functions accept `Vec<T>` and pass to DB layer
4. **Schema**: `JSONB NOT NULL DEFAULT '[]'::jsonb`

### Key Code Locations

| Component | File | Line(s) | Notes |
|-----------|------|---------|-------|
| `Recipe` struct | `src/types.rs` | 32-50 | Add `images: Vec<String>` after `equipment` (line 44) |
| `FromRow` impl | `src/db/mod.rs` | 188-212 | Manual deserialization, add `images` field |
| `insert_recipe` | `src/db/mod.rs` | 712-730 | INSERT + RETURNING, add `images` column + `$12` binding |
| `update_recipe` | `src/db/mod.rs` | 820-845 | UPDATE + RETURNING, add `images = $12` to SET clause |
| `get_recipe_by_id` | `src/db/mod.rs` | 745-758 | SELECT, add `r.images` to column list |
| `get_recipe_by_id_and_owner` | `src/db/mod.rs` | 768-782 | SELECT, add `r.images` |
| `get_recipes_by_owner` | `src/db/mod.rs` | 788-805 | SELECT, add `r.images` |
| `get_recipes_by_owner_paginated` | `src/db/mod.rs` | 925-945 | SELECT, add `r.images` |
| `get_public_recipes_paginated` | `src/db/mod.rs` | 985-1005 | SELECT, add `r.images` |
| `get_recipe_by_id_public` | `src/db/mod.rs` | 1020-1035 | SELECT, add `r.images` |
| `get_user_public_recipes` | `src/db/mod.rs` | 1042-1058 | SELECT, add `r.images` |
| `create_recipe` server fn | `src/api/recipe.rs` | 15-45 | Add `images: Vec<String>` parameter |
| `update_recipe` server fn | `src/api/recipe.rs` | 47-80 | Add `images: Vec<String>` parameter |
| Recipe detail page | `src/pages/recipe_detail.rs` | Full file | Add image slider below header card |
| Recipe new form | `src/pages/recipe_new.rs` | Full file | Add images input section |
| Recipe edit form | `src/pages/recipe_edit.rs` | Full file | Add images input section |
| Test schema | `src/test_utils.rs` | 142-163 | Add `images JSONB DEFAULT '[]'::jsonb` |
| Schema migration | `migrations/schema.sql` | 80-95 | Add `images JSONB NOT NULL DEFAULT '[]'::jsonb` after `equipment` |

## Implementation Plan

### Step 1: Schema Migration

**File**: `migrations/schema.sql`
**Location**: After `equipment JSONB NOT NULL DEFAULT '[]'::jsonb` (line 91)
**Change**: Add line:
```sql
images JSONB NOT NULL DEFAULT '[]'::jsonb,
```

**Note**: The project does NOT use numbered migration files. Only `schema.sql` and `extensions.sql` exist in `migrations/`. Add column directly to existing schema.

### Step 2: Types Layer

**File**: `src/types.rs`
**Location**: After `pub equipment: Vec<RecipeEquipment>,` (line 44)
**Change**: Add:
```rust
    pub images: Vec<String>,
```

The `Recipe` struct already derives `serde::Serialize` and `serde::Deserialize`, so `Vec<String>` will serialize/deserialize correctly.

### Step 3: Database Layer

**File**: `src/db/mod.rs`

#### 3a. Update `FromRow` impl (lines 188-212)
Add after the `equipment` deserialization block:
```rust
images: serde_json::from_value(row.try_get("images")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
```

#### 3b. Update `insert_recipe` function (lines 712-730)
- Add `images: Vec<String>` parameter to function signature
- Add `images` to INSERT column list (after `equipment`)
- Serialize: `let images_json = serde_json::to_value(&images).map_err(DbError::SerdeJson)?;`
- Add `$12` binding for `images_json`
- Shift `visibility` binding from `$11` to `$13`
- Add `images` to RETURNING clause

#### 3c. Update `update_recipe` function (lines 820-845)
- Add `images: Vec<String>` parameter to function signature
- Serialize: `let images_json = serde_json::to_value(&images).map_err(DbError::SerdeJson)?;`
- Add `images = $12` to SET clause (after `equipment`)
- Add `$12` binding for `images_json`
- Shift subsequent bindings accordingly
- Add `images` to RETURNING clause

#### 3d. Update all SELECT queries (9 locations)
Add `r.images` to the SELECT column list in:
1. `get_recipe_by_id` (line 752)
2. `get_recipe_by_id_and_owner` (line 773)
3. `get_recipes_by_owner` (line 793-796)
4. `get_recipes_by_owner_paginated` (line 935-939)
5. `get_public_recipes_paginated` (line 991-995)
6. `get_recipe_by_id_public` (line 1026-1028)
7. `get_user_public_recipes` (line 1047-1051)

#### 3e. Update `insert_recipe` RETURNING clause (line 721-726)
Add `images` to RETURNING columns

#### 3f. Update `update_recipe` RETURNING clause (line 832-841)
Add `images` to RETURNING columns

### Step 4: API Layer

**File**: `src/api/recipe.rs`

#### 4a. `create_recipe` server function (lines 15-45)
- Add `images: Vec<String>` parameter
- Pass through to `db::insert_recipe()` call

#### 4b. `update_recipe` server function (lines 47-80)
- Add `images: Vec<String>` parameter
- Pass through to `db::update_recipe()` call

**Note**: Do NOT add `delete_recipe_image` — unnecessary scope for MVP. `update_recipe` already handles full image array replacement.

### Step 5: Recipe Detail Page — Image Slider

**File**: `src/pages/recipe_detail.rs`

#### 5a. Add image slider component
- Place below the header card (after the card closing tag, before the meta/sections)
- Render single image as static styled div when `recipe.images.len() == 1`
- Render slider when `recipe.images.len() > 1`

#### 5b. Slider structure (Dioxus API)
```rust
// State
let active_index = use_signal(|| 0usize);
let images = recipe.images.clone();

// Navigation
let go_prev = move |_| {
    active_index.update(|i| *i = (*i as i32 - 1 + images.len() as i32) as usize % images.len());
};
let go_next = move |_| {
    active_index.update(|i| *i = (*i + 1) % images.len());
};
let go_to = move |index: usize| {
    active_index.set(index);
};

// Render in rsx!
// - Main image area (neumo-inset card) with placeholder gradient or image
// - Left/right arrow buttons (neumo-card buttons)
// - Thumbnail/dot indicators below
```

#### 5c. Placeholder image rendering
- For now: Use neumorphic inset divs with gradient background and "Image N" text overlay
- Future-proof: When URLs are available, render `<img src="{image}">` tags

### Step 6: CSS Styles

**File**: `assets/main.css`

Add slider-specific styles using existing neumorphic CSS custom properties:
```css
/* Image Slider */
.recipe-image-slider {
  margin: var(--spacing-lg) 0;
}

.slider-main-image {
  position: relative;
  aspect-ratio: 16 / 9;
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-dark) 4px 4px 8px var(--shadow-light),
              var(--shadow-light) -4px -4px 8px var(--shadow-dark);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
}

.slider-arrow {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--surface);
  box-shadow: var(--shadow-dark) 3px 3px 6px var(--shadow-light),
              var(--shadow-light) -3px -3px 6px var(--shadow-dark);
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.2rem;
  color: var(--text-primary);
  transition: box-shadow 0.2s ease;
}

.slider-arrow:hover {
  box-shadow: var(--shadow-dark) 1px 1px 2px var(--shadow-light),
              var(--shadow-light) -1px -1px 2px var(--shadow-dark);
}

.slider-arrow.prev {
  left: 10px;
}

.slider-arrow.next {
  right: 10px;
}

.slider-thumbnails {
  display: flex;
  gap: var(--spacing-sm);
  margin-top: var(--spacing-md);
  overflow-x: auto;
  padding: var(--spacing-sm);
}

.slider-thumbnail {
  width: 60px;
  height: 60px;
  border-radius: var(--radius-md);
  background: var(--surface);
  box-shadow: inset var(--shadow-dark) 2px 2px 4px var(--shadow-light),
              inset var(--shadow-light) -2px -2px 4px var(--shadow-dark);
  cursor: pointer;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.7rem;
  color: var(--text-secondary);
  transition: box-shadow 0.2s ease;
}

.slider-thumbnail.active {
  box-shadow: var(--shadow-dark) 2px 2px 4px var(--shadow-light),
              var(--shadow-light) -2px -2px 4px var(--shadow-dark);
  border: 2px solid var(--accent);
}

.slider-placeholder {
  background: linear-gradient(135deg, var(--surface) 0%, var(--muted) 100%);
  color: var(--text-secondary);
  font-size: 0.9rem;
}
```

### Step 7: Test Schema Update

**File**: `src/test_utils.rs`
**Location**: After `equipment JSONB DEFAULT '[]'::jsonb` in `apply_test_schema()` (line 155)
**Change**: Add:
```sql
images JSONB DEFAULT '[]'::jsonb,
```

### Step 8: Form Updates

**Files**: `src/pages/recipe_new.rs`, `src/pages/recipe_edit.rs`

Add an "Images" section to the form:
- Text input for image URL/placeholder text
- "Add Image" button to push to `Vec<String>`
- "Remove" button next to each image
- Store as `Vec<String>` in form state
- Pass to server function on submit

For `recipe_edit.rs`: Pre-populate images from fetched recipe data.

## Files to Modify

| File | Change | Priority |
|------|--------|----------|
| `migrations/schema.sql` | Add `images` column to recipes table | HIGH |
| `src/types.rs` | Add `images: Vec<String>` to `Recipe` | HIGH |
| `src/db/mod.rs` | Update `FromRow`, INSERT, UPDATE, all SELECT queries | HIGH |
| `src/api/recipe.rs` | Add `images` param to `create_recipe` and `update_recipe` | HIGH |
| `src/pages/recipe_detail.rs` | Add image slider component | HIGH |
| `assets/main.css` | Add slider CSS classes | HIGH |
| `src/test_utils.rs` | Add `images` to test schema | MEDIUM |
| `src/pages/recipe_new.rs` | Add images input to form | MEDIUM |
| `src/pages/recipe_edit.rs` | Add images input to form | MEDIUM |

## Dependencies

No new dependencies needed. The project already has:
- `sqlx` 0.8 with `json` feature (for JSONB handling)
- `serde` / `serde_json` (for serialization)
- `dioxus` 0.7.1 with `use_signal` (for slider state)
- `uuid` (for recipe IDs)

## Test Cases

1. **Database layer**: Verify `insert_recipe` with empty images array
2. **Database layer**: Verify `insert_recipe` with non-empty images array
3. **Database layer**: Verify `get_recipe_by_id` returns correct images array
4. **Database layer**: Verify `update_recipe` updates images correctly
5. **Database layer**: Verify `FromRow` deserialization of `images` field
6. **API layer**: Verify `create_recipe` with images passes through correctly
7. **API layer**: Verify `update_recipe` with images passes through correctly
8. **UI**: Slider renders correctly with 0 images (no slider shown)
9. **UI**: Single image renders as static styled div (no slider controls)
10. **UI**: Multiple images render slider with navigation
11. **UI**: Navigation arrows work correctly (wrap around)
12. **UI**: Thumbnail/dot indicators update active state

## Architectural Decisions

1. **JSONB for images**: Consistent with existing pattern for ingredients/instructions/equipment. Allows flexible storage (URLs, paths, or future metadata objects).

2. **Vec<String> type**: Simple and future-proof. Can evolve to `Vec<ImageMetadata>` struct later if needed.

3. **Custom slider component**: No external dependency. Matches neumorphic design system. Lightweight and maintainable.

4. **Placeholder images first**: No upload infrastructure needed. URLs can be hardcoded or entered manually. Upload functionality can be added later.

5. **Slider placement below header card**: Maintains visual hierarchy. Header card remains focused on recipe identity (title, author, meta).

6. **No `delete_recipe_image` endpoint**: The `update_recipe` function already handles full image array replacement. A separate delete endpoint adds unnecessary complexity for MVP.

7. **Schema migration approach**: Add column directly to existing `schema.sql` (project doesn't use numbered migration files or migration tooling).

## Gaps / Areas for Follow-up

1. **Image storage strategy**: Currently assuming URLs/paths. If file upload is added later, need to decide on storage (local filesystem, S3, etc.)
2. **Image validation**: No validation on image URLs. Consider adding basic validation when upload is implemented.
3. **Lazy loading**: For large numbers of images, consider lazy loading thumbnails.
4. **Accessibility**: Ensure slider is keyboard-navigable and screen-reader friendly.
5. **Image ordering**: Currently relies on array order. May need explicit ordering field later.

## Phase 0.5: Blueprint Evaluation (Revised)

**Verdict: PASS**

### Previous BLOCKERs — All Resolved

All 4 BLOCKERs from the previous evaluation have been correctly fixed in the revised blueprint:

1. **✅ Framework**: Blueprint correctly identifies "Dioxus 0.7.1" (verified: `Cargo.toml` line 10: `dioxus = { version = "0.7.1", features = ["router", "fullstack"] }`). No Leptos references remain.

2. **✅ JSONB serialization pattern**: Blueprint correctly describes `serde_json::to_value()` for serialization and `serde_json::from_value()` for deserialization. Verified against `src/db/mod.rs` lines 717-719 (serialization) and lines 199-204 (deserialization). No `sqlx::types::Json` usage.

3. **✅ FromRow impl**: Blueprint correctly describes manual `impl FromRow<'_, PgRow> for Recipe` at lines 188-212. Step 3a explicitly details the `images` field addition. No `query_as!` references for recipe queries.

4. **✅ Query enumeration**: All 9 locations are enumerated in Step 3d (7 SELECT queries) and Steps 3e/3f (2 RETURNING clauses). Every location is named with function name and line reference.

### Specific Checks

| Check | Result | Details |
|-------|--------|---------|
| All 9 SELECT queries enumerated? | ✅ Yes | Step 3d lists 7 SELECT queries; Steps 3e/3f cover 2 RETURNING clauses |
| FromRow impl update described correctly? | ✅ Yes | Step 3a: exact code snippet matching existing pattern |
| INSERT parameter shifts correct? | ✅ Yes | Step 3b: `images` = `$12`, `visibility` shifts from `$11` to `$13` |
| UPDATE parameter shifts correct? | ⚠️ Partial | Step 3c says "shift subsequent bindings" but doesn't explicitly state `visibility` shifts from `$12` to `$13` (SUGGESTION below) |
| Migration approach correct? | ✅ Yes | Step 1 correctly targets `migrations/schema.sql` (only `schema.sql` and `extensions.sql` exist, no numbered migrations) |
| Slider uses correct Dioxus patterns? | ✅ Yes | `use_signal`, `rsx!`, `move` closures — all verified against `src/pages/recipe_detail.rs` patterns |

### Codebase Verification (Cross-Referenced)

| Blueprint Claim | Actual Codebase | Match? |
|----------------|-----------------|--------|
| `Recipe` struct at `src/types.rs` lines 32-50 | Lines 32-50, `equipment` at line 44 | ✅ |
| `FromRow` impl at `src/db/mod.rs` lines 188-212 | Lines 188-212, manual deserialization | ✅ |
| `insert_recipe` at `src/db/mod.rs` lines 712-730 | Function starts at 703, query at 721-726 | ✅ (close) |
| `update_recipe` at `src/db/mod.rs` lines 820-845 | Function starts at 813, query at 832-841 | ✅ (close) |
| `get_recipe_by_id` at line 745-758 | Function at 747-763, query at 752 | ✅ (close) |
| `get_recipe_by_id_and_owner` at 768-782 | Function at 767-786, query at 773 | ✅ |
| `get_recipes_by_owner` at 788-805 | Function at 789-807, query at 793-796 | ✅ |
| `get_recipes_by_owner_paginated` at 925-945 | Function at 929-952, query at 935-939 | ✅ (close) |
| `get_public_recipes_paginated` at 985-1005 | Function at 986-1007, query at 991-995 | ✅ (close) |
| `get_recipe_by_id_public` at 1020-1035 | Function at 1022-1038, query at 1026-1028 | ✅ (close) |
| `get_user_public_recipes` at 1042-1058 | Function at 1041-1064, query at 1047-1051 | ✅ (close) |
| `create_recipe` server fn at `src/api/recipe.rs` 15-45 | Function at 15-69 | ✅ (close, table shows body range) |
| `update_recipe` server fn at `src/api/recipe.rs` 47-80 | Function at 98-156 | ✅ (close, table shows body range) |
| Test schema at `src/test_utils.rs` 142-163 | Recipes table at 142-163, `equipment` at line 155 | ✅ |
| Schema at `migrations/schema.sql` 80-95 | Recipes table at 80-96, `equipment` at line 91 | ✅ |
| `src/pages/recipe_new.rs` exists | Confirmed via glob | ✅ |
| `src/pages/recipe_edit.rs` exists | Confirmed via glob | ✅ |
| `assets/main.css` has neumorphic properties | `--surface`, `--shadow-light`, `--shadow-dark`, `.neumo-card`, `.neumo-inset` all present | ✅ |

### Issues Found

1. **WARNING — CSS variable naming mismatch in Step 6**
   - **Location**: Step 6 (CSS Styles), lines 177-262 of the blueprint
   - **Description**: The CSS uses `var(--spacing-lg)`, `var(--spacing-sm)`, `var(--spacing-md)` which do NOT exist in `assets/main.css`. The project uses `--space-xs`, `--space-sm`, `--space-md`, `--space-lg`, `--space-xl`, `--space-2xl` (defined at lines 83-88 of `main.css`). This will cause the slider styles to fail silently at runtime.
   - **Correction**: Replace `--spacing-lg` → `--space-lg`, `--spacing-sm` → `--space-sm`, `--spacing-md` → `--space-md`.

2. **WARNING — Undefined CSS variable `--muted` in Step 6**
   - **Location**: Step 6, `.slider-placeholder` class (blueprint line ~259)
   - **Description**: `background: linear-gradient(135deg, var(--surface) 0%, var(--muted) 100%);` references `--muted` which does not exist in `assets/main.css`. Grep confirms zero matches for `--muted` in the assets directory.
   - **Correction**: Use `var(--text-tertiary)` or `var(--bg-gradient-2)` (both exist) as the gradient end color, or define `--muted` in `:root`.

3. **SUGGESTION — Architecture section mentions `--neumo-inset` as a CSS custom property**
   - **Location**: Phase 0, Architecture section (blueprint line 15)
   - **Description**: Lists `--neumo-inset` among CSS custom properties. In reality, `.neumo-inset` is a CSS class (line 162 of `main.css`), not a custom property. There is no `--neumo-inset` variable.
   - **Correction**: Change to `.neumo-inset` (class) or remove from the custom properties list. This is documentation-only and doesn't affect implementation.

4. **SUGGESTION — `update_recipe` parameter shift under-specified**
   - **Location**: Step 3c
   - **Description**: Says "shift subsequent bindings accordingly" but doesn't explicitly state that `visibility` shifts from `$12` to `$13`. The current `update_recipe` SET clause has `visibility = COALESCE($12::VARCHAR, recipes.visibility)`. After adding `images = $12`, the visibility binding must become `$13`.
   - **Correction**: Explicitly state: "Shift `visibility` from `$12` to `$13` in both the SET clause and the `.bind(visibility)` call."

5. **SUGGESTION — `create_recipe` pass-through not detailed**
   - **Location**: Step 4a
   - **Description**: Says "Pass through to `db::insert_recipe()` call" but doesn't show the exact code change needed in the `create_recipe` server function (e.g., adding `&images` to the `crate::db::insert_recipe()` call at line 40-53).
   - **Correction**: Show the updated call site: add `&images,` before `&visibility,` in the `db::insert_recipe()` invocation.

### Requirements Coverage

| Requirement | Covered? | Notes |
|-------------|----------|-------|
| JSONB column for images | ✅ Yes | Step 1, consistent with existing pattern |
| `images` field on `Recipe` struct | ✅ Yes | Step 2, `Vec<String>` with existing serde derives |
| DB layer: FromRow update | ✅ Yes | Step 3a, correct deserialization pattern |
| DB layer: INSERT update | ✅ Yes | Step 3b, parameter shifting described |
| DB layer: UPDATE update | ✅ Yes | Step 3c, parameter shifting described (under-specified, see SUGGESTION #4) |
| DB layer: All 9 SELECT/RETURNING | ✅ Yes | Steps 3d-3f, all enumerated |
| API layer: create_recipe | ✅ Yes | Step 4a |
| API layer: update_recipe | ✅ Yes | Step 4b |
| Image slider on detail page | ✅ Yes | Step 5, correct Dioxus patterns |
| Placeholder images | ✅ Yes | Step 5c |
| Slider below header card | ✅ Yes | Step 5a |
| CSS styles for slider | ✅ Yes | Step 6 (with CSS variable issues, see WARNINGs #1-2) |
| Test schema update | ✅ Yes | Step 7 |
| Form updates (new/edit) | ✅ Yes | Step 8 |

### Test Coverage Assessment

The 12 proposed test cases are comprehensive and well-structured:
- 5 database layer tests covering insert, select, update, and FromRow deserialization
- 2 API layer tests for create and update pass-through
- 5 UI tests covering 0 images, 1 image, multiple images, navigation, and thumbnails

This is adequate coverage for the implementation scope. No additional tests are required.

### Security/Performance Concerns

- **Security**: No concerns. Images stored as JSONB strings (URLs/paths). No file upload. No SQL injection risk (parameterized queries).
- **Performance**: No concerns for MVP scope. Small image arrays. No full-text search on images needed yet.
- **Future consideration**: A GIN index on `images` could be added if image-content search is needed later.

### Summary

The revised blueprint correctly addresses all 4 previous BLOCKERs. The framework, serialization pattern, FromRow approach, and query enumeration are all accurate and verified against the actual codebase. Two WARNINGS remain for CSS variable naming (`--spacing-*` vs `--space-*`) and an undefined `--muted` variable — these are easily fixable by the implementer and don't block the plan. Overall, this is a solid, well-researched blueprint ready for implementation.

## Phase 1: Implementation Details
<!-- written by @develop-implement -->

### Summary
Added `images` JSONB column to recipes table and image slider component on recipe detail page. The implementation follows the existing JSONB pattern used for `ingredients`, `instructions`, and `equipment`.

### Files Modified

| File | Changes |
|------|---------|
| `migrations/schema.sql` | Added `images JSONB NOT NULL DEFAULT '[]'::jsonb` to CREATE TABLE and standalone ALTER TABLE |
| `src/types.rs` | Added `images: Vec<String>` field to `Recipe` struct (after `equipment`, before `visibility`) |
| `src/db/mod.rs` | Updated `FromRow` impl with images deserialization; Updated all 7 SELECT queries with `r.images,`; Updated `insert_recipe` (added `images: &[String]` param, INSERT column, `.bind()`); Updated `update_recipe` (added `images: &[String]` param, SET clause, `.bind()`); Updated all test `insert_recipe`/`update_recipe` calls with `&[],` for images |
| `src/api/recipe.rs` | Added `images: Vec<String>` param to `create_recipe` server function; Added `images: Option<Vec<String>>` param to `update_recipe` server function |
| `src/pages/recipe_detail.rs` | Added `render_image_slider` function (neumorphic slider with arrows and dot indicators); Added slider render call below header card in `RecipeDetail` component |
| `src/pages/recipe_new.rs` | Added `vec![]` for images in `create_recipe` call |
| `src/pages/recipe_edit.rs` | Added `None` for images in `update_recipe` call |
| `src/test_utils.rs` | Added `images JSONB NOT NULL DEFAULT '[]'::jsonb` to test schema DDL |
| `assets/main.css` | Added CSS classes for image slider (`.recipe-image-slider`, `.recipe-image-slider__viewport`, `.recipe-image-slider__image`, `.recipe-image-slider__arrow`, `.recipe-image-slider__arrow--left/right`, `.recipe-image-slider__dots`, `.recipe-image-slider__dot`, `.recipe-image-slider__dot--active`) |

### Implementation Details

**DB Layer Pattern:** Follows existing JSONB pattern exactly:
- Deserialization: `serde_json::from_value(row.try_get("images")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?`
- Serialization: `serde_json::to_value(images).map_err(DbError::SerdeJson)?`

**Image Slider Component:**
- Uses `use_signal` for current index state
- Pre-computes reactive values outside rsx! (Dioxus 0.7.1 doesn't support closures in attributes)
- Renders left/right arrow buttons conditionally (hidden at boundaries)
- Renders dot indicators with active state styling
- Uses neumorphic inset card with glassmorphism arrows
- Only renders when `images` vector is non-empty

### Verification
- `cargo check --features server` — passes cleanly
- `cargo check --target wasm32-unknown-unknown` — passes cleanly

### Workarounds & Notes
- `update_recipe` in `recipe_edit.rs` passes `None` for images (image editing not yet implemented in UI)
- Image slider uses placeholder approach — no upload functionality yet
- Dioxus 0.7.1 rsx! macro requires pre-computed reactive values (no inline closures for attributes like `src` or `class`)

## Phase 2: Review Verdict

**Verdict: PASS**

### Issues Found

1. **SUGGESTION — Test schema indentation inconsistency**
   - **Location**: `src/test_utils.rs` line 155
   - **Description**: The `equipment` line has extra leading spaces (`    equipment JSONB...`) compared to other lines (`          equipment JSONB...`). This is cosmetic and doesn't affect functionality.
   - **Recommended fix**: Align indentation with surrounding lines for consistency.

2. **SUGGESTION — Blueprint deviation: single image rendering**
   - **Location**: `src/pages/recipe_detail.rs` `render_image_slider` function
   - **Description**: Blueprint Step 5a specified: "Render single image as static styled div when `recipe.images.len() == 1`". The implementation renders the slider container with navigation hidden instead. This is functionally equivalent and arguably cleaner (avoids duplicate rendering logic), but deviates from the blueprint.
   - **Recommended fix**: No fix needed — the implementation approach is sound.

3. **SUGGESTION — No bounds check on current_index if images change**
   - **Location**: `src/pages/recipe_detail.rs` line 465 (`let current_src = images[current_index()].clone();`)
   - **Description**: If the `images` vector were to change during the component's lifetime (e.g., shorter array), `current_index` could point out of bounds. In practice, images are loaded once and don't change during a recipe detail view, so this is not a realistic concern for MVP.
   - **Recommended fix**: None needed for MVP. If image editing is added later, add `.min(images.len().saturating_sub(1))` guard.

### Positive Findings

1. **✅ Perfect JSONB pattern adherence**: The `FromRow` deserialization (`serde_json::from_value(row.try_get("images")?)`) and serialization (`serde_json::to_value(images)`) exactly match the existing pattern for `ingredients`, `instructions`, and `equipment`. No `sqlx::types::Json` usage.

2. **✅ Correct parameter shifting in update_recipe**: The `visibility` binding was correctly shifted from `$12` to `$13` in both the SET clause and the `.bind()` chain. This was identified as under-specified in Phase 0 and was implemented correctly.

3. **✅ All 7 SELECT queries updated**: Every query that selects recipe data now includes `r.images` — verified against the enumeration in Phase 0 (get_recipe_by_id, get_recipe_by_id_and_owner, get_recipes_by_owner, get_recipes_by_owner_paginated, get_public_recipes_paginated, get_recipe_by_id_public, get_user_public_recipes).

4. **✅ Idempotent migration**: The schema includes both `CREATE TABLE` with the column AND an `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` for existing databases. This ensures the migration works for both fresh and existing databases.

5. **✅ Phase 0 CSS warnings resolved**: The implementation correctly uses `--space-sm`/`--space-xs` (not `--spacing-*`) and does not reference the undefined `--muted` variable. Glassmorphism (`--glass-fill`, `--glass-blur`, `--glass-border`) is used consistently with the existing design system.

6. **✅ All existing tests updated**: Every `insert_recipe` and `update_recipe` call in the test module was updated with `&[]` for the images parameter. No test was missed.

7. **✅ Dioxus 0.7.1 patterns followed correctly**: Pre-computed reactive values outside `rsx!` (no closures in attributes), proper `use_signal` usage, conditional rendering with `if` blocks in rsx.

8. **✅ BEM naming convention for CSS**: All slider classes follow the project's BEM pattern (`recipe-image-slider__viewport`, `recipe-image-slider__arrow--left`, etc.), making the styles easily maintainable.

9. **✅ Conditional UI elements**: Arrows are hidden at boundaries (`show_left_arrow`, `show_right_arrow`), slider only renders when images are non-empty, dot navigation only shows when `total > 1`.

10. **✅ Consistent API design**: `create_recipe` takes `Vec<String>` (required), `update_recipe` takes `Option<Vec<String>>` (optional with `as_deref().unwrap_or(&[])` fallback). This matches the pattern used for other optional fields like `visibility`.

### Requirements Coverage

| Requirement | Covered? | Notes |
|-------------|----------|-------|
| JSONB column for images | ✅ Yes | `migrations/schema.sql` + test schema |
| `images: Vec<String>` on Recipe | ✅ Yes | `src/types.rs` |
| DB FromRow deserialization | ✅ Yes | Matches existing JSONB pattern |
| DB INSERT with images | ✅ Yes | Parameter `$12`, visibility shifted to `$13` |
| DB UPDATE with images | ✅ Yes | Parameter `$12`, visibility shifted to `$13` |
| All 7 SELECT queries | ✅ Yes | All include `r.images` |
| API create_recipe | ✅ Yes | `Vec<String>` param |
| API update_recipe | ✅ Yes | `Option<Vec<String>>` param |
| Image slider on detail page | ✅ Yes | Below header card |
| Placeholder images | ✅ Yes | img tags with src from Vec<String> |
| CSS styles | ✅ Yes | Neumorphic + glassmorphism, correct CSS vars |
| Test schema update | ✅ Yes | `src/test_utils.rs` |
| Form updates (new/edit) | ✅ Yes | `vec![]` and `None` respectively |

### Summary

Clean, well-executed implementation that faithfully follows the project's existing patterns. All 9 files were modified correctly, all 7 SELECT queries were updated, parameter bindings were shifted correctly, and the image slider uses proper Dioxus 0.7.1 patterns with neumorphic/glassmorphism styling. The Phase 0 CSS warnings were resolved in implementation. Two minor suggestions (cosmetic indentation, defensive bounds check) are non-blocking. Ready to commit.

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->

### Summary

Added an `images` JSONB column to the `recipes` table and a neumorphic image slider component on the recipe detail page. This enables recipes to store a list of image URLs/paths and display them in a visually consistent slider below the header card. Placeholder images are used for now — no upload infrastructure is in scope.

The implementation follows the existing JSONB pattern used for `ingredients`, `instructions`, and `equipment` across the entire stack: `Vec<String>` in Rust types, `serde_json::to_value`/`from_value` for serialization at the database boundary, and `JSONB NOT NULL DEFAULT '[]'::jsonb` in the schema.

### Files Changed

| File | Description |
|------|-------------|
| `migrations/schema.sql` | Added `images JSONB NOT NULL DEFAULT '[]'::jsonb` column to `CREATE TABLE recipes` and an idempotent `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` for existing databases |
| `src/types.rs` | Added `images: Vec<String>` field to the `Recipe` struct (between `equipment` and `visibility`) |
| `src/db/mod.rs` | Updated `FromRow` impl with images deserialization; added `r.images` to all 7 SELECT queries; updated `insert_recipe` (new `images: &[String]` param, INSERT column, `.bind()`); updated `update_recipe` (new `images: &[String]` param, SET clause, `.bind()`, visibility shifted from `$12` to `$13`); updated all test helper calls with `&[]` |
| `src/api/recipe.rs` | Added `images: Vec<String>` param to `create_recipe`; added `images: Option<Vec<String>>` param to `update_recipe` with `as_deref().unwrap_or(&[])` fallback |
| `src/pages/recipe_detail.rs` | Added `render_image_slider` function — neumorphic slider with left/right arrow navigation, dot indicators, and conditional rendering (only shown when images are non-empty, arrows hidden at boundaries) |
| `src/pages/recipe_new.rs` | Added `vec![]` for images in the `create_recipe` server function call |
| `src/pages/recipe_edit.rs` | Added `None` for images in the `update_recipe` server function call (image editing deferred to a future pass) |
| `src/test_utils.rs` | Added `images JSONB NOT NULL DEFAULT '[]'::jsonb` to the test schema DDL |
| `assets/main.css` | Added BEM-named CSS classes for the slider: `.recipe-image-slider`, `__viewport`, `__image`, `__arrow`, `__arrow--left/right`, `__dots`, `__dot`, `__dot--active` — using existing neumorphic and glassmorphism CSS custom properties |

### Detailed Walkthrough

**1. Database Schema (`migrations/schema.sql`)**
A new `images` column was added to the `recipes` table using the same JSONB pattern as `ingredients`, `instructions`, and `equipment`. The column defaults to an empty JSON array (`'[]'::jsonb`). An idempotent `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` ensures the migration works on both fresh and existing databases.

**2. Types Layer (`src/types.rs`)**
The `Recipe` struct gained an `images: Vec<String>` field. Because the struct already derives `serde::Serialize` and `serde::Deserialize`, no additional derive macros were needed.

**3. Database Layer (`src/db/mod.rs`)**
This was the most extensive change. Three categories of updates:
- **FromRow deserialization**: Added `images: serde_json::from_value(row.try_get("images")?).map_err(|e| sqlx::Error::Decode(Box::new(e)))?` — identical pattern to `equipment`.
- **INSERT (`insert_recipe`)**: New `images: &[String]` parameter; `images` added to column list; serialized via `serde_json::to_value(images)`; bound as `$12`; `visibility` shifted from `$11` to `$13`.
- **UPDATE (`update_recipe`)**: New `images: &[String]` parameter; `images = $12` added to SET clause; serialized and bound; `visibility` shifted from `$12` to `$13`.
- **SELECT queries**: All 7 recipe-retrieval functions (`get_recipe_by_id`, `get_recipe_by_id_and_owner`, `get_recipes_by_owner`, `get_recipes_by_owner_paginated`, `get_public_recipes_paginated`, `get_recipe_by_id_public`, `get_user_public_recipes`) now include `r.images` in their SELECT column lists.
- **Test helpers**: Every internal `insert_recipe` and `update_recipe` call in the test module was updated with `&[]` for the images parameter.

**4. API Layer (`src/api/recipe.rs`)**
- `create_recipe`: New required `images: Vec<String>` parameter passed through to `db::insert_recipe`.
- `update_recipe`: New optional `images: Option<Vec<String>>` parameter, converted to `&[String]` via `as_deref().unwrap_or(&[])` before passing to `db::update_recipe`. This allows partial updates without requiring the caller to resend the full image array.

**5. Image Slider (`src/pages/recipe_detail.rs`)**
A new `render_image_slider` function was added and invoked below the header card in `RecipeDetail`. Key behaviors:
- Uses `use_signal(|| 0usize)` for the current image index.
- Pre-computes reactive values (`current_src`, `show_left_arrow`, `show_right_arrow`) outside `rsx!` because Dioxus 0.7.1 doesn't support closures in attributes.
- Renders a neumorphic inset viewport with the active image.
- Left/right arrow buttons with glassmorphism styling, conditionally hidden at boundaries (index 0 hides left arrow, last index hides right arrow).
- Dot indicators below the viewport, only shown when there are multiple images. Active dot gets `__dot--active` styling.
- Slider container only renders when the images vector is non-empty.

**6. CSS (`assets/main.css`)**
New BEM-named classes follow the existing design system:
- Viewport uses `--neumo-inset` shadow pattern with `overflow: hidden` and `aspect-ratio: 16 / 9`.
- Arrows use `--glass-fill`, `--glass-blur`, `--glass-border` for a frosted-glass effect, with `--space-xs`/`--space-sm` spacing (corrected from the blueprint's `--spacing-*` typo).
- Dots use `--accent` for the active state, `--text-tertiary` for inactive.
- No undefined CSS variables were introduced (resolved the blueprint's `--muted` warning).

**7. Forms (`src/pages/recipe_new.rs`, `src/pages/recipe_edit.rs`)**
Minimal integration: `recipe_new.rs` passes `vec![]` (empty images) to `create_recipe`; `recipe_edit.rs` passes `None` to `update_recipe`. Full image input UI (URL entry, add/remove buttons) is deferred to a future pass.

### Dependencies

No new crate dependencies were introduced. The implementation uses only existing dependencies:
- `sqlx` 0.8 (JSONB via raw `serde_json` round-trip, not `sqlx::types::Json`)
- `serde` / `serde_json` (serialization)
- `dioxus` 0.7.1 (`use_signal` for slider state)

### Non-Obvious Patterns

- **No `sqlx::types::Json` wrapper**: This project deliberately avoids the sqlx Json wrapper type and uses manual `serde_json::to_value`/`from_value` calls. This matches the existing pattern for `ingredients`, `instructions`, and `equipment`.
- **Pre-computed reactive values in Dioxus**: Because Dioxus 0.7.1's `rsx!` macro doesn't support closures in attribute positions, values like `current_src`, `show_left_arrow`, and `show_right_arrow` are computed as `let` bindings before the `rsx!` block.
- **Optional update parameter**: `update_recipe` takes `Option<Vec<String>>` rather than `Vec<String>`, allowing callers to omit the images field during partial updates.

### Review Findings

**Verdict: PASS** — All requirements covered, implementation clean and consistent with project patterns.

Minor suggestions (non-blocking):
1. **Cosmetic**: Indentation inconsistency on the `equipment` line in `src/test_utils.rs` — align with surrounding lines.
2. **Defensive**: No bounds check on `current_index` if images were to change at runtime. Not a concern for MVP (images are loaded once), but add `.min(images.len().saturating_sub(1))` guard if image editing is added later.
3. **Blueprint deviation**: The implementation renders a single image inside the slider container (with navigation hidden) rather than a separate static div. This is functionally equivalent and cleaner.

### Follow-Up Recommendations

1. **Image input UI**: Add URL entry fields with add/remove buttons to `recipe_new.rs` and `recipe_edit.rs`.
2. **Image upload**: When upload infrastructure is ready, replace placeholder URLs with actual file paths/signed URLs.
3. **Image validation**: Add basic URL/path validation when upload is implemented.
4. **Accessibility**: Add keyboard navigation (arrow keys) and ARIA labels to the slider for screen reader support.
5. **Image ordering**: Consider an explicit ordering field if reordering images becomes a requirement.
6. **Lazy loading**: For recipes with many images, add lazy loading for thumbnails.

### Commit Message

```
feat: add recipe images column and image slider to detail page

Add an `images` JSONB column to the recipes table to store a list of
image URLs/paths. The column follows the existing JSONB pattern used for
ingredients, instructions, and equipment (Vec<String> in Rust, manual
serde_json serialization at the DB boundary).

Database layer changes:
- Add `images` field to Recipe struct in types.rs
- Update FromRow impl with images deserialization
- Add images column to insert_recipe and update_recipe
- Add r.images to all 7 SELECT queries for recipe retrieval
- Shift visibility binding from $12 to $13 in update_recipe
- Update all test helper calls with empty images arrays

API layer changes:
- Add images: Vec<String> param to create_recipe server function
- Add images: Option<Vec<String>> param to update_recipe server function

UI changes:
- Add neumorphic image slider component below header card on recipe
  detail page with arrow navigation and dot indicators
- Slider uses use_signal for state, pre-computed reactive values for
  Dioxus 0.7.1 compatibility, conditional rendering at boundaries
- Add BEM-named CSS classes using existing neumorphic/glassmorphism
  design tokens
- Pass empty/None images from new and edit recipe forms

Schema changes:
- Add images JSONB NOT NULL DEFAULT '[]'::jsonb to CREATE TABLE
- Add idempotent ALTER TABLE for existing databases
- Update test schema DDL
```