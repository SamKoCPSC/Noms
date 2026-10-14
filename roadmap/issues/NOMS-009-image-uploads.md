# NOMS-009: Image Uploads

**Status:** ✅ Phase 1 Complete | ⏳ Phase 2 Planned  
**Phase:** Phase 2 (media)  
**Depends on:** NOMS-008 (Recipe CRUD)

## Overview

Add image upload capability for recipe images and step photos. Phase 1 established the data model and display UI using an `images` JSONB array. Phase 2 will add the full upload pipeline: file selection → upload → storage → URL → display.

### ✅ Phase 1 — Implemented

The `images` JSONB column on the `recipes` table, `Recipe.images` field, full DB layer support (7 SELECTs, INSERT, UPDATE, FromRow), API pass-through on `create_recipe` and `update_recipe`, and a neumorphic image slider on the recipe detail page are all implemented. Placeholder image URLs are used — no upload infrastructure exists yet.

### ⏳ Phase 2 — Planned

Upload infrastructure (R2 storage, file validation, image processing), upload UI (file picker, drag-and-drop, progress), and step photos.

## Context

NOMS-008 defines `steps[].photo_url` as a nullable string inside the steps JSONB array. NOMS-009 Phase 1 added `images JSONB NOT NULL DEFAULT '[]'::jsonb` to the `recipes` table and an image slider on the detail page. Phase 2 adds the full upload pipeline: file selection → upload → storage → URL → display.

## Acceptance Criteria

### AC1: Recipe images data model and display ✅ Done

- [x] `images JSONB NOT NULL DEFAULT '[]'::jsonb` column on `recipes` table (array of image URL strings)
- [x] `images: Vec<String>` field on `Recipe` struct in `src/types.rs`
- [x] All DB queries (7 SELECTs, INSERT, UPDATE) include `images` column
- [x] `FromRow` impl deserializes images via `serde_json::from_value`
- [x] API `create_recipe` accepts `images` parameter
- [x] API `update_recipe` accepts `images` parameter
- [x] Image slider component on recipe detail page below header card
- [x] Slider: neumorphic inset, left/right arrows, dot indicators, placeholder gradients
- [x] Slider: responsive viewport, `aspect-ratio: 16 / 9`
- [x] Slider arrows: glassmorphism-styled, conditionally hidden at boundaries
- [x] Slider dots: active state with accent color, only shown for multiple images
- [x] Placeholder/skeleton: gradient background when image URL is placeholder or empty
- [x] Graceful degradation: slider only renders when images vector is non-empty
- [x] Test schema (`src/test_utils.rs`) includes images column

### AC2: Storage backend setup ⏳ Planned

- [ ] Cloudflare R2 bucket created for recipe images
- [ ] S3-compatible credentials configured as environment variables: `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`, `R2_ENDPOINT`
- [ ] Rust S3 client (`aws-sdk-s3` or `reqwest` + presigned URLs) configured in application
- [ ] Upload endpoint: `POST /api/upload` accepts multipart file upload, returns signed URL
- [ ] Delete endpoint: `DELETE /api/upload/:key` removes image from storage
- [ ] File validation server-side: type (JPEG, PNG, WebP), max size (10 MB), dimensions (max 6000px on longest side)
- [ ] Uploaded files stored with UUID-based keys: `recipes/{recipe_id}/{timestamp}_{random}.{ext}`
- [ ] CDN URL returned for image access (Cloudflare R2 public bucket URL or presigned read URL)

### AC3: Recipe image upload UI ⏳ Planned

- [ ] Recipe create form includes image upload area at top (drag-and-drop + file picker)
- [ ] Recipe edit form allows adding, reordering, or removing images
- [ ] Image thumbnails shown on recipe card in dashboard and collection views
- [ ] Removing an image removes it from the `images` array and deletes image from storage
- [ ] Multiple images can be uploaded in sequence (not parallel batch upload — MVP)

### AC4: Step photos ⏳ Planned

- [ ] Each step row in recipe form includes optional photo upload area
- [ ] Photo upload replaces `photo_url: null` with actual URL in steps JSONB
- [ ] Step photos displayed inline with step text on recipe detail page
- [ ] Photos can be removed from individual steps (sets `photo_url: null`, deletes from storage)
- [ ] Reordering steps preserves their associated photos

### AC5: Upload UX ⏳ Planned

- [ ] File picker supports: click to browse, drag-and-drop onto drop zone
- [ ] Image preview shown immediately after selection (before upload completes)
- [ ] Upload progress indicator: progress bar or percentage during upload
- [ ] Optimistic UI: show preview while upload processes in background
- [ ] Upload error handling: retry button, clear error state, graceful fallback
- [ ] File type restriction in file picker: `accept="image/jpeg,image/png,image/webp"`
- [ ] Max file size enforced in UI: reject files > 10 MB with error message
- [ ] Multiple files can be uploaded in sequence (not parallel batch upload — MVP)

### AC6: Image optimization ⏳ Planned

