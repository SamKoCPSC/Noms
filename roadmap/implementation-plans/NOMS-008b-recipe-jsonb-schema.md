# NOMS-008b: Recipe JSONB Schema Refactor — Implementation Plan

**Parent Issue:** [NOMS-008-recipe-crud.md](../issues/NOMS-008-recipe-crud.md)
**Created:** 2026-06-14
**Approach:** Bottom-up by dependency, 6 incremental checkpoints

**Goal:** Migrate recipe data from serialized TEXT columns to structured JSONB columns for `ingredients`, `instructions`, and `equipment`. Eliminates custom serialization/parsing logic and enables future querying, filtering, and structured features.

---

## Current State

| Column | Type | Format |
|--------|------|--------|
| `instructions` | `TEXT` | Serialized: `INGREDIENTS:\n- 2 cup flour\n\nSTEPS:\n1. Mix...\n` |
| `equipment` | `TEXT` | Free text: comma or newline separated |

**Problems:**
- Custom `serialize_instructions()` / `parse_instructions()` duplicated across 3 files
- Ingredients and steps tangled in one column
- Equipment is unstructured text
- Scaler must parse text to extract amounts
- No ability to query/filter by ingredients or equipment

## Target State

| Column | Type | Format |
|--------|------|--------|
| `ingredients` | `JSONB` | `[{amount: "2", unit: "cup", name: "flour"}]` |
| `instructions` | `JSONB` | `[{text: "Prepare dough", sub_steps: [{text: "Mix dry"}, {text: "Add wet"}]}]` |
| `equipment` | `JSONB` | `[{name: "Large mixing bowl"}]` |

**Instructions nesting:** Unlimited depth via recursive `sub_steps` array. Enables hierarchical step structures:

```json
[
  {
    "text": "Prepare the dough",
    "sub_steps": [
      {"text": "Mix dry ingredients", "sub_steps": []},
      {
        "text": "Add wet ingredients",
        "sub_steps": [
          {"text": "Whisk eggs separately", "sub_steps": []},
          {"text": "Pour into dry mix", "sub_steps": []}
        ]
      }
    ]
  }
]
```

---

## Checkpoint 1: Migration + typed structs

**Files:** `migrations/002_recipe_jsonb.sql` (new), `src/types.rs`, `src/db/mod.rs`

**Migration** (`002_recipe_jsonb.sql`):
- Drop `instructions TEXT` column from `recipes`
- Drop `equipment TEXT` column from `recipes`
- Add `ingredients JSONB`
- Add `instructions JSONB`
- Add `equipment JSONB`

No backfill — no production data to preserve.

