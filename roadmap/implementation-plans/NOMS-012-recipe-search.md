# NOMS-012: Recipe Search — Implementation Plan

**Issue:** [NOMS-012-recipe-search.md](../issues/NOMS-012-recipe-search.md)
**Created:** 2026-06-10
**Revised:** 2026-07-17
**Depends on:** NOMS-008 current recipe model; NOMS-011 collection hierarchy for collection search
**Coordination:** NOMS-010 may be implemented in parallel. NOMS-012 searches published `recipes` rows, owner-only NOMS-011 collections, and public user profiles; it does not search `recipe_drafts`.
**Approach:** Reconcile the entity-search contract first, then implement per-entity BM25 indexes, a typed logical query planner, authenticated server functions, URL-backed scoped search UI, typed autocomplete, and focused ParadeDB tests. Add the landing page as a lightweight entry point into the canonical search route.

---

## Implementation contract

### Search corpus and authorization

- The default corpus is the current user’s recipes at any visibility plus other users’ `public` recipes.
- Other users’ `unlisted` and `private` recipes are excluded from global search and autocomplete.
- `Mine only` returns every recipe owned by the current user.
- `Public only` returns only rows with `visibility = 'public'`.
- Drafts are excluded because NOMS-010 stores them in `recipe_drafts`; no `is_draft` filter or draft index is introduced.
- Collection search returns only collections owned by the current user because NOMS-011 collections are private. Nested collections are searchable and results include breadcrumb paths.
- User search returns public profile fields only: `username`, `display_name`, `bio`, and avatar metadata where appropriate. Email, OAuth, session, and other private account fields are never indexed or returned.
- Search scopes are `All`, `Recipes`, `Collections`, and `Users`.
- The `All` scope returns grouped entity sections with independent per-entity ranking. BM25 scores from different indexes are not compared directly.
- A single `SearchRequest` carries distinct `query` and `filters` sections alongside multi-key sorting, facet requests, pagination, and scope.
- `query` is one user-entered query string. The backend applies the selected entity’s tuned multi-field BM25 profile, including field selection, token handling, and boosts.
- `filters` is a hard constraint tree supporting nested `AND`, `OR`, and `NOT` nodes with typed field/operator/value combinations. Filters include/exclude rows and never contribute to score.
- Full-text matching, query-parser syntax, and field boosts are not permitted inside filters; text-valued filters are exact/membership constraints only.
- A simple query string is interpreted as an `AND` across terms and an `OR` across the entity’s tuned searchable fields. Quoted phrases remain phrase searches where supported.
- The `All` scope applies the query string through each entity’s tuned text profile and returns grouped sections. Entity-specific hard filters, such as recipe tags or collection fields, require switching to the corresponding scoped search.
- The backend validates filter depth, node count, field capabilities, value types, and query lengths before compilation.
- Only `query` produces relevance. When query is absent or blank, relevance is null and the requested non-relevance sort is used.
- Server functions derive the current user from the authenticated session. They do not accept a caller-supplied user ID for authorization.

### Current recipe fields

Use the current schema and shared type names:

- `recipes.user_id`
- `recipes.visibility` with `private`, `unlisted`, and `public`
- `recipes.title`, `description`, `ingredients`, and `instructions`
- `recipes.prep_time_minutes`, `cook_time_minutes`, `servings`
- `recipes.created_at`, `updated_at`
- `recipe_tags(recipe_id, tag)`

Do not use those legacy names for recipe columns: `owner_id`, `is_public`, `is_draft`, `steps`, `total_time_min`, `prep_time_min`, or `cook_time_min`. NOMS-011’s collection table intentionally uses `owner_id` for collection ownership.

### Total time and ranges

Define total time as:

```sql
COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0)
```

A time filter must also require `r.prep_time_minutes IS NOT NULL OR r.cook_time_minutes IS NOT NULL`, so a recipe with both fields null does not appear in a time bucket merely because its calculated value is zero. Servings filters exclude null `servings` values.

