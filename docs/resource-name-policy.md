# Collection, pantry and grocery-list names

Each user can have one collection, one pantry profile and one grocery list per
normalized name. These are separate namespaces; another user can use any of the
same names. Collections reserve names across the owner's entire hierarchy,
regardless of parent folder or visibility. Inventory entry names are unchanged.

As with recipe names, PostgreSQL ignores case, surrounding whitespace and
repeated whitespace using
`lower(btrim(regexp_replace(name, '[[:space:]]+', ' ', 'g')))`.
Display spelling is preserved. Renaming to the same normalized name is allowed;
renaming or deleting a resource releases the old name.

The database unique indexes are authoritative, including direct and concurrent
writes. There are no name-availability queries or background client checks.
Only violations of the relevant indexes become typed name conflicts; the API
owns friendly collision messages. Create and rename forms retain inputs and
permit a new name after a confirmed rejection. Unknown write outcomes keep the
existing frozen, idempotent retry behavior.

Pantry duplication uses the existing "name copy" convention. If that name
already exists, rename the existing copy before duplicating again, or create a
pantry with another name. Collection imports roll back on a name conflict,
including repeated names across folders within the import. The existing imported-root
copy naming remains; its name search now covers all folders.

## Existing databases

Run `scripts/resource-name-conflicts.sql` read-only before deploying. Resolve
reported conflicts through the app by renaming or deleting unwanted resources;
no contents or names are changed automatically. Existing collections may share
a name across different folders, so audit the full hierarchy.

The declarative schema adds three owner/name unique indexes. The equivalent
additive migration is `migrations/resource-owner-name-unique.sql`; it creates
all three atomically and fails on unresolved duplicates. The existing weaker
collection sibling-name index is retained to keep this deployment additive.
Isolated database fixtures apply the same migration.
