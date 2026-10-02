//! Recipe edit page.
//!
//! Loads an owner-only draft, autosaves it with revision guards, and publishes
//! the draft explicitly while leaving the current recipe visible.

use crate::components::base::PageIconKind;
use dioxus::prelude::*;

use crate::api::recipe::{
    create_edit_draft, discard_recipe_draft, get_recipe_draft_api, publish_recipe_draft,
    save_recipe_draft,
};
#[allow(unused_imports)]
use crate::api::upload::upload_image;
use crate::components::base::{
    Button, ButtonVariant, Card, ImageEditorModal, ImageUpload, LoadingSpinner, PageHeader,
    UploadOutcome,
};
use crate::components::AuthRequired;
use crate::pages::draft_save::draft_save_button_state;
use crate::pages::recipe_editor_form::{
    apply_step_photo_edit, build_step_tree, can_indent_step, flatten_steps,
    format_step_photo_file_size, reset_step_photo_input, step_photo_display_name,
    step_photo_editor_edit, RecipeEditorContents, RecipeEditorInput as Input, StepForm,
    StepPhotoEditButton, StepPhotoPreview, StepPhotoUploadFeedback,
};
use crate::types::{
    RecipeEditorState, RecipeEquipment, RecipeImage, RecipeImageEdit, RecipeIngredient,
};
use crate::utils::reorder::move_item;

// ── Local form types ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct PendingStepPhotoEdit {
    idx: usize,
    preview_url: String,
    edit: RecipeImageEdit,
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

fn draft_allows_initial_visibility(recipe_id: Option<uuid::Uuid>) -> bool {
    recipe_id.is_none()
}

#[derive(Clone, Copy)]
enum RecipeWriteAction {
    #[cfg(any(target_arch = "wasm32", test))]
    Autosave,
    SaveDraft,
    Publish,
    Discard,
}

// Publication, draft saving and discard all mutate the same draft. Recheck
// this at action time as well as disabling controls, so queued clicks cannot
// start a competing write before the next render.
fn editor_write_in_progress(publishing: bool, saving_draft: bool, discarding: bool) -> bool {
    publishing || saving_draft || discarding
}

fn recipe_write_feedback(action: RecipeWriteAction, error: &ServerFnError) -> &'static str {
    if matches!(error, ServerFnError::ServerError { message, .. } if message == "Not authenticated")
    {
        return "Sign in again in another tab, then try again. Keep this page open to retain your entries.";
    }
    match action {
        #[cfg(any(target_arch = "wasm32", test))]
        RecipeWriteAction::Autosave => "Autosave could not complete. Your entries are still here; try Save draft or edit again to retry.",
        RecipeWriteAction::SaveDraft => "Could not save the draft. Your entries are still here; try Save draft again.",
        // A lost response is ambiguous: publication may already have committed.
        RecipeWriteAction::Publish => "Could not confirm publication. Your draft may have been published; check your dashboard before trying again.",
        RecipeWriteAction::Discard => "Could not confirm draft removal. Check your dashboard before trying again; your entries remain on this page.",
    }
}

fn render_recipe_editor_unavailable() -> Element {
    // Missing, inaccessible and failed loads share one useful public state,
    // without exposing database identifiers or transport exception wrappers.
    rsx! {
        div { class: "container recipe-editor",
            Card {
                div {
                    role: "alert",
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    text_align: "center",
                    gap: "var(--space-md)",
                    h1 { "Recipe editor unavailable" }
                    p { "This recipe or draft may not exist or you may not have access to edit it. Try opening it again from your dashboard." }
                    Link {
                        to: crate::Route::Dashboard {},
                        class: "btn btn-secondary touch-target",
                        "Back to Dashboard"
                    }
                }
            }
        }
    }
}

/// Edit an existing published recipe through its mutable draft.
#[component]
pub fn RecipeEdit(id: String) -> Element {
    recipe_editor(id, false)
}

/// Edit an unsaved new/fork draft directly by draft ID.
#[component]
pub fn RecipeDraftEdit(id: String) -> Element {
    recipe_editor(id, true)
}

