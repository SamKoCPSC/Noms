# NOMS-016 — recipe JSON with reusable image references

The current single-recipe export keeps recipe image and nested step-photo URLs, placement, and edit metadata. The optional saved-image ZIP remains a backup for manual replacement when links cannot be restored.

## Workflow

Choose **Export recipe**, then **Download JSON**. Selecting **Download saved images too** downloads a ZIP with `recipe.json`, `README.txt`, and saved full-size WebP files. The ZIP includes each stored object once. External images are referenced in JSON but never fetched by the ZIP; crops and adjustments remain metadata rather than baked into image files.

Import validates the entire file before checking external images. It creates a separate private draft and never overwrites or publishes existing content. Accessible images are reused. **Imported images need attention** lists unresolved URLs and original placements; users can retry, upload replacement files with the editor's existing photo controls, or leave images out. Save pending editor changes before retrying or omitting a reference. Recovery information survives autosaves and reloads and is private to the draft; unresolved images are not included when publishing.

## Contract and compatibility

New files use `format: "noms.recipe.json"`, `version: 3`, `snapshot_schema: "noms.recipe.snapshot.v3"`, and `image_mode: "references"`. The existing generator/time, scope, source portable identity, author attribution, empty `missing_fields`, and RFC 8785 JCS/SHA-256 snapshot digest remain required. All authored content, image metadata, nulls, empty lists and nested steps retain their strict shape. Tags use sorted set semantics. Files are limited to 2 MiB and application content/depth limits.

The digest includes image references and edits. IDs and source attribution are metadata, never proof of access. Import accepts original v1/reference files and image-free v2/none files as well as v3; v2 files containing images are still rejected. No persistent cross-installation provenance or duplicate/conflict detection is introduced.

## Image checks and permissions

For Noms-managed images, the server matches canonical full-size URLs to the asset registry, verifies uploader permission or access to the source recipe's current published media, and checks the configured bucket with HEAD. File-supplied asset IDs cannot grant authority. Private source images, unrelated assets, historical-only assets, and unpublished draft images cannot be inherited through the source reference. Permissions and asset lifecycle are checked again under transaction locks when attaching media. Retained draft references use the existing media lifecycle, so removing the source does not remove an imported copy's attachments.

Confirmed storage 404s are reported as missing. Timeouts and other storage failures say the image could not be accessed. Permission failures do not reveal whether an inaccessible object exists. Checks have a three-second per-object timeout, twelve-second aggregate storage budget, bounded database work and the existing four-slot expensive-read admission limit.

External HTTPS image URLs are checked by browser image decoding with no referrer, four workers, and three-second timeouts. The server never fetches arbitrary imported URLs or follows their redirects. Active schemes, credential-bearing URLs, IP literals and obvious local hostnames are not eligible for reuse. Browser failures (including blocking, expired links and timeouts) remain **could not access**, since the browser cannot reliably distinguish a missing image from other failures. Foreign asset IDs and thumbnails are cleared before external reuse. External references remain remote resources, not managed copies or guaranteed backups.

## Draft recovery and persistence

Recovery metadata is stored under `_import_image_review` in the existing draft JSONB. It is not recipe content, never enters a published snapshot or export, and requires no schema migration. Autosave preserves it. Retry and omission are owner-bound, revision-guarded transactions; retry rechecks the original stored reference rather than accepting replacement image metadata from a request. A changed destination step requires manual placement rather than attaching an old photo to unrelated text. Saved content is protected while the UI applies a retry result.

The optional ZIP retains its existing read authorization, 10 MiB per-image/32 MiB archive limits, 60-second timeout and private/no-store caching. Missing files can fail the ZIP while JSON export still works. Historical/collection exports, other formats, origin provenance and duplicate handling remain separate work.
