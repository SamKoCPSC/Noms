//! Image editor modal: displays a selected image in a modal overlay.
//!
//! Used by `ImageUpload` to open the image editor for a pending image after
//! the user selects a file.

use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use crate::components::base::{Button, ButtonVariant};

/// Global counter for unique image element IDs.
static IMG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Props for [`ImageEditorModal`].
#[derive(Props, Clone, PartialEq)]
pub struct ImageEditorModalProps {
    /// The image URL to display. `None` hides the modal.
    pub image_url: Signal<Option<String>>,
    /// Called to close the modal (sets the parent's signal to `None`).
    pub on_close: EventHandler<()>,
}

/// Modal that opens the image editor for a selected image.
#[component]
pub fn ImageEditorModal(props: ImageEditorModalProps) -> Element {
    let image_url = props.image_url;
    let on_close = props.on_close;

    // Unique ID for this component instance's image element.
    let img_id = use_hook(|| {
        format!("img-editor-{}", IMG_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    });

    // Crop rectangle — `None` while the image is downloading, `Some` once computed.
    #[allow(unused_mut)]
    let mut crop_rect = use_signal(|| None::<CropRectPct>);

    if image_url().is_none() {
        return VNode::empty();
    }

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| {
                on_close.call(());
            },
            div {
                class: "modal-card modal-card--editor",
                onclick: move |evt| evt.stop_propagation(),
                // Close button
                div {
                    position: "absolute",
                    top: "var(--space-sm)",
                    right: "var(--space-sm)",
                    z_index: "1",
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| {
                            on_close.call(());
                        },
                        "✕"
                    }
                }
                // Image with crop rectangle outline
                if let Some(ref url) = image_url() {
                    ImageWithCrop {
                        img_id: img_id.clone(),
                        url: url.clone(),
                        crop_rect: crop_rect,
                    }
                }
            }
        }
    }
}

/// Internal component that renders the image and crop overlay.
/// Handles the `onload` event differently for wasm vs non-wasm targets.
#[component]
fn ImageWithCrop(
    img_id: String,
    url: String,
    crop_rect: Signal<Option<CropRectPct>>,
) -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        let img_id_clone = img_id.clone();
        return rsx! {
            div {
                class: "image-editor__image-wrapper",
                img {
                    id: img_id,
                    class: "image-editor__image",
                    src: url,
                    alt: "Image editor",
                    onload: move |_| {
                        use wasm_bindgen::JsCast;

                        let doc = web_sys::window().unwrap().document().unwrap();
                        let Some(el) = doc
                            .get_element_by_id(&img_id_clone)
                            .and_then(|e| e.dyn_into::<web_sys::HtmlImageElement>().ok())
                        else {
                            return;
                        };

                        let w = el.natural_width() as f64;
                        let h = el.natural_height() as f64;
                        if w > 0.0 && h > 0.0 {
                            crop_rect.set(Some(compute_crop_rect_pct(w, h)));
                        }
                    },
                }
                // Crop rectangle outline (only rendered once computed)
                if let Some(rect) = crop_rect() {
                    div {
                        class: "image-editor__crop-rect",
                        style: format!(
                            "top: {}%; left: {}%; width: calc(100% - {}% - {}%); height: calc(100% - {}% - {}%);",
                            rect.top,
                            rect.left,
                            rect.left,
                            rect.right,
                            rect.top,
                            rect.bottom,
                        ),
                    }
                }
            }
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    rsx! {
        div {
            class: "image-editor__image-wrapper",
            img {
                id: img_id,
                class: "image-editor__image",
                src: url,
                alt: "Image editor",
            }
        }
    }
}

/// Crop rectangle edge positions as percentages (0–100) of the image.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct CropRectPct {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
}

/// Compute a centered 16:9 crop rectangle as percentages of the image dimensions.
#[cfg(target_arch = "wasm32")]
fn compute_crop_rect_pct(img_width: f64, img_height: f64) -> CropRectPct {
    let target_ratio = 16.0 / 9.0;
    let img_ratio = img_width / img_height;

    if img_ratio < target_ratio {
        // Image is narrower than 16:9 — use full width, center vertically.
        let crop_height = img_width / target_ratio;
        let top_bottom_pct = (1.0 - crop_height / img_height) * 50.0;
        CropRectPct {
            top: top_bottom_pct,
            bottom: top_bottom_pct,
            left: 0.0,
            right: 0.0,
        }
    } else {
        // Image is wider than or equal to 16:9 — use full height, center horizontally.
        let crop_width = img_height * target_ratio;
        let left_right_pct = (1.0 - crop_width / img_width) * 50.0;
        CropRectPct {
            top: 0.0,
            bottom: 0.0,
            left: left_right_pct,
            right: left_right_pct,
        }
    }
}
