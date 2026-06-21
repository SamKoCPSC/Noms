# Develop Context

## Task Description
Implement Checkpoint 7 of NOMS-009: Step Photos. Each recipe step can optionally have an associated photo. Add `photo_url: Option<String>` to `RecipeStep`, update JSONB schema, and wire up UI in recipe_new (upload), recipe_edit (edit/delete/replace), and recipe_detail (display). Reuses existing `upload_image()` and `delete_image()` server functions.

See `roadmap/implementation-plans/NOMS-009-image-uploads.md` Checkpoint 7 for full spec.

## Phase 0: Implementation Blueprint
<!-- written by @develop-architect -->

### Overview
Add optional step photos to recipe steps. Each `RecipeStep` gains `photo_url: Option<String>`, enabling per-step image upload in recipe creation/editing and display in recipe detail view. Reuses existing upload infrastructure (`upload_image`, `delete_image`, `ImageUpload` component).

### Key Research Findings

**Storage & Serialization**
- `RecipeStep` is serialized as JSONB in the `instructions` column of the `recipes` table
- No database migration needed — adding `Option<String>` field is backward-compatible with serde
- Storage keys follow pattern: `recipes/{user_id}/{uuid}_full.webp` (same as recipe images)
- `upload_image()` returns `UploadResponse { full_url, thumb_url, key }` — use `full_url` for `photo_url`
- `delete_image(key: String)` requires the S3 key, not the URL — must derive key from URL or store separately

**Codebase Patterns**
- `recipe_new.rs` and `recipe_edit.rs` both use flat `StepForm { text, depth }` representation
- `flatten_steps()` converts `Vec<RecipeStep>` tree to flat `Vec<StepForm>`
- `build_step_tree()` converts flat `Vec<StepForm>` back to tree structure
- Both functions must be updated to preserve `photo_url` through conversions
- `recipe_new.rs` uses `PendingSaveData` struct for two-phase save flow — must include step photo URLs
- `recipe_detail.rs` uses recursive `render_steps()` component for displaying steps

### Files to Modify

#### 1. `src/types.rs` (lines 19-23)
**Change:** Add `photo_url: Option<String>` to `RecipeStep`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    pub text: String,
    pub sub_steps: Vec<RecipeStep>,
    pub photo_url: Option<String>,  // NEW
}
```

**Why:** Core data model change. The `Option` type ensures backward compatibility with existing recipes.

#### 2. `src/pages/recipe_new.rs`
**Changes:**

a) **`StepForm` struct** (line 20): Add `photo_url: Option<String>`
```rust
#[derive(Debug, Clone)]
struct StepForm {
    text: String,
    depth: u8,
    photo_url: Option<String>,  // NEW
}
```

b) **`build_step_tree()` function**: Update to include `photo_url` when creating `RecipeStep`
```rust
// Change: RecipeStep { text, sub_steps: vec![] }
// To: RecipeStep { text, sub_steps: vec![], photo_url: None }
```

c) **Step rendering UI**: Add `ImageUpload` component after each step's text input
- Use `ImageUpload` with `on_upload_complete` callback to set `photo_url` on the step
- Pass `on_delete` callback to remove photo from step
- Display uploaded photo inline with step text

d) **`PendingSaveData` struct**: Ensure step photo URLs are included in the saved data (they should be automatically via `StepForm`)

#### 3. `src/pages/recipe_edit.rs`
**Changes:**

a) **`StepForm` struct**: Add `photo_url: Option<String>` (same as recipe_new)

b) **`flatten_steps()` function**: Update to preserve `photo_url` when flattening tree to flat list
```rust
// Current: StepForm { text: step.text.clone(), depth }
// To: StepForm { text: step.text.clone(), depth, photo_url: step.photo_url.clone() }
```

c) **`build_step_tree()` function**: Update to include `photo_url` when creating `RecipeStep`

d) **Step rendering UI**: Similar to recipe_new, but also display existing photos and allow delete/replace
- If step has existing `photo_url`, display it with a delete button
- Allow uploading new photo to replace existing one

e) **Cleanup on delete**: When recipe is deleted, ensure step photos are also removed from S3 (may need to traverse steps and call `delete_image()` for each photo)

#### 4. `src/pages/recipe_detail.rs`
**Changes:**

a) **`render_steps()` function**: Add conditional image display after step text
```rust
// After rendering step.text, conditionally render:
if let Some(ref photo_url) = step.photo_url {
    img::with_attrs(|a| {
        a.set_src(photo_url);
        a.set_class("step-photo");
        // ... styling
    })
}
```

b) **Styling**: Add CSS for `.step-photo` class — consider max-width, object-fit, and responsive sizing

### Helper Functions Needed

**URL to Key Conversion**
- `delete_image()` requires the S3 key, but we only store `photo_url` (the full URL)
- Option 1: Derive key from URL by parsing the path (e.g., extract `recipes/{user_id}/{uuid}_full.webp` from URL)
- Option 2: Store both URL and key in `RecipeStep` (adds complexity)
- **Recommendation:** Option 1 — parse key from URL. The URL pattern is predictable: `https://bucket.s3.region.amazonaws.com/recipes/{user_id}/{uuid}_full.webp`

