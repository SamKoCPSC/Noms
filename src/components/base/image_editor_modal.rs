//! Image editor modal: displays a selected image in a modal overlay.
//!
//! Used by `ImageUpload` to open the image editor for a pending image after
//! the user selects a file.

use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use crate::types::{ImageColorEdit, ImageCropRect, ImageTransformEdit, RecipeImageEdit};

/// Global counter for unique image element IDs.
static IMG_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Fixed crop templates supported by the image editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageEditorCropMode {
    #[default]
    Rectangle16x9,
    Ellipse1x1,
}

impl ImageEditorCropMode {
    fn target_ratio(self) -> f64 {
        match self {
            Self::Rectangle16x9 => 16.0 / 9.0,
            Self::Ellipse1x1 => 1.0,
        }
    }

    fn is_ellipse(self) -> bool {
        self == Self::Ellipse1x1
    }

    fn shows_adjustment_controls(self) -> bool {
        matches!(self, Self::Rectangle16x9 | Self::Ellipse1x1)
    }

    fn template(self) -> RecipeImageEdit {
        match self {
            Self::Rectangle16x9 => RecipeImageEdit::default_rect_16x9(),
            Self::Ellipse1x1 => RecipeImageEdit::default_ellipse_1x1(),
        }
    }
}

const DEFAULT_EDITOR_STATE: EditorEditState = EditorEditState {
    scale: 1.0,
    offset: (0.0, 0.0),
    stretch_x: 1.0,
    stretch_y: 1.0,
    rotation: 0.0,
    flip_x: false,
    flip_y: false,
    contrast_r: 1.0,
    contrast_g: 1.0,
    contrast_b: 1.0,
    saturation: 1.0,
};

#[derive(Clone, Copy, PartialEq)]
struct EditorEditState {
    scale: f64,
    offset: (f64, f64),
    stretch_x: f64,
    stretch_y: f64,
    rotation: f64,
    flip_x: bool,
    flip_y: bool,
    contrast_r: f64,
    contrast_g: f64,
    contrast_b: f64,
    saturation: f64,
}

#[cfg(target_arch = "wasm32")]
fn editor_transform_fit_scale(
    img_dims: Option<(f64, f64)>,
    stretch_x: f64,
    stretch_y: f64,
    rotation: f64,
) -> f64 {
    let Some((img_w, img_h)) = img_dims else {
        return 1.0;
    };
    if img_w <= 0.0 || img_h <= 0.0 {
        return 1.0;
    }

    let sx = stretch_x.abs();
    let sy = stretch_y.abs();
    let radians = rotation.to_radians();
    let cos = radians.cos().abs();
    let sin = radians.sin().abs();
    let transformed_w = img_w * sx * cos + img_h * sy * sin;
    let transformed_h = img_w * sx * sin + img_h * sy * cos;

    let fit_x = if transformed_w > 0.0 {
        img_w / transformed_w
    } else {
        1.0
    };
    let fit_y = if transformed_h > 0.0 {
        img_h / transformed_h
    } else {
        1.0
    };

    fit_x.min(fit_y).min(1.0).max(0.1)
}

/// Props for [`ImageEditorModal`].
#[derive(Props, Clone, PartialEq)]
pub struct ImageEditorModalProps {
    /// The image URL to display. `None` hides the modal.
    pub image_url: Signal<Option<String>>,
    /// Called when the user confirms the current editor state.
    pub on_save: EventHandler<RecipeImageEdit>,
    /// Called to close the modal (sets the parent's signal to `None`).
    pub on_close: EventHandler<()>,
    /// Crop geometry and aspect ratio. Recipe editing remains rectangular by
    /// default; avatars opt into the circular 1:1 template.
    #[props(default)]
    pub crop_mode: ImageEditorCropMode,
    /// Previously saved non-destructive edit state to restore when reopening.
    #[props(default)]
    pub initial_edit: Option<RecipeImageEdit>,
}

#[derive(Props, Clone, PartialEq)]
struct EditorSliderProps {
    label: String,
    tooltip: String,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
    #[props(default = 1.0)]
    display_multiplier: f64,
    #[props(default = 0)]
    decimals: usize,
    #[props(default)]
    suffix: String,
    on_edit_start: EventHandler<()>,
    on_edit_commit: EventHandler<()>,
    on_change: EventHandler<f64>,
}

