# NOMS-012: Recipe Search

**Status:** ⚪ Backlog  
**Phase:** Phase 2 (discovery and organization)  
**Depends on:** NOMS-008 (Recipe CRUD)

## Overview

Add advanced full-text search across all recipes using the `pg_search` extension (ParadeDB) with BM25 ranking, ngram-based partial matching, field boosting, faceted filtering, sorting, and pagination. Search covers title, description, ingredients, and steps. Results are ranked by relevance and displayed on a dedicated search page with a filter panel.

Search returns:
- **All of the user's own recipes** (regardless of `is_public` status)
- **Public recipes from other users** (`is_public = true`)

Private recipes from other users are never returned. Results show owner attribution so users can distinguish their own recipes from public ones.

This is a read-only feature that layers on top of the existing `recipes` and `recipe_tags` tables. The `pg_search` extension is already available in the database. No changes to create/edit/delete flows are required — the BM25 index maintains itself on INSERT/UPDATE/DELETE.

## Context

After NOMS-008 (and NOMS-010), users can create, edit, and version recipes, but have no way to find a recipe by name or content beyond scrolling their dashboard. As recipe collections grow — both personal and public — search becomes essential for discovering recipes across the community.

`pg_search` (ParadeDB) provides Elasticsearch-like search inside PostgreSQL:

| Feature | pg_search | PostgreSQL native tsvector |
|---------|-----------|---------------------------|
| BM25 relevance ranking | Yes | Basic ts_rank only |
| ngram partial matching | Yes (`pdb.ngram`) | No (would need pg_trgm) |
| Field boosting | Yes (`pdb.boost`) | No |
| JSONB sub-field indexing | Automatic | Manual extraction needed |
| Fuzzy/typo tolerance | Yes | No |
| Phrase search | Yes | Limited |
| Self-maintaining index | Yes (on DML) | Yes (GIN) |

## Acceptance Criteria

### AC1: BM25 search index

- [ ] Migration creates BM25 index on `recipes` table covering: `id` (key_field), `title`, `description`, `ingredients` (JSONB), `steps` (JSONB)
- [ ] `title` and `description` use ngram tokenizer (`pdb.ngram(2, 15)`) for partial matching — typing "past" matches "pasta carbonara"
- [ ] `ingredients` and `steps` JSONB fields indexed with default unicode tokenizer (auto-indexes all text sub-fields: `ingredients[].name`, `ingredients[].note`, `steps[].text`)
- [ ] Index uses `key_field='id'` (primary key)
- [ ] Existing rows are automatically indexed by `CREATE INDEX` — no backfill needed
- [ ] Index is maintained automatically on INSERT/UPDATE/DELETE

### AC2: Search query with ranking, boosting, and filters

- [ ] `search_recipes(user_id, &SearchFilters)` query function in `src/db/mod.rs` accepting a `SearchFilters` struct
- [ ] `SearchFilters` struct: `{ query_text, offset, limit, sort, tags, time_range, servings_range, visibility, only_mine }`
- [ ] Uses `|||` operator for multi-field match across title, description, ingredients, steps
- [ ] Title matches boosted 3x, description 2x, ingredients/steps at 1x (via `pdb.boost`)
- [ ] Dynamic SQL query builder composes WHERE clauses from active filters (no hardcoded query permutations)
- [ ] Visibility filter: returns user's own recipes (any visibility) + public recipes from other users (`is_public = true`). Private recipes from other users are never returned.
- [ ] Tag filter: `WHERE recipe_tags.tag = ANY($tags)` — AND semantics (recipe must have ALL selected tags)
- [ ] Time range filter: `WHERE total_time_min BETWEEN $min AND $max`
- [ ] Servings range filter: `WHERE servings BETWEEN $min AND $max`
- [ ] Handles empty query gracefully (returns all visible recipes, fallback to `updated_at DESC` ordering)
- [ ] Handles special characters, stopwords, and partial input safely (ngram handles short/partial queries naturally)
- [ ] Returns `Recipe` rows (same type as `get_recipes_by_owner`)
- [ ] Separate `count_search_results(user_id, &SearchFilters)` function for pagination total

