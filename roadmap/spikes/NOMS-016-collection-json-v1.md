# NOMS-016: Collection JSON round trips

Collection owners can choose **Export JSON** on a collection page. The file contains that collection and its descendants, their sibling order and ordered recipe memberships, and one canonical v3 snapshot for each distinct saved recipe. Empty collections and recipes shared by multiple collections are preserved. Draft content and historical versions are excluded.

**Import collection JSON** on the collection list first checks the file and shows its name and collection/recipe counts. A separate **Import private collection** action creates a new private collection tree and new private saved recipes owned by the signed-in account. Single-recipe JSON import continues to create a private draft. Collection import uses saved recipes because collection membership currently refers to saved recipes; no draft-membership schema is added.

Imported IDs are new. Source IDs are portable references, never authority to overwrite or attach to existing rows. A recipe shared by several imported collections is copied once and linked from each collection. Importing again creates another copy. If the root name already exists, the new root receives an ` (imported)` or numbered suffix within the existing name limit. Existing collections, recipes, visibility and drafts are kept.

## File contract

The envelope has `format: "noms.collection.json"`, `version: 1`, `snapshot_schema: "noms.collection.snapshot.v1"`, `scope: "collection_tree"`, generator/export-time metadata, a `snapshot_digest`, and a `snapshot` object. The digest is SHA-256 over the JCS-canonicalized entire snapshot. It detects accidental changes; it is not a signature or proof of permission.

The snapshot contains:

- `root_collection`: the root's `urn:noms:collection:<uuid>` identity.
- `collections`: a parent-before-child array. Every entry has `portable_id`, nullable `parent_collection`, `name`, nullable `description`, nullable `image`, and an ordered `recipes` array of portable recipe identities. Relative sibling order follows this array. The exported root has no parent even when it was exported from a nested collection.
- `recipes`: unique canonical `noms.recipe.json` v3 envelopes, including their existing snapshot digests and image references/edits. Every membership has a matching snapshot and every snapshot must be used.

Unknown/missing snapshot fields, unsupported versions, checksums that do not match, duplicate identities, duplicate memberships, disconnected/cyclic trees, invalid names, excessive depth and dangling/unused recipe snapshots are rejected. Recipe snapshots also pass the existing canonical format checks and saved-recipe publication validation. All imported access policies are private; the file cannot set visibility or ownership.

Limits: 8 MiB per file, 100 collections, 100 distinct recipes, 1,000 total memberships, 128 image placements including covers, and the existing ten-level hierarchy and recipe-field limits. Existing per-user collection, immutable-version and active-draft quotas still apply. Export refuses inaccessible recipe memberships or a limit violation rather than silently creating an incomplete backup. Export a smaller subcollection to reduce size.

## Images

JSON contains image references and edits, not image bytes. Recipe images follow the existing import rules: storage permissions are checked before a bounded existence check, safe external images are checked by the browser, and permissions are rechecked within the write transaction. Browser checks have a twelve-second batch deadline, four workers and a three-second per-image ceiling. Server checks share a separate twelve-second deadline; database permission checks and storage requests respect the remaining deadline. External URLs are never fetched by the server.

Accessible recipe images are reused. Unresolved images are excluded from the saved recipe and retained with their original placement and metadata in an owner-only edit draft. The success dialog links to each affected draft, where the user can retry, upload a replacement, or leave the image out. This review survives reloads. If the necessary review drafts would exceed the account's quota, the complete import rolls back rather than discarding recovery information. Text-only imports do not require new drafts.

Collection covers follow the existing uploader-only media policy. A retained cover can be reused by its uploader after an existence check. Covers that cannot be reused are omitted with a warning directing the user to Edit collection for a replacement. The original file retains the original reference.

## Authorization and atomicity

Export requires the collection owner and reads collection metadata, memberships, saved recipe content and tags within one repeatable-read, read-only transaction. Recipe access is checked independently of collection ownership. Reads use bounded batch queries and existing owner/membership/recipe indexes; recipe bodies are size-checked before loading them.

File checking and import require an authenticated session bound to the form's initiating account. Mutation authority is derived from the verified server session. Import acquires the existing owner lock, enforces quotas, creates recipes and their initial versions/tags, creates any image-review drafts, and restores the collection tree and memberships in one transaction. Late SQL errors roll everything back. Existing mutation/read rate limits, the expensive-work admission limit, the five-second database budget and the HTTP body limit remain in force.

A timeout or lost response can leave the client uncertain whether a commit completed; the error asks users to check their collections before retrying. The format does not implement deduplication or an exactly-once request key. Successful repeated imports intentionally create separate private copies.

## Verification

Ten focused collection tests pass, covering canonical content, nested/empty collections, order, shared membership, fresh/private copies, root-name collisions, checksums and invalid shapes, access revocation, uploader-only covers, image reuse/recovery, quota behavior and rollback after a deliberately late database failure. Fifteen existing recipe JSON tests and twelve media tests also pass after sharing the image-preparation logic. No schema migration or dependency version change is needed.

`EXPLAIN (ANALYZE, BUFFERS)` on the isolated fixture confirmed owner and membership index access for collection/membership reads. The planner used a small-table sequential scan for recipe selection. These are query-plan checks on synthetic data, not a production-scale performance claim; no additional index was justified.

Chromium exercised the real app with isolated synthetic accounts: owner export, anonymous export gating, damaged-checksum rejection, preview without writes, switched-account refusal, private import, actual import/re-export equality of recipe snapshots and hierarchy/memberships, repeated-import naming, and image recovery through an edit-draft reload. A virtual-clock check also exercises the actual image-check script with stalled and successful image loads to verify the batch deadline. Desktop and 390px dialog captures were inspected; screenshot files are kept outside the repository. Server/WebAssembly lint, formatting and fullstack build results are recorded in the PR. This is focused verification; other browsers and a physical phone were not tested.
