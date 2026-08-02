# NOMS-012: Recipe Search — Implementation Plan

**Issue:** [NOMS-012-recipe-search.md](../issues/NOMS-012-recipe-search.md)
**Created:** 2026-06-10
**Revised:** 2026-08-01
**Depends on:** NOMS-008 current recipe model; NOMS-011 collection hierarchy for collection search
**Coordination:** NOMS-010 may be implemented in parallel. NOMS-012 searches published `recipes` rows, owner-only NOMS-011 collections, and public user profiles; it does not search `recipe_drafts`.
**Approach:** Establish the shared request contract and ParadeDB-capable test path first, then deliver recipe, user, collection/All, and autocomplete as complete backend operations. Add URL-backed simple and bounded advanced forms after those operations are stable, followed by UI integration and performance verification.

---

## Implementation contract

### Search corpus and authorization

- The default recipe corpus is the current user's recipes at any visibility plus other users' `public` recipes.
- Other users' `unlisted` and `private` recipes are excluded from global search and autocomplete.
- `Mine only` returns every recipe owned by the current user. `Public only` returns only rows with `visibility = 'public'`.
- Drafts are excluded because NOMS-010 stores them in `recipe_drafts`; no `is_draft` filter or draft index is introduced.
- Collection search returns only collections owned by the current user. Nested collections are searchable and results include breadcrumb paths.
- User search indexes and returns public profile fields only: `username`, `display_name`, `bio`, and avatar metadata where appropriate. Email, OAuth, session, and other private account fields are never indexed or returned.
- Search scopes are `All`, `Recipes`, `Collections`, and `Users`. `All` returns grouped sections with independent per-entity ranking; scores from different indexes are never compared.
- The client calls fixed scope-specific server functions with `RecipeSearchRequest`, `CollectionSearchRequest`, `UserSearchRequest`, or the simple-text-only `AllSearchRequest`. It cannot submit a query language, expression tree, field name, SQL fragment, boost, or ParadeDB syntax.
- Server functions derive authorization from the authenticated session. They never accept a caller-supplied user or owner ID.

### Simple and advanced text semantics

- Simple `text` is ordinary literal input. The backend tokenizes it through a fixed entity profile, requires every token across that entity's searchable fields, and permits different tokens to occur in different fields.
- Quotes, colons, `AND`, `OR`, `-`, `*`, and similar parser-looking input have no caller-controlled query-language meaning.
- Advanced search is a bounded scope-specific form. Each statically named field has at most one positive `TextCriterion` and a bounded list of exclusions. Match modes are fixed to `AllTerms`, `AnyTerm`, and `ExactPhrase`.
- Simple text and every populated positive advanced field combine with `AND`. Each exclusion rejects rows matching that exclusion in its assigned field.
- Recipe advanced criteria cover title, description, ingredients, instructions, tag-any/tag-excluded sets, and created/updated ranges. Visibility, tag-all, total time, and servings remain top-level recipe controls.
- Collection advanced criteria cover name, description, and updated date. User advanced criteria cover username, display name, and bio. `All` accepts literal text only.
- Simple text and positive advanced text contribute relevance. Exclusions and structured filters only include or exclude rows.
- With no positive text, relevance is `None`; `Relevance` normalizes to `UpdatedNewest` for recipes/collections and `UsernameAscending` for users.
- Blank `All` text is a valid bounded browse state. Recipe and collection sections use `UpdatedNewest`; the user section uses `UsernameAscending`; every section retains its deterministic ID tie-breaker.

### Initial request limits

Define these once as shared search-validation constants used by server validation, URL parsing, UI controls, and tests:

| Input | Initial limit |
|---|---|
| Simple `text` or one advanced text criterion | 256 Unicode scalar values |
| Exclusions | 5 criteria per statically defined text field |
| Tags | 100 Unicode scalar values per normalized tag; 25 distinct tags across all tag lists |
| Scoped page | Default 20, maximum 100 rows; 1-indexed with calculated offset at most 10,000 |
| `All` sections | Default 5, maximum 10 rows per entity section |
| Advanced URL state | 4,096 decoded bytes and 8,192 URL-encoded bytes |
| Autocomplete | Minimum 2 Unicode scalar values after trimming; maximum 8 suggestions |

