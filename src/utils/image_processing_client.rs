//! Client-side image processing using Canvas API.
//!
//! Only compiled for WASM target. Uses `web_sys` to access browser's
//! Canvas API for resizing, format conversion, and thumbnail generation.
#![cfg(target_arch = "wasm32")]
#![allow(dead_code)]

use super::image_dimensions::calculate_dimensions;
use super::image_processing_lifecycle::{
    wait_for_image_operation, ImageCompletionOwner, ImageDecodeUrl,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Blob, CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

const IMAGE_OPERATION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Maximum dimensions for full-size images.
pub const MAX_DIMENSION: u32 = 1920;

/// Thumbnail dimensions (square).
pub const THUMB_SIZE: u32 = 800;

/// WebP encoding quality (0.0-1.0 for canvas.toBlob).
pub const WEBP_QUALITY: f64 = 0.85;

/// Processed image result: full-size and thumbnail as byte arrays.
pub struct ProcessedImage {
    /// Full-size WebP bytes (resized to max 1920x1920)
    pub full_bytes: Vec<u8>,
    /// Thumbnail WebP bytes (fit within 800x800, aspect ratio preserved)
    pub thumb_bytes: Vec<u8>,
}

/// Process a File object: resize, convert to WebP, generate thumbnail.
///
/// Pipeline:
/// 1. Load file into HtmlImageElement via a temporary ObjectURL
/// 2. Calculate target dimensions (maintain aspect ratio, fit within MAX_DIMENSION)
/// 3. Draw to offscreen canvas at full-size dimensions
/// 4. Export canvas as WebP blob via canvas.toBlob()
/// 5. Read blob as ArrayBuffer → Vec<u8>
/// 6. Repeat steps 3-5 for thumbnail (fit within THUMB_SIZE x THUMB_SIZE, aspect ratio preserved)
///
/// The decoder revokes its ObjectURL on completion, error, or cancellation.
///
pub async fn process_image(file: web_sys::File) -> Result<ProcessedImage, String> {
    // Step 1: Load image into HtmlImageElement
    let img = load_image_from_file(&file).await?;

    // Step 2: Calculate full-size dimensions
    let (full_w, full_h) =
        calculate_dimensions(img.natural_width(), img.natural_height(), MAX_DIMENSION);

    // Step 3-5: Process full-size
    let full_bytes = render_to_webp_with_quality(&img, full_w, full_h, WEBP_QUALITY).await?;

    // Step 6: Process thumbnail (fit within 800x800, preserving aspect ratio)
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
    load_image_from_blob(file).await
}

/// Load any image Blob into an HtmlImageElement. This also supports existing
/// stored avatars fetched back from object storage for re-editing.
pub async fn load_image_from_blob(blob: &web_sys::Blob) -> Result<HtmlImageElement, String> {
    let url = web_sys::Url::create_object_url_with_blob(blob)
        .map_err(|e| format!("Failed to create object URL: {:?}", e))?;

    let url = ImageDecodeUrl::new(url, |url| {
        let _ = web_sys::Url::revoke_object_url(url);
    });
    // `complete` is also true for a broken image. The URL lease stays alive
    // across polling and is released even when the caller drops this future.
    let decode = async {
        let img = web_sys::HtmlImageElement::new()
            .map_err(|_| "Could not prepare this image. Choose it again.".to_string())?;
        img.set_src(url.as_str());
        loop {
            if img.complete() {
                if img.natural_width() == 0 || img.natural_height() == 0 {
                    return Err("Could not decode this image. Remove it and choose a valid JPEG, PNG, or WebP image.".to_string());
                }
                return Ok(img);
            }
            gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
        }
    };
    wait_for_image_operation(
        decode,
        gloo_timers::future::sleep(IMAGE_OPERATION_TIMEOUT),
        "This image took too long to decode. Remove it and choose another image.",
    )
    .await?
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
/// The browser owns a one-shot callback; the Rust wait owns its result channel.
/// Native encoding cannot be cancelled, so late callbacks safely discard results.
pub async fn blob_from_canvas(canvas: &HtmlCanvasElement, quality: f64) -> Result<Vec<u8>, String> {
    let (owner, receiver) = ImageCompletionOwner::channel();
    let completion = owner.callback();
    let callback = Closure::once(move |blob: Option<Blob>| completion.complete(blob));

    // canvas.toBlob(callback, mimeType, encoderOptions)
    canvas
        .to_blob_with_type_and_encoder_options(
            callback.as_ref().unchecked_ref(),
            "image/webp",
            &wasm_bindgen::JsValue::from_f64(quality),
        )
        .map_err(|e| format!("Failed to call toBlob: {:?}", e))?;
    // Transfer callback ownership only after toBlob succeeds. It stays callable
    // after cancellation and is reclaimed on invocation or browser JS GC.
    let _callback = callback.into_js_value();
    let blob = wait_for_image_operation(
        receiver,
        gloo_timers::future::sleep(IMAGE_OPERATION_TIMEOUT),
        "This image took too long to encode. Remove it and choose another image.",
    )
    .await?
    .map_err(|_| "Could not finish encoding this image. Choose it again.".to_string())?
    .ok_or("Could not encode this image as WebP. Remove it and choose another image.")?;
    drop(owner);

    // Read blob as ArrayBuffer
    blob_to_bytes(&blob).await
}

/// Read a Blob as ArrayBuffer, then convert to Vec<u8>.
///
/// Use the native Blob promise rather than retaining FileReader event handlers.
/// Both handlers hold weak completion references, not the waiting task or bytes.
pub async fn blob_to_bytes(blob: &Blob) -> Result<Vec<u8>, String> {
    let (owner, receiver) = ImageCompletionOwner::channel();
    let resolve = owner.callback();
    let reject = owner.callback();
    let on_resolve = Closure::once(move |value| resolve.complete(Ok(value)));
    let on_reject = Closure::once(move |reason| reject.complete(Err(reason)));
    let _settled = blob.array_buffer().then2(&on_resolve, &on_reject);
    let _on_resolve = on_resolve.into_js_value();
    let _on_reject = on_reject.into_js_value();
    let result = wait_for_image_operation(
        receiver,
        gloo_timers::future::sleep(IMAGE_OPERATION_TIMEOUT),
        "This image took too long to read. Remove it and choose another image.",
    )
    .await?
    .map_err(|_| "Could not finish reading this image. Choose it again.".to_string())?
    .map_err(|_| "Could not read this image. Remove it and choose another image.".to_string())?;
    drop(owner);
    let array_buf = result
        .dyn_into::<js_sys::ArrayBuffer>()
        .map_err(|_| "Could not read this image's bytes. Choose another image.")?;

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
        // Large landscape photo for thumbnail: 3000x2000 should fit within 800x800
        let (w, h) = calculate_dimensions(3000, 2000, THUMB_SIZE);
        assert_eq!(
            (w, h),
            (800, 533),
            "Wide image should preserve aspect ratio in thumbnail"
        );
    }

    #[test]
    fn test_calculate_dimensions_thumb_tall() {
        // Large portrait photo for thumbnail: 2000x3000 should fit within 800x800
        let (w, h) = calculate_dimensions(2000, 3000, THUMB_SIZE);
        assert_eq!(
            (w, h),
            (533, 800),
            "Tall image should preserve aspect ratio in thumbnail"
        );
    }
}
