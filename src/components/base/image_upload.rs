//! Image upload component: drag-and-drop / file picker, client-side resize +
//! WebP processing (CP3), deferred server upload (CP2 `#[server] upload_image`),
//! progress + error UI with per-item retry, size-reduction readout, and a
//! client-side 10 MB overflow guard.
//!
//! Compiles on both `wasm32-unknown-unknown` (the real client) and the server
//! target (where the component is never rendered but is still type-checked by
//! `dx serve` SSR and `cargo check --features server`). All
//! `web_sys`/`process_image`/`WebFileExt`/object-URL logic that is wasm-only
//! lives behind `#[cfg(target_arch = "wasm32")]` blocks with a non-wasm no-op
//! branch.

use dioxus::html::HasFileData;
use dioxus::prelude::*;
use std::sync::Arc;

use crate::components::base::{EditedRecipeImage, ImageEditorModal};
use crate::types::{RecipeImage, RecipeImageEdit};
use crate::utils::reorder::move_item;

// WASM-only bridge: connects Dioxus `FileData` to CP3's `process_image_*`
// pipeline. Gated at module scope so the imports don't trigger
// `unused_imports` on the server build (`warnings = "deny"`).
#[cfg(target_arch = "wasm32")]
use crate::utils::image_processing_client::process_image;
#[cfg(target_arch = "wasm32")]
use dioxus_web::WebFileExt;

// ── Constants ───────────────────────────────────────────────────────────────

/// Maximum original file size accepted by the dropzone (10 MB).
/// Matches the server's per-field limit (`crate::api::upload::MAX_FILE_SIZE`).
/// Only referenced inside wasm-gated code — `allow(dead_code)` covers the
/// server build where the wasm blocks are stripped.
#[allow(dead_code)]
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Allowed image file extensions (checked case-insensitively).
/// Only referenced inside wasm-gated code — `allow(dead_code)` covers the
/// server build where the wasm blocks are stripped.
#[allow(dead_code)]
const ALLOWED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

// ── Pure helpers (cross-platform, unit-tested) ──────────────────────────────

/// Check if a filename has an allowed image extension (case-insensitive).
/// Only referenced inside wasm-gated code — `allow(dead_code)` covers the
/// server build where the wasm blocks are stripped.
#[allow(dead_code)]
fn is_allowed_image_type(filename: &str) -> bool {
    let Some(ext) = filename.rsplit('.').next() else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();
    ALLOWED_EXTENSIONS.iter().any(|&a| a == ext)
}

/// Format a byte count as a human-readable string: `"500 B"`, `"800 KB"`,
/// `"5.0 MB"`.
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Percentage reduction from `original` to `processed`. Returns 0 when
/// `original == 0` (avoids divide-by-zero) or when the processed size is
/// not smaller than the original.
fn reduction_percent(original: u64, processed: usize) -> u64 {
    if original == 0 || (processed as u64) >= original {
        return 0;
    }
    ((original - processed as u64) * 100) / original
}

/// Return how many files from a new selection fit in the configured gallery.
fn available_image_slots(current: usize, incoming: usize, max_images: Option<usize>) -> usize {
    max_images
        .map(|limit| limit.saturating_sub(current).min(incoming))
        .unwrap_or(incoming)
}

// ── Public types ────────────────────────────────────────────────────────────

/// Outcome of an upload batch, reported to the parent via `on_upload_complete`.
#[derive(Clone, Debug, PartialEq)]
pub struct UploadOutcome {
    /// Number of images successfully uploaded this batch.
    pub succeeded: usize,
    /// Number of images that failed this batch (status `Failed`; the user
    /// can retry each via the per-item Retry button).
    pub failed: usize,
    /// Set when the batch could not run at all (e.g., some images are still
    /// `Processing`). When `Some`, succeeded/failed are 0 and the parent
    /// should surface the message and NOT call `create_recipe`.
    pub aborted: Option<String>,
}

/// Result of completing a deferred upload before a parent create/update call.
/// Taking the pending value here makes each save attempt single-use, including
/// failed or aborted attempts; a retry must enqueue the current form state.
#[derive(Debug, PartialEq)]
pub(crate) enum DeferredUploadCompletion<T> {
    NoPendingSave,
    Ready(T),
    Aborted(String),
    Failed,
}

pub(crate) fn complete_deferred_upload<T>(
    pending: &mut Option<T>,
    outcome: UploadOutcome,
) -> DeferredUploadCompletion<T> {
    let Some(pending) = pending.take() else {
        return DeferredUploadCompletion::NoPendingSave;
    };
    if let Some(problem) = outcome.aborted {
        return DeferredUploadCompletion::Aborted(problem);
    }
    if outcome.failed > 0 {
        return DeferredUploadCompletion::Failed;
    }
    DeferredUploadCompletion::Ready(pending)
}

pub(crate) fn single_gallery_image(images: &[RecipeImage]) -> Option<RecipeImage> {
    images.first().cloned()
}

