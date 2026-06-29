//! Image upload component: drag-and-drop / file picker, client-side resize +
//! WebP processing (CP3), deferred server upload (CP2 `#[server] upload_image`),
//! progress + error UI with per-item retry, and an optional High-Quality toggle
//! with size-reduction readout and a client-side 10 MB overflow guard.
//!
//! Compiles on both `wasm32-unknown-unknown` (the real client) and the server
//! target (where the component is never rendered but is still type-checked by
//! `dx serve` SSR and `cargo check --features server`). All
//! `web_sys`/`process_image_with_options`/`WebFileExt`/object-URL logic that
//! is wasm-only lives behind `#[cfg(target_arch = "wasm32")]` blocks with a
//! non-wasm no-op branch.

use dioxus::html::HasFileData;
use dioxus::prelude::*;
use std::sync::Arc;

use crate::components::base::{Button, ButtonVariant, ImageEditorModal};

// WASM-only bridge: connects Dioxus `FileData` to CP3's `process_image_*`
// pipeline. Gated at module scope so the imports don't trigger
// `unused_imports` on the server build (`warnings = "deny"`).
#[cfg(target_arch = "wasm32")]
use crate::utils::image_processing_client::process_image_with_options;
#[cfg(target_arch = "wasm32")]
use dioxus_web::WebFileExt;

// ── Constants ───────────────────────────────────────────────────────────────

/// Maximum original file size accepted by the dropzone (10 MB).
/// Matches the server's per-field limit (`crate::api::upload::MAX_FILE_SIZE`).
/// Only referenced inside wasm-gated code — `allow(dead_code)` covers the
/// server build where the wasm blocks are stripped.
#[allow(dead_code)]
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Originals larger than this show the "High Quality" toggle (per the plan).
const HQ_TOGGLE_THRESHOLD: u64 = 1_000_000;

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

/// Props for [`ImageUpload`].
#[derive(Props, Clone, PartialEq)]
pub struct ImageUploadProps {
    /// Shared sink: the component appends each successfully uploaded
    /// `full_url` here as uploads complete. The parent reads `images()`
    /// when calling `create_recipe`.
    pub images: Signal<Vec<String>>,
    /// Parent sets this to `true` to trigger the upload batch. The component
    /// resets it to `false` when the batch finishes — BEFORE clearing its
    /// internal `uploading` flag (this ordering prevents the `use_effect`
    /// double-trigger race; see the upload-trigger effect below).
    pub upload_now: Signal<bool>,
    /// Mirrors "any pending file is `Processing` or `Uploading`". The parent
    /// reads this to disable Save / guard submit. The component keeps it in
    /// sync via a `use_effect` over a memo of `pending` + `uploading`.
    pub busy: Signal<bool>,
    /// Fired exactly once when the upload batch finishes. The parent uses
    /// this to proceed to `create_recipe` (on success) or surface an error
    /// (on abort/failure) — NO polling loop.
    pub on_upload_complete: EventHandler<UploadOutcome>,
}

// ── Internal state types ────────────────────────────────────────────────────

/// Per-file upload lifecycle.
#[derive(Clone, PartialEq)]
enum UploadStatus {
    /// Canvas resize/WebP in progress.
    Processing,
    /// Processed; awaiting upload at save time.
    Ready,
    /// Server fn call in flight.
    Uploading,
    /// Done — `full_url` already pushed into `images`.
    Uploaded,
    /// Upload failed OR HQ overflow — user can Retry / toggle HQ off.
    Failed,
}

/// Snapshot of a ready-to-upload image: (id, full bytes, thumb bytes, name).
/// Extracted as a type alias to satisfy `clippy::type_complexity`.
type UploadItem = (usize, Arc<Vec<u8>>, Arc<Vec<u8>>, String);

