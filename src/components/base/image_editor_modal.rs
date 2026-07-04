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

    // Raw natural image dimensions (width, height) — set when the image loads.
    #[allow(unused_mut)]
    let mut img_dims = use_signal(|| None::<(f64, f64)>);

    // Scale multiplier: 0.1 (smallest) to 1.0 (maximum 16:9 fit).
    #[allow(unused_mut)]
    let mut scale = use_signal(|| 1.0f64);

    // X/Y pixel offset of the crop box centre from the image centre.
    #[allow(unused_mut)]
    let mut offset = use_signal(|| (0.0f64, 0.0f64));

    // Drag state.
    #[allow(unused_mut)]
    let mut is_dragging = use_signal(|| false);
    // (cursor screen position, offset) at drag start.
    #[allow(unused_mut)]
    let mut drag_start = use_signal(|| ((0.0f64, 0.0f64), (0.0f64, 0.0f64)));
    // True immediately after a drag release — used to suppress the overlay click-close.
    #[allow(unused_mut)]
    let mut just_released_drag = use_signal(|| false);

    // Resize state.
    #[allow(unused_mut)]
    let mut is_resizing = use_signal(|| false);
    // Which edge is being resized.
    #[allow(unused_mut)]
    let mut resize_handle = use_signal(|| ResizeHandle::None);
    // (cursor screen position, initial scale) at resize start.
    #[allow(unused_mut)]
    let mut resize_start = use_signal(|| ((0.0f64, 0.0f64), 0.0f64));

    // Image stretch/compress multipliers: 0.1 (compressed) to 1.0 (original).
    #[allow(unused_mut)]
    let mut stretch_x = use_signal(|| 1.0f64);
    #[allow(unused_mut)]
    let mut stretch_y = use_signal(|| 1.0f64);

    // Image rotation in degrees: -180 to 180, default 0.
    #[allow(unused_mut)]
    let mut rotation = use_signal(|| 0.0f64);

    // Memo: compute final bounding box percentages from img_dims, scale, offset.
    let crop_rect = use_memo(move || {
        let Some((img_w, img_h)) = img_dims() else {
            return None;
        };

        let target_ratio = 16.0 / 9.0;
        let img_ratio = img_w / img_h;

        // Base crop dimensions at scale 1.0 (maximum 16:9 fit).
        let (base_crop_w, base_crop_h) = if img_ratio < target_ratio {
            // Image is narrower than 16:9 — full width.
            (img_w, img_w / target_ratio)
        } else {
            // Image is wider or equal — full height.
            (img_h * target_ratio, img_h)
        };

        let crop_w = base_crop_w * scale();
        let crop_h = base_crop_h * scale();

        // Maximum allowed pixel offset in each direction.
        let max_off_x = (img_w - crop_w) / 2.0;
        let max_off_y = (img_h - crop_h) / 2.0;

        let (off_x, off_y) = offset();
        let off_x = off_x.clamp(-max_off_x, max_off_x);
        let off_y = off_y.clamp(-max_off_y, max_off_y);

        // Position of the crop box edges in image pixels (from top-left origin).
        let left_px = img_w / 2.0 - crop_w / 2.0 + off_x;
        let top_px = img_h / 2.0 - crop_h / 2.0 + off_y;

        // Convert to percentages of the image.
        Some(CropRectPct {
            top: (top_px / img_h) * 100.0,
            bottom: ((img_h - top_px - crop_h) / img_h) * 100.0,
            left: (left_px / img_w) * 100.0,
            right: ((img_w - left_px - crop_w) / img_w) * 100.0,
        })
    });

    // Memo: translation slider bounds (max offset per axis).
    let translation_bounds = use_memo(move || {
        let Some((img_w, img_h)) = img_dims() else {
            return None;
        };

        let target_ratio = 16.0 / 9.0;
        let img_ratio = img_w / img_h;
        let (base_crop_w, base_crop_h) = if img_ratio < target_ratio {
            (img_w, img_w / target_ratio)
        } else {
            (img_h * target_ratio, img_h)
        };

        let crop_w = base_crop_w * scale();
        let crop_h = base_crop_h * scale();
        let max_off_x = (img_w - crop_w) / 2.0;
        let max_off_y = (img_h - crop_h) / 2.0;
        Some((max_off_x, max_off_y))
    });

    if image_url().is_none() {
        return VNode::empty();
    }

    rsx! {
        div {
            class: "modal-overlay",
            onclick: move |_| {
                // Don't close if the user just finished dragging the crop box.
                let released = just_released_drag();
                just_released_drag.set(false);
                if !released {
                    on_close.call(());
                }
            },
            div {
                class: "modal-card modal-card--editor",
                onclick: move |evt| evt.stop_propagation(),
                // Action toolbar
                div {
                    class: "image-editor__toolbar",
                    // Crop size control
                    div {
                        class: "image-editor__control-group",
                        label {
                            class: "image-editor__scale-label",
                            "Crop size"
                        }
                        input {
                            r#type: "range",
                            class: "image-editor__scale-slider",
                            min: "0.1",
                            max: "1.0",
                            step: "0.01",
                            value: format!("{:.2}", scale()),
                            oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                                evt.stop_propagation();
                                let val: f64 = evt.value().parse().unwrap_or(1.0);
                                scale.set(val.clamp(0.1, 1.0));
                            },
                        }
                    }
                    // Stretch X/Y sliders (only shown once dimensions are known)
                    if let Some(_) = img_dims() {
                        div {
                            class: "image-editor__control-group",
                            label {
                                class: "image-editor__scale-label",
                                "Stretch"
                            }
                            div {
                                class: "image-editor__slider-group",
                                div {
                                    class: "image-editor__slider-row",
                                    label {
                                        class: "image-editor__scale-label",
                                        "X"
                                    }
                                    input {
                                        r#type: "range",
                                        class: "image-editor__scale-slider",
                                        min: "0.1",
                                        max: "1.0",
                                        step: "0.01",
                                        value: format!("{:.2}", stretch_x()),
                                        oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                                            evt.stop_propagation();
                                            let val: f64 = evt.value().parse().unwrap_or(1.0);
                                            stretch_x.set(val.clamp(0.1, 1.0));
                                        },
                                    }
                                }
                                div {
                                    class: "image-editor__slider-row",
                                    label {
                                        class: "image-editor__scale-label",
                                        "Y"
                                    }
                                    input {
                                        r#type: "range",
                                        class: "image-editor__scale-slider",
                                        min: "0.1",
                                        max: "1.0",
                                        step: "0.01",
                                        value: format!("{:.2}", stretch_y()),
                                        oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                                            evt.stop_propagation();
                                            let val: f64 = evt.value().parse().unwrap_or(1.0);
                                            stretch_y.set(val.clamp(0.1, 1.0));
                                        },
                                    }
                                }
                            }
                        }
                    }
                    // X/Y translation sliders (only shown once dimensions are known)
                    if let Some((max_off_x, max_off_y)) = (*translation_bounds)() {
                        div {
                            class: "image-editor__control-group",
                            label {
                                class: "image-editor__scale-label",
                                "Position"
                            }
                            div {
                                class: "image-editor__slider-group",
                                div {
                                    class: "image-editor__slider-row",
                                    label {
                                        class: "image-editor__scale-label",
                                        "X"
                                    }
                                    input {
                                        r#type: "range",
                                        class: "image-editor__scale-slider",
                                        min: format!("{}", -max_off_x),
                                        max: format!("{}", max_off_x),
                                        step: "1",
                                        value: format!("{:.0}", offset().0),
                                        oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                                            evt.stop_propagation();
                                            let val: f64 = evt.value().parse().unwrap_or(0.0);
                                            let (_, cur_y) = offset();
                                            offset.set((val.clamp(-max_off_x, max_off_x), cur_y));
                                        },
                                    }
                                }
                                div {
                                    class: "image-editor__slider-row",
                                    label {
                                        class: "image-editor__scale-label",
                                        "Y"
                                    }
                                    input {
                                        r#type: "range",
                                        class: "image-editor__scale-slider",
                                        min: format!("{}", -max_off_y),
                                        max: format!("{}", max_off_y),
                                        step: "1",
                                        value: format!("{:.0}", offset().1),
                                        oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                                            evt.stop_propagation();
                                            let val: f64 = evt.value().parse().unwrap_or(0.0);
                                            let (cur_x, _) = offset();
                                            offset.set((cur_x, val.clamp(-max_off_y, max_off_y)));
                                        },
                                    }
                                }
                            }
                        }
                    }
                    // Rotation slider (only shown once dimensions are known)
                    if let Some(_) = img_dims() {
                        div {
                            class: "image-editor__control-group",
                            label {
                                class: "image-editor__scale-label",
                                "Rotate"
                            }
                            input {
                                r#type: "range",
                                class: "image-editor__scale-slider",
                                min: "-180",
                                max: "180",
                                step: "1",
                                value: format!("{:.0}", rotation()),
                                oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                                    evt.stop_propagation();
                                    let val: f64 = evt.value().parse().unwrap_or(0.0);
                                    rotation.set(val.clamp(-180.0, 180.0));
                                },
                            }
                        }
                    }
                }
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
                        img_dims: img_dims,
                        scale: scale,
                        offset: offset,
                        is_dragging: is_dragging,
                        drag_start: drag_start,
                        is_resizing: is_resizing,
                        resize_handle: resize_handle,
                        resize_start: resize_start,
                        just_released_drag: just_released_drag,
                        stretch_x: stretch_x,
                        stretch_y: stretch_y,
                        rotation: rotation,
                        crop_rect: crop_rect,
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers (wasm only)
// ---------------------------------------------------------------------------