fn recipe_editor(id: String, id_is_draft: bool) -> Element {
    // ── Form signals ─────────────────────────────────────────────────────
    let mut title = use_signal(String::new);
    let mut active_editor_section = use_signal(|| "basics");
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
    let pending_image_count = use_signal(|| 0usize);
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

    // CP11: deferred delete — track URLs to delete after save succeeds
    let mut pending_deletes = use_signal(Vec::<String>::new);

    // UI state
    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    #[allow(unused_mut)]
    let mut is_manually_saving = use_signal(|| false);
    let mut draft_save_after_upload = use_signal(|| false);
    let mut draft_save_destination = use_signal(|| Option::<String>::None);
    let mut load_error = use_signal(|| Option::<String>::None);
    let mut form_error = use_signal(|| Option::<String>::None);
    #[allow(unused_mut)]
    let mut draft_save_error = use_signal(|| Option::<String>::None);
    let mut is_discarding = use_signal(|| false);
    let mut draft_id = use_signal(|| Option::<String>::None);
    let mut draft_recipe_id = use_signal(|| Option::<uuid::Uuid>::None);
    let mut draft_revision = use_signal(|| 0i64);
    let mut last_saved_editor = use_signal(|| Option::<RecipeEditorState>::None);
    let mut autosave_generation = use_signal(|| 0u64);
    let mut forked_from_title = use_signal(|| Option::<String>::None);
    let mut forked_from_recipe_id = use_signal(|| Option::<String>::None);
    let mut fork_message = use_signal(|| Option::<String>::None);

    // ── Fetch recipe data on mount ───────────────────────────────────────
    let id_for_fetch = id.clone();
    use_effect(move || {
        let id = id_for_fetch.clone();
        spawn(async move {
            let loaded = if id_is_draft {
                get_recipe_draft_api(id.clone()).await
            } else {
                create_edit_draft(id.clone()).await
            };
            match loaded {
                Ok(draft) => {
                    let editor = draft.editor.clone();
                    draft_id.set(Some(draft.id.to_string()));
                    draft_recipe_id.set(draft.recipe_id);
                    draft_revision.set(draft.revision);
                    forked_from_title.set(draft.forked_from_title.clone());
                    forked_from_recipe_id.set(draft.forked_from_recipe_id.map(|id| id.to_string()));
                    fork_message.set(draft.fork_message.clone());
                    last_saved_editor.set(Some(editor.clone()));
                    title.set(editor.title.clone());
                    description.set(editor.description.clone().unwrap_or_default());
                    commentary.set(editor.commentary.clone().unwrap_or_default());
                    prep_time.set(
                        editor
                            .prep_time_minutes
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    );
                    cook_time.set(
                        editor
                            .cook_time_minutes
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    );
                    servings.set(editor.servings.map(|v| v.to_string()).unwrap_or_default());
                    ingredients.set(editor.ingredients);
                    steps.set(flatten_steps(&editor.instructions));
                    tags_input.set(editor.tags.join(", "));
                    equipment.set(editor.equipment);
                    visibility.set(editor.visibility);
                    images.set(editor.images);
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

    // Two-second debounced autosave. Generation checks prevent an older
    // response from replacing the revision for newer local editor state.
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
        if is_loading()
            || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
            || last_saved_editor().as_ref() == Some(&editor)
        {
            return;
        }
        #[allow(unused_variables)]
        let Some(current_draft_id) = draft_id() else {
            return;
        };
        // `peek` avoids making the generation counter itself an effect
        // dependency; subscribing to it would continuously reschedule this
        // effect before the debounce could elapse.
        let generation = *autosave_generation.peek() + 1;
        autosave_generation.set(generation);
        #[allow(unused_variables)]
        let expected_revision = draft_revision();
        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(2_000).await;
            if autosave_generation() != generation
                || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
            {
                return;
            }
            let first_save =
                save_recipe_draft(current_draft_id.clone(), expected_revision, editor.clone())
                    .await;
            let save_result = match first_save {
                Ok(saved) => Ok(saved),
                Err(_) if autosave_generation() == generation => {
                    // A previous in-flight autosave may have advanced the
                    // revision. Refresh and retry only for the newest editor
                    // generation so stale content can never win.
                    match get_recipe_draft_api(current_draft_id.clone()).await {
                        Ok(current) => {
                            save_recipe_draft(current_draft_id, current.revision, editor.clone())
                                .await
                        }
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            };
            match save_result {
                Ok(saved) if autosave_generation() == generation => {
                    draft_revision.set(saved.revision);
                    last_saved_editor.set(Some(editor));
                    draft_save_error.set(None);
                }
                Ok(_) => {}
                Err(error) if autosave_generation() == generation => {
                    draft_save_error.set(Some(
                        recipe_write_feedback(RecipeWriteAction::Autosave, &error).to_string(),
                    ));
                }
                Err(_) => {}
            }
        });
    });

    #[allow(unused_variables)]
    let on_save_draft = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if is_loading()
                || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
                || busy()
                || step_photo_upload_count() > 0
            {
                return;
            }
            if draft_id().is_none() {
                form_error.set(Some(
                    "Draft is still loading. Try again in a moment.".to_string(),
                ));
                return;
            }
            form_error.set(None);
            draft_save_error.set(None);
            is_manually_saving.set(true);
            draft_save_after_upload.set(true);
            draft_save_destination.set(None);
            autosave_generation.set(autosave_generation() + 1);
            upload_now.set(true);
        }
    };

    // ── Clone id for closures that need it ──────────────────────────────
    let _id_for_submit = id.clone(); // used in on_submit (two-phase: no direct DB call)
    #[allow(unused_variables)]
    let id_for_cancel = id.clone();

    // ── Step photo upload handler ────────────────────────────────────────
    #[allow(unused_variables)]
    let step_photo_controls_locked = move || {
        editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
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
                                    crate::pages::recipe_editor_form::step_photo_upload_failure(
                                        &photo_file_name,
                                        true,
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
                                        crate::pages::recipe_editor_form::step_photo_upload_failure(
                                            &photo_file_name,
                                            true,
                                        ),
                                    );
                                    release_failed_preview(&preview_url);
                                    continue;
                                }
                            };
                        let response = match upload_image(
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
                                    crate::pages::recipe_editor_form::step_photo_upload_failure(
                                        &photo_file_name,
                                        false,
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

    // ── Submit handler (two-phase: upload images → save recipe) ────────
    let on_submit = move |_| {
        if is_loading()
            || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
            || step_photo_upload_count() > 0
        {
            return;
        }
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
        draft_save_after_upload.set(false);
        is_saving.set(true);
        upload_now.set(true);
    };

    // ── CP5: Upload complete handler ────────────────────────────────────
    // Fired by ImageUpload when the upload batch finishes. Proceeds to
    // update_recipe with the merged images list (existing + newly uploaded).
    let on_upload_complete = move |outcome: UploadOutcome| {
        spawn(async move {
            if let Some(msg) = outcome.aborted {
                form_error.set(Some(msg));
                is_saving.set(false);
                is_manually_saving.set(false);
                draft_save_after_upload.set(false);
                draft_save_destination.set(None);
                return;
            }

            if outcome.failed > 0 {
                form_error.set(Some(format!(
                    "{} image(s) failed to upload. Use the Retry button on each failed image (or remove them), then click Save again.",
                    outcome.failed
                )));
                is_saving.set(false);
                is_manually_saving.set(false);
                draft_save_after_upload.set(false);
                draft_save_destination.set(None);
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
            let Some(current_draft_id) = draft_id() else {
                form_error.set(Some("Draft is not ready yet".to_string()));
                is_saving.set(false);
                is_manually_saving.set(false);
                draft_save_after_upload.set(false);
                draft_save_destination.set(None);
                return;
            };
            autosave_generation.set(autosave_generation() + 1);
            if draft_save_after_upload() {
                let current = get_recipe_draft_api(current_draft_id.clone()).await;
                let save_result = match current {
                    Ok(current) => {
                        save_recipe_draft(current_draft_id, current.revision, editor.clone()).await
                    }
                    Err(error) => Err(error),
                };
                match save_result {
                    Ok(saved) => {
                        draft_revision.set(saved.revision);
                        last_saved_editor.set(Some(editor));
                        draft_save_error.set(None);
                        #[cfg(target_arch = "wasm32")]
                        if let Some(destination) = draft_save_destination() {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(&destination);
                            }
                        }
                    }
                    Err(save_error) => {
                        draft_save_error.set(Some(
                            recipe_write_feedback(RecipeWriteAction::SaveDraft, &save_error)
                                .to_string(),
                        ));
                    }
                }
                draft_save_after_upload.set(false);
                draft_save_destination.set(None);
                is_manually_saving.set(false);
                return;
            }
            let publish_result = async {
                // Refresh the revision in case an autosave completed immediately
                // before the publish click, then save the exact submitted state.
                let current = get_recipe_draft_api(current_draft_id.clone()).await?;
                let first_save =
                    save_recipe_draft(current_draft_id.clone(), current.revision, editor.clone())
                        .await;
                let saved = match first_save {
                    Ok(saved) => saved,
                    Err(_) => {
                        let refreshed = get_recipe_draft_api(current_draft_id.clone()).await?;
                        save_recipe_draft(current_draft_id.clone(), refreshed.revision, editor)
                            .await?
                    }
                };
                draft_revision.set(saved.revision);
                draft_save_error.set(None);
                publish_recipe_draft(current_draft_id, None).await
            }
            .await;

            match publish_result {
                #[allow(unused_variables)]
                Ok(recipe) => {
                    // Published history retains image metadata, so removed
                    // current images must stay in storage while an immutable
                    // version still references them. The orphan worker now
                    // scans versions and drafts and will clean them only after
                    // every database reference is gone.
                    pending_deletes.set(Vec::new());

                    // Redirect to recipe detail page
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
                    form_error.set(Some(
                        recipe_write_feedback(RecipeWriteAction::Publish, &e).to_string(),
                    ));
                    is_saving.set(false);
                }
            }
        });
    };

    // ── Cancel handler ──────────────────────────────────────────────────
    let on_cancel = move |_| {
        if is_loading()
            || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
            || busy()
            || step_photo_upload_count() > 0
        {
            return;
        }
        let destination = if id_is_draft {
            "/dashboard".to_string()
        } else {
            format!("/recipes/{id_for_cancel}")
        };
        form_error.set(None);
        is_manually_saving.set(true);
        draft_save_after_upload.set(true);
        draft_save_destination.set(Some(destination));
        autosave_generation.set(autosave_generation() + 1);
        upload_now.set(true);
    };

    let id_for_discard = id.clone();
    let on_discard = move |_| {
        if is_loading()
            || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
            || busy()
            || step_photo_upload_count() > 0
        {
            return;
        }
        let Some(current_draft_id) = draft_id() else {
            return;
        };
        is_discarding.set(true);
        form_error.set(None);
        #[allow(unused_variables)]
        let published_recipe_id = id_for_discard.clone();
        spawn(async move {
            match discard_recipe_draft(current_draft_id).await {
                Ok(()) =>
                {
                    #[cfg(target_arch = "wasm32")]
                    if let Some(window) = web_sys::window() {
                        let destination = if id_is_draft {
                            "/dashboard".to_string()
                        } else {
                            format!("/recipes/{published_recipe_id}")
                        };
                        let _ = window.location().set_href(&destination);
                    }
                }
                Err(error) => {
                    form_error.set(Some(
                        recipe_write_feedback(RecipeWriteAction::Discard, &error).to_string(),
                    ));
                    is_discarding.set(false);
                }
            }
        });
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

    // ── CP11: Deferred image delete handler ────────────────────────────────────
    let on_delete_image = move |url: String| {
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
    if load_error().is_some() {
        return rsx! {
            AuthRequired {
                {render_recipe_editor_unavailable()}
            }
        };
    }

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
    let draft_save_state = draft_save_button_state(
        is_manually_saving(),
        pending_image_count() > 0 || last_saved_editor().as_ref() != Some(&current_editor),
    );

    // ── Render: form ────────────────────────────────────────────────────
    rsx! {
        AuthRequired {
            div { class: "container recipe-editor",
                PageHeader {
                    icon: Some(PageIconKind::Editor),
                    title: if id_is_draft { "Edit Draft" } else { "Edit Recipe Draft" },
                }
                if let Some(source_title) = forked_from_title() {
                    div {
                        class: "recipe-detail__tags",
                        span { class: "recipe-detail__tag", "DRAFT" }
                        if let Some(source_id) = forked_from_recipe_id() {
                            Link {
                                to: crate::Route::RecipeDetail { id: source_id },
                                "Forked from {source_title}"
                            }
                        } else {
                            span { "Forked from {source_title}" }
                        }
                        if let Some(message) = fork_message() {
                            span { " · {message}" }
                        }
                    }
                } else {
                    div { class: "recipe-detail__tags",
                        span { class: "recipe-detail__tag", "DRAFT" }
                        span { "Changes autosave privately until you publish." }
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
                                || editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding())
                                || busy()
                                || step_photo_upload_count() > 0,
                            onclick: on_save_draft,
                            "{draft_save_state.label()}"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding()) || busy() || step_photo_upload_count() > 0,
                            onclick: on_submit,
                            if step_photo_upload_count() > 0 || (is_saving() && busy()) {
                                "Uploading images..."
                            } else if is_saving() {
                                "Publishing..."
                            } else {
                                "Publish Changes"
                            }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            disabled: editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding()) || busy() || step_photo_upload_count() > 0,
                            onclick: on_cancel,
                            "Cancel"
                        }
                        Button {
                            variant: ButtonVariant::Danger,
                            disabled: editor_write_in_progress(is_saving(), is_manually_saving(), is_discarding()) || busy() || step_photo_upload_count() > 0,
                            onclick: on_discard,
                            if is_discarding() { "Discarding..." } else { "Discard Draft" }
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
                                form_error.set(None);
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
                            aria_label: "Description",
                            rows: "4",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            resize: "vertical",
                            width: "100%",
                            value: description().clone(),
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

                    // Visibility is chosen here only for first publication.
                    if draft_allows_initial_visibility(draft_recipe_id()) {
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

                    if !draft_allows_initial_visibility(draft_recipe_id()) {
                        p { class: "recipe-editor__hint", "Publishing keeps the recipe’s current visibility. Manage visibility from the published recipe." }
                    }
                    }

                    section { class: "recipe-editor__panel recipe-editor__photos neumo-raised", aria_labelledby: "recipe-photos",
                        onfocusin: move |_| active_editor_section.set("photos"),
                        onclick: move |_| active_editor_section.set("photos"),
                        h2 { id: "recipe-photos", tabindex: "-1", "Photos" span { " · Optional" } }
                    // ── CP5: Recipe Images ──────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",

                        // Persisted and newly selected images render together.
                        ImageUpload {
                            images,
                            upload_now,
                            busy,
                            pending_count: pending_image_count,
                            purpose: crate::api::upload::UploadPurpose::Recipe,
                            on_remove_existing: on_delete_image,
                            on_upload_complete,
                        }
                        span {
                            font_size: "12px",
                            color: "var(--text-tertiary)",
                            "Add more images (JPEG, PNG, or WebP · max 10 MB each)"
                        }
                    }

                    }

                    section { class: "recipe-editor__panel recipe-editor__ingredients neumo-raised", aria_labelledby: "recipe-ingredients",
                        onfocusin: move |_| active_editor_section.set("ingredients"),
                        onclick: move |_| active_editor_section.set("ingredients"),
                        h2 { id: "recipe-ingredients", tabindex: "-1", "Ingredients" }
                    // ── Ingredients ─────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",

                        for (idx, _ing) in ingredients().iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "recipe-editor__ingredient-row",
                                display: "flex",
                                gap: "var(--space-sm)",
                                align_items: "center",
                                // Amount
                                div {
                                    style: "width: 70px; flex-shrink: 0;",
                                    Input {
                            accessible_name: "Ingredient {idx + 1} amount",
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
                            accessible_name: "Ingredient {idx + 1} unit",
                                        value: ingredients()[idx].unit.clone(),
                                        placeholder: "Unit",
                                        oninput: move |v| {
                                            ingredients.write()[idx].unit = v;
                                        },
                                    }
                                }
                                // Name
                                Input {
                            accessible_name: "Ingredient {idx + 1} name",
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
                                    span { aria_hidden: "true", "✕" }
                                        span { class: "sr-only", "Remove item" }
                                }
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
                                                                            if let Err(problem) = crate::api::upload::discard_upload(capability).await {
                                                                                tracing::warn!("Failed to discard unsaved step photo: {problem}");
                                                                            }
                                                                        });
                                                                    } else {
                                                                        pending_deletes.write().push(removed.url);
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
                                                        if let Err(problem) = crate::api::upload::discard_upload(capability).await {
                                                            tracing::warn!("Failed to discard unsaved step photo: {problem}");
                                                        }
                                                    });
                                                } else {
                                                    pending_deletes.write().push(photo.url);
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
                        onfocusin: move |_| active_editor_section.set("equipment"),
                        onclick: move |_| active_editor_section.set("equipment"),
                        h2 { id: "recipe-equipment", tabindex: "-1", "Equipment" span { " · Optional" } }
                    // ── Equipment (dynamic list) ────────────────────────
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
                                Input {
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
                            aria_label: "Notes",
                            rows: "4",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            resize: "vertical",
                            width: "100%",
                            value: commentary().clone(),
                            oninput: move |evt| {
                                commentary.set(evt.value());
                            },
                        }
                    }

                    }

                    // ── Error message ───────────────────────────────────
                    if let Some(err) = draft_save_error() {
                        div {
                            role: "alert", padding: "var(--space-sm) var(--space-md)",
                            background_color: "var(--error-bg)", border_radius: "var(--radius-md)",
                            color: "var(--error-ink)", font_size: "14px", "{err}"
                        }
                    }
                    if let Some(err) = form_error() {
                        div {
                            role: "alert",
                            padding: "var(--space-sm) var(--space-md)",
                            background_color: "var(--error-bg)",
                            border_radius: "var(--radius-md)",
                            color: "var(--error-ink)",
                            font_size: "14px",
                            "{err}"
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

#[cfg(test)]
mod load_error_tests {
    use super::*;
    use dioxus::router::components::HistoryProvider;

    #[derive(Clone, PartialEq, Routable)]
    enum ErrorTestRoute {
        #[route("/")]
        ErrorTestPage {},
    }

    #[component]
    fn ErrorTestPage() -> Element {
        render_recipe_editor_unavailable()
    }

    #[test]
    fn unavailable_editor_has_generic_feedback_and_a_clear_safe_exit() {
        let mut dom = VirtualDom::new(|| {
            rsx! { HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<ErrorTestRoute> {}
            } }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Recipe editor unavailable"), "{html}");
        assert!(
            html.contains("may not exist or you may not have access"),
            "{html}"
        );
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(html.contains("href=\"/dashboard\""), "{html}");
        assert!(html.contains("Back to Dashboard"), "{html}");
        for technical_copy in [
            "server function",
            "recipe not found:",
            "Not authenticated",
            "details:",
        ] {
            assert!(!html.contains(technical_copy), "{html}");
        }
    }

    #[test]
    fn recipe_write_failures_keep_recovery_copy_without_backend_details() {
        let raw = ServerFnError::new("database error with internal details");
        for action in [
            RecipeWriteAction::Autosave,
            RecipeWriteAction::SaveDraft,
            RecipeWriteAction::Publish,
            RecipeWriteAction::Discard,
        ] {
            let message = recipe_write_feedback(action, &raw);
            assert!(!message.contains("database"));
            assert!(!message.contains("internal"));
            assert!(message.contains("try") || message.contains("Try"));
            assert!(
                recipe_write_feedback(action, &ServerFnError::new("Not authenticated"))
                    .contains("Sign in")
            );
        }
        assert!(recipe_write_feedback(RecipeWriteAction::Publish, &raw).contains("may have"));
    }

    #[test]
    fn autosave_feedback_is_separate_from_form_validation_and_cleared_on_success() {
        let source = include_str!("recipe_edit.rs");
        let autosave = source
            .split("// Two-second debounced autosave.")
            .nth(1)
            .unwrap()
            .split("let on_save_draft =")
            .next()
            .unwrap();
        assert!(autosave.contains("draft_save_error.set(None)"));
        assert!(autosave.contains("draft_save_error.set(Some"));
        assert!(!autosave.contains("form_error.set"));
        let manual_save = source
            .split("if draft_save_after_upload() {")
            .nth(1)
            .unwrap()
            .split("let publish_result =")
            .next()
            .unwrap();
        assert!(manual_save.contains("draft_save_error.set(Some"));
    }

    #[test]
    fn editor_write_guard_blocks_every_pending_write_kind() {
        for publishing in [false, true] {
            for saving_draft in [false, true] {
                for discarding in [false, true] {
                    assert_eq!(
                        editor_write_in_progress(publishing, saving_draft, discarding),
                        publishing || saving_draft || discarding
                    );
                }
            }
        }
    }

    #[test]
    fn editor_actions_and_autosave_use_the_shared_write_guard() {
        let source = include_str!("recipe_edit.rs");
        for marker in [
            "// Two-second debounced autosave.",
            "let on_save_draft = move",
            "let on_submit = move",
            "let on_cancel = move",
            "let on_discard = move",
        ] {
            let section = source.split(marker).nth(1).unwrap();
            let before_first_return = section.split("return;").next().unwrap();
            assert!(
                before_first_return.contains("editor_write_in_progress("),
                "Missing synchronous guard after {marker}"
            );
        }
        let publication_button = source
            .split("// ── Action buttons")
            .nth(1)
            .unwrap()
            .split("variant: ButtonVariant::Primary,")
            .nth(1)
            .unwrap()
            .split("onclick: on_submit")
            .next()
            .unwrap();
        assert!(publication_button.contains("editor_write_in_progress("));
    }
}

#[cfg(test)]
mod visibility_tests {
    use super::draft_allows_initial_visibility;

    #[test]
    fn only_unpublished_recipe_drafts_allow_initial_visibility_edits() {
        assert!(draft_allows_initial_visibility(None));
        assert!(!draft_allows_initial_visibility(Some(uuid::Uuid::new_v4())));
    }
}
