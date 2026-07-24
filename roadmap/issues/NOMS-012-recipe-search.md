# NOMS-012: Recipe Search

**Status:** ⚪ Backlog  
**Phase:** Phase 2 (discovery and organization)  
**Revised:** 2026-07-17
**Depends on:** NOMS-008 current recipe model; NOMS-011 collection hierarchy for collection search; NOMS-010 may be implemented in parallel

## Overview

Add authenticated search across recipes, private owner collections, and public user profiles using ParadeDB `pg_search` BM25 ranking, ngram matching, field boosting, filtering, sorting, pagination, facets, and typed autocomplete.

Search uses separate BM25 indexes and query paths for each entity type. NOMS-010 drafts are stored separately in `recipe_drafts` and are excluded from this feature. Draft search would require a later design for a separate search source.

## Search contract

- The default corpus includes all recipes owned by the current user, regardless of `visibility`, plus other users' `public` recipes.
- Other users' `unlisted` and `private` recipes are never returned by global search or autocomplete.
- The `Mine only` filter returns all recipes owned by the current user.
- The `Public only` filter returns only recipes with `visibility = 'public'`.
- No draft filter exists in this issue. `recipe_drafts` is not indexed or queried.
- Collection search returns only collections owned by the current user because NOMS-011 collections are private. Child collections are searchable; results include their breadcrumb path.
- User search returns public profile fields (`username`, `display_name`, `bio`, and avatar metadata if needed). It never searches or returns email, OAuth, session, or other private account fields.
- Entity scopes are `All`, `Recipes`, `Collections`, and `Users`. `All` returns grouped sections rather than comparing BM25 scores across different indexes.
- Search indexes `title`, `description`, `ingredients`, and `instructions`.
- Current recipe fields are `user_id`, `prep_time_minutes`, `cook_time_minutes`, `servings`, `visibility`, `ingredients`, and `instructions`.
- Total time is `COALESCE(prep_time_minutes, 0) + COALESCE(cook_time_minutes, 0)`. A time filter excludes rows where both time fields are null.
- A recipe must match every selected tag. Tag filtering must not duplicate result rows.
- Facet counts are counts of distinct recipes in the current filtered result set, including the selected tag filters.
- Empty query text is valid and returns visible recipes ordered by `updated_at DESC` unless another sort is selected.
- A single search request has distinct `query` and `filters` sections alongside sorting, pagination, and facet requests.
- `query` contains one user-entered query string. The backend applies an entity-specific tuned multi-field BM25 profile, including field selection, token handling, and boosts.
- `filters` is a hard constraint tree supporting nested `AND`, `OR`, and `NOT` groups. Filters include/exclude rows and never contribute to relevance.
- Full-text matching, field boosts, and query-parser syntax are not allowed inside filters; text-valued filters are exact/membership constraints only.
- A simple query string is interpreted as an AND across terms and an OR across the entity’s tuned searchable fields; quoted phrases remain phrase searches where ParadeDB supports them.
- Structured predicates support typed equality, membership, existence, and numeric/date ranges where the entity supports them.
- The `All` scope applies the query string through each entity’s tuned text profile and returns grouped entity sections. Entity-specific hard filters such as recipe tags or collection fields require a scoped search.
- Only `query` contributes to BM25 relevance. Filter-only requests use non-relevance sorting.
- Search and autocomplete authorization is derived from the authenticated session, not a caller-supplied user ID.

## Acceptance Criteria

### AC1: BM25 search index

- [ ] Migration creates a BM25 index on `recipes` with `id` as `key_field`.
- [ ] The index covers `title`, `description`, `ingredients`, and `instructions`.
- [ ] `title` and `description` use `pdb.ngram(2, 15)` for partial matching.
- [ ] JSONB `ingredients` and `instructions` use the default tokenizer so their text subfields are searchable.
- [ ] The index uses the documented `pdb.boost` cast syntax for relevance queries.
- [ ] A supporting tag index exists for facet queries.
- [ ] Creating the index indexes existing development rows automatically; no application backfill is required.
- [ ] The index remains maintained by recipe INSERT/UPDATE/DELETE operations.
- [ ] A separate BM25 index covers owner-searchable collection names/descriptions after NOMS-011 creates the table.
- [ ] A separate BM25 index covers public user profile fields: `username`, `display_name`, and `bio`.
- [ ] Email, OAuth, session, and other private account fields are not indexed.

