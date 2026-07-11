//! Recipe edit page.
//!
//! Fetches an existing recipe and tags, pre-populates a form identical to the
//! create form, and calls `update_recipe()` on submit.

use dioxus::prelude::*;
use std::rc::Rc;

use crate::api::recipe::{get_recipe, get_recipe_tags, update_recipe};
use crate::api::upload::delete_image;
#[allow(unused_imports)]
use crate::api::upload::upload_image;
use crate::components::base::{
    Button, ButtonVariant, EditedRecipeImage, ImageEditorModal, ImageUpload, Input, LoadingSpinner,
    PageHeader, UploadOutcome,
};
use crate::components::AuthRequired;
use crate::types::{RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient, RecipeStep};
use crate::utils::reorder::move_item;

// ── Local form types ─────────────────────────────────────────────────────────

/// A single step row in the form (flat representation with depth for nesting).
#[derive(Clone, Debug)]
struct StepForm {
    text: String,
    depth: usize,
    photos: Vec<RecipeImage>,
}

#[derive(Clone, Debug, PartialEq)]
struct PendingStepPhotoEdit {
    idx: usize,
    preview_url: String,
    edit: RecipeImageEdit,
}

/// Flatten a tree of `RecipeStep` into a flat list with depth information.
fn flatten_steps(steps: &[RecipeStep], depth: usize) -> Vec<StepForm> {
    let mut result = Vec::new();
    for step in steps {
        result.push(StepForm {
            text: step.text.clone(),
            depth,
            photos: step.display_photos(),
        });
        result.extend(flatten_steps(&step.sub_steps, depth + 1));
    }
    result
}

/// Build a tree of `RecipeStep` from a flat list with depth information.
fn build_step_tree(flat: &[StepForm]) -> Vec<RecipeStep> {
    if flat.is_empty() {
        return Vec::new();
    }

    // Build tree using a stack of (depth, index_in_parent_list)
    // We construct the tree bottom-up using indices into the result vector.
    let mut root = Vec::new();
    // Stack holds (depth, parent_index) where parent_index is the index in
    // the vector that currently holds sub_steps we're appending to.
    // We use a flat representation: a list of RecipeSteps where we track
    // parent-child relationships via indices.
    let mut nodes: Vec<RecipeStep> = Vec::new();
    // Stack of (depth, node_index) — the node at that index is the current parent
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for step in flat {
        let node = RecipeStep {
            text: step.text.clone(),
            sub_steps: Vec::new(),
            photos: step.photos.clone(),
        };
        let node_idx = nodes.len();
        nodes.push(node);

        // Pop stack until we find the right parent depth
        while stack.len() > 1 && stack.last().unwrap().0 >= step.depth {
            let _ = stack.pop();
        }

        if step.depth == 0 {
            // Root-level step — add to root
            root.push(nodes[node_idx].clone());
            stack.push((0, node_idx));
        } else {
            // Find parent and add this node as sub_step
            let parent_idx = stack.last().unwrap().1;
            let node_clone = nodes[node_idx].clone();
            nodes[parent_idx].sub_steps.push(node_clone);
            stack.push((step.depth, node_idx));
        }
    }

    root
}

// ── Component ────────────────────────────────────────────────────────────────