### AC3: Search page

- [ ] `/search` route with search input field at top, filter panel, sort bar, results grid, and pagination
- [ ] Input triggers search on Enter key press and/or after 300ms debounce on typing
- [ ] Results displayed as recipe cards (reuse `RecipeCard` from dashboard)
- [ ] Each result shows: title, description snippet, updated date, owner attribution (for non-owned recipes: "by @username" or creator name)
- [ ] Result count displayed: "Found N recipes"
- [ ] Empty state: "No recipes match your search" when query returns zero results, with suggestions to broaden filters
- [ ] Initial state (no query entered): shows hint text "Search all recipes..."
- [ ] Loading state during search (spinner or skeleton)
- [ ] Error state if search fails

### AC4: Search integration

- [ ] Search icon/link in app header or sidebar navigation
- [ ] Dashboard "search" button or input field that navigates to `/search`
- [ ] Search query preserved in URL query string (`/search?q=pasta`) for shareability and browser back/forward
- [ ] All filter state preserved in URL: `/search?q=pasta&sort=newest&tags=quick%2Cvegetarian&time=0-30&servings=1-4&mine=true`
- [ ] Search input and filters pre-populate from URL query parameters on page load
- [ ] Changing any filter updates the URL (history.pushState) for browser back/forward

### AC5: Search DB types and tests

- [ ] Query function `search_recipes()` tested with known data
- [ ] Test: exact title match ranks highest
- [ ] Test: partial match (ngram) — "past" finds "pasta carbonara"
- [ ] Test: ingredient name match returns recipe
- [ ] Test: step text match returns recipe
- [ ] Test: multi-word query returns union of matches
- [ ] Test: empty query returns all recipes ordered by updated_at
- [ ] Test: visibility guard — user sees own private recipes but not other users' private recipes
- [ ] Test: public recipes from other users appear in results
- [ ] Test: owner attribution shown for non-owned recipes
- [ ] Test: single character query works (ngram min=2, so 1 char returns nothing — acceptable)

### AC6: Autocomplete

- [ ] `search_recipes_autocomplete(user_id, query_text, limit)` query function in `src/db/mod.rs` — returns only `{id, title}`
- [ ] Matches against `title` field only (ngram partial matching inherited from BM25 index)
- [ ] Limited to 5-10 suggestions, ordered by `pdb.score(id) DESC`
- [ ] Same visibility filter: own recipes (any) + public recipes from others
- [ ] Autocomplete dropdown below search input, appears after 2+ characters typed
- [ ] Keyboard navigable (↑↓ arrows, Enter to select, Escape to dismiss)
- [ ] Clicking a suggestion navigates to that recipe's detail page
- [ ] Debounced at 200ms to avoid excessive requests while typing

### AC7: Autocomplete tests

- [ ] Test: "past" suggests "Pasta Carbonara" (ngram partial match on title)
- [ ] Test: suggestions limited to configured max (e.g., 8)
- [ ] Test: private recipes from other users never appear in suggestions
- [ ] Test: own private recipes appear in suggestions
- [ ] Test: empty or 1-character query returns no suggestions

### AC8: Filters and facets

- [ ] **Filter panel** below search input with collapsible sections:
  - **Visibility**: toggle buttons — "All" (default), "Mine only", "Published", "Drafts"
  - **Time**: bucket buttons — "< 30 min", "30–60 min", "1–2 hr", "2+ hr" (maps to `total_time_min`)
  - **Servings**: bucket buttons — "1–2", "3–4", "5–8", "8+"
  - **Tags**: checklist of tags present in current result set, with counts (e.g., "quick (12)")
- [ ] Active filters displayed as removable chips/pills above results grid
- [ ] "Clear all" button resets all filters
- [ ] Tag facet queries `recipe_tags` joined with current filtered results to show only relevant tags with counts
- [ ] Filter panel is responsive: collapses to horizontal scrollable bar on narrow screens

### AC9: Sorting