#[component]
fn EditorSlider(props: EditorSliderProps) -> Element {
    let range_value = format!("{:.4}", props.value);
    let number_value = format!(
        "{:.*}",
        props.decimals,
        props.value * props.display_multiplier
    );
    let fill_pct = if props.max > props.min {
        ((props.value - props.min) / (props.max - props.min) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let min = format!("{:.4}", props.min);
    let max = format!("{:.4}", props.max);
    let step = format!("{:.4}", props.step);
    let number_min = format!("{:.4}", props.min * props.display_multiplier);
    let number_max = format!("{:.4}", props.max * props.display_multiplier);
    let number_step = format!("{:.4}", props.step * props.display_multiplier);

    rsx! {
        div {
            class: "image-editor__slider-row",
            title: "{props.tooltip}",
            label {
                class: "image-editor__slider-label",
                "{props.label}"
            }
            input {
                r#type: "range",
                class: "image-editor__scale-slider",
                style: format!("--slider-fill: {:.2}%;", fill_pct),
                min,
                max,
                step,
                value: "{range_value}",
                onmousedown: move |evt| {
                    evt.stop_propagation();
                    props.on_edit_start.call(());
                },
                ontouchstart: move |evt| {
                    evt.stop_propagation();
                    props.on_edit_start.call(());
                },
                oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                    evt.stop_propagation();
                    if let Ok(val) = evt.value().parse::<f64>() {
                        props.on_edit_start.call(());
                        props.on_change.call(val.clamp(props.min, props.max));
                    }
                },
                onchange: move |evt| {
                    evt.stop_propagation();
                    props.on_edit_commit.call(());
                },
                onmouseup: move |evt| {
                    evt.stop_propagation();
                    props.on_edit_commit.call(());
                },
                ontouchend: move |evt| {
                    evt.stop_propagation();
                    props.on_edit_commit.call(());
                },
            }
            div {
                class: "image-editor__number-shell",
                input {
                    r#type: "number",
                    class: "image-editor__number-input",
                    min: "{number_min}",
                    max: "{number_max}",
                    step: "{number_step}",
                    value: "{number_value}",
                    onfocus: move |_| props.on_edit_start.call(()),
                    oninput: move |evt: dioxus::prelude::Event<dioxus::html::events::FormData>| {
                        evt.stop_propagation();
                        if let Ok(display_val) = evt.value().parse::<f64>() {
                            let val = display_val / props.display_multiplier;
                            props.on_edit_start.call(());
                            props.on_change.call(val.clamp(props.min, props.max));
                        }
                    },
                    onchange: move |evt| {
                        evt.stop_propagation();
                        props.on_edit_commit.call(());
                    },
                    onblur: move |_| props.on_edit_commit.call(()),
                }
                if !props.suffix.is_empty() {
                    span { class: "image-editor__number-suffix", "{props.suffix}" }
                }
            }
        }
    }
}

/// Modal that opens the image editor for a selected image.
#[component]
pub fn ImageEditorModal(props: ImageEditorModalProps) -> Element {
    let image_url = props.image_url;
    let on_save = props.on_save;
    let on_close = props.on_close;
    let crop_mode = props.crop_mode;
    let initial_edit = props.initial_edit;
    let target_ratio = crop_mode.target_ratio();

    // Unique ID for this component instance's image element.
    let img_id = use_hook(|| {
        format!(
            "img-editor-{}",
            IMG_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });

    // Raw natural image dimensions (width, height) — set when the image loads.
    #[allow(unused_mut)]
    let mut img_dims = use_signal(|| None::<(f64, f64)>);

    // Scale multiplier: 0.1 (smallest) to 1.0 (maximum template fit).
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

    // Image flip/mirror toggles.
    #[allow(unused_mut)]
    let mut flip_x = use_signal(|| false);
    #[allow(unused_mut)]
    let mut flip_y = use_signal(|| false);

    // Per-channel RGB contrast multipliers: 0.1 (washed out) to 2.0 (high contrast).
    #[allow(unused_mut)]
    let mut contrast_r = use_signal(|| 1.0f64);
    #[allow(unused_mut)]
    let mut contrast_g = use_signal(|| 1.0f64);
    #[allow(unused_mut)]
    let mut contrast_b = use_signal(|| 1.0f64);

    // Global color saturation multiplier: 0.0 (grayscale) to 2.0 (extra vivid).
    #[allow(unused_mut)]
    let mut saturation = use_signal(|| 1.0f64);

    // History stacks store committed editor snapshots. `active_edit_start` is
    // used while a slider, number field, drag, or resize is still in progress.
    #[allow(unused_mut)]
    let mut undo_stack = use_signal(Vec::<EditorEditState>::new);
    #[allow(unused_mut)]
    let mut redo_stack = use_signal(Vec::<EditorEditState>::new);
    #[allow(unused_mut)]
    let mut active_edit_start = use_signal(|| None::<EditorEditState>);
    #[allow(unused_mut)]
    let mut active_image_url = use_signal(|| None::<String>);
    let mut initialized_image_url = use_signal(|| None::<String>);

    use_effect(move || {
        let current_url = image_url();
        if active_image_url() == current_url {
            return;
        }

        active_image_url.set(current_url);
        initialized_image_url.set(None);
        img_dims.set(None);
        apply_editor_state(
            DEFAULT_EDITOR_STATE,
            scale,
            offset,
            stretch_x,
            stretch_y,
            rotation,
            flip_x,
            flip_y,
            contrast_r,
            contrast_g,
            contrast_b,
            saturation,
        );
        undo_stack.write().clear();
        redo_stack.write().clear();
        active_edit_start.set(None);
        is_dragging.set(false);
        drag_start.set(((0.0, 0.0), (0.0, 0.0)));
        just_released_drag.set(false);
        is_resizing.set(false);
        resize_handle.set(ResizeHandle::None);
        resize_start.set(((0.0, 0.0), 0.0));
    });

    use_effect(move || {
        let Some(current_url) = image_url() else {
            return;
        };
        if initialized_image_url().as_ref() == Some(&current_url) {
            return;
        }
        let Some(dimensions) = img_dims() else {
            return;
        };
        let Some(edit) = initial_edit.as_ref() else {
            initialized_image_url.set(Some(current_url));
            return;
        };

        apply_editor_state(
            editor_state_from_edit(edit, dimensions, target_ratio),
            scale,
            offset,
            stretch_x,
            stretch_y,
            rotation,
            flip_x,
            flip_y,
            contrast_r,
            contrast_g,
            contrast_b,
            saturation,
        );
        initialized_image_url.set(Some(current_url));
    });

    #[cfg(target_arch = "wasm32")]
    {
        use dioxus::core::use_hook;
        use dioxus::prelude::use_drop;
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let shortcut_closure = use_signal(|| None::<Closure<dyn FnMut(web_sys::KeyboardEvent)>>);

        use_hook({
            let mut closure_signal = shortcut_closure;
            move || {
                let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                    return;
                };

                let closure = Closure::wrap(Box::new(move |evt: web_sys::KeyboardEvent| {
                    let key = evt.key().to_ascii_lowercase();
                    let uses_command_key = evt.ctrl_key() || evt.meta_key();
                    let undo_requested = key == "z" && uses_command_key && !evt.shift_key();
                    let redo_requested = (key == "y" && evt.ctrl_key())
                        || (key == "z" && uses_command_key && evt.shift_key());

                    if undo_requested {
                        evt.prevent_default();
                        undo_editor_state(
                            undo_stack,
                            redo_stack,
                            active_edit_start,
                            scale,
                            offset,
                            stretch_x,
                            stretch_y,
                            rotation,
                            flip_x,
                            flip_y,
                            contrast_r,
                            contrast_g,
                            contrast_b,
                            saturation,
                        );
                    } else if redo_requested {
                        evt.prevent_default();
                        redo_editor_state(
                            undo_stack,
                            redo_stack,
                            active_edit_start,
                            scale,
                            offset,
                            stretch_x,
                            stretch_y,
                            rotation,
                            flip_x,
                            flip_y,
                            contrast_r,
                            contrast_g,
                            contrast_b,
                            saturation,
                        );
                    }
                })
                    as Box<dyn FnMut(web_sys::KeyboardEvent)>);

                let _ = document
                    .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
                closure_signal.set(Some(closure));
            }
        });

        use_drop({
            let mut closure_signal = shortcut_closure;
            move || {
                if let Some(closure) = closure_signal.take() {
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        let _ = document.remove_event_listener_with_callback(
                            "keydown",
                            closure.as_ref().unchecked_ref(),
                        );
                    }
                }
            }
        });
    }

    // Memo: compute final bounding box percentages from img_dims, scale, offset.
    let crop_rect = use_memo(move || {
        let Some((img_w, img_h)) = img_dims() else {
            return None;
        };

        let img_ratio = img_w / img_h;

        // Base crop dimensions at scale 1.0 (maximum template fit).
        let (base_crop_w, base_crop_h) = if img_ratio < target_ratio {
            // Image is narrower than the target — full width.
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

    macro_rules! on_edit_start {
        () => {
            move |_| {
                begin_editor_edit(
                    active_edit_start,
                    scale,
                    offset,
                    stretch_x,
                    stretch_y,
                    rotation,
                    flip_x,
                    flip_y,
                    contrast_r,
                    contrast_g,
                    contrast_b,
                    saturation,
                );
            }
        };
    }

    macro_rules! on_edit_commit {
        () => {
            move |_| {
                commit_editor_edit(
                    undo_stack,
                    redo_stack,
                    active_edit_start,
                    scale,
                    offset,
                    stretch_x,
                    stretch_y,
                    rotation,
                    flip_x,
                    flip_y,
                    contrast_r,
                    contrast_g,
                    contrast_b,
                    saturation,
                );
            }
        };
    }

    if image_url().is_none() {
        return VNode::empty();
    }

    let current_state = current_editor_state(
        scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r, contrast_g,
        contrast_b, saturation,
    );
    let active_edit_can_undo = match active_edit_start() {
        Some(start_state) => start_state != current_state,
        None => false,
    };
    let undo_disabled = undo_stack().is_empty() && !active_edit_can_undo;
    let redo_disabled = redo_stack().is_empty();

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
                div {
                    class: "image-editor__header",
                    div {
                        h2 { "Edit image" }
                        span { class: "image-editor__subtitle", "Visual preview" }
                    }
                    div {
                        class: "image-editor__header-actions",
                        div {
                            class: "image-editor__history-actions",
                            button {
                                class: "image-editor__icon-button",
                                title: "Undo (Ctrl+Z / Cmd+Z)",
                                disabled: undo_disabled,
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    undo_editor_state(
                                        undo_stack,
                                        redo_stack,
                                        active_edit_start,
                                        scale,
                                        offset,
                                        stretch_x,
                                        stretch_y,
                                        rotation,
                                        flip_x,
                                        flip_y,
                                        contrast_r,
                                        contrast_g,
                                        contrast_b,
                                        saturation,
                                    );
                                },
                                "↶"
                            }
                            button {
                                class: "image-editor__icon-button",
                                title: "Redo (Ctrl+Y / Cmd+Shift+Z)",
                                disabled: redo_disabled,
                                onclick: move |evt| {
                                    evt.stop_propagation();
                                    redo_editor_state(
                                        undo_stack,
                                        redo_stack,
                                        active_edit_start,
                                        scale,
                                        offset,
                                        stretch_x,
                                        stretch_y,
                                        rotation,
                                        flip_x,
                                        flip_y,
                                        contrast_r,
                                        contrast_g,
                                        contrast_b,
                                        saturation,
                                    );
                                },
                                "↷"
                            }
                        }
                        button {
                            class: "image-editor__soft-button",
                            title: "Reset adjustments",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                set_editor_state_with_history(
                                    undo_stack,
                                    redo_stack,
                                    active_edit_start,
                                    DEFAULT_EDITOR_STATE,
                                    scale,
                                    offset,
                                    stretch_x,
                                    stretch_y,
                                    rotation,
                                    flip_x,
                                    flip_y,
                                    contrast_r,
                                    contrast_g,
                                    contrast_b,
                                    saturation,
                                );
                            },
                            "Reset"
                        }
                        button {
                            class: "image-editor__icon-button",
                            title: "Close editor",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                on_close.call(());
                            },
                            "×"
                        }
                    }
                }
                div {
                    class: "image-editor__body",
                    div {
                        class: "image-editor__preview-panel",
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
                                flip_x: flip_x,
                                flip_y: flip_y,
                                contrast_r: contrast_r,
                                contrast_g: contrast_g,
                                contrast_b: contrast_b,
                                saturation: saturation,
                                crop_rect: crop_rect,
                                crop_mode: crop_mode,
                                on_edit_start: on_edit_start!(),
                                on_edit_commit: on_edit_commit!(),
                            }
                        }
                    }
                    aside {
                        class: "image-editor__controls",
                        div {
                            class: "image-editor__section",
                            div { class: "image-editor__section-title", "Frame" }
                            EditorSlider {
                                label: "Crop".to_string(),
                                tooltip: "Crop window size",
                                min: 0.1,
                                max: 1.0,
                                step: 0.01,
                                value: scale(),
                                display_multiplier: 100.0,
                                decimals: 0,
                                suffix: "%".to_string(),
                                on_edit_start: on_edit_start!(),
                                on_edit_commit: on_edit_commit!(),
                                on_change: move |val| scale.set(val),
                            }
                            if let Some((max_off_x, max_off_y)) = (*translation_bounds)() {
                                EditorSlider {
                                    label: "X".to_string(),
                                    tooltip: "Horizontal crop position",
                                    min: -max_off_x,
                                    max: max_off_x,
                                    step: 1.0,
                                    value: offset().0.clamp(-max_off_x, max_off_x),
                                    decimals: 0,
                                    suffix: "px".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| {
                                        let (_, cur_y) = offset();
                                        offset.set((val, cur_y));
                                    },
                                }
                                EditorSlider {
                                    label: "Y".to_string(),
                                    tooltip: "Vertical crop position",
                                    min: -max_off_y,
                                    max: max_off_y,
                                    step: 1.0,
                                    value: offset().1.clamp(-max_off_y, max_off_y),
                                    decimals: 0,
                                    suffix: "px".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| {
                                        let (cur_x, _) = offset();
                                        offset.set((cur_x, val));
                                    },
                                }
                            }
                        }
                        if let Some(_) = img_dims() {
                            if crop_mode.shows_adjustment_controls() {
                            div {
                                class: "image-editor__section",
                                div { class: "image-editor__section-title", "Transform" }
                                EditorSlider {
                                    label: "Rotate".to_string(),
                                    tooltip: "Rotation in degrees",
                                    min: -180.0,
                                    max: 180.0,
                                    step: 1.0,
                                    value: rotation(),
                                    decimals: 0,
                                    suffix: "°".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| rotation.set(val),
                                }
                                EditorSlider {
                                    label: "Width".to_string(),
                                    tooltip: "Horizontal image scale",
                                    min: 0.1,
                                    max: 1.0,
                                    step: 0.01,
                                    value: stretch_x(),
                                    display_multiplier: 100.0,
                                    decimals: 0,
                                    suffix: "%".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| stretch_x.set(val),
                                }
                                EditorSlider {
                                    label: "Height".to_string(),
                                    tooltip: "Vertical image scale",
                                    min: 0.1,
                                    max: 1.0,
                                    step: 0.01,
                                    value: stretch_y(),
                                    display_multiplier: 100.0,
                                    decimals: 0,
                                    suffix: "%".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| stretch_y.set(val),
                                }
                                div {
                                    class: "image-editor__toggle-row",
                                    span { class: "image-editor__slider-label", "Flip" }
                                    div {
                                        class: "image-editor__toggle-group",
                                        button {
                                            class: format!("image-editor__flip-toggle {}", if flip_x() { "image-editor__flip-toggle--active" } else { "" }),
                                            title: "Flip horizontally",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                let mut next = current_editor_state(
                                                    scale,
                                                    offset,
                                                    stretch_x,
                                                    stretch_y,
                                                    rotation,
                                                    flip_x,
                                                    flip_y,
                                                    contrast_r,
                                                    contrast_g,
                                                    contrast_b,
                                                    saturation,
                                                );
                                                next.flip_x = !next.flip_x;
                                                set_editor_state_with_history(
                                                    undo_stack,
                                                    redo_stack,
                                                    active_edit_start,
                                                    next,
                                                    scale,
                                                    offset,
                                                    stretch_x,
                                                    stretch_y,
                                                    rotation,
                                                    flip_x,
                                                    flip_y,
                                                    contrast_r,
                                                    contrast_g,
                                                    contrast_b,
                                                    saturation,
                                                );
                                            },
                                            "↔"
                                        }
                                        button {
                                            class: format!("image-editor__flip-toggle {}", if flip_y() { "image-editor__flip-toggle--active" } else { "" }),
                                            title: "Flip vertically",
                                            onclick: move |evt| {
                                                evt.stop_propagation();
                                                let mut next = current_editor_state(
                                                    scale,
                                                    offset,
                                                    stretch_x,
                                                    stretch_y,
                                                    rotation,
                                                    flip_x,
                                                    flip_y,
                                                    contrast_r,
                                                    contrast_g,
                                                    contrast_b,
                                                    saturation,
                                                );
                                                next.flip_y = !next.flip_y;
                                                set_editor_state_with_history(
                                                    undo_stack,
                                                    redo_stack,
                                                    active_edit_start,
                                                    next,
                                                    scale,
                                                    offset,
                                                    stretch_x,
                                                    stretch_y,
                                                    rotation,
                                                    flip_x,
                                                    flip_y,
                                                    contrast_r,
                                                    contrast_g,
                                                    contrast_b,
                                                    saturation,
                                                );
                                            },
                                            "↕"
                                        }
                                    }
                                }
                            }
                            div {
                                class: "image-editor__section",
                                div { class: "image-editor__section-title", "Color" }
                                EditorSlider {
                                    label: "Saturation".to_string(),
                                    tooltip: "Color intensity",
                                    min: 0.0,
                                    max: 2.0,
                                    step: 0.01,
                                    value: saturation(),
                                    display_multiplier: 100.0,
                                    decimals: 0,
                                    suffix: "%".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| saturation.set(val),
                                }
                                EditorSlider {
                                    label: "Red".to_string(),
                                    tooltip: "Red channel contrast",
                                    min: 0.1,
                                    max: 2.0,
                                    step: 0.01,
                                    value: contrast_r(),
                                    display_multiplier: 100.0,
                                    decimals: 0,
                                    suffix: "%".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| contrast_r.set(val),
                                }
                                EditorSlider {
                                    label: "Green".to_string(),
                                    tooltip: "Green channel contrast",
                                    min: 0.1,
                                    max: 2.0,
                                    step: 0.01,
                                    value: contrast_g(),
                                    display_multiplier: 100.0,
                                    decimals: 0,
                                    suffix: "%".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| contrast_g.set(val),
                                }
                                EditorSlider {
                                    label: "Blue".to_string(),
                                    tooltip: "Blue channel contrast",
                                    min: 0.1,
                                    max: 2.0,
                                    step: 0.01,
                                    value: contrast_b(),
                                    display_multiplier: 100.0,
                                    decimals: 0,
                                    suffix: "%".to_string(),
                                    on_edit_start: on_edit_start!(),
                                    on_edit_commit: on_edit_commit!(),
                                    on_change: move |val| contrast_b.set(val),
                                }
                            }
                            }
                        }
                    }
                }
                div {
                    class: "image-editor__footer",
                    button {
                        class: "image-editor__soft-button",
                        title: "Cancel edits and close",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            on_close.call(());
                        },
                        "Cancel"
                    }
                    button {
                        class: "image-editor__primary-button",
                        title: "Confirm edits and close",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            on_save.call(current_recipe_image_edit(
                                crop_mode,
                                (*crop_rect)(),
                                stretch_x(),
                                stretch_y(),
                                rotation(),
                                flip_x(),
                                flip_y(),
                                contrast_r(),
                                contrast_g(),
                                contrast_b(),
                                saturation(),
                            ));
                            on_close.call(());
                        },
                        "OK"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod crop_mode_tests {
    use super::{editor_state_from_edit, ImageEditorCropMode};
    use crate::types::{ImageCropRect, RecipeImageEdit};

    #[test]
    fn editor_layout_bounds_preview_and_keeps_content_reachable() {
        let editor_styles = include_str!("../../../assets/main.css");

        assert!(editor_styles.contains(".image-editor__body {\n    flex: 1 1 auto;"));
        assert!(editor_styles.contains("    overflow-y: auto;\n}"));
        assert!(editor_styles.contains(".image-editor__preview-panel {\n    min-width: 0;"));
        assert!(editor_styles.contains("    height: min(48vh, 560px);"));
        assert!(editor_styles.contains("    flex: 0 0 auto;"));
        assert!(editor_styles.contains("    container-type: size;"));
        assert!(editor_styles.contains("100cqw"));
        assert!(editor_styles.contains("100cqh"));
        assert!(editor_styles.contains(
            ".image-editor__body {\n        overflow: hidden;\n    }\n\n    .image-editor__controls {"
        ));
        assert!(editor_styles.contains(
            "        flex: 1 1 auto;\n        min-height: 0;\n        max-height: none;\n        padding-right: 0;\n        overflow-y: auto;"
        ));
        assert!(editor_styles.contains(
            "@media (min-width: 901px) and (max-height: 720px) {\n    .image-editor__body {\n        overflow: hidden;\n    }"
        ));
        assert!(editor_styles.contains(
            "        flex: 1 1 auto;\n        min-height: 0;\n        max-height: none;\n        overflow-y: auto;\n    }\n}"
        ));
    }

    #[test]
    fn both_crop_modes_show_transform_and_color_controls() {
        assert!(ImageEditorCropMode::Rectangle16x9.shows_adjustment_controls());
        assert!(ImageEditorCropMode::Ellipse1x1.shows_adjustment_controls());
    }

    #[test]
    fn saved_edit_restores_crop_transform_and_color_state() {
        let mut edit = RecipeImageEdit::default_ellipse_1x1();
        edit.crop = ImageCropRect {
            x: 0.25,
            y: 0.0,
            width: 0.5,
            height: 1.0,
        };
        edit.transform.rotation = 30.0;
        edit.transform.flip_x = true;
        edit.color.saturation = 0.4;

        let state = editor_state_from_edit(&edit, (800.0, 400.0), 1.0);

        assert_eq!(state.scale, 1.0);
        assert_eq!(state.offset, (0.0, 0.0));
        assert_eq!(state.rotation, 30.0);
        assert!(state.flip_x);
        assert_eq!(state.saturation, 0.4);
    }
}