### AC2: Search filters and results

- [ ] Shared `SearchRequest` types represent entity scope, an optional tuned text `query`, a hard logical `filters` tree, multi-key sort, requested facets, page, and page size.
- [ ] Visibility predicates implement the search contract above.
- [ ] Search matches title, description, ingredients, and instructions.
- [ ] Title and description relevance are boosted above ingredients and instructions.
- [ ] Multiple selected tags use AND semantics.
- [ ] Time filters use the defined total-time expression and exclude recipes with both time fields null.
- [ ] Servings ranges exclude null servings.
- [ ] Sort choices are whitelisted; user input never becomes raw SQL.
- [ ] Multiple sort keys are supported with deterministic ID tie-breaking.
- [ ] Query text, hard filters, sorting, pagination, and facets can be submitted together in one request.
- [ ] Unsupported fields/operators for an entity scope are rejected with controlled validation errors.
- [ ] Empty query text skips BM25 matching and defaults to newest-first ordering.
- [ ] Search results are returned through a shared `SearchResult` type containing `Recipe` and optional relevance.
- [ ] Recipe, collection, and user results use typed response variants; unrelated entities are not coerced into `Recipe`.
- [ ] Each entity scope has a count query using the same query and applicable filters; recipe counts are distinct recipe counts.

### AC3: Facets

- [ ] Tag facets are computed from the current filtered result set.
- [ ] Facets use `COUNT(DISTINCT recipe_id)`.
- [ ] Facet queries do not reintroduce duplicate recipes through tag joins.
- [ ] Facets are limited and deterministically ordered by count descending, then tag ascending.
- [ ] Recipe-only facets are hidden or omitted for collection and user scopes.
- [ ] Facet requests are evaluated against the same query and unpaginated hard-filter relation as the result set.

### AC4: Search page and filters

- [ ] `/search` is an authenticated route with search input, filters, sorting, results, facets, and pagination.
- [ ] The landing page includes a prominent search bar that submits to the canonical `/search?q=...` route.
- [ ] The landing-page bar does not execute a separate search query or maintain separate result state.
- [ ] The page supports entity scope tabs or equivalent controls for All, Recipes, Collections, and Users.
- [ ] The All view renders separate recipe, collection, and user sections with per-entity result limits and links to scoped searches.
- [ ] Preset controls compile into the shared hard-filter tree.
- [ ] An advanced-search builder supports field/operator/value clauses with nested `AND`, `OR`, and `NOT` groups.
- [ ] The builder exposes only fields and operators supported by the selected entity scope.
- [ ] Results reuse the existing recipe card pattern and show owner attribution for recipes not owned by the current user.
- [ ] Collection results show collection name, description, breadcrumb path, and a direct navigation link.
- [ ] User results show username/display name and public profile metadata only.
- [ ] The page supports empty, loading, error, and no-results states.
- [ ] Visibility filters are `All`, `Mine only`, and `Public only`; there is no `Drafts` filter.
- [ ] Time buckets are `< 30 min`, `30–60 min`, `1–2 hr`, and `2+ hr`.
- [ ] Servings buckets are `1–2`, `3–4`, `5–8`, and `8+`.
- [ ] Active filters appear as removable chips and a clear-all action resets them.

### AC5: URL state and navigation

- [ ] Entity scope, query, applicable filters, sort, and page are encoded in `/search` query parameters.
- [ ] Query text, advanced filter trees, and requested facets can be serialized into a bounded, versioned URL representation.
- [ ] Initial page state is reconstructed from the URL.
- [ ] Browser back/forward restores the corresponding search state.
- [ ] Typing and debounced autocomplete updates use `replaceState`.
- [ ] Enter, selected suggestions, filter changes, sort changes, and pagination use `pushState`.
- [ ] Search input and filter controls remain synchronized with the URL.

### AC6: Autocomplete