/// Programmatically trigger a click on a file input element by selector.
/// Only active on wasm32; no-op during SSR.
#[cfg(target_arch = "wasm32")]
fn trigger_file_input(selector: &str) {
    use wasm_bindgen::JsCast;
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Ok(Some(input)) = doc.query_selector(selector) {
                if let Some(file_input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    let _ = file_input.click();
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn trigger_file_input(_selector: &str) {}

fn format_step_photo_file_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;

    if bytes >= MIB {
        format!("{:.1} MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{} KB", bytes / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn step_photo_display_name(photo: &RecipeImage) -> String {
    photo.file_name.clone().unwrap_or_else(|| {
        photo
            .url
            .split('?')
            .next()
            .and_then(|url| url.rsplit('/').next())
            .filter(|name| !name.is_empty())
            .unwrap_or("Step photo")
            .to_string()
    })
}

/// Edit recipe page — fetches existing data, pre-populates the form, saves via update_recipe.
#[component]
pub fn RecipeEdit(id: String) -> Element {
    // ── Form signals ─────────────────────────────────────────────────────
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut commentary = use_signal(String::new);
    let mut prep_time = use_signal(String::new);
    let mut cook_time = use_signal(String::new);
    let mut servings = use_signal(String::new);
    let mut ingredients = use_signal(Vec::<RecipeIngredient>::new);
    let mut steps = use_signal(Vec::<StepForm>::new);
    let mut tags_input = use_signal(String::new);
    let mut equipment = use_signal(Vec::<RecipeEquipment>::new);
    let mut visibility = use_signal(|| "private".to_string());

    // CP5: image management signals
    let mut images = use_signal(Vec::<RecipeImage>::new);
    let mut upload_now = use_signal(|| false);
    let busy = use_signal(|| false);
    let mut step_editor_modal_url = use_signal(|| Option::<String>::None);
    let mut step_editor_idx = use_signal(|| Option::<usize>::None);
    let mut step_editor_photo_idx = use_signal(|| Option::<usize>::None);
    let mut pending_step_photo_edits = use_signal(Vec::<PendingStepPhotoEdit>::new);
    let step_photo_upload_count = use_signal(|| 0usize);
    let mut dragged_step_photo = use_signal(|| Option::<(usize, usize)>::None);

    // CP11: deferred delete — track URLs to delete after save succeeds
    let mut pending_deletes = use_signal(Vec::<String>::new);

    // UI state
    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut load_error = use_signal(|| Option::<String>::None);
    let mut form_error = use_signal(|| Option::<String>::None);

    // ── Fetch recipe data on mount ───────────────────────────────────────
    let id_for_fetch = id.clone();
    use_effect(move || {
        let id = id_for_fetch.clone();
        spawn(async move {
            match get_recipe(id.clone()).await {
                Ok(recipe) => {
                    title.set(recipe.title.clone());
                    description.set(recipe.description.clone().unwrap_or_default());
                    commentary.set(recipe.commentary.clone().unwrap_or_default());
                    prep_time.set(
                        recipe
                            .prep_time_minutes
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    );
                    cook_time.set(
                        recipe
                            .cook_time_minutes
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    );
                    servings.set(recipe.servings.map(|v| v.to_string()).unwrap_or_default());

                    // Use typed ingredients directly
                    ingredients.set(recipe.ingredients);

                    // Flatten tree steps into flat list with depth
                    steps.set(flatten_steps(&recipe.instructions, 0));

                    // Fetch tags
                    if let Ok(tags) = get_recipe_tags(id).await {
                        tags_input.set(tags.join(", "))
                    }

                    // Use typed equipment directly
                    equipment.set(recipe.equipment);

                    visibility.set(recipe.visibility);

                    // CP5: pre-populate images signal with existing recipe images
                    images.set(recipe.images.clone());
                }
                Err(e) => {
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };
                    if !msg.contains("Not authenticated") {
                        load_error.set(Some(msg));
                    }
                }
            }
            is_loading.set(false);
        });
    });

    // ── Clone id for closures that need it ──────────────────────────────
    let _id_for_submit = id.clone(); // used in on_submit (two-phase: no direct DB call)
    #[allow(unused_variables)]
    let id_for_cancel = id.clone();
    // CP5: Rc for the on_upload_complete handler (callable multiple times)
    let id_for_upload_complete = Rc::new(id.clone());

    // ── Step photo upload handler ────────────────────────────────────────
    #[allow(unused_variables)]
    let on_step_photo_change = move |idx: usize| {
        move |files: Vec<dioxus::html::FileData>| {
            if files.is_empty() {
                return;
            }
            #[cfg(target_arch = "wasm32")]
            {
                let mut steps = steps;
                let mut step_editor_modal_url = step_editor_modal_url;
                let mut step_editor_idx = step_editor_idx;
                let mut step_editor_photo_idx = step_editor_photo_idx;
                let mut pending_step_photo_edits = pending_step_photo_edits;
                let mut step_photo_upload_count = step_photo_upload_count;
                spawn(async move {
                    use dioxus_web::WebFileExt;

                    for (file_idx, file_data) in files.iter().enumerate() {
                        let web_file = match file_data.get_web_file() {
                            Some(f) => f,
                            None => continue,
                        };
                        let photo_file_name = web_file.name();
                        let photo_file_size = web_file.size() as u64;
                        step_photo_upload_count.set(step_photo_upload_count().saturating_add(1));
                        let preview_url = web_sys::Url::create_object_url_with_blob(&web_file).ok();
                        if let Some(url) = &preview_url {
                            step_editor_idx.set(Some(idx));
                            step_editor_photo_idx.set(None);
                            step_editor_modal_url.set(Some(url.clone()));
                        }

                        let processed =
                            match crate::utils::image_processing_client::process_image(web_file)
                                .await
                            {
                                Ok(p) => p,
                                Err(e) => {
                                    tracing::error!("Step photo processing failed: {e}");
                                    step_photo_upload_count
                                        .set(step_photo_upload_count().saturating_sub(1));
                                    continue;
                                }
                            };

                        let resp = match upload_image(
                            processed.full_bytes,
                            processed.thumb_bytes,
                            format!("step_{idx}_{file_idx}.webp"),
                        )
                        .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::error!("Step photo upload failed: {e}");
                                step_photo_upload_count
                                    .set(step_photo_upload_count().saturating_sub(1));
                                continue;
                            }
                        };

                        let edit = preview_url
                            .as_ref()
                            .and_then(|url| {
                                pending_step_photo_edits()
                                    .iter()
                                    .rev()
                                    .find(|pending| {
                                        pending.idx == idx && pending.preview_url == *url
                                    })
                                    .map(|pending| pending.edit.clone())
                            })
                            .unwrap_or_else(RecipeImageEdit::default_16x9);
                        if let Some(url) = &preview_url {
                            pending_step_photo_edits.write().retain(|pending| {
                                !(pending.idx == idx && pending.preview_url == *url)
                            });
                        }

                        let full_url = resp.full_url;
                        let photo = RecipeImage::new(full_url, Some(resp.thumb_url), edit)
                            .with_file_metadata(photo_file_name, photo_file_size);
                        if let Some(step) = steps.write().get_mut(idx) {
                            let photo_idx = step.photos.len();
                            step.photos.push(photo);
                            if preview_url.as_ref() == step_editor_modal_url().as_ref()
                                && step_editor_idx() == Some(idx)
                            {
                                step_editor_photo_idx.set(Some(photo_idx));
                            }
                        }
                        step_photo_upload_count.set(step_photo_upload_count().saturating_sub(1));
                    }
                });
            }
        }
    };

    // ── Submit handler (two-phase: upload images → save recipe) ────────
    let on_submit = move |_| {
        // Clear previous errors
        form_error.set(None);

        // ── Client-side validation ──────────────────────────────────────
        let trimmed_title = title().trim().to_string();
        if trimmed_title.is_empty() {
            form_error.set(Some("Title is required".to_string()));
            return;
        }

        let ings = ingredients();
        if ings.is_empty() {
            form_error.set(Some("Add at least one ingredient".to_string()));
            return;
        }
        if ings.iter().all(|i| i.name.trim().is_empty()) {
            form_error.set(Some("At least one ingredient must have a name".to_string()));
            return;
        }

        let sts = steps();
        if sts.is_empty() {
            form_error.set(Some("Add at least one step".to_string()));
            return;
        }
        if sts.iter().all(|s| s.text.trim().is_empty()) {
            form_error.set(Some("At least one step must have text".to_string()));
            return;
        }

        // ── Early guard: images still processing ────────────────────────
        if busy() {
            form_error.set(Some(
                "Images are still processing — please wait a moment".to_string(),
            ));
            return;
        }

        // ── Parse optional numeric fields ───────────────────────────────
        let _prep = prep_time().trim().parse::<i32>().ok();
        let _cook = cook_time().trim().parse::<i32>().ok();
        let _serv = servings().trim().parse::<i32>().ok();

        // ── Trigger upload batch (may be empty — no-op) ─────────────────
        is_saving.set(true);
        upload_now.set(true);
    };

    // ── CP5: Upload complete handler ────────────────────────────────────
    // Fired by ImageUpload when the upload batch finishes. Proceeds to
    // update_recipe with the merged images list (existing + newly uploaded).
    let on_upload_complete = move |outcome: UploadOutcome| {
        let id_rc = id_for_upload_complete.clone();
        spawn(async move {
            if let Some(msg) = outcome.aborted {
                form_error.set(Some(msg));
                is_saving.set(false);
                return;
            }

            if outcome.failed > 0 {
                form_error.set(Some(format!(
                    "{} image(s) failed to upload. Use the Retry button on each failed image (or remove them), then click Save again.",
                    outcome.failed
                )));
                is_saving.set(false);
                return;
            }

            // Re-validate and parse form data (signals are still accessible)
            let trimmed_title = title().trim().to_string();
            let ings = ingredients();
            let sts = steps();

            // Parse optional numeric fields
            let prep = prep_time().trim().parse::<i32>().ok();
            let cook = cook_time().trim().parse::<i32>().ok();
            let serv = servings().trim().parse::<i32>().ok();

            // Parse tags
            let tags: Vec<String> = tags_input()
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            // Build typed vectors from form data
            let typed_ingredients = ings;
            let instructions = build_step_tree(&sts);
            let equip_vec = equipment();

            let desc = if description().trim().is_empty() {
                None
            } else {
                Some(description().trim().to_string())
            };

            let comm = if commentary().trim().is_empty() {
                None
            } else {
                Some(commentary().trim().to_string())
            };

            let vis = visibility().clone();
            let recipe_id = (*id_rc).clone();

            match update_recipe(
                recipe_id.clone(),
                Some(trimmed_title),
                desc,
                comm,
                prep,
                cook,
                serv,
                Some(typed_ingredients),
                Some(instructions),
                Some(equip_vec),
                Some(images()), // CP5: merged image list (existing + newly uploaded)
                Some(tags),
                Some(vis),
            )
            .await
            {
                Ok(_) => {
                    // CP11: Delete images that were marked for removal
                    let deletes = pending_deletes();
                    for url in deletes {
                        let _ = delete_image(url).await;
                    }
                    // Clear pending deletes
                    pending_deletes.set(Vec::new());

                    // Redirect to recipe detail page
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window
                                .location()
                                .set_href(&format!("/recipes/{}", recipe_id));
                        }
                    }
                }
                Err(e) => {
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };
                    form_error.set(Some(msg));
                    is_saving.set(false);
                }
            }
        });
    };

    // ── Cancel handler ──────────────────────────────────────────────────
    let on_cancel = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let _ = window
                    .location()
                    .set_href(&format!("/recipes/{}", id_for_cancel));
            }
        }
    };

    // ── Ingredient helpers ──────────────────────────────────────────────
    let on_add_ingredient = move |_| {
        ingredients.write().push(RecipeIngredient {
            amount: String::new(),
            unit: String::new(),
            name: String::new(),
        });
    };

    // ── Step helpers ────────────────────────────────────────────────────
    let on_add_step = move |_| {
        steps.write().push(StepForm {
            text: String::new(),
            depth: 0,
            photos: Vec::new(),
        });
    };

    // ── Equipment helpers ───────────────────────────────────────────────
    let on_add_equipment = move |_| {
        equipment.write().push(RecipeEquipment {
            name: String::new(),
        });
    };

    // ── CP11: Deferred image delete handler ────────────────────────────────────
    let mut on_delete_image = move |url: String| {
        // Optimistically remove from UI
        images.write().retain(|image| image.url != url);
        // Track for deferred deletion after save
        pending_deletes.write().push(url);
    };

    // ── Render: loading ─────────────────────────────────────────────────
    if is_loading() {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "300px",
                        LoadingSpinner {}
                        p {
                            margin_top: "var(--space-md)",
                            color: "var(--text-secondary)",
                            "Loading recipe..."
                        }
                    }
                }
            }
        };
    }

    // ── Render: error ───────────────────────────────────────────────────
    if let Some(err_msg) = load_error() {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        text_align: "center",
                        gap: "var(--space-md)",
                        padding: "var(--space-xl)",
                        p {
                            color: "var(--error)",
                            font_weight: "600",
                            font_size: "18px",
                            "{err_msg}"
                        }
                        Link {
                            to: crate::Route::Dashboard {},
                            class: "btn btn-secondary touch-target",
                            "Back to Recipes"
                        }
                    }
                }
            }
        };
    }

    // ── Render: form ────────────────────────────────────────────────────
    rsx! {
            AuthRequired {
                div { class: "container",
                    PageHeader {
                        title: "Edit Recipe",
                    }
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-md)",
                        max_width: "600px",

                        // ── Basic Info ──────────────────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Recipe Name *"
                            }
                            Input {
                                value: title().clone(),
                                placeholder: "e.g. Grandma's Chocolate Chip Cookies",
                                oninput: move |v| {
                                    title.set(v);
                                    form_error.set(None);
                                },
                            }
                        }

                        // ── CP5: Recipe Images ──────────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Recipe Images"
                            }

    // Existing images grid (index-based to avoid temporary borrow issues)
                            for idx in 0..images().len() {
                                div {
                                    key: "{idx}",
                                    position: "relative",
                                    display: "inline-block",
                                    width: "100px",
                                    margin_right: "var(--space-sm)",
                                    margin_bottom: "var(--space-sm)",
                                    EditedRecipeImage {
                                        image: images()[idx].clone(),
                                        class: "recipe-image-preview",
                                        alt: "Recipe image".to_string(),
                                        prefer_thumbnail: true,
                                        on_error: move |_| {},
                                    }
                                    div {
                                        position: "absolute",
                                        top: "4px",
                                        right: "4px",
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                let url = images()[idx].url.clone();
                                                on_delete_image(url);
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }

                            // ImageUpload component for adding new images
                            ImageUpload {
                                images,
                                upload_now,
                                busy,
                                on_upload_complete,
                            }
                            span {
                                font_size: "12px",
                                color: "var(--text-tertiary)",
                                "Add more images (JPEG, PNG, or WebP · max 10 MB each)"
                            }
                        }

                        // Description
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Description"
                            }
                            textarea {
                                class: "neumo-inset input",
                                placeholder: "Brief description of the recipe...",
                                rows: "4",
                                padding: "var(--space-sm) var(--space-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-primary)",
                                background_color: "var(--surface)",
                                outline: "none",
                                resize: "vertical",
                                width: "100%",
                                value: description().clone(),
                                oninput: move |evt| {
                                    description.set(evt.value());
                                },
                            }
                        }

                        // Commentary
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Commentary"
                            }
                            textarea {
                                class: "neumo-inset input",
                                placeholder: "Notes, tips, or additional context about this recipe...",
                                rows: "4",
                                padding: "var(--space-sm) var(--space-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-primary)",
                                background_color: "var(--surface)",
                                outline: "none",
                                resize: "vertical",
                                width: "100%",
                                value: commentary().clone(),
                                oninput: move |evt| {
                                    commentary.set(evt.value());
                                },
                            }
                        }

                        // Time & servings row
                        div {
                            display: "grid",
                            grid_template_columns: "1fr 1fr 1fr",
                            gap: "var(--space-sm)",
                            div {
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-xs)",
                                label {
                                    font_size: "14px",
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    "Prep Time (min)"
                                }
                                Input {
                                    value: prep_time().clone(),
                                    placeholder: "0",
                                    input_type: "number",
                                    oninput: move |v| prep_time.set(v),
                                }
                            }
                            div {
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-xs)",
                                label {
                                    font_size: "14px",
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    "Cook Time (min)"
                                }
                                Input {
                                    value: cook_time().clone(),
                                    placeholder: "0",
                                    input_type: "number",
                                    oninput: move |v| cook_time.set(v),
                                }
                            }
                            div {
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-xs)",
                                label {
                                    font_size: "14px",
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    "Servings"
                                }
                                Input {
                                    value: servings().clone(),
                                    placeholder: "4",
                                    input_type: "number",
                                    oninput: move |v| servings.set(v),
                                }
                            }
                        }

                        // ── Ingredients ─────────────────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Ingredients *"
                            }

                            for (idx, _ing) in ingredients().iter().enumerate() {
                                div {
                                    key: "{idx}",
                                    display: "flex",
                                    gap: "var(--space-sm)",
                                    align_items: "center",
                                    // Amount
                                    div {
                                        style: "width: 70px; flex-shrink: 0;",
                                        Input {
                                            value: ingredients()[idx].amount.clone(),
                                            placeholder: "Amt",
                                            oninput: move |v| {
                                                ingredients.write()[idx].amount = v;
                                            },
                                        }
                                    }
                                    // Unit
                                    div {
                                        style: "width: 80px; flex-shrink: 0;",
                                        Input {
                                            value: ingredients()[idx].unit.clone(),
                                            placeholder: "Unit",
                                            oninput: move |v| {
                                                ingredients.write()[idx].unit = v;
                                            },
                                        }
                                    }
                                    // Name
                                    Input {
                                        value: ingredients()[idx].name.clone(),
                                        placeholder: "Ingredient name",
                                        oninput: move |v| {
                                            ingredients.write()[idx].name = v;
                                        },
                                    }
                                    // Remove
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
                                            ingredients.write().remove(idx);
                                        },
                                        "✕"
                                    }
                                }
                            }

                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: on_add_ingredient,
                                "+ Add Ingredient"
                            }
                        }

                        // ── Steps ───────────────────────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Steps *"
                            }

                            for (idx, _step) in steps().iter().enumerate() {
                                div {
                                    key: "{idx}",
                                    display: "flex",
                                    gap: "var(--space-sm)",
                                    align_items: "flex-start",
                                    // Indentation padding
                                    style: format!("padding-left: {}px", steps()[idx].depth * 24),
                                    // Step number
                                    span {
                                        font_weight: "600",
                                        color: "var(--text-secondary)",
                                        padding: "var(--space-sm) 0",
                                        min_width: "24px",
                                        "{idx + 1}."
                                    }
                                    div {
                                        display: "flex",
                                        flex_direction: "column",
                                        flex: "1",
                                        min_width: "0",
                                        gap: "var(--space-xs)",
                                        textarea {
                                            class: "neumo-inset input",
                                            placeholder: "Describe this step...",
                                            rows: "2",
                                            width: "100%",
                                            padding: "var(--space-sm) var(--space-md)",
                                            font_family: "var(--font-body)",
                                            font_size: "14px",
                                            color: "var(--text-primary)",
                                            background_color: "var(--surface)",
                                            outline: "none",
                                            resize: "vertical",
                                            value: steps()[idx].text.clone(),
                                            oninput: move |evt| {
                                                steps.write()[idx].text = evt.value();
                                            },
                                        }
                                        if !steps()[idx].photos.is_empty() {
                                            div {
                                                display: "flex",
                                                flex_wrap: "wrap",
                                                gap: "var(--space-xs)",
                                                margin_top: "var(--space-xs)",
                                                for photo_idx in 0..steps()[idx].photos.len() {
                                                    div {
                                                        key: "{idx}-{photo_idx}",
                                                        class: if dragged_step_photo() == Some((idx, photo_idx)) { "step-photo-item step-photo-item--dragging" } else { "step-photo-item" },
                                                        draggable: true,
                                                        ondragstart: move |_| dragged_step_photo.set(Some((idx, photo_idx))),
                                                        ondragend: move |_| dragged_step_photo.set(None),
                                                        ondragover: move |event| {
                                                            if dragged_step_photo().is_some() {
                                                                event.prevent_default();
                                                            }
                                                        },
                                                        ondrop: move |event| {
                                                            event.prevent_default();
                                                            if let Some((source_step_idx, source_photo_idx)) = dragged_step_photo() {
                                                                if source_step_idx == idx {
                                                                    move_item(
                                                                        &mut steps.write()[idx].photos,
                                                                        source_photo_idx,
                                                                        photo_idx,
                                                                    );
                                                                }
                                                            }
                                                            dragged_step_photo.set(None);
                                                        },
                                                        div { class: "step-photo-item__preview neumo-inset",
                                                            EditedRecipeImage {
                                                                image: steps()[idx].photos[photo_idx].clone(),
                                                                class: "image-upload__thumb step-photo-preview",
                                                                alt: "Step photo".to_string(),
                                                                prefer_thumbnail: true,
                                                                on_error: move |_| {},
                                                            }
                                                            div { class: "step-photo-item__meta",
                                                                span {
                                                                    class: "step-photo-item__name",
                                                                    title: step_photo_display_name(&steps()[idx].photos[photo_idx]),
                                                                    "{step_photo_display_name(&steps()[idx].photos[photo_idx])}"
                                                                }
                                                                span { class: "step-photo-item__size",
                                                                    if let Some(file_size) = steps()[idx].photos[photo_idx].file_size {
                                                                        "{format_step_photo_file_size(file_size)}"
                                                                    } else {
                                                                        "Size unavailable"
                                                                    }
                                                                }
                                                            }
                                                            div { class: "step-photo-item__controls",
                                                                button {
                                                                    r#type: "button",
                                                                    class: "btn btn-ghost touch-target",
                                                                    disabled: photo_idx == 0,
                                                                    aria_label: "Move step photo left",
                                                                    onclick: move |_| {
                                                                        steps.write()[idx].photos.swap(photo_idx, photo_idx - 1);
                                                                    },
                                                                    "←"
                                                                }
                                                                button {
                                                                    r#type: "button",
                                                                    class: "btn btn-ghost touch-target",
                                                                    disabled: photo_idx + 1 >= steps()[idx].photos.len(),
                                                                    aria_label: "Move step photo right",
                                                                    onclick: move |_| {
                                                                        steps.write()[idx].photos.swap(photo_idx, photo_idx + 1);
                                                                    },
                                                                    "→"
                                                                }
                                                                button {
                                                                    r#type: "button",
                                                                    class: "btn btn-ghost touch-target step-photo-item__remove",
                                                                    aria_label: "Remove step photo",
                                                                    onclick: move |_| {
                                                                        let removed = steps.write()[idx].photos.remove(photo_idx);
                                                                        pending_deletes.write().push(removed.url);
                                                                    },
                                                                    "✕"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // Reorder + indent + remove controls
                                    div {
                                        display: "flex",
                                        flex_direction: "column",
                                        gap: "var(--space-xs)",
                                        align_items: "center",
                                        // Step photo upload
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                trigger_file_input(&format!("#step-photo-{idx}"));
                                            },
                                            "📷"
                                        }
                                        input {
                                            id: format!("step-photo-{idx}"),
                                            r#type: "file",
                                            accept: "image/*",
                                            multiple: true,
                                            display: "none",
                                            onchange: move |evt| {
                                                on_step_photo_change(idx)(evt.files());
                                            },
                                        }
                                        // Move up
                                        if idx > 0 {
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| {
                                                    let mut s = steps.write();
                                                    s.swap(idx, idx - 1);
                                                },
                                                "↑"
                                            }
                                        }
                                        // Move down
                                        if idx < steps().len() - 1 {
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| {
                                                    let mut s = steps.write();
                                                    s.swap(idx, idx + 1);
                                                },
                                                "↓"
                                            }
                                        }
                                        // Indent (increase depth)
                                        if steps()[idx].depth < 3 {
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| {
                                                    steps.write()[idx].depth += 1;
                                                },
                                                "→"
                                            }
                                        }
                                        // Unindent (decrease depth)
                                        if steps()[idx].depth > 0 {
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| {
                                                    steps.write()[idx].depth -= 1;
                                                },
                                                "←"
                                            }
                                        }
                                        // Remove
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                for photo in steps()[idx].photos.clone() {
                                                    pending_deletes.write().push(photo.url);
                                                }
                                                steps.write().remove(idx);
                                            },
                                            "✕"
                                        }
                                    }
                                }
                            }

                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: on_add_step,
                                "+ Add Step"
                            }
                        }

                        // ── Tags ────────────────────────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Tags"
                            }
                            Input {
                                value: tags_input().clone(),
                                placeholder: "dinner, chicken, quick",
                                oninput: move |v| tags_input.set(v),
                            }
                            span {
                                font_size: "12px",
                                color: "var(--text-tertiary)",
                                "Comma-separated"
                            }
                        }

                        // ── Equipment (dynamic list) ────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Equipment"
                            }

                            for (idx, _eq) in equipment().iter().enumerate() {
                                div {
                                    key: "{idx}",
                                    display: "flex",
                                    gap: "var(--space-sm)",
                                    align_items: "center",
                                    Input {
                                        value: equipment()[idx].name.clone(),
                                        placeholder: "e.g. mixing bowl",
                                        oninput: move |v| {
                                            equipment.write()[idx].name = v;
                                        },
                                    }
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
                                            equipment.write().remove(idx);
                                        },
                                        "✕"
                                    }
                                }
                            }

                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: on_add_equipment,
                                "+ Add Equipment"
                            }
                        }

                        // ── Visibility ──────────────────────────────────────
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Visibility"
                            }
                            select {
                                class: "neumo-inset input",
                                padding: "var(--space-sm) var(--space-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-primary)",
                                background_color: "var(--surface)",
                                outline: "none",
                                border_radius: "var(--radius-md)",
                                width: "100%",
                                value: visibility().clone(),
                                onchange: move |evt| {
                                    visibility.set(evt.value());
                                },
                                option { value: "private", "Private — only you can see this recipe" }
                                option { value: "unlisted", "Unlisted — anyone with the link can view" }
                                option { value: "public", "Public — appears in Explore and search" }
                            }
                        }

                        // ── Error message ───────────────────────────────────
                        if let Some(err) = form_error() {
                            div {
                                padding: "var(--space-sm) var(--space-md)",
                                background_color: "var(--error-bg)",
                                border_radius: "var(--radius-md)",
                                color: "var(--error)",
                                font_size: "14px",
                                "{err}"
                            }
                        }

                        // ── Action buttons ──────────────────────────────────
                        div {
                            display: "flex",
                            gap: "var(--space-md)",
                            margin_top: "var(--space-md)",
                            Button {
                                variant: ButtonVariant::Primary,
                                disabled: is_saving() || busy() || step_photo_upload_count() > 0,
                                onclick: on_submit,
                                if step_photo_upload_count() > 0 || (is_saving() && busy()) {
                                    "Uploading images..."
                                } else if is_saving() {
                                    "Saving..."
                                } else {
                                    "Save Changes"
                                }
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                disabled: is_saving() || busy() || step_photo_upload_count() > 0,
                                onclick: on_cancel,
                                "Cancel"
                            }
                        }
                    }
                }
            }
            ImageEditorModal {
                image_url: step_editor_modal_url,
                on_save: move |edit: RecipeImageEdit| {
                    let current_url = step_editor_modal_url();
                    let current_idx = step_editor_idx();
                    let current_photo_idx = step_editor_photo_idx();
                    if let (Some(url), Some(idx)) = (current_url, current_idx) {
                        pending_step_photo_edits.write().retain(|pending| {
                            !(pending.idx == idx && pending.preview_url == url)
                        });
                        pending_step_photo_edits.write().push(PendingStepPhotoEdit {
                            idx,
                            preview_url: url.clone(),
                            edit: edit.clone(),
                        });
                        if let Some(step) = steps.write().get_mut(idx) {
                            if let Some(photo_idx) = current_photo_idx {
                                if let Some(photo) = step.photos.get_mut(photo_idx) {
                                    photo.edits = vec![edit];
                                }
                            }
                        }
                    }
                },
                on_close: move |_| {
                    step_editor_modal_url.set(None);
                    step_editor_idx.set(None);
                    step_editor_photo_idx.set(None);
                },
            }
        }
}