/// Props for [`ImageUpload`].
#[derive(Props, Clone, PartialEq)]
pub struct ImageUploadProps {
    /// Shared sink: the component appends each successfully uploaded image
    /// record here as uploads complete. The parent reads `images()` when
    /// calling `create_recipe`.
    pub images: Signal<Vec<RecipeImage>>,
    /// Parent sets this to `true` to trigger the upload batch. The component
    /// resets it to `false` when the batch finishes — BEFORE clearing its
    /// internal `uploading` flag (this ordering prevents the `use_effect`
    /// double-trigger race; see the upload-trigger effect below).
    pub upload_now: Signal<bool>,
    /// Mirrors "any pending file is `Processing` or `Uploading`". The parent
    /// reads this to disable Save / guard submit. The component keeps it in
    /// sync via a `use_effect` over a memo of `pending` + `uploading`.
    pub busy: Signal<bool>,
    /// Number of locally selected files that have not uploaded successfully.
    /// Parents use this to mark an otherwise unchanged draft dirty.
    pub pending_count: Signal<usize>,
    /// Selects the server-controlled storage namespace for this uploader.
    pub purpose: crate::api::upload::UploadPurpose,
    /// Optional gallery capacity. Recipe forms omit this to preserve their
    /// unlimited gallery behavior; collection covers set it to one.
    #[props(default)]
    pub max_images: Option<usize>,
    /// Removes an image that was already persisted in the draft.
    pub on_remove_existing: EventHandler<String>,
    /// Fired exactly once when the upload batch finishes. The parent uses
    /// this to proceed to `create_recipe` (on success) or surface an error
    /// (on abort/failure) — NO polling loop.
    pub on_upload_complete: EventHandler<UploadOutcome>,
}

// ── Internal state types ────────────────────────────────────────────────────

/// Per-file upload lifecycle.
#[derive(Clone, PartialEq)]
#[allow(dead_code)]
enum UploadStatus {
    /// Canvas resize/WebP in progress.
    Processing,
    /// Processed; awaiting upload at save time.
    Ready,
    /// Server fn call in flight.
    Uploading,
    /// Done — uploaded URL pair already pushed into `images`.
    Uploaded,
    /// Upload failed or processed bytes exceeded the upload limit.
    Failed,
}

/// Snapshot of a ready-to-upload image: (id, full bytes, thumb bytes, name).
/// Extracted as a type alias to satisfy `clippy::type_complexity`.
type UploadItem = (usize, Arc<Vec<u8>>, Arc<Vec<u8>>, String, RecipeImageEdit);

fn upload_batch_plan(pending: &[PendingImage]) -> (Vec<UploadItem>, usize) {
    let ready = pending
        .iter()
        .filter(|item| item.status == UploadStatus::Ready)
        .map(|item| {
            (
                item.id,
                item.full_bytes.clone(),
                item.thumb_bytes.clone(),
                item.name.clone(),
                item.edit.clone(),
            )
        })
        .collect();
    let failed = pending
        .iter()
        .filter(|item| item.status == UploadStatus::Failed)
        .count();
    (ready, failed)
}

/// One selected image, its processed bytes, and its upload status.
///
/// `full_bytes` and `thumb_bytes` are `Arc<Vec<u8>>` so signal-read clones
/// (per render) are cheap ref-count bumps instead of multi-MB deep copies.
/// The bytes are written once after processing and only read at upload time.
#[derive(Clone, PartialEq)]
struct PendingImage {
    id: usize,
    /// Whether this entry existed in the draft when the gallery mounted.
    persisted: bool,
    name: String,
    /// Object URL for the preview thumbnail (wasm only); `None` on SSR.
    preview_url: Option<String>,
    /// Original File size in bytes.
    original_size: u64,
    /// Processed full-size WebP bytes.
    full_bytes: Arc<Vec<u8>>,
    /// Processed 800px thumbnail WebP bytes.
    thumb_bytes: Arc<Vec<u8>>,
    /// Edit metadata captured from the image editor modal.
    edit: RecipeImageEdit,
    /// Where this file is in the upload lifecycle.
    status: UploadStatus,
    /// Failure message when `status == Failed`.
    error: Option<String>,
    /// Uploaded record retained here so the public image list can always be
    /// rebuilt in the same order as the pending list (including after retry).
    uploaded_image: Option<RecipeImage>,
}

fn gallery_images_from_persisted(images: &[RecipeImage]) -> Vec<PendingImage> {
    images
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, image)| PendingImage {
            id,
            persisted: true,
            name: image
                .file_name
                .clone()
                .unwrap_or_else(|| "Saved recipe image".to_string()),
            preview_url: None,
            original_size: image.file_size.unwrap_or_default(),
            full_bytes: Arc::new(Vec::new()),
            thumb_bytes: Arc::new(Vec::new()),
            edit: image
                .edits
                .first()
                .cloned()
                .unwrap_or_else(RecipeImageEdit::default_rect_16x9),
            status: UploadStatus::Uploaded,
            error: None,
            uploaded_image: Some(image),
        })
        .collect()
}

fn uploaded_images_in_pending_order(pending: &[PendingImage]) -> Vec<RecipeImage> {
    pending
        .iter()
        .filter_map(|pending_image| pending_image.uploaded_image.clone())
        .collect()
}

fn pending_upload_count(pending: &[PendingImage]) -> usize {
    pending
        .iter()
        .filter(|pending_image| pending_image.status != UploadStatus::Uploaded)
        .count()
}

fn reorder_gallery_images(
    gallery: &mut Vec<PendingImage>,
    dragged_id: usize,
    target_id: usize,
) -> bool {
    let Some(from) = gallery.iter().position(|image| image.id == dragged_id) else {
        return false;
    };
    let Some(to) = gallery.iter().position(|image| image.id == target_id) else {
        return false;
    };
    move_item(gallery, from, to)
}

fn adjacent_gallery_id(
    gallery: &[PendingImage],
    current_id: usize,
    direction: i8,
) -> Option<usize> {
    let current_index = gallery.iter().position(|image| image.id == current_id)?;
    let adjacent_index = match direction {
        -1 => current_index.checked_sub(1)?,
        1 => current_index.checked_add(1)?,
        _ => return None,
    };
    gallery.get(adjacent_index).map(|image| image.id)
}

