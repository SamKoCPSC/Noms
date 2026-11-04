# Almost-cookable library recipes

The Pantry's What can I cook? section has Can cook now and Almost cookable views.
The latter includes recipes missing one or two distinct required ingredient
names. It sorts by fewest missing, then recipe update time descending, then
recipe UUID descending. Each card names its missing requirements. Optional
shortages are shown separately and never affect eligibility or rank. Complete
and optional-only-shortage recipes remain in Can cook now.

Candidates remain owned recipes and accessible recipes saved in the viewer's
collections, deduplicated across nested memberships. Unsaved public discovery
recipes, other users' private recipes, drafts, empty or malformed ingredient
lists are excluded. Case-insensitive name matching, explicit availability,
required-over-optional duplicate precedence and authored display spelling are
shared with recipe-detail checks. Quantities, units, equipment, modifiers and
substitutions do not affect this presence-based check.

## API and pagination

The existing authenticated find_pantry_recipes endpoint accepts optional mode:
cookable (default) or almost_cookable. A missing mode retains the old behavior.
The account ID is a consistency guard; authorization derives from the verified
session and owner-scoped pantry query. Result, missing-name metadata and card
hydration use one read-only repeatable-read transaction.

PantryRecipePage adds mode and required_ingredients_missing, a map from
returned recipe UUID to one or two authored names. Optional-shortage counts
remain separate. Pages contain at most 12 bounded cards and no full ingredient
documents. Names are capped at 500 characters; the whole serialized page is
capped at 512 KiB. Cursor inputs retain their 512-byte limit and are bound to
pantry, revision and mode. The rank includes the missing-required count, update
time and UUID; cookable cursors from the previous version remain valid only in
the default view.

Almost-cookable ranking scans the authorized library's lightweight names/flags,
retaining only the best 13 candidates for one page and lookahead. This avoids
retaining all matches or fetching complete recipes. Unlike the default view,
global ranking cannot stop after the first 13 matches in update-time order.
Both paths retain the existing per-user budget, expensive-read semaphore and
aggregate five-second deadline. A timeout returns retry feedback, not a partial
ranking. No schema changes or indexes are needed for this read path.

## Client freshness

Changing views resets pagination and unmounts the old request scope. Pending
reads hide old cards. Pantry revision changes, edits and Retry recipe check
reload results; the selected readiness view survives inventory updates.
Account changes hide private results through the existing binding guard.
Reload reflects external recipe, membership and visibility changes.

## Verification

Regression tests cover distinct required names, optional-only exclusions,
duplicate precedence, malformed data, global rank and stable multi-page
ordering, mode/pantry/revision cursor isolation, ownership/library visibility,
and promotion to Can cook now when an ingredient becomes available. Browser
verification exercises the real authenticated API and both readiness views.