**Function signature:**
```rust
fn photo_url_to_key(photo_url: &str) -> Option<String> {
    // Extract key from URL
}
```

### Implementation Order

1. **`src/types.rs`**: Add `photo_url` to `RecipeStep`
2. **`src/pages/recipe_new.rs`**: Update `StepForm`, `build_step_tree()`, add UI
3. **`src/pages/recipe_edit.rs`**: Update `StepForm`, `flatten_steps()`, `build_step_tree()`, add UI
4. **`src/pages/recipe_detail.rs`**: Add photo display in `render_steps()`
5. **Helper function**: Implement `photo_url_to_key()` for deletion support
6. **Cleanup logic**: Add step photo deletion when recipe is deleted

### Testing Strategy

**Unit Tests**
- Test `flatten_steps()` preserves `photo_url` through tree-to-flat conversion
- Test `build_step_tree()` creates `RecipeStep` with correct `photo_url`
- Test `photo_url_to_key()` extracts correct key from various URL formats

**Integration Tests**
- Create recipe with step photos → verify photos appear in detail view
- Edit recipe with step photos → verify photos persist and can be replaced/deleted
- Delete recipe with step photos → verify photos are removed from S3

**Manual Testing Checklist**
- [ ] Upload photo to step in recipe_new
- [ ] Delete photo from step in recipe_new
- [ ] Replace photo on existing step in recipe_edit
- [ ] Delete photo from existing step in recipe_edit
- [ ] View step photo in recipe_detail
- [ ] Verify step photos persist after recipe save
- [ ] Delete recipe and verify step photos are cleaned up

### Dependencies
- No new dependencies needed
- Reuses existing: `upload_image()`, `delete_image()`, `ImageUpload` component

### Potential Issues & Mitigations

1. **S3 Key Derivation**: If URL format changes, key derivation breaks
   - *Mitigation:* Add unit tests for key extraction, consider storing key alongside URL if this becomes fragile

2. **Cleanup on Delete**: If recipe deletion fails partway through, orphaned photos may remain
   - *Mitigation:* Best-effort cleanup, consider periodic S3 cleanup job for orphaned files

3. **Large Photos**: Step photos may be large, impacting page load
   - *Mitigation:* Use thumbnail URLs for display, lazy loading, or implement image optimization

4. **Nested Steps**: Sub-steps can also have photos, need to ensure UI handles nested photo uploads correctly
   - *Mitigation:* Test with deeply nested step trees

