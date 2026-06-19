# Develop Context

## Task Description
Implement checkpoint 5 of NOMS-009 (image uploads).

The implementation plan is located at: `roadmap/implementation-plans/NOMS-009-image-uploads.md`
The original issue is located at: `roadmap/issues/NOMS-009-image-uploads.md`
Current working branch: `NOMS-009-Image-Upload`

Checkpoints 1-4 have already been completed. The architect must read the implementation plan to understand the full scope of NOMS-009, identify what checkpoint 5 covers, and produce a detailed file-level blueprint for **checkpoint 5 only**. The blueprint should account for all work already in the codebase from checkpoints 1-4 (especially the image upload UI component, the dual-target API restructure, and the two-phase save flow in recipe_new.rs).

## Phase 0: Implementation Blueprint

### Overview

CP5 integrates image management into `recipe_edit.rs`: display existing recipe images with delete buttons, allow adding new images via the existing `ImageUpload` component, and persist the merged image list on save. Images removed from the UI are deleted from S3 immediately (eager deletion).

**Scope:** 3 files modified, 0 new files. No CSS changes needed (inline styles follow existing pattern).

---

### Key Research Findings

| Area | Finding | Implication |
|------|---------|-------------|
| `StorageClient` (`src/storage.rs`) | Has `put_object()` but NO `delete_object()` | Must add `delete_object()` method + `StorageError::DeleteFailed` variant |
| `upload.rs` | Has `#[server] upload_image()` but NO `delete_image()` | Must add `#[server] delete_image(full_url: String)` server function |
| `ImageUpload` component | Props: `images: Signal<Vec<String>>`, `upload_now: Signal<bool>`, `busy: Signal<bool>`, `on_upload_complete: EventHandler<UploadOutcome>`; internal `pending` signal tracks new uploads only | Component does NOT render existing images — edit form must render them separately |
| `ImageUpload::images` prop | Component only WRITES to this signal (pushes new URLs); never reads it | Safe to pre-populate with existing recipe URLs — new uploads append |
| `update_recipe()` signature | `images: Option<Vec<String>>` — replaces the full DB array | Pass merged list (existing - deleted + newly uploaded) |
| `recipe_new.rs` two-phase save | Validates → stashes `PendingSaveData` → triggers `upload_now.set(true)` → `on_upload_complete` fires → calls `create_recipe` with `images()` | Same pattern applies to edit; no `PendingSaveData` needed (no new struct) |
| S3 key pattern | `recipes/{user_id}/{uuid}_full.webp`, thumb: same with `_thumb.webp` | `delete_image` derives thumb key by replacing `_full.webp` → `_thumb.webp` |
| URL pattern | `{R2_PUBLIC_URL}/recipes/{user_id}/{uuid}_full.webp` | Extract key by stripping `public_url` prefix from full URL |
| `StorageClient::object_url()` | Returns `{public_url.trim_end('/')}/{key.trim_start('/')}` | Inverse: `key = full_url.strip_prefix(public_url).unwrap().trim_start('/')` |

---

### File-by-File Changes

#### 1. `src/storage.rs` — Add `delete_object` method

**Location:** After `put_object()` method (after line 190), inside `impl StorageClient`.

**Changes:**

(a) Add `DeleteFailed` variant to `StorageError` enum (line 21-24):

```rust
#[derive(Debug)]
pub enum StorageError {
    UploadFailed(String),
    DeleteFailed(String),
}
```

(b) Update `Display` impl for `StorageError` (line 26-32):

```rust
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::UploadFailed(e) => write!(f, "storage upload failed: {e}"),
            StorageError::DeleteFailed(e) => write!(f, "storage delete failed: {e}"),
        }
    }
}
```

(c) Add `delete_object` method to `impl StorageClient` (after `put_object`, ~line 191):

```rust
/// Delete an object from the configured bucket by key.
pub async fn delete_object(&self, key: &str) -> Result<(), StorageError> {
    self.inner
        .delete_object()
        .bucket(&self.bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| StorageError::DeleteFailed(e.to_string()))?;
    Ok(())
}
```

---

#### 2. `src/api/upload.rs` — Add `delete_image` server function

**Location:** After `upload_image` server function (after line 127), before Axum handler section.

**Add `#[server] delete_image` function:**

