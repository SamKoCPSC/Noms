//! Recipe edit page.
//!
//! Loads an owner-only draft, autosaves it with revision guards, and publishes
//! the draft explicitly while leaving the current recipe visible.

use dioxus::prelude::*;

use crate::api::recipe::{
    create_edit_draft, discard_recipe_draft, get_recipe_draft_api, publish_recipe_draft,
    save_recipe_draft,
};
#[allow(unused_imports)]
use crate::api::upload::upload_image;
use crate::components::base::{
    Button, ButtonVariant, EditedRecipeImage, ImageEditorModal, ImageUpload, Input, LoadingSpinner,
    PageHeader, UploadOutcome,
};
use crate::components::AuthRequired;
use crate::pages::draft_save::draft_save_button_state;
use crate::pages::recipe_editor_form::{
    build_step_tree, can_indent_step, flatten_steps, format_step_photo_file_size,
    step_photo_display_name, StepForm,
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
    let mut pending_step_photo_edits = use_signal(Vec::<PendingStepPhotoEdit>::new);
    let mut step_photo_discard_capabilities =
        use_signal(std::collections::HashMap::<String, String>::new);
    let step_photo_upload_count = use_signal(|| 0usize);
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
            || is_saving()
            || is_manually_saving()
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
            if autosave_generation() != generation || is_saving() || is_manually_saving() {
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
                }
                Ok(_) => {}
                Err(error) if autosave_generation() == generation => {
                    form_error.set(Some(format!("Autosave failed: {error}")));
                }
                Err(_) => {}
            }
        });
    });

    #[allow(unused_variables)]
    let on_save_draft = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if draft_id().is_none() {
                form_error.set(Some(
                    "Draft is still loading. Try again in a moment.".to_string(),
                ));
                return;
            }
            if is_loading() || is_saving() || is_manually_saving() || busy() {
                return;
            }

            form_error.set(None);
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
                        #[cfg(target_arch = "wasm32")]
                        if let Some(destination) = draft_save_destination() {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(&destination);
                            }
                        }
                    }
                    Err(save_error) => {
                        form_error.set(Some(format!("Save draft failed: {save_error}")));
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
        if busy() || is_saving() || is_manually_saving() {
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
                    form_error.set(Some(error.to_string()));
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
            div { class: "container",
                PageHeader {
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
                                                                    } else {
                                                                        pending_deletes.write().push(removed.url);
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
                                                } else {
                                                    pending_deletes.write().push(photo.url);
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
                            variant: ButtonVariant::Secondary,
                            disabled: !draft_save_state.is_actionable()
                                || is_saving()
                                || is_discarding()
                                || busy()
                                || step_photo_upload_count() > 0,
                            onclick: on_save_draft,
                            "{draft_save_state.label()}"
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: is_saving() || is_manually_saving() || busy() || step_photo_upload_count() > 0,
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
                            disabled: is_saving() || is_manually_saving() || is_discarding() || busy() || step_photo_upload_count() > 0,
                            onclick: on_cancel,
                            "Cancel"
                        }
                        Button {
                            variant: ButtonVariant::Danger,
                            disabled: is_saving() || is_manually_saving() || is_discarding() || busy(),
                            onclick: on_discard,
                            if is_discarding() { "Discarding..." } else { "Discard Draft" }
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

#[cfg(test)]
mod visibility_tests {
    use super::draft_allows_initial_visibility;

    #[test]
    fn only_unpublished_recipe_drafts_allow_initial_visibility_edits() {
        assert!(draft_allows_initial_visibility(None));
        assert!(!draft_allows_initial_visibility(Some(uuid::Uuid::new_v4())));
    }
}
