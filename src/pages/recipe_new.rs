//! Recipe creation form.
//!
//! Collects typed ingredients, steps (with depth for nesting), and equipment
//! in the UI, then passes them directly to `create_recipe()`.

use dioxus::prelude::*;

use crate::api::recipe::create_recipe;
#[allow(unused_imports)]
use crate::api::upload::upload_image;
use crate::components::base::{
    Button, ButtonVariant, EditedRecipeImage, ImageEditorModal, ImageUpload, Input, PageHeader,
    UploadOutcome,
};
use crate::components::AuthRequired;
use crate::types::{RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient, RecipeStep};

// ── Local form types ─────────────────────────────────────────────────────────

/// A single step row in the flat form list.
/// `depth` controls nesting level (0 = top-level, 1 = sub-step, etc.).
#[derive(Clone, Debug)]
struct StepForm {
    text: String,
    depth: usize,
    photo_url: Option<String>,
    photo: Option<RecipeImage>,
}

impl StepForm {
    fn display_photo(&self) -> Option<RecipeImage> {
        self.photo.clone().or_else(|| {
            self.photo_url.as_ref().map(|url| RecipeImage {
                url: url.clone(),
                thumb_url: None,
                edits: Vec::new(),
            })
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PendingStepPhotoEdit {
    idx: usize,
    preview_url: String,
    edit: RecipeImageEdit,
}

/// Validated form fields captured at Save-click time, consumed by the
/// `on_upload_complete` handler after the image upload batch finishes.
///
/// CP4: the save flow is now two-phase — (1) the user clicks Save, we stash
/// the validated form here and trigger the image upload batch via
/// `upload_now`; (2) `ImageUpload` fires `on_upload_complete`, which
/// consumes this stash and calls `create_recipe` with the uploaded URLs.
#[derive(Clone)]
struct PendingSaveData {
    title: String,
    description: Option<String>,
    commentary: Option<String>,
    prep: Option<i32>,
    cook: Option<i32>,
    serv: Option<i32>,
    ingredients: Vec<RecipeIngredient>,
    instructions: Vec<RecipeStep>,
    equipment: Vec<RecipeEquipment>,
    tags: Vec<String>,
    visibility: String,
}

// ── Tree builder ─────────────────────────────────────────────────────────────

/// Convert a flat list of `StepForm` entries into a tree of `RecipeStep`s.
///
/// Uses a stack-based algorithm: each entry's `depth` determines where it
/// attaches in the tree relative to previously seen entries.
fn build_step_tree(steps: &[StepForm]) -> Vec<RecipeStep> {
    if steps.is_empty() {
        return Vec::new();
    }

    // Stack holds (depth, mutable reference into the current sub_steps Vec)
    // We use a Vec of owned RecipeStep nodes and track parent indices.
    let mut nodes: Vec<RecipeStep> = Vec::with_capacity(steps.len());
    // Stack of (depth, node_index) — the node at that index is the current
    // parent for children at depth+1.
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for step in steps {
        let node = RecipeStep {
            text: step.text.clone(),
            sub_steps: Vec::new(),
            photo_url: step.photo_url.clone(),
            photo: step.photo.clone(),
        };
        let idx = nodes.len();
        nodes.push(node);

        // Pop stack entries that are at >= current depth (they're siblings or
        // ancestors that we've finished adding children to)
        while let Some(&(d, _)) = stack.last() {
            if d >= step.depth {
                stack.pop();
            } else {
                break;
            }
        }

        if let Some(&(_, parent_idx)) = stack.last() {
            // Attach as child of the top of stack
            // Remove first (idx is always the last element) to avoid double mutable borrow
            let node = nodes.remove(idx);
            nodes[parent_idx].sub_steps.push(node);
        }
        // else: depth 0 or stack is empty → stays as root-level node

        stack.push((step.depth, idx));
    }

    nodes
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

/// Create recipe page — full form with ingredients, steps, and tags.
#[component]
pub fn RecipeNew() -> Element {
    // Basic fields
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut commentary = use_signal(String::new);
    let mut prep_time = use_signal(String::new);
    let mut cook_time = use_signal(String::new);
    let mut servings = use_signal(String::new);

    // Dynamic lists — typed directly
    let mut ingredients = use_signal(Vec::<RecipeIngredient>::new);
    let mut steps = use_signal(Vec::<StepForm>::new);
    let mut equipment = use_signal(Vec::<RecipeEquipment>::new);

    // Tags (comma-separated)
    let mut tags_input = use_signal(String::new);

    // Visibility (default: private)
    let mut visibility = use_signal(|| "private".to_string());

    // UI state
    let mut is_saving = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);

    // CP4: image upload state — shared with the `ImageUpload` component.
    // `images` is the sink for uploaded image records (read when calling
    // `create_recipe`); `upload_now` triggers the upload batch; `busy`
    // mirrors "processing or uploading" and disables Save; `pending_save`
    // stashes the validated form at Save-click time for the
    // `on_upload_complete` handler to consume after the batch finishes.
    let images = use_signal(Vec::<RecipeImage>::new);
    let mut upload_now = use_signal(|| false);
    let busy = use_signal(|| false);
    let mut pending_save = use_signal(|| Option::<PendingSaveData>::None);
    let mut step_editor_modal_url = use_signal(|| Option::<String>::None);
    let mut step_editor_idx = use_signal(|| Option::<usize>::None);
    let mut pending_step_photo_edits = use_signal(Vec::<PendingStepPhotoEdit>::new);
    let step_photo_upload_count = use_signal(|| 0usize);

    // ── Step photo upload handler ────────────────────────────────────────
    // Created per-step via closure capturing the step index
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
                let mut pending_step_photo_edits = pending_step_photo_edits;
                let mut step_photo_upload_count = step_photo_upload_count;
                spawn(async move {
                    use dioxus_web::WebFileExt;

                    let file_data = &files[0];
                    let web_file = match file_data.get_web_file() {
                        Some(f) => f,
                        None => return,
                    };
                    step_photo_upload_count.set(step_photo_upload_count().saturating_add(1));
                    let preview_url = web_sys::Url::create_object_url_with_blob(&web_file).ok();
                    if let Some(url) = &preview_url {
                        step_editor_idx.set(Some(idx));
                        step_editor_modal_url.set(Some(url.clone()));
                    }

                    let processed = match crate::utils::image_processing_client::process_image(
                        web_file,
                    )
                    .await
                    {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!("Step photo processing failed: {e}");
                            step_photo_upload_count
                                .set(step_photo_upload_count().saturating_sub(1));
                            return;
                        }
                    };

                    let resp = match upload_image(
                        processed.full_bytes,
                        processed.thumb_bytes,
                        format!("step_{idx}.webp"),
                    )
                    .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("Step photo upload failed: {e}");
                            step_photo_upload_count
                                .set(step_photo_upload_count().saturating_sub(1));
                            return;
                        }
                    };

                    let edit = preview_url
                        .as_ref()
                        .and_then(|url| {
                            pending_step_photo_edits()
                                .iter()
                                .rev()
                                .find(|pending| pending.idx == idx && pending.preview_url == *url)
                                .map(|pending| pending.edit.clone())
                        })
                        .unwrap_or_else(RecipeImageEdit::default_16x9);
                    if let Some(url) = &preview_url {
                        pending_step_photo_edits.write().retain(|pending| {
                            !(pending.idx == idx && pending.preview_url == *url)
                        });
                    }

                    let full_url = resp.full_url;
                    let photo = RecipeImage::new(full_url.clone(), Some(resp.thumb_url), edit);
                    if let Some(step) = steps.write().get_mut(idx) {
                        step.photo_url = Some(full_url);
                        step.photo = Some(photo);
                    }
                    step_photo_upload_count.set(step_photo_upload_count().saturating_sub(1));
                });
            }
        }
    };

    // ── Submit handler ───────────────────────────────────────────────────
    let on_submit = move |_| {
        // Clear previous errors
        error.set(None);

        // ── Client-side validation ──────────────────────────────────────
        let trimmed_title = title().trim().to_string();
        if trimmed_title.is_empty() {
            error.set(Some("Title is required".to_string()));
            return;
        }

        // Validate at least one ingredient with a non-empty name
        let ings = ingredients();
        if ings.is_empty() {
            error.set(Some("Add at least one ingredient".to_string()));
            return;
        }
        if ings.iter().all(|i| i.name.trim().is_empty()) {
            error.set(Some("At least one ingredient must have a name".to_string()));
            return;
        }

        // Validate at least one step with non-empty text
        let sts = steps();
        if sts.is_empty() {
            error.set(Some("Add at least one step".to_string()));
            return;
        }
        if sts.iter().all(|s| s.text.trim().is_empty()) {
            error.set(Some("At least one step must have text".to_string()));
            return;
        }

        // ── Parse optional numeric fields ───────────────────────────────
        let prep = prep_time().trim().parse::<i32>().ok();
        let cook = cook_time().trim().parse::<i32>().ok();
        let serv = servings().trim().parse::<i32>().ok();

        // ── Parse tags ──────────────────────────────────────────────────
        let tags: Vec<String> = tags_input()
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        // ── Convert flat steps to tree ──────────────────────────────────
        let instructions = build_step_tree(&sts);

        // ── Early guard: images still processing ────────────────────────
        // Belt-and-suspenders — `ImageUpload::upload_all()` also checks
        // this and aborts. This gives immediate feedback.
        if busy() {
            error.set(Some(
                "Images are still processing — please wait a moment".to_string(),
            ));
            return;
        }

        // ── Commit values and stash for the post-upload handler ─────────
        is_saving.set(true);

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
        let equip = equipment();

        // Stash the validated form data. The `on_upload_complete` handler
        // (fired by `ImageUpload` after the upload batch) consumes this and
        // calls `create_recipe` with `images()`.
        pending_save.set(Some(PendingSaveData {
            title: trimmed_title,
            description: desc,
            commentary: comm,
            prep,
            cook,
            serv,
            ingredients: ings,
            instructions,
            equipment: equip,
            tags,
            visibility: vis,
        }));

        // Trigger the upload batch. If there are no pending images,
        // `upload_all` returns immediately with `succeeded: 0, failed: 0,
        // aborted: None` and the handler proceeds to `create_recipe` with
        // `images() == vec![]` — recipes with zero images work.
        upload_now.set(true);
    };

    // ── on_upload_complete handler ───────────────────────────────────────
    // Fired by `ImageUpload` when the upload batch finishes. Consumes the
    // stashed form data and calls `create_recipe` with the uploaded URLs.
    let on_upload_complete = move |outcome: UploadOutcome| {
        spawn(async move {
            // S5: Dioxus `Signal` requires `.write()` for mutation (returns
            // `&mut T`; `Option::take` needs `&mut self`). Matches the
            // codebase's `signal.write().push(...)` pattern.
            let Some(data) = pending_save.write().take() else {
                return;
            };

            if let Some(msg) = outcome.aborted {
                error.set(Some(msg));
                is_saving.set(false);
                return;
            }

            if outcome.failed > 0 {
                // Some uploads failed — do NOT save. User retries/removes
                // via the component UI, then clicks Save again.
                error.set(Some(format!(
                    "{} image(s) failed to upload. Use the Retry button on each failed image (or remove them), then click Save again.",
                    outcome.failed
                )));
                is_saving.set(false);
                return;
            }

            error.set(None);
            match create_recipe(
                data.title,
                data.description,
                data.commentary,
                data.prep,
                data.cook,
                data.serv,
                data.ingredients,
                data.instructions,
                data.equipment,
                images(), // uploaded full and thumbnail URLs
                data.tags,
                data.visibility,
            )
            .await
            {
                #[allow(unused_variables)]
                Ok(recipe) => {
                    // Navigate to recipe detail page
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            let _ = window
                                .location()
                                .set_href(&format!("/recipes/{}", recipe.id));
                        }
                    }
                }
                Err(e) => {
                    let msg = match &e {
                        dioxus::prelude::ServerFnError::ServerError { message, .. } => {
                            message.clone()
                        }
                        _ => e.to_string(),
                    };
                    error.set(Some(msg));
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
                let _ = window.location().set_href("/dashboard");
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
            photo_url: None,
            photo: None,
        });
    };

    // ── Equipment helpers ───────────────────────────────────────────────
    let on_add_equipment = move |_| {
        equipment.write().push(RecipeEquipment {
            name: String::new(),
        });
    };

    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "New Recipe",
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
                                error.set(None);
                            },
                        }
                    }

                    // ── Recipe Images ───────────────────────────────────────
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
                        ImageUpload {
                            images,
                            upload_now,
                            busy,
                            on_upload_complete,
                        }
                        span {
                            font_size: "12px",
                            color: "var(--text-tertiary)",
                            "Images are processed and uploaded when you save"
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
                                // Indentation offset based on depth
                                padding_left: format!("{}px", steps()[idx].depth * 24),

                                // Step number
                                span {
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    padding: "var(--space-sm) 0",
                                    min_width: "24px",
                                    "{idx + 1}."
                                }
                                // Step text
                                textarea {
                                    class: "neumo-inset input",
                                    placeholder: "Describe this step...",
                                    rows: "2",
                                    style: "flex: 1;",
                                    padding: "var(--space-sm) var(--space-md)",
                                    font_family: "var(--font-body)",
                                    font_size: "14px",
                                    color: "var(--text-primary)",
                                    background_color: "var(--surface)",
                                    outline: "none",
                                    resize: "vertical",
                                    oninput: move |evt| {
                                        steps.write()[idx].text = evt.value();
                                    },
                                }
                                // Step photo thumbnail
                                if let Some(photo) = steps()[idx].display_photo() {
                                    div {
                                        position: "relative",
                                        display: "inline-block",
                                        margin_top: "var(--space-xs)",
                                        EditedRecipeImage {
                                            image: photo,
                                            class: "image-upload__thumb step-photo-preview",
                                            alt: "Step photo".to_string(),
                                            prefer_thumbnail: true,
                                            on_error: move |_| {},
                                        }
                                        button {
                                            onclick: move |_| {
                                                steps.write()[idx].photo_url = None;
                                                steps.write()[idx].photo = None;
                                            },
                                            style: "position: absolute; top: -6px; right: -6px; font-size: 12px; width: 20px; height: 20px; padding: 0; line-height: 1; background: none; border: none; cursor: pointer; color: var(--text-secondary);",
                                            "✕"
                                        }
                                    }
                                }
                                // Controls: photo + indent/unindent + reorder + remove
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
                                        display: "none",
                                        onchange: move |evt| {
                                            on_step_photo_change(idx)(evt.files());
                                        },
                                    }
                                    // Indent
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
                                            steps.write()[idx].depth += 1;
                                        },
                                        "→"
                                    }
                                    // Unindent
                                    if steps()[idx].depth > 0 {
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                steps.write()[idx].depth -= 1;
                                            },
                                            "←"
                                        }
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
                                    // Remove
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
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

                    // ── Equipment ───────────────────────────────────────
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
                    if let Some(err) = error() {
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
                            if step_photo_upload_count() > 0 {
                                "Uploading images..."
                            } else if is_saving() {
                                "Saving..."
                            } else {
                                "Save Recipe"
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
            ImageEditorModal {
                image_url: step_editor_modal_url,
                on_save: move |edit: RecipeImageEdit| {
                    let current_url = step_editor_modal_url();
                    let current_idx = step_editor_idx();
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
                            if let Some(photo) = step.photo.as_mut() {
                                photo.edits = vec![edit];
                            }
                        }
                    }
                },
                on_close: move |_| {
                    step_editor_modal_url.set(None);
                    step_editor_idx.set(None);
                },
            }
        }
    }
}