- [ ] Sort dropdown in results bar: "Relevance" (default when query present), "Newest", "Oldest", "Name A→Z", "Name Z→A", "Quickest", "Longest"
- [ ] Sort maps to ORDER BY:
  - Relevance: `pdb.score(id) DESC`
  - Newest: `updated_at DESC`
  - Oldest: `updated_at ASC`
  - Name A→Z: `title COLLATE "C" ASC`
  - Name Z→A: `title COLLATE "C" DESC`
  - Quickest: `total_time_min ASC NULLS LAST`
  - Longest: `total_time_min DESC NULLS FIRST`
- [ ] Sort preference persists in URL: `&sort=newest`

### AC10: Pagination

- [ ] Page size: 12 recipes per page (configurable constant)
- [ ] Pagination controls at bottom of results: page numbers with ellipsis for large result sets (e.g., "1 2 3 ... 24 25")
- [ ] "Previous" / "Next" buttons, disabled at boundaries
- [ ] Current page tracked in URL: `&page=3`
- [ ] Result count shows context: "121–132 of 847 recipes"
- [ ] Navigating to a page scrolls results into view

### AC11: Filter and sort tests

- [ ] Test: tag filter returns only recipes with ALL selected tags (AND semantics)
- [ ] Test: time range filter correctly bounds `total_time_min`
- [ ] Test: servings range filter correctly bounds `servings`
- [ ] Test: visibility filter "Mine only" excludes other users' public recipes
- [ ] Test: visibility filter "Drafts" only returns `is_draft = true`
- [ ] Test: combined filters (tags + time + visibility) work together
- [ ] Test: sort by relevance ranks BM25 score correctly
- [ ] Test: sort by newest orders by `updated_at DESC`
- [ ] Test: pagination offset/limit produces correct page boundaries
- [ ] Test: count query matches actual result count for given filters
- [ ] Test: NULL time/servings values handled gracefully (excluded from time/servings ranges)

## Technical Details

### Database Schema (index creation)

No table alterations needed. Two indexes: the BM25 search index on `recipes`, and a supporting index on `recipe_tags` for facet queries:

```sql
-- BM25 search index with ngram partial matching on title/description,
-- auto-indexed JSONB sub-fields on ingredients/steps
CREATE INDEX IF NOT EXISTS idx_recipes_search
ON recipes USING bm25 (
    id,
    (title::pdb.ngram(2, 15)),
    (description::pdb.ngram(2, 15)),
    ingredients,
    steps
)
WITH (key_field = 'id');

-- Supporting index for tag facet queries (count tags within filtered results)
CREATE INDEX IF NOT EXISTS idx_recipe_tags_tag ON recipe_tags(tag);
```

**Tokenizer choices:**

| Field | Tokenizer | Rationale |
|-------|-----------|-----------|
| `title` | `pdb.ngram(2, 15)` | Partial matching: "garb" finds "garbanzo beans in pasta". Max 15 avoids excessive token generation on long titles. |
| `description` | `pdb.ngram(2, 15)` | Same as title — users search descriptions with partial terms |
| `ingredients` (JSONB) | Default (unicode_words) | Auto-indexes `ingredients[].name`, `ingredients[].note`, etc. Full-word matching is appropriate for ingredient names. |
| `steps` (JSONB) | Default (unicode_words) | Auto-indexes `steps[].text`. Full-word matching for instruction text. |

**Why ngram(2, 15) for title/description:**
- Min 2: single-character queries are noise; 2+ characters give meaningful partial matches
- Max 15: covers most recipe name lengths; longer tokens add little value and bloat the index
- Typing "carbon" matches "carbonara", "slow cook" matches "slow cooker chicken"
- Trade-off: ngram indices are larger than word-tokenized indices, but recipe text is small

### Search query (SQL) — dynamic query builder

The search function builds SQL dynamically from a `SearchFilters` struct. Only non-empty filter fields produce WHERE clauses. Parameterized queries prevent SQL injection.

**Base query template:**