- [ ] Uploaded images are resized if longest side exceeds 1920px (preserves aspect ratio)
- [ ] Images converted to WebP format on upload (better compression, universal browser support)
- [ ] Original image discarded after conversion (storage only keeps optimized version)
- [ ] Resize and convert happen server-side before uploading to R2
- [ ] Thumbnail generation: 400px wide version for card/grid display, stored as separate key with `-thumb` suffix

### AC7: Image display (enhanced) ⏳ Planned

- [ ] Recipe images: responsive full-width display, `object-fit: cover`
- [ ] Step photos: displayed below step text, max-width 100%, responsive
- [ ] Card thumbnails: 400px wide, `object-fit: cover`, rounded corners
- [ ] Lazy loading: `loading="lazy"` on all images below the fold
- [ ] `alt` text derived from recipe title (hero) or step text (step photos) for accessibility

### AC8: Upload API security ⏳ Planned

- [ ] Upload endpoint requires authentication (logged-in user only)
- [ ] User can only delete images they own (ownership tracked via `recipe_id` → `owner_id`)
- [ ] Rate limiting: max 20 uploads per minute per user (configurable)
- [ ] File extension validated against MIME type (prevent disguised executables)
- [ ] Uploaded files never executed or served as HTML/JS
- [ ] CORS headers configured for R2 bucket if served from different origin

## Technical Details

### ✅ Implemented: Database Schema

```sql
-- Already implemented in migrations/schema.sql
-- Part of CREATE TABLE recipes:
images JSONB NOT NULL DEFAULT '[]'::jsonb,

-- Idempotent migration for existing databases:
ALTER TABLE recipes ADD COLUMN IF NOT EXISTS images JSONB NOT NULL DEFAULT '[]'::jsonb;
```

The `images` column stores a JSON array of URL strings (e.g., `["https://cdn.example.com/img1.webp", "https://cdn.example.com/img2.webp"]`). Consistent with the existing JSONB pattern for `ingredients`, `instructions`, and `equipment`.

### ✅ Implemented: Rust Types

```rust
// src/types.rs
pub struct Recipe {
    // ... other fields ...
    pub images: Vec<String>,
}
```

All 7 SELECT queries include `r.images` in the column list. INSERT and UPDATE handle the `images` field. `FromRow` deserializes via `serde_json::from_value`.

### ✅ Implemented: API Pass-Through

- `create_recipe` accepts `images` parameter and persists it to the database
- `update_recipe` accepts `images` parameter and updates the existing array

### ✅ Implemented: Image Slider UI

Located on the recipe detail page below the header card. Neumorphic inset design with:
- Left/right navigation arrows (glassmorphism-styled)
- Dot indicators for multiple images
- Placeholder gradient backgrounds for empty or placeholder URLs
- Graceful degradation when the images vector is empty

### ⏳ Planned: Upload flow

```
1. User selects image file in browser
2. Frontend validates: type, size
3. File sent as multipart/form-data to POST /api/upload
4. Server validates: type, size, dimensions
5. Server resizes + converts to WebP, generates thumbnail
6. Server uploads to R2 via S3 API
7. Server returns JSON: { url, thumbnail_url, key }
8. Frontend shows preview, stores URL in form state
9. On recipe save, URLs persisted in DB (images array, steps[].photo_url)
```

### ⏳ Planned: Storage key format

```
recipes/{recipe_id}/{timestamp}_{random8}.{ext}
recipes/{recipe_id}/{timestamp}_{random8}-thumb.webp
```

Example:
```
recipes/a1b2c3d4-.../1718123456_a1b2c3d4.webp
recipes/a1b2c3d4-.../1718123456_a1b2c3d4-thumb.webp
```

UUID-based recipe_id ensures no naming collisions. Timestamp + random suffix prevents cache issues on re-upload.

### ⏳ Planned: Server functions

| Function | Purpose |
|----------|---------|
| `upload_image(user_id, file)` | Validate, resize, convert, upload to R2, return URL |
| `delete_image(user_id, recipe_id, image_key)` | Delete image from R2 (ownership-gated) |
| `update_recipe_images(recipe_id, user_id, images)` | Replace images array on recipe |

### ⏳ Planned: Image processing (Rust)

Use `image` crate for resize and format conversion:

```rust
use image::{io::Reader as ImageReader, DynamicImage, ImageError};

fn optimize_image(data: &[u8], max_longest: u32) -> Result<Vec<u8>, ImageError> {
    let img = ImageReader::new(std::io::Cursor::new(data)).with_guessed_format()?.decode()?;
    let resized = img.resize_to_max(max_longest, max_longest, image::imageops::FilterType::Lanczos3);
    let mut buf = Vec::new();
    resized.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::WebP)?;
    Ok(buf)
}
```

### ⏳ Planned: R2/S3 client setup

```rust
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;

async fn get_s3_client() -> Client {
    let region = aws_types::region::Region::new("auto");
    let creds = aws_types::credentials::Credentials::new(
        env::var("R2_ACCESS_KEY_ID").unwrap(),
        env::var("R2_SECRET_ACCESS_KEY").unwrap(),
        None, None, "r2",
    );
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .credentials_provider(creds)
        .endpoint_url(env::var("R2_ENDPOINT").unwrap())
        .load().await;
    Client::new(&config)
}
```