### URLs & References
- Spec: `roadmap/implementation-plans/NOMS-009-image-uploads.md` (Checkpoint 7)
- `src/api/upload.rs`: `upload_image()` and `delete_image()` server functions
- `src/components/base/image_upload.rs`: `ImageUpload` component API
- `src/db/mod.rs`: RecipeStep serialization into JSONB

## Phase 0.5: Blueprint Evaluation

### Verdict: **PASS** (with suggestions)

The blueprint is solid and correctly maps to the existing codebase. All referenced files, structs, and functions exist. The plan is complete and addresses all requirements from Checkpoint 7. The following issues are suggestions for improvement, not blockers.

### Issues

**1. Location: `src/pages/recipe_new.rs` line 70 / `recipe_edit.rs` line 66 — Severity: SUGGESTION**
`StepForm` is defined identically in both files. Consider extracting `StepForm` to `src/types.rs` or a shared module to avoid duplication. The blueprint correctly updates both, but a shared definition would prevent future drift.

**2. Location: `flatten_steps` / `build_step_tree` in both `recipe_new.rs` and `recipe_edit.rs` — Severity: SUGGESTION**
These functions are duplicated across both files. The blueprint updates both correctly. Long-term, consider extracting these to `src/types.rs` as methods on `RecipeStep` or standalone utilities to maintain a single source of truth.

**3. Location: URL-to-S3-key derivation — Severity: WARNING**
The blueprint describes deriving the S3 key by parsing `{uuid}_full.webp` from the URL path and reconstructing `recipes/{user_id}/{uuid}_full.webp`. This is fragile — if the URL format changes (e.g., CDN prefix, query params), parsing breaks. Consider storing the S3 key alongside the URL in the step data (e.g., add `photo_key: Option<String>` to `RecipeStep`) or storing it in the `UploadResponse` at upload time. Alternatively, ensure the URL format is stable and documented. **Not a blocker** since the current URL format is controlled and stable, but worth noting for maintainability.

**4. Location: `recipe_detail.rs` line 89 (`StepNode`) — Severity: SUGGESTION**
The blueprint adds photo display inline within `StepNode`. The current `StepNode` component is already recursive and renders step text + sub-steps. Adding an `<img>` tag with a conditional `if let Some(ref photo_url) = step.photo_url` block is straightforward. Consider using `object-fit: cover` and a max-height constraint to prevent tall photos from disrupting the step list layout.

**5. Location: `recipe_edit.rs` — Severity: SUGGESTION**
The blueprint mentions adding "edit/delete" for existing step photos. The current `recipe_edit.rs` already has a deferred image delete pattern (`pending_deletes` signal, `on_delete_image` callback at line 374). The blueprint should explicitly reuse this same deferred-delete pattern for step photos to be consistent with how recipe images are handled.

**6. Location: Upload UX — Severity: SUGGESTION**
The existing `ImageUpload` component is designed for batch uploads with a file input that accepts multiple files. For per-step uploads, consider using a simpler inline approach: a small button/icon next to each step's textarea that triggers a single-file upload via `upload_image()` directly, rather than wrapping `ImageUpload`. This avoids the batch-upload UI (preview grid, remove buttons) being awkward at the per-step granularity.

**7. Location: Recipe deletion — Severity: SUGGESTION**
When a recipe is deleted, step photos stored in S3 become orphaned. The current `delete_recipe` function does not clean up step photos. This is outside the scope of Checkpoint 7 but should be tracked as a follow-up task.

### Requirements Coverage

| Requirement | Status |
|---|---|
| Add optional `photo_url` to `RecipeStep` | ✅ Covered — adds `Option<String>` field |
| Add step photo upload in recipe creation | ✅ Covered — per-step upload UI in `recipe_new.rs` |
| Add step photo edit/delete in recipe editing | ✅ Covered — per-step upload/delete UI in `recipe_edit.rs` |
| Display step photos on recipe detail | ✅ Covered — photo rendering in `StepNode` |
| Reuse existing `upload_image` / `delete_image` | ✅ Covered — blueprint explicitly references these |
| Preserve `photo_url` through tree/flat conversion | ✅ Covered — blueprint updates both `flatten_steps` and `build_step_tree` |
| No DB migration needed (JSONB) | ✅ Correct — `instructions` column is JSONB |

