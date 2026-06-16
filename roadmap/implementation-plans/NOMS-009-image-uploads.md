# NOMS-009: Image Uploads - Incremental Implementation Plan

> **Status:** Draft - ready for implementation
> **Depends on:** NOMS-008 (recipe CRUD), NOMS-008b (JSONB schema)
> **Current state:** images JSONB column exists, slider UI renders placeholders, no upload infrastructure

---

## Architecture Overview

```
Client (Tauri/eGui)          Server (Axum)              Minio (S3-compatible)
-------------------          ---------------            -----------------------

  [Choose Files]
       |
       v
  [ImageUploadState]
       |
       | multipart/form-data
       v
  POST /api/upload  ----->  upload_image_handler()
                               |
                               v
                          validate (mime, size)
                               |
                               v
                          process_image()
                     [resize, WebP, thumb]
                               |
                               v
                          upload_processed()
                     [put_object x2]
                               |
                               v
                        bucket: noms-images
                        |-- recipes/{uid}/{uuid}.webp
                        |-- recipes/{uid}/{uuid}-thumb.webp
                               |
                               | { url, key, thumb_url, thumb_key }
                               v
                          UploadResponse
                               |
                               v
                        [slider in UI]
```

**Key conventions:**
- Storage keys: `recipes/{user_id}/{uuid}.{ext}`
- Thumbnail keys: `recipes/{user_id}/{uuid}-thumb.{ext}`
- All uploaded images converted to WebP
- Max original size: 10 MB
- Resize threshold: longest side > 1920 px
- Thumbnail size: 400 px longest side

---

## Checkpoint 1 (CP1): Storage Backend Wiring

**Goal:** `StorageClient` can connect to Minio and pass a health check.

### Dependencies
- None (standalone)

### Environment Variables

| Variable | Example | Description |
|---|---|---|
| `S3_ENDPOINT` | `http://localhost:9000` | Minio/S3 endpoint URL |
| `S3_ACCESS_KEY` | `noms-admin` | Access key |
| `S3_SECRET_KEY` | `noms-admin-secret` | Secret key |
| `S3_BUCKET` | `noms-images` | Bucket name |
| `S3_PUBLIC_URL` | `http://localhost:9000/noms-images` | Public base URL for constructed image URLs |
| `S3_REGION` | `us-east-1` | Region (optional, defaults to us-east-1) |

### Exact File Changes

#### `Cargo.toml` - add dependency
Add `aws-sdk-s3 = { version = "1", optional = true }` to dependencies.
Add `dep:aws-sdk-s3` to the `server` feature list.

#### New file: `src/storage.rs`
- `StorageError` enum: Init, HealthCheck, Upload, Download, Delete variants
- `StorageConfig` struct: endpoint, access_key, secret_key, bucket, public_url, region
- `StorageConfig::from_env()` - reads all S3_* env vars
- `StorageClient` struct: wraps aws-sdk-s3 Client + StorageConfig
- `StorageClient::new(config)` - builds client with custom endpoint, path-style, credentials
- `StorageClient::health_check()` - HEAD bucket
- Accessors: `bucket()`, `public_url()`, `config()`, `client()`

#### `src/server.rs` - integrate into AppState
- Add `storage: Option<StorageClient>` to AppState
- In init: if S3_ENDPOINT is set, create StorageClient; otherwise None

#### `src/lib.rs` or `src/main.rs` - register module
`#[cfg(feature = "server")] pub mod storage;`

### Verification

```bash
# 1. Code compiles
cargo check --features server

# 2. Ensure Minio is running
docker ps | grep noms-minio

# 3. Smoke test
cargo test --features server storage::tests
```

**Test to add in `src/storage.rs`:**
`#[tokio::test] #[ignore] async fn test_health_check()` - sets env vars, creates client, asserts health_check succeeds.

---

## Checkpoint 2 (CP2): Upload API Endpoint

**Goal:** `POST /api/upload` accepts a file, validates it, stores it in Minio, returns `{ url, key }`.

### Dependencies
- CP1 (StorageClient must exist)

### Server Function Signatures

```rust
pub async fn upload_image(
    user_id: &str,
    file_bytes: Vec<u8>,
    mime_type: &str,
    filename: &str,
    storage: &StorageClient,
) -> Result<UploadResponse, String>

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub url: String,
    pub key: String,
}
```

### Exact File Changes

#### `src/server.rs` - new route handler
- Import `axum::extract::Multipart`
- Add route: `"/api/upload" -> post(upload_image_handler)` (protected)
- Handler: extract multipart field "file", get bytes/name/content-type, delegate to `upload_image()`
- Error cases: no storage configured (503), no file (400), validation failure (400)

#### New file: `src/server_functions/upload.rs`
- `UploadResponse` struct with url, key
- `upload_image()`: validate mime type, check 10MB size limit, generate UUID key, put_object to Minio
- `validate_image_mime()`: accepts image/jpeg, image/png, image/webp by MIME or extension
- `mime_to_ext()`: maps MIME type to file extension

#### `src/server_functions/mod.rs` - register module
`pub mod upload;`

### Verification