### ⏳ Planned: Route protection

- `POST /api/upload` added to `PROTECTED_PATHS`
- `DELETE /api/upload/:key` added to `PROTECTED_PATHS`

### Component changes

| Component | Status | Details |
|-----------|--------|---------|
| `render_image_slider` (in `recipe_detail.rs`) | ✅ Done | Neumorphic slider with arrows, dots, placeholder support |
| New: `ImageUpload` | ⏳ Planned | Reusable upload component: drag-and-drop zone, file picker, preview, progress bar, error handling |
| `RecipeForm` / `RecipeNew` | ⏳ Planned | Add image upload area at top |
| `RecipeEdit` | ⏳ Planned | Wire image add/remove to form state |
| `StepRow` | ⏳ Planned | Add `ImageUpload` per step row for step photos |
| `RecipeCard` | ⏳ Planned | Render image thumbnail (or gradient fallback if none) |

### ⏳ Planned: Cleanup strategy

- When recipe is deleted: cascade deletes recipe row, but orphaned images remain in R2
- Periodic cleanup job (future): scan R2 for images not referenced by any recipe
- When image is replaced: old image deleted from R2 on save
- When step photo is removed: image deleted from R2 on save

### WASM considerations

The `image` crate and `aws-sdk-s3` are native-only. Upload processing (resize, convert, S3 upload) happens on the server function side, not in WASM. Frontend only handles file selection and display.

### ⏳ Planned: Environment variables

| Variable | Description |
|----------|-------------|
| `R2_ACCESS_KEY_ID` | R2/S3 access key |
| `R2_SECRET_ACCESS_KEY` | R2/S3 secret key |
| `R2_BUCKET` | Bucket name |
| `R2_ENDPOINT` | R2 endpoint URL (e.g., `https://account-id.r2.cloudflarestorage.com`) |
| `R2_PUBLIC_URL` | Public CDN URL for accessing images (e.g., `https://cdn.example.com`) |
| `IMAGE_MAX_SIZE_BYTES` | Max upload size (default: 10485760 = 10 MB) |
| `IMAGE_MAX_DIMENSION` | Max longest side in pixels (default: 1920) |
| `IMAGE_THUMB_WIDTH` | Thumbnail width in pixels (default: 400) |

## Out of Scope

- Video upload for recipes
- Image cropping/editing UI
- Bulk image upload
- EXIF data stripping (future privacy enhancement)
- Image moderation / content scanning
- Collection cover images auto-selected from recipe images
- Watermarking

**Note:** Multiple recipe images/gallery is already supported via the `images` JSONB array (Phase 1). This is no longer out of scope.

## Checkpoints

| # | Checkpoint | Status | Deliverable |
|---|------------|--------|-------------|
| 1 | Schema + types + DB layer | ✅ Done | `images` JSONB column, `Recipe.images: Vec<String>` field, all 7 SELECTs + INSERT + UPDATE updated, FromRow handles images via serde_json |
| 2 | API pass-through | ✅ Done | `create_recipe` and `update_recipe` accept and persist images parameter |
| 3 | Image slider UI | ✅ Done | Neumorphic slider on recipe detail page with arrows, dots, placeholder support, graceful degradation |
| 4 | R2 setup + upload endpoint | ⏳ Planned | Storage bucket configured, `POST /api/upload` works end-to-end, returns URL |
| 5 | Image processing | ⏳ Planned | Resize + WebP conversion + thumbnail generation working, tests pass |
| 6 | Recipe image upload UI | ⏳ Planned | Create/edit forms wire upload, detail page displays real images, card thumbnails |
| 7 | Step photo integration | ⏳ Planned | `StepRow` has upload, photos render inline on detail page |
| 8 | Delete + cleanup | ⏳ Planned | Remove recipe/step images deletes from R2, DB updated |
| 9 | Security + edge cases | ⏳ Planned | Auth guards, file validation, rate limiting, error handling, WASM target builds |

## Success Metrics

### ✅ Phase 1 — Done

- [x] `images` column persists and retrieves correctly via DB layer
- [x] `create_recipe` and `update_recipe` accept and pass through images
- [x] Image slider renders on recipe detail page below header card
- [x] Slider handles 0, 1, and multiple images correctly
- [x] Navigation arrows wrap around, dots indicate active image
- [x] Zero clippy warnings on both wasm32 and x86_64 targets

### ⏳ Phase 2 — Planned

- [ ] User uploads recipe image → sees it on recipe detail page slider and dashboard card
- [ ] User uploads step photo → sees it inline with step text
- [ ] Large image (> 1920px) is resized and converted to WebP automatically
- [ ] Thumbnail generated for card display
- [ ] Removing photo deletes from storage and clears DB field
- [ ] Deleting recipe cascades DB rows (orphaned R2 cleanup deferred)
- [ ] File validation rejects non-image files and oversized files
- [ ] All 9 checkpoints pass with tests
- [ ] Zero clippy warnings on both wasm32 and x86_64 targets
- [ ] Upload completes in < 5 seconds for typical phone photo (3-5 MB) over broadband
