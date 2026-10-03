use crate::components::base::{PageIcon, PageIconKind};
use dioxus::prelude::*;

use crate::api::collection::create_collection;
use crate::auth::account_binding::{
    account_completion_matches, ACCOUNT_COMPLETION_CHANGED_FEEDBACK,
};
use crate::auth::context::AuthContext;
use crate::components::base::{
    complete_deferred_upload, single_gallery_image, validate_collection_form, CollectionForm,
    CollectionVisibilityField, DeferredUploadCompletion, ImageUpload, UploadOutcome,
};
use crate::components::AuthRequired;
use crate::types::{CollectionVisibility, RecipeImage};
use crate::Route;

const UNCONFIRMED_COLLECTION_CREATION_MESSAGE: &str = "We couldn't confirm whether your collection was created. Your entries are kept here. Check Collections in another tab before starting a new collection to avoid duplicates.";

fn collection_creation_submit_blocked(saving: bool, unconfirmed: bool, created: bool) -> bool {
    saving || unconfirmed || created
}

#[derive(Clone)]
struct PendingCollectionCreate {
    name: String,
    description: Option<String>,
    parent_id: Option<String>,
    visibility: CollectionVisibility,
}

#[component]
pub fn CollectionNew(parent: Option<uuid::Uuid>) -> Element {
    let form_account = crate::auth::account_binding::use_form_account();
    let auth_context = use_context::<Signal<AuthContext>>();
    // Route props preserve the parent through SSR, hydration and query-only
    // navigation; do not initialize it once from browser-only location state.
    let name = use_signal(|| "Untitled Collection".to_string());
    let description = use_signal(String::new);
    let mut saving = use_signal(|| false);
    // A failed response may follow a committed insert. Keep this visit's
    // entries, but do not issue another non-idempotent creation blindly.
    let mut creation_unconfirmed = use_signal(|| false);
    // A confirmed response can arrive after account replacement. Remember its
    // destination without navigating or allowing a duplicate insert on recovery.
    let mut created_collection = use_signal(|| None::<uuid::Uuid>);
    let mut error = use_signal(|| None::<String>);
    let mut visibility = use_signal(|| CollectionVisibility::Private);
    let navigator = use_navigator();
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
                        "Collection changes were not saved. Check the cover image status, then retry or remove it and save again."
                            .to_string(),
                    ));
                    saving.set(false);
                    return;
                }
            };
            match create_collection(
                form_account(),
                pending.name,
                pending.description,
                pending.parent_id,
                pending.visibility,
                single_gallery_image(&images()),
            )
            .await
            {
                Ok(collection) => {
                    created_collection.set(Some(collection.id));
                    if !account_completion_matches(
                        form_account(),
                        auth_context.peek().current_user_id,
                        collection.owner_id,
                    ) {
                        error.set(Some(ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));
                        saving.set(false);
                        return;
                    }
                    navigator.push(Route::CollectionDetail {
                        id: collection.id.to_string(),
                    });
                }
                Err(problem) => {
                    if let Some(feedback) =
                        crate::auth::account_binding::account_error_feedback(&problem)
                    {
                        // This explicit account rejection occurs before writes.
                        error.set(Some(feedback.to_string()));
                    } else {
                        creation_unconfirmed.set(true);
                        error.set(Some(UNCONFIRMED_COLLECTION_CREATION_MESSAGE.to_string()));
                    }
                    saving.set(false);
                }
            }
        });
    };

    rsx! { AuthRequired { div { class: "container collection-editor-page",
        nav { class: "collection-editor-breadcrumb", aria_label: "Breadcrumb", Link { to: Route::CollectionList {}, "Collections" } span { aria_hidden: "true", " / " } span { if parent.is_some() { "New subcollection" } else { "New collection" } } }
        h1 { class: "collection-editor-heading brand-title", PageIcon { kind: PageIconKind::Collections } span { if parent.is_some() { "New subcollection" } else { "New collection" } } }
        p { class: "collection-editor-intro", "Start an album for recipes you want to keep together." }
        CollectionForm { name, description, submit_label: "Create collection", submitting: saving(),
            submit_disabled: collection_creation_submit_blocked(false, creation_unconfirmed(), created_collection().is_some()), error: error(),
            oncancel: move |_| { navigator.push(parent.map(|id| Route::CollectionDetail { id: id.to_string() }).unwrap_or(Route::CollectionList {})); },
            extra_fields: rsx! {
                section { class: "collection-form-panel neumo-raised",
                h2 { "Location & sharing" }
                p { class: "collection-form-hint",
                    if parent.is_some() { "This album will be added inside the collection you came from." }
                    else { "This album will appear at the top level of your collections." }
                }
                CollectionVisibilityField {
                    visibility: visibility(),
                    disabled: saving(),
                    onchange: move |selected| visibility.set(selected),
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
                    p { class: "collection-form-hint", "Optional · uploaded when you create the collection" }
                }
            },
            onsubmit: move |_| {
                if collection_creation_submit_blocked(saving(), creation_unconfirmed(), created_collection().is_some()) { return; }
                let Ok((clean_name, clean_description)) = validate_collection_form(&name(), &description()) else {
                    error.set(Some(validate_collection_form(&name(), &description()).unwrap_err().to_string())); return;
                };
                if image_busy() { error.set(Some("The cover image is still processing.".to_string())); return; }
                saving.set(true); error.set(None);
                pending_save.set(Some(PendingCollectionCreate {
                    name: clean_name,
                    description: clean_description,
                    parent_id: parent.map(|id| id.to_string()),
                    visibility: visibility(),
                }));
                upload_now.set(true);
            }
        }
        if let Some(collection_id) = created_collection() {
            p { class: "collection-form-hint",
                Link { to: Route::CollectionDetail { id: collection_id.to_string() }, new_tab: true,
                    class: "btn btn-secondary touch-target", "Check created collection" }
            }
        } else if creation_unconfirmed() {
            p { class: "collection-form-hint",
                Link { to: Route::CollectionList {}, new_tab: true,
                    class: "btn btn-secondary touch-target", "Check Collections" }
            }
        }
    } } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::context::AuthContext;
    use crate::Route;
    use dioxus::router::components::HistoryProvider;
    use dioxus_core::NoOpMutations;

    const PARENT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn completed_collection_creation_blocks_duplicate_submission_after_account_recovery() {
        for (saving, unconfirmed, created, blocked) in [
            (false, false, false, false),
            (true, false, false, true),
            (false, true, false, true),
            (false, false, true, true),
        ] {
            assert_eq!(
                collection_creation_submit_blocked(saving, unconfirmed, created),
                blocked
            );
        }
    }

    #[test]
    fn collection_creation_checks_completion_account_before_navigation() {
        let source = include_str!("collection_new.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let completion = source.split("Ok(collection) => {").nth(1).unwrap();
        assert!(
            completion
                .find("created_collection.set(Some(collection.id))")
                .unwrap()
                < completion.find("account_completion_matches(").unwrap()
        );
        assert!(
            completion.find("account_completion_matches(").unwrap()
                < completion.find("navigator.push(").unwrap()
        );
        assert!(source.contains("ACCOUNT_COMPLETION_CHANGED_FEEDBACK"));
        assert!(source.contains("created_collection().is_some()"));
        assert!(source.contains("Check created collection"));
    }

    #[component]
    fn CollectionCreationTestRoot(url: String) -> Element {
        use_context_provider(|| {
            Signal::new(AuthContext {
                is_authenticated: true,
                ..Default::default()
            })
        });
        rsx! {
            HistoryProvider {
                history: move |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path(&url)) as std::rc::Rc<dyn History>,
                Router::<Route> {}
            }
        }
    }

    fn render_creation_route(url: &str) -> String {
        let mut dom = VirtualDom::new_with_props(
            CollectionCreationTestRoot,
            CollectionCreationTestRootProps {
                url: url.to_string(),
            },
        );
        dom.rebuild(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn collection_creation_route_preserves_parent_without_a_browser() {
        let url = "/collections/new?parent=550e8400-e29b-41d4-a716-446655440000";
        let route: Route = url.parse().expect("collection creation route should parse");
        assert_eq!(route.to_string(), url);
        assert_eq!(
            route,
            Route::CollectionNew {
                parent: Some(PARENT_ID.parse().unwrap())
            }
        );
    }

    #[test]
    fn missing_empty_or_invalid_parent_remains_top_level() {
        for url in [
            "/collections/new",
            "/collections/new?parent=",
            "/collections/new?parent=not-a-uuid",
        ] {
            assert_eq!(
                url.parse::<Route>().unwrap(),
                Route::CollectionNew { parent: None }
            );
            let html = render_creation_route(url);
            assert!(
                html.contains("This album will appear at the top level"),
                "{html}"
            );
            assert!(!html.contains("New subcollection"), "{html}");
        }
    }

    #[test]
    fn server_rendered_subcollection_form_retains_parent_context() {
        let html = render_creation_route(&format!("/collections/new?parent={PARENT_ID}"));
        assert!(
            html.contains("<span>New subcollection</span></h1>"),
            "{html}"
        );
        assert!(
            html.contains("This album will be added inside the collection you came from."),
            "{html}"
        );
        assert!(
            !html.contains("This album will appear at the top level"),
            "{html}"
        );
    }

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

    #[test]
    fn collection_creation_recovery_blocks_blind_retries_and_keeps_a_separate_read_path() {
        let source = include_str!("collection_new.rs");
        let component = source.split("#[cfg(test)]").next().unwrap();
        assert!(component.contains("let mut creation_unconfirmed"));
        assert!(component.contains("submit_disabled: collection_creation_submit_blocked(false, creation_unconfirmed(), created_collection().is_some())"));
        assert!(component.contains("if collection_creation_submit_blocked(saving(), creation_unconfirmed(), created_collection().is_some())"));
        assert!(component.contains("creation_unconfirmed.set(true)"));
        assert!(component.contains("new_tab: true"));
        assert!(component.contains("Check Collections"));
        assert!(!component.contains("problem.to_string()"));
    }
}
