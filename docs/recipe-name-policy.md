# Recipe names

A user can save one recipe per normalized name. The rule covers public, private
and unlisted recipes. Other users can use the same name. Case, leading/trailing
whitespace and repeated whitespace do not distinguish names; display spelling is
preserved. The PostgreSQL key is
`lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g')))`.

Drafts and immutable historical snapshots do not reserve names. Imported and
forked recipes remain editable drafts: if their title conflicts, choose another
title before publishing. Publishing a new version can retain its own name.
Renaming or restoring an old name conflicts if another saved recipe now owns it.
Deleting or renaming a recipe releases its previous name.

Publication validates the authenticated owner's name inside the existing write
transaction. A unique owner/name index also protects concurrent and direct SQL
writes. A conflict returns: "You already have a recipe with this name. Choose a
different name to publish." Publication rolls back, preserving the editable draft,
its media references and the existing version history.

## Existing databases

Run `scripts/recipe-name-conflicts.sql` read-only against the migration database
before deployment. Resolve every reported group through the app by renaming a
recipe or deleting an unwanted duplicate. Do not rewrite immutable versions.
Neither the migration nor deployment automatically renames or deletes recipes.

The declarative schema includes the index; `migrations/recipe-owner-title-unique.sql`
contains the equivalent additive migration for explicit application. Creating the
index fails if unresolved duplicates remain, preserving existing data. Drafts
need no cleanup. Test fixtures use distinct names per owner and exercise shared
titles using different owners.

## API and editor

The authenticated `recipe_name_available` API checks only the verified account.
An optional recipe ID must belong to that account and allows an unchanged name
during edits. Requests validate title length and use autocomplete rate limits.

Both editors debounce checks, ignore stale account/title/recipe responses, show
inline feedback linked to the name field, and block publication while checking
or when a conflict is known. Draft saves remain available. A failed advisory
check permits publication through the authoritative server check. Confirmed
publication conflicts permit renaming and retrying; unknown write outcomes retain
the existing recovery behavior. Collection imports reject conflicting names with
an explicit message and roll back the whole import.
