use std::collections::HashSet;

use crate::components::base::{PageIcon, PageIconKind};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::collection::{
    get_collection_share_preview, get_owned_collection, load_all_my_collections, move_collection,
    update_collection, update_collection_visibility,
};
use crate::auth::account_binding::{
    account_completion_matches, ACCOUNT_COMPLETION_CHANGED_FEEDBACK,
};
use crate::auth::context::AuthContext;
use crate::components::base::{
    complete_deferred_upload, single_gallery_image, validate_collection_form, Button,
    ButtonVariant, Card, CollectionForm, CollectionVisibilityField, DeferredUploadCompletion,
    ImageUpload, LoadingSpinner, UploadOutcome,
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

#[derive(Clone, Copy, Debug)]
enum CollectionEditSaveStage {
    Details,
    Location,
    Sharing,
}

struct CollectionEditSaveFailure {
    message: String,
    unconfirmed: bool,
}

impl CollectionEditSaveStage {
    fn failure(self, problem: &ServerFnError) -> CollectionEditSaveFailure {
        // Only the explicit pre-write identity rejection proves this request did
        // not commit. A transport/error response otherwise cannot settle outcome.
        if let Some(feedback) = crate::auth::account_binding::account_error_feedback(problem) {
            let confirmed = match self {
                Self::Details => "",
                Self::Location | Self::Sharing => "Collection details were saved. ",
            };
            return CollectionEditSaveFailure {
                message: format!("{confirmed}{feedback}"),
                unconfirmed: false,
            };
        }
        let outcome = match self {
            Self::Details => "We couldn't confirm whether your collection details were saved.",
            Self::Location => {
                "Collection details were saved, but we couldn't confirm your location change."
            }
            Self::Sharing => {
                "Collection details were saved, but we couldn't confirm your sharing change."
            }
        };
        CollectionEditSaveFailure {
            message: format!("{outcome} Your entries are kept here. Check the saved collection in another tab before continuing; this save will not be retried automatically."),
            unconfirmed: true,
        }
    }
}

#[component]
fn CollectionEditorUnavailable(onretry: EventHandler<()>) -> Element {
    // Denied and failed loads must not expose transport/database details.
    rsx! { Card {
        div { role: "alert",
            h1 { class: "collection-editor-heading brand-title", "Collection editor unavailable" }
            p { "This collection may not exist or you may not have access to edit it. Try loading it again or return to your collections." }
            div { class: "collection-form-actions",
                Button { variant: ButtonVariant::Secondary, onclick: move |_| onretry.call(()), "Try again" }
                Link { class: "btn btn-secondary touch-target", to: Route::CollectionList {}, "Back to Collections" }
            }
        }
    } }
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
    let form_account = crate::auth::account_binding::use_form_account();
    let auth_context = use_context::<Signal<AuthContext>>();
    let resource_id = id.clone();
    let mut data = use_resource(move || {
        let id = resource_id.clone();
        async move {
            (
                get_owned_collection(id).await,
                load_all_my_collections().await,
            )
        }
    });
    let mut name = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut selected_parent = use_signal(String::new);
    let mut selected_visibility = use_signal(|| CollectionVisibility::Private);
    let mut initialized = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut save_unconfirmed = use_signal(|| false);
    let mut completion_account_changed = use_signal(|| false);
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

    rsx! { AuthRequired { div { class: "container collection-editor-page",
        match loaded {
            None => rsx! { LoadingSpinner {} },
            Some((Err(_), _)) | Some((_, Err(_))) => rsx! { CollectionEditorUnavailable { onretry: move |_| data.restart() } },
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
                                error.set(Some("Collection changes were not saved. Check the cover image status, then retry or remove it and save again.".to_string()));
                                saving.set(false); return;
                            }
                        };
                        let mut record_save_failure = move |stage: CollectionEditSaveStage, problem: ServerFnError| {
                            let failure = stage.failure(&problem);
                            save_unconfirmed.set(failure.unconfirmed);
                            error.set(Some(failure.message));
                            saving.set(false);
                        };
                        // Each accepted stage may outlive the current account.
                        // Stop the chain before applying local results, starting
                        // another stage or navigating away from the retained form.
                        let mut accept_completion = move |owner_id| {
                            if account_completion_matches(form_account(), auth_context.peek().current_user_id, owner_id) {
                                true
                            } else {
                                completion_account_changed.set(true);
                                error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                                saving.set(false);
                                false
                            }
                        };
                        let update = update_collection(form_account(),
                            collection_id.to_string(),
                            pending.name,
                            pending.description,
                            single_gallery_image(&images()),
                        ).await;
                        let updated = match update {
                            Ok(updated) => updated,
                            Err(problem) => { record_save_failure(CollectionEditSaveStage::Details, problem); return; }
                        };
                        if !accept_completion(updated.owner_id) { return; }
                        name.set(updated.name);
                        description.set(updated.description.unwrap_or_default());
                        if pending.desired_parent != pending.current_parent {
                            let moved = match move_collection(form_account(), collection_id.to_string(), pending.desired_parent).await {
                                Ok(moved) => moved,
                                Err(problem) => {
                                    // Keep the intended destination after a lost acknowledgement.
                                    record_save_failure(CollectionEditSaveStage::Location, problem); return;
                                }
                            };
                            if !accept_completion(moved.owner_id) { return; }
                        }
                        if pending.desired_visibility != pending.current_visibility {
                            match update_collection_visibility(form_account(), collection_id.to_string(), pending.desired_visibility).await {
                                Ok(updated_visibility) => {
                                    if !accept_completion(updated_visibility.owner_id) { return; }
                                    selected_visibility.set(updated_visibility.visibility);
                                }
                                Err(problem) => { record_save_failure(CollectionEditSaveStage::Sharing, problem); return; }
                            }
                        }
                        navigator.push(Route::CollectionDetail { id: collection_id.to_string() });
                    });
                };
                rsx! {
                    nav { class: "collection-editor-breadcrumb", aria_label: "Breadcrumb",
                        Link { to: Route::CollectionList {}, "Collections" }
                        span { aria_hidden: "true", " / " }
                        Link { to: Route::CollectionDetail { id: collection_id.to_string() }, "{contents.collection.name}" }
                        span { aria_hidden: "true", " / " } span { "Edit" }
                    }
                    h1 { class: "collection-editor-heading brand-title", PageIcon { kind: PageIconKind::Collections } span { "Edit collection" } }
                    p { class: "collection-editor-intro", "Update your album’s details, cover and place in your library." }
                    CollectionForm { name, description, submit_label: "Save changes", submitting: saving(), error: error(),
                        submit_disabled: save_unconfirmed() || completion_account_changed(),
                        oncancel: move |_| { navigator.push(Route::CollectionDetail { id: collection_id.to_string() }); },
                        extra_fields: rsx! {
                            section { class: "collection-form-panel neumo-raised",
                            h2 { "Location & sharing" }
                            label { class: "collection-form-field",
                                span { "Inside collection" }
                                select { class: "neumo-inset input", value: "{selected_parent}", disabled: saving(), onchange: move |event| selected_parent.set(event.value()),
                                    option { value: "", "Your collections (top level)" }
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
                            if selected_visibility() != CollectionVisibility::Private {
                                match share_preview.read().as_ref() {
                                    Some(Ok(preview)) if preview.hidden_recipe_count > 0 => rsx! {
                                        p { class: "collection-form-hint", role: "status",
                                            "{preview.hidden_recipe_count} of {preview.total_recipe_count} recipes will be hidden from visitors at this visibility."
                                        }
                                    },
                                    Some(Err(_)) => rsx! {
                                        p { class: "collection-form-error", role: "alert", "Could not preview sharing. Check your connection and try selecting the sharing setting again." }
                                    },
                                    _ => rsx! {},
                                }
                            }
                            }
                            section { class: "collection-form-panel neumo-raised",
                                h2 { "Cover image" }
                                p { class: "collection-form-hint", "Choose a photo to make this album easy to spot." }
                                ImageUpload {
                                    form_account,
                                    images,
                                    upload_now,
                                    busy: image_busy,
                                    pending_count: pending_image_count,
                                    purpose: crate::api::upload::UploadPurpose::CollectionCover,
                                    max_images: 1,
                                    on_remove_existing: move |url: String| images.write().retain(|image| image.url != url),
                                    on_upload_complete,
                                }
                                p { class: "collection-form-hint", "Optional · uploaded when you save" }
                            }
                        },
                        onsubmit: move |_| {
                            if saving() || save_unconfirmed() || completion_account_changed() { return; }
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
                    }
                    if save_unconfirmed() || completion_account_changed() {
                        p { class: "collection-form-hint",
                            Link {
                                class: "btn btn-secondary touch-target",
                                to: Route::CollectionDetail { id: collection_id.to_string() },
                                new_tab: true,
                                "Check saved collection"
                            }
                        }
                    }
                }
            }
        }
    } } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::router::components::HistoryProvider;

    #[test]
    fn collection_edit_checks_each_completion_before_followup_or_navigation() {
        let source = include_str!("collection_edit.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(source.contains("account_completion_matches("));
        assert!(source.contains("ACCOUNT_COMPLETION_CHANGED_FEEDBACK"));
        assert!(source.contains("completion_account_changed.set(true)"));
        assert_eq!(source.matches("if !accept_completion(").count(), 3);
        let details = source
            .find("if !accept_completion(updated.owner_id)")
            .unwrap();
        assert!(details < source.find("name.set(updated.name)").unwrap());
        let location = source
            .find("if !accept_completion(moved.owner_id)")
            .unwrap();
        assert!(location < source.find("if pending.desired_visibility").unwrap());
        let sharing = source
            .find("if !accept_completion(updated_visibility.owner_id)")
            .unwrap();
        assert!(
            sharing
                < source
                    .find("navigator.push(Route::CollectionDetail")
                    .unwrap()
        );
        assert!(source.contains("save_unconfirmed() || completion_account_changed()"));
    }

    #[test]
    fn collection_edit_recovery_blocks_blind_retries_but_preserves_local_inputs() {
        let source = include_str!("collection_edit.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(source.contains("submit_disabled: save_unconfirmed()"));
        assert!(source.contains(
            "if saving() || save_unconfirmed() || completion_account_changed() { return; }"
        ));
        assert!(!source.contains("selected_parent.set(pending.current_parent"));
        assert!(source.contains("new_tab: true"));
        assert!(source.contains("Check saved collection"));
    }

    #[test]
    fn collection_edit_recovery_records_the_first_unconfirmed_stage() {
        let source = include_str!("collection_edit.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for stage in ["Details", "Location", "Sharing"] {
            assert!(source.contains(&format!(
                "record_save_failure(CollectionEditSaveStage::{stage}, problem)"
            )));
        }
        assert!(!source.contains(".await.is_err()"));
        assert!(!source.contains("Err(_) =>"));
    }

    #[test]
    fn collection_edit_recovery_distinguishes_confirmed_details_from_unknown_outcomes() {
        for (stage, expected) in [
            (
                CollectionEditSaveStage::Details,
                "We couldn't confirm whether your collection details were saved.",
            ),
            (
                CollectionEditSaveStage::Location,
                "Collection details were saved, but we couldn't confirm your location change.",
            ),
            (
                CollectionEditSaveStage::Sharing,
                "Collection details were saved, but we couldn't confirm your sharing change.",
            ),
        ] {
            let failure = stage.failure(&ServerFnError::new("private database/transport details"));
            assert!(failure.unconfirmed);
            assert!(failure.message.starts_with(expected), "{}", failure.message);
            assert!(failure.message.contains("Your entries are kept here"));
            assert!(!failure
                .message
                .contains("private database/transport details"));
            assert!(!failure.message.contains("try saving again"));
        }
    }

    #[test]
    fn collection_edit_recovery_allows_retry_only_for_explicit_prewrite_account_rejection() {
        for stage in [
            CollectionEditSaveStage::Details,
            CollectionEditSaveStage::Location,
            CollectionEditSaveStage::Sharing,
        ] {
            let failure = stage.failure(&ServerFnError::new(
                crate::auth::account_binding::ACCOUNT_CHANGED_MESSAGE,
            ));
            assert!(!failure.unconfirmed);
            assert!(failure
                .message
                .contains(crate::auth::account_binding::ACCOUNT_CHANGED_FEEDBACK));
            assert_eq!(
                failure
                    .message
                    .starts_with("Collection details were saved."),
                !matches!(stage, CollectionEditSaveStage::Details)
            );
        }
    }

    #[derive(Clone, PartialEq, Routable)]
    enum ErrorTestRoute {
        #[route("/")]
        ErrorTestPage {},
    }

    #[component]
    fn ErrorTestPage() -> Element {
        rsx! { CollectionEditorUnavailable { onretry: |_| {} } }
    }

    #[test]
    fn unavailable_collection_editor_offers_retry_and_safe_navigation() {
        let mut dom = VirtualDom::new(|| {
            rsx! { HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<ErrorTestRoute> {}
            } }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        for expected in [
            "Collection editor unavailable",
            "may not exist",
            "role=\"alert\"",
            "Try again",
            "Back to Collections",
            "href=\"/collections\"",
        ] {
            assert!(html.contains(expected), "{html}");
        }
        assert!(!html.contains("server function"), "{html}");
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