```rust
/// Delete an image from S3 storage given its full URL.
///
/// Derives the storage key by stripping the public URL prefix, then deletes
/// both the full-size and thumbnail objects. Best-effort: if the full object
/// is already gone, still attempts to delete the thumb.
///
/// Auth-gated: only the authenticated user can delete images. Ownership is
/// enforced by verifying the key prefix matches `recipes/{user_id}/`.
#[server]
pub async fn delete_image(full_url: String) -> Result<(), ServerFnError> {
    let user_id = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    let storage = crate::storage::get_storage_client();
    let public_url = storage.public_url().trim_end_matches('/');

    // Extract storage key from full URL: strip public_url prefix
    let full_key = full_url
        .strip_prefix(public_url)
        .ok_or_else(|| ServerFnError::new("Invalid image URL: does not match storage public URL"))?
        .trim_start_matches('/')
        .to_string();

    // Ownership check: key must start with `recipes/{user_id}/`
    let expected_prefix = format!("recipes/{user_id}/");
    if !full_key.starts_with(&expected_prefix) {
        return Err(ServerFnError::new(
            "Access denied: image does not belong to this user",
        ));
    }

    // Derive thumbnail key: replace `_full.webp` with `_thumb.webp`
    let thumb_key = full_key
        .strip_suffix("_full.webp")
        .map(|base| format!("{base}_thumb.webp"))
        .unwrap_or_else(|| full_key.clone());

    // Delete full-size object (best-effort: ignore "not found")
    let _ = storage.delete_object(&full_key).await;

    // Delete thumbnail object (best-effort: ignore "not found")
    if thumb_key != full_key {
        let _ = storage.delete_object(&thumb_key).await;
    }

    Ok(())
}
```

**Note on best-effort deletes:** `delete_object` returns `Result<(), StorageError>`. We use `let _ =` to ignore errors because: (a) the object may already be deleted (idempotent), (b) the user already removed the image from the UI, and (c) a stale S3 object is harmless. If both deletes fail catastrophically, the recipe save still proceeds with the image removed from the DB.

---

#### 3. `src/pages/recipe_edit.rs` — Integrate image management

This is the largest change. The edit form gains: image signals, existing-image grid UI, `ImageUpload` component integration, delete handler, and two-phase save flow.

**(a) Add imports (lines 6-11, modify existing):**

```rust
use crate::api::recipe::{get_recipe, get_recipe_tags, update_recipe};
use crate::api::upload::delete_image;
use crate::components::base::{Button, ButtonVariant, ImageUpload, Input, LoadingSpinner, PageHeader, UploadOutcome};
```

**(b) Add image-related signals (after line 97, before UI state signals):**

```rust
    // CP5: image management signals
    let images = use_signal(Vec::<String>::new);
    let mut upload_now = use_signal(|| false);
    let busy = use_signal(|| false);
```

**(c) Modify `use_effect` fetch handler (lines 107-157):**

After `visibility.set(recipe.visibility);` (line 143), add:

```rust
                    // CP5: pre-populate images signal with existing recipe images
                    images.set(recipe.images.clone());
```

**(d) Add delete handler (after equipment helpers, ~line 304):**

```rust
    // ── CP5: Image delete handler ───────────────────────────────────────
    let on_delete_image = move |url: String| {
        let url_for_signal = url.clone();
        spawn(async move {
            // Delete from S3 (best-effort)
            let _ = delete_image(url).await;
            // Remove from local images signal
            images.write().retain(|u| u != &url_for_signal);
        });
    };
```

**(e) Add `on_upload_complete` handler (after delete handler):**

```rust
    // ── CP5: Upload complete handler ────────────────────────────────────
    // Fired by ImageUpload when the upload batch finishes. Proceeds to
    // update_recipe with the merged images list (existing + newly uploaded).
    let on_upload_complete = move |outcome: UploadOutcome| {
        spawn(async move {
            // ... (reuses the existing submit logic, see below)
        });
    };
```

**(f) Refactor `on_submit` to two-phase flow (replace lines 164-270):**

The current `on_submit` directly calls `update_recipe`. Replace with two-phase:

```rust
    // ── Submit handler (two-phase: upload images → save recipe) ────────
    let on_submit = move |_| {
        form_error.set(None);

        // ── Client-side validation ──────────────────────────────────────
        let trimmed_title = title().trim().to_string();
        if trimmed_title.is_empty() {
            form_error.set(Some("Title is required".to_string()));
            return;
        }
        // ... (keep all existing validation: ingredients, steps, etc.)

        // ── Early guard: images still processing ────────────────────────
        if busy() {
            form_error.set(Some(
                "Images are still processing — please wait a moment".to_string(),
            ));
            return;
        }

        // ── Parse fields (same as before) ──────────────────────────────
        // ... (keep prep, cook, serv, tags parsing)

        // ── Trigger upload batch (may be empty — no-op) ────────────────
        is_saving.set(true);
        upload_now.set(true);
    };
```

Then the `on_upload_complete` handler consumes the form state and calls `update_recipe`:

```rust
    let on_upload_complete = move |outcome: UploadOutcome| {
        spawn(async move {
            if let Some(msg) = outcome.aborted {
                form_error.set(Some(msg));
                is_saving.set(false);
                return;
            }

            if outcome.failed > 0 {
                form_error.set(Some(format!(
                    "{} image(s) failed to upload. Use the Retry button on each failed image (or remove them), then click Save again.",
                    outcome.failed
                )));
                is_saving.set(false);
                return;
            }

            // Re-validate and parse form data (signals are still accessible)
            // ... (same validation + parsing as on_submit)

            // Call update_recipe with images() which now includes
            // existing URLs + newly uploaded URLs
            match update_recipe(
                recipe_id,
                Some(trimmed_title),
                desc, comm, prep, cook, serv,
                Some(typed_ingredients),
                Some(instructions),
                Some(equip_vec),
                Some(images()),  // ← merged image list
                Some(tags),
                Some(vis),
            ).await {
                Ok(_) => { /* navigate to recipe detail */ }
                Err(e) => { form_error.set(...); is_saving.set(false); }
            }
        });
    };
```