/// One selected image, its processed bytes, and its upload status.
///
/// `full_bytes` and `thumb_bytes` are `Arc<Vec<u8>>` so signal-read clones
/// (per render) are cheap ref-count bumps instead of multi-MB deep copies.
/// The bytes are written once after processing and only read at upload time.
#[derive(Clone, PartialEq)]
struct PendingImage {
    id: usize,
    name: String,
    /// Object URL for the preview thumbnail (wasm only); `None` on SSR.
    preview_url: Option<String>,
    /// Original File size in bytes.
    original_size: u64,
    /// Processed full-size WebP bytes (always WebP regardless of HQ toggle).
    full_bytes: Arc<Vec<u8>>,
    /// Processed 400px thumbnail WebP bytes.
    thumb_bytes: Arc<Vec<u8>>,
    /// Original Dioxus `FileData` — stashed for HQ-toggle re-processing
    /// (avoids re-picking the file). `None` on SSR; only `Some` after a
    /// wasm pick. `FileData: Clone + PartialEq + Send + Sync`, so it
    /// participates cleanly in the derives.
    file_data: Option<dioxus::html::FileData>,
    /// Per-file High-Quality toggle state.
    high_quality: bool,
    /// Where this file is in the upload lifecycle.
    status: UploadStatus,
    /// Failure message when `status == Failed`.
    error: Option<String>,
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
            name: name.clone(),
            preview_url: preview_url.clone(),
            original_size,
            full_bytes: Arc::new(Vec::new()),
            thumb_bytes: Arc::new(Vec::new()),
            file_data: Some(file.clone()),
            high_quality: false,
            status: UploadStatus::Processing,
            error: None,
        });

        // Open the preview modal for the newly selected image.
        if let Some(url) = &preview_url {
            editor_modal_url.set(Some(url.clone()));
        }

        // Process (default: compress — HQ off).
        match process_image_with_options(web_file, false).await {
            Ok(processed) => {
                let full_len = processed.full_bytes.len();
                let mut guard = pending.write();
                if let Some(p) = guard.iter_mut().find(|p| p.id == id) {
                    p.full_bytes = Arc::new(processed.full_bytes);
                    p.thumb_bytes = Arc::new(processed.thumb_bytes);
                    // HQ overflow guard — at default HQ=false this is
                    // unlikely, but a max-size original at 0.85 could
                    // still exceed 10 MB. Belt-and-suspenders.
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
        let _ = (file, pending, next_id, error, editor_modal_url);
    }
}

/// Re-process a stashed file with the new HQ setting (wasm-only logic; the
/// non-wasm branch is a no-op). Updates the pending item's bytes, status,
/// and size readout. Applies the HQ overflow guard (WARNING #6): if the
/// HQ-processed bytes exceed the server's 10 MB limit, marks `Failed`
/// before any network round-trip.
async fn reprocess_for_hq(img_id: usize, new_hq: bool, mut pending: Signal<Vec<PendingImage>>) {
    let file_data = pending()
        .iter()
        .find(|p| p.id == img_id)
        .and_then(|p| p.file_data.clone());
    let Some(file) = file_data else {
        return;
    };

    // Mark as Processing and record the new toggle state.
    if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
        p.status = UploadStatus::Processing;
        p.high_quality = new_hq;
        p.error = None;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let Some(web_file) = file.get_web_file() else {
            if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
                p.status = UploadStatus::Failed;
                p.error = Some("Could not re-read the original file.".to_string());
            }
            return;
        };
        match process_image_with_options(web_file, new_hq).await {
            Ok(processed) => {
                let full_len = processed.full_bytes.len();
                let mut guard = pending.write();
                if let Some(p) = guard.iter_mut().find(|p| p.id == img_id) {
                    p.full_bytes = Arc::new(processed.full_bytes);
                    p.thumb_bytes = Arc::new(processed.thumb_bytes);
                    // HQ overflow guard (WARNING #6).
                    if full_len > MAX_FILE_SIZE as usize {
                        p.status = UploadStatus::Failed;
                        p.error = Some(format!(
                            "Image is {} — too large even in high quality. Turn off High Quality or use a smaller image.",
                            format_file_size(full_len as u64)
                        ));
                    } else {
                        p.status = UploadStatus::Ready;
                    }
                }
            }
            Err(msg) => {
                if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
                    p.status = UploadStatus::Failed;
                    p.error = Some(msg);
                }
            }
        }
    }

    // Non-wasm no-op: consume the params so nothing is unused.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (file, new_hq, img_id);
    }
}