### Tags, deduplication, and facets

- A recipe must contain every selected tag.
- Prefer one correlated `EXISTS` clause per selected tag. This avoids duplicate recipe rows and makes AND semantics obvious.
- Results and count queries must not paginate over a tag join that multiplies rows.
- Facet counts are counts of distinct recipes in the current filtered result set, including the selected tag filters. This means unselected tags show co-occurrence counts and selected tags normally equal the current result count.
- Facets are requested as part of the same search request and run against the unpaginated filtered result relation. A facet may optionally exclude its own active filter for disjunctive counts.

### Search results and ordering

Search responses use shared types in `src/types.rs` rather than returning raw database rows:

```rust
pub struct SearchResult {
    pub recipe: Recipe,
    pub relevance: Option<f32>,
}

pub struct SearchFacet {
    pub tag: String,
    pub count: i64,
}

pub struct CollectionSearchResult {
    pub collection: CollectionSummary,
    pub breadcrumb: Vec<String>,
    pub relevance: Option<f32>,
}

pub struct UserSearchResult {
    pub user: UserSearchSummary,
    pub relevance: Option<f32>,
}

pub enum SearchScope {
    All,
    Recipes,
    Collections,
    Users,
}

pub struct SearchResponse {
    pub scope: SearchScope,
    pub recipes: Vec<SearchResult>,
    pub recipe_total_count: i64,
    pub collections: Vec<CollectionSearchResult>,
    pub collection_total_count: i64,
    pub users: Vec<UserSearchResult>,
    pub user_total_count: i64,
    pub facets: Vec<SearchFacet>,
}
```

When `query` is absent or its text is blank, relevance is `None` and the default sort is `updated_at DESC`. When query text is present, the default sort is BM25 relevance descending. Every sort option is selected from a fixed server-side allowlist.

### URL history behavior

- URL state is the source of truth for committed search state.
- While typing, use `replaceState` so the browser history does not receive one entry per keystroke.
- Enter, selecting an autocomplete result, changing a filter, changing sort, and changing page use `pushState`.
- Initial load and `popstate` rebuild the local search state from the URL.

---

## Checkpoint 1: Entity contract reconciliation and search test prerequisites

**Files:** `roadmap/issues/NOMS-012-recipe-search.md`, `roadmap/implementation-plans/NOMS-012-recipe-search.md`, `src/test_utils.rs`, `migrations/extensions.sql`

Before implementing search SQL or UI, verify that the issue and plan use the current schema, NOMS-010 draft model, and NOMS-011 collection ownership model.

### Tasks

- Remove legacy field names and the `Drafts` filter from the issue.
- Document the published-recipe corpus and owner/public visibility rules.
- Document owner-only collection search, public-profile-only user search, and the four search scopes.
- Document that `All` uses grouped sections rather than cross-index relevance comparison.
- Document total-time null behavior, tag AND semantics, deduplicated counts, and facet semantics.
- Keep `migrations/extensions.sql` as the required source for `pg_search`; it intentionally fails when the production image lacks the extension.
- Add a search-specific test database setup path that creates `pg_search` and fails explicitly if creation fails.
- Create the BM25 index in search test setup before search tests run. Do not silently skip BM25 tests because an optional extension is unavailable.
- Keep unrelated database tests independent of `pg_search` if they do not exercise search.

### Verify

- No NOMS-012 requirement references `is_draft`, `is_public`, `owner_id`, `steps`, or `total_time_min`.
- Search tests cannot report success after silently failing to create `pg_search` or the BM25 index.
- Collection and user search tests cannot read another user’s private collection or private account fields.
- `git diff --check` passes.

**Risk:** Medium. This checkpoint prevents the implementation from being built against the wrong data model.

---

## Checkpoint 2: BM25 index and supporting database indexes

**Files:** `migrations/schema.sql`, `src/test_utils.rs`, NOMS-011 collection schema integration