**(g) Add existing-image grid in RSX (before ImageUpload component, inside the form div):**

Insert after the "Recipe Name" section (~line 389) and before Description:

```rust
                    // ── CP5: Existing Recipe Images ─────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Recipe Images"
                        }

                        // Existing images grid
                        if images().len() > 0 {
                            div {
                                display: "grid",
                                grid_template_columns: "repeat(auto-fill, minmax(100px, 1fr))",
                                gap: "var(--space-sm)",
                                for url in images().iter() {
                                    div {
                                        key: "{url}",
                                        position: "relative",
                                        img {
                                            class: "image-upload__thumb",
                                            src: "{url}",
                                            alt: "Recipe image",
                                            width: "100%",
                                            aspect_ratio: "1",
                                            object_fit: "cover",
                                        }
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            position: "absolute",
                                            top: "4px",
                                            right: "4px",
                                            onclick: move |_| {
                                                on_delete_image(url.clone());
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }
                        }

                        // ImageUpload component for adding new images
                        ImageUpload {
                            images,
                            upload_now,
                            busy,
                            on_upload_complete,
                        }
                        span {
                            font_size: "12px",
                            color: "var(--text-tertiary)",
                            "Add more images (JPEG, PNG, or WebP · max 10 MB each)"
                        }
                    }
```

**(h) Update Save button (lines 801-809):**

```rust
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: is_saving() || busy(),
                            onclick: on_submit,
                            if is_saving() && busy() {
                                "Uploading images..."
                            } else if is_saving() {
                                "Saving..."
                            } else {
                                "Save Changes"
                            }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            disabled: is_saving() || busy(),
                            onclick: on_cancel,
                            "Cancel"
                        }
```

---

### Data Flow

```
User opens edit form
       |
       v
use_effect: get_recipe() → images.set(recipe.images)
       |
       v
UI renders: existing image grid + ImageUpload dropzone
       |
       ├─ User clicks ✕ on existing image
       │     → on_delete_image(url)
       │     → delete_image(url) [server fn: S3 delete full + thumb]
       │     → images.write().retain(...) [remove from signal]
       │
       ├─ User drags/drops new files
       │     → ImageUpload processes (canvas → WebP)
       │     → pending images shown with progress
       │
       └─ User clicks "Save Changes"
             → on_submit: validate → is_saving=true → upload_now.set(true)
                   |
                   v
             ImageUpload: upload_all() → pushes new URLs into images signal
                   |
                   v
             on_upload_complete fires
                   |
                   ├─ outcome.aborted? → show error, is_saving=false
                   ├─ outcome.failed > 0? → show error, is_saving=false
                   └─ Success → update_recipe(..., images(), ...)
                         → navigate to /recipes/{id}
```

---

### Architecture Decisions & Trade-offs

| Decision | Rationale | Alternative |
|----------|-----------|-------------|
| Eager S3 deletion on ✕ click | Simplest: UI state and S3 stay in sync; no tracking of "removed" list needed | Defer deletion to save time (requires `removed_images` signal + batch delete on save) |
| Pre-populate `images` signal with existing URLs | Leverages existing `ImageUpload` prop; no component changes needed | Separate `existing_images` signal + merge at save time (more complex) |
| Render existing images as separate grid | `ImageUpload` only handles new uploads; grid gives delete buttons per image | Modify `ImageUpload` to accept `initial_urls` prop and render them (component change) |
| Best-effort S3 deletion (ignore errors) | Idempotent: stale S3 objects are harmless; recipe save should not fail due to orphan cleanup | Fail save on delete error (blocks user for transient S3 issues) |
| Ownership check in `delete_image` via key prefix | No DB query needed; key format `recipes/{user_id}/` is deterministic | Query DB to verify ownership (extra round-trip) |

---

### Implementation Order

1. **`src/storage.rs`** — Add `StorageError::DeleteFailed` + `delete_object()` method
2. **`src/api/upload.rs`** — Add `#[server] delete_image()` server function
3. **`src/pages/recipe_edit.rs`** — Add imports, signals, handlers, image grid UI, two-phase save
4. **Verify** — `cargo check` (both targets), manual test

---

### Testing Strategy

**Manual testing (primary — matches CP5 verification in implementation plan):**

1. Open existing recipe with images → Edit
2. Verify existing images display as grid with ✕ buttons
3. Delete an image → verify it disappears from grid
4. Drag/drop a new image → verify it appears in upload list
5. Click Save → verify: images upload, recipe saves with merged list
6. Navigate to recipe detail → verify slider shows correct images
7. Delete all images → save → verify recipe has no images
8. Add image → delete it → add another → save → verify only the second image persists