```sql
SELECT r.id, r.owner_id, r.title, r.description, r.is_public,
       r.prep_time_min, r.cook_time_min, r.total_time_min, r.servings,
       r.ingredients, r.steps, r.created_at, r.updated_at, r.is_draft,
       pdb.score(r.id) AS relevance
FROM recipes r
{JOIN_TAGS}
WHERE {VISIBILITY}
  {AND_SEARCH}
  {AND_TAGS}
  {AND_TIME}
  {AND_SERVINGS}
  {AND_DRAFT}
ORDER BY {SORT}
LIMIT $limit OFFSET $offset;
```

**Clause composition (Rust-side):**

| Filter | SQL Clause | Example |
|--------|-----------|---------|
| Visibility (default) | `(r.owner_id = $user_id OR r.is_public = true)` | Always present |
| Visibility: "Mine only" | `r.owner_id = $user_id` | Replaces default |
| Visibility: "Published" | adds `AND r.is_draft = false` | |
| Visibility: "Drafts" | adds `AND r.is_draft = true` | |
| Search query | `AND (r.title \|\|\| $q::pdb.boost(3.0) OR ...)` | Skipped if query empty |
| Tags | `JOIN recipe_tags rt ON rt.recipe_id = r.id AND rt.tag = ANY($tags)` | Skipped if no tags |
| Time range | `AND r.total_time_min BETWEEN $time_min AND $time_max` | Skipped if no range |
| Servings range | `AND r.servings BETWEEN $serv_min AND $serv_max` | Skipped if no range |

**Sort clause mapping:**

| Sort | ORDER BY |
|------|----------|
| Relevance (default with query) | `pdb.score(r.id) DESC` |
| Newest | `r.updated_at DESC` |
| Oldest | `r.updated_at ASC` |
| Name A→Z | `r.title COLLATE "C" ASC` |
| Name Z→A | `r.title COLLATE "C" DESC` |
| Quickest | `r.total_time_min ASC NULLS LAST` |
| Longest | `r.total_time_min DESC NULLS FIRST` |

**Fallback for empty query:** when `query_text` is empty, the search BM25 clause is omitted and sort defaults to `updated_at DESC`.

**Count query:** same WHERE clauses, `SELECT COUNT(r.id)` for pagination total.

**Tag facet query:** returns distinct tags with counts from current filtered result set:

```sql
SELECT rt.tag, COUNT(*) AS tag_count
FROM recipes r
JOIN recipe_tags rt ON rt.recipe_id = r.id
WHERE {same visibility + search + time + servings filters}
GROUP BY rt.tag
ORDER BY tag_count DESC, rt.tag ASC
LIMIT 50;
```

### Query builder implementation (Rust)

```rust
pub struct SearchFilters {
    pub query_text: String,      // Full-text search query
    pub offset: i64,
    pub limit: i64,
    pub sort: SearchSort,        // Enum: Relevance, Newest, Oldest, NameAsc, NameDesc, Quickest, Longest
    pub tags: Vec<String>,       // Filter by tags (AND semantics)
    pub time_range: Option<TimeRange>,  // { min: i32, max: i32 } minutes
    pub servings_range: Option<ServingsRange>, // { min: i32, max: i32 }
    pub visibility: SearchVisibility, // Enum: All, MineOnly, Published, Drafts
}

#[derive(Clone, Copy)]
pub enum SearchSort { Relevance, Newest, Oldest, NameAsc, NameDesc, Quickest, Longest }

#[derive(Clone, Copy)]
pub enum SearchVisibility { All, MineOnly, Published, Drafts }

#[derive(Clone, Copy)]
pub struct TimeRange { pub min: i32, pub max: i32 }

#[derive(Clone, Copy)]
pub struct ServingsRange { pub min: i32, pub max: i32 }
```

The query builder assembles the SQL string and collects parameters in order, then executes via `sqlx::query_as_with!(Recipe, bindings)`. This avoids hardcoding every filter combination while keeping queries parameterized and safe.

### Autocomplete query (SQL)