fn editor_state_from_edit(
    edit: &RecipeImageEdit,
    (image_width, image_height): (f64, f64),
    target_ratio: f64,
) -> EditorEditState {
    let image_ratio = image_width / image_height;
    let (base_crop_width, base_crop_height) = if image_ratio < target_ratio {
        (image_width, image_width / target_ratio)
    } else {
        (image_height * target_ratio, image_height)
    };
    let crop_width = edit.crop.width.clamp(0.001, 1.0) * image_width;
    let crop_height = edit.crop.height.clamp(0.001, 1.0) * image_height;
    let width_scale = crop_width / base_crop_width;
    let height_scale = crop_height / base_crop_height;
    let scale = width_scale.min(height_scale).clamp(0.1, 1.0);
    let rendered_crop_width = base_crop_width * scale;
    let rendered_crop_height = base_crop_height * scale;
    let crop_center_x = edit.crop.x * image_width + crop_width / 2.0;
    let crop_center_y = edit.crop.y * image_height + crop_height / 2.0;
    let max_offset_x = (image_width - rendered_crop_width) / 2.0;
    let max_offset_y = (image_height - rendered_crop_height) / 2.0;

    EditorEditState {
        scale,
        offset: (
            (crop_center_x - image_width / 2.0).clamp(-max_offset_x, max_offset_x),
            (crop_center_y - image_height / 2.0).clamp(-max_offset_y, max_offset_y),
        ),
        stretch_x: edit.transform.stretch_x,
        stretch_y: edit.transform.stretch_y,
        rotation: edit.transform.rotation,
        flip_x: edit.transform.flip_x,
        flip_y: edit.transform.flip_y,
        contrast_r: edit.color.contrast_r,
        contrast_g: edit.color.contrast_g,
        contrast_b: edit.color.contrast_b,
        saturation: edit.color.saturation,
    }
}