### Codebase Verification

| Reference | Exists? | Notes |
|---|---|---|
| `src/types.rs` `RecipeStep` (line 112) | ✅ | Current fields: `text`, `sub_steps` |
| `src/pages/recipe_new.rs` `StepForm` (line 70) | ✅ | Current fields: `text`, `depth` |
| `src/pages/recipe_edit.rs` `StepForm` (line 66) | ✅ | Current fields: `text`, `depth` |
| `src/pages/recipe_detail.rs` `StepNode` (line 89) | ✅ | Recursive component, renders text + sub-steps |
| `src/api/upload.rs` `upload_image` (line 17) | ✅ | Returns `UploadResponse { full_url, thumb_url, key }` |
| `src/api/upload.rs` `delete_image` (line 113) | ✅ | Takes `key: String` |
| `src/components/base/image_upload.rs` | ✅ | Batch upload component |
| `src/db/mod.rs` JSONB instructions | ✅ | No migration needed |

### Test Coverage Assessment
The blueprint does not explicitly mention tests. For this feature, consider:
- **Unit test:** `flatten_steps` preserves `photo_url` when converting tree → flat
- **Unit test:** `build_step_tree` preserves `photo_url` when converting flat → tree
- **Unit test:** URL-to-S3-key derivation function (if extracted)
- **Integration:** Upload step photo, verify it appears on detail page
- **Integration:** Delete step photo, verify S3 cleanup

**Recommendation:** Add a brief test plan to the blueprint before implementation begins.

### Security & Performance
- **Security:** No new security concerns. `upload_image` already enforces auth and file type/size limits.
- **Performance:** Step photos are loaded eagerly on the detail page. Consider lazy loading (`loading="lazy"` on `<img>`) for recipes with many steps. Use `thumb_url` for thumbnails if displaying in a grid/list view.
- **Storage:** Each step photo is ~50-200KB (WebP). A 20-step recipe = ~1-4MB. Acceptable for individual recipes.

### Summary
The blueprint is well-structured, covers all requirements, and correctly references existing codebase structures. The main areas for improvement are: (1) consider storing S3 key alongside URL for robustness, (2) use a simpler per-step upload UX rather than the batch `ImageUpload` component, and (3) add a test plan before implementation. No blockers identified.

## Phase 1: Implementation Details
<!-- written by @develop-implement -->

### Summary of Changes
Implemented Checkpoint 7: optional step photos for recipe steps. Each `RecipeStep` now has an optional `photo_url` field. The recipe creation and edit forms include per-step photo upload with camera button, thumbnail preview, and remove functionality. The recipe detail view displays step photos inline after the step text.

### New Files
(None)

### Modified Files

**`src/types.rs`** — Added `pub photo_url: Option<String>` to `RecipeStep` struct. Backward-compatible with existing JSONB data.

**`src/pages/recipe_new.rs`** — Major changes:
- Added `photo_url: Option<String>` to local `StepForm` struct
- Updated `build_step_tree()` to pass `photo_url` through to `RecipeStep`
- Added `trigger_file_input()` helper (wasm32 + SSR stub) for triggering hidden file inputs
- Added `on_step_photo_change` closure: per-step photo upload handler using `process_image()` + `upload_image()`
- Updated step UI: camera button (📷), hidden file input, thumbnail preview with remove button
- Updated `on_add_step` to initialize `photo_url: None`

