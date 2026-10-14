# NOMS-012: Recipe Search

**Status:** ⚪ Backlog  
**Phase:** Phase 2 (discovery and organization)  
**Revised:** 2026-08-01
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
- Facet counts are distinct recipe counts from fixed backend queries. The tag facet retains current tag criteria so its buckets describe additive AND-tag selection. Visibility, total-time, and servings facets omit only their own filter. Authorization, simple and advanced text criteria, and every other filter remain active.
- Total-time buckets are `[0,30)`, `[30,60)`, `[60,120)`, and `[120,+∞)` minutes. Servings buckets are `[1,3)`, `[3,5)`, `[5,9)`, and `[9,+∞)`. Numeric bounds are lower-inclusive and upper-exclusive.
- Empty query text is valid and returns visible recipes ordered by `updated_at DESC` unless another sort is selected.
- Blank `All` text is a valid bounded browse state: recipes and collections use newest-first ordering, users use username ordering, and the canonical URL omits `q`.
- The client calls fixed scope-specific server functions using `RecipeSearchRequest`, `CollectionSearchRequest`, `UserSearchRequest`, or the simple-text-only `AllSearchRequest`. It never supplies an arbitrary query/filter tree, field name, SQL fragment, boost, or ParadeDB syntax.
- Each request contains optional literal `text` and only that scope's fixed filters and sort enum. The backend owns SQL structure, authorization, BM25 operators, field profiles, boosts, facets, and deterministic tie-breaking.
- Simple text requires all backend-tokenized terms across the entity's tuned searchable fields, while allowing different terms to match different fields. Boolean-looking words and punctuation have no caller-controlled query-language meaning.
- Advanced search remains in scope through bounded scope-specific criteria. Recipe advanced search provides field-specific include/exclude text with `AllTerms`, `AnyTerm`, or `ExactPhrase`, tag any/excluded sets, and created/updated ranges alongside the normal visibility, tag-all, total-time, and servings controls. Collection and user advanced search expose smaller fixed field sets.
- Populated positive advanced fields and simple text are required together. Exclusions and structured criteria include or exclude rows without contributing relevance. Advanced criteria cannot express arbitrary nesting or custom operators.
- Structured criteria are limited to the statically declared tag, visibility, numeric-range, and date-range fields supported by the selected scope.
- The `All` scope applies literal `text` through each entity’s tuned text profile and returns grouped entity sections. Entity-specific hard filters such as recipe tags or collection fields require a scoped search.
- Simple text and positive advanced text criteria contribute to BM25 relevance. When no positive text remains, `Relevance` is normalized to `UpdatedNewest` for recipes/collections or `UsernameAscending` for users.
- Search and autocomplete authorization is derived from the authenticated session, not a caller-supplied user ID.
- Shared initial limits are: 256 Unicode scalar values per simple/advanced text criterion; 5 exclusions per text field; 100 Unicode scalar values per normalized tag and 25 distinct tags across all tag lists; page size 20 by default and 100 maximum with a maximum offset of 10,000; `All` section size 5 by default and 10 maximum; advanced URL state capped at 4,096 decoded and 8,192 encoded bytes; autocomplete minimum 2 Unicode scalar values after trimming and maximum 8 suggestions.
- `All` autocomplete uses a fixed grouped allocation of 4 recipes, 2 collections, and 2 users. Unused slots are not reassigned by comparing scores across entity indexes.
- Tag limits are measured after normalization over the distinct union of all tag lists. A tag present in either positive list and `tags_excluded` is a validation error.

## Acceptance Criteria

### AC1: BM25 search index

