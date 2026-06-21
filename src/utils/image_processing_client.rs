//! Client-side image processing using Canvas API.
//!
//! Only compiled for WASM target. Uses `web_sys` to access browser's
//! Canvas API for resizing, format conversion, and thumbnail generation.
#![cfg(target_arch = "wasm32")]
#![allow(dead_code)]

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Blob, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

/// Maximum dimensions for full-size images.
pub const MAX_DIMENSION: u32 = 1920;

/// Thumbnail dimensions (square).
pub const THUMB_SIZE: u32 = 400;

/// WebP encoding quality (0.0-1.0 for canvas.toBlob).
pub const WEBP_QUALITY: f64 = 0.85;

/// Processed image result: full-size and thumbnail as byte arrays.
pub struct ProcessedImage {
    /// Full-size WebP bytes (resized to max 1920x1920)
    pub full_bytes: Vec<u8>,
    /// Thumbnail WebP bytes (fit within 400x400, aspect ratio preserved)
    pub thumb_bytes: Vec<u8>,
}

/// Process a File object: resize, convert to WebP, generate thumbnail.
///
/// Pipeline:
/// 1. Load file into HtmlImageElement via FileReader + ObjectURL
/// 2. Calculate target dimensions (maintain aspect ratio, fit within MAX_DIMENSION)
/// 3. Draw to offscreen canvas at full-size dimensions
/// 4. Export canvas as WebP blob via canvas.toBlob()
/// 5. Read blob as ArrayBuffer → Vec<u8>
/// 6. Repeat steps 3-5 for thumbnail (fit within THUMB_SIZE x THUMB_SIZE, aspect ratio preserved)
/// 7. Revoke ObjectURL to free memory
///
/// Equivalent to `process_image_with_options(file, false)` — compress path
/// (resize to ≤1920px, WebP @ 0.85). Kept for backward compatibility.
pub async fn process_image(file: web_sys::File) -> Result<ProcessedImage, String> {
    process_image_with_options(file, false).await
}

/// Process a File with a quality/resize option (CP4 High-Quality toggle).
///
/// - `high_quality = true`  → keep original dimensions, WebP @ 0.9.
/// - `high_quality = false` → resize to ≤`MAX_DIMENSION` px, WebP @ 0.85
///   (the existing `process_image` behavior).
///
/// The thumbnail is always generated at `THUMB_SIZE` and the standard
/// `WEBP_QUALITY` regardless of the toggle — thumbnails are never the
/// size-limit bottleneck.
pub async fn process_image_with_options(
    file: web_sys::File,
    high_quality: bool,
) -> Result<ProcessedImage, String> {
    // Step 1: Load image into HtmlImageElement
    let img = load_image_from_file(&file).await?;

    // Step 2: Calculate full-size dimensions
    let (full_w, full_h) = if high_quality {
        // HQ: keep the original pixel dimensions
        (img.natural_width(), img.natural_height())
    } else {
        // Compress: fit within MAX_DIMENSION, preserving aspect ratio
        calculate_dimensions(img.natural_width(), img.natural_height(), MAX_DIMENSION)
    };
    let full_quality = if high_quality { 0.9 } else { WEBP_QUALITY };

    // Step 3-5: Process full-size
    let full_bytes = render_to_webp_with_quality(&img, full_w, full_h, full_quality).await?;

    // Step 6: Process thumbnail (fit within 400x400, preserving aspect ratio)
    let (thumb_w, thumb_h) =
        calculate_dimensions(img.natural_width(), img.natural_height(), THUMB_SIZE);
    let thumb_bytes = render_to_webp_with_quality(&img, thumb_w, thumb_h, WEBP_QUALITY).await?;

    Ok(ProcessedImage {
        full_bytes,
        thumb_bytes,
    })
}