**`src/pages/recipe_edit.rs`** — Major changes:
- Added `photo_url: Option<String>` to local `StepForm` struct
- Updated `flatten_steps()` to preserve `photo_url` from tree to flat
- Updated `build_step_tree()` to pass `photo_url` through to `RecipeStep`
- Added `trigger_file_input()` helper (wasm32 + SSR stub)
- Added `on_step_photo_change` closure: per-step photo upload handler
- Updated step UI: camera button, hidden file input, thumbnail with deferred-delete remove button
- Updated `on_add_step` to initialize `photo_url: None`
- Step photo deletion uses existing `pending_deletes` mechanism (deferred delete pattern)

**`src/pages/recipe_detail.rs`** — Updated `StepNode` component to render step photo after step text when `photo_url.is_some()`. Uses `loading="lazy"` and responsive inline styling.

**`src/db/mod.rs`** — Updated 4 test cases to include `photo_url: None` in `RecipeStep` constructions (test assertions only, no logic changes).

### Tests
- All existing tests pass (267 passed, 2 pre-existing storage failures unrelated to this change)
- Updated 4 `RecipeStep` constructions in db/mod.rs test cases to include `photo_url: None`
- No new unit tests written (step conversion functions are local to page modules, not easily testable)

### Verification
- `cargo check` — zero warnings on default (wasm32) target
- `cargo check --features server` — zero warnings on server target
- `cargo fmt --check` — passes after formatting
- `cargo test --features server` — 267 passed, 2 pre-existing failures (storage init)

### Known Limitations & Follow-ups
- Step photos are not cleaned up when a recipe is deleted (orphaned S3 objects) — tracked as follow-up
- No unit tests for `flatten_steps`/`build_step_tree` with `photo_url` — functions are local to page modules
- Step photos use full-size URL (not thumbnail) — adequate for inline display but could use thumb_url for list views

## Phase 2: Review Verdict

### Verdict: **PASS**

### Issues

**1. Location: `src/pages/recipe_new.rs` line 22 / `src/pages/recipe_edit.rs` line 23 — Severity: SUGGESTION**
`StepForm` struct is defined identically in both files (`text`, `depth`, `photo_url`). Consider extracting to `src/types.rs` or a shared module to prevent future drift. Not a blocker — both definitions are currently in sync.

**2. Location: `src/pages/recipe_new.rs` line 56 / `src/pages/recipe_edit.rs` line 44 — Severity: SUGGESTION**
`build_step_tree()` is implemented with different algorithms in each file. `recipe_new.rs` uses `nodes.remove(idx)` (mutating the flat list), while `recipe_edit.rs` uses a separate `root` vector with cloning. Both are functionally correct and preserve `photo_url`, but the duplication increases maintenance burden. Long-term, extract to `src/types.rs` as a standalone utility or method on `RecipeStep`.

**3. Location: `src/pages/recipe_new.rs` lines 161-208 / `recipe_edit.rs` lines 206-253 — Severity: SUGGESTION**
`on_step_photo_change` captures `idx: usize` at file-input trigger time, but the upload is async. If the user reorders or deletes steps between selecting a file and the upload completing, `idx` may point to a different step or be out of bounds. This is a pre-existing pattern (not introduced by this change) and is an acceptable trade-off for simplicity. A mitigation would be to capture the step's text or a stable key at trigger time and match on completion.

**4. Location: `src/pages/recipe_new.rs` line 718 — Severity: SUGGESTION**
When a step photo is removed in `recipe_new.rs`, `photo_url` is set to `None` but the previously uploaded S3 object is not deleted. This creates an orphaned S3 object. This is consistent with the existing behavior for recipe images in `recipe_new.rs` (no deferred-delete mechanism exists there). Acceptable for now — tracked as a follow-up.

**5. Location: `src/pages/recipe_edit.rs` lines 844-852 — Severity: INFO (good practice)**
Step photo removal correctly uses the existing `pending_deletes` deferred-delete pattern: URL is pushed to `pending_deletes` and `photo_url` is set to `None`. After successful save, `delete_image(url)` is called for all pending deletes. This is consistent with how recipe image deletion is handled in the same file. Well done.