### Index definition

Add the BM25 index after the `recipes` table exists and before search queries are implemented. Verify the exact syntax against the PostgreSQL/ParadeDB image used by the repository before applying it:

```sql
CREATE INDEX IF NOT EXISTS idx_recipes_search
ON recipes USING bm25 (
    id,
    (title::pdb.ngram(2, 15)),
    (description::pdb.ngram(2, 15)),
    ingredients,
    instructions
)
WITH (key_field = 'id');
```

Add a supporting tag index if needed by the query plan:

```sql
CREATE INDEX IF NOT EXISTS idx_recipe_tags_tag
    ON recipe_tags(tag);
```

After NOMS-011’s `collections` table is present, add a BM25 index covering collection `name` and `description`. The query must use the collection’s owner predicate and return a breadcrumb path through the NOMS-011 hierarchy query; do not denormalize paths into the search document in this phase.

The intended collection index shape is:

```sql
CREATE INDEX IF NOT EXISTS idx_collections_search
ON collections USING bm25 (
    id,
    (name::pdb.ngram(2, 15)),
    description
)
WITH (key_field = 'id');
```

Add a separate BM25 index on `users` covering `username`, `display_name`, and `bio`. Use ngram tokenization for username/display-name partial matching and the default tokenizer for bio if bio search is retained. Never index `email`, OAuth fields, session fields, or other private account data.

The intended user index shape is:

```sql
CREATE INDEX IF NOT EXISTS idx_users_search
ON users USING bm25 (
    id,
    (username::pdb.ngram(2, 20)),
    (display_name::pdb.ngram(2, 20)),
    bio
)
WITH (key_field = 'id');
```

The JSONB fields use the default tokenizer so text within ingredient and instruction objects can be indexed. The application does not maintain a second search document or perform a data backfill; `CREATE INDEX` indexes existing development rows and the BM25 index maintains itself on recipe DML.

### Verify

- Fresh migrations create the extension before the BM25 index.
- `EXPLAIN` confirms the BM25 index is used for a representative non-empty search.
- A recipe update to title, description, ingredients, or instructions is visible to search without application-side index maintenance.
- The index uses `instructions`, not the legacy `steps` field.
- Collection and user indexes are created only after their source tables exist.

**Risk:** High. ParadeDB syntax and runtime availability must be proven before the query layer is built around it.

---

## Checkpoint 3: Tuned text query, hard-filter planner, and request types

**Files:** `src/types.rs`, `src/db/mod.rs` or a new server-only search module, `src/db/error.rs` if applicable

### Shared types

Add serializable shared types for server-function payloads:

```rust
pub struct SearchRequest {
    pub scope: SearchScope,
    pub query: Option<SearchQuery>,
    pub filters: FilterExpression,
    pub sort: Vec<SearchSortClause>,
    pub facets: Vec<FacetRequest>,
    pub page: u32,
    pub page_size: u32,
}

pub struct SearchQuery {
    pub text: String,
}

pub enum FilterExpression {
    MatchAll,
    MatchNone,
    And(Vec<FilterExpression>),
    Or(Vec<FilterExpression>),
    Not(Box<FilterExpression>),
    Term { field: FilterField, value: SearchValue },
    In { field: FilterField, values: Vec<SearchValue> },
    Range {
        field: FilterField,
        lower: Option<SearchValue>,
        upper: Option<SearchValue>,
    },
    Exists { field: FilterField },
}

pub struct SearchSortClause {
    pub field: SearchSortField,
    pub direction: SortDirection,
}

pub struct FacetRequest {
    pub field: SearchFacetField,
    pub limit: u32,
    pub exclude_current_filter: bool,
}

pub enum FilterField {
    RecipeTitle,
    RecipeDescription,
    RecipeIngredients,
    RecipeInstructions,
    RecipeTag,
    RecipeVisibility,
    RecipeTotalTime,
    RecipeServings,
    RecipeCreatedAt,
    RecipeUpdatedAt,
    CollectionName,
    CollectionDescription,
    CollectionCreatedAt,
    CollectionUpdatedAt,
    UserUsername,
    UserDisplayName,
    UserBio,
}

pub enum SearchValue { Text(String), Integer(i32), Boolean(bool), Timestamp(DateTime<Utc>) }
pub enum SearchSortField { Relevance, Name, UpdatedAt, CreatedAt, RecipeTotalTime, Id }
pub enum SearchFacetField { RecipeTag, RecipeVisibility, RecipeTotalTime, RecipeServings }
pub enum SortDirection { Asc, Desc }

pub struct UserSearchSummary {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub public_recipe_count: i64,
}

pub struct CollectionSearchResult {
    pub collection: CollectionSummary,
    pub breadcrumb: Vec<String>,
    pub relevance: Option<f32>,
}

pub struct UserSearchResult {
    pub user: UserSearchSummary,
    pub relevance: Option<f32>,
}
```