```bash
# 1. Code compiles
cargo check --features server

# 2. Manual curl test (requires running server + Minio + auth token)
curl -X POST http://localhost:3000/api/upload   -H "Authorization: Bearer <token>"   -F "file=@/path/to/test.jpg" | jq .

# Expected: { "url": "...", "key": "recipes/<user>/<uuid>.jpg" }

# 3. Verify file exists in Minio
mc ls myminio/noms-images/recipes/<user_id>/

# 4. Test oversized file -> 400
# 5. Test unsupported type (PDF) -> 400
```

---

## Checkpoint 3 (CP3): Image Processing

**Goal:** Uploaded images are resized, converted to WebP, and a thumbnail is generated.

### Dependencies
- CP2 (upload endpoint must exist)

### Server Function Signatures

```rust
impl StorageClient {
    pub fn process_image(&self, file_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), StorageError>
    pub async fn upload_processed(&self, user_id: &str, uuid: &str,
        main_bytes: Vec<u8>, thumb_bytes: Vec<u8>) -> Result<(String, String, String, String), StorageError>
}
```

### Exact File Changes

#### `Cargo.toml` - add image crate
Add `image = { version = "0.25", optional = true }` and `dep:image` to server features.

#### `src/storage.rs` - add processing methods
- `process_image()`: decode with ImageReader, resize if longest > 1920px (Lanczos3), encode as WebP, generate 400px thumb, encode as WebP
- `upload_processed()`: put_object for main .webp and -thumb.webp, return (main_url, main_key, thumb_url, thumb_key)

#### `src/server_functions/upload.rs` - use processing pipeline
- Replace raw upload: call `storage.process_image()` then `storage.upload_processed()`
- Update `UploadResponse` to include optional `thumb_url` and `thumb_key` fields

### Verification

```bash
# 1. Code compiles
cargo check --features server

# 2. Upload a large image (e.g., 4000x3000 JPEG)
curl -X POST http://localhost:3000/api/upload   -H "Authorization: Bearer <token>"   -F "file=@/path/to/large_4k.jpg" | jq .

# 3. Verify in Minio: two .webp files exist (main + -thumb)
mc ls myminio/noms-images/recipes/<user_id>/

# 4. Check dimensions: main <= 1920px, thumb <= 400px
```

---

## Checkpoint 4 (CP4): Profile Image Upload

**Goal:** Users can upload, view, and remove a profile avatar image. Reuses the storage layer from CP1-CP3; simpler than the recipe upload UI.

### Dependencies
- CP2 (`POST /api/upload` endpoint must exist)
- CP3 (image processing pipeline preferred — avatars get resized + converted to WebP)

### Server Function Signatures

```rust
pub async fn update_avatar(
    user_id: &str,
    avatar_url: &str,
    pool: &sqlx::PgPool,
) -> Result<(), String>

pub async fn remove_avatar(
    user_id: &str,
    pool: &sqlx::PgPool,
) -> Result<(), String>
```

### Exact File Changes

#### `src/server_functions/upload.rs` — add `type` query param routing
- Accept optional `type` query parameter on `POST /api/upload`:
  - `type=avatar` (or omitted default) → routes to different key prefix
- When `type=avatar`: use storage key `avatars/{user_id}/{uuid}.webp` (and `avatars/{user_id}/{uuid}-thumb.webp` for thumbnail)
- When `type=recipe` (or omitted): existing behavior — `recipes/{user_id}/{uuid}.webp`
- Avatar uploads use the same processing pipeline (resize, WebP, thumbnail) as recipe images