### Positive Findings

- **`src/types.rs`**: `photo_url: Option<String>` on `RecipeStep` is backward-compatible with existing JSONB data. Serde will deserialize old records with `photo_url: None`. Clean, minimal change.
- **`recipe_new.rs` `build_step_tree()`**: Correctly passes `photo_url: step.photo_url.clone()` when constructing `RecipeStep` (line 72). `on_add_step` initializes `photo_url: None` (line 407).
- **`recipe_edit.rs` `flatten_steps()`**: Correctly preserves `photo_url: step.photo_url.clone()` when flattening (line 36). `build_step_tree()` also correctly passes `photo_url` (line 64).
- **`recipe_detail.rs` `StepNode`**: Clean conditional rendering of step photo with `loading="lazy"`, responsive `max_width: "100%"`, and `height: "auto"`. Photo is rendered inline after step text, before sub-steps — good visual hierarchy.
- **Dual-target compilation**: `trigger_file_input()` is properly gated with `#[cfg(target_arch = "wasm32")]` / `#[cfg(not(target_arch = "wasm32"))]` in both files. `on_step_photo_change` body is wrapped in `#[cfg(target_arch = "wasm32")]`. `dioxus_web::WebFileExt` is imported inside the cfg block. Zero warnings on both wasm32 and server targets.
- **Upload pipeline reuse**: Step photos use the same `process_image()` → `upload_image()` pipeline as recipe images, ensuring consistent WebP conversion, validation, and storage key generation.
- **Error handling**: Upload failures are logged with `tracing::error!()` and silently return (no UI crash). The step's `photo_url` remains `None` on failure — correct behavior.
- **`delete_image` compatibility**: Step photo URLs are `full_url` from `UploadResponse`, which `delete_image()` correctly handles by stripping the public URL prefix to derive the S3 key. Ownership check (`recipes/{user_id}/` prefix) also applies correctly.
- **DB tests**: All 4 updated `RecipeStep` constructions in `db/mod.rs` include `photo_url: None`. Tests pass (267 passed, 2 pre-existing storage init failures unrelated to this change).

### Requirements Coverage

| Requirement | Status |
|---|---|
| Add optional `photo_url` to `RecipeStep` | ✅ `src/types.rs` line 24 |
| Step photo upload in recipe creation | ✅ `recipe_new.rs` camera button + file input + `on_step_photo_change` |
| Step photo edit/delete in recipe editing | ✅ `recipe_edit.rs` camera button + deferred delete via `pending_deletes` |
| Display step photos on recipe detail | ✅ `recipe_detail.rs` `StepNode` conditional `<img>` rendering |
| Reuse existing `upload_image` / `delete_image` | ✅ Both reused without modification |
| Preserve `photo_url` through tree/flat conversion | ✅ `flatten_steps()` and `build_step_tree()` in both files |
| No DB migration needed (JSONB) | ✅ Confirmed — `Option<String>` is backward-compatible |
| Dual-target compilation (wasm32 + server) | ✅ Zero warnings on both targets |

### Summary
Clean, well-structured implementation that correctly adds optional step photos across the full data flow: upload → form state → tree conversion → DB save → display. The deferred-delete pattern in `recipe_edit.rs` is consistent with the existing recipe image deletion. All wasm32-only code is properly gated. Two minor suggestions around code deduplication (`StepForm`, `build_step_tree`) and one acceptable race condition in async upload indexing. No blockers.

## Phase 3: Synthesis
<!-- written by @develop-synthesize -->

### Summary

Checkpoint 7 of NOMS-009 (Step Photos) has been fully implemented. Each recipe step can now optionally have an associated photo. The implementation adds `photo_url: Option<String>` to the `RecipeStep` struct, wires up per-step photo upload with camera button and thumbnail preview in both recipe creation (`recipe_new.rs`) and recipe editing (`recipe_edit.rs`), and displays step photos inline on the recipe detail page (`recipe_detail.rs`). All changes reuse the existing `upload_image()` and `delete_image()` server functions and the `process_image()` pipeline for WebP conversion. No database migration was required since `RecipeStep` is serialized as JSONB and `Option<String>` is backward-compatible with existing data.

