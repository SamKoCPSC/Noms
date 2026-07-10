use std::sync::atomic::{AtomicU64, Ordering};

use dioxus::prelude::*;

use crate::types::{RecipeImage, RecipeImageEdit};

static FILTER_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Props, Clone, PartialEq)]
pub struct EditedRecipeImageProps {
    pub image: RecipeImage,
    pub alt: String,
    #[props(default)]
    pub class: String,
    #[props(default = "lazy".to_string())]
    pub loading: String,
    #[props(default = "cover".to_string())]
    pub fallback_fit: String,
    #[props(default = false)]
    pub prefer_thumbnail: bool,
    pub on_error: EventHandler<()>,
}

#[component]
pub fn EditedRecipeImage(props: EditedRecipeImageProps) -> Element {
    let filter_id = use_hook(|| {
        format!(
            "edited-recipe-image-filter-{}",
            FILTER_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    });
    let selected_edit = props
        .image
        .edits
        .iter()
        .find(|edit| edit.name == "16:9")
        .or_else(|| props.image.edits.first());

    let class = if props.class.is_empty() {
        "edited-recipe-image".to_string()
    } else {
        format!("edited-recipe-image {}", props.class)
    };
    let image_src = props.image.display_url(props.prefer_thumbnail).to_string();

    if let Some(edit) = selected_edit {
        let stage_style = crop_stage_style(edit);
        let image_style = edited_image_style(edit, &filter_id);
        let r = format!("{:.4}", edit.color.contrast_r);
        let g = format!("{:.4}", edit.color.contrast_g);
        let b = format!("{:.4}", edit.color.contrast_b);

        rsx! {
            div {
                class: "{class}",
                svg {
                    style: "position: absolute; width: 0; height: 0; overflow: hidden;",
                    xmlns: "http://www.w3.org/2000/svg",
                    defs {
                        filter {
                            id: "{filter_id}",
                            x: "0",
                            y: "0",
                            width: "100%",
                            height: "100%",
                            feComponentTransfer {
                                feFuncR { r#type: "linear", slope: "{r}" }
                                feFuncG { r#type: "linear", slope: "{g}" }
                                feFuncB { r#type: "linear", slope: "{b}" }
                            }
                        }
                    }
                }
                div {
                    class: "edited-recipe-image__stage",
                    style: "{stage_style}",
                    img {
                        class: "edited-recipe-image__img",
                        src: "{image_src}",
                        alt: "{props.alt}",
                        loading: "{props.loading}",
                        style: "{image_style}",
                        onerror: move |_| props.on_error.call(()),
                    }
                }
            }
        }
    } else {
        let plain_style = format!("object-fit: {};", props.fallback_fit);
        rsx! {
            div {
                class: "{class}",
                img {
                    class: "edited-recipe-image__plain",
                    src: "{image_src}",
                    alt: "{props.alt}",
                    loading: "{props.loading}",
                    style: "{plain_style}",
                    onerror: move |_| props.on_error.call(()),
                }
            }
        }
    }
}

fn crop_stage_style(edit: &RecipeImageEdit) -> String {
    let crop_w = edit.crop.width.clamp(0.001, 1.0);
    let crop_h = edit.crop.height.clamp(0.001, 1.0);
    let crop_x = edit.crop.x.clamp(0.0, 1.0 - crop_w);
    let crop_y = edit.crop.y.clamp(0.0, 1.0 - crop_h);

    let width_pct = 100.0 / crop_w;
    let height_pct = 100.0 / crop_h;
    let left_pct = -(crop_x / crop_w) * 100.0;
    let top_pct = -(crop_y / crop_h) * 100.0;

    format!(
        "left: {:.6}%; top: {:.6}%; width: {:.6}%; height: {:.6}%;",
        left_pct, top_pct, width_pct, height_pct
    )
}

fn edited_image_style(edit: &RecipeImageEdit, filter_id: &str) -> String {
    let flip_x = if edit.transform.flip_x { -1.0 } else { 1.0 };
    let flip_y = if edit.transform.flip_y { -1.0 } else { 1.0 };
    let scale_x = edit.transform.stretch_x * flip_x;
    let scale_y = edit.transform.stretch_y * flip_y;
    let fit_scale = transform_fit_scale(edit);

    format!(
        "transform: scale({:.6}) rotate({:.4}deg) scaleX({:.6}) scaleY({:.6}); filter: url(#{}) saturate({:.6});",
        fit_scale,
        edit.transform.rotation,
        scale_x,
        scale_y,
        filter_id,
        edit.color.saturation
    )
}

fn transform_fit_scale(edit: &RecipeImageEdit) -> f64 {
    let aspect_ratio = inferred_original_aspect_ratio(edit);
    if aspect_ratio <= 0.0 {
        return 1.0;
    }

    let stretch_x = edit.transform.stretch_x.abs();
    let stretch_y = edit.transform.stretch_y.abs();
    let radians = edit.transform.rotation.to_radians();
    let cos = radians.cos().abs();
    let sin = radians.sin().abs();

    let transformed_w = stretch_x * cos + (stretch_y * sin / aspect_ratio);
    let transformed_h = aspect_ratio * stretch_x * sin + stretch_y * cos;
    let fit_x = if transformed_w > 0.0 {
        1.0 / transformed_w
    } else {
        1.0
    };
    let fit_y = if transformed_h > 0.0 {
        1.0 / transformed_h
    } else {
        1.0
    };

    fit_x.min(fit_y).min(1.0).max(0.1)
}

fn inferred_original_aspect_ratio(edit: &RecipeImageEdit) -> f64 {
    let target_aspect = if edit.aspect_ratio.height > 0 {
        edit.aspect_ratio.width as f64 / edit.aspect_ratio.height as f64
    } else {
        16.0 / 9.0
    };

    let crop_w = edit.crop.width.clamp(0.001, 1.0);
    let crop_h = edit.crop.height.clamp(0.001, 1.0);

    target_aspect * crop_h / crop_w
}

#[cfg(test)]
mod tests {
    use crate::types::{
        ImageAspectRatio, ImageColorEdit, ImageCropRect, ImageTransformEdit, RecipeImageEdit,
    };

    use super::{edited_image_style, transform_fit_scale};

    fn edit_for_crop(crop: ImageCropRect, rotation: f64) -> RecipeImageEdit {
        RecipeImageEdit {
            name: "16:9".to_string(),
            version: 1,
            aspect_ratio: ImageAspectRatio {
                width: 16,
                height: 9,
            },
            crop,
            transform: ImageTransformEdit {
                rotation,
                stretch_x: 1.0,
                stretch_y: 1.0,
                flip_x: false,
                flip_y: false,
            },
            color: ImageColorEdit {
                saturation: 1.0,
                contrast_r: 1.0,
                contrast_g: 1.0,
                contrast_b: 1.0,
            },
        }
    }

    #[test]
    fn transform_fit_scale_matches_editor_for_rotated_four_by_three_image() {
        let edit = edit_for_crop(
            ImageCropRect {
                x: 0.0,
                y: 0.125,
                width: 1.0,
                height: 0.75,
            },
            90.0,
        );

        assert!((transform_fit_scale(&edit) - 0.75).abs() < 0.0001);
    }

    #[test]
    fn edited_image_style_replays_rotation_fit_scale() {
        let edit = edit_for_crop(
            ImageCropRect {
                x: 0.0,
                y: 0.125,
                width: 1.0,
                height: 0.75,
            },
            90.0,
        );

        let style = edited_image_style(&edit, "test-filter");

        assert!(style.contains("scale(0.750000) rotate(90.0000deg)"));
    }
}