/// Load a File into an HtmlImageElement. Returns the loaded image element.
pub async fn load_image_from_file(file: &web_sys::File) -> Result<HtmlImageElement, String> {
    let url = web_sys::Url::create_object_url_with_blob(file)
        .map_err(|e| format!("Failed to create object URL: {:?}", e))?;

    let img = web_sys::HtmlImageElement::new()
        .map_err(|e| format!("Failed to create image element: {:?}", e))?;

    // Wait for image to load using polling pattern
    let loaded = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let loaded_clone = loaded.clone();

    let on_load = Closure::wrap(Box::new(move || {
        loaded_clone.store(true, std::sync::atomic::Ordering::Release);
    }) as Box<dyn FnMut()>);

    img.set_onload(Some(on_load.as_ref().unchecked_ref()));
    on_load.forget(); // Prevent closure from being dropped

    img.set_src(&url);

    // Poll for load completion
    loop {
        gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
        if loaded.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
    }

    // Check for errors
    let complete = img.complete();
    let natural_width = img.natural_width();
    if !complete || natural_width == 0 {
        return Err("Failed to load image or image has zero dimensions".to_string());
    }

    // Revoke ObjectURL to free memory
    let _ = web_sys::Url::revoke_object_url(&url);

    Ok(img)
}

/// Calculate target dimensions maintaining aspect ratio.
///
/// If the source fits within max_dim x max_dim, returns original dimensions.
/// Otherwise, scales down to fit within the constraint.
fn calculate_dimensions(src_w: u32, src_h: u32, max_dim: u32) -> (u32, u32) {
    if src_w <= max_dim && src_h <= max_dim {
        return (src_w, src_h);
    }

    let ratio = src_w as f64 / src_h as f64;
    if src_w > src_h {
        (max_dim, (max_dim as f64 / ratio) as u32)
    } else {
        ((max_dim as f64 * ratio) as u32, max_dim)
    }
}

/// Draw an image to an offscreen canvas and export as WebP blob at the
/// default `WEBP_QUALITY`. Thin wrapper around `render_to_webp_with_quality`.
async fn render_to_webp(
    img: &HtmlImageElement,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    render_to_webp_with_quality(img, width, height, WEBP_QUALITY).await
}

/// Draw an image to an offscreen canvas and export as a WebP blob at the
/// given encoder `quality` (0.0–1.0 for `canvas.toBlob`'s third arg).
async fn render_to_webp_with_quality(
    img: &HtmlImageElement,
    width: u32,
    height: u32,
    quality: f64,
) -> Result<Vec<u8>, String> {
    let document = web_sys::window()
        .ok_or("No window available")?
        .document()
        .ok_or("No document available")?;

    // Create offscreen canvas
    let canvas = document
        .create_element("canvas")
        .map_err(|e| format!("Failed to create canvas: {:?}", e))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| "Failed to cast to HtmlCanvasElement")?;

    canvas.set_width(width);
    canvas.set_height(height);

    // Get 2D context
    let context = canvas
        .get_context("2d")
        .map_err(|e| format!("Failed to get 2D context: {:?}", e))?
        .and_then(|v| {
            v.dyn_into::<CanvasRenderingContext2d>()
                .map_err(|_| "Failed to cast context to CanvasRenderingContext2d")
                .ok()
        })
        .ok_or("Failed to cast context to CanvasRenderingContext2d")?;

    // Draw image (resized)
    context
        .draw_image_with_html_image_element_and_dw_and_dh(
            img,
            0.0,
            0.0,
            width as f64,
            height as f64,
        )
        .map_err(|e| format!("Failed to draw image: {:?}", e))?;

    // Export as WebP blob using canvas.toBlob()
    blob_from_canvas(&canvas, quality).await
}

/// Convert canvas content to a WebP blob at the given `quality`, then read as `Vec<u8>`.
///
/// Uses polling approach: canvas.toBlob() callback sets AtomicBool,
/// we poll with gloo_timers::future::sleep() until ready.
pub async fn blob_from_canvas(canvas: &HtmlCanvasElement, quality: f64) -> Result<Vec<u8>, String> {
    let blob = std::sync::Arc::new(std::sync::Mutex::new(None));
    let blob_clone = blob.clone();
    let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready_clone = ready.clone();

    let callback = Closure::wrap(Box::new(move |b: Option<Blob>| {
        *blob_clone.lock().unwrap() = b;
        ready_clone.store(true, std::sync::atomic::Ordering::Release);
    }) as Box<dyn FnMut(Option<Blob>)>);

    // canvas.toBlob(callback, mimeType, encoderOptions)
    canvas
        .to_blob_with_type_and_encoder_options(
            callback.as_ref().unchecked_ref(),
            "image/webp",
            &wasm_bindgen::JsValue::from_f64(quality),
        )
        .map_err(|e| format!("Failed to call toBlob: {:?}", e))?;
    callback.forget(); // Prevent closure from being dropped

    // Poll for blob
    loop {
        gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
        if ready.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
    }

    let blob = blob
        .lock()
        .unwrap()
        .clone()
        .ok_or("toBlob returned null (WebP not supported?)")?;

    // Read blob as ArrayBuffer
    array_buffer_from_blob(&blob).await
}