**Typed structs** (`src/types.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeIngredient {
    pub amount: String,
    pub unit: String,
    pub name: String,
}

/// Recursive step node — supports unlimited nesting depth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeStep {
    pub text: String,
    pub sub_steps: Vec<RecipeStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeEquipment {
    pub name: String,
}
```

**`Recipe` struct update** (`src/types.rs`):
- Replace `instructions: Option<String>` with:
  - `ingredients: Vec<RecipeIngredient>`
  - `instructions: Vec<RecipeStep>`
  - `equipment: Vec<RecipeEquipment>`

**`RecipeListResponse` and `UserProfile`**: No changes (don't include ingredients/instructions/equipment).

**DB layer** (`src/db/mod.rs`):
- Update `Recipe` struct to use typed fields
- Update `from_row` impl: parse JSONB columns via `serde_json::from_value`
- Update `insert_recipe()`: accept typed vectors, serialize to JSONB
- Update `update_recipe()`: accept typed vectors, serialize to JSONB
- Update all SELECT queries to include new columns
- Update all tests to use new schema

**Verify:**
- `cargo test --features server` — all DB tests pass
- `cargo check --features server` — zero errors
- Migration applies cleanly on fresh DB via `just migrate`

**Risk:** Low. Straightforward schema change, no data to migrate.

---

## Checkpoint 2: Server functions (API layer)

**File:** `src/api/recipe.rs`

**`create_recipe` signature change:**
```rust
// Before:
pub async fn create_recipe(
    title: String,
    description: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    instructions: Option<String>,
    tags: String,
    equipment: Option<String>,
    visibility: String,
) -> Result<Recipe, ServerFnError>

// After:
pub async fn create_recipe(
    title: String,
    description: Option<String>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
    ingredients: Vec<RecipeIngredient>,
    instructions: Vec<RecipeStep>,
    equipment: Vec<RecipeEquipment>,
    tags: String,
    visibility: String,
) -> Result<Recipe, ServerFnError>
```

**`update_recipe` signature change:** Same as above (replace `instructions: Option<String>` and `equipment: Option<String>` with typed vectors).

**`get_recipe` return type:** Already returns `Recipe` — no change needed (type update propagates automatically).

**Verify:**
- `cargo check --features server` — server functions compile
- `cargo check --target wasm32-unknown-unknown` — client-side compiles
- Server function endpoints work with new payload structure

**Risk:** Low. Type changes propagate through serde automatically.

---

## Checkpoint 3: Create recipe form

**File:** `src/pages/recipe_new.rs`

**Remove:**
- `serialize_instructions()` function
- `IngredientDraft` and `StepDraft` local structs (replace with `RecipeIngredient` and `RecipeStep` from types)

**State changes:**
```rust
// Before:
let mut ingredients = use_signal(Vec::<IngredientDraft>::new);
let mut steps = use_signal(Vec::<StepDraft>::new);
let mut equipment = use_signal(String::new);

// After:
let mut ingredients = use_signal(Vec::<RecipeIngredient>::new);
let mut steps = use_signal(Vec::<RecipeStep>::new);
let mut equipment = use_signal(Vec::<RecipeEquipment>::new);
```

**Step input changes (nested instructions):**
- Each step row: text input + controls (remove, indent, unindent)
- "Indent" button: converts step into sub-step of previous step
- "Unindent" button: promotes sub-step back to parent level
- Visual indentation: left padding increases with nesting depth (e.g., `padding-left: ${depth * 24}px`)
- "+ Add Step" button: adds step at root level
- "+ Add Sub-step" button (on hover): adds sub-step to specific step
- Steps render as a flat list internally but track nesting via the recursive `sub_steps` structure
- UI builds flat list from tree for editing, flattens on render; rebuilds tree on submit

**Equipment input change:**
- Replace single textarea with dynamic list (add/remove rows)
- Each row: single text input for equipment name + remove button
- "+ Add Equipment" button

**Tree building helper:**
- `fn build_step_tree(flat_steps: Vec<FlatStep>) -> Vec<RecipeStep>` — converts flat list with depth indices into recursive tree
- `fn flatten_steps(tree: &[RecipeStep]) -> Vec<FlatStep>` — reverses the operation for editing

**Submit handler:**
- Remove `serialize_instructions()` call
- Pass typed vectors directly to `create_recipe()`:
  ```rust
  create_recipe(
      title, description, prep_time, cook_time, servings,
      ingredients, instructions, equipment,  // typed vectors
      tags, visibility
  ).await
  ```

**Verify:**
- Fill out form with ingredients, steps, and equipment → recipe saved correctly
- Indent/unindent steps → nesting structure correct in DB
- Deep nesting (3+ levels) works correctly
- Equipment stored as JSONB array in DB
- `cargo clippy --target wasm32-unknown-unknown` — zero warnings

**Risk:** Medium. Nested step UI (indent/unindent, tree building) is the most complex part of this refactor.

---

## Checkpoint 4: Edit recipe form

**File:** `src/pages/recipe_edit.rs`

**Remove:**
- `parse_instructions()` function
- `serialize_instructions()` function
- `IngredientDraft` and `StepDraft` local structs

**Load recipe data:**
```rust
// Before:
let parsed = parse_instructions(&recipe.instructions);
ingredients.set(parsed.ingredients);
steps.set(parsed.steps);
equipment.set(recipe.equipment.unwrap_or_default());

// After:
ingredients.set(recipe.ingredients);
steps.set(recipe.instructions);  // instructions are steps
equipment.set(recipe.equipment);
```

**Submit handler:**
- Pass typed vectors directly to `update_recipe()`

**Verify:**
- Edit existing recipe → all fields pre-populated correctly
- Modify ingredients, steps, or equipment → saves correctly
- `cargo clippy --target wasm32-unknown-unknown` — zero warnings

**Risk:** Low. Simplifies existing code by removing parse/serialize.

---

## Checkpoint 5: Recipe detail + scaler

**Files:** `src/pages/recipe_detail.rs`, `src/utils/recipe_scaler.rs`

**`recipe_detail.rs` changes:**
- Remove `parse_instructions()` function and `ParsedInstructions` struct
- Remove `render_equipment()` helper (replace with direct iteration)

**Render changes:**
```rust
// Before:
let parsed = recipe.instructions.as_ref().map(parse_instructions);
// parsed.ingredients, parsed.steps

// After:
// recipe.ingredients, recipe.instructions, recipe.equipment — direct access
```

**Instructions display (recursive rendering):**
- New `RenderSteps` component that recursively renders `Vec<RecipeStep>`
- Each step: numbered text + recursively renders `sub_steps` if non-empty
- Numbering: `1.`, `1a.`, `1a-i.`, etc. (letter-based sub-numbering)
- Visual indentation: nested steps have increased left padding
- Skip entire instructions section if empty

**Equipment display:**
- Iterate `recipe.equipment` directly, render as bullet list
- Skip section if empty

**`recipe_scaler.rs` changes:**
- Update `ScaleCalculator::new()` to accept `Vec<RecipeIngredient>` instead of parsed text
- `parse_amount()` still needed for extracting numeric values from ingredient amounts
- Remove any text parsing logic — work directly with struct fields

**Verify:**
- Recipe detail renders ingredients, steps, and equipment correctly
- Nested steps render with proper numbering (`1.`, `1a.`, `1a-i.`)
- Nested steps render with proper visual indentation
- Scaler works with typed ingredient data
- Empty equipment section doesn't render
- `cargo clippy --target wasm32-unknown-unknown` — zero warnings

**Risk:** Low-Medium. Recursive rendering is straightforward but numbering scheme needs care.

---

## Checkpoint 6: Test schema + cleanup

**Files:** `src/test_utils.rs`, `assets/main.css`

**`test_utils.rs`:**
- Update test DDL to use new JSONB columns
- Update all test insert statements to use JSONB format
- Verify all 222 tests pass

**`assets/main.css`:**
- Update any equipment-related styling if needed (bullet list → structured list)
- Add nested step styling: indentation, sub-numbering colors, connector lines

**Final verification:**
- `cargo test --features server` — all tests pass
- `cargo check --features server` — zero errors
- `cargo check --target wasm32-unknown-unknown` — zero errors
- `cargo clippy --target wasm32-unknown-unknown` — zero warnings
- Manual test: create → view → edit → delete full flow
- Manual test: scaler works with new data format
- Manual test: equipment displays correctly on detail page

**Risk:** Low. Final cleanup pass.

---

## Dependencies Summary

| Crate | Feature | Purpose |
|-------|---------|---------|
| `serde_json` | both | JSONB serialization/deserialization |
| `serde` | both | Struct serialization for server functions |
| `sqlx` | `server` | JSONB column support (built-in) |

No new dependencies — all crates already in `Cargo.toml`.

---

## File Structure (Changes)

```
migrations/
└── 002_recipe_jsonb.sql          # NEW: drop TEXT cols, add JSONB cols

src/
├── api/
│   └── recipe.rs                 # UPDATED: typed struct params
├── db/
│   └── mod.rs                    # UPDATED: JSONB queries, typed from_row
├── pages/
│   ├── recipe_detail.rs          # UPDATED: remove parse, direct iteration
│   ├── recipe_edit.rs            # UPDATED: remove parse/serialize
│   └── recipe_new.rs             # UPDATED: remove serialize, typed state
├── test_utils.rs                 # UPDATED: JSONB test schema
├── types.rs                      # UPDATED: new structs, Recipe fields
└── utils/
    └── recipe_scaler.rs          # UPDATED: typed ingredient input
```

---

## Rollback Plan

If any checkpoint reveals issues:
- Revert migration: `ALTER TABLE recipes ADD COLUMN instructions TEXT, ADD COLUMN equipment TEXT; DROP COLUMN ingredients, instructions, equipment;`
- Code changes are isolated per checkpoint — each is independently reversible via git
- Existing tests cover all query paths

---

## Notes

- **No data migration needed** — development database only, no production data
- **Breaking change** — any existing recipes in DB will be lost (acceptable for dev)
- **Future extensibility** — JSONB arrays can evolve: add fields to structs without schema changes
- **GIN indexes** — can be added later if ingredient/equipment search is needed
- **Recursive RecipeStep** — unlimited nesting depth via `sub_steps: Vec<RecipeStep>`. JSONB handles arbitrary depth naturally. Form UI uses flat list with depth indices, converts to tree on submit.