Normalize tags before measuring them. Count the distinct union of `tags_all`, `tags_any`, and `tags_excluded`; reject a tag that appears in a positive list and `tags_excluded`. Reject oversized requests rather than truncating criteria or tags.

`All` autocomplete allocates at most 4 recipe, 2 collection, and 2 user suggestions in fixed entity-group order. Unused slots remain empty rather than being reassigned through cross-index score comparison. Scoped autocomplete may use all 8 slots.

### Recipe fields, ranges, and tags

Use the current fields `user_id`, `visibility`, `title`, `description`, `ingredients`, `instructions`, `prep_time_minutes`, `cook_time_minutes`, `servings`, `created_at`, and `updated_at`. Recipe tags live in `recipe_tags(recipe_id, tag)`. Do not use legacy recipe names such as `owner_id`, `is_public`, `is_draft`, `steps`, `total_time_min`, `prep_time_min`, or `cook_time_min`; NOMS-011 collections intentionally use `owner_id`.

Total time is:

```sql
COALESCE(r.prep_time_minutes, 0) + COALESCE(r.cook_time_minutes, 0)
```

A total-time predicate also requires at least one time column to be non-null. Servings predicates exclude null servings.

`tags_all` has AND semantics and uses duplicate-safe `EXISTS` predicates. Tag joins must not multiply result or count rows. The tag facet retains current tag criteria: an unselected bucket reports the result count after additively selecting that tag, selected tag-all buckets equal the current result count, and zero-count tags are omitted. Visibility, total-time, and servings facets omit only their own filter while retaining authorization and every other criterion.

Facet buckets are fixed:

| Facet | Buckets | Ordering and zero counts |
|---|---|---|
| Tag | Dynamic normalized tags | Count descending, tag ascending; omit zero counts; cap at 50 |
| Visibility | `private`, `unlisted`, `public` | Canonical enum order; include zero counts |
| Total time | `[0,30)`, `[30,60)`, `[60,120)`, `[120,+∞)` | Lower-bound order; include zero counts; exclude both-null times |
| Servings | `[1,3)`, `[3,5)`, `[5,9)`, `[9,+∞)` | Lower-bound order; include zero counts; exclude null servings |

### Shared request and response shape

The concrete shared types may be split into smaller structs following repository conventions, but the client-facing capability remains bounded:

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
```

Recipe, collection, user, grouped, facet, and suggestion responses use typed shared structs rather than raw rows or arbitrary maps. `RecipeSearchResponse` always contains results, a distinct total count, and fixed `RecipeSearchFacets` fields for tags, visibility, total time, and servings. The grouped response contains independent entity sections and totals but no recipe facets.

### URL history behavior

- URL state is the source of truth for committed search state.
- Typing uses `replaceState`; Enter, suggestion selection, filter/sort changes, and pagination use `pushState`.
- Initial load and `popstate` rebuild local state from the canonical URL.

---

## Checkpoint 1: Foundation — shared contract, validation, and ParadeDB-capable tests

**Files:** `src/types.rs`, search validation module, `src/test_utils.rs`, `migrations/extensions.sql`, `.github/workflows/ci.yml`, `docker/postgres/Dockerfile`

Write pure validation tests first, then add the shared DTOs, response types, enums, defaults, and named limit constants described above. Validation must trim where documented, count Unicode scalar values consistently on client and server, use checked pagination arithmetic, normalize tags before conflict/count checks, reject empty or inverted ranges, and reject scope-inapplicable enum values.

Add a search-specific database test path that requires `pg_search` and fails explicitly if the extension or index cannot be created. It must use the repository's pinned PostgreSQL image rather than the normal `pgtemp` path, whose host PostgreSQL may not contain ParadeDB. Existing unrelated database tests remain independent of `pg_search`.

Add a dedicated CI search job that builds and starts `docker/postgres`, waits for readiness, applies `migrations/extensions.sql` and the declarative schema, and runs search integration tests against it. Update schema-plan CI to use the same extension-capable image whenever either migration file changes; do not apply the extension migration to plain `postgres:17`.

### Verify

- Boundary tests cover text, exclusions, normalized/distinct tags, positive/excluded tag conflicts, range validation, page size/offset, grouped limits, URL byte limits, and autocomplete allocation.
- Serialized requests cannot contain free-form field names, operators, boosts, authorization IDs, SQL, or ParadeDB syntax.
- Search test setup cannot pass after failing to create `pg_search` or a BM25 index.
- CI proves the pinned image starts and schema-plan CI no longer relies on plain PostgreSQL for extension-bearing migrations.
- `git diff --check` and the focused validation tests pass.

**Risk:** Medium. This checkpoint establishes the safety boundary used by every later operation.

---

## Checkpoint 2: Search indexes and minimal ParadeDB smoke queries

**Files:** `migrations/schema.sql`, search test setup, NOMS-011 schema integration when available

Add the recipe BM25 index after `recipes` and the user BM25 index after `users`. Add the collection index only after NOMS-011 provides `collections`.

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

CREATE INDEX IF NOT EXISTS idx_users_search
ON users USING bm25 (
    id,
    (username::pdb.simple('alias=user_username_words')),
    (username::pdb.ngram(2, 20, 'alias=user_username_partial')),
    (display_name::pdb.simple('alias=user_display_name_words')),
    (display_name::pdb.ngram(2, 20, 'alias=user_display_name_partial')),
    (bio::pdb.simple('alias=user_bio_words'))
)
WITH (key_field = 'id');
```