#[cfg(test)]
fn persisted_images_not_in_pending(
    images: &[RecipeImage],
    pending: &[PendingImage],
) -> Vec<RecipeImage> {
    images
        .iter()
        .filter(|image| {
            !pending.iter().any(|pending_image| {
                pending_image
                    .uploaded_image
                    .as_ref()
                    .is_some_and(|uploaded| uploaded.url == image.url)
            })
        })
        .cloned()
        .collect()
}

#[cfg(test)]
fn reorder_images_by_url(
    images: &mut Vec<RecipeImage>,
    dragged_url: &str,
    target_url: &str,
) -> bool {
    let Some(from) = images.iter().position(|image| image.url == dragged_url) else {
        return false;
    };
    let Some(to) = images.iter().position(|image| image.url == target_url) else {
        return false;
    };
    move_item(images, from, to)
}

#[cfg(test)]
fn merge_existing_and_uploaded_images(
    mut existing: Vec<RecipeImage>,
    uploaded: Vec<RecipeImage>,
) -> Vec<RecipeImage> {
    for uploaded_image in uploaded {
        if !existing
            .iter()
            .any(|existing_image| existing_image.url == uploaded_image.url)
        {
            existing.push(uploaded_image);
        }
    }
    existing
}

fn sync_uploaded_images(pending: Signal<Vec<PendingImage>>, mut images: Signal<Vec<RecipeImage>>) {
    images.set(uploaded_images_in_pending_order(&pending()));
}

fn reorder_pending_images(
    dragged_id: usize,
    target_id: usize,
    mut pending: Signal<Vec<PendingImage>>,
    images: Signal<Vec<RecipeImage>>,
) {
    reorder_gallery_images(&mut pending.write(), dragged_id, target_id);
    sync_uploaded_images(pending, images);
}

// ── Module-scope async helpers ──────────────────────────────────────────────
//
// Defined at module scope (not as `let upload_all = async move { ... }` inside
// the component) because `use_effect` re-fires when its signal dependencies
// change, and a non-`Copy` async block captured by move into the effect
// closure could only be consumed once. Passing the (Copy) signals into a
// plain `async fn` lets the effect spawn a fresh future on every re-fire.

/// Process a single picked file: validate type/size (client), create a
/// preview object URL (wasm), push a `Processing` placeholder, run the CP3
/// resize+WebP pipeline, fill the byte fields, and transition to `Ready`
/// (or `Failed` on error / overflow).
///
/// The non-wasm branch is a no-op so the function compiles on both targets
/// (the handlers never fire on the server, but `dx serve` SSR still
/// type-checks the call site). The `mut` on signal params is only used
/// inside the wasm block — `allow(unused_mut)` covers the server build.
#[allow(unused_mut)]
async fn process_one_file(
    file: dioxus::html::FileData,
    mut pending: Signal<Vec<PendingImage>>,
    mut next_id: Signal<usize>,
    mut error: Signal<Option<String>>,
    mut editor_modal_url: Signal<Option<String>>,
    mut editor_modal_img_id: Signal<Option<usize>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let name = file.name();
        let original_size = file.size();

        let Some(web_file) = file.get_web_file() else {
            return;
        };

        // Validate type by extension (the `accept` attribute is a hint, not
        // enforcement — browsers may still let users pick disallowed types).
        if !is_allowed_image_type(&name) {
            error.set(Some(format!(
                "Skipped \"{name}\": only JPEG, PNG, or WebP are allowed."
            )));
            return;
        }

        // Validate size — match the server's per-field limit.
        if original_size > MAX_FILE_SIZE {
            error.set(Some(format!(
                "Skipped \"{name}\": files must be ≤ {}. Got {}.",
                format_file_size(MAX_FILE_SIZE),
                format_file_size(original_size)
            )));
            return;
        }

        // Build a preview object URL (wasm only).
        let preview_url = web_sys::Url::create_object_url_with_blob(&web_file).ok();

        // Allocate an id + push a placeholder so the UI shows "Processing…"
        // immediately (before the await on the canvas pipeline).
        let id = next_id();
        next_id.set(id + 1);
        pending.write().push(PendingImage {
            id,
            persisted: false,
            name: name.clone(),
            preview_url: preview_url.clone(),
            original_size,
            full_bytes: Arc::new(Vec::new()),
            thumb_bytes: Arc::new(Vec::new()),
            edit: RecipeImageEdit::default_rect_16x9(),
            status: UploadStatus::Processing,
            error: None,
            uploaded_image: None,
        });

        // Open the preview modal for the newly selected image.
        if let Some(url) = &preview_url {
            editor_modal_img_id.set(Some(id));
            editor_modal_url.set(Some(url.clone()));
        }

        // Process using the standard compressed path.
        match process_image(web_file).await {
            Ok(processed) => {
                let full_len = processed.full_bytes.len();
                let mut guard = pending.write();
                if let Some(p) = guard.iter_mut().find(|p| p.id == id) {
                    p.full_bytes = Arc::new(processed.full_bytes);
                    p.thumb_bytes = Arc::new(processed.thumb_bytes);
                    if full_len > MAX_FILE_SIZE as usize {
                        p.status = UploadStatus::Failed;
                        p.error = Some(format!(
                            "Image is {} — too large even at standard quality. Use a smaller image.",
                            format_file_size(full_len as u64)
                        ));
                    } else {
                        p.status = UploadStatus::Ready;
                    }
                }
            }
            Err(msg) => {
                let mut guard = pending.write();
                if let Some(p) = guard.iter_mut().find(|p| p.id == id) {
                    p.status = UploadStatus::Failed;
                    p.error = Some(msg);
                }
            }
        }
    }

    // Non-wasm no-op: consume the params so nothing is unused.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            file,
            pending,
            next_id,
            error,
            editor_modal_url,
            editor_modal_img_id,
        );
    }
}

