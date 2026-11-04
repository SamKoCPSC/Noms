# Public Search

Search is available at /search without signing in, including entry from the
landing page and navigation search bars. The existing query, scope, advanced
filters, pagination and title-completion contracts are retained.

Anonymous viewers can search public recipes, public collections and public
profile fields. A signed-in viewer can additionally search their own private
and unlisted recipes and collections. Unlisted content is not publicly
discoverable. The Mine only recipe filter returns no results for a guest.

The server derives the optional viewer from the verified session. Missing,
invalid, expired or revoked sessions use the public result set. Client input
cannot select an owner or viewer. Every result, count, facet and suggestion
query applies the same public-or-owner predicate before projection.

Collection search keeps its minimal summary projection. Breadcrumb traversal
stops at the first inaccessible parent, including private and unlisted parents;
a public descendant remains independently discoverable. Search does not add
member recipe counts, hidden recipe metadata or private ancestor identifiers.

Authenticated requests use the existing per-user budgets. Anonymous requests
use the client IP pinned by the trusted request middleware; client-supplied
forwarding/internal headers cannot select an anonymous budget identity. Missing
trusted identity fails closed. Search/autocomplete retain their distinct rate
windows, shared work admission, database deadlines, pagination and output
ceilings. User and collection search also apply the existing 64-term aggregate
ceiling across simple and advanced fields/exclusions.

Client results and suggestions are associated with both query and viewer.
A response belonging to a prior account or query is hidden, and account
changes restart the request. Recipe title completion already follows this
same audience rule.

The schema and indexes are unchanged. Database regressions cover anonymous
visibility, authenticated ownership, counts/facets, pagination, ancestor
privacy and all suggestion scopes. The pinned ParadeDB suite exercises text
matching and title suggestions against the real search index.
