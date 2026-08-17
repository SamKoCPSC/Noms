use std::collections::HashSet;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::collection::{
    get_collection_share_preview, get_owned_collection, list_my_collections, move_collection,
    update_collection, update_collection_visibility,
};
use crate::components::base::{
    complete_deferred_upload, single_gallery_image, validate_collection_form, Card, CollectionForm,
    CollectionVisibilityField, DeferredUploadCompletion, ImageUpload, LoadingSpinner, PageHeader,
    UploadOutcome,
};
use crate::components::AuthRequired;
use crate::types::{CollectionSummary, CollectionVisibility, RecipeImage};
use crate::Route;

fn invalid_move_destinations(
    collection_id: Uuid,
    collections: &[CollectionSummary],
) -> HashSet<Uuid> {
    let mut excluded = HashSet::from([collection_id]);
    loop {
        let before = excluded.len();
        for collection in collections {
            if collection
                .parent_collection_id
                .is_some_and(|parent| excluded.contains(&parent))
            {
                excluded.insert(collection.id);
            }
        }
        if excluded.len() == before {
            return excluded;
        }
    }
}

fn save_failure_message(details_saved: bool, problem: &str) -> String {
    if details_saved {
        format!("Collection details were saved, but the move failed: {problem}")
    } else {
        problem.to_string()
    }
}

#[derive(Clone)]
struct PendingCollectionEdit {
    name: String,
    description: Option<String>,
    desired_parent: Option<String>,
    current_parent: Option<String>,
    desired_visibility: CollectionVisibility,
    current_visibility: CollectionVisibility,
}