fn current_editor_state(
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
) -> EditorEditState {
    EditorEditState {
        scale: scale(),
        offset: offset(),
        stretch_x: stretch_x(),
        stretch_y: stretch_y(),
        rotation: rotation(),
        flip_x: flip_x(),
        flip_y: flip_y(),
        contrast_r: contrast_r(),
        contrast_g: contrast_g(),
        contrast_b: contrast_b(),
        saturation: saturation(),
    }
}

fn apply_editor_state(
    state: EditorEditState,
    mut scale: Signal<f64>,
    mut offset: Signal<(f64, f64)>,
    mut stretch_x: Signal<f64>,
    mut stretch_y: Signal<f64>,
    mut rotation: Signal<f64>,
    mut flip_x: Signal<bool>,
    mut flip_y: Signal<bool>,
    mut contrast_r: Signal<f64>,
    mut contrast_g: Signal<f64>,
    mut contrast_b: Signal<f64>,
    mut saturation: Signal<f64>,
) {
    scale.set(state.scale);
    offset.set(state.offset);
    stretch_x.set(state.stretch_x);
    stretch_y.set(state.stretch_y);
    rotation.set(state.rotation);
    flip_x.set(state.flip_x);
    flip_y.set(state.flip_y);
    contrast_r.set(state.contrast_r);
    contrast_g.set(state.contrast_g);
    contrast_b.set(state.contrast_b);
    saturation.set(state.saturation);
}