The collection index uses word and ngram aliases for `name` and `description`, matching the fixed collection criteria. Add `idx_recipe_tags_tag ON recipe_tags(tag)` because the existing primary-key order begins with `recipe_id` and does not serve tag-first facet access.

```sql
CREATE INDEX IF NOT EXISTS idx_collections_search
ON collections USING bm25 (
    id,
    (name::pdb.simple('alias=collection_name_words')),
    (name::pdb.ngram(2, 15, 'alias=collection_name_partial')),
    (description::pdb.simple('alias=collection_description_words')),
    (description::pdb.ngram(2, 15, 'alias=collection_description_partial'))
)
WITH (key_field = 'id');
```

The word aliases retain positions for `ExactPhrase`; ngram aliases provide partial title/name matching and are never used for phrase evaluation. Ingredient extraction recursively indexes string values. Instruction extraction indexes only nested `text` values, not keys, numeric metadata, or photo metadata.

The exact index syntax and prepared bound boost/phrase forms have already been runtime-probed against pinned ParadeDB 0.23.4. Implementation must turn those probes into repeatable smoke tests rather than reopening the index design.

### Verify

- Fresh extension/schema application creates recipe and user indexes; collection index creation is gated on its table.
- Smoke tests prove exact, partial, prepared boost, and positional phrase matching on their intended aliases.
- Nested instruction text is searchable while instruction photo metadata is not.
- Updating indexed source fields is visible without application-managed index maintenance.
- Representative `EXPLAIN` output uses the intended BM25 and supporting tag indexes.

**Risk:** High. The extension and index behavior must be stable before scope operations depend on them.

---

## Checkpoint 3: Recipe search operation

**Files:** server-only search database module, `src/api/search.rs`, `src/api/mod.rs`, `src/types.rs`, search integration tests

Implement one complete authenticated recipe operation:

```text
search_recipes(request: RecipeSearchRequest) -> RecipeSearchResponse
```

The server function validates the fixed request, derives the authenticated user, and calls an internal database function that accepts the resolved user ID. Use explicit recipe SQL and fixed helper functions for repeated bound clauses; do not build a generic field/operator/filter compiler.

The recipe operation must:

