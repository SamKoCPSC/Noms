# NOMS-008: Recipe CRUD

**Status:** 🔵 Ready  
**Phase:** Phase 1 (core content creation)  
**Depends on:** NOMS-004 (OAuth authentication), NOMS-005 (user profile)

## Overview

Implement the core recipe creation, viewing, editing, and deletion flows. Every recipe is a single row — edit overwrites the row. Versioning, drafts, and branching are deferred to NOMS-009.

This is the foundational content feature — without it, the application has no core value proposition.

## Context

The current codebase has authentication and user management working, but all recipe-related pages are empty placeholders:

| File | Current State |
|------|---------------|
| `src/pages/recipe_new.rs` | Empty form shell (title + description fields, no ingredients/steps, no save logic) |
| `src/pages/recipe_detail.rs` | Placeholder showing "Recipe content will appear here" |
| `src/pages/dashboard.rs` | Empty state with "No recipes yet" message |
| `src/db/mod.rs` | No recipe-related types or queries |
| `migrations/schema.sql` | No recipe tables exist yet |

## Acceptance Criteria

### AC1: Database schema for recipes

- [ ] Migration creates `recipes` table with owner, title, description, visibility, times, servings, ingredients (JSONB), steps (JSONB), timestamps
- [ ] Migration creates `recipe_tags` table for associating tags with recipes
- [ ] Proper indexes: `recipes(owner_id)`, `recipes(updated_at DESC)`
- [ ] Foreign key: `recipes.owner_id REFERENCES users(id) ON DELETE CASCADE`
- [ ] Foreign key: `recipe_tags.recipe_id REFERENCES recipes(id) ON DELETE CASCADE`
- [ ] Schema is additive-only (IF NOT EXISTS) and idempotent

### AC2: Create a recipe from scratch

- [ ] `/recipes/new` form collects: title (required), description (optional), ingredients (dynamic list), instructions (dynamic ordered list), prep time, cook time, servings
- [ ] Each ingredient row has: amount (optional), unit (optional), name (required), note (optional)
- [ ] Each instruction row has: step text (required), optional photo placeholder
- [ ] User can add/remove ingredient rows dynamically
- [ ] User can add/remove instruction rows dynamically
- [ ] User can reorder instruction rows (up/down buttons)
- [ ] Form validates required fields before submission
- [ ] Saving creates a new `recipes` row
- [ ] User is redirected to the new recipe's detail page on success
- [ ] Cancel button navigates back to dashboard

### AC3: View a recipe (detail page)

- [ ] `/recipes/:id` displays: title, description, prep/cook/total time, servings
- [ ] Ingredients rendered as a styled list with amounts and units
- [ ] Instructions rendered as numbered steps
- [ ] Tags displayed as chips below the title
- [ ] Author attribution: avatar + username + created date
- [ ] "Edit" and "Delete" buttons in header (only visible to recipe owner)
- [ ] Loading state while fetching recipe data
- [ ] Error state if recipe not found or user lacks permission
- [ ] Recipe is private by default (only owner can view)

### AC4: List user's recipes (dashboard)

- [ ] `/dashboard` shows paginated grid of user's recipes
- [ ] Each card shows: title, description snippet, creation date
- [ ] Clicking a card navigates to recipe detail page
- [ ] "New Recipe" button in header links to `/recipes/new`
- [ ] Empty state when user has no recipes (existing placeholder is sufficient)
- [ ] Recipes sorted by `updated_at DESC` (most recently modified first)

### AC5: Edit an existing recipe

- [ ] "Edit" button on detail page navigates to edit form (reuses create form with pre-populated data)
- [ ] Edit form loads the recipe's current data
- [ ] Saving overwrites the existing `recipes` row (no versioning yet — NOMS-009)
- [ ] Recipe's `updated_at` timestamp is refreshed on save
- [ ] User is redirected back to detail page on success

### AC6: Delete a recipe

- [ ] "Delete" button on detail page triggers confirmation dialog
- [ ] Confirmation: "Delete this recipe? This will permanently remove the recipe."
- [ ] Deleting removes the `recipes` row; `recipe_tags` cascade via foreign key
- [ ] User is redirected to dashboard after deletion
- [ ] Error handling: graceful message if deletion fails

### AC7: Recipe DB queries and types

- [ ] Rust types in `src/db/mod.rs`: `Recipe`, `RecipeTag`
- [ ] Query functions:
  - `insert_recipe()` — create new recipe
  - `get_recipe_by_id()` — fetch recipe by ID
  - `get_recipe_by_id_and_owner()` — ownership-gated lookup
  - `get_recipes_by_owner()` — list user's recipes (paginated)
  - `update_recipe()` — overwrite recipe row, refresh `updated_at`
  - `delete_recipe()` — delete recipe (tags cascade)
  - `insert_recipe_tags()` — upsert tags for a recipe
  - `get_recipe_tags()` — fetch tags for a recipe