- [ ] Migration creates a BM25 index on `recipes` with `id` as `key_field`.
- [ ] The index covers `title`, `description`, `ingredients`, and `instructions`.
- [ ] `title` and `description` each have a word-tokenized alias for phrase matching and a `pdb.ngram(2, 15)` alias for partial matching.
- [ ] Immutable index expressions recursively extract ingredient string values and only instruction `text` values, including nested sub-steps, into word-tokenized aliases without indexing JSON keys or photo metadata as content.
- [ ] The index uses the documented `pdb.boost` cast syntax for relevance queries.
- [ ] A supporting tag index exists for facet queries.
- [ ] Creating the index indexes existing development rows automatically; no application backfill is required.
- [ ] The index remains maintained by recipe INSERT/UPDATE/DELETE operations.
- [ ] A separate BM25 index covers owner-searchable collection names/descriptions after NOMS-011 creates the table.
- [ ] A separate BM25 index covers public user profile fields: `username`, `display_name`, and `bio`.
- [ ] Email, OAuth, session, and other private account fields are not indexed.

### AC2: Search filters and results

- [ ] Shared scope-specific request types expose literal text, bounded advanced criteria, fixed filters, one scope-specific sort enum, page, and page size.
- [ ] Visibility predicates implement the search contract above.
- [ ] Search matches title, description, ingredients, and instructions.
- [ ] Title and description relevance are boosted above ingredients and instructions.
- [ ] Multiple selected tags use AND semantics.
- [ ] Time filters use the defined total-time expression and exclude recipes with both time fields null.
- [ ] Servings ranges exclude null servings.
- [ ] Sort choices are whitelisted; user input never becomes raw SQL.
- [ ] Each fixed sort enum maps to backend-owned SQL with deterministic ID tie-breaking.
- [ ] Relevance sorting without positive text is normalized to the scope's documented canonical browse sort.
- [ ] Simple text, advanced criteria, hard filters, sorting, pagination, and fixed facets are handled by one scope-specific operation.
- [ ] Multi-term simple text requires every backend-tokenized term but allows terms to match different searchable fields.
- [ ] Advanced text modes provide all-terms, any-term, and positional exact-phrase behavior on statically assigned fields.
- [ ] Query-parser-like characters remain ordinary bound text and cannot select fields, inject operators, or provide boosts.
- [ ] Oversized text/criteria/tag collections, invalid ranges, and unsupported enum values return controlled validation errors.
- [ ] Shared validation constants enforce the documented text, exclusion, tag, pagination, grouped-result, URL-size, and autocomplete limits without silently truncating criteria.
- [ ] Empty query text skips BM25 matching and defaults to newest-first ordering.
- [ ] Search results are returned through a shared `SearchResult` type containing `Recipe` and optional relevance.
- [ ] Recipe, collection, and user results use typed response variants; unrelated entities are not coerced into `Recipe`.
- [ ] Each entity scope has a count query using the same text criteria and applicable filters; recipe counts are distinct recipe counts.

### AC3: Facets

- [ ] Tag facets retain current tag criteria: unselected buckets show the count after adding that AND tag, selected tag-all buckets equal the current result count, and zero-count tags are omitted.
- [ ] Facets use `COUNT(DISTINCT recipe_id)`.
- [ ] Facet queries do not reintroduce duplicate recipes through tag joins.
- [ ] Dynamic tag buckets are limited and deterministically ordered by count descending, then tag ascending.
- [ ] Recipe-only facets are hidden or omitted for collection and user scopes.
- [ ] Fixed recipe facets are evaluated from the same validated request and unpaginated relation as the result set.
- [ ] The fixed recipe facet object uses typed tag, visibility, and lower-inclusive/upper-exclusive range bucket fields.
- [ ] Fixed visibility, total-time, and servings facets return canonical buckets, including zero-count buckets; dynamic tags omit zero-count values and are capped.
- [ ] Explicit backend facet queries retain tag criteria for tag counts and self-exclude only visibility, total-time, or servings for those respective facets.
- [ ] Tag facet buckets are backend-capped at 50; the other fixed facet fields return their complete canonical bucket lists.

### AC4: Search page and filters