- apply the baseline visibility corpus before matching;
- implement literal simple text as AND across tokens, with each token OR'd across the tuned recipe fields;
- apply statically assigned advanced include/exclude templates for title, description, ingredients, and instructions;
- compute `pdb.score` whenever any positive simple or advanced text criterion exists;
- apply fixed relevance weights of 3x title, 2x description, and 1x ingredients/instructions to simple and positive advanced text templates;
- bind every caller value and map one fixed sort enum to SQL with `id ASC` as the final tie-breaker;
- apply duplicate-safe tag-all/tag-any/tag-excluded predicates, time/servings ranges, and created/updated ranges;
- derive page results, distinct total count, and all four fixed facets from the same validated request;
- retain all tag criteria for additive tag facets, while omitting only visibility, total time, or servings for those respective facets;
- return typed recipe rows with author attribution and no SQL details in controlled errors.

Prefer a fixed base recipe relation plus explicit result, count, and facet variants. Internal reuse is encouraged, but the abstraction must not expose arbitrary fields or logical structure.

### Tests and verification

- Authentication and own/public/private/unlisted authorization boundaries.
- Exact and partial matching, field boosts, cross-field all-term matching, every advanced mode, exclusions, and literal parser metacharacters.
- Ingredient/nested-instruction matches and photo-metadata exclusion.
- Tag AND/OR/excluded behavior without duplicate results or counts.
- Time/servings null rules, date ranges, all sort variants, relevance fallback, deterministic pagination, and limit validation.
- Results/count agreement and fixed typed facet counts, including additive tags, canonical zero-count ranges, and the three self-excluding facets.
- No untrusted SQL fragments appear in query construction.

**Risk:** High. This is the main correctness, authorization, ranking, and facet boundary.

---

## Checkpoint 4: User search operation

**Files:** server-only search database module, `src/api/search.rs`, shared types, search integration tests

Implement the complete authenticated user operation:

```text
search_users(request: UserSearchRequest) -> UserSearchResponse
```

Use explicit user SQL over only `username`, `display_name`, and `bio`. Simple text uses the fixed user field profile; advanced criteria bind only their statically assigned field aliases. Positive advanced text contributes relevance, and filter-only/relevance-less requests normalize to username ordering. Always append the deterministic ID tie-breaker.

Use fixed relevance weights of 3x username, 2x display name, and 1x bio. Word aliases handle terms and phrases; ngram aliases are used only for partial username/display-name matching.

Return a dedicated public `UserSearchSummary`. Do not select email, OAuth, session, refresh-token, or other private fields even into an intermediate row. If public recipe count remains in the summary, calculate it from public recipes only.

### Tests and verification

- Username/display-name partial and exact matching, bio terms and phrases, every fixed advanced mode, exclusions, sort fallback, pagination, and limits.
- User results contain only the public projection and public recipe count cannot reveal private/unlisted recipes.
- Parser-looking literal input cannot select a field or inject syntax.
- Result count uses the identical validated criteria as the result page.

**Risk:** Medium. The primary concern is preventing private account-field leakage.

---

## Checkpoint 5: Collection search and grouped `All`

**Files:** NOMS-011 collection schema/types, server-only search database module, `src/api/search.rs`, shared types, integration tests

This checkpoint starts after NOMS-011 is available in the integration base. Implement:

```text
search_collections(request: CollectionSearchRequest) -> CollectionSearchResponse
search_all(request: AllSearchRequest) -> SearchResponse
```

Collection SQL enforces `owner_id = authenticated_user_id`, searches only name and description, supports the fixed advanced collection fields, and returns root-to-leaf breadcrumb labels through the NOMS-011 hierarchy. It uses collection-specific sorts and deterministic ID tie-breaking.

Use fixed relevance weights of 3x collection name and 1x description. Word aliases handle terms and phrases; ngram aliases are used only for partial name/description matching.

`search_all` accepts literal text and a bounded section limit only. It runs lightweight fixed recipe, collection, and user section queries with independent ranking and total counts. Blank trimmed text uses the canonical per-section browse sorts rather than BM25. It must not call the public `search_recipes` path in a way that computes unused recipe facets, and it must not compare or merge raw BM25 scores across indexes.

### Tests and verification