The intended wire shape keeps query and filters visibly separate:

```json
{
  "scope": "recipes",
  "query": { "text": "pasta carbonara" },
  "filters": {
    "kind": "and",
    "children": [
      { "kind": "term", "field": "recipe_tag", "value": "quick" },
      { "kind": "range", "field": "recipe_total_time", "lower": 0, "upper": 30 }
    ]
  },
  "sort": [
    { "field": "relevance", "direction": "desc" },
    { "field": "updated_at", "direction": "desc" }
  ],
  "facets": [
    { "field": "recipe_tag", "limit": 50, "exclude_current_filter": false }
  ],
  "page": 1,
  "page_size": 20
}
```

Use the repository’s existing derive conventions. Validate and normalize page size, page number, query length, filter depth/node count, field/operator compatibility, value types, facet limits, and ranges before constructing SQL. The client may send serialized values through server functions, but the database layer receives typed values after parsing.

### Query builder rules

- Build a fixed recipe query selecting the full `Recipe` projection plus `pdb.score(r.id)` when a text query exists.
- Build separate collection and user queries with their own projections and indexes. Do not join all entity tables into one polymorphic SQL query.
- Compile `SearchQuery.text` through an entity-specific tuning profile. The profile controls searchable fields, token parsing, phrase handling, partial matching, and boosts; the client does not submit field boosts or raw ParadeDB syntax.
- For recipes, apply the planned title/description/ingredients/instructions tuning. Collections and users use their own field profiles.
- Compile `FilterExpression` recursively into a parameterized predicate tree. `And`, `Or`, and `Not` must preserve grouping rather than flattening all constraints into one implicit conjunction.
- Map filter leaves to typed hard SQL predicates: tag membership, visibility, total-time/servings/date ranges, collection ownership, and exact/in/existence checks where supported.
- Keep query matching/scoring separate from hard filters so filter clauses cannot accidentally affect BM25 relevance.
- Join `users` to populate the existing `author_username` and `author_avatar_url` fields.
- Apply visibility predicates before search and filter predicates.
- Use parameter binding for every user value.
- Use a fixed allowlist for sort SQL; never interpolate a client-provided sort string.
- Use the documented ParadeDB cast form for boosts, for example `'$query'::pdb.boost(3)` after binding the query value through the supported SQLx pattern.
- Boost title at 3x, description at 2x, and ingredients/instructions at 1x.
- Use one `EXISTS` predicate per selected tag.
- Apply the total-time expression and null guard for time filters.
- Use `LIMIT` and `OFFSET` only after the filtered entity set is deduplicated.
- Collection queries enforce `c.owner_id = user_id`, search `name` and `description`, and fetch root-to-leaf breadcrumb labels through the NOMS-011 recursive hierarchy query.
- User queries search only `username`, `display_name`, and optionally `bio`, returning a `UserSearchSummary` without email or authentication fields.
- `All` executes the applicable per-entity queries and returns grouped sections with independent limits; it never sorts by a raw cross-entity BM25 score.
- Compile a multi-key sort list from a fixed allowlist and append `id ASC` as a deterministic final tie-breaker when the client does not provide it.
- Evaluate facet requests against the same query and unpaginated hard-filter relation. If `exclude_current_filter` is true, remove only that facet field’s predicate while retaining every other filter node and the complete query.
- Reject unsupported combinations, such as recipe tag facets in user scope or collection fields in recipe scope, before SQL generation.