- [ ] All queries guard by `owner_id` where appropriate
- [ ] Tests for each query function (using existing `test_utils` pattern)

## Technical Details

### Database Schema (new tables)

```sql
-- Recipe (single row, all data inline)
CREATE TABLE IF NOT EXISTS recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    prep_time_min INTEGER,
    cook_time_min INTEGER,
    total_time_min INTEGER,
    servings INTEGER,
    ingredients JSONB NOT NULL DEFAULT '[]'::jsonb,
    steps JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipes_owner_id ON recipes(owner_id);
CREATE INDEX IF NOT EXISTS idx_recipes_updated_at ON recipes(updated_at DESC);

-- Recipe tags (many-to-many, freeform text)
CREATE TABLE IF NOT EXISTS recipe_tags (
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (recipe_id, tag)
);
```

### Ingredient JSONB structure

Each element in the `ingredients` JSONB array:
```json
{
    "amount": "2",
    "unit": "cups",
    "name": "all-purpose flour",
    "note": "sifted"
}
```

### Instruction/Step JSONB structure

Each element in the `steps` JSONB array:
```json
{
    "text": "Preheat oven to 350°F (175°C).",
    "photo_url": null
}
```

### Server functions

| Function | Purpose |
|----------|---------|
| `create_recipe(title, description, prep_time, cook_time, total_time, servings, ingredients, steps, tags)` | Insert recipe row + tags |
| `get_recipe(recipe_id, user_id)` | Fetch recipe (ownership-gated) |
| `update_recipe(recipe_id, user_id, title, description, prep_time, cook_time, total_time, servings, ingredients, steps, tags)` | Overwrite recipe row + upsert tags |
| `delete_recipe(recipe_id, user_id)` | Delete recipe (ownership-gated, tags cascade) |
| `list_my_recipes(user_id, offset, limit)` | Paginated recipe list for dashboard |

### AuthContext changes

No changes needed. Recipe pages use the existing `current_user_id` from AuthContext for ownership checks.

### Route protection changes

- `/recipes/new` is already protected (in `PROTECTED_PATHS`)
- `/recipes/:id` should be protected (add to `PROTECTED_PATHS` in auth middleware)
- `/recipes/:id/edit` should be protected (add to `PROTECTED_PATHS` in auth middleware)
- Recipe detail page enforces ownership at the application layer (query by `recipe_id + owner_id`)

### Component changes

| Component | Change |
|-----------|--------|
| `RecipeNew` | Full form with dynamic ingredients/steps, wired to `create_recipe` server function |
| `RecipeDetail` | Fetches recipe via server function, renders all fields, shows edit/delete |
| `RecipeEdit` | New page, pre-populated `RecipeForm`, wired to `update_recipe` |
| `Dashboard` | Fetches user's recipes via `list_my_recipes`, renders card grid |
| New: `RecipeForm` | Shared form component used by both create and edit pages |
| New: `IngredientRow` | Single ingredient input row (amount, unit, name, note, remove button) |
| New: `StepRow` | Single instruction step input row (text, reorder buttons, remove button) |
| New: `RecipeCard` | Recipe preview card for dashboard grid |

## Out of Scope

- Recipe versioning and history (NOMS-009)
- Draft saving (NOMS-009)
- Recipe forking/branching (NOMS-009)
- Recipe images/hero photos (NOMS-011)
- Recipe import from URLs (Phase 2)
- Public recipe visibility (Phase 3)
- Recipe sharing via link (Phase 3)
- Recipe search and filtering (NOMS-012)
- Recipe scaling UI (Phase 4)
- Comments and likes (Phase 3)
- Collections/folders (NOMS-010)
- Nutritional information (Phase 6)
- Print/PDF export (Phase 6)

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | DB schema + queries | Migration applied, all 8 query functions working, tests pass |
| 2 | Server functions | All 5 server functions compile and work end-to-end |
| 3 | Create recipe form | `recipe_new.rs` fully functional, saves to DB, redirects to detail |
| 4 | Recipe detail view | `recipe_detail.rs` fetches and displays recipe data |
| 5 | Dashboard recipe list | Dashboard shows user's recipes in a card grid |
| 6 | Edit recipe | Edit form pre-populates, overwrites row, redirects back |
| 7 | Delete recipe | Confirmation dialog, deletes recipe, redirects to dashboard |

## Success Metrics

- User can sign in → create a recipe → see it on dashboard → view detail → edit it → delete it
- All 7 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- No unhandled error paths in server functions