- Root and nested collection matching, breadcrumb correctness, owner isolation, advanced collection fields, sorts, pagination, and count agreement.
- `All` returns grouped sections in canonical recipe/collection/user order with the requested bounded limit and correct per-entity totals.
- Blank `All` returns the deterministic grouped browse state and its canonical URL omits `q`.
- Other users' collections and non-public recipe/account data cannot enter `All`.
- `All` performs no recipe facet queries and no cross-index score ordering.

**Risk:** Medium. This checkpoint is gated by NOMS-011 and must be integrated against its actual hierarchy API.

---

## Checkpoint 6: Typed autocomplete operation

**Files:** server-only search database module, `src/api/search.rs`, shared suggestion types, autocomplete integration tests

Implement the authenticated bounded autocomplete operation. Recipe and user scopes may be completed immediately after Checkpoints 3–4; add collection suggestions and the complete `All` allocation after Checkpoint 5:

```text
autocomplete_entities(scope: SearchScope, text: String) -> Vec<SearchSuggestion>
```

The scope enum is the only caller-controlled routing choice. Each entity uses a fixed query and typed `SearchSuggestionKind`; no caller field, boost, or operator is accepted. Apply the same recipe visibility, collection ownership, and public-user projection rules as full search.

Require two Unicode scalar values after trimming. Scoped autocomplete returns at most 8 suggestions. `All` uses fixed grouped allocation and order: 4 recipes, 2 collections, 2 users, with unused slots left empty. Debounce is a client concern; limits and authorization are server-enforced.

### Tests and verification

- Minimum/maximum boundaries, Unicode counting, deterministic allocation/order, and stable tie-breaking.
- Recipe visibility, collection ownership, and user public-field projection match their full-search operations.
- Each suggestion kind contains only the ID, label, and safe secondary text needed for navigation.
- No score comparison or unused-slot refill occurs across entity indexes.

**Risk:** Medium. Authorization drift between autocomplete and full search would leak otherwise hidden entities.

---

## Checkpoint 7: Canonical URL state, route, and search UI

**Files:** `src/pages/search.rs`, route/auth integration, URL codec module, search components, `src/pages/home.rs`, navigation integration

Add the protected `/search` route after the backend operations are stable. The URL codec creates exactly one active scope request from these canonical keys:

| Key | Values |
|---|---|
| `scope` | `all`, `recipes`, `collections`, `users` |
| `q` | URL-encoded literal text |
| `sort` | one value from the active scope's sort enum |
| `tags` | normalized recipe `tags_all` values |
| `time` | `0-30`, `30-60`, `60-120`, `120+` |
| `servings` | `1-2`, `3-4`, `5-8`, `9+` |
| `vis` | `all`, `mine`, `public` |
| `advanced` | versioned URL-encoded advanced DTO for the active scope |
| `page` | positive 1-indexed page |

Omitted values receive documented defaults. Explicit malformed enums/ranges, invalid advanced versions/shapes, conflicting tags, and oversized state produce a controlled validation state and never reach SQL. Scope-inapplicable parameters are removed during canonicalization. When no positive text remains, canonicalize relevance to the scope's browse sort. For `scope=all`, blank trimmed text is canonicalized by omitting `q` and renders the deterministic grouped browse state.

Typing updates local state and uses `replaceState`; committed searches, suggestion selection, filters, sort, and pagination use `pushState`. Initial load and `popstate` parse the URL and reload the matching request. Guard browser APIs for SSR/WASM safety.

Build the page from focused components for input/autocomplete, bounded advanced form, fixed recipe filters/facets, typed results, and pagination. The advanced form exposes only the active scope's static fields and modes—never nesting, free-form fields/operators, boosts, SQL, or ParadeDB syntax. Sort controls are scope-specific. Recipe/user URL state and UI may proceed once their operations and autocomplete paths are stable; collection and complete `All` controls remain gated on Checkpoint 5.

The `All` view renders grouped entity sections. Recipe scope renders the fixed facets. Collection results show breadcrumbs; user results show public metadata only. Include accessible loading, error, empty, no-results, keyboard-autocomplete, filter-chip, and clear-all states.

Add a landing-page search bar that only navigates to `/search?q=...`; it does not own result state or call database functions. Keep landing autocomplete disabled unless unauthenticated search receives a separate design.