Keep the base filtered query reusable by results, count, and facet functions. A CTE or an equivalent builder abstraction is preferred over three independently maintained sets of clauses.

### Database functions

Implement server-only functions with session-neutral database signatures:

```rust
search_recipes(pool, user_id, request) -> Result<Vec<SearchResult>, DbError>
count_search_recipes(pool, user_id, request) -> Result<i64, DbError>
get_search_facets(pool, user_id, request) -> Result<Vec<SearchFacet>, DbError>
search_collections(pool, user_id, request) -> Result<Vec<CollectionSearchResult>, DbError>
count_search_collections(pool, user_id, request) -> Result<i64, DbError>
search_users(pool, request) -> Result<Vec<UserSearchResult>, DbError>
count_search_users(pool, request) -> Result<i64, DbError>
```

The server-function layer will obtain `user_id` from the session and pass it to these database functions.

### Verify

- Exact title matches rank above weaker field matches.
- Partial title input such as `past` matches `pasta`.
- Ingredient and instruction text are searchable.
- Empty query uses recency ordering and does not call BM25 scoring unnecessarily.
- Nested AND/OR/NOT filter trees produce the expected result sets.
- Results, counts, and facets agree for every valid combined request.
- Query construction contains no untrusted SQL fragments.
- Collection search cannot return another user’s collections.
- User search cannot return private account fields.

**Risk:** High. This is the core correctness and injection-safety boundary.

---

## Checkpoint 4: Facets, entity queries, and typed autocomplete

**Files:** `src/db/mod.rs` or the server-only search module, `src/types.rs`, `src/test_utils.rs`

### Facets

Build requested facets from the same filtered entity relation used by results and counts. Recipe tag facets are one example:

```sql
SELECT rt.tag, COUNT(DISTINCT fr.id) AS tag_count
FROM filtered_recipes fr
JOIN recipe_tags rt ON rt.recipe_id = fr.id
GROUP BY rt.tag
ORDER BY tag_count DESC, rt.tag ASC
LIMIT 50;
```

The filtered relation must apply the complete tuned query and hard-filter tree. Do not paginate the relation used by facets. For `exclude_current_filter`, rebuild only the requested facet’s base relation without that facet field predicate while preserving the query and all other filters.

### Collection and user results

- `search_collections` returns owner-owned collections at every hierarchy depth, with direct collection metadata and breadcrumb labels.
- `count_search_collections` counts only the current owner’s matching collections.
- `search_users` returns public profile summaries and may include public recipe counts, but never email or authentication data.
- `count_search_users` counts matching public profile rows.
- Collection and user sorts are scoped to their entity type. Use relevance, newest/updated, and name sorts where meaningful; do not expose recipe time or tag filters for these scopes.

### Typed autocomplete

Implement per-entity autocomplete queries and a typed aggregate response:

```rust
pub enum SearchSuggestionKind {
    Recipe,
    Collection,
    User,
}

pub struct SearchSuggestion {
    pub kind: SearchSuggestionKind,
    pub id: Uuid,
    pub label: String,
    pub secondary_text: Option<String>,
}

autocomplete_entities(pool, user_id, scope, query_text, limit)
    -> Result<Vec<SearchSuggestion>, DbError>
```