fn push_editor_history(
    mut undo_stack: Signal<Vec<EditorEditState>>,
    mut redo_stack: Signal<Vec<EditorEditState>>,
    before: EditorEditState,
    after: EditorEditState,
) {
    if before == after {
        return;
    }

    undo_stack.write().push(before);
    redo_stack.write().clear();
}

fn begin_editor_edit(
    mut active_edit_start: Signal<Option<EditorEditState>>,
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
) {
    if active_edit_start().is_some() {
        return;
    }

    active_edit_start.set(Some(current_editor_state(
        scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r, contrast_g,
        contrast_b, saturation,
    )));
}

fn commit_editor_edit(
    undo_stack: Signal<Vec<EditorEditState>>,
    redo_stack: Signal<Vec<EditorEditState>>,
    mut active_edit_start: Signal<Option<EditorEditState>>,
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
) {
    let Some(before) = active_edit_start.take() else {
        return;
    };
    let after = current_editor_state(
        scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r, contrast_g,
        contrast_b, saturation,
    );
    push_editor_history(undo_stack, redo_stack, before, after);
}

fn set_editor_state_with_history(
    undo_stack: Signal<Vec<EditorEditState>>,
    redo_stack: Signal<Vec<EditorEditState>>,
    mut active_edit_start: Signal<Option<EditorEditState>>,
    next: EditorEditState,
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
) {
    let before = active_edit_start.take().unwrap_or_else(|| {
        current_editor_state(
            scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r, contrast_g,
            contrast_b, saturation,
        )
    });

    push_editor_history(undo_stack, redo_stack, before, next);
    apply_editor_state(
        next, scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r,
        contrast_g, contrast_b, saturation,
    );
}