### Tests and verification

- URL parse/canonicalize/round-trip tests for every scope, advanced DTO version, size boundary, invalid state, filter reset, and relevance fallback.
- `replaceState` versus `pushState`, initial hydration, back/forward restoration, page reset, and scope switching.
- Advanced form produces one bounded scope request and preserves simple text.
- Autocomplete keyboard/mouse behavior and typed navigation.
- Correct scope-specific filters, sorts, facets, result projections, pagination, and visible UI states.
- Landing submission creates one canonical navigation without a second search state machine.

**Risk:** Medium. URL canonicalization and browser history require explicit client/SSR boundary tests.

---

## Checkpoint 8: Combined integration, migration safety, and performance

**Files:** all NOMS-012 files plus shared NOMS-010/NOMS-011 integration points

Run this checkpoint only after the preceding operations and NOMS-011 integration are present. Tests belong with the checkpoint that introduces behavior; this checkpoint reruns them together and concentrates on cross-scope regression, migration safety, runtime plans, and the complete user flow.

### Required verification

```text
just ci
just schema-check-test
just migrate-plan
just sqlx-prepare
```

Run the dedicated ParadeDB search integration job locally or in CI against the pinned image. Apply extensions and schema from a fresh database, inspect the schema plan for destructive operations, and confirm the offline SQLx cache is current when compile-time queries change.

Review representative `EXPLAIN` plans for recipe/user/collection text search, empty-query browsing, tag predicates, all four facets, autocomplete, and grouped `All`. Confirm index use, deduplication before pagination, bounded offsets, and the absence of unused facet work in `All`.

Start a fresh isolated application instance and manually verify authenticated simple search, bounded advanced search, filters/facets, all scopes, autocomplete navigation, URL back/forward, empty/error states, and existing recipe/collection/profile navigation. Coordinate shared route, home, header/sidebar, dashboard, and recipe-detail files without overwriting NOMS-010/NOMS-011 changes.

### Migration and rollback

- `migrations/extensions.sql` remains responsible for required extensions; `migrations/schema.sql` adds indexes declaratively after their source tables.
- No application backfill is required because BM25 indexes cover existing rows and remain transactionally maintained.
- If an index definition changes, build and validate its replacement before removing the old index in a separate safe migration step.
- Do not add destructive rollback SQL. Development data may be recreated during integration where the existing roadmap permits it.

### Exit criteria

- All checkpoint tests and CI gates pass against the combined feature base.
- Authorization and public projections agree across full search, `All`, and autocomplete.
- Results/count/facet relationships and URL/UI behavior remain deterministic.
- Performance plans use intended indexes and no unbounded or cross-index-ranked operation remains.
- No regressions are found in existing authenticated navigation and recipe/collection/profile flows.

**Risk:** High. This is the first verification against the complete combined schema and application.

---

## Dependencies and sequencing

| Checkpoint | Depends on | Execution note |
|---|---|---|
| 1 Foundation | Current recipe schema and pinned database image | Start here; completes before query/index implementation |
| 2 Indexes/smoke queries | Checkpoint 1 | Recipe/user work proceeds immediately; collection subtask waits for NOMS-011 |
| 3 Recipe operation | Checkpoints 1–2 recipe index | First complete searchable vertical slice |
| 4 User operation | Checkpoints 1–2 user index | May proceed alongside Checkpoint 3 |
| 5 Collection and `All` | Checkpoints 3–4 plus NOMS-011 and collection index | Explicit integration gate |
| 6 Autocomplete | Checkpoints 3–4 for recipe/user; Checkpoint 5 for collection/complete `All` | Implement recipe/user paths before NOMS-011 and extend the same fixed operation after collection integration |
| 7 URL/route/UI | Checkpoints 3–4 plus corresponding Checkpoint 6 paths; Checkpoint 5 for collection/complete `All` | Recipe/user client work proceeds before NOMS-011; gated scopes are added when their backend is stable |
| 8 Combined verification | All previous checkpoints plus NOMS-010/NOMS-011 integration | Final migration, performance, manual, and regression gate |
