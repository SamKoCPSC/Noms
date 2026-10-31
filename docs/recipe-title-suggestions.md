# Live recipe-title suggestions

Search fields suggest query text from the current recipe title index. Selecting
a suggestion fills the field and submits that text through its normal search
action. It never selects a particular recipe. Multiple matching recipes can
therefore appear in the results, even when they share one title.

## API

The Dioxus server function is `suggest_recipe_titles(text, public_only)`.
It returns a JSON array of at most eight distinct strings:

```json
["Chicken curry", "Chicken curry with spinach"]
```

No recipe IDs, authors, counts, images or other recipe metadata are returned.
The server resolves the viewer from the session; callers cannot supply a user
ID. Anonymous requests see public recipes. Signed-in callers can see public
recipes and their own recipes, including their private/unlisted recipes.
`public_only: true` restricts even a signed-in caller to public titles, which
keeps suggestions consistent with landing-page and Explore results. Other
users' private/unlisted titles never enter deduplication or ranking. Drafts
are not part of the published recipe index.

Requests require 2–256 trimmed Unicode characters and use the existing search
term limit, per-user/per-IP autocomplete rate limit, search concurrency limit
and database execution deadline. Failures use the normal server-function error
contract. The UI keeps ordinary search submission available on suggestion errors.

## Matching, ranking and freshness

One parameterized SQL query searches only the existing ParadeDB title word and
ngram fields. Description, ingredient and instruction text cannot produce a
title suggestion. No schema/index migration or seeded title data is needed.

Titles are deduplicated by lowercase text with collapsed whitespace before
applying the result limit. A representative spelling comes from a matching
visible recipe. Ranking prefers an exact normalized title, a complete-title
prefix, query words matching word prefixes, then internal partial matches.
Within each tier, BM25 relevance ranks matches, with normalized-title ordering
as a deterministic tie-breaker. Multiple matching recipes produce one title
suggestion, while full search can still show all of those recipes.

The query reads live indexed recipes. Publishing, renaming, changing visibility
and deleting a recipe affect subsequent requests through the database's normal
index updates. There is no persisted browser title catalog or application-wide
suggestion cache. Submitted search still uses the existing result matching and
filters; the title-only restriction applies to suggestions.

## Client integration

The shared field is used by the desktop navigation search, mobile drawer,
recipe/all-scope search page, landing-page public search and Explore. Explicit
collection/cook scopes retain their existing entity suggestions.

After two characters, a 200 ms debounce coalesces typing. Resource cancellation
and response checks prevent an older query, audience or account response from
appearing in the current list. Arrow keys highlight suggestions; Enter submits
the highlighted title or the typed text. Mouse/touch selection submits the title.
Escape and Tab close the list without replacing the text. The field uses the
shared compact, visual-viewport-aware autocomplete surface and accessible IDs.

The main recipe/all-scope search bar submits on Enter, the search button or
suggestion selection, preserving its scope/filters and resetting pagination.
Selecting in the mobile navigation drawer also dismisses the drawer. Landing
search submits to public Explore and preserves literal `+`, `&` and `#` text.
Explore retains its existing live filtering behavior.

## Verification

```sh
# Requires SEARCH_TEST_DATABASE_URL pointing to the pinned ParadeDB image.
cargo test --locked --no-default-features --features server \
  search_integration_recipe_title_suggestions -- --ignored --nocapture

# Requires a running local mock-auth stack, Chromium and Python Playwright.
python3 scripts/tests/test-recipe-title-suggestions-browser.py \
  --instance noms-cloud --base-url http://127.0.0.1:PORT
```

The integration test checks index-backed execution with EXPLAIN ANALYZE, ranking,
deduplication before limiting, visibility, body-only exclusions, invalid input,
literal query values and live database changes. Browser checks exercise the real
API response, all search entry points, keyboard/mouse/touch submission, encoded
text, debounce, delayed-response rejection and error recovery. Synthetic local
recipe fixtures are removed when the browser check completes.