#### New file: `src/server_functions/avatar.rs`
- `update_avatar()`: `UPDATE users SET avatar_url = $1 WHERE id = $2 RETURNING id`
- `remove_avatar()`:
  1. `SELECT avatar_url FROM users WHERE id = $1` — fetch current URL
  2. `UPDATE users SET avatar_url = NULL WHERE id = $1`
  3. If old URL exists, extract key from URL and call `DELETE /api/upload/:key` (CP6's delete endpoint) to remove from Minio

#### `src/server_functions/mod.rs` — register module
`pub mod avatar;`

#### `src/pages/settings/settings_profile.rs` — avatar upload UI
- Add avatar upload section to the profile settings page:
  - **Current avatar preview**: circular image showing uploaded avatar, OAuth avatar, or initials fallback (in that priority order)
  - **Upload zone**: small drag-drop zone or "Choose Photo" button for selecting new avatar
  - **Remove button**: "Remove Avatar" — calls `remove_avatar()`, clears local state
- State: `current_avatar_url: Option<String>`, `uploading: bool`, `error: Option<String>`
- On upload: send file via `POST /api/upload?type=avatar`, then call `update_avatar()` with returned URL
- On remove: call `remove_avatar()`, reset preview to fallback

#### `src/api.rs` — avatar API methods
- `AppApi::upload_avatar(file_path, auth_token)`: multipart POST to `/api/upload?type=avatar`
- `AppApi::update_avatar(avatar_url, auth_token)`: PUT/POST to new avatar endpoint
- `AppApi::remove_avatar(auth_token)`: POST/DELETE to avatar removal endpoint

#### `src/components/user_avatar.rs` (or equivalent display component) — avatar display
- Render uploaded avatar image if `avatar_url` is `Some`
- Fallback chain: uploaded avatar → OAuth provider avatar → initials-based placeholder
- Circular avatar with consistent sizing across settings page and public profile page

### Verification

```bash
# 1. Code compiles
cargo check --features server
cargo check

# 2. Run the app
cargo run --release

# 3. Manual test:
#    - Navigate to Settings > Profile
#    - Upload a profile photo via the upload zone
#    - Verify: avatar appears as circular preview on settings page
#    - Verify: same avatar appears on public profile page
#    - Click "Remove Avatar" -> avatar reverts to OAuth avatar or initials
#    - Upload a new photo -> replaces previous avatar
#    - Verify in Minio: avatars/<user_id>/ contains .webp files
#    - After removing avatar: verify old file is deleted from Minio
```

---

## Checkpoint 5 (CP5): Upload UI Component

**Goal:** Users can select and upload images from the recipe creation form.

### Dependencies
- CP2 (upload endpoint) or CP3 (with processing)

### Exact File Changes

#### New file: `src/components/base/image_upload.rs`
- `ImageUploadState`: files list, drag-over flag, error
- `UploadFile`: path, name, uploading, progress, url, key, error
- `ImageUploadState::new()`, `add_file()`, `remove_file()`, `get_uploaded_urls()`
- `ImageUploadState::ui()`: drag-drop zone, "Choose Files" button (rfd file dialog), file list with progress/error status
- `ImageUploadState::upload_all()`: iterates pending files, calls api.upload_image(), updates progress

#### `src/api.rs` - add upload method
- `AppApi::upload_image(file_path, auth_token)`: reads file, sends multipart POST to /api/upload, parses UploadResponse

#### `src/pages/recipe_new.rs` - integrate upload
- Add `image_upload: ImageUploadState` to RecipeNewState
- In ui(): add "Recipe Images" section with image_upload.ui()
- In save flow: upload pending images first, then include URLs in recipe save payload

#### `src/components/base/mod.rs` - register module
`pub mod image_upload;`

#### `Cargo.toml` - client dependencies
Add `rfd = "0.14"` (file dialog), ensure reqwest has "multipart" feature

#### `src/components/base/image_upload.rs` — client-side compression (toggle + preview)
- `ImageUpload` component adds `compression_enabled: Signal<bool>` state
- `UploadFile` gains additional fields: `original_size: u64`, `compressed_size: Option<u64>`, `compressed_blob: Option<Blob>`, `needs_compression: bool`
- On file select: read file size, set `needs_compression = true` if file > 1MB, show original file size
- If file > 1MB: show "Compress" toggle button alongside the file entry
- When toggled on: compress image via canvas — draw image to `web_sys::HtmlCanvasElement`, resize so longest side <= 1920px, export via `canvas.to_blob("image/webp", 0.8)`
- Show compressed file size and reduction percentage (e.g., "8.2 MB → 1.4 MB (83% smaller)")
- Show side-by-side preview: original thumbnail vs compressed thumbnail
- Upload flow uses compressed blob when toggle is on, original File when off
- Compression happens entirely in browser — no server changes needed; the compressed blob is uploaded the same way as the original file

### Verification

```bash
# 1. Code compiles
cargo check

# 2. Run the app
cargo run --release

# 3. Manual test — basic upload:
#    - Navigate to "New Recipe"
#    - Click "Choose Files" or drag an image into the zone
#    - Verify file appears in list with upload progress
#    - Save recipe - images upload first, then recipe saves with URLs
#    - Navigate to recipe detail - slider shows uploaded image

# 4. Manual test — compression toggle:
#    - Select a file > 1MB (e.g., a 5MB+ photo from a phone/camera)
#    - Verify: original file size is displayed (e.g., "5.2 MB")
#    - Verify: "Compress" toggle button appears
#    - Toggle "Compress" ON
#    - Verify: compressed file size and reduction % appear (e.g., "5.2 MB → 0.8 MB (85% smaller)")
#    - Verify: side-by-side preview shows original vs compressed thumbnail
#    - Toggle "Compress" OFF
#    - Verify: compressed size info disappears, original size shown
#    - Select a file < 1MB
#    - Verify: no compression toggle appears

# 5. Manual test — upload with/without compression:
#    - Upload a large image WITH compression enabled
#    - Verify: the compressed blob is sent to the server (check network tab for smaller payload)
#    - Navigate to recipe detail — verify image displays correctly
#    - Upload a large image WITHOUT compression (toggle off)
#    - Verify: the original file is sent to the server (check network tab for full-size payload)
#    - Navigate to recipe detail — verify image displays correctly
```

---

## Checkpoint 6 (CP6): Edit Form Integration

**Goal:** Edit existing recipe images (add new, delete existing) from the edit form.

### Dependencies
- CP5 (upload UI component exists)

### Server Function Signatures

```rust
pub async fn delete_image(
    user_id: &str,
    image_key: &str,
    storage: &StorageClient,
) -> Result<(), String>

pub async fn replace_image(
    user_id: &str,
    old_key: &str,
    file_bytes: Vec<u8>,
    mime_type: &str,
    filename: &str,
    storage: &StorageClient,
) -> Result<UploadResponse, String>
```

### Exact File Changes

#### `src/server_functions/upload.rs` - delete + replace functions
- `delete_image()`: verify key starts with `recipes/{user_id}/` (authorization), delete main object, best-effort delete matching -thumb key
- `replace_image()`: verify old_key starts with `recipes/{user_id}/` (authorization), process new image via `process_image()` + `upload_processed()`, then delete old main object and old -thumb object from Minio, return `UploadResponse` for the new image
- Update `upload_image()` to accept optional `replace_key: Option<&str>` parameter: when present, after uploading the new image, delete the old key (main + thumb) from Minio — same atomic swap as `replace_image()` but routed through the existing multipart handler

#### `src/server.rs` - delete + replace routes
- Add route: `"/api/upload/:key" -> delete(delete_image_handler)` (protected)
- Handler: extract key from path, call delete_image, return 200 or 403
- Add route: `"/api/upload/replace" -> post(replace_image_handler)` (protected)
- Handler: extract multipart field "file" + query param `old_key`, call `replace_image()`, return 200 with `UploadResponse` or 403/400

#### `src/api.rs` - delete + replace methods
- `AppApi::delete_image(key, auth_token)`: DELETE /api/upload/:key
- `AppApi::replace_image(file_path, old_key, auth_token)`: multipart POST to /api/upload/replace with `old_key` query param, parses `UploadResponse`

#### `src/components/base/image_upload.rs` - replace mode
- `ImageUploadState` gains `replace_target: Option<String>` field (tracks the key of the image being replaced)
- `ImageUploadState` accepts optional `replace_key: Option<String>` constructor parameter to enter replace mode immediately
- When `replace_target` is `Some(key)`: upload button label changes to "Replace Image"; after successful upload, calls `api.replace_image()` (or `upload_image` with `replace_key`) instead of plain `upload_image()`
- After replace completes: clear `replace_target`, update the existing image entry at the matching position rather than appending
- UI shows a "Cancel Replace" button when in replace mode to clear `replace_target` back to `None`

#### `src/pages/recipe_edit.rs` - image management with replace
- Add `existing_images: Vec<ImageEntry>` (url + key) and `image_upload: ImageUploadState` to RecipeEditState
- In ui(): each existing image thumbnail gets two buttons: **"Replace"** and **"Delete"**
  - Clicking **"Replace"**: sets `image_upload.replace_target = Some(image.key)` and opens/focuses the upload dialog in replace mode
  - Clicking **"Delete"**: immediately calls `api.delete_image(key)`, removes entry from `existing_images`
- In save flow: merge existing + new image URLs, call delete_image for removed images
- Initialize existing_images from loaded recipe data

### Verification

```bash
# 1. Code compiles
cargo check --features server
cargo check

# 2. Manual test — delete:
#    - Open existing recipe with images -> Edit
#    - Verify existing images display with "Replace" and "Delete" buttons
#    - Delete an image, save
#    - Verify slider reflects changes (removed image gone)
#    - Verify deleted image (main + thumb) is gone from Minio

# 3. Manual test — replace:
#    - Open existing recipe with images -> Edit
#    - Click "Replace" on one of the existing image thumbnails
#    - Verify upload dialog opens in replace mode (label says "Replace Image")
#    - Select a new image file and upload
#    - Verify: new image takes the same position in the array as the old one
#    - Verify: old image (main + thumb) is deleted from Minio
#    - Verify: new image (main + thumb) exists in Minio
#    - Save recipe, navigate to detail page
#    - Verify slider shows the new image in the correct position

# 4. Manual test — cancel replace:
#    - Click "Replace" on a thumbnail
#    - Click "Cancel Replace" in the upload dialog
#    - Verify: replace mode is cleared, no upload occurs, existing image is untouched
```

---

## Checkpoint 7 (CP7): Step Photos

**Goal:** Each recipe step can optionally have an associated photo.

### Dependencies
- CP5 (upload infrastructure)

### Server Function Signatures

No new server functions needed - reuses `upload_image()` and `delete_image()` from CP2/CP6.

### Exact File Changes

#### `src/models/recipe.rs` (or equivalent) - add photo_url to step
- Add `pub photo_url: Option<String>` to `RecipeStep` struct
- Ensure JSON serialization includes photo_url (Option handles None gracefully)

#### Database migration
- Update the JSONB instructions schema to include optional photo_url per step
- If using a migration: alter or add column that stores step photo URLs

#### `src/pages/recipe_new.rs` - step photo upload
- In each step row: add small image upload button
- On upload: store photo_url in the step data
- Reuse ImageUploadState or a simpler single-file picker per step

#### `src/pages/recipe_edit.rs` - step photo edit
- Show existing step photo thumbnail with delete button
- Allow replacing step photo

#### `src/pages/recipe_detail.rs` - display step photos
- After each step text block: if photo_url is Some, display the image
- Use egui image loading (load from URL or cached texture)

### Verification

```bash
# 1. Code compiles
cargo check --features server
cargo check

# 2. Manual test:
#    - Create recipe with 3 steps
#    - Add photo to step 2
#    - Save recipe
#    - View detail page: step 2 shows photo inline
#    - Edit recipe: remove step 2 photo, add to step 1
#    - Verify photo moved correctly
```

---

## Checkpoint 8 (CP8): Card Thumbnails + Polish

**Goal:** Recipe cards show the first image as a thumbnail, with graceful fallback.

### Dependencies
- CP5 (images stored and accessible)

### Exact File Changes

#### `src/components/recipe_card.rs` (or equivalent) - thumbnail display
- If recipe.images is non-empty: load first image URL as card thumbnail
- Fallback: gradient placeholder (existing behavior)
- Lazy loading: only load image when card is visible in viewport
- Error handling: if image fails to load, fall back to gradient placeholder
- Alt text: use recipe title as alt text

#### `src/pages/recipe_list.rs` (or wherever cards are rendered)
- Pass images array to RecipeCard component
- Ensure card layout accommodates thumbnail (image on top or side)

### Verification

```bash
# 1. Code compiles
cargo check

# 2. Manual test:
#    - Navigate to recipe list
#    - Cards with images show thumbnails
#    - Cards without images show gradient placeholder
#    - Simulate slow network: images load lazily
#    - Corrupt/broken image URL: falls back to gradient gracefully
```

---

## Checkpoint 9 (CP9): Image Preview Editor with Crop Tool

**Goal:** Enhancement — full-featured image editor modal that replaces the basic file picker in the image upload component. This is a later upgrade that can be added after the basic upload flow (CP1–CP8) is fully working. Custom canvas-based cropper with no external dependencies.

### Dependencies
- CP5 (ImageUploadState and `image_upload.rs` must exist — the crop editor is invoked from there)

### Overview
- This checkpoint is an enhancement layer, not a blocking dependency for the core upload flow
- The crop editor replaces the basic file picker in `image_upload.rs` as a later upgrade
- Full-featured image editor modal that opens after file selection, before upload
- Custom canvas-based cropper (no external dependencies)
- Allows users to crop, zoom, pan, and rotate their image before uploading
- Shows preview of how the image will appear in context (card thumbnail, slider, etc.)

### Features
- Canvas renders the uploaded image at full resolution
- Draggable, resizable crop overlay with handles on corners and edges
- Aspect ratio presets: 16:9 (slider), 1:1 (card/avatar), 4:3, freeform
- Zoom: scroll wheel or pinch gesture, min 25%, max 500%
- Pan: click and drag within crop area when zoomed in
- Rotate: 90° clockwise/counter-clockwise buttons
- Flip: horizontal and vertical mirror buttons
- Preview panel: shows cropped result as it will appear (card thumbnail mockup, slider frame)
- Cancel: discard edits and return to file picker
- Apply: render cropped region to new canvas, export as WebP blob, proceed to upload

### Technical Details
- Use `web_sys::HtmlCanvasElement` and `web_sys::CanvasRenderingContext2d` for canvas operations
- Crop state: `struct CropState { x, y, width, height, rotation, zoom, aspect_ratio }`
- Render loop: `use_effect` + `requestAnimationFrame` for smooth drag/zoom
- Touch support: `touchstart`/`touchmove`/`touchend` for mobile pinch-to-zoom
- Export: `canvas.draw_image()` with crop transform, then `canvas.to_blob()` for WebP

### Exact File Changes

#### New file: `src/components/base/image_crop_editor.rs`
- `CropState` struct: `{ x: f64, y: f64, width: f64, height: f64, rotation: i32, zoom: f64, aspect_ratio: Option<(u32, u32)> }`
- `CropEditorState` struct: canvas ref, image element ref, CropState, is_dragging, is_resizing, drag_start, resize_handle, original_image_data_url
- `CropEditorState::new(image_data_url)`: initialize canvas, load image, set default crop to full image
- `CropEditorState::ui()`: modal overlay with:
  - Main canvas area (left): renders image with zoom/rotation/pan applied
  - Crop overlay: semi-transparent darkened regions outside crop box, draggable/resizable crop rectangle with corner and edge handles
  - Toolbar (top): rotate CW/CCW buttons, flip H/V buttons, aspect ratio preset buttons (16:9, 1:1, 4:3, freeform)
  - Preview panel (right): small mockup showing cropped result in context (card thumbnail frame, slider frame)
  - Footer: "Cancel" and "Apply" buttons
- Mouse handlers: `mousedown` on crop handles → start resize, `mousedown` inside crop → start drag, `wheel` → zoom in/out, `mousemove` → apply drag/resize, `mouseup` → end interaction
- Touch handlers: `touchstart` (1 finger → drag, 2 fingers → pinch start), `touchmove` (1 finger → drag, 2 fingers → pinch zoom), `touchend` → reset
- `render_canvas()`: clear canvas, apply transform (translate to center, rotate, scale by zoom, translate by pan), draw image
- `render_crop_overlay()`: draw semi-transparent overlay outside crop bounds, draw crop rectangle with handles
- `apply_crop()`: create offscreen canvas at crop dimensions, `drawImage` with source rect mapped through rotation/flip transform, `to_blob` callback returns WebP blob
- `get_cropped_blob(callback)`: calls `apply_crop()`, invokes callback with resulting Blob

#### `src/components/base/image_upload.rs` — Enhanced: opens crop editor after file selection
- Add `crop_editor: Option<CropEditorState>` field to `ImageUploadState`
- When a file is selected via `add_file()`: instead of immediately queuing for upload, open the crop editor modal by setting `crop_editor = Some(CropEditorState::new(data_url))`
- When crop editor "Apply" is pressed: take the cropped blob, convert to file path or store as in-memory blob, proceed to upload queue
- When crop editor "Cancel" is pressed: discard the crop editor, remove the file from the pending list
- Add `skip_crop: bool` toggle in the upload UI for users who want to skip editing

#### `assets/main.css` — Editor modal styles
- `.crop-editor-modal`: fixed overlay, dark background, flex layout
- `.crop-editor-canvas-container`: relative positioning, overflow hidden, cursor changes for drag/resize
- `.crop-editor-canvas`: absolute positioned canvas element
- `.crop-overlay`: semi-transparent dark layer with crop hole
- `.crop-handle`: small squares on corners and edges of crop rectangle
- `.crop-editor-toolbar`: top bar with icon buttons
- `.crop-editor-preview`: right panel with mockup frames
- `.crop-editor-footer`: bottom bar with Cancel/Apply buttons
- Responsive: on mobile, stack toolbar above canvas, preview below

#### `src/components/base/mod.rs` — register module
`pub mod image_crop_editor;`

#### `Cargo.toml` — no new dependencies
- Uses existing `web-sys` with features: `CanvasRenderingContext2d`, `HtmlCanvasElement`, `HtmlImageElement`, `Blob`, `File`

### Verification

```bash
# 1. Code compiles
cargo check

# 2. Run the app
cargo run --release

# 3. Manual test flow:
#    - Navigate to "New Recipe"
#    - Click "Choose Files" or drag an image into the upload zone
#    - Crop editor modal opens immediately
#    - Select 16:9 aspect ratio preset → crop box snaps to 16:9
#    - Drag crop box to desired region
#    - Scroll wheel to zoom to 300% → pan to fine-tune crop region
#    - Rotate 90° CW → verify image rotates and crop box adjusts
#    - Flip horizontal → verify mirror effect
#    - Check preview panel: updates in real-time as crop changes
#    - Click "Apply" → modal closes, cropped image appears in upload list
#    - Save recipe → cropped image uploads
#    - Navigate to recipe detail → slider shows the cropped (not original) image

# 4. Rotation test:
#    - Select a landscape image
#    - Rotate 90° → becomes portrait
#    - Apply → upload → verify orientation preserved after upload

# 5. Zoom + pan test:
#    - Zoom to 300% → pan to a different region of the image
#    - Set a tight crop box
#    - Apply → verify only the correct zoomed region was uploaded

# 6. Mobile test:
#    - Open on mobile device or DevTools mobile emulation
#    - Pinch to zoom → smooth zooming
#    - Drag to pan → smooth panning
#    - Tap aspect ratio presets → crop box adjusts
#    - Apply → upload succeeds

# 7. Cancel test:
#    - Open crop editor → make changes → click "Cancel"
#    - Modal closes → file is removed from upload list
#    - No upload occurs
```

---

## Checkpoint 10 (CP10): Upload Security Hardening

**Goal:** Harden all upload and delete paths against common attack vectors: MIME spoofing, oversized payloads, cross-user deletion, path traversal, abuse via rate limiting, and resource exhaustion via concurrent uploads.

### Dependencies
- CP2 (upload endpoint must exist)
- CP6 (delete_image must exist)

### 1. MIME Type Validation

**Problem:** A `.jpg` extension doesn't guarantee the file is actually an image. An attacker can upload an `.exe` renamed to `.jpg`.

**Solution:** Validate magic bytes against declared MIME type and file extension.

**Implementation:**
- Use `mime_guess` crate (server-only dependency) for extension-to-MIME mapping
- Read first 12 bytes of file to check magic bytes:
  - JPEG: starts with `FF D8 FF`
  - PNG: starts with `89 50 4E 47 0D 0A 1A 0A`
  - WebP: starts with `RIFF` + `WEBP` at offset 8
- Cross-check: magic bytes must match both the declared Content-Type and the file extension
- Reject mismatches with **415 Unsupported Media Type**
- Allowed types: `image/jpeg`, `image/png`, `image/webp`

**File changes:**
- `src/storage.rs` — new function `validate_magic_bytes(bytes: &[u8], declared_mime: &str, filename: &str) -> Result<(), String>`
- `Cargo.toml` — add `mime_guess = { version = "0.16", optional = true }` and `dep:mime_guess` to server features

### 2. File Size Enforcement

**Problem:** Without a hard limit, a single user can exhaust disk space or memory.

**Solution:** Enforce 10 MB maximum at two layers.

**Implementation:**
- **Layer 1 (Axum extractor):** Use `axum::extract::Multipart` with a per-request body limit configured on the route layer
- **Layer 2 (Server function):** In `upload_image()`, check `file_bytes.len() > 10_485_760` before any processing
- Return **413 Payload Too Large** with JSON error: `{ "error": "File exceeds 10 MB limit" }`

**File changes:**
- `src/server.rs` — add `.layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))` to the upload route layer
- `src/server_functions/upload.rs` — size check at top of `upload_image()`: `if file_bytes.len() > 10_485_760 { return Err("File exceeds 10 MB limit".into()); }`

### 3. Ownership Verification

**Problem:** A user could delete another user's image by guessing or constructing their storage key.

**Solution:** Verify the image key belongs to the authenticated user before allowing deletion.

**Implementation:**
- In `delete_image()`: assert `image_key` starts with `recipes/{user_id}/` or `avatars/{user_id}/`
- If the prefix doesn't match the authenticated user's ID, return **403 Forbidden**: `{ "error": "Access denied: image does not belong to this user" }`
- Upload keys are always scoped to the uploading user's ID (already enforced by key generation)

**File changes:**
- `src/server_functions/upload.rs` — add ownership check in `delete_image()`:
  ```rust
  let prefix = format!("recipes/{}/", user_id);
  let avatar_prefix = format!("avatars/{}/", user_id);
  if !image_key.starts_with(&prefix) && !image_key.starts_with(&avatar_prefix) {
      return Err("Access denied: image does not belong to this user".into());
  }
  ```

### 4. Path Traversal Prevention

**Problem:** User-provided filenames containing `../` could escape the intended storage directory.

**Solution:** Sanitize all user-provided filenames; generated keys use only UUID + sanitized extension.

**Implementation:**
- Strip `../`, `..\\`, and null bytes from any user-provided filename
- Reject filenames containing `/` or `\` path separators (after stripping traversal sequences)
- Extract only the file extension from the sanitized filename
- Generated storage keys use format: `{prefix}/{user_id}/{uuid}.{ext}` — no user input in the key prefix, only UUID and sanitized extension

**File changes:**
- `src/storage.rs` — new function `sanitize_filename(filename: &str) -> Result<String, String>`:
  - Strip `../`, `..\\`, null bytes
  - If filename contains `/` or `\` after stripping, reject
  - Extract extension (everything after last `.`), lowercase it
  - Return sanitized extension
- `src/server_functions/upload.rs` — use `sanitize_filename()` when generating keys

### 5. Rate Limiting

**Problem:** An attacker (or buggy client) can hammer the upload endpoint, exhausting resources.

**Solution:** Token bucket rate limiter per user.

**Implementation:**
- In-memory token bucket: `DashMap<Uuid, TokenBucket>` stored in `AppState`
- `TokenBucket` struct: `{ tokens: AtomicF64, last_refill: Instant }`
- Max **20 uploads per minute** per user (configurable via `UPLOAD_RATE_LIMIT` env var, default 20)
- On each upload attempt: check bucket, consume 1 token, return **429 Too Many Requests** if exhausted
- Refill: tokens replenish at rate of `limit / 60` per second, capped at `limit`
- JSON error: `{ "error": "Rate limit exceeded: max 20 uploads per minute" }`

**File changes:**
- `src/server.rs` — add to AppState:
  ```rust
  pub upload_rate_limiter: DashMap<Uuid, TokenBucket>,
  pub upload_rate_limit: usize,
  ```
- `src/server.rs` — new middleware `upload_rate_limit_layer` that extracts user_id from auth, checks token bucket, returns 429 if exceeded
- `src/server.rs` — new struct `TokenBucket` with `try_consume() -> bool` and `refill()` methods

### 6. Storage Isolation

**Problem:** If key generation allows user input in the path prefix, a user could potentially access another user's files.

**Solution:** Keys always prefixed with user ID; no user input in key prefix.

**Implementation:**
- All recipe image keys: `recipes/{user_id}/{uuid}.{ext}`
- All avatar keys: `avatars/{user_id}/{uuid}.{ext}`
- The `{user_id}` comes from the authenticated session, never from user input
- The `{uuid}` is a server-generated UUID (uuid crate)
- The `{ext}` is the sanitized extension (see #4)
- This is already the convention from CP2; CP10 audits all key generation paths to ensure compliance

**File changes:**
- `src/server_functions/upload.rs` — audit `upload_image()`, `replace_image()`, and avatar upload to confirm key generation follows pattern
- No structural changes needed if CP2/CP4 already follow this pattern; add comments documenting the invariant

### 7. Concurrent Upload Limits

**Problem:** A user could spawn hundreds of simultaneous uploads, exhausting server memory.

**Solution:** Per-user semaphore limiting concurrent uploads.

**Implementation:**
- Max **3 simultaneous uploads** per user
- Per-user semaphore stored in AppState: `DashMap<Uuid, Arc<Semaphore>>`
- On upload: acquire permit from user's semaphore; if unavailable (all 3 slots taken), return **503 Service Unavailable**: `{ "error": "Too many concurrent uploads: max 3 at a time" }`
- Permit is released when upload completes (success or failure)
- Implemented as Axum middleware that wraps the upload handler

**File changes:**
- `src/server.rs` — add to AppState:
  ```rust
  pub upload_semaphores: DashMap<Uuid, Arc<Semaphore>>,
  ```
- `src/server.rs` — new middleware `upload_concurrency_layer`:
  - Extract user_id from auth
  - Get or insert semaphore (3 permits) from `upload_semaphores`
  - `semaphore.acquire().await` — if it would block beyond a short timeout, return 503
  - Wrap handler execution in a guard that releases the permit on drop

### Exact File Changes Summary

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `mime_guess = { version = "0.16", optional = true }` and `dep:mime_guess` to server features |
| `src/storage.rs` | Add `validate_magic_bytes()`, `sanitize_filename()` |
| `src/server.rs` | Add `TokenBucket` struct, `DashMap<Uuid, TokenBucket>` rate limiter, `DashMap<Uuid, Arc<Semaphore>>` concurrency limiter, `DefaultBodyLimit` on upload route, rate limit middleware, concurrency middleware |
| `src/server_functions/upload.rs` | Add ownership check in `delete_image()`, size check in `upload_image()`, use `validate_magic_bytes()` and `sanitize_filename()` in key generation |

### Verification

```bash
# 1. Code compiles
cargo check --features server

# 2. MIME spoofing test
# Upload an .exe file renamed to test.jpg
curl -X POST http://localhost:3000/api/upload \
  -H "Authorization: Bearer <token>" \
  -F "file=@/path/to/malware.exe.jpg" | jq .
# Expected: 415 { "error": "MIME type mismatch: expected image/jpeg, found application/x-dosexec" }

# 3. Oversized file test
# Upload a 15 MB file
truncate -s 15M /tmp/bigfile.jpg
curl -X POST http://localhost:3000/api/upload \
  -H "Authorization: Bearer <token>" \
  -F "file=@/tmp/bigfile.jpg" | jq .
# Expected: 413 { "error": "File exceeds 10 MB limit" }

# 4. Cross-user deletion test
# User A tries to delete User B's image
curl -X DELETE http://localhost:3000/api/upload/recipes/other-user-uuid/image.webp \
  -H "Authorization: Bearer <user-a-token>" | jq .
# Expected: 403 { "error": "Access denied: image does not belong to this user" }

# 5. Rate limiting test
# Rapid-fire 25 uploads in one minute
for i in $(seq 1 25); do
  curl -s -o /dev/null -w "%{http_code}" -X POST http://localhost:3000/api/upload \
    -H "Authorization: Bearer <token>" \
    -F "file=@/path/to/small.jpg"
  echo " upload $i"
done
# Expected: uploads 1-20 return 200, uploads 21-25 return 429

# 6. Concurrent upload test
# 4 simultaneous uploads
for i in 1 2 3 4; do
  curl -s -o /dev/null -w "%{http_code}" -X POST http://localhost:3000/api/upload \
    -H "Authorization: Bearer <token>" \
    -F "file=@/path/to/small.jpg" &
done
wait
# Expected: 3 return 200, 1 returns 503

# 7. Path traversal test
# Filename containing ../
curl -X POST http://localhost:3000/api/upload \
  -H "Authorization: Bearer <token>" \
  -F "file=@/path/to/small.jpg;filename=../../etc/passwd.jpg" | jq .
# Expected: sanitized filename, no path traversal in storage key
```

---

## Summary Table

| CP | Feature | New Files | Modified Files | Verification |
|----|---------|-----------|----------------|--------------|
| CP1 | Storage backend | `src/storage.rs` | `Cargo.toml`, `src/server.rs`, `src/lib.rs` | `cargo check --features server`, health check test |
| CP2 | Upload API | `src/server_functions/upload.rs` | `src/server.rs`, `src/server_functions/mod.rs` | curl POST test, Minio verification |
| CP3 | Image processing | (none) | `Cargo.toml`, `src/storage.rs`, `src/server_functions/upload.rs` | Upload large image, verify WebP + thumb |
| CP4 | Profile image upload | `src/server_functions/avatar.rs` | `src/server_functions/upload.rs`, `src/server_functions/mod.rs`, `src/pages/settings/settings_profile.rs`, `src/api.rs`, `src/components/user_avatar.rs` | Upload avatar, see on settings + profile, remove avatar |
| CP5 | Upload UI + client-side compression | `src/components/base/image_upload.rs` | `src/api.rs`, `src/pages/recipe_new.rs`, `src/components/base/mod.rs`, `Cargo.toml` | Create recipe with image, see in slider, compression toggle for >1MB files |
| CP6 | Edit form + replace | (none) | `src/server_functions/upload.rs`, `src/server.rs`, `src/api.rs`, `src/components/base/image_upload.rs`, `src/pages/recipe_edit.rs` | Edit recipe, add/remove/replace images, cancel replace |
| CP7 | Step photos | (none) | `src/models/recipe.rs`, `src/pages/recipe_new.rs`, `src/pages/recipe_edit.rs`, `src/pages/recipe_detail.rs`, migration | Add photo to step, see on detail |
| CP8 | Card thumbnails | (none) | `src/components/recipe_card.rs`, `src/pages/recipe_list.rs` | Cards show thumbnails, graceful fallback |
| CP9 | Image preview editor + crop tool (enhancement) | `src/components/base/image_crop_editor.rs` | `src/components/base/image_upload.rs`, `src/components/base/mod.rs`, `assets/main.css` | Crop, zoom, pan, rotate, flip, preview, apply → upload cropped |
| CP10 | Upload security hardening | (none) | `Cargo.toml`, `src/storage.rs`, `src/server.rs`, `src/server_functions/upload.rs` | MIME spoofing → 415, oversized → 413, cross-user delete → 403, rate limit → 429, concurrent → 503, path traversal sanitized |

## Execution Order

1. CP1 -> CP2 -> CP3 (server-side pipeline, linear dependency)
2. CP4 (depends on CP2/CP3, simpler than CP5 — good quick win)
3. CP5 (can start once CP2 is done, but benefits from CP3)
4. CP6 (depends on CP5)
5. CP7 (depends on CP5, independent of CP6)
6. CP8 (depends on CP5, independent of CP6/CP7)
7. CP9 (depends on CP5 — enhancement added after basic upload flow is working; most complex checkpoint)
8. CP10 (depends on CP2 + CP6 — security hardening; can be done after core upload flow is functional, ideally before production deployment)

Recommended: CP1 -> CP2 -> CP3 -> CP4 -> CP5 -> CP6 -> CP7 -> CP8 -> CP9 -> CP10 (sequential minimizes integration risk; CP10 is the final hardening pass before production)