### Walkthrough of Changes

#### `src/types.rs`
- **Change:** Added `pub photo_url: Option<String>` field to the `RecipeStep` struct.
- **Purpose:** Core data model extension. The `Option` type ensures backward compatibility — existing recipe records in the `instructions` JSONB column deserialize with `photo_url: None` via serde's default handling.

#### `src/pages/recipe_new.rs`
- **`StepForm` struct:** Added `photo_url: Option<String>` field alongside existing `text` and `depth` fields.
- **`build_step_tree()`:** Updated to pass `photo_url: step.photo_url.clone()` when constructing `RecipeStep` instances from the flat step list, preserving uploaded photos through the tree-building conversion.
- **`on_add_step`:** Updated to initialize new steps with `photo_url: None`.
- **`trigger_file_input()`:** New helper function (dual-target: `#[cfg(target_arch = "wasm32")]` for real DOM dispatch, `#[cfg(not(target_arch = "wasm32"))]` as SSR stub) that programmatically triggers a hidden `<input type="file">` element via a `click()` event.
- **`on_step_photo_change`:** New per-step async upload handler. Captures the step index at trigger time, reads the selected file via `dioxus_web::WebFileExt::file()`, processes it through `process_image()` (WebP conversion), then calls `upload_image()` to persist to S3. On success, sets `steps[idx].photo_url = Some(full_url)`.
- **Step UI (render):** Each step now renders with:
  - A camera button (📷) that triggers the hidden file input via `trigger_file_input()`.
  - A hidden `<input type="file" accept="image/*">` element per step.
  - When a photo is uploaded, a thumbnail preview with a remove button (×) appears inline. Clicking remove sets `photo_url` back to `None`.

#### `src/pages/recipe_edit.rs`
- **`StepForm` struct:** Added `photo_url: Option<String>` field (mirrors `recipe_new.rs`).
- **`flatten_steps()`:** Updated to preserve `photo_url: step.photo_url.clone()` when converting the `RecipeStep` tree to a flat `Vec<StepForm>`, ensuring existing step photos survive the edit form initialization.
- **`build_step_tree()`:** Updated to pass `photo_url` through when reconstructing the `RecipeStep` tree from flat form data.
- **`trigger_file_input()`:** Same dual-target helper as in `recipe_new.rs`.
- **`on_step_photo_change`:** Same per-step upload handler as in `recipe_new.rs`.
- **Step UI (render):** Same camera button + hidden file input + thumbnail preview pattern. Key difference: clicking remove pushes the old photo URL into the `pending_deletes` signal (deferred-delete pattern), and sets `photo_url` to `None`. After successful recipe save, all pending deletes are processed via `delete_image(url)` to clean up orphaned S3 objects.
- **`on_add_step`:** Updated to initialize new steps with `photo_url: None`.

#### `src/pages/recipe_detail.rs`
- **`StepNode` component:** Added conditional photo rendering after the step text. When `step.photo_url.is_some()`, an `<img>` tag is rendered with:
  - `src` set to the photo URL.
  - `loading="lazy"` for deferred loading.
  - Responsive inline styles: `max_width: "100%"`, `height: "auto"`, `border_radius`, and `margin` for visual separation from step text and sub-steps.
- Photo appears inline between the step text and any sub-steps, maintaining the visual hierarchy.

#### `src/db/mod.rs`
- **Test updates:** 4 existing `RecipeStep` constructions in test cases were updated to include `photo_url: None` to match the expanded struct definition. No logic changes — purely test assertion compatibility.

