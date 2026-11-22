# NOMS-016 — image-free JSON and optional image files

Historical contract. New exports use the [v3 reference-preserving contract](NOMS-016-json-portability-v3.md); v2 files remain importable.

The user accepted the clipboard-import milestone, approved JSON portability, and then requested that image URLs be excluded from recipe JSON. Images are downloaded separately when explicitly selected and added manually after re-import.

## User workflow

Choose **Export recipe** on a saved recipe. The dialog defaults to **Download JSON**. Its optional **Download saved images too** checkbox changes the download to one ZIP containing `recipe.json`, `README.txt`, and saved full-size WebP files under `images/`.

JSON never carries structured image URLs, asset IDs, filenames, image bytes or edit metadata. Recipe image arrays and every nested step-photo array are empty compatibility placeholders, regardless of whether the source has images. Authored text, including any links deliberately written in Notes, is preserved. Images are outside the v2 content contract; this is a text-content round trip rather than a lossless image backup.

On **Import recipe**, choose the JSON file and click **Import JSON as private draft**. The importer creates a separate private draft and opens the normal editor. When using a ZIP, extract `recipe.json` first. Add desired image files manually with the editor's existing upload controls. Import never publishes or overwrites a recipe, and file selection alone saves nothing.

## File contract

The required envelope fields are `format: "noms.recipe.json"`, `version: 2`, `snapshot_schema: "noms.recipe.snapshot.v2"`, generator name/version, UTC export timestamp, `scope: "recipe"`, portable source recipe identity, exported author username, `image_mode: "none"`, an empty `missing_fields` array, snapshot digest and snapshot. The v2 schema deliberately excludes image content; the image mode declares that omission rather than implying preservation of the source's images.

The snapshot retains title, description, commentary, numeric times and servings, ingredients, nested instructions, equipment and tags. Null and empty values stay explicit. Tags are sorted/deduplicated for export; all meaningful content order remains intact. Image arrays are always empty, including photos on recursively nested steps. The SHA-256 digest covers RFC 8785 JCS snapshot bytes, so adding/removing source images does not affect the text snapshot digest.

New v2 files containing images are rejected, along with unsupported versions, missing/unknown fields, invalid checksums, unsupported scalar values, excessive nesting and application-limit violations. Files are limited to 2 MiB. Previously generated v1/reference-mode files are still validated against their original checksum and shape, then all image references are discarded before private-draft creation. Neither old nor new JSON imports fetch images, query source-recipe access or attach managed media.

Portable source identity and exported author remain envelope metadata; persistent origin provenance and duplicate/conflict handling are still follow-ups. Existing source attribution already authored in Notes is preserved unchanged. Future NOMS-017 fields require an explicit schema-version decision.

## Optional ZIP

`GET /api/recipes/{id}/export-images` applies the same private-owner/public/unlisted read policy as JSON export. It reads saved content and tags together in a read-only repeatable-read transaction. Drafts and version history are excluded. Only current recipe images and nested step photos are considered.

The server reads objects from the configured Noms bucket using canonical, validated recipe-full object keys. It does not fetch external URLs, avatars, thumbnails, unrelated storage objects, or unpublished draft media. Repeated saved objects are included once. Controlled names such as `images/recipe-001.webp` and `images/step-1-1-photo-001.webp` indicate original placement without exposing URLs or storage identities. Files are stored full-size WebP images; non-destructive editor crops/adjustments are not baked in.

The dialog and ZIP README disclose that externally linked images are omitted. Missing or invalid saved files fail the ZIP clearly; users can still choose JSON only. Downloads are bounded to 10 MiB per image, 32 MiB per archive, 60 seconds total, the existing request-rate budget and four concurrent expensive reads. Reads check both declared and actual streamed sizes. Responses use `application/zip`, an attachment filename and `Cache-Control: private, no-store`.

No database migration is needed. JSON import uses the ordinary new-private-draft path; the earlier source-media inheritance machinery has been removed.

## Verification scope

Focused tests cover image-free recipe and nested-photo JSON, v1 compatibility without attachment, strict v2 rejection of embedded image references, checksums, field/size/depth limits, saved-versus-draft content, owner/public/unlisted and anonymous ZIP access, unique image files, no external fetching, ZIP readability, missing images, bounded reads, and JSON-only fallback. Browser acceptance exercises both actual downloads, manual image upload, JSON re-import without images, and synthetic-content cleanup. Broader archive formats, historical/collection exports, duplicate handling and persistent origin provenance remain separate work.