Autocomplete must apply the same visibility rules as search, require at least two characters, cap the limit server-side at 8, and use the appropriate entity index. For `All`, allocate a deterministic small number of suggestions per entity or return grouped suggestions; never compare raw BM25 scores from different indexes. Use the documented `pdb.boost` cast syntax.

### Verify

- Facets count distinct recipes and expose tag co-occurrence correctly.
- Selected tags remain AND filters in the facet base relation.
- Facets can be requested alongside the text query, hard filters, sorting, and pagination.
- Disjunctive facet mode excludes only the requested facet field’s predicate.
- Autocomplete never returns another user’s private or unlisted recipe.
- Own private and unlisted recipes can appear in autocomplete.
- Collection autocomplete returns only the current user’s collections.
- User autocomplete returns only public profile fields.
- Suggestions are capped even if a client requests a larger limit.

**Risk:** Medium. Facets are easy to make subtly inconsistent with the main result query.

---

## Checkpoint 5: Authenticated server functions

**Files:** `src/api/search.rs` (new), `src/api/mod.rs`, `src/types.rs`

Implement server functions that extract the authenticated session user and parse/validate client payloads:

```text
search_entities(request) -> SearchResponse
autocomplete_entities(scope, query_text) -> Vec<SearchSuggestion>
```

The search server function should validate and normalize the complete `SearchRequest`, then dispatch to the typed recipe, collection, and user database functions using the same query, hard-filter tree, sort, pagination, and facet requests. For `All`, it should fetch bounded per-entity sections and return their independent counts. Recipe facets are returned only for recipe-capable scopes. It should return a stable response when query is absent and the request is filter-only. Errors should be safe for client display and should not expose SQL details.

Do not accept `user_id`, `owner_id`, or an authorization visibility predicate from the client. The client may select a search visibility mode, but the server applies the authenticated-user visibility contract.

### Tests

- unauthenticated search and autocomplete fail
- authenticated users see their own private/unlisted recipes
- other users’ private/unlisted recipes are excluded
- other users’ public recipes are included
- users can search their own private collections at any hierarchy depth
- users cannot search another user’s private collections
- user results contain no email, OAuth, session, or other private account fields
- malformed filters and invalid ranges return controlled errors
- malformed, overly deep, or unsupported filter trees return controlled errors
- page size and autocomplete limits are server-enforced

---

## Checkpoint 6: Search route, URL state, and page shell

**Files:** `src/pages/search.rs` (new), `src/pages/mod.rs`, `src/main.rs`, `src/middleware/auth.rs`, URL helper module as appropriate

### Route

Add a protected `/search` route and validate any query parameters before creating `SearchRequest`. The route has no path identifier; all search state lives in the query string.

### URL codec

Use a small parser/serializer with canonical keys:

| Key | Values |
|---|---|
| `scope` | `all`, `recipes`, `collections`, `users` |
| `q` | URL-encoded query text |
| `sort` | comma-separated scope-specific keys such as `relevance:desc,updated:desc` |
| `tags` | comma-separated normalized tags; recipe scope only |
| `time` | `0-30`, `30-60`, `60-120`, `120+`; recipe scope only |
| `servings` | `1-2`, `3-4`, `5-8`, `8+`; recipe scope only |
| `vis` | `all`, `mine`, `public`; recipe scope only |
| `facets` | comma-separated requested facet fields |
| `filters` | optional versioned URL-encoded `FilterExpression` for advanced nested hard constraints |
| `page` | positive 1-indexed page |

Omit default values from canonical URLs where practical. Invalid values fall back to defaults rather than reaching SQL.
Recipe-only filters are removed or ignored when the scope is collections or users.
The `q` parameter maps only to `SearchQuery.text`. Simple filter parameters are shorthand that compile into `FilterExpression`. If `filters` is present, it is parsed and validated as the authoritative filter tree; conflicting shorthand filters are rejected or replaced during canonicalization. Cap filter URL length and complexity before sending it to the server.

### History and loading behavior