- [ ] Typed autocomplete returns recipe, collection, and user suggestions with an entity kind, ID, label, and optional secondary text.
- [ ] Suggestions require at least two characters and are capped at a configured limit, initially 8.
- [ ] Suggestions are grouped or deterministically allocated by entity kind; BM25 scores from different indexes are not compared as if they were one scale.
- [ ] Recipe suggestions use the recipe visibility contract; collection suggestions are owner-only; user suggestions contain public profile fields only.
- [ ] The dropdown supports keyboard navigation, Enter selection, Escape dismissal, and mouse selection.
- [ ] Selecting a suggestion navigates to the recipe detail page.
- [ ] Requests are debounced to avoid a request for every keystroke.

### AC7: Database, server-function, and UI tests

- [ ] Search tests explicitly require `pg_search` and the BM25 index; extension/index setup failures are not silently ignored.
- [ ] Exact and partial title matches are covered.
- [ ] Ingredient and instruction text matches are covered.
- [ ] Empty query ordering is covered.
- [ ] Own private/unlisted recipes are returned; other users' private/unlisted recipes are excluded.
- [ ] Other users' public recipes are returned.
- [ ] Multiple tags require all selected tags.
- [ ] Tag joins do not duplicate results or counts.
- [ ] Time, servings, visibility, combined-filter, sort, facet, and pagination behavior is covered.
- [ ] Tuned query behavior, nested hard-filter trees, unsupported-field validation, multi-key sorting, and combined request execution are covered.
- [ ] Autocomplete visibility, minimum length, relevance, and limit behavior is covered.
- [ ] Collection and user search/autocomplete authorization is covered.
- [ ] URL parsing, URL synchronization, and browser navigation behavior is covered at the component/manual-test level.
- [ ] Advanced filter URL serialization round-trips without changing logical grouping.

## Technical details

### BM25 index

The implementation plan must verify the exact ParadeDB syntax supported by the repository's PostgreSQL image before applying the migration. The intended index shape is:

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

Add a supporting index on `recipe_tags(tag)` if the existing primary key index is insufficient for facet access.

Add separate BM25 indexes after the NOMS-011 collections table and the existing `users` table are available. Collection indexing covers `name` and `description`; user indexing covers `username`, `display_name`, and `bio`. Do not index email, OAuth data, session data, or other private fields.

### Shared response types

Shared types belong in `src/types.rs`; SQLx row adapters remain server-only:

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
    Range { field: FilterField, lower: Option<SearchValue>, upper: Option<SearchValue> },
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

pub struct SearchResult {
    pub recipe: Recipe,
    pub relevance: Option<f32>,
}
pub struct CollectionSearchResult { /* collection summary, breadcrumb, relevance */ }
pub struct UserSearchResult { /* public user summary, relevance */ }
pub enum SearchEntity {
    Recipe(SearchResult),
    Collection(CollectionSearchResult),
    User(UserSearchResult),
}
pub enum SearchScope { All, Recipes, Collections, Users }
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
pub struct SearchFacet {
    pub tag: String,
    pub count: i64,
}
```

### Query semantics

The query planner handles `query` and `filters` independently. It compiles the query string through the selected entity’s tuned multi-field BM25 profile, while it compiles `FilterExpression` recursively into parameterized hard predicates. `And`, `Or`, and `Not` preserve the frontend’s filter grouping. Field names, operators, and value types come from typed allowlists; the client never supplies SQL fragments.

The recipe query profile interprets a simple query such as `pasta carbonara` conceptually as:

```text
AND(
  OR(title CONTAINS "pasta", ingredients CONTAINS "pasta", instructions CONTAINS "pasta"),
  OR(title CONTAINS "carbonara", ingredients CONTAINS "carbonara", instructions CONTAINS "carbonara")
)
```

Structured filters such as tags, visibility, time, and servings are `FilterExpression` nodes, so query text, hard filters, sorting, pagination, and facets are evaluated as one request. Tag-all behavior can be represented as an `And` of tag term nodes.

Use a shared filtered-recipe CTE or equivalent base query for results, counts, and facets. The results and counts must not join tags in a way that multiplies recipe rows.

Facet requests retain the same text query and run against the unpaginated hard-filter relation. `exclude_current_filter` supports disjunctive facet counts when the UI needs to show alternative values for the same facet; the default for the initial UI may remain false.

## Out of scope

- Searching `recipe_drafts`
- Searching private or unlisted recipes owned by other users
- Semantic/vector search
- Search analytics, saved searches, or alerts
- Search within a collection only
- New recipe fields or tag taxonomy changes
- Fuzzy typo correction beyond the selected ngram behavior