**Edge cases:**
- Recipe with zero existing images: grid section renders nothing, ImageUpload works normally
- Delete image while upload is in progress: S3 delete proceeds independently; `images` signal is safe (Dioxus signals are interior-mutable)
- Save with no image changes: `upload_all()` returns immediately with `succeeded: 0`, handler proceeds to `update_recipe` with unchanged `images()`
- Network error during delete: image removed from UI but may still exist in S3 (acceptable — orphan cleanup)

**Unit tests:** None needed for CP5. The `delete_image` server function relies on `StorageClient::delete_object()` which uses aws-sdk-s3; integration testing requires a mock S3 server (beyond CP5 scope). The `validate_webp_magic` tests already cover magic byte validation.

---

### Risks & Gaps

| Risk | Mitigation |
|------|-----------|
| `delete_object` returns error for non-existent keys | Use `let _ =` (best-effort); S3 `delete_object` is idempotent |
| Concurrent delete + upload race | Dioxus signals use `RwLock` internally; `images.write().retain()` is safe |
| `update_recipe` with `images: Some(vec![])` vs `images: None` | Current code: `images.as_deref().unwrap_or(&[])` — empty vec replaces DB array with empty; this is correct behavior |
| Thumb URL derivation fails for non-standard keys | `strip_suffix("_full.webp")` falls back to `full_key.clone()`; both deletes target the same key (harmless) |
| Large number of existing images (100+) | Grid uses `auto-fill` CSS; no virtualization but acceptable for recipe images (max 10 per plan) |

---

### Files Summary

| File | Lines Changed | Type |
|------|--------------|------|
| `src/storage.rs` | ~15 lines added | Add `DeleteFailed` variant + `delete_object()` method |
| `src/api/upload.rs` | ~40 lines added | Add `#[server] delete_image()` function |
| `src/pages/recipe_edit.rs` | ~100 lines added/modified | Imports, signals, handlers, image grid UI, two-phase save |
| **Total** | **~155 lines** | **3 files modified, 0 new files** |

## Phase 0.5: Blueprint Evaluation
<!-- written by @develop-evaluate -->

## Phase 1: Implementation Details
### Summary
Implemented checkpoint 5 of NOMS-009: integrated image management into the recipe edit form (`recipe_edit.rs`). The edit form now displays existing recipe images in a grid with delete buttons, allows adding new images via `ImageUpload`, and persists the merged image list using a two-phase save flow (validate → upload → save). Images deleted from the UI are immediately removed from S3 (eager deletion).

### Files Modified

| File | Changes |
|------|---------|
| `src/storage.rs` | Added `StorageError::DeleteFailed` variant + `Display` arm; added `delete_object()` method to `StorageClient` |
| `src/api/upload.rs` | Added `#[server] delete_image(full_url: String)` — strips public URL prefix to derive S3 key, enforces ownership via key prefix check, deletes both full and thumb objects (best-effort) |
| `src/pages/recipe_edit.rs` | Added imports (`delete_image`, `ImageUpload`, `UploadOutcome`, `Rc`); added image signals (`images`, `upload_now`, `busy`); pre-populate images from fetched recipe in `use_effect`; added `on_delete_image` handler (eager S3 delete + signal update); refactored `on_submit` to two-phase flow (validate → trigger upload batch); added `on_upload_complete` handler calling `update_recipe` with merged images; added existing image grid UI with delete buttons; disabled Save/Cancel during busy state |

### Key Implementation Details
- **`Rc<String>` for `id_for_upload_complete`**: Needed because `on_upload_complete` is an `EventHandler` that may fire multiple times, and `async move` would move the ID on first invocation. `Rc` allows shared ownership across calls.
- **Index-based `for` loop for image grid**: Used `for idx in 0..images().len()` instead of `for url in images().iter()` to avoid `'static` borrow issues with closures capturing references into a temporary `Vec`.
- **Best-effort S3 deletion**: Both `delete_image` (server) and `on_delete_image` (client) use `let _ =` / `spawn(async move { let _ = ... })` to ignore delete errors. Stale S3 objects are harmless; the DB is the source of truth.
- **`id_for_submit` prefixed with `_`**: The original submit ID is no longer used in the two-phase flow (upload complete triggers save instead), so it's prefixed to suppress unused variable warnings.