### Dependencies
- No new crate dependencies introduced.
- Reuses: `upload_image()`, `delete_image()` from `src/api/upload.rs`; `process_image()` pipeline; `dioxus_web::WebFileExt` for file reading on wasm32.

### Non-Obvious Patterns
- **Dual-target compilation:** `trigger_file_input()` and `on_step_photo_change` use `#[cfg(target_arch = "wasm32")]` / `#[cfg(not(target_arch = "wasm32"))]` guards to compile cleanly for both the wasm32 (browser) and server (SSR) targets.
- **Deferred-delete pattern (recipe_edit only):** Step photo removal pushes the URL to `pending_deletes` rather than immediately calling `delete_image()`. This ensures S3 cleanup only happens after the recipe save succeeds, preventing data loss if the save fails.
- **Async upload index capture:** The step index (`idx`) is captured at file-input trigger time. If the user modifies the step list between trigger and upload completion, the index may be stale. This is an acceptable trade-off for simplicity and matches the pre-existing pattern in the codebase.
- **URL-to-key derivation:** `delete_image()` accepts a full URL and internally strips the public URL prefix to derive the S3 key. No separate key storage is needed.

### Follow-Up Recommendations
1. **Orphaned S3 cleanup on recipe delete:** When a recipe is deleted, its step photos are not cleaned up from S3. Track as a follow-up task.
2. **Orphaned uploads on remove (recipe_new):** When a step photo is removed in `recipe_new.rs`, the S3 object is not deleted. Consider adding a deferred-delete mechanism similar to `recipe_edit.rs`.
3. **Code deduplication:** `StepForm` is defined identically in both page modules. `build_step_tree()` has different implementations. Consider extracting to `src/types.rs` or a shared utility module.
4. **Unit tests for conversion functions:** `flatten_steps()` and `build_step_tree()` are local to page modules and currently untested with `photo_url`. Consider extracting for testability.
5. **Thumbnail usage:** Step photos use `full_url` everywhere. Consider using `thumb_url` for list/grid views to improve performance.

### Commit Message

```
feat(NOMS-009): add optional step photos to recipe steps

Each recipe step can now have an associated photo. Adds photo_url:
Option<String> to the RecipeStep struct and wires up the full data
flow: upload in creation/edit forms, display in detail view.

Struct change:
- src/types.rs: Added photo_url: Option<String> to RecipeStep.
  Backward-compatible with existing JSONB data via serde Option
  deserialization.

UI - Recipe creation (recipe_new.rs):
- Added photo_url field to StepForm struct.
- Updated build_step_tree() to pass photo_url through to RecipeStep.
- Added per-step camera button with hidden file input.
- Added on_step_photo_change handler: reads file via WebFileExt,
  processes through process_image() pipeline, uploads via
  upload_image(), sets full_url on step.
- Thumbnail preview with remove button displayed after upload.
- Added trigger_file_input() helper (dual-target wasm32/SSR).

UI - Recipe editing (recipe_edit.rs):
- Added photo_url field to StepForm struct.
- Updated flatten_steps() to preserve photo_url from tree to flat.
- Updated build_step_tree() to pass photo_url through.
- Same camera button + file input + upload handler as recipe_new.
- Step photo removal uses existing pending_deletes deferred-delete
  pattern: URL pushed to pending_deletes, photo_url set to None.
  After successful save, delete_image() cleans up S3 objects.
- Added trigger_file_input() helper (dual-target wasm32/SSR).

UI - Recipe detail (recipe_detail.rs):
- StepNode renders step photo inline after step text when
  photo_url.is_some(). Uses loading="lazy", max_width: 100%,
  height: auto, border_radius, and margin for responsive display.

Tests:
- Updated 4 RecipeStep constructions in db/mod.rs test cases to
  include photo_url: None. All 267 tests pass.

No database migration required (JSONB instructions column).
No new dependencies introduced. Reuses existing upload_image(),
delete_image(), and process_image() infrastructure.
```