/// Re-attempt the upload for a single `Failed` item (per-item Retry, AC5).
/// Reuses the stashed `full_bytes`/`thumb_bytes` from the last successful
/// processing — does NOT re-process.
async fn retry_upload(
    img_id: usize,
    mut pending: Signal<Vec<PendingImage>>,
    images: Signal<Vec<RecipeImage>>,
    purpose: crate::api::upload::UploadPurpose,
    mut discard_capabilities: Signal<std::collections::HashMap<usize, String>>,
) {
    let item = pending().iter().find(|p| p.id == img_id).map(|p| {
        (
            p.full_bytes.clone(),
            p.thumb_bytes.clone(),
            p.name.clone(),
            p.edit.clone(),
        )
    });
    let Some((full, thumb, name, edit)) = item else {
        return;
    };

    if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
        p.status = UploadStatus::Uploading;
        p.error = None;
    }

    // The one unavoidable `Vec<u8>` clone — to pass bytes by value to the
    // server fn. Happens once per retry, not per render.
    let result =
        crate::api::upload::upload_image((*full).clone(), (*thumb).clone(), name, purpose).await;
    match result {
        Ok(resp) => {
            discard_capabilities
                .write()
                .insert(img_id, resp.discard_capability.clone());
            let uploaded_image =
                RecipeImage::managed(resp.asset_id, resp.full_url, Some(resp.thumb_url), edit);
            if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
                p.status = UploadStatus::Uploaded;
                p.uploaded_image = Some(uploaded_image);
            }
            sync_uploaded_images(pending, images);
        }
        Err(e) => {
            let msg = e.to_string();
            if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
                p.status = UploadStatus::Failed;
                p.error = Some(msg);
            }
        }
    }
}

/// Run the save-time upload batch: upload all `Ready` items sequentially,
/// pushing each successful image URL pair into `images`. Returns the outcome
/// (succeeded/failed counts, or an abort reason if any item is still
/// `Processing`).
async fn upload_all(
    mut pending: Signal<Vec<PendingImage>>,
    images: Signal<Vec<RecipeImage>>,
    purpose: crate::api::upload::UploadPurpose,
    mut discard_capabilities: Signal<std::collections::HashMap<usize, String>>,
) -> UploadOutcome {
    // Processing guard — abort if any item is still in `Processing`.
    if pending()
        .iter()
        .any(|p| matches!(p.status, UploadStatus::Processing))
    {
        return UploadOutcome {
            succeeded: 0,
            failed: 0,
            aborted: Some(
                "Some images are still processing — please wait a moment and try again."
                    .to_string(),
            ),
        };
    }

    // Snapshot the ids + bytes of `Ready` items (can't hold
    // `pending.write()` across `.await`).
    let (to_upload, mut failed) = upload_batch_plan(&pending());

    let mut succeeded = 0usize;

    for (id, full, thumb, name, edit) in to_upload {
        if let Some(p) = pending.write().iter_mut().find(|p| p.id == id) {
            p.status = UploadStatus::Uploading;
            p.error = None;
        }

        let result =
            crate::api::upload::upload_image((*full).clone(), (*thumb).clone(), name, purpose)
                .await;

        match result {
            Ok(resp) => {
                discard_capabilities
                    .write()
                    .insert(id, resp.discard_capability.clone());
                let uploaded_image =
                    RecipeImage::managed(resp.asset_id, resp.full_url, Some(resp.thumb_url), edit);
                if let Some(p) = pending.write().iter_mut().find(|p| p.id == id) {
                    p.status = UploadStatus::Uploaded;
                    p.uploaded_image = Some(uploaded_image);
                }
                sync_uploaded_images(pending, images);
                succeeded += 1;
            }
            Err(e) => {
                let msg = e.to_string();
                if let Some(p) = pending.write().iter_mut().find(|p| p.id == id) {
                    p.status = UploadStatus::Failed;
                    p.error = Some(msg);
                }
                failed += 1;
            }
        }
    }

    UploadOutcome {
        succeeded,
        failed,
        aborted: None,
    }
}

/// Remove a pending item by id, revoking its preview object URL (wasm only).
/// An item uploaded during a partially failed batch is left for the existing
/// orphan cleanup worker when the user removes it before saving the recipe.
fn remove_pending(
    img_id: usize,
    mut pending: Signal<Vec<PendingImage>>,
    mut images: Signal<Vec<RecipeImage>>,
) -> Option<String> {
    let uploaded_url = pending()
        .iter()
        .find(|pending_image| pending_image.id == img_id)
        .and_then(|pending_image| {
            pending_image
                .uploaded_image
                .as_ref()
                .map(|image| image.url.clone())
        });
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(p) = pending().iter().find(|p| p.id == img_id) {
            if let Some(url) = &p.preview_url {
                let _ = web_sys::Url::revoke_object_url(url);
            }
        }
    }
    pending.write().retain(|p| p.id != img_id);
    if let Some(uploaded_url) = uploaded_url.as_deref() {
        images.write().retain(|image| image.url != uploaded_url);
    }
    sync_uploaded_images(pending, images);
    uploaded_url
}

// ── Component ───────────────────────────────────────────────────────────────