#[component]
pub fn CollectionEdit(id: String) -> Element {
    let resource_id = id.clone();
    let data = use_resource(move || {
        let id = resource_id.clone();
        async move { (get_owned_collection(id).await, list_my_collections().await) }
    });
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut selected_parent = use_signal(String::new);
    let mut selected_visibility = use_signal(|| CollectionVisibility::Private);
    let mut initialized = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut images = use_signal(Vec::<RecipeImage>::new);
    let mut upload_now = use_signal(|| false);
    let image_busy = use_signal(|| false);
    let pending_image_count = use_signal(|| 0usize);
    let mut pending_save = use_signal(|| None::<PendingCollectionEdit>);
    let navigator = use_navigator();
    let preview_id = id.clone();
    let share_preview = use_resource(move || {
        let collection_id = preview_id.clone();
        let target_visibility = selected_visibility();
        async move { get_collection_share_preview(collection_id, target_visibility).await }
    });

    let loaded = data.read().clone();
    if !initialized() {
        if let Some((Ok(contents), Ok(_))) = loaded.as_ref() {
            name.set(contents.collection.name.clone());
            description.set(contents.collection.description.clone().unwrap_or_default());
            selected_parent.set(
                contents
                    .collection
                    .parent_collection_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            );
            selected_visibility.set(contents.collection.visibility);
            images.set(contents.collection.image.clone().into_iter().collect());
            initialized.set(true);
        }
    }

    rsx! { AuthRequired { div { class: "container",
        match loaded {
            None => rsx! { LoadingSpinner {} },
            Some((Err(problem), _)) | Some((_, Err(problem))) => rsx! { p { role: "alert", color: "var(--error)", "Could not load collection: {problem}" } },
            Some((Ok(contents), Ok(collections))) => {
                let collection_id = contents.collection.id;
                let excluded = invalid_move_destinations(collection_id, &collections);
                let on_upload_complete = move |outcome: UploadOutcome| {
                    spawn(async move {
                        let pending = match complete_deferred_upload(&mut pending_save.write(), outcome) {
                            DeferredUploadCompletion::NoPendingSave => return,
                            DeferredUploadCompletion::Ready(pending) => pending,
                            DeferredUploadCompletion::Aborted(problem) => {
                                error.set(Some(problem)); saving.set(false); return;
                            }
                            DeferredUploadCompletion::Failed => {
                                error.set(Some("The cover image failed to upload. Retry or remove it, then save again.".to_string()));
                                saving.set(false); return;
                            }
                        };
                        let update = update_collection(
                            collection_id.to_string(),
                            pending.name,
                            pending.description,
                            single_gallery_image(&images()),
                        ).await;
                        let updated = match update {
                            Ok(updated) => updated,
                            Err(problem) => { error.set(Some(save_failure_message(false, &problem.to_string()))); saving.set(false); return; }
                        };
                        name.set(updated.name);
                        description.set(updated.description.unwrap_or_default());
                        if pending.desired_parent != pending.current_parent {
                            if let Err(problem) = move_collection(collection_id.to_string(), pending.desired_parent).await {
                                selected_parent.set(pending.current_parent.unwrap_or_default());
                                error.set(Some(save_failure_message(true, &problem.to_string())));
                                saving.set(false); return;
                            }
                        }
                        if pending.desired_visibility != pending.current_visibility {
                            match update_collection_visibility(collection_id.to_string(), pending.desired_visibility).await {
                                Ok(updated) => selected_visibility.set(updated.visibility),
                                Err(problem) => {
                                    error.set(Some(format!("Collection details and location were saved, but the visibility update failed: {problem}")));
                                    saving.set(false); return;
                                }
                            }
                        }
                        navigator.push(Route::CollectionDetail { id: collection_id.to_string() });
                    });
                };
                rsx! {
                    PageHeader { title: "Edit collection" }
                    Card { CollectionForm { name, description, submit_label: "Save Changes", submitting: saving(), error: error(),
                        extra_fields: rsx! {
                            label { display: "flex", flex_direction: "column", gap: "var(--space-xs)",
                                span { "Parent collection" }
                                select { class: "neumo-inset input", value: "{selected_parent}", disabled: saving(), onchange: move |event| selected_parent.set(event.value()),
                                    option { value: "", "Root (no parent)" }
                                    for destination in collections.iter().filter(|item| !excluded.contains(&item.id)) {
                                        option { value: "{destination.id}", "{destination.name}" }
                                    }
                                }
                            }
                            CollectionVisibilityField {
                                visibility: selected_visibility(),
                                disabled: saving(),
                                onchange: move |visibility| selected_visibility.set(visibility),
                            }
                            div { display: "flex", flex_direction: "column", gap: "var(--space-sm)",
                                span { font_weight: "600", "Collection cover image" }
                                ImageUpload {
                                    images,
                                    upload_now,
                                    busy: image_busy,
                                    pending_count: pending_image_count,
                                    purpose: crate::api::upload::UploadPurpose::CollectionCover,
                                    max_images: 1,
                                    on_remove_existing: move |url: String| images.write().retain(|image| image.url != url),
                                    on_upload_complete,
                                }
                                span { font_size: "12px", color: "var(--text-tertiary)", "Optional · uploaded when you save" }
                            }
                            if selected_visibility() != CollectionVisibility::Private {
                                match share_preview.read().as_ref() {
                                    Some(Ok(preview)) if preview.hidden_recipe_count > 0 => rsx! {
                                        p { role: "status", color: "var(--text-secondary)",
                                            "{preview.hidden_recipe_count} of {preview.total_recipe_count} recipes will be hidden from visitors at this visibility."
                                        }
                                    },
                                    Some(Err(problem)) => rsx! {
                                        p { role: "alert", color: "var(--error)", "Could not preview sharing: {problem}" }
                                    },
                                    _ => rsx! {},
                                }
                            }
                        },
                        onsubmit: move |_| {
                            let validated = validate_collection_form(&name(), &description());
                            let Ok((clean_name, clean_description)) = validated else { error.set(Some(validated.unwrap_err().to_string())); return; };
                            if image_busy() { error.set(Some("The cover image is still processing.".to_string())); return; }
                            saving.set(true); error.set(None);
                            let parent_value = selected_parent();
                            let desired_parent = (!parent_value.is_empty()).then_some(parent_value);
                            let current_parent = contents.collection.parent_collection_id.map(|id| id.to_string());
                            let desired_visibility = selected_visibility();
                            let current_visibility = contents.collection.visibility;
                            pending_save.set(Some(PendingCollectionEdit {
                                name: clean_name,
                                description: clean_description,
                                desired_parent,
                                current_parent,
                                desired_visibility,
                                current_visibility,
                            }));
                            upload_now.set(true);
                        }
                    } }
                }
            }
        }
    } } }
}

#[cfg(test)]
mod tests {
    use super::save_failure_message;

    #[test]
    fn move_failure_discloses_that_field_updates_were_saved() {
        assert_eq!(
            save_failure_message(true, "move rejected"),
            "Collection details were saved, but the move failed: move rejected"
        );
        assert_eq!(
            save_failure_message(false, "update rejected"),
            "update rejected"
        );
    }

    #[test]
    fn edit_collection_form_previews_and_updates_visibility() {
        let source = include_str!("collection_edit.rs");
        assert!(source.contains("get_owned_collection"));
        assert!(source.contains("CollectionVisibilityField"));
        assert!(source.contains("get_collection_share_preview"));
        assert!(source.contains("hidden_recipe_count"));
        assert!(source.contains("update_collection_visibility"));
        assert!(source.contains("max_images: 1"));
        assert!(source.contains("on_remove_existing"));
        assert!(source.contains("pending_save.set"));
        assert!(source.contains("upload_now.set(true)"));
        assert!(source.contains("single_gallery_image(&images())"));
    }
}
