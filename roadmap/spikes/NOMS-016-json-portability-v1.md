# NOMS-016 — single-recipe JSON portability v1

This is the first export slice. The user accepted the clipboard-import workflow after manual testing and approved moving on to simple JSON export and canonical JSON import.

## User workflow

On a saved recipe, choose **Export JSON**. The download contains the current saved recipe, including tags, nested instructions, equipment, recipe images and step photos with their edit metadata. Unsaved editor changes and private drafts are not exported. Export is available to anyone who can read that recipe: owners may export private recipes, and public/unlisted recipes retain their existing read policy.

On **Import recipe**, choose the file under **Or import a Noms JSON file**, then click **Import JSON as private draft**. File selection alone saves nothing. Validation failures leave the file selected and create no draft. A successful import opens a new private draft in the normal editor, with a new local identity. Publication is a separate action. The website clipboard workflow remains available above the file picker.

## File contract

The envelope has these required fields:

| Field | Value |
| --- | --- |
| `format` | `noms.recipe.json` |
| `version` | `1` |
| `snapshot_schema` | `noms.recipe.snapshot.v1` |
| `generator` | Noms name and application version |
| `exported_at` | UTC timestamp |
| `scope` | `recipe` |
| `portable_id` | `urn:noms:recipe:{saved-recipe-uuid}` |
| `source_attribution` | Exported recipe's author username |
| `image_mode` | `references` |
| `missing_fields` | Empty array for this lossless snapshot renderer |
| `snapshot_digest` | `sha256:` followed by lowercase SHA-256 of RFC 8785 JCS snapshot bytes |
| `snapshot` | Complete recipe content described below |

The snapshot contains `title`, `description`, `commentary`, `prep_time_minutes`, `cook_time_minutes`, `servings`, `ingredients`, `instructions`, `equipment`, `images`, and `tags`. There is no visibility or owner field. Nullable values are explicit JSON nulls, and empty values remain empty. Tags are sorted and deduplicated for export; ingredient, equipment, step, image and edit order remains meaningful. Nested steps always contain `text`, `sub_steps`, and `photos`. Images always contain `asset_id`, `url`, `thumb_url`, `file_name`, `file_size`, and `edits`, with explicit nulls where applicable. Image edits retain the application's typed geometry, colour and crop-shape fields.

The snapshot schema covers currently stored content only. Future NOMS-017 metadata requires an explicit schema-version decision; it must not be silently added to or discarded from v1. Import rejects unsupported envelope versions, scopes, image modes, missing or unknown snapshot/nested fields, damaged checksums, invalid values, excessive nesting, and application-limit violations. The file limit is 2 MiB and the existing recipe-content limit also applies. A checksum detects corruption; it does not establish trust, ownership or permission.

## Database and media boundaries

Export reads content and tags in one read-only repeatable-read transaction. It excludes drafts, histories, diffs, internal owner IDs, sessions, storage capabilities and operational records.

Import reuses the initiating-account check, mutation budget, recipe validation, draft quotas and atomic media-reference attachment. It never publishes or overwrites an existing recipe. No schema migration is required.

Image bytes are not included or downloaded. URLs and edit metadata remain in the snapshot. When a URL refers to another installation's HTTPS media, the source asset ID is cleared before persistence; it cannot identify a local managed asset. When a URL refers to this installation's managed media, the existing URL/asset checks and transactional attachment policy apply. An accessible source recipe permits reuse of its **current published** media only, never unrelated versions or unpublished draft assets. The source read is locked until draft creation commits. Inaccessible or invalid local media rejects the whole import without leaving a partial draft. Ordinary editor rendering can subsequently load accepted image references; this is not an image-byte archive or an availability guarantee.

## Deliberate first-slice limits

- Every explicit import creates a new private copy. Duplicate/conflict detection, retained cross-installation origin identity and persistent envelope provenance are follow-up work. Attribution already stored in recipe Notes remains unchanged; the exported author and portable identity remain in the downloaded envelope rather than being injected into authored Notes.
- Only current saved single recipes are exported. Drafts, historical snapshots, collections, lineage archives, other renderers and image-byte packaging remain planned.
- This is a content round trip for the currently persisted recipe model. It does not claim complete delivery of NOMS-016's wider portability plan.

## Verification

Focused regressions cover complete content round trips, explicit null/empty values, nested steps/photos/edits, deterministic tag/JCS hashing, corruption, unsupported versions, missing/unknown fields, scalar/size/depth limits, private/public/unlisted export reads, saved content versus drafts, foreign image IDs, unsafe URL schemes, readable-current media inheritance, and rollback after denied media attachment. Browser verification exercises the actual download and file-input paths separately from the existing clipboard smoke tests.


Final acceptance on 2026-10-16: 36 focused native tests passed. The browser confirmed damaged-file rejection without writes, complete snapshot/digest equality in the actual download, re-import into a separate private draft, reload, disabled pre-hydration file input and removal of synthetic test content. Existing application counts were restored. Optional external fonts were blocked in the browser harness; this was functional verification, not typography or cross-browser acceptance. Formatting and server/WebAssembly Clippy passed; the full CI suite was not rerun. The completed browser round trip additionally asserts that successful download shows no error alert; the JavaScript bridge explicitly returns success.
