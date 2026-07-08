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
                        src: "{props.image.url}",
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
                    src: "{props.image.url}",
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

    format!(
        "transform: rotate({:.4}deg) scaleX({:.6}) scaleY({:.6}); filter: url(#{}) saturate({:.6});",
        edit.transform.rotation, scale_x, scale_y, filter_id, edit.color.saturation
    )
}