- On input, update the local query and use `replaceState` for the URL while autocomplete is debounced.
- Enter commits the current query with `pushState` and reloads search results.
- Filter, sort, and page changes use `pushState`, reset page to 1 when appropriate, and reload results.
- Register a `popstate` handler to parse the URL and reload the page state.
- Guard browser APIs behind the existing client/WASM patterns so SSR remains safe.

### Verify

- `/search?scope=recipes&q=pasta&tags=quick%2Cvegetarian&sort=updated:desc&page=2` restores all recipe controls correctly.
- `/search?scope=collections&q=summer` restores collection scope and does not display recipe facets.
- An advanced filter URL round-trips without changing its logical grouping or query text.
- Back/forward restores prior results and filter state.
- Typing does not flood browser history.
- Committed changes create navigable history entries.

**Risk:** Medium. Browser history and SSR/WASM boundaries need explicit testing.

---

## Checkpoint 7: Search UI, landing entry point, filters, facets, and autocomplete

**Files:** `src/components/base/search_input.rs` (new), `src/components/base/landing_search_bar.rs` (new), `src/components/base/autocomplete_dropdown.rs` (new), `src/components/base/search_filters.rs` (new), `src/components/base/advanced_search_builder.rs` (new), `src/components/base/search_results.rs` (new), `src/components/base/search_pagination.rs` (new), `src/pages/home.rs`, `src/pages/search.rs`, `src/components/base/mod.rs`, `src/components/navbar.rs`, header/sidebar integration files

Implement the UI in separate components so URL state and server calls remain testable independently.

### Search input and autocomplete

- Debounce autocomplete requests at approximately 200ms.
- Do not request suggestions for fewer than two characters.
- Support arrow-key navigation, Enter, Escape, mouse selection, loading, and error states.
- Render suggestions with entity-kind labels and appropriate secondary text.
- Selecting a recipe suggestion navigates to the recipe detail route.
- Selecting a collection suggestion navigates to the collection detail route.
- Selecting a user suggestion navigates to the existing public user profile route.

### Landing-page search bar

- Add a prominent `LandingSearchBar` to `src/pages/home.rs` using the shared search-input styling and validation.
- On Enter or submit, navigate to `/search?q=<encoded query>`; preserve no separate landing-page result state.
- The landing bar may be visible before authentication, but `/search` remains protected in this phase. An unauthenticated submit follows the existing protected-route login flow.
- Keep landing autocomplete optional and disabled initially unless the product chooses to make public recipe/user search available without authentication.
- Do not call search database functions directly from `Home`.

### Filters and results

- Search input at the top of the page.
- Scope tabs or equivalent controls: All, Recipes, Collections, Users.
- Recipe scope shows visibility controls, time buckets, servings buckets, and tag facets.
- Collection scope shows owner-owned folder results with breadcrumb paths and collection-appropriate sort controls.
- User scope shows public profile results with user-appropriate sort controls.
- All scope renders bounded grouped sections rather than one cross-entity ranked list.
- Tag checklist is populated from recipe facet results with counts and is hidden for non-recipe scopes.
- The main text input writes only `SearchQuery.text`; field selection and relevance tuning stay server-owned.
- Preset controls compile into the same `FilterExpression` used by the advanced filter builder.
- An advanced builder allows users to add field/operator/value clauses, nest groups, and choose `AND`, `OR`, or `NOT` logic.
- The builder exposes only fields and operators supported by the active entity scope.
- Users can request multiple facets and multi-key sorting without losing the text query or other filters.
- Active filter chips and clear-all action.
- Sort selector with relevance, newest, oldest, name ascending/descending, quickest, and longest.
- Recipe cards with relevance-independent owner attribution for non-owned results.
- Collection result cards/rows link to collection detail and show their breadcrumb path.
- User result cards/rows link to public profiles and show only public profile metadata.
- Result count and range context such as `121–132 of 847`.
- Empty, loading, error, and no-results states.
- Pagination with previous/next controls and bounded page-number display.
- Landing-page submission produces one canonical `/search?q=...` navigation and does not create a second search state machine.
- Advanced filter edits preserve query text and produce a validated canonical URL state plus one combined backend request.