/// Convert a screen-pixel delta to an image-pixel delta.
/// `natural_dim` and `displayed_dim` must be the same axis (both width or both height).
#[cfg(target_arch = "wasm32")]
fn screen_to_image_delta(
    screen_delta: f64,
    natural_dim: f64,
    displayed_dim: f64,
) -> f64 {
    if displayed_dim <= 0.0 {
        return screen_delta;
    }
    let ratio = natural_dim / displayed_dim;
    screen_delta * ratio
}

/// Query the rendered (CSS) width and height of the image element.
/// Returns `(offset_width, offset_height)` as f64, or `(0.0, 0.0)` on failure.
#[cfg(target_arch = "wasm32")]
fn displayed_dims(img_id: &str) -> (f64, f64) {
    use wasm_bindgen::JsCast;

    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return (0.0, 0.0),
    };

    let Some(el) = doc.get_element_by_id(img_id) else {
        return (0.0, 0.0);
    };

    let Some(html_el) = el.dyn_ref::<web_sys::HtmlElement>() else {
        return (0.0, 0.0);
    };

    (html_el.offset_width() as f64, html_el.offset_height() as f64)
}

/// Extract (clientX, clientY) from the first touch point of a TouchEvent.
#[cfg(target_arch = "wasm32")]
fn touch_client_pos(evt: &dioxus::html::events::TouchEvent) -> Option<(f64, f64)> {
    let touches = evt.data.touches();
    if touches.is_empty() {
        return None;
    }
    let first = &touches[0];
    let coords = first.client_coordinates();
    Some((coords.x, coords.y))
}