```sql
-- Title-only autocomplete, narrow result set (id + title only)
-- Same visibility filter: own recipes + public recipes from others
SELECT r.id, r.title
FROM recipes r
WHERE (r.owner_id = $1 OR r.is_public = true)
  AND r.title ||| ($2::text)::pdb.boost(3.0)
ORDER BY pdb.score(r.id) DESC
LIMIT $3;
```

**Parameters:** `$1` = user_id (Uuid), `$2` = query_text (String), `$3` = limit (i64, default 8)

Only queries the `title` field — fast, focused suggestions. Uses the same BM25 index, so no additional index needed.

### Autocomplete query function (Rust)

```rust
#[derive(sqlx::FromRow)]
pub struct RecipeSuggestion {
    pub id: Uuid,
    pub title: String,
}

pub async fn search_recipes_autocomplete(
    pool: &PgPool,
    user_id: Uuid,
    query_text: String,
    limit: i64,
) -> Result<Vec<RecipeSuggestion>, DbError>
```

Returns early if `query_text.len() < 2` (ngram min = 2, so fewer characters produce no matches anyway).

### Query function signatures (Rust)

```rust
pub async fn search_recipes(
    pool: &PgPool,
    user_id: Uuid,
    filters: &SearchFilters,
) -> Result<Vec<Recipe>, DbError>

pub async fn count_search_results(
    pool: &PgPool,
    user_id: Uuid,
    filters: &SearchFilters,
) -> Result<i64, DbError>

pub async fn get_search_facets(
    pool: &PgPool,
    user_id: Uuid,
    filters: &SearchFilters,
) -> Result<Vec<FacetTag>, DbError>
```

Where `FacetTag { tag: String, count: i64 }`.

The `search_recipes` function uses a dynamic query builder that composes SQL from the `SearchFilters` struct. Empty filters produce the simplest query (all visible recipes, sorted by recency).

### Server function

| Function | Purpose |
|----------|---------|
| `search_recipes(user_id, filters)` | Search all visible recipes with filters, sorting, and pagination |
| `count_search_results(user_id, filters)` | Count matching recipes for pagination total |
| `get_search_facets(user_id, filters)` | Get tag facet counts from current filtered result set |
| `autocomplete_recipes(user_id, query, limit)` | Return up to `limit` title suggestions for typeahead dropdown |

### URL design

- `/search` — search page, no query, no filters
- `/search?q=pasta` — search page with pre-filled query
- `/search?q=pasta&sort=newest&tags=quick%2Cvegetarian&time=0-30&servings=1-4&vis=mine&page=2`
- All parameters optional; omitting a parameter means "no filter applied"
- Parameter encoding:
  - `q` — search query text
  - `sort` — sort key: `relevance`, `newest`, `oldest`, `name-asc`, `name-desc`, `quickest`, `longest`
  - `tags` — comma-separated tag list: `quick,vegetarian`
  - `time` — time range bucket: `0-30`, `30-60`, `60-120`, `120-9999`
  - `servings` — servings range bucket: `1-2`, `3-4`, `5-8`, `8-999`
  - `vis` — visibility: `all` (default), `mine`, `published`, `drafts`
  - `page` — page number (1-indexed)

### Component changes

| Component | Change |
|-----------|--------|
| New: `SearchPage` | Search input, filter panel, sort bar, results grid, pagination, owner attribution |
| New: `SearchInput` | Reusable search bar with debounce, Enter key handler, URL sync, autocomplete dropdown |
| New: `AutocompleteDropdown` | Keyboard-navigable suggestion list (↑↓, Enter, Escape), renders recipe titles, click navigates to detail |
| New: `FilterPanel` | Collapsible sections: visibility toggles, time buckets, servings buckets, tag checklist with counts. Active filters shown as removable chips. "Clear all" button. |
| New: `SortDropdown` | Sort selector: relevance, newest, oldest, name A-Z/Z-A, quickest, longest |
| New: `Pagination` | Page numbers with ellipsis, prev/next buttons, "X–Y of Z" context display |
| `Header` / `Sidebar` | Add search icon/link |
| `Dashboard` | Optional: add search input that navigates to `/search` |
| `RecipeCard` | Show owner attribution ("by @username") when `owner_id` differs from current user |