fn undo_editor_state(
    mut undo_stack: Signal<Vec<EditorEditState>>,
    mut redo_stack: Signal<Vec<EditorEditState>>,
    mut active_edit_start: Signal<Option<EditorEditState>>,
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
) {
    let current = current_editor_state(
        scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r, contrast_g,
        contrast_b, saturation,
    );

    if let Some(before_active_edit) = active_edit_start.take() {
        if before_active_edit != current {
            redo_stack.write().push(current);
            apply_editor_state(
                before_active_edit,
                scale,
                offset,
                stretch_x,
                stretch_y,
                rotation,
                flip_x,
                flip_y,
                contrast_r,
                contrast_g,
                contrast_b,
                saturation,
            );
            return;
        }
    }

    let Some(previous) = undo_stack.write().pop() else {
        return;
    };
    if previous == current {
        return;
    }

    redo_stack.write().push(current);
    apply_editor_state(
        previous, scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r,
        contrast_g, contrast_b, saturation,
    );
}

fn redo_editor_state(
    mut undo_stack: Signal<Vec<EditorEditState>>,
    mut redo_stack: Signal<Vec<EditorEditState>>,
    active_edit_start: Signal<Option<EditorEditState>>,
    scale: Signal<f64>,
    offset: Signal<(f64, f64)>,
    stretch_x: Signal<f64>,
    stretch_y: Signal<f64>,
    rotation: Signal<f64>,
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
) {
    commit_editor_edit(
        undo_stack,
        redo_stack,
        active_edit_start,
        scale,
        offset,
        stretch_x,
        stretch_y,
        rotation,
        flip_x,
        flip_y,
        contrast_r,
        contrast_g,
        contrast_b,
        saturation,
    );

    let current = current_editor_state(
        scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r, contrast_g,
        contrast_b, saturation,
    );
    let Some(next) = redo_stack.write().pop() else {
        return;
    };
    if next == current {
        return;
    }

    undo_stack.write().push(current);
    apply_editor_state(
        next, scale, offset, stretch_x, stretch_y, rotation, flip_x, flip_y, contrast_r,
        contrast_g, contrast_b, saturation,
    );
}

