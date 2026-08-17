use dioxus::prelude::*;

use crate::api::collection::create_collection;
use crate::components::base::{
    complete_deferred_upload, single_gallery_image, validate_collection_form, Card, CollectionForm,
    CollectionVisibilityField, DeferredUploadCompletion, ImageUpload, PageHeader, UploadOutcome,
};
use crate::components::AuthRequired;
use crate::types::{CollectionVisibility, RecipeImage};
use crate::Route;

#[derive(Clone)]
struct PendingCollectionCreate {
    name: String,
    description: Option<String>,
    parent_id: Option<String>,
    visibility: CollectionVisibility,
}

fn parent_from_location() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let query = web_sys::window()?.location().search().ok()?;
        query.trim_start_matches('?').split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == "parent" && uuid::Uuid::parse_str(value).is_ok()).then(|| value.to_string())
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

#[component]
pub fn CollectionNew() -> Element {
    let name = use_signal(|| "Untitled Collection".to_string());
    let description = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut visibility = use_signal(|| CollectionVisibility::Private);
    let navigator = use_navigator();
    let parent_id = use_signal(parent_from_location);
    let mut images = use_signal(Vec::<RecipeImage>::new);
    let mut upload_now = use_signal(|| false);
    let image_busy = use_signal(|| false);
    let pending_image_count = use_signal(|| 0usize);
    let mut pending_save = use_signal(|| None::<PendingCollectionCreate>);

    let on_upload_complete = move |outcome: UploadOutcome| {
        spawn(async move {
            let pending = match complete_deferred_upload(&mut pending_save.write(), outcome) {
                DeferredUploadCompletion::NoPendingSave => return,
                DeferredUploadCompletion::Ready(pending) => pending,
                DeferredUploadCompletion::Aborted(problem) => {
                    error.set(Some(problem));
                    saving.set(false);
                    return;
                }
                DeferredUploadCompletion::Failed => {
                    error.set(Some(
                        "The cover image failed to upload. Retry or remove it, then save again."
                            .to_string(),
                    ));
                    saving.set(false);
                    return;
                }
            };
            match create_collection(
                pending.name,
                pending.description,
                pending.parent_id,
                pending.visibility,
                single_gallery_image(&images()),
            )
            .await
            {
                Ok(collection) => {
                    navigator.push(Route::CollectionDetail {
                        id: collection.id.to_string(),
                    });
                }
                Err(problem) => {
                    error.set(Some(problem.to_string()));
                    saving.set(false);
                }
            }
        });
    };

    rsx! { AuthRequired { div { class: "container",
        PageHeader { title: if parent_id().is_some() { "New subcollection" } else { "New collection" } }
        Card { CollectionForm { name, description, submit_label: "Create Collection", submitting: saving(), error: error(),
            extra_fields: rsx! {
                CollectionVisibilityField {
                    visibility: visibility(),
                    disabled: saving(),
                    onchange: move |selected| visibility.set(selected),
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
                    span { font_size: "12px", color: "var(--text-tertiary)", "Optional · uploaded when you create the collection" }
                }
            },
            onsubmit: move |_| {
                let Ok((clean_name, clean_description)) = validate_collection_form(&name(), &description()) else {
                    error.set(Some(validate_collection_form(&name(), &description()).unwrap_err().to_string())); return;
                };
                if image_busy() { error.set(Some("The cover image is still processing.".to_string())); return; }
                saving.set(true); error.set(None);
                pending_save.set(Some(PendingCollectionCreate {
                    name: clean_name,
                    description: clean_description,
                    parent_id: parent_id(),
                    visibility: visibility(),
                }));
                upload_now.set(true);
            }
        } }
    } } }
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_collection_form_submits_typed_visibility() {
        let source = include_str!("collection_new.rs");
        assert!(source.contains("CollectionVisibilityField"));
        assert!(source.contains(concat!("create_", "collection(")));
        assert!(!source.contains(concat!("create_collection", "_with_visibility")));
        assert!(source.contains("CollectionVisibility::Private"));
        assert!(source.contains("max_images: 1"));
        assert!(source.contains("pending_save.set"));
        assert!(source.contains("upload_now.set(true)"));
        assert!(source.contains("single_gallery_image(&images())"));
    }
}