### AuthContext changes

No changes. Search page is protected (logged-in users only), results gated by `owner_id`.

### Route protection changes

- `/search` added to `PROTECTED_PATHS`

### pg_search configuration

The extension is already installed. Two runtime settings worth noting:

```sql
-- For single-row inserts/updates (default recipe CRUD), use single-threaded indexing
SET paradedb.statement_parallelism = 1;
```

This can be set at the connection level or in `postgresql.conf`. Default is `0` (auto-detect), which is fine for most workloads. Setting to `1` avoids unnecessary threading overhead on single-row DML.

### Index maintenance

- BM25 index is maintained automatically on INSERT/UPDATE/DELETE — no application-side bookkeeping
- Index rebuild: `CREATE INDEX CONCURRENTLY` + `DROP INDEX` if tokenizer configuration needs to change
- Only one BM25 index per table is active — if schema changes require a new index, create concurrently then drop the old one

### sqlx compatibility

pg_search adds custom operators (`|||`, `&&&`) and functions (`pdb.score()`, `pdb.boost()`) to PostgreSQL. These work through standard SQL queries executed via sqlx — no special sqlx configuration needed. The `|||` operator is just a PostgreSQL operator, and `pdb.score()` / `pdb.boost()` are regular SQL functions.

Queries use `sqlx::query_as!(Recipe, ...)` with raw SQL strings. The `pdb.score()` column is excluded from the SELECT list (or selected into a discarded column) since `Recipe` struct doesn't have a `relevance` field.

Alternative: use a separate `RecipeSearchResult` struct that wraps `Recipe` + `relevance f64`, then strip the relevance at the API layer.

## Out of Scope

- Field-specific search (search ingredients only, steps only, etc. via UI toggle)
- Search result highlighting (snippets showing matched terms with `<mark>` tags)
- Cuisine / dietary category facets (requires new schema fields — future enhancement)
- Hybrid search (BM25 + vector embeddings for semantic search)
- Recipe import search (searching external URLs)
- Fuzzy/typo-tolerant search beyond ngram partial matching (pg_search supports this via `fuzzy` queries, but ngram partial matching covers the common case)
- Search analytics (popular searches, zero-result tracking)
- Saved searches / alerts

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | BM25 index migration | Index created on recipes table, existing rows indexed, ngram + JSONB tokenization working |
| 2 | Query builder + base search | `search_recipes()` with dynamic query builder, boosting, visibility filter, count query |
| 3 | Filters + sorting + pagination | Tag/time/servings filters, sort options, pagination all working in query builder, tests pass |
| 4 | Facet query | `get_search_facets()` returns tag counts from filtered result set |
| 5 | Server functions | All search endpoints compile and return results end-to-end |
| 6 | Search page UI | `/search` route with input, filter panel, sort bar, results grid, pagination, owner attribution |
| 7 | Autocomplete UI | Dropdown below search input, keyboard navigable, debounced, navigates on select |
| 8 | URL state + navigation | All filter state in URL, pre-population on load, browser back/forward, search link in header |

## Success Metrics

- User types "past" in search → autocomplete dropdown suggests "Pasta Carbonara" etc.
- User types "past" in search → finds "pasta carbonara" via ngram partial matching
- User types "chicken breast" → recipes with that ingredient rank highly (own + public)
- User types "slow cook" → finds recipes with "slow cooker" in steps or description
- Title matches rank above description matches, which rank above ingredient matches (boosting)
- User applies "quick" tag + "< 30 min" filter → results narrow correctly
- User sorts by "Quickest" → recipes with shortest total_time_min appear first
- User navigates to `/search?q=chicken&tags=quick&sort=newest&page=2` → all filters pre-populate
- Empty search → shows all visible recipes (own + public) ordered by recency
- User sees their own private recipes but never other users' private recipes
- Non-owned recipes show "by @username" attribution
- Pagination shows correct page boundaries and "X–Y of Z" count
- All 8 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- Search response time < 100ms for up to 10,000 recipes (BM25 index)
