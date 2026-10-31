# Recipe search count consolidation

The Recipes tab could exceed its five-second database deadline with the local
212-recipe demo catalog. The default All search and title suggestions use lighter
queries and did not reproduce that failure.

The original operation issued 12 sequential queries: results, total count, tags,
visibility, four time buckets and four servings buckets. EXPLAIN ANALYZE confirmed
that the ParadeDB index was used. A representative count spent 274–380 ms planning
and 45–56 ms executing. Disabling JIT in a diagnostic session did not remove the
planning cost. Repeating index planning and matching for every bucket exhausted
the shared deadline; refreshing table statistics did not resolve the timeout.

The operation now issues two queries: the existing paginated result query and
one summary query. A materialized CTE contains only authorized matching recipe
IDs, visibility, time, servings and selected-filter flags. Conditional aggregates
calculate the total and all numeric/visibility buckets. A second CTE read joins
tags for the selected result set. Text matching runs once in the summary.

Each facet still omits only its own selected filter. Authorization, tags, dates
and advanced text criteria always apply. Unknown time/servings remain in ordinary
results and are excluded from range buckets. Empty results return zero counts and
an empty tag list. The API response, pagination and five-second deadline are
preserved; no schema or index changes are required.

Real authenticated API requests against the same local catalog, measured without
concurrent builds or integration fixtures, produced these results:

| Query | Before | After |
| --- | --- | --- |
| `chicken` | 4.99–5.02 s, barely succeeded | 0.86 s, succeeded twice |
| `Chicken curry` | 5.03 s, timeout | 1.55 s, succeeded |
| A nonmatching title | 5.03 s, timeout | 1.02 s, succeeded with zero results |

Successful baseline responses and the optimized responses had identical counts,
facets and result ordering for `chicken`. These are local diagnostic timings,
not a production latency guarantee.

The pinned-ParadeDB integration suite verifies existing search behavior and a new
regression covers exact range boundaries, null values, ownership, empty matches,
and one shared indexed scan in the summary's EXPLAIN ANALYZE plan:

```sh
cargo test --locked --no-default-features --features server \
  search_integration_ -- --ignored --test-threads=1
```
