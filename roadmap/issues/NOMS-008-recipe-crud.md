# NOMS-008: Recipe CRUD + Visibility & Discovery

**Status:** 🔵 Ready  
**Phase:** Phase 1 (core content creation + discovery)  
**Depends on:** NOMS-004 (OAuth authentication), NOMS-005 (user profile)

## Overview

Implement the core recipe creation, viewing, editing, and deletion flows, plus a three-tier visibility model (private/unlisted/public) and community discovery features. Every recipe is a single row — edit overwrites the row. Versioning, drafts, and branching are deferred to NOMS-009.

This is the foundational content feature — without it, the application has no core value proposition. The visibility and discovery layer enables community sharing while respecting user privacy.

### Visibility Model

| Level | Who can view | Appears in |
|---|---|---|
| **Private** | Owner only | Owner's dashboard only |
| **Unlisted** | Anyone with the link | Nowhere (no public listings) |
| **Public** | Anyone (authenticated or not) | Explore page + owner's public profile |

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
- [ ] All mutation queries enforce `user_id` in WHERE clause (defense-in-depth)
- [ ] Tests for each query function (using existing `test_utils` pattern)
- [ ] Cross-user ownership tests: updating/deleting another user's recipe returns `RecipeNotFound`

### AC8: Recipe visibility (public/unlisted/private)

- [ ] `recipes.visibility` column: `VARCHAR(20)` with CHECK constraint (`'private'`, `'unlisted'`, `'public'`)
- [ ] Default visibility: `'private'`
- [ ] Create form includes visibility selector (radio/dropdown)
- [ ] Edit form includes visibility selector (pre-populated)
- [ ] `get_public_recipes_paginated()` — lists recipes where `visibility = 'public'`
- [ ] `get_recipe_by_id_public()` — fetches recipe without ownership check (for public/unlisted viewing)
- [ ] `get_user_public_recipes()` — lists user's public recipes for profile page
- [ ] `get_user_by_username()` — lookup user by username for profile routing
- [ ] Tests for visibility filtering and cross-user access

### AC9: Explore page (public recipe discovery)

- [ ] `/explore` route renders public recipe listing
- [ ] Paginated grid of public recipes (same card component as dashboard)
- [ ] Filter by tags (clickable tag chips)
- [ ] Search by title (text input, client-side or server-side)
- [ ] Recipe cards link to `/recipes/:id` detail page
- [ ] Empty state when no public recipes exist
- [ ] Non-authenticated users can browse (optional: require auth to view details)

### AC10: User profile page (discover other users)

- [ ] `/u/:username` route renders user's public profile
- [ ] Shows: avatar, display name, bio, join date
- [ ] Lists user's public recipes in a card grid (paginated)
- [ ] "View Profile" link on recipe detail pages navigates to owner's `/u/:username`
- [ ] 404 if username doesn't exist
- [ ] Only public recipes shown (unlisted/private hidden)

### AC11: Public recipe detail (view-only for non-owners)

- [ ] `/recipes/:id` resolves for public/unlisted recipes without ownership check
- [ ] Owner sees: full detail + "Edit" and "Delete" buttons
- [ ] Non-owner sees: full detail + "View Owner's Profile" link (no edit/delete)
- [ ] Private recipe accessed by non-owner: "Recipe not found" (no leakage that it exists)
- [ ] Unlisted recipe: only accessible via direct link, not in any listing
- [ ] Recipe detail shows owner attribution: avatar + username (linkable to `/u/:username`) + created date

## Technical Details

### Database Schema (new tables)

```sql
-- Recipe (single row, all data inline)
CREATE TABLE IF NOT EXISTS recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    visibility VARCHAR(20) NOT NULL DEFAULT 'private'
        CONSTRAINT valid_visibility CHECK (visibility IN ('private', 'unlisted', 'public')),
    prep_time_minutes INT,
    cook_time_minutes INT,
    servings INT,
    instructions TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipes_user_id ON recipes(user_id);
CREATE INDEX IF NOT EXISTS idx_recipes_created_at ON recipes(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_recipes_visibility_created ON recipes(visibility, created_at DESC)
    WHERE visibility = 'public';

-- Recipe tags (many-to-many, freeform text)
CREATE TABLE IF NOT EXISTS recipe_tags (
    recipe_id UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (recipe_id, tag)
);
```

### New queries for public access