/// Drag-and-drop + file-picker image upload component with client-side
/// resize/WebP processing, progress, error handling, and per-item retry.
#[component]
pub fn ImageUpload(props: ImageUploadProps) -> Element {
    let initial_gallery = gallery_images_from_persisted(&(props.images)());
    let initial_next_id = initial_gallery.len();
    let mut drag_over = use_signal(|| false);
    let mut pending = use_signal(move || initial_gallery);
    let mut uploading = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    let next_id = use_signal(move || initial_next_id);
    let mut editor_modal_url = use_signal(|| Option::<String>::None);
    let mut editor_modal_img_id = use_signal(|| Option::<usize>::None);
    let mut dragged_image_id = use_signal(|| Option::<usize>::None);
    // Raw capabilities never enter RecipeImage or any persisted form payload.
    let mut discard_capabilities = use_signal(std::collections::HashMap::<usize, String>::new);

    let images = props.images;
    let mut upload_now = props.upload_now;
    let mut busy = props.busy;
    let mut pending_count = props.pending_count;
    let on_remove_existing = props.on_remove_existing;
    let on_upload_complete = props.on_upload_complete;
    let max_images = props.max_images;
    let purpose = props.purpose;

    // ── busy sync ────────────────────────────────────────────────────────
    // Memo over `pending` + `uploading`; an effect writes the result into
    // the shared `busy` signal. Writing `busy` does not retrigger this
    // effect (it does not read `busy`), so there is no loop. The parent
    // reads `busy()` to disable the Save button.
    let busy_memo = use_memo(move || {
        pending()
            .iter()
            .any(|p| matches!(p.status, UploadStatus::Processing | UploadStatus::Uploading))
            || uploading()
    });
    use_effect(move || busy.set(busy_memo()));
    use_effect(move || pending_count.set(pending_upload_count(&pending())));

    // ── handle_files ─────────────────────────────────────────────────────
    // Shared by the `<input onchange>` and `<div ondrop>` handlers.
    let handle_files = move |files: Vec<dioxus::html::FileData>| async move {
        let current_count = pending().len();
        let available = available_image_slots(current_count, files.len(), max_images);
        if available < files.len() {
            error.set(Some(match max_images {
                Some(1) => "A collection can have only one cover image.".to_string(),
                Some(limit) => format!("This gallery can contain up to {limit} images."),
                None => unreachable!("an unlimited gallery accepts every selected file"),
            }));
        }
        for file in files.into_iter().take(available) {
            process_one_file(
                file,
                pending,
                next_id,
                error,
                editor_modal_url,
                editor_modal_img_id,
            )
            .await;
        }
    };

    // ── use_effect for the upload trigger ────────────────────────────────
    // Race fix (WARNING #2): reset `upload_now` BEFORE clearing `uploading`,
    // so the effect's re-fire (caused by `uploading.set(false)`) sees
    // `upload_now() == false` and no-ops instead of re-running the batch.
    // Do not reorder these two lines.
    use_effect(move || {
        if upload_now() && !uploading() {
            uploading.set(true);
            spawn(async move {
                let outcome = upload_all(pending, images, purpose, discard_capabilities).await;
                upload_now.set(false);
                uploading.set(false);
                on_upload_complete.call(outcome);
            });
        }
    });

    // Pre-compute the dropzone class (reactive on `drag_over`).
    let dropzone_class = if drag_over() {
        "image-upload__dropzone image-upload__dropzone--active"
    } else {
        "image-upload__dropzone"
    };

    rsx! {
        div { class: "image-upload",
            // Drop zone (clickable via wrapped label → hidden input)
            label {
                class: "{dropzone_class}",
                ondragover: move |evt| {
                    evt.prevent_default();
                    drag_over.set(true);
                },
                ondragleave: move |_| drag_over.set(false),
                ondrop: move |evt| async move {
                    evt.prevent_default();
                    drag_over.set(false);
                    handle_files(evt.files()).await;
                },
                // Hidden file input triggered by the label click
                input {
                    r#type: "file",
                    accept: "image/jpeg,image/png,image/webp",
                    multiple: max_images != Some(1),
                    class: "image-upload__file-input",
                    aria_label: if max_images == Some(1) { "Choose collection cover image" } else { "Choose recipe images" },
                    onchange: move |evt| async move {
                        handle_files(evt.files()).await;
                    },
                }
                span { "📷  Choose or drag images here" }
                span { class: "image-upload__hint", "JPEG, PNG, or WebP · max 10 MB each" }
            }

            // Error banner
            if let Some(err) = error() {
                div { class: "image-upload__error image-upload__error--banner", role: "alert", aria_live: "assertive", "{err}" }
            }

            // One ordered gallery contains persisted and newly selected images.
            for img in pending().iter() {
                {
                    let img_id = img.id;
                    let img_persisted = img.persisted;
                    let img_name = img.name.clone();
                    let img_preview = img.preview_url.clone();
                    let img_uploaded = img.uploaded_image.clone();
                    let img_original = img.original_size;
                    let img_full_len = img.full_bytes.len();
                    let img_status = img.status.clone();
                    let img_error = img.error.clone();
                    let img_edit = img.edit.clone();
                    // The web renderer uses this key to remount the preview after an
                    // edit. The server half of a full-stack bundle intentionally
                    // discards DOM keys, so keep the binding warning-free there.
                    let _img_edit_key = format!(
                        "{:.4}-{:.4}-{:.4}-{:.4}-{:.2}-{:.4}-{:.4}-{:.4}-{:.4}",
                        img_edit.crop.x,
                        img_edit.crop.y,
                        img_edit.crop.width,
                        img_edit.crop.height,
                        img_edit.transform.rotation,
                        img_edit.transform.stretch_x,
                        img_edit.transform.stretch_y,
                        img_edit.color.saturation,
                        img_edit.color.contrast_r * img_edit.color.contrast_g * img_edit.color.contrast_b
                    );
                    let img_error_display = img_error
                        .as_deref()
                        .unwrap_or("Failed")
                        .to_string();
                    let previous_id = adjacent_gallery_id(&pending(), img_id, -1);
                    let next_id = adjacent_gallery_id(&pending(), img_id, 1);
                    let retry_disabled = uploading()
                        || pending()
                            .iter()
                            .any(|p| matches!(p.status, UploadStatus::Processing));
                    let reorder_disabled = uploading() || img_status == UploadStatus::Uploading;
                    let item_class = if dragged_image_id() == Some(img_id) {
                        "image-upload__item image-upload__item--dragging"
                    } else {
                        "image-upload__item"
                    };

                    rsx! {
                        div {
                            key: "{img_id}",
                            class: "{item_class}",
                            draggable: !reorder_disabled,
                            ondragstart: move |_| dragged_image_id.set(Some(img_id)),
                            ondragend: move |_| dragged_image_id.set(None),
                            ondragover: move |event| {
                                if dragged_image_id().is_some() {
                                    event.prevent_default();
                                }
                            },
                            ondrop: move |event| {
                                event.prevent_default();
                                if let Some(dragged_id) = dragged_image_id() {
                                    reorder_pending_images(dragged_id, img_id, pending, images);
                                }
                                dragged_image_id.set(None);
                            },
                            if img_persisted {
                                if let Some(image) = img_uploaded.clone() {
                                    div { class: "image-upload__preview-button",
                                        EditedRecipeImage {
                                            image,
                                            class: "recipe-image-preview",
                                            alt: img_name.clone(),
                                            prefer_thumbnail: true,
                                            on_error: move |_| {},
                                        }
                                    }
                                }
                            // Preview thumbnail (object URL) — only on client.
                            } else if let Some(url) = img_preview.clone() {
                                div {
                                    class: "image-upload__preview-button",
                                    style: "cursor: pointer;",
                                    onclick: move |_| {
                                        editor_modal_img_id.set(Some(img_id));
                                        editor_modal_url.set(Some(url.clone()));
                                    },
                                    EditedRecipeImage {
                                        key: "{img_id}-{_img_edit_key}",
                                        image: RecipeImage::new(url.clone(), None, img_edit),
                                        class: "recipe-image-preview",
                                        alt: img_name.clone(),
                                        prefer_thumbnail: false,
                                        on_error: move |_| {},
                                    }
                                }
                            } else {
                                div {
                                    class: "image-upload__thumb image-upload__thumb--placeholder",
                                    "🖼"
                                }
                            }
                            // Meta: name + size readout
                            div { class: "image-upload__meta",
                                span { "{img_name}" }
                                if img_full_len > 0 {
                                    span {
                                        class: "image-upload__size",
                                        "{format_file_size(img_original)} → {format_file_size(img_full_len as u64)} ({reduction_percent(img_original, img_full_len)}% smaller)"
                                    }
                                }
                            }
                            // Status / progress
                            div { class: "image-upload__status",
                                match &img_status {
                                    UploadStatus::Processing => rsx! {
                                        span { "Processing…" }
                                        div { class: "image-upload__progress" }
                                    },
                                    UploadStatus::Ready => rsx! {
                                        span { "Ready" }
                                    },
                                    UploadStatus::Uploading => rsx! {
                                        span { "Uploading…" }
                                        div { class: "image-upload__progress" }
                                    },
                                    UploadStatus::Uploaded => rsx! {
                                        span {
                                            if img_persisted { "✓ Saved" } else { "✓ Uploaded" }
                                        }
                                    },
                                    UploadStatus::Failed => rsx! {
                                        span { class: "image-upload__error",
                                            "{img_error_display}"
                                        }
                                    },
                                }
                            }
                            div {
                                class: "image-upload__order-controls",
                                aria_label: "Reorder image",
                                button {
                                    r#type: "button",
                                    class: "btn btn-ghost touch-target",
                                    disabled: reorder_disabled || previous_id.is_none(),
                                    aria_label: "Move image up",
                                    onclick: move |_| {
                                        if let Some(previous_id) = previous_id {
                                            reorder_pending_images(img_id, previous_id, pending, images);
                                        }
                                    },
                                    "↑"
                                }
                                button {
                                    r#type: "button",
                                    class: "btn btn-ghost touch-target",
                                    disabled: reorder_disabled || next_id.is_none(),
                                    aria_label: "Move image down",
                                    onclick: move |_| {
                                        if let Some(next_id) = next_id {
                                            reorder_pending_images(img_id, next_id, pending, images);
                                        }
                                    },
                                    "↓"
                                }
                            }
                            // Per-item Retry (AC5) — only on Failed items
                            if img_status == UploadStatus::Failed {
                                button {
                                    r#type: "button",
                                    class: "btn btn-ghost touch-target",
                                    disabled: retry_disabled,
                                    onclick: move |_| {
                                        spawn(async move {
                                            retry_upload(
                                                img_id,
                                                pending,
                                                images,
                                                purpose,
                                                discard_capabilities,
                                            )
                                            .await;
                                        });
                                    },
                                    "↻ Retry"
                                }
                            }
                            // Remove (disabled while Uploading)
                            button {
                                r#type: "button",
                                class: "btn btn-ghost touch-target",
                                disabled: img_status == UploadStatus::Uploading,
                                onclick: move |_| {
                                    if img_persisted {
                                        if let Some(image) = &img_uploaded {
                                            on_remove_existing.call(image.url.clone());
                                        }
                                    }
                                    let _removed_uploaded_url = remove_pending(img_id, pending, images);
                                    let discard_capability =
                                        discard_capabilities.write().remove(&img_id);
                                    if !img_persisted {
                                        if let Some(discard_capability) = discard_capability {
                                            spawn(async move {
                                                if let Err(problem) = crate::api::upload::discard_upload(discard_capability).await {
                                                    tracing::warn!("Failed to discard unsaved upload: {problem}");
                                                }
                                            });
                                        }
                                    }
                                },
                                "✕"
                            }
                        }
                    }
                }
            }

            // Image editor modal
            ImageEditorModal {
                image_url: editor_modal_url,
                on_save: move |edit: RecipeImageEdit| {
                    if let Some(img_id) = editor_modal_img_id() {
                        if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
                            p.edit = edit.clone();
                            if let Some(uploaded_image) = &mut p.uploaded_image {
                                uploaded_image.edits = vec![edit];
                            }
                        }
                        sync_uploaded_images(pending, images);
                    }
                },
                on_close: move |_| {
                    editor_modal_url.set(None);
                    editor_modal_img_id.set(None);
                },
            }
        }
    }
}