- [ ] `/search` is an authenticated route with search input, filters, sorting, results, facets, and pagination.
- [ ] The landing page includes a prominent search bar that submits to the canonical `/search?q=...` route.
- [ ] The landing-page bar does not execute a separate search query or maintain separate result state.
- [ ] The page supports entity scope tabs or equivalent controls for All, Recipes, Collections, and Users.
- [ ] The All view renders separate recipe, collection, and user sections with per-entity result limits and links to scoped searches.
- [ ] Preset controls map directly to fixed request fields.
- [ ] An expandable advanced-search form exposes field-specific include/exclude text modes, tags, and ranges supported by the selected entity scope.
- [ ] The advanced form does not expose arbitrary nesting, free-form fields, custom operators, boosts, SQL, or ParadeDB syntax.
- [ ] Results reuse the existing recipe card pattern and show owner attribution for recipes not owned by the current user.
- [ ] Collection results show collection name, description, breadcrumb path, and a direct navigation link.
- [ ] User results show username/display name and public profile metadata only.
- [ ] The page supports empty, loading, error, and no-results states.
- [ ] Visibility filters are `All`, `Mine only`, and `Public only`; there is no `Drafts` filter.
- [ ] Time buckets are `< 30 min`, `30–59 min`, `60–119 min`, and `2+ hr`.
- [ ] Servings buckets are `1–2`, `3–4`, `5–8`, and `9+`.
- [ ] Active filters appear as removable chips and a clear-all action resets them.

### AC5: URL state and navigation

- [ ] Entity scope, simple text, applicable filters, sort, and page are encoded in `/search` query parameters.
- [ ] Simple text and the active scope's fixed advanced criteria can be serialized into a bounded, versioned URL representation.
- [ ] Initial page state is reconstructed from the URL.
- [ ] Browser back/forward restores the corresponding search state.
- [ ] Typing and debounced autocomplete updates use `replaceState`.
- [ ] Enter, selected suggestions, filter changes, sort changes, and pagination use `pushState`.
- [ ] Search input and filter controls remain synchronized with the URL.

### AC6: Autocomplete

- [ ] Typed autocomplete returns recipe, collection, and user suggestions with an entity kind, ID, label, and optional secondary text.
- [ ] Suggestions require at least two Unicode scalar values after trimming and are capped at 8.
- [ ] `All` suggestions use the fixed 4 recipe / 2 collection / 2 user grouping; BM25 scores from different indexes are not compared or used to refill unused slots.
- [ ] Recipe suggestions use the recipe visibility contract; collection suggestions are owner-only; user suggestions contain public profile fields only.
- [ ] The dropdown supports keyboard navigation, Enter selection, Escape dismissal, and mouse selection.
- [ ] Selecting a recipe, collection, or user suggestion navigates to its corresponding detail/profile route.
- [ ] Requests are debounced to avoid a request for every keystroke.

### AC7: Database, server-function, and UI tests

- [ ] Search tests explicitly require `pg_search` and the BM25 index; extension/index setup failures are not silently ignored.
- [ ] A dedicated CI search job builds and starts the pinned `docker/postgres` image, applies the extensions, and runs search integration tests against it.
- [ ] Migration/schema-plan CI uses the extension-capable image rather than applying `pg_search` and the other required extensions to plain `postgres:17`.
- [ ] Exact and partial title matches are covered.
- [ ] Ingredient and instruction text matches are covered.
- [ ] Nested instruction text is searchable, while instruction photo URLs and metadata are not.
- [ ] Empty query ordering is covered.
- [ ] Own private/unlisted recipes are returned; other users' private/unlisted recipes are excluded.
- [ ] Other users' public recipes are returned.
- [ ] Multiple tags require all selected tags.
- [ ] Tag joins do not duplicate results or counts.
- [ ] Time, servings, visibility, combined-filter, sort, facet, and pagination behavior is covered.
- [ ] Simple text, each advanced text mode, positive-field conjunction, exclusions, scope validation, fixed sorting, and combined request execution are covered.
- [ ] Cross-field simple-term conjunction and query-parser metacharacters as literal text are covered.
- [ ] Typed facet results, additive AND-tag counts, fixed range boundaries, zero-count buckets, and the three self-excluding facets are covered.
- [ ] Autocomplete visibility, minimum length, relevance, and limit behavior is covered.
- [ ] Boundary tests cover all shared request limits and the fixed 4/2/2 `All` autocomplete allocation.
- [ ] Collection and user search/autocomplete authorization is covered.
- [ ] URL parsing, URL synchronization, and browser navigation behavior is covered at the component/manual-test level.
- [ ] Scope-specific advanced criteria URL serialization round-trips without changing text modes, exclusions, or ranges.

