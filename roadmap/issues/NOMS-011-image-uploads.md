# NOMS-011: Image Uploads

**Status:** ⚪ Backlog  
**Phase:** Phase 2 (media)  
**Depends on:** NOMS-008 (Recipe CRUD)

## Overview

Add image upload capability for recipe hero photos and step photos. Images are uploaded to cloud storage (Cloudflare R2), URLs are stored in the existing `recipes.hero_photo_url` and `steps[].photo_url` fields. Includes file picker UI, upload progress, image optimization, and CDN serving.

This feature fills the `photo_url` placeholders already defined in the NOMS-008 schema and adds a new `hero_photo_url` column to the `recipes` table.

## Context

NOMS-008 defines `steps[].photo_url` as a nullable string inside the steps JSONB array, and the UI has "optional photo placeholder" per step. No upload infrastructure exists yet. This issue adds the full pipeline: file selection → upload → storage → URL → display.

## Acceptance Criteria

### AC1: Storage backend setup

- [ ] Cloudflare R2 bucket created for recipe images
- [ ] S3-compatible credentials configured as environment variables: `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`, `R2_ENDPOINT`
- [ ] Rust S3 client (`aws-sdk-s3` or `reqwest` + presigned URLs) configured in application
- [ ] Upload endpoint: `POST /api/upload` accepts multipart file upload, returns signed URL
- [ ] Delete endpoint: `DELETE /api/upload/:key` removes image from storage
- [ ] File validation server-side: type (JPEG, PNG, WebP), max size (10 MB), dimensions (max 6000px on longest side)
- [ ] Uploaded files stored with UUID-based keys: `recipes/{recipe_id}/{timestamp}_{random}.{ext}`
- [ ] CDN URL returned for image access (Cloudflare R2 public bucket URL or presigned read URL)

### AC2: Hero photo for recipe

- [ ] Migration adds `hero_photo_url TEXT` column to `recipes` table (nullable, default NULL)
- [ ] Recipe create form includes hero photo upload area at top (drag-and-drop + file picker)
- [ ] Recipe edit form allows changing or removing hero photo
- [ ] Hero photo displayed prominently on recipe detail page (full-width banner below title)
- [ ] Hero photo thumbnail shown on recipe card in dashboard and collection views
- [ ] Removing hero photo sets `hero_photo_url = NULL` and deletes image from storage

### AC3: Step photos

- [ ] Each step row in recipe form includes optional photo upload area
- [ ] Photo upload replaces `photo_url: null` with actual URL in steps JSONB
- [ ] Step photos displayed inline with step text on recipe detail page
- [ ] Photos can be removed from individual steps (sets `photo_url: null`, deletes from storage)
- [ ] Reordering steps preserves their associated photos

### AC4: Upload UX

- [ ] File picker supports: click to browse, drag-and-drop onto drop zone
- [ ] Image preview shown immediately after selection (before upload completes)
- [ ] Upload progress indicator: progress bar or percentage during upload
- [ ] Optimistic UI: show preview while upload processes in background
- [ ] Upload error handling: retry button, clear error state, graceful fallback
- [ ] File type restriction in file picker: `accept="image/jpeg,image/png,image/webp"`
- [ ] Max file size enforced in UI: reject files > 10 MB with error message
- [ ] Multiple files can be uploaded in sequence (not parallel batch upload — MVP)

### AC5: Image optimization

- [ ] Uploaded images are resized if longest side exceeds 1920px (preserves aspect ratio)
- [ ] Images converted to WebP format on upload (better compression, universal browser support)
- [ ] Original image discarded after conversion (storage only keeps optimized version)
- [ ] Resize and convert happen server-side before uploading to R2
- [ ] Thumbnail generation: 400px wide version for card/grid display, stored as separate key with `-thumb` suffix

### AC6: Image display

- [ ] Hero photo: responsive full-width banner, `object-fit: cover`, max-height 400px on desktop
- [ ] Step photos: displayed below step text, max-width 100%, responsive
- [ ] Card thumbnails: 400px wide, `object-fit: cover`, rounded corners
- [ ] Lazy loading: `loading="lazy"` on all images below the fold
- [ ] Placeholder/skeleton while image loads
- [ ] Graceful degradation: if image fails to load, show placeholder icon or gradient background
- [ ] `alt` text derived from recipe title (hero) or step text (step photos) for accessibility

### AC7: Image DB queries and types

- [ ] `hero_photo_url TEXT` column added to `recipes` table via migration
- [ ] `Recipe` struct in `src/db/mod.rs` includes `hero_photo_url: Option<String>`
- [ ] `insert_recipe()` and `update_recipe()` handle `hero_photo_url`
- [ ] No new tables needed — URLs stored as strings in existing columns
- [ ] Tests for hero photo CRUD: set, update, remove