// ---------------------------------------------------------------------------
// Helpers (wasm only)
// ---------------------------------------------------------------------------

/// Convert a screen-pixel delta to an image-pixel delta.
/// `natural_dim` and `displayed_dim` must be the same axis (both width or both height).
#[cfg(target_arch = "wasm32")]
fn screen_to_image_delta(screen_delta: f64, natural_dim: f64, displayed_dim: f64) -> f64 {
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

    (
        html_el.offset_width() as f64,
        html_el.offset_height() as f64,
    )
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
    flip_x: Signal<bool>,
    flip_y: Signal<bool>,
    contrast_r: Signal<f64>,
    contrast_g: Signal<f64>,
    contrast_b: Signal<f64>,
    saturation: Signal<f64>,
    crop_rect: Memo<Option<CropRectPct>>,
    crop_mode: ImageEditorCropMode,
    on_edit_start: EventHandler<()>,
    on_edit_commit: EventHandler<()>,
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

        // --- SVG filter for per-channel RGB contrast ---
        let filter_id = format!("{}-contrast-filter", img_id);
        let filter_id_style = filter_id.clone();
        // Use a memo to track contrast values so we can trigger an effect on change.
        let contrast_vals = use_memo(move || (contrast_r(), contrast_g(), contrast_b()));
        let filter_id_memo = filter_id.clone();
        use dioxus::prelude::use_effect;
        use_effect(move || {
            let (r, g, b) = contrast_vals();
            let doc = match web_sys::window().and_then(|w| w.document()) {
                Some(d) => d,
                None => return,
            };
            for (sel, val) in [("feFuncR", r), ("feFuncG", g), ("feFuncB", b)] {
                let full_sel = format!("#{} {}", filter_id_memo, sel);
                if let Some(el) = doc.query_selector(&full_sel).ok().flatten() {
                    let _ = el.set_attribute("slope", &format!("{:.4}", val));
                }
            }
        });

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
        let on_edit_commit_g = on_edit_commit.clone();

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
                            let target_ratio = crop_mode.target_ratio();
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
                            let new_scale =
                                (start_scale_val + dir * delta_px / base_dim).clamp(0.1, 1.0);
                            scale_g.set(new_scale);
                        }
                        return;
                    }
                })
                    as Box<dyn FnMut(web_sys::MouseEvent)>);

                let on_global_up = Closure::wrap(Box::new(move |_evt: web_sys::MouseEvent| {
                    let was_dragging = is_dragging_g();
                    let was_resizing = is_resizing_g();
                    is_dragging_g.set(false);
                    if was_dragging {
                        just_released_drag_g.set(true);
                    }
                    if was_resizing {
                        is_resizing_g.set(false);
                        just_released_drag_g.set(true);
                    }
                    if was_dragging || was_resizing {
                        on_edit_commit_g.call(());
                    }
                })
                    as Box<dyn FnMut(web_sys::MouseEvent)>);

                let _ = doc.add_event_listener_with_callback(
                    "mousemove",
                    on_global_move.as_ref().unchecked_ref(),
                );
                let _ = doc.add_event_listener_with_callback(
                    "mouseup",
                    on_global_up.as_ref().unchecked_ref(),
                );

                closures.set(Some((on_global_move, on_global_up)));
            }
        });

        use_drop({
            let mut closures = global_closures;
            move || {
                if let Some((move_cl, up_cl)) = closures.take() {
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        let _ = doc.remove_event_listener_with_callback(
                            "mousemove",
                            move_cl.as_ref().unchecked_ref(),
                        );
                        let _ = doc.remove_event_listener_with_callback(
                            "mouseup",
                            up_cl.as_ref().unchecked_ref(),
                        );
                    }
                }
            }
        });

        // --- Mouse drag ---
        let on_drag_start = on_edit_start.clone();
        let mut on_crop_mousedown = move |evt: dioxus::html::events::MouseEvent| {
            evt.prevent_default(); // prevent browser's native image drag
            evt.stop_propagation();
            on_drag_start.call(());
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

        let on_drag_commit = on_edit_commit.clone();
        let mut on_crop_mouseup = move |_: dioxus::html::events::MouseEvent| {
            let was_dragging = is_dragging();
            is_dragging.set(false);
            if was_dragging {
                just_released_drag.set(true);
                on_drag_commit.call(());
            }
        };

        // --- Touch drag ---
        let on_touch_start = on_edit_start.clone();
        let mut on_crop_touchstart = move |evt: dioxus::html::events::TouchEvent| {
            evt.stop_propagation();
            if let Some(pos) = touch_client_pos(&evt) {
                on_touch_start.call(());
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

        let on_touch_commit = on_edit_commit.clone();
        let mut on_crop_touchend = move |_: dioxus::html::events::TouchEvent| {
            let was_dragging = is_dragging();
            is_dragging.set(false);
            if was_dragging {
                just_released_drag.set(true);
                on_touch_commit.call(());
            }
        };

        // --- Resize handle mousedown ---
        let on_resize_start = on_edit_start.clone();
        let mut on_resize_mousedown =
            move |handle: ResizeHandle, evt: dioxus::html::events::MouseEvent| {
                evt.prevent_default();
                evt.stop_propagation();
                on_resize_start.call(());
                let coords = evt.data.client_coordinates();
                let pos = (coords.x, coords.y);
                resize_handle.set(handle);
                resize_start.set((pos, scale()));
                is_resizing.set(true);
            };

        let transform_fit =
            editor_transform_fit_scale(img_dims(), stretch_x(), stretch_y(), rotation());
        let transform_style = format!(
            "transform: scale({}) rotate({}deg) scaleX({}) scaleY({});",
            transform_fit,
            rotation(),
            stretch_x() * if flip_x() { -1.0 } else { 1.0 },
            stretch_y() * if flip_y() { -1.0 } else { 1.0 },
        );
        let image_filter_style = format!(
            "filter: url(#{}) saturate({});",
            filter_id_style,
            saturation()
        );
        let wrapper_style = img_dims().map(|(w, h)| {
            let aspect = if h > 0.0 { w / h } else { 1.0 };
            format!(
                "--image-aspect: {:.6}; --image-natural-width: {:.2}px; aspect-ratio: {:.6};",
                aspect, w, aspect
            )
        });

        return rsx! {
            div {
                class: "image-editor__image-wrapper",
                style: wrapper_style.unwrap_or_default(),
                // Drag handlers on the wrapper so they fire even when the cursor
                // leaves the crop rectangle but is still over the image.
                onmousedown: move |evt| on_crop_mousedown(evt),
                onmousemove: move |evt| on_crop_mousemove(evt),
                onmouseup: move |evt| on_crop_mouseup(evt),
                ontouchstart: move |evt| on_crop_touchstart(evt),
                ontouchmove: move |evt| on_crop_touchmove(evt),
                ontouchend: move |evt| on_crop_touchend(evt),
                div {
                    class: "image-editor__transform-stage",
                    style: transform_style,
                    // Inline SVG filter for per-channel RGB contrast
                    svg {
                        style: "position: absolute; width: 0; height: 0; overflow: hidden;",
                        xmlns: "http://www.w3.org/2000/svg",
                        defs {
                            filter {
                                id: filter_id,
                                x: "0",
                                y: "0",
                                width: "100%",
                                height: "100%",
                                feComponentTransfer {
                                    feFuncR {
                                        r#type: "linear",
                                        slope: "1.0",
                                    }
                                    feFuncG {
                                        r#type: "linear",
                                        slope: "1.0",
                                    }
                                    feFuncB {
                                        r#type: "linear",
                                        slope: "1.0",
                                    }
                                }
                            }
                        }
                    }
                    img {
                        id: img_id,
                        class: "image-editor__image",
                        src: url,
                        alt: "Image editor",
                        onload: move |evt| on_img_load(evt),
                        style: image_filter_style,
                    }
                }
                // Crop rectangle outline with resize handles. This sits outside
                // the transformed image stage so rotation/stretch never rotate
                // or resize the crop window itself.
                if let Some(rect) = (*crop_rect)() {
                    div {
                        class: if crop_mode.is_ellipse() { "image-editor__crop-stage image-editor__crop-stage--ellipse" } else { "image-editor__crop-stage" },
                        if crop_mode.is_ellipse() {
                            div {
                                class: "image-editor__crop-ellipse-mask",
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
                        } else {
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
                        }
                        div {
                            class: if crop_mode.is_ellipse() { "image-editor__crop-rect image-editor__crop-rect--ellipse" } else { "image-editor__crop-rect" },
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
            }
        };
    }

    #[cfg(not(target_arch = "wasm32"))]
    rsx! {
        div {
            class: if crop_mode.is_ellipse() { "image-editor__image-wrapper image-editor__image-wrapper--ellipse" } else { "image-editor__image-wrapper" },
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

fn current_recipe_image_edit(
    crop_mode: ImageEditorCropMode,
    crop_rect: Option<CropRectPct>,
    stretch_x: f64,
    stretch_y: f64,
    rotation: f64,
    flip_x: bool,
    flip_y: bool,
    contrast_r: f64,
    contrast_g: f64,
    contrast_b: f64,
    saturation: f64,
) -> RecipeImageEdit {
    let crop = crop_rect
        .map(|rect| {
            let x = (rect.left / 100.0).clamp(0.0, 1.0);
            let y = (rect.top / 100.0).clamp(0.0, 1.0);
            let width = (1.0 - (rect.left + rect.right) / 100.0).clamp(0.0, 1.0);
            let height = (1.0 - (rect.top + rect.bottom) / 100.0).clamp(0.0, 1.0);
            ImageCropRect {
                x,
                y,
                width,
                height,
            }
        })
        .unwrap_or(ImageCropRect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        });

    let mut edit = crop_mode.template();
    edit.crop = crop;
    edit.transform = ImageTransformEdit {
        rotation,
        stretch_x,
        stretch_y,
        flip_x,
        flip_y,
    };
    edit.color = ImageColorEdit {
        saturation,
        contrast_r,
        contrast_g,
        contrast_b,
    };
    edit
}