```sql
-- Public recipes (for explore page)
SELECT * FROM recipes WHERE visibility = 'public' ORDER BY created_at DESC LIMIT $1 OFFSET $2;

-- User's public recipes (for profile page)
SELECT * FROM recipes WHERE user_id = $1 AND visibility = 'public' ORDER BY created_at DESC LIMIT $2 OFFSET $3;

-- Public recipe by ID (no ownership check)
SELECT * FROM recipes WHERE id = $1 AND visibility IN ('public', 'unlisted');

-- User by username (for profile routing)
SELECT * FROM users WHERE username = $1;
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

| Function | Purpose | Auth required? |
|----------|---------|----------------|
| `create_recipe(title, description, visibility, prep_time, cook_time, servings, instructions, tags)` | Insert recipe row + tags atomically | Yes |
| `get_recipe(recipe_id)` | Fetch recipe (ownership-gated for private) | Yes |
| `get_public_recipe(recipe_id)` | Fetch public/unlisted recipe (no ownership check) | No |
| `update_recipe(recipe_id, user_id, title, description, visibility, prep_time, cook_time, servings, instructions, tags)` | Overwrite recipe row + upsert tags (ownership in WHERE) | Yes |
| `delete_recipe(recipe_id, user_id)` | Delete recipe (ownership in WHERE clause, tags cascade) | Yes |
| `list_my_recipes(user_id, offset, limit)` | Paginated recipe list for dashboard | Yes |
| `get_public_recipes(offset, limit)` | Paginated public recipes for explore page | No |
| `get_user_public_recipes(username, offset, limit)` | User's public recipes for profile page | No |
| `get_user_profile(username)` | User info by username | No |
| `get_recipe_tags(recipe_id)` | Fetch tags for a recipe | Yes |

### AuthContext changes

No changes needed. Recipe pages use the existing `current_user_id` from AuthContext for ownership checks.

### Route protection changes

- `/recipes/new` is protected (auth required)
- `/recipes/:id/edit` is protected (auth required)
- `/recipes/:id` is **public** (no auth middleware) — ownership enforced at application layer:
  - Private recipes: only owner can view
  - Unlisted recipes: anyone with link can view
  - Public recipes: anyone can view
- `/explore` is **public** (no auth required)
- `/u/:username` is **public** (no auth required)

### Component changes

| Component | Change |
|-----------|--------|
| `RecipeNew` | Full form with visibility selector, wired to `create_recipe` server function |
| `RecipeDetail` | Fetches recipe via server function, renders all fields, conditional edit/delete for owners |
| `RecipeEdit` | Pre-populated form with visibility selector, wired to `update_recipe` |
| `Dashboard` | Fetches user's recipes via `list_my_recipes`, renders card grid |
| `Explore` | Public recipe listing with tag filters and search |
| `UserProfile` | User's public profile with recipe grid |
| New: `RecipeCard` | Recipe preview card for dashboard/explore/profile grids |
| New: `VisibilitySelector` | Radio/dropdown for private/unlisted/public |

## Out of Scope

- Recipe versioning and history (NOMS-009)
- Draft saving (NOMS-009)
- Recipe forking/branching (NOMS-009)
- Recipe images/hero photos (NOMS-011)
- Recipe import from URLs (Phase 2)
- Advanced recipe search and full-text filtering (NOMS-012)
- Recipe scaling UI (Phase 4)
- Comments and likes (Phase 3)
- Collections/folders (NOMS-010)
- Nutritional information (Phase 6)
- Print/PDF export (Phase 6)
- Anonymous/guest access (auth required for all recipe views)

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | DB schema + queries | Migration applied, all query functions working, tests pass |
| 2 | Server functions (owner CRUD) | All owner CRUD server functions compile and work end-to-end |
| 3 | Create recipe form | `recipe_new.rs` fully functional with visibility selector, saves to DB |
| 4 | Recipe detail view | `recipe_detail.rs` fetches and displays recipe data, conditional owner controls |
| 5 | Dashboard recipe list | Dashboard shows user's recipes in a card grid |
| 6 | Edit recipe | Edit form pre-populates with visibility, overwrites row, redirects back |
| 7 | Delete recipe | Confirmation dialog, deletes recipe, redirects to dashboard |
| 8 | DB + API for public access | Public queries, user lookup, visibility filtering, tests |
| 9 | Explore page | `/explore` renders public recipe grid with tag filters |
| 10 | User profile page | `/u/:username` shows user's public recipes and profile info |
| 11 | Public recipe detail | Non-owner viewing, view-only mode, owner attribution |

## Success Metrics

- User can sign in → create a recipe → see it on dashboard → view detail → edit it → delete it
- User can set recipe to public → another user can find it on Explore page → view it (read-only)
- User can visit `/u/:username` → see that user's public recipes
- Owner sees edit/delete on their recipes; non-owner sees "View Profile" link instead
- Private recipes are invisible to non-owners; unlisted recipes only accessible via direct link
- All 11 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- No unhandled error paths in server functions
- Defense-in-depth: DB layer enforces ownership on mutations even if API layer forgets
