# Recipe names

A user can save one recipe per normalized name. The rule covers public, private
and unlisted recipes. Other users can use the same name. Case, leading/trailing
whitespace and repeated whitespace do not distinguish names; display spelling is
preserved. The PostgreSQL key is
`lower(btrim(regexp_replace(title, '[[:space:]]+', ' ', 'g')))`.

Drafts and immutable historical snapshots do not reserve names. Single-recipe
imports and forks remain editable drafts. Publishing a new version can retain
its own name. Renaming or restoring an old name conflicts if another saved
recipe now owns it. Deleting or renaming a recipe releases its previous name.

## Collision handling

The database unique index is the sole authority for name uniqueness. There are
no preflight name queries, availability endpoint, background editor checks or
name-based publication disabling.

When a create or rename violates this specific index, the database layer maps it
to a recipe-name conflict. The API returns the clear message: "You already have
a recipe with this name. Choose a different name to publish." The editor displays
that confirmed failure and permits renaming and retrying. Other database errors
retain their existing handling.

The publication transaction rolls back on a collision, preserving the draft,
media references and history. Unknown write outcomes still use the existing
recovery behavior. Collection imports show an explicit collision message and
roll back the whole import; rename the conflicting recipe in the file to retry.

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