## Technical details

### BM25 index

The index shape below has been runtime-verified against the repository's pinned ParadeDB image. Implementation must preserve it in the migration and add repeatable smoke tests for the supported aliases, prepared boosts, phrases, and JSON extraction behavior:

```sql
CREATE INDEX IF NOT EXISTS idx_recipes_search
ON recipes USING bm25 (
    id,
    (title::pdb.simple('alias=recipe_title_words')),
    (title::pdb.ngram(2, 15, 'alias=recipe_title_partial')),
    (description::pdb.simple('alias=recipe_description_words')),
    (description::pdb.ngram(2, 15, 'alias=recipe_description_partial')),
    (jsonb_path_query_array(ingredients, 'strict $.** ? (@.type() == "string")')::pdb.simple('alias=recipe_ingredients_words')),
    (jsonb_path_query_array(instructions, 'strict $.**.text')::pdb.simple('alias=recipe_instructions_words'))
)
WITH (key_field = 'id');
```

Add a supporting index on `recipe_tags(tag)` because the existing primary key begins with `recipe_id` and does not serve tag-first facet access.

Add separate BM25 indexes after the NOMS-011 collections table and the existing `users` table are available. Collection indexing covers `name` and `description`; user indexing covers `username`, `display_name`, and `bio`. Do not index email, OAuth data, session data, or other private fields.

### Shared response types

Shared types belong in `src/types.rs`; SQLx row adapters remain server-only:

```rust
pub enum TextMatchMode { AllTerms, AnyTerm, ExactPhrase }
pub struct TextCriterion { pub text: String, pub mode: TextMatchMode }
pub struct AdvancedTextField {
    pub include: Option<TextCriterion>,
    pub exclude: Vec<TextCriterion>,
}
pub struct IntegerRange {
    pub lower_inclusive: Option<i32>,
    pub upper_exclusive: Option<i32>,
}
pub struct TimestampRange {
    pub lower_inclusive: Option<DateTime<Utc>>,
    pub upper_exclusive: Option<DateTime<Utc>>,
}

pub struct AdvancedRecipeSearchCriteria {
    pub title: AdvancedTextField,
    pub description: AdvancedTextField,
    pub ingredients: AdvancedTextField,
    pub instructions: AdvancedTextField,
    pub tags_any: Vec<String>,
    pub tags_excluded: Vec<String>,
    pub created_at: Option<TimestampRange>,
    pub updated_at: Option<TimestampRange>,
}
pub struct RecipeSearchRequest {
    pub text: Option<String>,
    pub advanced: Option<AdvancedRecipeSearchCriteria>,
    pub visibility: RecipeSearchVisibility,
    pub tags_all: Vec<String>,
    pub total_time: Option<IntegerRange>,
    pub servings: Option<IntegerRange>,
    pub sort: RecipeSearchSort,
    pub page: u32,
    pub page_size: u32,
}

pub struct AdvancedCollectionSearchCriteria {
    pub name: AdvancedTextField,
    pub description: AdvancedTextField,
    pub updated_at: Option<TimestampRange>,
}
pub struct CollectionSearchRequest {
    pub text: Option<String>,
    pub advanced: Option<AdvancedCollectionSearchCriteria>,
    pub sort: CollectionSearchSort,
    pub page: u32,
    pub page_size: u32,
}

pub struct AdvancedUserSearchCriteria {
    pub username: AdvancedTextField,
    pub display_name: AdvancedTextField,
    pub bio: AdvancedTextField,
}
pub struct UserSearchRequest {
    pub text: Option<String>,
    pub advanced: Option<AdvancedUserSearchCriteria>,
    pub sort: UserSearchSort,
    pub page: u32,
    pub page_size: u32,
}

pub struct AllSearchRequest { pub text: String, pub section_limit: u32 }
pub enum RecipeSearchVisibility { AllVisible, MineOnly, PublicOnly }
pub enum RecipeSearchSort { Relevance, UpdatedNewest, UpdatedOldest, TitleAscending, TitleDescending, Quickest, Longest }
pub enum CollectionSearchSort { Relevance, UpdatedNewest, UpdatedOldest, NameAscending, NameDescending }
pub enum UserSearchSort { Relevance, UsernameAscending, DisplayNameAscending }

pub struct SearchResult {
    pub recipe: Recipe,
    pub relevance: Option<f32>,
}
pub struct CollectionSearchResult { /* collection summary, breadcrumb, relevance */ }
pub struct UserSearchResult { /* public user summary, relevance */ }
pub enum SearchScope { All, Recipes, Collections, Users }
pub struct SearchResponse {
    pub scope: SearchScope,
    pub recipes: Vec<SearchResult>,
    pub recipe_total_count: i64,
    pub collections: Vec<CollectionSearchResult>,
    pub collection_total_count: i64,
    pub users: Vec<UserSearchResult>,
    pub user_total_count: i64,
}
pub struct TagFacetBucket {
    pub tag: String,
    pub count: i64,
}
pub enum RecipeVisibilityFacetValue { Private, Unlisted, Public }
pub struct VisibilityFacetBucket {
    pub visibility: RecipeVisibilityFacetValue,
    pub count: i64,
}
pub struct IntegerRangeFacetBucket {
    pub lower_inclusive: Option<i32>,
    pub upper_exclusive: Option<i32>,
    pub count: i64,
}
pub struct RecipeSearchFacets {
    pub tags: Vec<TagFacetBucket>,
    pub visibility: Vec<VisibilityFacetBucket>,
    pub total_time: Vec<IntegerRangeFacetBucket>,
    pub servings: Vec<IntegerRangeFacetBucket>,
}
pub struct RecipeSearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: i64,
    pub facets: RecipeSearchFacets,
}
pub struct CollectionSearchResponse {
    pub results: Vec<CollectionSearchResult>,
    pub total_count: i64,
}
pub struct UserSearchResponse {
    pub results: Vec<UserSearchResult>,
    pub total_count: i64,
}
```