### AC8: Upload API security

- [ ] Upload endpoint requires authentication (logged-in user only)
- [ ] User can only delete images they own (ownership tracked via `recipe_id` → `owner_id`)
- [ ] Rate limiting: max 20 uploads per minute per user (configurable)
- [ ] File extension validated against MIME type (prevent disguised executables)
- [ ] Uploaded files never executed or served as HTML/JS
- [ ] CORS headers configured for R2 bucket if served from different origin

## Technical Details

### Database Schema (alteration)

```sql
-- Add hero photo URL to recipes table
ALTER TABLE recipes ADD COLUMN IF NOT EXISTS hero_photo_url TEXT;
```

### Upload flow

```
1. User selects image file in browser
2. Frontend validates: type, size
3. File sent as multipart/form-data to POST /api/upload
4. Server validates: type, size, dimensions
5. Server resizes + converts to WebP, generates thumbnail
6. Server uploads to R2 via S3 API
7. Server returns JSON: { url, thumbnail_url, key }
8. Frontend shows preview, stores URL in form state
9. On recipe save, URLs persisted in DB (hero_photo_url, steps[].photo_url)
```

### Storage key format

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

### Server functions

| Function | Purpose |
|----------|---------|
| `upload_image(user_id, file)` | Validate, resize, convert, upload to R2, return URL |
| `delete_image(user_id, recipe_id, image_key)` | Delete image from R2 (ownership-gated) |
| `update_hero_photo(recipe_id, user_id, hero_photo_url)` | Set/update hero photo URL |
| `remove_hero_photo(recipe_id, user_id)` | Remove hero photo (set NULL, delete from R2) |

### Image processing (Rust)

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

### R2/S3 client setup

```rust
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;

async fn get_s3_client() -> Client {
    let region = aws_types::region::Region::new("auto"); // R2 uses "auto" or custom endpoint
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

### AuthContext changes

No changes. Upload endpoint uses existing auth middleware.

### Route protection changes

- `POST /api/upload` added to `PROTECTED_PATHS`
- `DELETE /api/upload/:key` added to `PROTECTED_PATHS`

### Component changes

| Component | Change |
|-----------|--------|
| New: `ImageUpload` | Reusable upload component: drag-and-drop zone, file picker, preview, progress bar, error handling |
| New: `HeroPhotoUpload` | Hero photo specific: full-width upload area, preview banner, remove button |
| `RecipeForm` | Add `HeroPhotoUpload` at top |
| `StepRow` | Add `ImageUpload` per step row |
| `RecipeDetail` | Render hero photo banner, step photos inline |
| `RecipeCard` | Render hero photo thumbnail (or gradient fallback if none) |
| `RecipeNew` | Wire hero photo upload to form state |
| `RecipeEdit` | Wire hero photo change/remove to form state |

### Cleanup strategy

- When recipe is deleted: cascade deletes recipe row, but orphaned images remain in R2
- Periodic cleanup job (future): scan R2 for images not referenced by any recipe
- When hero photo is replaced: old image deleted from R2 on save
- When step photo is removed: image deleted from R2 on save

### WASM considerations

The `image` crate and `aws-sdk-s3` are native-only. Upload processing (resize, convert, S3 upload) happens on the Leptos server function side, not in WASM. Frontend only handles file selection and display.

### Environment variables

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
- Collection cover images auto-selected from recipe hero photos
- Watermarking
- Multiple hero photos / gallery

## Checkpoints

| # | Checkpoint | Deliverable |
|---|------------|-------------|
| 1 | R2 setup + upload endpoint | Storage bucket configured, `POST /api/upload` works end-to-end, returns URL |
| 2 | Image processing | Resize + WebP conversion + thumbnail generation working, tests pass |
| 3 | Hero photo integration | `hero_photo_url` column added, create/edit forms wire upload, detail page displays hero |
| 4 | Step photo integration | `StepRow` has upload, photos render inline on detail page |
| 5 | Card thumbnails | `RecipeCard` shows hero thumbnail or fallback |
| 6 | Delete + cleanup | Remove hero/step photos deletes from R2, DB updated |
| 7 | Security + edge cases | Auth guards, file validation, rate limiting, error handling, WASM target builds |

## Success Metrics

- User uploads hero photo → sees it on recipe detail page and dashboard card
- User uploads step photo → sees it inline with step text
- Large image (> 1920px) is resized and converted to WebP automatically
- Thumbnail generated for card display
- Removing photo deletes from storage and clears DB field
- Deleting recipe cascades DB rows (orphaned R2 cleanup deferred)
- File validation rejects non-image files and oversized files
- All 7 checkpoints pass with tests
- Zero clippy warnings on both wasm32 and x86_64 targets
- Upload completes in < 5 seconds for typical phone photo (3-5 MB) over broadband