/// Read a Blob as ArrayBuffer, then convert to Vec<u8>.
///
/// Uses polling approach: FileReader.onload sets AtomicBool,
/// we poll with gloo_timers::future::sleep() until ready.
async fn array_buffer_from_blob(blob: &Blob) -> Result<Vec<u8>, String> {
    let file_reader =
        web_sys::FileReader::new().map_err(|e| format!("Failed to create FileReader: {:?}", e))?;

    let loaded = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let loaded_clone = loaded.clone();

    let on_load = Closure::wrap(Box::new(move || {
        loaded_clone.store(true, std::sync::atomic::Ordering::Release);
    }) as Box<dyn FnMut()>);

    file_reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
    on_load.forget();

    file_reader
        .read_as_array_buffer(blob)
        .map_err(|e| format!("Failed to read blob: {:?}", e))?;

    // Poll for completion
    loop {
        gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
        if loaded.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
    }

    // Get result
    let result = file_reader
        .result()
        .map_err(|e| format!("Failed to get FileReader result: {:?}", e))?;
    let array_buf = result
        .dyn_into::<js_sys::ArrayBuffer>()
        .map_err(|_| "FileReader result is not an ArrayBuffer")?;

    // Convert ArrayBuffer to Vec<u8>
    let uint8 = js_sys::Uint8Array::new(&array_buf);
    let mut bytes = vec![0u8; uint8.length() as usize];
    uint8.copy_to(&mut bytes);

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions_small() {
        let (w, h) = calculate_dimensions(100, 100, MAX_DIMENSION);
        assert_eq!((w, h), (100, 100), "Small image should not be resized");
    }

    #[test]
    fn test_calculate_dimensions_large() {
        let (w, h) = calculate_dimensions(2000, 2000, MAX_DIMENSION);
        assert_eq!(
            (w, h),
            (1920, 1920),
            "Large square should be resized to max"
        );
    }

    #[test]
    fn test_calculate_dimensions_wide() {
        let (w, h) = calculate_dimensions(3000, 1000, MAX_DIMENSION);
        assert_eq!(
            (w, h),
            (1920, 640),
            "Wide image should preserve aspect ratio"
        );
    }

    #[test]
    fn test_calculate_dimensions_tall() {
        let (w, h) = calculate_dimensions(1000, 3000, MAX_DIMENSION);
        assert_eq!(
            (w, h),
            (640, 1920),
            "Tall image should preserve aspect ratio"
        );
    }

    #[test]
    fn test_calculate_dimensions_exact_boundary() {
        let (w, h) = calculate_dimensions(1920, 1080, MAX_DIMENSION);
        assert_eq!(
            (w, h),
            (1920, 1080),
            "Image at boundary should not be resized"
        );
    }

    #[test]
    fn test_calculate_dimensions_thumb_size() {
        // Small image upscaled for thumbnail
        let (w, h) = calculate_dimensions(100, 100, THUMB_SIZE);
        assert_eq!(
            (w, h),
            (100, 100),
            "Small image should not be resized even for thumb"
        );
    }

    #[test]
    fn test_calculate_dimensions_thumb_wide() {
        // Large landscape photo for thumbnail: 3000x2000 should fit within 400x400
        let (w, h) = calculate_dimensions(3000, 2000, THUMB_SIZE);
        assert_eq!(
            (w, h),
            (400, 267),
            "Wide image should preserve aspect ratio in thumbnail"
        );
    }

    #[test]
    fn test_calculate_dimensions_thumb_tall() {
        // Large portrait photo for thumbnail: 2000x3000 should fit within 400x400
        let (w, h) = calculate_dimensions(2000, 3000, THUMB_SIZE);
        assert_eq!(
            (w, h),
            (267, 400),
            "Tall image should preserve aspect ratio in thumbnail"
        );
    }
}
