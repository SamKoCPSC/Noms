//! Recipe creation form.
//!
//! Collects typed ingredients, steps (with depth for nesting), and equipment
//! in the UI, then passes them directly to `create_recipe()`.

use dioxus::prelude::*;

use crate::api::recipe::{
    create_recipe_draft, get_recipe_draft_api, publish_recipe_draft, save_recipe_draft,
};
#[allow(unused_imports)]
use crate::api::upload::upload_image;
use crate::components::base::{
    Button, ButtonVariant, EditedRecipeImage, ImageEditorModal, ImageUpload, Input, PageHeader,
    UploadOutcome,
};
use crate::components::AuthRequired;
use crate::pages::draft_save::draft_save_button_state;
use crate::pages::recipe_editor_form::{
    build_step_tree, can_indent_step, format_step_photo_file_size, step_photo_display_name,
    StepForm,
};
use crate::types::{
    RecipeEditorState, RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient, RecipeStep,
};
use crate::utils::reorder::move_item;

// ── Local form types ─────────────────────────────────────────────────────────

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

#[allow(clippy::too_many_arguments)]
fn editor_state(
    title: String,
    description: String,
    commentary: String,
    prep_time: String,
    cook_time: String,
    servings: String,
    ingredients: Vec<RecipeIngredient>,
    steps: Vec<StepForm>,
    equipment: Vec<RecipeEquipment>,
    images: Vec<RecipeImage>,
    visibility: String,
    tags_input: String,
) -> RecipeEditorState {
    RecipeEditorState {
        title: title.trim().to_string(),
        description: (!description.trim().is_empty()).then(|| description.trim().to_string()),
        commentary: (!commentary.trim().is_empty()).then(|| commentary.trim().to_string()),
        prep_time_minutes: prep_time.trim().parse().ok(),
        cook_time_minutes: cook_time.trim().parse().ok(),
        servings: servings.trim().parse().ok(),
        ingredients,
        instructions: build_step_tree(&steps),
        equipment,
        images,
        visibility,
        tags: tags_input
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect(),
    }
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
                    file_input.click();
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
    #[allow(unused_mut)]
    let mut is_manually_saving = use_signal(|| false);
    let mut error = use_signal(|| Option::<String>::None);
    #[allow(unused_mut)]
    let mut draft_id = use_signal(|| Option::<String>::None);
    #[allow(unused_mut, unused_variables)]
    let mut draft_revision = use_signal(|| 0i64);
    #[allow(unused_mut)]
    let mut last_saved_editor = use_signal(|| Option::<RecipeEditorState>::None);
    let mut autosave_generation = use_signal(|| 0u64);
    #[allow(unused_mut)]
    let mut draft_creation_in_flight = use_signal(|| false);

    // CP4: image upload state — shared with the `ImageUpload` component.
    // `images` is the sink for uploaded image records (read when calling
    // `create_recipe`); `upload_now` triggers the upload batch; `busy`
    // mirrors "processing or uploading" and disables Save; `pending_save`
    // stashes the validated form at Save-click time for the
    // `on_upload_complete` handler to consume after the batch finishes.
    let mut images = use_signal(Vec::<RecipeImage>::new);
    let mut upload_now = use_signal(|| false);
    let busy = use_signal(|| false);
    let pending_image_count = use_signal(|| 0usize);
    let mut pending_save = use_signal(|| Option::<PendingSaveData>::None);
    let mut step_editor_modal_url = use_signal(|| Option::<String>::None);
    let mut step_editor_idx = use_signal(|| Option::<usize>::None);
    let mut step_editor_photo_idx = use_signal(|| Option::<usize>::None);
    let mut pending_step_photo_edits = use_signal(Vec::<PendingStepPhotoEdit>::new);
    let mut step_photo_discard_capabilities =
        use_signal(std::collections::HashMap::<String, String>::new);
    let step_photo_upload_count = use_signal(|| 0usize);
    let mut dragged_step_photo = use_signal(|| Option::<(usize, usize)>::None);

    // Start a new mutable draft on the first meaningful edit, then autosave
    // subsequent changes after two idle seconds. No version is created here.
    use_effect(move || {
        let editor = editor_state(
            title(),
            description(),
            commentary(),
            prep_time(),
            cook_time(),
            servings(),
            ingredients(),
            steps(),
            equipment(),
            images(),
            visibility(),
            tags_input(),
        );
        let meaningful = !editor.title.is_empty()
            || editor.description.is_some()
            || editor.commentary.is_some()
            || !editor.ingredients.is_empty()
            || !editor.instructions.is_empty()
            || !editor.equipment.is_empty()
            || !editor.images.is_empty()
            || !editor.tags.is_empty();
        if !meaningful
            || is_saving()
            || is_manually_saving()
            || last_saved_editor().as_ref() == Some(&editor)
        {
            return;
        }

        // Do not subscribe the effect to its own generation counter.
        let generation = *autosave_generation.peek() + 1;
        autosave_generation.set(generation);
        #[allow(unused_variables)]
        let current_draft_id = draft_id();
        #[allow(unused_variables)]
        let expected_revision = draft_revision();
        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(2_000).await;
            if autosave_generation() != generation || is_saving() || is_manually_saving() {
                return;
            }
            let result = if let Some(current_draft_id) = current_draft_id {
                let first_save =
                    save_recipe_draft(current_draft_id.clone(), expected_revision, editor.clone())
                        .await;
                match first_save {
                    Ok(saved) => Ok(saved),
                    Err(_) if autosave_generation() == generation => {
                        match get_recipe_draft_api(current_draft_id.clone()).await {
                            Ok(current) => {
                                save_recipe_draft(
                                    current_draft_id,
                                    current.revision,
                                    editor.clone(),
                                )
                                .await
                            }
                            Err(error) => Err(error),
                        }
                    }
                    Err(error) => Err(error),
                }
            } else if *draft_creation_in_flight.peek() {
                return;
            } else {
                draft_creation_in_flight.set(true);
                let created = create_recipe_draft(editor.clone()).await;
                draft_creation_in_flight.set(false);
                created
            };
            match result {
                Ok(saved) => {
                    if draft_id().is_none() {
                        let saved_id = saved.id.to_string();
                        draft_id.set(Some(saved_id.clone()));
                        if let Some(window) = web_sys::window() {
                            if let Ok(history) = window.history() {
                                let _ = history.replace_state_with_url(
                                    &wasm_bindgen::JsValue::NULL,
                                    "",
                                    Some(&format!("/recipes/drafts/{saved_id}/edit")),
                                );
                            }
                        }
                    }
                    draft_revision.set(saved.revision);
                    if autosave_generation() == generation {
                        last_saved_editor.set(Some(editor));
                    }
                }
                Err(save_error) if autosave_generation() == generation => {
                    error.set(Some(format!("Autosave failed: {save_error}")));
                }
                Err(_) => {}
            }
        });
    });

    #[allow(unused_variables)]
    let on_save_draft = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let editor = editor_state(
                title(),
                description(),
                commentary(),
                prep_time(),
                cook_time(),
                servings(),
                ingredients(),
                steps(),
                equipment(),
                images(),
                visibility(),
                tags_input(),
            );
            let meaningful = !editor.title.is_empty()
                || editor.description.is_some()
                || editor.commentary.is_some()
                || !editor.ingredients.is_empty()
                || !editor.instructions.is_empty()
                || !editor.equipment.is_empty()
                || !editor.images.is_empty()
                || !editor.tags.is_empty();
            if !meaningful || is_saving() || is_manually_saving() {
                return;
            }

            error.set(None);
            is_manually_saving.set(true);
            autosave_generation.set(autosave_generation() + 1);
            let current_draft_id = draft_id();
            let expected_revision = draft_revision();
            spawn(async move {
                let save_result = if let Some(current_draft_id) = current_draft_id {
                    let first_save = save_recipe_draft(
                        current_draft_id.clone(),
                        expected_revision,
                        editor.clone(),
                    )
                    .await;
                    match first_save {
                        Ok(saved) => Ok(saved),
                        Err(_) => match get_recipe_draft_api(current_draft_id.clone()).await {
                            Ok(current) => {
                                save_recipe_draft(
                                    current_draft_id,
                                    current.revision,
                                    editor.clone(),
                                )
                                .await
                            }
                            Err(error) => Err(error),
                        },
                    }
                } else if *draft_creation_in_flight.peek() {
                    Err(ServerFnError::new(
                        "Draft creation is already finishing. Try again in a moment.",
                    ))
                } else {
                    draft_creation_in_flight.set(true);
                    let created = create_recipe_draft(editor.clone()).await;
                    draft_creation_in_flight.set(false);
                    created
                };

                match save_result {
                    Ok(saved) => {
                        if draft_id().is_none() {
                            let saved_id = saved.id.to_string();
                            draft_id.set(Some(saved_id.clone()));
                            if let Some(window) = web_sys::window() {
                                if let Ok(history) = window.history() {
                                    let _ = history.replace_state_with_url(
                                        &wasm_bindgen::JsValue::NULL,
                                        "",
                                        Some(&format!("/recipes/drafts/{saved_id}/edit")),
                                    );
                                }
                            }
                        }
                        draft_revision.set(saved.revision);
                        last_saved_editor.set(Some(editor));
                    }
                    Err(save_error) => {
                        error.set(Some(format!("Save draft failed: {save_error}")));
                    }
                }
                is_manually_saving.set(false);
            });
        }
    };

    // ── Step photo upload handler ────────────────────────────────────────
    // Created per-step via closure capturing the step index
    #[allow(unused_variables)]
    let on_step_photo_change = move |idx: usize| {
        move |files: Vec<dioxus::html::FileData>| {
            #[cfg(target_arch = "wasm32")]
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
                let mut step_photo_discard_capabilities = step_photo_discard_capabilities;
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
                            crate::api::upload::UploadPurpose::Recipe,
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
                            .unwrap_or_else(RecipeImageEdit::default_rect_16x9);
                        if let Some(url) = &preview_url {
                            pending_step_photo_edits.write().retain(|pending| {
                                !(pending.idx == idx && pending.preview_url == *url)
                            });
                        }

                        let full_url = resp.full_url;
                        step_photo_discard_capabilities
                            .write()
                            .insert(full_url.clone(), resp.discard_capability);
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
            if *draft_creation_in_flight.peek() {
                error.set(Some(
                    "The first autosave is still finishing. Publish again in a moment.".to_string(),
                ));
                is_saving.set(false);
                return;
            }
            autosave_generation.set(autosave_generation() + 1);
            let editor = RecipeEditorState {
                title: data.title,
                description: data.description,
                commentary: data.commentary,
                prep_time_minutes: data.prep,
                cook_time_minutes: data.cook,
                servings: data.serv,
                ingredients: data.ingredients,
                instructions: data.instructions,
                equipment: data.equipment,
                images: images(),
                visibility: data.visibility,
                tags: data.tags,
            };
            let publish_result = async {
                let saved = if let Some(current_draft_id) = draft_id() {
                    let current = get_recipe_draft_api(current_draft_id.clone()).await?;
                    let first_save = save_recipe_draft(
                        current_draft_id.clone(),
                        current.revision,
                        editor.clone(),
                    )
                    .await;
                    match first_save {
                        Ok(saved) => saved,
                        Err(_) => {
                            let refreshed = get_recipe_draft_api(current_draft_id.clone()).await?;
                            save_recipe_draft(current_draft_id, refreshed.revision, editor).await?
                        }
                    }
                } else {
                    create_recipe_draft(editor).await?
                };
                publish_recipe_draft(saved.id.to_string(), None).await
            }
            .await;
            match publish_result {
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

    let current_editor = editor_state(
        title(),
        description(),
        commentary(),
        prep_time(),
        cook_time(),
        servings(),
        ingredients(),
        steps(),
        equipment(),
        images(),
        visibility(),
        tags_input(),
    );
    let has_meaningful_draft_content = !current_editor.title.is_empty()
        || current_editor.description.is_some()
        || current_editor.commentary.is_some()
        || !current_editor.ingredients.is_empty()
        || !current_editor.instructions.is_empty()
        || !current_editor.equipment.is_empty()
        || !current_editor.images.is_empty()
        || !current_editor.tags.is_empty();
    let draft_save_state = draft_save_button_state(
        is_manually_saving(),
        has_meaningful_draft_content && last_saved_editor().as_ref() != Some(&current_editor),
    );

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

    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "New Recipe",
                }
                div { class: "recipe-detail__tags",
                    span { class: "recipe-detail__tag", "DRAFT" }
                    span {
                        if draft_id().is_some() {
                            "Autosaved privately"
                        } else {
                            "Autosaves after your first edit"
                        }
                    }
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
                            pending_count: pending_image_count,
                            purpose: crate::api::upload::UploadPurpose::Recipe,
                            on_remove_existing: move |url: String| {
                                images.write().retain(|image| image.url != url);
                            },
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
                                // Keep photos below the text instead of letting
                                // them compete with the textarea for row width.
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
                                                                "{format_step_photo_file_size(steps()[idx].photos[photo_idx].file_size)}"
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
                                                                if let Some(capability) = step_photo_discard_capabilities.write().remove(&removed.url) {
                                                                    spawn(async move {
                                                                        if let Err(problem) = crate::api::upload::discard_upload(capability).await {
                                                                            tracing::warn!("Failed to discard unsaved step photo: {problem}");
                                                                        }
                                                                    });
                                                                }
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
                                    if can_indent_step(steps()[idx].depth) {
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
                                                if let Some(capability) = step_photo_discard_capabilities.write().remove(&photo.url) {
                                                    spawn(async move {
                                                        if let Err(problem) = crate::api::upload::discard_upload(capability).await {
                                                            tracing::warn!("Failed to discard unsaved step photo: {problem}");
                                                        }
                                                    });
                                                }
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
                            variant: ButtonVariant::Secondary,
                            disabled: !draft_save_state.is_actionable()
                                || is_saving()
                                || busy()
                                || step_photo_upload_count() > 0,
                            onclick: on_save_draft,
                            "{draft_save_state.label()}"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: is_saving() || is_manually_saving() || busy() || step_photo_upload_count() > 0,
                            onclick: on_submit,
                            if step_photo_upload_count() > 0 {
                                "Uploading images..."
                            } else if is_saving() {
                                "Publishing..."
                            } else {
                                "Publish Recipe"
                            }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            disabled: is_saving() || is_manually_saving() || busy() || step_photo_upload_count() > 0,
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
}