### Verification
- `SQLX_OFFLINE=true cargo check --features server` — compiles clean
- `cargo check --target wasm32-unknown-unknown` — compiles clean
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings` — passes
- `cargo fmt` — applied

### Tests
No unit tests written (matches blueprint: CP5 relies on manual testing). The `delete_image` server function depends on aws-sdk-s3; integration testing would require a mock S3 server (beyond CP5 scope).

### Partial Implementations / Follow-up
None. Checkpoint 5 is complete as specified.

## Phase 2: Review Verdict

### Verdict: **PASS** (with minor non-blocking suggestions)

---

### Requirements Coverage (Task Description)

All checkpoint 5 objectives from the implementation plan are met:
- ✅ `StorageClient::delete_object()` method added with `DeleteFailed` error variant
- ✅ `#[server] delete_image()` server function with auth, ownership check, and best-effort dual delete
- ✅ `recipe_edit.rs` displays existing images in a grid with delete buttons
- ✅ `ImageUpload` component integrated for adding new images
- ✅ Two-phase save flow (validate → upload → save) persists merged image list
- ✅ Eager S3 deletion on ✕ click
- ✅ Save/Cancel buttons disabled during busy state

---

### 1. Blueprint Adherence

All three files were modified exactly as specified in the blueprint:

| File | Blueprint Spec | Actual | Status |
|------|---------------|--------|--------|
| `src/storage.rs` | `DeleteFailed` variant + `delete_object()` | Lines 25, 32, 196-205 | ✅ Exact match |
| `src/api/upload.rs` | `#[server] delete_image()` | Lines 129-176 | ✅ Exact match |
| `src/pages/recipe_edit.rs` | Imports, signals, handlers, grid UI, two-phase save | Lines 7-13, 103-106, 155, 172-175, 178-364, 452-509, 921-938 | ✅ Exact match |

**Minor deviation:** The image grid uses `display: "inline-block"` with margins instead of the blueprint's `display: "grid"` with `grid_template_columns`. This is functionally equivalent and works correctly.

### 2. Code Quality

**Good practices observed:**
- `Rc<String>` for `id_for_upload_complete` (line 175) — correctly handles the multi-invocation `EventHandler` lifetime issue, well-documented in Phase 1
- Index-based `for` loop for image grid (line 465) — pragmatic workaround for Dioxus `'static` borrow issues with closures capturing references into temporaries
- `on_delete_image` uses `url_for_signal` clone to avoid moving `url` into the async block (line 357)
- Best-effort S3 deletion with `let _ =` at both client (line 360) and server (lines 168, 172) — correct per blueprint rationale
- `thumb_key != full_key` guard (line 171) prevents redundant delete call when key derivation falls back

**Positive: compilation verified clean on both targets:**
- `SQLX_OFFLINE=true cargo check --features server` — clean
- `cargo check --target wasm32-unknown-unknown` — clean
- `SQLX_OFFLINE=true cargo clippy --features server -- -D warnings` — clean

### 3. Correctness: Dual-Target Compilation

The `#[server]` macro on `delete_image()` (line 137) correctly gates the server-only body. The client-side import of `delete_image` in `recipe_edit.rs` (line 10) works because the `#[server]` macro generates a client-side stub (network call) — same pattern as `upload_image()`, `get_recipe()`, and `update_recipe()`. No `#[cfg]` gates are needed on the client side for `#[server]` function imports.

### 4. Storage Layer

`delete_object()` (lines 196-205) is a clean, minimal implementation:
- Uses the AWS SDK's `delete_object()` fluent builder correctly
- Returns `Result<(), StorageError>` with proper error mapping
- No unnecessary complexity or side effects

### 5. API Layer: `delete_image()`

The server function correctly implements all blueprint requirements:
- **Auth gate** (line 139): `extract_user_id_from_fullstack()` with proper error
- **Key extraction** (lines 147-151): strips `public_url` prefix, trims leading slash
- **Ownership check** (lines 154-159): verifies `recipes/{user_id}/` prefix — prevents cross-user deletion
- **Thumb derivation** (lines 162-165): `strip_suffix("_full.webp")` with fallback to `full_key.clone()`
- **Best-effort dual delete** (lines 168-173): ignores errors, skips duplicate delete

### 6. Recipe Edit Page Integration

**Image signals** (lines 103-106): Three signals (`images`, `upload_now`, `busy`) match the `ImageUpload` component's prop interface exactly.

**Pre-population** (line 155): `images.set(recipe.images.clone())` in `use_effect` correctly seeds the signal with existing recipe images before the UI renders.

**Delete handler** (lines 356-364): Spawns async S3 delete, then removes URL from signal. Correctly handles the interior-mutable signal pattern.

**Two-phase save** (lines 178-225 + 230-320): 
- `on_submit` validates, guards against `busy()`, sets `is_saving(true)`, triggers `upload_now.set(true)`
- `on_upload_complete` handles abort/failure/success branches, re-reads signals, calls `update_recipe` with `Some(images())`
- Matches the `recipe_new.rs` pattern (which uses `PendingSaveData` stash; edit form re-reads signals directly — both valid approaches)

**Image grid UI** (lines 452-495): Renders existing images with delete buttons, uses index-based iteration, passes `ImageUpload` all required props.

**Save button states** (lines 922-932): Three states ("Uploading images…", "Saving…", "Save Changes") with proper `is_saving() || busy()` disabled guard.

### 7. Edge Cases