### Query semantics

The backend owns each scope's SQL. Simple text is tokenized as ordinary input and compiled through the entity's tuned multi-field profile; the client never supplies field names, operators, boosts, or SQL fragments.

The recipe query profile interprets a simple query such as `pasta carbonara` conceptually as:

```text
AND(
  OR(title:pasta^3, description:pasta^2, ingredients:pasta, instructions:pasta),
  OR(title:carbonara^3, description:carbonara^2, ingredients:carbonara, instructions:carbonara)
)
```

The backend owns this boolean shape and binds every value. Ngram aliases handle partial title/description matching; word-tokenized aliases, including recursively extracted ingredient and instruction string values, handle term matching. Exact phrases are requested only through a typed advanced field criterion and use its phrase-capable word alias. Caller input is never treated as raw ParadeDB query syntax.

The fixed request fields define structured behavior: `tags_all` uses AND, `advanced.tags_any` uses OR, and `advanced.tags_excluded` uses NOT EXISTS. Populated positive advanced fields and simple text combine with AND. Visibility, time, servings, and date ranges are parameterized hard predicates.

Use a shared filtered-recipe CTE or equivalent base query for results, counts, and facets. The results and counts must not join tags in a way that multiplies recipe rows.

Recipe facets are always returned as the fixed fields of `RecipeSearchFacets`. Four explicit backend facet queries retain the same text and advanced criteria. The tag query also retains all tag criteria; the visibility, total-time, and servings queries omit only their respective filter. Clients cannot request custom facets or alter this behavior.

## Out of scope

- Searching `recipe_drafts`
- Searching private or unlisted recipes owned by other users
- Semantic/vector search
- Search analytics, saved searches, or alerts
- Search within a collection only
- New recipe fields or tag taxonomy changes
- Fuzzy typo correction beyond the selected ngram behavior