// ---------------------------------------------------------------------------
// Internal rendering component
// ---------------------------------------------------------------------------

/// Renders the image, crop overlay, and scale slider.
/// Splits wasm / non-wasm so that web_sys imports only appear under wasm32.
#[component]
fn ImageWithCrop(
    img_id: String,
    url: String,
    img_dims: Signal<Option<(f64, f64)>>,
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    is_dragging: Signal<bool>,
    drag_start: Signal<((f64, f64), (f64, f64))>,
    is_resizing: Signal<bool>,
    resize_handle: Signal<ResizeHandle>,
    resize_start: Signal<((f64, f64), f64)>,
    just_released_drag: Signal<bool>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    crop_rect: Memo<Option<CropRectPct>>,
) -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;

        // --- Image onload: capture natural dimensions ---
        let img_id_load = img_id.clone();
        let mut on_img_load = move |_evt: dioxus::html::events::ImageEvent| {
            let doc = match web_sys::window().and_then(|w| w.document()) {
                Some(d) => d,
                None => return,
            };
            let Some(el) = doc
                .get_element_by_id(&img_id_load)
                .and_then(|e| e.dyn_into::<web_sys::HtmlImageElement>().ok())
            else {
                return;
            };
            let w = el.natural_width() as f64;
            let h = el.natural_height() as f64;
            if w > 0.0 && h > 0.0 {
                img_dims.set(Some((w, h)));
            }
        };

        // --- Global mouse listeners (document-level) so drag/resize continues
        //     when the cursor leaves the image wrapper ---
        use dioxus::core::use_hook;
        use dioxus::prelude::use_drop;
        use wasm_bindgen::closure::Closure;

        let img_id_global = img_id.clone();
        let mut is_dragging_g = is_dragging;
        let drag_start_g = drag_start;
        let mut offset_g = offset;
        let img_dims_g = img_dims;
        let mut just_released_drag_g = just_released_drag;
        let mut is_resizing_g = is_resizing;
        let resize_handle_g = resize_handle;
        let resize_start_g = resize_start;
        let mut scale_g = scale;

        // Store closures in a signal so use_drop can access them for cleanup.
        let global_closures = use_signal(|| {
            None::<(
                Closure<dyn FnMut(web_sys::MouseEvent)>,
                Closure<dyn FnMut(web_sys::MouseEvent)>,
            )>
        });

        use_hook({
            let mut closures = global_closures;
            move || {
                let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
                    return;
                };

                let on_global_move = Closure::wrap(Box::new(move |evt: web_sys::MouseEvent| {
                    evt.stop_propagation();
                    let current_screen = (evt.client_x() as f64, evt.client_y() as f64);

                    // Handle drag
                    if is_dragging_g() {
                        let (start_screen, start_offset_val) = drag_start_g();
                        let delta_screen = (
                            current_screen.0 - start_screen.0,
                            current_screen.1 - start_screen.1,
                        );
                        if let Some((iw, ih)) = img_dims_g() {
                            let (dw, dh) = displayed_dims(&img_id_global);
                            let px = screen_to_image_delta(delta_screen.0, iw, dw);
                            let py = screen_to_image_delta(delta_screen.1, ih, dh);
                            offset_g.set((start_offset_val.0 + px, start_offset_val.1 + py));
                        }
                        return;
                    }

                    // Handle resize
                    if is_resizing_g() {
                        let handle = resize_handle_g();
                        if handle == ResizeHandle::None {
                            return;
                        }
                        let (start_screen, start_scale_val) = resize_start_g();
                        let delta_screen = current_screen.0 - start_screen.0;
                        let delta_screen_y = current_screen.1 - start_screen.1;
                        if let Some((iw, ih)) = img_dims_g() {
                            let (dw, dh) = displayed_dims(&img_id_global);
                            let target_ratio = 16.0 / 9.0;
                            let img_ratio = iw / ih;
                            let (base_crop_w, base_crop_h) = if img_ratio < target_ratio {
                                (iw, iw / target_ratio)
                            } else {
                                (ih * target_ratio, ih)
                            };
                            let delta_px = match handle {
                                ResizeHandle::Left | ResizeHandle::Right => {
                                    screen_to_image_delta(delta_screen, iw, dw)
                                }
                                ResizeHandle::Top | ResizeHandle::Bottom => {
                                    screen_to_image_delta(delta_screen_y, ih, dh)
                                }
                                _ => 0.0,
                            };
                            let base_dim = match handle {
                                ResizeHandle::Left | ResizeHandle::Right => base_crop_w,
                                ResizeHandle::Top | ResizeHandle::Bottom => base_crop_h,
                                _ => 1.0,
                            };
                            let dir = match handle {
                                ResizeHandle::Right | ResizeHandle::Bottom => 1.0,
                                ResizeHandle::Left | ResizeHandle::Top => -1.0,
                                _ => 1.0,
                            };
                            let new_scale = (start_scale_val + dir * delta_px / base_dim)
                                .clamp(0.1, 1.0);
                            scale_g.set(new_scale);
                        }
                        return;
                    }
                }) as Box<dyn FnMut(web_sys::MouseEvent)>);

                let on_global_up = Closure::wrap(Box::new(move |_evt: web_sys::MouseEvent| {
                    let was_dragging = is_dragging_g();
                    is_dragging_g.set(false);
                    if was_dragging {
                        just_released_drag_g.set(true);
                    }
                    if is_resizing_g() {
                        is_resizing_g.set(false);
                        just_released_drag_g.set(true);
                    }
                }) as Box<dyn FnMut(web_sys::MouseEvent)>);

                let _ = doc.add_event_listener_with_callback("mousemove", on_global_move.as_ref().unchecked_ref());
                let _ = doc.add_event_listener_with_callback("mouseup", on_global_up.as_ref().unchecked_ref());

                closures.set(Some((on_global_move, on_global_up)));
            }
        });

        use_drop({
            let mut closures = global_closures;
            move || {
                if let Some((move_cl, up_cl)) = closures.take() {
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        let _ = doc.remove_event_listener_with_callback("mousemove", move_cl.as_ref().unchecked_ref());
                        let _ = doc.remove_event_listener_with_callback("mouseup", up_cl.as_ref().unchecked_ref());
                    }
                }
            }
        });

        // --- Mouse drag ---
        let mut on_crop_mousedown = move |evt: dioxus::html::events::MouseEvent| {
            evt.prevent_default(); // prevent browser's native image drag
            evt.stop_propagation();
            let coords = evt.data.client_coordinates();
            let pos = (coords.x, coords.y);
            drag_start.set((pos, offset()));
            is_dragging.set(true);
        };

        let img_id_move = img_id.clone();
        let mut on_crop_mousemove = move |evt: dioxus::html::events::MouseEvent| {
            if !is_dragging() {
                return;
            }
            evt.stop_propagation();
            let (start_screen, start_offset_val) = drag_start();
            let coords = evt.data.client_coordinates();
            let current_screen = (coords.x, coords.y);
            let delta_screen = (
                current_screen.0 - start_screen.0,
                current_screen.1 - start_screen.1,
            );
            if let Some((iw, ih)) = img_dims() {
                let (dw, dh) = displayed_dims(&img_id_move);
                let px = screen_to_image_delta(delta_screen.0, iw, dw);
                let py = screen_to_image_delta(delta_screen.1, ih, dh);
                offset.set((start_offset_val.0 + px, start_offset_val.1 + py));
            }
        };

        let mut on_crop_mouseup = move |_: dioxus::html::events::MouseEvent| {
            is_dragging.set(false);
        };

        // --- Touch drag ---
        let mut on_crop_touchstart = move |evt: dioxus::html::events::TouchEvent| {
            evt.stop_propagation();
            if let Some(pos) = touch_client_pos(&evt) {
                drag_start.set((pos, offset()));
                is_dragging.set(true);
            }
        };

        let img_id_tm = img_id.clone();
        let mut on_crop_touchmove = move |evt: dioxus::html::events::TouchEvent| {
            if !is_dragging() {
                return;
            }
            evt.prevent_default();
            evt.stop_propagation();
            if let Some(current_screen) = touch_client_pos(&evt) {
                let (start_screen, start_offset_val) = drag_start();
                let delta_screen = (
                    current_screen.0 - start_screen.0,
                    current_screen.1 - start_screen.1,
                );
                if let Some((iw, ih)) = img_dims() {
                    let (dw, dh) = displayed_dims(&img_id_tm);
                    let px = screen_to_image_delta(delta_screen.0, iw, dw);
                    let py = screen_to_image_delta(delta_screen.1, ih, dh);
                    offset.set((start_offset_val.0 + px, start_offset_val.1 + py));
                }
            }
        };

        let mut on_crop_touchend = move |_: dioxus::html::events::TouchEvent| {
            is_dragging.set(false);
        };

        // --- Resize handle mousedown ---
        let mut on_resize_mousedown =
            move |handle: ResizeHandle, evt: dioxus::html::events::MouseEvent| {
                evt.prevent_default();
                evt.stop_propagation();
                let coords = evt.data.client_coordinates();
                let pos = (coords.x, coords.y);
                resize_handle.set(handle);
                resize_start.set((pos, scale()));
                is_resizing.set(true);
            };

        return rsx! {
            div {
                class: "image-editor__image-wrapper",
                // Drag handlers on the wrapper so they fire even when the cursor
                // leaves the crop rectangle but is still over the image.
                onmousedown: move |evt| on_crop_mousedown(evt),
                onmousemove: move |evt| on_crop_mousemove(evt),
                onmouseup: move |evt| on_crop_mouseup(evt),
                ontouchstart: move |evt| on_crop_touchstart(evt),
                ontouchmove: move |evt| on_crop_touchmove(evt),
                ontouchend: move |evt| on_crop_touchend(evt),
                img {
                    id: img_id,
                    class: "image-editor__image",
                    src: url,
                    alt: "Image editor",
                    onload: move |evt| on_img_load(evt),
                    style: format!("transform: rotate({}deg) scaleX({}) scaleY({});", rotation(), stretch_x(), stretch_y()),
                }
                // Crop rectangle outline with resize handles (only rendered once dimensions are known).
                if let Some(rect) = (*crop_rect)() {
                    // Dark overlay: four panels outside the crop area
                    div {
                        class: "image-editor__crop-overlay image-editor__crop-overlay--top",
                        style: format!("height: {}%;", rect.top),
                    }
                    div {
                        class: "image-editor__crop-overlay image-editor__crop-overlay--bottom",
                        style: format!("height: {}%;", rect.bottom),
                    }
                    div {
                        class: "image-editor__crop-overlay image-editor__crop-overlay--left",
                        style: format!(
                            "top: {}%; width: {}%; height: calc(100% - {}% - {}%);",
                            rect.top, rect.left, rect.top, rect.bottom
                        ),
                    }
                    div {
                        class: "image-editor__crop-overlay image-editor__crop-overlay--right",
                        style: format!(
                            "top: {}%; width: {}%; height: calc(100% - {}% - {}%);",
                            rect.top, rect.right, rect.top, rect.bottom
                        ),
                    }
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
                        // Full-edge resize bars (invisible, clickable anywhere on the edge)
                        div {
                            class: "image-editor__resize-edge image-editor__resize-edge--top",
                            onmousedown: move |evt| on_resize_mousedown(ResizeHandle::Top, evt),
                        }
                        div {
                            class: "image-editor__resize-edge image-editor__resize-edge--bottom",
                            onmousedown: move |evt| on_resize_mousedown(ResizeHandle::Bottom, evt),
                        }
                        div {
                            class: "image-editor__resize-edge image-editor__resize-edge--left",
                            onmousedown: move |evt| on_resize_mousedown(ResizeHandle::Left, evt),
                        }
                        div {
                            class: "image-editor__resize-edge image-editor__resize-edge--right",
                            onmousedown: move |evt| on_resize_mousedown(ResizeHandle::Right, evt),
                        }
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

/// Which crop edge is being resized.
#[derive(Clone, Copy, PartialEq, Default)]
#[allow(dead_code)]
enum ResizeHandle {
    #[default]
    None,
    Top,
    Bottom,
    Left,
    Right,
}

/// Crop rectangle edge positions as percentages (0–100) of the image.
#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
struct CropRectPct {
    top: f64,
    bottom: f64,
    left: f64,
    right: f64,
}