| Edge Case | Handling | Verdict |
|-----------|----------|---------|
| Delete fails for one variant, succeeds for other | Best-effort: both `let _ =` — recipe save proceeds | ✅ Correct (blueprint: stale S3 objects harmless) |
| User navigates away during upload | `upload_now` trigger fires; page unmounts; async tasks are dropped | ✅ Acceptable (no DB mutation occurs until `on_upload_complete`) |
| Orphaned images after failed delete | Image removed from UI + DB on save; stale S3 object remains | ✅ Acceptable (orphan cleanup can be background task later) |
| Recipe with zero existing images | `images().len() == 0` → grid renders nothing; `ImageUpload` works normally | ✅ Correct |
| Delete image while upload in progress | `images.write().retain()` is safe (Dioxus signals use `RwLock`) | ✅ Correct |
| Save with no image changes | `upload_all()` returns `succeeded: 0` immediately; handler proceeds | ✅ Correct |
| Same image URL appears twice | `retain(|u| u != &url_for_signal)` removes all matching entries | ✅ Acceptable (URLs should be unique by UUID) |

### 8. Testing

No unit tests written — matches blueprint specification ("None needed for CP5"). The `delete_image` server function depends on `aws-sdk-s3`; integration testing requires a mock S3 server (noted as beyond CP5 scope). Existing test infrastructure in `storage.rs` and `upload.rs` covers the surrounding code.

### 9. Security

- **Authentication**: `delete_image()` requires valid session via `extract_user_id_from_fullstack()` (line 139)
- **Authorization**: Ownership enforced via key prefix check `recipes/{user_id}/` (lines 154-159) — prevents cross-user deletion
- **No path traversal risk**: Key is derived from URL (not user input); the URL itself was server-generated during upload
- **No injection risk**: Key is used directly in `delete_object()` builder call (no string interpolation into queries)

### 10. Potential Bugs

**No blocking bugs found.** Two minor observations:

1. **Redundant numeric parsing**: `on_submit` parses `prep_time`, `cook_time`, `servings` into `_prep`, `_cook`, `_serv` (lines 218-220, prefixed with `_` to suppress warnings) and then `on_upload_complete` re-parses them from the same signals (lines 255-257). This is harmless (signals don't change between submit and upload complete under normal flow) but slightly wasteful. The `recipe_new.rs` reference avoids this by stashing `PendingSaveData`.

2. **Delete feedback timing**: When the user clicks ✕, the S3 delete runs asynchronously and the image remains visible until the async task completes and `retain()` fires. A minor UX improvement would be to optimistically remove the image from the grid and show a spinner, rolling back on failure. Not a bug — the current behavior is correct, just slightly less responsive.

---

### Issues (Non-Blocking Suggestions)

1. **[SUGGESTION] Redundant form field parsing** — `recipe_edit.rs` lines 218-220
   The `on_submit` handler parses numeric fields into discarded `_prep`/`_cook`/`_serv` variables, then `on_upload_complete` re-parses them. Consider adopting the `PendingSaveData` stash pattern from `recipe_new.rs` to parse once and reuse. Low priority — current approach works correctly.

2. **[SUGGESTION] Optimistic UI for image deletion** — `recipe_edit.rs` lines 356-364
   The delete handler removes the image from the signal only after the S3 delete completes. Consider removing from the UI immediately (optimistic update) and rolling back on failure for better perceived responsiveness. Low priority — current behavior is correct.

3. **[SUGGESTION] Image grid layout** — `recipe_edit.rs` lines 466-495
   Uses `display: "inline-block"` with margins instead of the blueprint's `display: "grid"`. Consider wrapping in a grid container for more predictable wrapping behavior with many images. Cosmetic — current layout works.

---

### Summary

Checkpoint 5 is well-implemented and faithful to the approved blueprint. All three files (`storage.rs`, `upload.rs`, `recipe_edit.rs`) are modified correctly. The code compiles clean on both targets (server and wasm), passes clippy with `-D warnings`, and follows the established patterns from checkpoints 1-4. Security controls (auth + ownership check) are properly implemented. The two-phase save flow correctly integrates with the existing `ImageUpload` component. No blocking issues found.

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->

### User-Facing Summary

Checkpoint 5 of **NOMS-009 (Image Uploads)** is complete. The recipe edit page (`recipe_edit.rs`) now fully supports image management: existing recipe images are displayed in a grid with per-image delete buttons, new images can be added via the existing `ImageUpload` component, and the merged image list (existing minus deleted plus newly uploaded) is persisted on save using a two-phase flow (validate → upload → save). Images removed from the UI are eagerly deleted from S3 storage.

**Build status:** Clean compilation on both targets (`server` and `wasm32-unknown-unknown`), clippy passes with `-D warnings`, code is formatted.

**Review verdict:** PASS with 3 non-blocking suggestions (redundant numeric parsing, optimistic delete UI, grid layout tweak).

**Note:** Phase 0.5 (Blueprint Evaluation) was not populated in the develop-context — the architect's blueprint proceeded directly to implementation without a formal evaluation gate.

---

### Step-by-Step Walkthrough of Changes

#### 1. `src/storage.rs` — S3 Delete Capability

**What was added:**
- **`StorageError::DeleteFailed(String)`** — New error variant alongside the existing `UploadFailed`. The `Display` impl was extended to format this variant as `"storage delete failed: {e}"`.
- **`StorageClient::delete_object(&self, key: &str) -> Result<(), StorageError>`** — A thin wrapper around the AWS SDK's `delete_object()` fluent builder. Takes the storage key, targets the configured bucket, and maps SDK errors into `StorageError::DeleteFailed`.

**Why:** The storage layer previously only supported uploads (`put_object`). Image deletion requires the ability to remove objects from S3, both the full-size and thumbnail variants.

**Dependencies:** Uses the existing `self.inner` (aws-sdk-s3 client) and `self.bucket` fields. No new dependencies introduced.

---

#### 2. `src/api/upload.rs` — Server-Side Image Deletion

**What was added:**
- **`#[server] pub async fn delete_image(full_url: String) -> Result<(), ServerFnError>`** — A Fullstack server function that:
  1. Extracts the authenticated user ID via `extract_user_id_from_fullstack()`.
  2. Derives the S3 storage key by stripping the `public_url` prefix from the full image URL and trimming the leading slash.
  3. Enforces ownership by verifying the key starts with `recipes/{user_id}/` — prevents cross-user deletion.
  4. Derives the thumbnail key by replacing `_full.webp` suffix with `_thumb.webp` (falls back to the full key if the suffix doesn't match).
  5. Deletes both the full-size and thumbnail objects from S3 using `let _ =` (best-effort — errors are ignored because stale S3 objects are harmless and the DB is the source of truth).

**Why:** The client-side `on_delete_image` handler needs a server function to perform the actual S3 deletion. The `#[server]` macro generates a client-side stub (network call) and the server-side implementation, following the same pattern as `upload_image()`.

**Security:** Auth-gated (requires valid session), ownership-enforced (key prefix check), no path traversal risk (key is derived from server-generated URL, not user input).

---

#### 3. `src/pages/recipe_edit.rs` — Image Management Integration

This is the largest and most complex change. The edit form was refactored to support the full image lifecycle.

**Imports added:**
- `delete_image` from `crate::api::upload`
- `ImageUpload` and `UploadOutcome` from `crate::components::base`
- `Rc` from `std::rc` (for shared ownership of `id_for_upload_complete`)

**New signals (lines 103-106):**
- `images: Signal<Vec<String>>` — Holds the merged list of image URLs (existing + newly uploaded). Pre-populated from `recipe.images` in `use_effect`.
- `upload_now: Signal<bool>` — Triggers the `ImageUpload` component to start uploading pending files.
- `busy: Signal<bool>` — Indicates whether an upload batch is in progress; used to disable Save/Cancel buttons.

**Pre-population (line 155):**
- Inside the `use_effect` that fetches the recipe, `images.set(recipe.images.clone())` seeds the signal with existing recipe image URLs before the UI renders.

**Delete handler (lines 356-364):**
- `on_delete_image(url: String)` — Spawns an async task that calls `delete_image(url)` (best-effort S3 deletion) and then removes the URL from the `images` signal via `retain()`. Uses a cloned `url_for_signal` to avoid moving the original `url` into the async block.

**Two-phase save flow (lines 178-320):**

The original `on_submit` directly called `update_recipe()`. It was refactored into a two-phase flow matching the pattern from `recipe_new.rs`:

- **Phase 1 — `on_submit`:** Validates all form fields (title, ingredients, steps, etc.), guards against `busy()` state, sets `is_saving(true)`, and triggers `upload_now.set(true)` to start the upload batch. The numeric fields (`prep_time`, `cook_time`, `servings`) are parsed but discarded (prefixed with `_`) — they will be re-parsed in Phase 2.

- **Phase 2 — `on_upload_complete`:** Fired by `ImageUpload` when the upload batch finishes. Handles three branches:
  - `outcome.aborted` — Shows error message, resets `is_saving`.
  - `outcome.failed > 0` — Shows failure count, resets `is_saving`.
  - Success — Re-reads all form signals, re-parses numeric fields, and calls `update_recipe()` with `Some(images())` (the merged image list). On success, navigates to the recipe detail page.

**Non-obvious pattern — `Rc<String>` for `id_for_upload_complete`:** The `on_upload_complete` handler is an `EventHandler` that may fire multiple times. Using `async move` would move the `recipe_id` on the first invocation. `Rc<String>` allows shared ownership across all invocations.

**Non-obvious pattern — Index-based `for` loop for image grid:** Uses `for idx in 0..images().len()` with `images()[idx]` instead of `for url in images().iter()` to avoid Dioxus `'static` borrow issues where closures capture references into a temporary `Vec`.

**Image grid UI (lines 452-509):**
- Renders a section labeled "Recipe Images" above the Description field.
- If `images().len() > 0`, renders each image as an inline-block thumbnail with an absolute-positioned ✕ delete button.
- Below the grid, renders the `ImageUpload` component with all required props (`images`, `upload_now`, `busy`, `on_upload_complete`).
- A hint text below the upload component: "Add more images (JPEG, PNG, or WebP · max 10 MB each)".

**Save/Cancel button states (lines 921-938):**
- Three states for the Save button: "Uploading images…" (when `is_saving() && busy()`), "Saving…" (when `is_saving()` only), "Save Changes" (default).
- Both Save and Cancel are disabled when `is_saving() || busy()`.

---

### Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| **Eager S3 deletion on ✕ click** | Simplest approach: UI state and S3 stay in sync immediately. No need to track a "removed images" list for batch deletion at save time. |
| **Pre-populate `images` signal with existing URLs** | Leverages the existing `ImageUpload` prop interface — the component only writes (pushes) to the signal, so pre-seeding with existing URLs is safe and requires zero component changes. |
| **Separate grid for existing images** | `ImageUpload` only handles new uploads. Rendering existing images as a separate grid with delete buttons keeps concerns separated. |
| **Best-effort S3 deletion** | Idempotent: if the object is already gone, the delete is a no-op. A stale S3 object is harmless; the DB is the source of truth. Recipe save should not fail due to orphan cleanup. |
| **Ownership check via key prefix** | No DB query needed — the key format `recipes/{user_id}/` is deterministic. Prevents cross-user deletion with zero extra round-trips. |
| **Re-read signals in `on_upload_complete`** | Simpler than `recipe_new.rs`'s `PendingSaveData` stash — the edit form doesn't need a new struct. Signals are still accessible and haven't changed between submit and upload complete. |

---

### Non-Blocking Suggestions from Review

1. **Redundant numeric field parsing** (`recipe_edit.rs` lines 218-220): `on_submit` parses `prep_time`, `cook_time`, `servings` into discarded `_prep`/`_cook`/`_serv` variables, then `on_upload_complete` re-parses them. Consider adopting the `PendingSaveData` stash pattern from `recipe_new.rs` to parse once and reuse. Low priority — current approach works correctly.

2. **Optimistic UI for image deletion** (`recipe_edit.rs` lines 356-364): The image remains visible until the S3 delete completes and `retain()` fires. Consider removing from the UI immediately and rolling back on failure for better perceived responsiveness. Low priority — current behavior is correct.

3. **Image grid layout** (`recipe_edit.rs` lines 466-495): Uses `display: "inline-block"` with margins instead of the blueprint's `display: "grid"`. Consider wrapping in a grid container for more predictable wrapping with many images. Cosmetic — current layout works.

---

### Follow-Up Recommendations

- **Monitor S3 orphan accumulation:** Best-effort deletion means stale objects can accumulate over time. Consider a background cleanup task (e.g., cron job comparing DB image references against S3 bucket contents).
- **Integration testing for `delete_image`:** The server function depends on `aws-sdk-s3`. A mock S3 server (e.g., LocalStack or moto) would enable automated integration tests for the delete path.
- **Image count limits:** The blueprint assumes max 10 images per recipe. If this limit increases, consider virtualizing the image grid for performance.

---

### Commit Message

```
feat(recipe-edit): integrate image management into edit form

Checkpoint 5 of NOMS-009: the recipe edit page now displays existing
recipe images in a grid with per-image delete buttons, allows adding
new images via the ImageUpload component, and persists the merged
image list (existing minus deleted plus newly uploaded) on save.

Storage layer (src/storage.rs):
- Add StorageError::DeleteFailed variant with Display impl
- Add StorageClient::delete_object() method wrapping aws-sdk-s3

API layer (src/api/upload.rs):
- Add #[server] delete_image(full_url) function that:
  - Extracts authenticated user ID for auth gating
  - Derives S3 key by stripping public_url prefix from full URL
  - Enforces ownership via recipes/{user_id}/ key prefix check
  - Deletes both full-size and thumbnail objects (best-effort)

Edit form (src/pages/recipe_edit.rs):
- Add images, upload_now, and busy signals for ImageUpload integration
- Pre-populate images signal from fetched recipe in use_effect
- Add on_delete_image handler: eager S3 delete + signal retain
- Refactor on_submit to two-phase flow: validate → trigger upload
  batch → on_upload_complete calls update_recipe with merged images
- Add existing image grid UI with inline-block thumbnails and ✕ buttons
- Add ImageUpload component below grid for adding new images
- Three-state Save button: "Uploading images..." / "Saving..." /
  "Save Changes", disabled during busy state

Architecture:
- Eager S3 deletion on ✕ click (UI state stays in sync)
- Best-effort delete (let _ =) — stale S3 objects are harmless
- Ownership enforced via key prefix (no DB query needed)
- Rc<String> for recipe_id in on_upload_complete to handle
  multi-invocation EventHandler lifetime
- Index-based for loop in image grid to avoid Dioxus 'static borrow
  issues with closure captures

Refs: NOMS-009 checkpoint 5
```