/// Re-attempt the upload for a single `Failed` item (per-item Retry, AC5).
/// Reuses the stashed `full_bytes`/`thumb_bytes` from the last successful
/// processing — does NOT re-process.
async fn retry_upload(
    img_id: usize,
    mut pending: Signal<Vec<PendingImage>>,
    mut images: Signal<Vec<String>>,
) {
    let item = pending()
        .iter()
        .find(|p| p.id == img_id)
        .map(|p| (p.full_bytes.clone(), p.thumb_bytes.clone(), p.name.clone()));
    let Some((full, thumb, name)) = item else {
        return;
    };

    if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
        p.status = UploadStatus::Uploading;
        p.error = None;
    }

    // The one unavoidable `Vec<u8>` clone — to pass bytes by value to the
    // server fn. Happens once per retry, not per render.
    let result = crate::api::upload::upload_image((*full).clone(), (*thumb).clone(), name).await;
    match result {
        Ok(resp) => {
            images.write().push(resp.full_url);
            if let Some(p) = pending.write().iter_mut().find(|p| p.id == img_id) {
                p.status = UploadStatus::Uploaded;
            }
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
/// pushing each successful `full_url` into `images`. Returns the outcome
/// (succeeded/failed counts, or an abort reason if any item is still
/// `Processing`).
async fn upload_all(
    mut pending: Signal<Vec<PendingImage>>,
    mut images: Signal<Vec<String>>,
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
    let to_upload: Vec<UploadItem> = pending()
        .iter()
        .filter(|p| p.status == UploadStatus::Ready)
        .map(|p| {
            (
                p.id,
                p.full_bytes.clone(),
                p.thumb_bytes.clone(),
                p.name.clone(),
            )
        })
        .collect();

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (id, full, thumb, name) in to_upload {
        if let Some(p) = pending.write().iter_mut().find(|p| p.id == id) {
            p.status = UploadStatus::Uploading;
            p.error = None;
        }

        let result =
            crate::api::upload::upload_image((*full).clone(), (*thumb).clone(), name).await;

        match result {
            Ok(resp) => {
                images.write().push(resp.full_url);
                if let Some(p) = pending.write().iter_mut().find(|p| p.id == id) {
                    p.status = UploadStatus::Uploaded;
                }
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
/// No server delete: nothing has been uploaded yet, so no orphans.
fn remove_pending(img_id: usize, mut pending: Signal<Vec<PendingImage>>) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(p) = pending().iter().find(|p| p.id == img_id) {
            if let Some(url) = &p.preview_url {
                let _ = web_sys::Url::revoke_object_url(url);
            }
        }
    }
    pending.write().retain(|p| p.id != img_id);
}

// ── Component ───────────────────────────────────────────────────────────────

/// Drag-and-drop + file-picker image upload component with client-side
/// resize/WebP processing, progress, error handling, and per-item retry.
#[component]
pub fn ImageUpload(props: ImageUploadProps) -> Element {
    let mut drag_over = use_signal(|| false);
    let pending = use_signal(Vec::<PendingImage>::new);
    let mut uploading = use_signal(|| false);
    let error = use_signal(|| Option::<String>::None);
    let next_id = use_signal(|| 0usize);
    let mut editor_modal_url = use_signal(|| Option::<String>::None);

    let images = props.images;
    let mut upload_now = props.upload_now;
    let mut busy = props.busy;
    let on_upload_complete = props.on_upload_complete;

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

    // ── handle_files ─────────────────────────────────────────────────────
    // Shared by the `<input onchange>` and `<div ondrop>` handlers.
    let handle_files = move |files: Vec<dioxus::html::FileData>| async move {
        for file in files {
            process_one_file(file, pending, next_id, error, editor_modal_url).await;
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
                let outcome = upload_all(pending, images).await;
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
                    multiple: true,
                    style: "display: none;",
                    onchange: move |evt| async move {
                        handle_files(evt.files()).await;
                    },
                }
                span { "📷  Click or drag images here" }
                span { class: "image-upload__hint", "JPEG, PNG, or WebP · max 10 MB each" }
            }

            // Error banner
            if let Some(err) = error() {
                div { class: "image-upload__error image-upload__error--banner", "{err}" }
            }

            // Pending file list
            for img in pending().iter() {
                {
                    let img_id = img.id;
                    let img_name = img.name.clone();
                    let img_preview = img.preview_url.clone();
                    let img_original = img.original_size;
                    let img_full_len = img.full_bytes.len();
                    let img_hq = img.high_quality;
                    let img_status = img.status.clone();
                    let img_error = img.error.clone();
                    let img_error_display = img_error
                        .as_deref()
                        .unwrap_or("Failed")
                        .to_string();
                    let retry_disabled = uploading()
                        || pending()
                            .iter()
                            .any(|p| matches!(p.status, UploadStatus::Processing));

                    rsx! {
                        div {
                            key: "{img_id}",
                            class: "image-upload__item",
                            // Preview thumbnail (object URL) — only on client
                            if let Some(url) = img_preview.clone() {
                                img {
                                    class: "image-upload__thumb",
                                    src: "{url}",
                                    alt: "{img_name}",
                                    style: "cursor: pointer;",
                                    onclick: move |_| {
                                        editor_modal_url.set(Some(url.clone()));
                                    },
                                }
                            } else {
                                div {
                                    class: "image-upload__thumb image-upload__thumb--placeholder",
                                    "🖼"
                                }
                            }
                            // Meta: name + size readout + HQ toggle
                            div { class: "image-upload__meta",
                                span { "{img_name}" }
                                if img_full_len > 0 {
                                    span {
                                        class: "image-upload__size",
                                        "{format_file_size(img_original)} → {format_file_size(img_full_len as u64)} ({reduction_percent(img_original, img_full_len)}% smaller)"
                                    }
                                }
                                // High-Quality toggle — only when original > 1 MB
                                // and the item is in a re-processable state.
                                if img_original > HQ_TOGGLE_THRESHOLD
                                    && (img_status == UploadStatus::Ready
                                        || img_status == UploadStatus::Failed)
                                {
                                    label { class: "image-upload__toggle",
                                        input {
                                            r#type: "checkbox",
                                            checked: img_hq,
                                            onchange: move |evt| {
                                                let new_hq = evt.checked();
                                                spawn(async move {
                                                    reprocess_for_hq(img_id, new_hq, pending).await;
                                                });
                                            },
                                        }
                                        "High Quality"
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
                                        span { "✓ Uploaded" }
                                    },
                                    UploadStatus::Failed => rsx! {
                                        span { class: "image-upload__error",
                                            "{img_error_display}"
                                        }
                                    },
                                }
                            }
                            // Per-item Retry (AC5) — only on Failed items
                            if img_status == UploadStatus::Failed {
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    disabled: retry_disabled,
                                    onclick: move |_| {
                                        spawn(async move {
                                            retry_upload(img_id, pending, images).await;
                                        });
                                    },
                                    "↻ Retry"
                                }
                            }
                            // Remove (disabled while Uploading)
                            Button {
                                variant: ButtonVariant::Ghost,
                                disabled: img_status == UploadStatus::Uploading,
                                onclick: move |_| {
                                    remove_pending(img_id, pending);
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
                on_close: move |_| {
                    editor_modal_url.set(None);
                },
            }
        }
    }
}

// ── Unit tests (pure helpers — cross-platform) ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