// ── Unit tests (pure helpers — cross-platform) ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_limit_reports_only_remaining_slots() {
        assert_eq!(available_image_slots(0, 3, Some(1)), 1);
        assert_eq!(available_image_slots(1, 3, Some(1)), 0);
        assert_eq!(available_image_slots(2, 3, None), 3);
    }

    #[test]
    fn deferred_upload_completion_executes_success_failure_abort_and_retry() {
        let success = UploadOutcome {
            succeeded: 1,
            failed: 0,
            aborted: None,
        };
        let mut pending = Some("first save");
        assert_eq!(
            complete_deferred_upload(&mut pending, success.clone()),
            DeferredUploadCompletion::Ready("first save")
        );
        assert_eq!(
            complete_deferred_upload(&mut pending, success.clone()),
            DeferredUploadCompletion::NoPendingSave
        );

        let mut pending = Some("failed save");
        assert_eq!(
            complete_deferred_upload(
                &mut pending,
                UploadOutcome {
                    succeeded: 0,
                    failed: 1,
                    aborted: None,
                },
            ),
            DeferredUploadCompletion::Failed
        );
        assert!(pending.is_none());

        pending = Some("retry");
        assert_eq!(
            complete_deferred_upload(&mut pending, success),
            DeferredUploadCompletion::Ready("retry")
        );

        pending = Some("aborted save");
        assert_eq!(
            complete_deferred_upload(
                &mut pending,
                UploadOutcome {
                    succeeded: 0,
                    failed: 0,
                    aborted: Some("still processing".to_string()),
                },
            ),
            DeferredUploadCompletion::Aborted("still processing".to_string())
        );
        assert!(pending.is_none());
    }

    #[test]
    fn single_gallery_image_reflects_add_replace_and_removal() {
        let first = RecipeImage::new(
            "first".to_string(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        );
        let replacement = RecipeImage::new(
            "replacement".to_string(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        );
        let mut gallery = vec![first.clone()];
        assert_eq!(single_gallery_image(&gallery), Some(first));

        gallery[0] = replacement.clone();
        assert_eq!(single_gallery_image(&gallery), Some(replacement));

        gallery.clear();
        assert_eq!(single_gallery_image(&gallery), None);
    }

    #[test]
    fn file_picker_remains_keyboard_accessible_and_errors_are_announced() {
        let source = include_str!("image_upload.rs");
        assert!(source.contains("image-upload__file-input"));
        assert!(!source.contains("style: \"display: none;\""));
        assert!(source.contains("role: \"alert\""));
        assert!(source.contains("aria_live: \"assertive\""));
    }

    fn pending_image_with_uploaded_url(id: usize, url: Option<&str>) -> PendingImage {
        PendingImage {
            id,
            persisted: false,
            name: format!("image-{id}.webp"),
            preview_url: None,
            original_size: 100,
            full_bytes: Arc::new(Vec::new()),
            thumb_bytes: Arc::new(Vec::new()),
            edit: RecipeImageEdit::default_rect_16x9(),
            status: if url.is_some() {
                UploadStatus::Uploaded
            } else {
                UploadStatus::Failed
            },
            error: None,
            uploaded_image: url.map(|url| {
                RecipeImage::new(url.to_string(), None, RecipeImageEdit::default_rect_16x9())
            }),
        }
    }

    #[test]
    fn uploaded_images_follow_pending_display_order_and_skip_failures() {
        let pending = vec![
            pending_image_with_uploaded_url(3, Some("third")),
            pending_image_with_uploaded_url(2, None),
            pending_image_with_uploaded_url(1, Some("first")),
        ];

        let ordered_urls: Vec<String> = uploaded_images_in_pending_order(&pending)
            .into_iter()
            .map(|image| image.url)
            .collect();

        assert_eq!(ordered_urls, vec!["third".to_string(), "first".to_string()]);
        assert_eq!(pending_upload_count(&pending), 1);
    }

    #[test]
    fn failed_pending_items_keep_later_save_batches_failed_until_retry_or_removal() {
        let failed = pending_image_with_uploaded_url(1, None);
        let (ready, failed_count) = upload_batch_plan(std::slice::from_ref(&failed));
        assert!(ready.is_empty());
        assert_eq!(failed_count, 1);

        let completion = complete_deferred_upload(
            &mut Some("replacement save"),
            UploadOutcome {
                succeeded: 0,
                failed: failed_count,
                aborted: None,
            },
        );
        assert_eq!(completion, DeferredUploadCompletion::Failed);

        let mut retried = failed;
        retried.status = UploadStatus::Ready;
        let (ready, failed_count) = upload_batch_plan(&[retried]);
        assert_eq!(ready.len(), 1);
        assert_eq!(failed_count, 0);

        let (ready, failed_count) = upload_batch_plan(&[]);
        assert!(ready.is_empty());
        assert_eq!(failed_count, 0);
    }

    #[test]
    fn syncing_new_uploads_preserves_existing_images_without_duplicates() {
        let existing = vec![
            RecipeImage::new(
                "existing".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
            RecipeImage::new(
                "already-uploaded".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
        ];
        assert_eq!(
            merge_existing_and_uploaded_images(existing.clone(), Vec::new()),
            existing.clone()
        );
        let uploaded = vec![
            RecipeImage::new(
                "already-uploaded".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
            RecipeImage::new(
                "new".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
        ];

        let merged = merge_existing_and_uploaded_images(existing, uploaded);
        let urls = merged
            .into_iter()
            .map(|image| image.url)
            .collect::<Vec<_>>();

        assert_eq!(urls, vec!["existing", "already-uploaded", "new"]);
    }

    #[test]
    fn persisted_gallery_excludes_images_already_rendered_as_pending_uploads() {
        let images = vec![
            RecipeImage::new(
                "existing".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
            RecipeImage::new(
                "uploaded-pending".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
        ];
        let pending = vec![pending_image_with_uploaded_url(1, Some("uploaded-pending"))];

        let visible = persisted_images_not_in_pending(&images, &pending);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].url, "existing");
    }

    #[test]
    fn persisted_images_reorder_by_stable_url() {
        let mut images = vec![
            RecipeImage::new(
                "first".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
            RecipeImage::new(
                "second".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
            RecipeImage::new(
                "third".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
        ];

        assert!(reorder_images_by_url(&mut images, "third", "first"));
        assert_eq!(
            images
                .iter()
                .map(|image| image.url.as_str())
                .collect::<Vec<_>>(),
            vec!["third", "first", "second"]
        );
        assert!(!reorder_images_by_url(&mut images, "missing", "first"));
    }

    #[test]
    fn unified_gallery_reorders_persisted_and_local_images_together() {
        let persisted = vec![
            RecipeImage::new(
                "first".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
            RecipeImage::new(
                "second".to_string(),
                None,
                RecipeImageEdit::default_rect_16x9(),
            ),
        ];
        let mut gallery = gallery_images_from_persisted(&persisted);
        gallery.push(pending_image_with_uploaded_url(2, Some("new")));

        assert!(reorder_gallery_images(&mut gallery, 0, 2));
        assert_eq!(
            uploaded_images_in_pending_order(&gallery)
                .iter()
                .map(|image| image.url.as_str())
                .collect::<Vec<_>>(),
            vec!["second", "new", "first"]
        );
    }

    #[test]
    fn adjacent_gallery_ids_are_safe_at_boundaries_and_after_reorder() {
        let mut gallery = vec![
            pending_image_with_uploaded_url(10, Some("first")),
            pending_image_with_uploaded_url(20, Some("second")),
            pending_image_with_uploaded_url(30, Some("third")),
        ];

        assert_eq!(adjacent_gallery_id(&gallery, 10, -1), None);
        assert_eq!(adjacent_gallery_id(&gallery, 10, 1), Some(20));
        assert_eq!(adjacent_gallery_id(&gallery, 30, 1), None);
        assert_eq!(adjacent_gallery_id(&gallery, 999, 1), None);

        assert!(reorder_gallery_images(&mut gallery, 30, 10));
        assert_eq!(adjacent_gallery_id(&gallery, 10, -1), Some(30));
        assert_eq!(adjacent_gallery_id(&gallery, 10, 1), Some(20));
    }

    // ── is_allowed_image_type ────────────────────────────────────────────

    #[test]
    fn test_is_allowed_image_type_jpg() {
        assert!(is_allowed_image_type("photo.jpg"));
    }

    #[test]
    fn test_is_allowed_image_type_jpeg() {
        assert!(is_allowed_image_type("photo.jpeg"));
    }

    #[test]
    fn test_is_allowed_image_type_png() {
        assert!(is_allowed_image_type("photo.png"));
    }

    #[test]
    fn test_is_allowed_image_type_webp() {
        assert!(is_allowed_image_type("photo.webp"));
    }

    #[test]
    fn test_is_allowed_image_type_rejects_pdf() {
        assert!(!is_allowed_image_type("doc.pdf"));
    }

    #[test]
    fn test_is_allowed_image_type_rejects_gif() {
        assert!(!is_allowed_image_type("anim.gif"));
    }

    #[test]
    fn test_is_allowed_image_type_rejects_exe() {
        assert!(!is_allowed_image_type("program.exe"));
    }

    #[test]
    fn test_is_allowed_image_type_case_insensitive_jpg() {
        assert!(is_allowed_image_type("PHOTO.JPG"));
    }

    #[test]
    fn test_is_allowed_image_type_case_insensitive_webp() {
        assert!(is_allowed_image_type("Photo.WebP"));
    }

    #[test]
    fn test_is_allowed_image_type_no_extension() {
        assert!(!is_allowed_image_type("noextension"));
    }

    // ── format_file_size ─────────────────────────────────────────────────

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(format_file_size(500), "500 B");
    }

    #[test]
    fn test_format_file_size_kb() {
        assert_eq!(format_file_size(1024 * 800), "800 KB");
    }

    #[test]
    fn test_format_file_size_mb() {
        assert_eq!(format_file_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn test_format_file_size_zero() {
        assert_eq!(format_file_size(0), "0 B");
    }

    // ── reduction_percent ────────────────────────────────────────────────

    #[test]
    fn test_reduction_percent_typical() {
        assert_eq!(reduction_percent(5_000_000, 1_000_000), 80);
    }

    #[test]
    fn test_reduction_percent_zero_original() {
        assert_eq!(reduction_percent(0, 100), 0);
    }

    #[test]
    fn test_reduction_percent_no_reduction() {
        // processed >= original → 0 (no reduction / grew)
        assert_eq!(reduction_percent(100, 100), 0);
        assert_eq!(reduction_percent(100, 200), 0);
    }
}