### Navigation integration

Add a search link or icon to the existing header/sidebar. If the dashboard receives a search shortcut, it should navigate to `/search` rather than duplicate search state or query logic.

Coordinate shared files with NOMS-010 and NOMS-011; do not overwrite their changes.

---

## Checkpoint 8: Integration, tests, and performance review

**Files:** search-related files plus shared integration files

### Database tests

Use a ParadeDB-capable test environment and fail explicitly when `pg_search` or the BM25 index cannot be created. Cover:

- exact and partial title matches
- description, ingredient, and instruction matches
- title/description boosting
- tuned query-string behavior, field boosts, phrase handling, and partial matching
- nested AND/OR/NOT filter compilation and execution
- typed filter fields and unsupported-field rejection
- query plus hard filters in one request
- multiple sort keys and deterministic tie-breaking
- empty query ordering
- own/private/unlisted versus other-user visibility
- public recipes from other users
- all-selected-tags AND behavior
- duplicate-safe result and count queries
- time expression, both-null exclusion, and one-null handling
- servings ranges and null exclusion
- every sort option
- facets and distinct counts
- multiple requested facets, including disjunctive facet counts
- pagination boundaries
- collection search at root and nested depths with breadcrumb paths
- collection owner isolation
- user search by username/display name with public profile-only projection
- autocomplete minimum length, per-entity relevance, visibility, and limit

### URL/UI tests

- query-string parsing and canonical serialization
- advanced filter URL round-tripping, query preservation, and complexity limits
- replaceState versus pushState behavior
- initial URL hydration
- browser back/forward restoration
- filter reset and page reset behavior
- keyboard autocomplete behavior
- scope switching and typed suggestion navigation

### Verification commands

```text
cargo test --features server
cargo check --features server
cargo check --target wasm32-unknown-unknown
cargo clippy --features server --all-targets -- -D warnings
just migrate
```

Review representative `EXPLAIN` plans for recipe text search, collection search, user search, empty-query browsing, tag filtering, and facets. Confirm that the appropriate per-entity BM25 indexes are used, that pagination is applied after deduplication, and that the All scope uses bounded per-entity queries.

---

## Migration and rollback

- `migrations/extensions.sql` remains responsible for installing `pg_search`.
- `migrations/schema.sql` adds recipe, collection, user, and tag search indexes additively. Collection index creation must follow the NOMS-011 table definition when branches are merged.
- No application data backfill is required; development data may be discarded during integration.
- If the index definition changes, create a replacement index using the repository’s supported concurrent/rebuild procedure, validate it, then remove the old index in a separate migration step if required.
- Do not add destructive rollback SQL to the plan.

## Dependencies and parallelization

| Checkpoint | Depends on | Parallelization |
|---|---|---|
| 1 Contract/test prerequisites | Current recipe schema, NOMS-010 draft decision, and NOMS-011 collection privacy model | Can proceed immediately |
| 2 BM25 indexes | Checkpoint 1; NOMS-011 collection table for collection index | Recipe/user indexes can proceed before NOMS-011; collection index waits for its table |
| 3 Query builder/types | Checkpoint 1; index syntax verified | Can proceed alongside Checkpoint 2 after field contract is settled |
| 4 Facets/autocomplete | Checkpoint 3 | Database-focused and independent of page UI |
| 5 Server functions | Checkpoints 3–4 | Can proceed before UI |
| 6 Route/URL state | Checkpoint 5 | Mostly client-side; coordinate route files with other work |
| 7 UI | Checkpoints 5–6 | Shared header/sidebar files may conflict with NOMS-010/NOMS-011 |
| 8 Integration | All previous checkpoints | Must run with the combined feature branches |
