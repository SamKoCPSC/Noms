//! Recipe creation form.
//!
//! Collects typed ingredients, steps (with depth for nesting), and equipment
//! in the UI, then passes them directly to `create_recipe()`.

use crate::components::base::PageIconKind;
use dioxus::prelude::*;

use crate::api::recipe::{
    create_recipe_draft, get_recipe_draft_api, publish_recipe_draft, save_recipe_draft,
};
#[allow(unused_imports)]
use crate::api::upload::upload_image;
use crate::components::base::{
    Button, ButtonVariant, ImageEditorModal, ImageUpload, PageHeader, UploadOutcome,
};
use crate::components::AuthRequired;
use crate::pages::draft_save::{
    draft_save_button_state, recipe_write_feedback, RecipeWriteAction,
    UNCONFIRMED_DRAFT_CREATION_MESSAGE, UNCONFIRMED_PUBLICATION_MESSAGE,
};
use crate::pages::recipe_editor_form::{
    apply_step_photo_edit, build_step_tree, can_indent_step, format_step_photo_file_size,
    reorder_step_photo_within_step, reset_step_photo_input, step_photo_display_name,
    step_photo_editor_edit, RecipeEditorContents, RecipeEditorInput as Input, StepForm,
    StepPhotoEditButton, StepPhotoPreview, StepPhotoUploadFeedback,
};
use crate::types::{
    RecipeEditorState, RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient, RecipeStep,
};

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
    crate::components::ingredient_vocabulary::use_editor_vocabulary();
    let form_account = crate::auth::account_binding::use_form_account();
    let navigator = use_navigator();
    // Basic fields
    let mut title = use_signal(String::new);
    let mut active_editor_section = use_signal(|| "basics");
    let mut publication_review_open = use_signal(|| false);
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
    // After an uncertain create, keep local entries but require recovery from
    // Dashboard: another create could duplicate an already committed draft.
    #[allow(unused_mut)]
    let mut draft_creation_unconfirmed = use_signal(|| false);
    let mut publication_unconfirmed = use_signal(|| false);

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
    let mut gallery_upload_error = use_signal(|| Option::<String>::None);
    // Clear only the gallery upload warning after all pending items are
    // resolved. A successful retry can autosave the draft without publication;
    // unrelated write/account errors must keep their own recovery feedback.
    use_effect(move || {
        if pending_image_count() == 0 && gallery_upload_error().is_some() {
            gallery_upload_error.set(None);
        }
    });
    let mut pending_save = use_signal(|| Option::<PendingSaveData>::None);
    let mut step_editor_modal_url = use_signal(|| Option::<String>::None);
    let mut step_editor_idx = use_signal(|| Option::<usize>::None);
    let mut step_editor_photo_idx = use_signal(|| Option::<usize>::None);
    let mut step_editor_initial_edit = use_signal(|| Option::<RecipeImageEdit>::None);
    let mut pending_step_photo_edits = use_signal(Vec::<PendingStepPhotoEdit>::new);
    let mut step_photo_discard_capabilities =
        use_signal(std::collections::HashMap::<String, String>::new);
    let step_photo_upload_count = use_signal(|| 0usize);
    let mut step_photo_previews = use_signal(Vec::<StepPhotoPreview>::new);
    let step_photo_upload_errors = use_signal(Vec::<String>::new);
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
        if editor
            .ingredients
            .iter()
            .flat_map(|row| std::iter::once(row).chain(row.substitutes.iter()))
            .any(|row| noms::units::canonicalize(&row.unit).is_err())
        {
            let next_generation = *autosave_generation.peek() + 1;
            autosave_generation.set(next_generation);
            return;
        }
        if !meaningful
            || draft_creation_unconfirmed()
            || publication_unconfirmed()
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
            if autosave_generation() != generation
                || draft_creation_unconfirmed()
                || publication_unconfirmed()
                || is_saving()
                || is_manually_saving()
            {
                return;
            }
            let result = if let Some(current_draft_id) = current_draft_id {
                let first_save = save_recipe_draft(
                    form_account(),
                    current_draft_id.clone(),
                    expected_revision,
                    editor.clone(),
                )
                .await;
                match first_save {
                    Ok(saved) => Ok(saved),
                    Err(_) if autosave_generation() == generation => {
                        match get_recipe_draft_api(form_account(), current_draft_id.clone()).await {
                            Ok(current) => {
                                save_recipe_draft(
                                    form_account(),
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
                let created = create_recipe_draft(form_account(), editor.clone()).await;
                if created.as_ref().err().is_some_and(|problem| {
                    crate::auth::account_binding::account_error_feedback(problem).is_none()
                }) {
                    draft_creation_unconfirmed.set(true);
                }
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
                    let action = if draft_creation_unconfirmed() {
                        RecipeWriteAction::CreateDraft
                    } else {
                        RecipeWriteAction::Autosave
                    };
                    error.set(Some(recipe_write_feedback(action, &save_error).to_string()));
                }
                Err(_) => {}
            }
        });
    });

    #[allow(unused_variables)]
    let on_save_draft = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if ingredients()
                .iter()
                .flat_map(|row| std::iter::once(row).chain(row.substitutes.iter()))
                .any(|row| noms::units::canonicalize(&row.unit).is_err())
            {
                error.set(Some(noms::units::INVALID_UNIT.into()));
                return;
            }
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
                || draft_creation_unconfirmed()
                || publication_unconfirmed()
                || is_saving()
                || is_manually_saving()
                || busy()
                || step_photo_upload_count() > 0
            {
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
                        form_account(),
                        current_draft_id.clone(),
                        expected_revision,
                        editor.clone(),
                    )
                    .await;
                    match first_save {
                        Ok(saved) => Ok(saved),
                        Err(_) => {
                            match get_recipe_draft_api(form_account(), current_draft_id.clone())
                                .await
                            {
                                Ok(current) => {
                                    save_recipe_draft(
                                        form_account(),
                                        current_draft_id,
                                        current.revision,
                                        editor.clone(),
                                    )
                                    .await
                                }
                                Err(error) => Err(error),
                            }
                        }
                    }
                } else if *draft_creation_in_flight.peek() {
                    Err(ServerFnError::new(
                        "Draft creation is already finishing. Try again in a moment.",
                    ))
                } else {
                    draft_creation_in_flight.set(true);
                    let created = create_recipe_draft(form_account(), editor.clone()).await;
                    if created.as_ref().err().is_some_and(|problem| {
                        crate::auth::account_binding::account_error_feedback(problem).is_none()
                    }) {
                        draft_creation_unconfirmed.set(true);
                    }
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
                        let action = if draft_creation_unconfirmed() {
                            RecipeWriteAction::CreateDraft
                        } else {
                            RecipeWriteAction::SaveDraft
                        };
                        error.set(Some(recipe_write_feedback(action, &save_error).to_string()));
                    }
                }
                is_manually_saving.set(false);
            });
        }
    };

    // ── Step photo upload handler ────────────────────────────────────────
    // Created per-step via closure capturing the step index
    #[allow(unused_variables)]
    let step_photo_controls_locked = move || {
        draft_creation_unconfirmed()
            || publication_unconfirmed()
            || is_saving()
            || is_manually_saving()
            || busy()
            || step_photo_upload_count() > 0
    };
    let on_step_photo_change = move |idx: usize| {
        move |files: Vec<dioxus::html::FileData>| {
            reset_step_photo_input(idx);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = files;
            #[cfg(target_arch = "wasm32")]
            {
                if files.is_empty() || step_photo_controls_locked() {
                    return;
                }
                let mut steps = steps;
                let mut step_editor_modal_url = step_editor_modal_url;
                let mut step_editor_idx = step_editor_idx;
                let mut step_editor_photo_idx = step_editor_photo_idx;
                let mut step_editor_initial_edit = step_editor_initial_edit;
                let mut pending_step_photo_edits = pending_step_photo_edits;
                let mut step_photo_upload_count = step_photo_upload_count;
                let mut step_photo_discard_capabilities = step_photo_discard_capabilities;
                let mut step_photo_upload_errors = step_photo_upload_errors;
                let mut step_photo_previews = step_photo_previews;
                // Claim the entire batch synchronously. Reordering/removal must
                // not change its destination index between asynchronous files.
                step_photo_upload_count.set(1);
                step_photo_upload_errors.write().clear();
                dragged_step_photo.set(None);
                // A batch must not replace a still-unconfirmed editor with the
                // next file. Batch previews can be edited after they are added.
                let open_preview = files.len() == 1;
                spawn(async move {
                    use dioxus_web::WebFileExt;
                    let mut release_failed_preview = move |preview_url: &Option<String>| {
                        if let Some(url) = preview_url {
                            if step_editor_modal_url().as_ref() == Some(url) {
                                step_editor_modal_url.set(None);
                                step_editor_idx.set(None);
                                step_editor_photo_idx.set(None);
                                step_editor_initial_edit.set(None);
                            }
                            pending_step_photo_edits
                                .write()
                                .retain(|pending| pending.preview_url != *url);
                            step_photo_previews
                                .write()
                                .retain(|preview| preview.0 != *url);
                        }
                    };
                    for (file_idx, file_data) in files.iter().enumerate() {
                        let photo_file_name = file_data.name();
                        if let Err(reason) = crate::components::base::validate_image_input(
                            &photo_file_name,
                            file_data.size(),
                        ) {
                            step_photo_upload_errors.write().push(format!(
                                "Skipped \"{}\": {reason}",
                                photo_file_name.chars().take(120).collect::<String>()
                            ));
                            continue;
                        }
                        let web_file = match file_data.get_web_file() {
                            Some(file) => file,
                            None => {
                                step_photo_upload_errors.write().push(
                                    crate::pages::recipe_editor_form::step_photo_processing_failure(
                                        &photo_file_name,
                                    ),
                                );
                                continue;
                            }
                        };
                        let photo_file_size = web_file.size() as u64;
                        let preview_url = if open_preview {
                            web_sys::Url::create_object_url_with_blob(&web_file).ok()
                        } else {
                            None
                        };
                        if let Some(url) = &preview_url {
                            step_photo_previews
                                .write()
                                .push(StepPhotoPreview(url.clone()));
                            step_editor_idx.set(Some(idx));
                            step_editor_photo_idx.set(None);
                            step_editor_initial_edit
                                .set(Some(RecipeImageEdit::default_rect_16x9()));
                            step_editor_modal_url.set(Some(url.clone()));
                        }
                        let processed =
                            match crate::utils::image_processing_client::process_image(web_file)
                                .await
                            {
                                Ok(processed) => processed,
                                Err(problem) => {
                                    tracing::warn!("Step photo processing failed: {problem}");
                                    step_photo_upload_errors.write().push(
                                    crate::pages::recipe_editor_form::step_photo_processing_failure(
                                        &photo_file_name,
                                    ),
                                );
                                    release_failed_preview(&preview_url);
                                    continue;
                                }
                            };
                        let response = match upload_image(
                            form_account(),
                            processed.full_bytes,
                            processed.thumb_bytes,
                            format!("step_{idx}_{file_idx}.webp"),
                            crate::api::upload::UploadPurpose::Recipe,
                        )
                        .await
                        {
                            Ok(response) => response,
                            Err(problem) => {
                                tracing::warn!("Step photo upload failed: {problem}");
                                step_photo_upload_errors.write().push(
                                    crate::pages::recipe_editor_form::step_photo_upload_error_message(
                                        &photo_file_name,
                                        &problem,
                                    ),
                                );
                                release_failed_preview(&preview_url);
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
                        let full_url = response.full_url;
                        step_photo_discard_capabilities
                            .write()
                            .insert(full_url.clone(), response.discard_capability);
                        let photo = RecipeImage::managed(
                            response.asset_id,
                            full_url,
                            Some(response.thumb_url),
                            edit,
                        )
                        .with_file_metadata(photo_file_name, photo_file_size);
                        if let Some(step) = steps.write().get_mut(idx) {
                            let photo_idx = step.photos.len();
                            step.photos.push(photo);
                            if preview_url.is_some()
                                && preview_url.as_ref() == step_editor_modal_url().as_ref()
                                && step_editor_idx() == Some(idx)
                            {
                                step_editor_photo_idx.set(Some(photo_idx));
                            }
                        }
                    }
                    step_photo_upload_count.set(0);
                });
            }
        }
    };

    // ── Submit handler ───────────────────────────────────────────────────
    let on_submit = move |_| {
        if draft_creation_unconfirmed()
            || publication_unconfirmed()
            || is_saving()
            || is_manually_saving()
            || busy()
            || step_photo_upload_count() > 0
        {
            return;
        }
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
        if ings
            .iter()
            .flat_map(|row| std::iter::once(row).chain(row.substitutes.iter()))
            .any(|row| noms::units::canonicalize(&row.unit).is_err())
        {
            error.set(Some(noms::units::INVALID_UNIT.into()));
            return;
        }
        if ings.is_empty() {
            error.set(Some("Add at least one ingredient".to_string()));
            return;
        }
        if ings.iter().all(|i| i.name.trim().is_empty()) {
            error.set(Some("At least one ingredient must have a name".to_string()));
            return;
        }
        if ings.iter().any(|ingredient| {
            ingredient
                .substitutes
                .iter()
                .any(|substitute| substitute.name.trim().is_empty())
                || (!ingredient.substitutes.is_empty() && ingredient.name.trim().is_empty())
        }) {
            error.set(Some(
                "Give each ingredient and substitute a name, or remove the empty substitute."
                    .into(),
            ));
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
                // Uploads need attention — do NOT perform the requested save. User retries/removes
                // via the component UI, then clicks Save again.
                gallery_upload_error.set(Some(format!(
                    "{} image(s) need attention before saving. Check each image's status, then retry or remove it and save again.",
                    outcome.failed
                )));
                is_saving.set(false);
                return;
            }

            error.set(None);
            if draft_creation_unconfirmed() {
                error.set(Some(UNCONFIRMED_DRAFT_CREATION_MESSAGE.to_string()));
                is_saving.set(false);
                return;
            }
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
            let mut publication_attempted = false;
            let publish_result = async {
                let saved = if let Some(current_draft_id) = draft_id() {
                    let current =
                        get_recipe_draft_api(form_account(), current_draft_id.clone()).await?;
                    let first_save = save_recipe_draft(
                        form_account(),
                        current_draft_id.clone(),
                        current.revision,
                        editor.clone(),
                    )
                    .await;
                    match first_save {
                        Ok(saved) => saved,
                        Err(_) => {
                            let refreshed =
                                get_recipe_draft_api(form_account(), current_draft_id.clone())
                                    .await?;
                            save_recipe_draft(
                                form_account(),
                                current_draft_id,
                                refreshed.revision,
                                editor,
                            )
                            .await?
                        }
                    }
                } else {
                    let created = create_recipe_draft(form_account(), editor).await;
                    if created.as_ref().err().is_some_and(|problem| {
                        crate::auth::account_binding::account_error_feedback(problem).is_none()
                    }) {
                        draft_creation_unconfirmed.set(true);
                    }
                    created?
                };
                // Retain the identity before publication. If publication
                // commits but its response is lost, never create again.
                draft_id.set(Some(saved.id.to_string()));
                draft_revision.set(saved.revision);
                publication_attempted = true;
                publish_recipe_draft(form_account(), saved.id.to_string(), None).await
            }
            .await;
            match publish_result {
                Ok(recipe) => {
                    // The write is confirmed. Keep the app shell alive so a
                    // failed destination read can use its normal retry UI.
                    navigator.push(crate::Route::RecipeDetail {
                        id: recipe.id.to_string(),
                    });
                }
                Err(e) => {
                    if publication_attempted
                        && crate::auth::account_binding::account_error_feedback(&e).is_none()
                    {
                        // Do not let an autosave failure overwrite an unknown
                        // publication outcome or suggest another blind write.
                        publication_unconfirmed.set(true);
                    }
                    let action = if draft_creation_unconfirmed() {
                        RecipeWriteAction::CreateDraft
                    } else {
                        RecipeWriteAction::Publish
                    };
                    error.set(Some(recipe_write_feedback(action, &e).to_string()));
                    is_saving.set(false);
                }
            }
        });
    };

    // ── Cancel handler ──────────────────────────────────────────────────
    let on_cancel = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if is_saving() || is_manually_saving() || busy() || step_photo_upload_count() > 0 {
                return;
            }
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
            vocabulary_id: None,
            amount: String::new(),
            unit: String::new(),
            name: String::new(),
            ..Default::default()
        });
    };

    // ── Step helpers ────────────────────────────────────────────────────
    let on_add_step = move |_| {
        if step_photo_controls_locked() {
            return;
        }
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
            if publication_review_open() {
                crate::pages::recipe_editor_form::PublicationReview {
                    title: title(), ingredient_count: ingredients().iter().filter(|item| !item.name.trim().is_empty()).count(),
                    step_count: steps().iter().filter(|step| !step.text.trim().is_empty()).count(), visibility: visibility(),
                    preserve_visibility: false,
                    oncancel: move |_| publication_review_open.set(false),
                    onconfirm: move |event| {
                        publication_review_open.set(false);
                        if title().trim().is_empty() { active_editor_section.set("basics"); }
                        else if ingredients().iter().all(|item| item.name.trim().is_empty()) { active_editor_section.set("ingredients"); }
                        else if steps().iter().all(|step| step.text.trim().is_empty()) { active_editor_section.set("method"); }
                        let mut submit = on_submit;
                        submit(event);
                    },
                }
            }
            div { class: "container recipe-editor",
                PageHeader {
                    title: "New Recipe",
                    icon: Some(PageIconKind::Editor),
                    action: rsx! { Link { to: crate::Route::RecipeImport {}, class: "btn btn-secondary", "Import Recipe" } },
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
                    // ── Action buttons ──────────────────────────────────
                    div { class: "recipe-editor__actions",
                        display: "flex",
                        gap: "var(--space-md)",
                        margin_top: "var(--space-md)",
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: !draft_save_state.is_actionable()
                                || draft_creation_unconfirmed()
                                || publication_unconfirmed()
                                || is_saving()
                                || busy()
                                || step_photo_upload_count() > 0,
                            onclick: on_save_draft,
                            span { class: "desktop-editor-label", "{draft_save_state.label()}" }
                            span { class: "mobile-only", if draft_save_state.is_actionable() { "Save" } else { "{draft_save_state.label()}" } }
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: draft_creation_unconfirmed() || publication_unconfirmed() || is_saving() || is_manually_saving() || busy() || step_photo_upload_count() > 0,
                            onclick: move |event| {
                                if crate::components::mobile::is_mobile_layout() { publication_review_open.set(true); }
                                else { let mut submit = on_submit; submit(event); }
                            },
                            if step_photo_upload_count() > 0 {
                                "Uploading images..."
                            } else if is_saving() {
                                "Publishing..."
                            } else {
                                span { class: "desktop-publish-label", "Publish Recipe" }
                                span { class: "mobile-only", "Review" }
                            }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            disabled: is_saving() || is_manually_saving() || busy() || step_photo_upload_count() > 0,
                            id: "recipe-editor-exit",
                            onclick: on_cancel,
                            "Cancel"
                        }
                    }
                div { class: "recipe-editor__layout",
                    RecipeEditorContents {
                        active_section: active_editor_section,
                        title_present: !title().trim().is_empty(),
                        photo_count: images().len() + pending_image_count(),
                        ingredient_count: ingredients().iter().filter(|ingredient| !ingredient.name.trim().is_empty()).count(),
                        step_count: steps().iter().filter(|step| !step.text.trim().is_empty()).count(),
                        equipment_count: equipment().iter().filter(|item| !item.name.trim().is_empty()).count(),
                        notes_present: !commentary().trim().is_empty(),
                    }
                    div { class: "recipe-editor__panels",

                    section { class: "recipe-editor__panel recipe-editor__basics neumo-raised", aria_labelledby: "recipe-basics",
                        "data-mobile-active": active_editor_section() == "basics",
                        onfocusin: move |_| active_editor_section.set("basics"),
                        onclick: move |_| active_editor_section.set("basics"),
                        h2 { id: "recipe-basics", tabindex: "-1", "Basics" }
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
                            accessible_name: "Recipe name",
                            value: title().clone(),
                            placeholder: "e.g. Grandma's Chocolate Chip Cookies",
                            oninput: move |v| {
                                title.set(v);
                                error.set(None);
                            },
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
                            value: description(),
                            aria_label: "Description",
                            rows: "4",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            resize: "vertical",
                            width: "100%",
                            oninput: move |evt| {
                                description.set(evt.value());
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
                            accessible_name: "Prep time in minutes",
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
                            accessible_name: "Cook time in minutes",
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
                            accessible_name: "Servings",
                                value: servings().clone(),
                                placeholder: "4",
                                input_type: "number",
                                oninput: move |v| servings.set(v),
                            }
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
                            accessible_name: "Tags",
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

                    // ── Visibility ──────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Visibility when published"
                        }
                        select {
                                aria_label: "Visibility when published",
                            class: "neumo-inset input",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
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

                    }

                    section { class: "recipe-editor__panel recipe-editor__photos neumo-raised", aria_labelledby: "recipe-photos",
                        "data-mobile-active": active_editor_section() == "photos",
                        onfocusin: move |_| active_editor_section.set("photos"),
                        onclick: move |_| active_editor_section.set("photos"),
                        h2 { id: "recipe-photos", tabindex: "-1", "Photos" span { " · Optional" } }
                    // ── Recipe Images ───────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        ImageUpload {
                            form_account,
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

                    }

                    section { class: "recipe-editor__panel recipe-editor__ingredients neumo-raised", aria_labelledby: "recipe-ingredients",
                        "data-mobile-active": active_editor_section() == "ingredients",
                        onfocusin: move |_| active_editor_section.set("ingredients"),
                        onclick: move |_| active_editor_section.set("ingredients"),
                        h2 { id: "recipe-ingredients", tabindex: "-1", "Ingredients" }
                    // ── Ingredients ─────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",

                        for idx in 0..ingredients.read().len() {
                            super::recipe_ingredient_editor::RecipeIngredientEditor {
                                key: "{idx}", ingredients, index: idx,
                            }
                        }

                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: on_add_ingredient,
                            "+ Add Ingredient"
                        }
                    }

                    }

                    section { class: "recipe-editor__panel recipe-editor__method neumo-raised", aria_labelledby: "recipe-method",
                        "data-mobile-active": active_editor_section() == "method",
                        onfocusin: move |_| active_editor_section.set("method"),
                        onclick: move |_| active_editor_section.set("method"),
                        h2 { id: "recipe-method", tabindex: "-1", "Method" }
                        StepPhotoUploadFeedback { uploading: step_photo_upload_count() > 0, errors: step_photo_upload_errors() }
                    // ── Steps ───────────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",

                        for (idx, _step) in steps().iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "recipe-editor__step-row",
                                display: "flex",
                                gap: "var(--space-sm)",
                                align_items: "flex-start",
                                // Indentation padding
                                style: format!("--step-indent: {}px; padding-left: var(--step-indent)", steps()[idx].depth * 24),

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
                            aria_label: "Method step {idx + 1}",
                                        rows: "4",
                                        width: "100%",
                                        padding: "var(--space-sm) var(--space-md)",
                                        font_family: "var(--font-body)",
                                        font_size: "14px",
                                        color: "var(--text-primary)",
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
                                                draggable: !step_photo_controls_locked(),
                                                ondragstart: move |_| { if step_photo_controls_locked() { return; } dragged_step_photo.set(Some((idx, photo_idx))); },
                                                ondragend: move |_| dragged_step_photo.set(None),
                                                ondragover: move |event| {
                                                    if dragged_step_photo().is_some() {
                                                        event.prevent_default();
                                                    }
                                                },
                                                ondrop: move |event| {
                                                    event.prevent_default();
                                                    if step_photo_controls_locked() { return; }
                                                    if let Some((source_step_idx, source_photo_idx)) = dragged_step_photo() {
                                                        reorder_step_photo_within_step(
                                                            &mut steps.write(),
                                                            (source_step_idx, source_photo_idx),
                                                            (idx, photo_idx),
                                                        );
                                                    }
                                                    dragged_step_photo.set(None);
                                                },
                                                    div { class: "step-photo-item__preview neumo-inset",
                                                        StepPhotoEditButton {
                                                            image: steps()[idx].photos[photo_idx].clone(),
                                                            disabled: step_photo_controls_locked(),
                                                            onedit: move |photo: RecipeImage| {
                                                                if step_photo_controls_locked() { return; }
                                                                step_editor_idx.set(Some(idx));
                                                                step_editor_photo_idx.set(Some(photo_idx));
                                                                step_editor_initial_edit.set(Some(step_photo_editor_edit(&photo)));
                                                                step_editor_modal_url.set(Some(photo.url));
                                                            },
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
                                                            disabled: (photo_idx == 0) || step_photo_controls_locked(),
                                                            aria_label: "Move step photo left",
                                                            onclick: move |_| {
                                                                if step_photo_controls_locked() { return; }
                                                                steps.write()[idx].photos.swap(photo_idx, photo_idx - 1);
                                                            },
                                                            span { aria_hidden: "true", "←" }
                                        span { class: "sr-only", "Outdent step" }
                                                        }
                                                        button {
                                                            r#type: "button",
                                                            class: "btn btn-ghost touch-target",
                                                            disabled: (photo_idx + 1 >= steps()[idx].photos.len()) || step_photo_controls_locked(),
                                                            aria_label: "Move step photo right",
                                                            onclick: move |_| {
                                                                if step_photo_controls_locked() { return; }
                                                                steps.write()[idx].photos.swap(photo_idx, photo_idx + 1);
                                                            },
                                                            span { aria_hidden: "true", "→" }
                                        span { class: "sr-only", "Indent step" }
                                                        }
                                                        button {
                                                            disabled: step_photo_controls_locked(),
                                                            r#type: "button",
                                                            class: "btn btn-ghost touch-target step-photo-item__remove",
                                                            aria_label: "Remove step photo",
                                                            onclick: move |_| {
                                                                if step_photo_controls_locked() { return; }
                                                                let removed = steps.write()[idx].photos.remove(photo_idx);
                                                                if let Some(capability) = step_photo_discard_capabilities.write().remove(&removed.url) {
                                                                    spawn(async move {
                                                                        if let Err(problem) = crate::api::upload::discard_upload(form_account(), capability).await {
                                                                            tracing::warn!("Failed to discard unsaved step photo: {problem}");
                                                                        }
                                                                    });
                                                                }
                                                            },
                                                            span { aria_hidden: "true", "✕" }
                                        span { class: "sr-only", "Remove item" }
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
                                        disabled: step_photo_controls_locked(),
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
                                            if step_photo_controls_locked() { return; }
                                            trigger_file_input(&format!("#step-photo-{idx}"));
                                        },
                                        span { aria_hidden: "true", "📷" }
                                        span { class: "sr-only", "Add step photo" }
                                    }
                                    input {
                                        disabled: step_photo_controls_locked(),
                                        id: format!("step-photo-{idx}"),
                                        r#type: "file",
                                        accept: "image/jpeg,image/png,image/webp",
                                        multiple: true,
                                        display: "none",
                                        onchange: move |evt| {
                                            on_step_photo_change(idx)(evt.files());
                                        },
                                    }
                                    // Move up
                                        Button {
                                            disabled: (idx == 0) || step_photo_controls_locked(),
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                if step_photo_controls_locked() { return; }
                                                let mut s = steps.write();
                                                s.swap(idx, idx - 1);
                                            },
                                            span { aria_hidden: "true", "↑" }
                                        span { class: "sr-only", "Move step up" }
                                        }
                                    // Move down
                                        Button {
                                            disabled: (idx + 1 >= steps().len()) || step_photo_controls_locked(),
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                if step_photo_controls_locked() { return; }
                                                let mut s = steps.write();
                                                s.swap(idx, idx + 1);
                                            },
                                            span { aria_hidden: "true", "↓" }
                                        span { class: "sr-only", "Move step down" }
                                        }
                                    // Indent (increase depth)
                                    if can_indent_step(steps()[idx].depth) {
                                        Button {
                                            disabled: step_photo_controls_locked(),
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                if step_photo_controls_locked() { return; }
                                                steps.write()[idx].depth += 1;
                                            },
                                            span { aria_hidden: "true", "→" }
                                        span { class: "sr-only", "Indent step" }
                                        }
                                    }
                                    // Unindent (decrease depth)
                                    if steps()[idx].depth > 0 {
                                        Button {
                                            disabled: step_photo_controls_locked(),
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                if step_photo_controls_locked() { return; }
                                                steps.write()[idx].depth -= 1;
                                            },
                                            span { aria_hidden: "true", "←" }
                                        span { class: "sr-only", "Outdent step" }
                                        }
                                    }
                                    // Remove
                                    Button {
                                        disabled: step_photo_controls_locked(),
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
                                            if step_photo_controls_locked() { return; }
                                            for photo in steps()[idx].photos.clone() {
                                                if let Some(capability) = step_photo_discard_capabilities.write().remove(&photo.url) {
                                                    spawn(async move {
                                                        if let Err(problem) = crate::api::upload::discard_upload(form_account(), capability).await {
                                                            tracing::warn!("Failed to discard unsaved step photo: {problem}");
                                                        }
                                                    });
                                                }
                                            }
                                            steps.write().remove(idx);
                                        },
                                        span { aria_hidden: "true", "✕" }
                                        span { class: "sr-only", "Remove item" }
                                    }
                                }
                            }
                        }

                        Button {
                            disabled: step_photo_controls_locked(),
                            variant: ButtonVariant::Secondary,
                            onclick: on_add_step,
                            "+ Add Step"
                        }
                    }

                    }

                    section { class: "recipe-editor__panel recipe-editor__equipment neumo-raised", aria_labelledby: "recipe-equipment",
                        "data-mobile-active": active_editor_section() == "equipment",
                        onfocusin: move |_| active_editor_section.set("equipment"),
                        onclick: move |_| active_editor_section.set("equipment"),
                        h2 { id: "recipe-equipment", tabindex: "-1", "Equipment" span { " · Optional" } }
                    // ── Equipment ───────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",

                        for (idx, _eq) in equipment().iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "recipe-editor__equipment-row",
                                display: "flex",
                                gap: "var(--space-sm)",
                                align_items: "center",
                                crate::components::equipment_autocomplete::EquipmentAutocomplete {
                                    id: "recipe-equipment-{idx}",
                                    accessible_name: "Equipment {idx + 1} name",
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
                                    span { aria_hidden: "true", "✕" }
                                        span { class: "sr-only", "Remove item" }
                                }
                            }
                        }

                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: on_add_equipment,
                            "+ Add Equipment"
                        }
                    }

                    }

                    section { class: "recipe-editor__panel recipe-editor__notes neumo-raised", aria_labelledby: "recipe-notes",
                        "data-mobile-active": active_editor_section() == "notes",
                        onfocusin: move |_| active_editor_section.set("notes"),
                        onclick: move |_| active_editor_section.set("notes"),
                        h2 { id: "recipe-notes", tabindex: "-1", "Notes" span { " · Optional" } }
                    // Commentary
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        textarea {
                            class: "neumo-inset input",
                            placeholder: "Notes, tips, or additional context about this recipe...",
                            value: commentary(),
                            aria_label: "Notes",
                            rows: "4",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            resize: "vertical",
                            width: "100%",
                            oninput: move |evt| {
                                commentary.set(evt.value());
                            },
                        }
                    }

                    }

                    // ── Error message ───────────────────────────────────
                    if let Some(err) = gallery_upload_error() {
                        div {
                            role: "alert", overflow_wrap: "anywhere",
                            padding: "var(--space-sm) var(--space-md)",
                            background_color: "var(--error-bg)", border_radius: "var(--radius-md)",
                            color: "var(--error-ink)", font_size: "14px", "{err}"
                        }
                    }
                    if draft_creation_unconfirmed() || publication_unconfirmed() || error().is_some() {
                        div {
                            role: "alert",
                            overflow_wrap: "anywhere",
                            padding: "var(--space-sm) var(--space-md)",
                            background_color: "var(--error-bg)",
                            border_radius: "var(--radius-md)",
                            color: "var(--error-ink)",
                            font_size: "14px",
                            if draft_creation_unconfirmed() {
                                "{UNCONFIRMED_DRAFT_CREATION_MESSAGE}"
                            } else if publication_unconfirmed() {
                                "{UNCONFIRMED_PUBLICATION_MESSAGE}"
                            } else if let Some(err) = error() {
                                "{err}"
                            }
                            if draft_creation_unconfirmed() || publication_unconfirmed() {
                                p {
                                    Link {
                                        to: crate::Route::Dashboard {},
                                        new_tab: true,
                                        class: "btn btn-secondary touch-target",
                                        "Check Dashboard"
                                    }
                                }
                            }
                        }
                    }


                    }
                }
            }
            ImageEditorModal {
                image_url: step_editor_modal_url,
                initial_edit: step_editor_initial_edit(),
                on_save: move |edit: RecipeImageEdit| {
                    let current_url = step_editor_modal_url();
                    let current_idx = step_editor_idx();
                    let current_photo_idx = step_editor_photo_idx();
                    if let (Some(url), Some(idx)) = (current_url, current_idx) {
                        // Only a local upload still in flight needs an edit hand-off.
                        if url.starts_with("blob:") && current_photo_idx.is_none() {
                            pending_step_photo_edits.write().retain(|pending| {
                                !(pending.idx == idx && pending.preview_url == url)
                            });
                            pending_step_photo_edits.write().push(PendingStepPhotoEdit {
                                idx,
                                preview_url: url.clone(),
                                edit: edit.clone(),
                            });
                        }
                        if let Some(step) = steps.write().get_mut(idx) {
                            if let Some(photo_idx) = current_photo_idx {
                                if let Some(photo) = step.photos.get_mut(photo_idx) {
                                    apply_step_photo_edit(photo, edit);
                                }
                            }
                        }
                    }
                },
                on_close: move |_| {
                    step_photo_previews.write().clear();
                    step_editor_modal_url.set(None);
                    step_editor_idx.set(None);
                    step_editor_photo_idx.set(None);
                    step_editor_initial_edit.set(None);
                },
            }
        }
    }
}
