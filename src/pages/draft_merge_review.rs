use dioxus::prelude::*;

use crate::components::base::{EditedRecipeImage, SharedModal};
use crate::types::{RecipeDraftView, RecipeEditorState, RecipeImage, RecipeStep};

use super::draft_merge::{MergeChoice, MergeChoices, MergePlan, RecipeField};
use super::draft_save::{
    draft_revision_conflict, recipe_write_feedback, DraftCheckpoint, DraftSync, RecipeWriteAction,
};

#[derive(Clone, PartialEq)]
struct Review {
    base: DraftCheckpoint,
    yours: RecipeEditorState,
    saved: RecipeDraftView,
    choices: MergeChoices,
}

impl Review {
    fn new(base: DraftCheckpoint, yours: RecipeEditorState, saved: RecipeDraftView) -> Self {
        let choices =
            MergePlan::new(&base.editor, yours.clone(), saved.editor.clone()).suggested_choices();
        Self {
            base,
            yours,
            saved,
            choices,
        }
    }

    fn plan(&self) -> MergePlan {
        MergePlan::new(
            &self.base.editor,
            self.yours.clone(),
            self.saved.editor.clone(),
        )
    }

    fn submission(&self) -> Option<(i64, RecipeEditorState)> {
        self.plan()
            .resolve(&self.choices)
            .map(|editor| (self.saved.revision, editor))
    }
}

#[component]
pub(super) fn DraftConflictReview(
    account: Option<uuid::Uuid>,
    draft_id: String,
    mut sync: DraftSync,
    read_local: Callback<(), RecipeEditorState>,
    ready: Callback<(), bool>,
    blocked: bool,
    pending_photos: usize,
    on_upload_pending: EventHandler<()>,
    on_merged: EventHandler<RecipeDraftView>,
) -> Element {
    let mut review = use_signal(|| None::<Review>);
    let mut pending = use_signal(|| false);
    let mut feedback = use_signal(|| None::<String>);
    let fetch_id = draft_id.clone();
    let open = move |_| {
        if !ready.call(()) || *sync.in_flight.peek() > 0 || sync.reviewing() {
            return;
        }
        let Some(base) = sync.checkpoint.peek().clone() else {
            return;
        };
        let yours = read_local.call(());
        sync.reviewing.set(true);
        pending.set(true);
        feedback.set(None);
        let id = fetch_id.clone();
        spawn(async move {
            match crate::api::recipe::get_recipe_draft_api(account, id).await {
                Ok(saved) => review.set(Some(Review::new(base, yours, saved))),
                Err(error) => feedback.set(Some(read_feedback(&error).into())),
            }
            pending.set(false);
        });
    };
    let close = move |_| {
        if !*pending.peek() {
            review.set(None);
            feedback.set(None);
            sync.reviewing.set(false);
        }
    };
    let save = move |_| {
        if *pending.peek() || !ready.call(()) {
            return;
        }
        let Some(current) = review.peek().clone() else {
            return;
        };
        if current.yours != read_local.call(())
            || sync.checkpoint.peek().as_ref() != Some(&current.base)
        {
            feedback.set(Some("Your entries changed while this review was open. Keep editing, then review again to include them.".into()));
            return;
        }
        let Some((expected_revision, merged)) = current.submission() else {
            return;
        };
        pending.set(true);
        feedback.set(None);
        let id = draft_id.clone();
        spawn(async move {
            // Only explicit confirmation may save against a fetched revision.
            // Never rebase and resubmit automatically, including after a timeout.
            match crate::api::recipe::save_recipe_draft(
                account,
                id.clone(),
                expected_revision,
                merged,
            )
            .await
            {
                Ok(saved) => {
                    sync.acknowledge(&saved);
                    on_merged.call(saved);
                    sync.reviewing.set(false);
                    sync.conflict.set(false);
                }
                Err(error) if draft_revision_conflict(&error) => {
                    match crate::api::recipe::get_recipe_draft_api(account, id).await {
                        Ok(saved) => {
                            review.set(Some(Review::new(current.base, current.yours, saved)));
                            feedback.set(Some("Another change was saved during your review. Review the updated version and choose again.".into()));
                        }
                        Err(error) => {
                            // Invalidate the obsolete review. Retry starts with
                            // a fresh read; it cannot resubmit these choices.
                            review.set(None);
                            feedback.set(Some(read_feedback(&error).into()));
                        }
                    }
                }
                Err(error) => feedback.set(Some(
                    recipe_write_feedback(RecipeWriteAction::SaveDraft, &error).into(),
                )),
            }
            pending.set(false);
        });
    };
    let current = review();
    let changed_locally = current
        .as_ref()
        .is_some_and(|current| current.yours != read_local.call(()));

    rsx! {
        aside { class: "draft-conflict-notice", role: "alert",
            strong { "Newer changes were saved elsewhere" }
            p { "Autosave is paused. Your entries are still here. Review and merge the changes to continue saving." }
            if pending_photos > 0 {
                p { "Upload or remove your selected gallery photos before reviewing. Your selections remain on this page." }
                button { r#type: "button", class: "btn btn-secondary touch-target", disabled: blocked || sync.reviewing(),
                    onclick: move |_| on_upload_pending.call(()), "Upload pending photos" }
            }
            button { r#type: "button", class: "btn btn-primary touch-target", disabled: !ready.call(()) || *sync.in_flight.read() > 0 || sync.reviewing(), onclick: open, "Review changes" }
        }
        if sync.reviewing() {
            MergeReviewDialog { review, pending, feedback, changed_locally, ready: ready.call(()), on_close: close, on_save: save }

        }
    }
}

#[component]
fn MergeReviewDialog(
    mut review: Signal<Option<Review>>,
    pending: Signal<bool>,
    feedback: Signal<Option<String>>,
    changed_locally: bool,
    ready: bool,
    on_close: EventHandler<()>,
    on_save: EventHandler<MouseEvent>,
) -> Element {
    let current = review();
    let plan = current.as_ref().map(Review::plan);
    let resolved = current
        .as_ref()
        .zip(plan.as_ref())
        .is_some_and(|(review, plan)| plan.resolve(&review.choices).is_some());
    rsx! {
            SharedModal { aria_labelledby: "draft-merge-title", focus_selector: "#draft-merge-back", pending: pending(), on_dismiss: on_close,
                div { class: "modal-card draft-merge-review", onclick: move |event| event.stop_propagation(),
                    h2 { id: "draft-merge-title", "Review draft changes" }
                    p { "Independent changes are selected for you. Choose which version to keep wherever both versions changed differently. You can change any selection before saving." }
                    p { class: "recipe-editor__hint", "Ingredients, steps, equipment, photos and tags are selected as complete lists." }
                    if pending() { p { role: "status", "Loading or saving your draft…" } }
                    if let Some(message) = feedback() { p { role: "alert", "{message}" } }
                    if changed_locally { p { role: "alert", "Your entries changed. Keep editing, then reopen this review before saving." } }
                    if let Some(plan) = plan {
                        if plan.changes.is_empty() {
                            p { "Both versions have the same recipe content. Confirm to continue saving from the latest draft." }
                        }
                        for change in plan.changes {
                            fieldset { class: "draft-merge-field", disabled: pending(),
                                legend { "{change.field.label()}" }
                                p { class: "recipe-editor__hint",
                                    if change.suggested.is_none() { "Changed in both versions. Choose which to keep." }
                                    else if change.suggested == Some(MergeChoice::Yours) { "Only your version changed. Your changes are selected." }
                                    else { "Only the saved version changed. Saved changes are selected." }
                                    if change.field.is_list() { " This choice keeps the complete list, including its order." }
                                }
                                div { class: "draft-merge-versions",
                                    for (choice, label, editor) in [(MergeChoice::Yours, "Your changes", plan.yours.clone()), (MergeChoice::Saved, "Saved changes", plan.saved.clone())] {
                                        label { class: "draft-merge-version",
                                            input { r#type: "radio", name: "draft-merge-{change.field as usize}", value: "{choice:?}",
                                                checked: current.as_ref().is_some_and(|review| review.choices[change.field as usize] == Some(choice)),
                                                onchange: move |_| { if !*pending.peek() { if let Some(current) = review.write().as_mut() { current.choices[change.field as usize] = Some(choice); } } }
                                            }
                                            strong { "{label}" }
                                            FieldContent { field: change.field, editor }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if current.is_some() && !resolved { p { role: "status", "Choose a version for every conflicting field before saving." } }
                    div { class: "draft-merge-actions",
                        button { id: "draft-merge-back", r#type: "button", class: "btn btn-secondary touch-target", disabled: pending(), onclick: move |_| on_close.call(()), "Keep editing" }
                        button { r#type: "button", class: "btn btn-primary touch-target", disabled: pending() || !resolved || changed_locally || !ready, onclick: on_save, "Save merged draft" }
                    }
                    p { class: "recipe-editor__hint", "Saving this merge updates your private draft. Publishing is a separate action." }
                }
            }
    }
}

fn read_feedback(error: &ServerFnError) -> &'static str {
    crate::auth::account_binding::account_error_feedback(error).unwrap_or("Could not load the newer draft. Your entries are still here. Keep editing, then try Review changes again.")
}

#[component]
fn FieldContent(field: RecipeField, editor: RecipeEditorState) -> Element {
    if field == RecipeField::Images {
        return rsx! { PhotoList { photos: editor.images } };
    }
    if field == RecipeField::Instructions {
        return rsx! { StepList { steps: editor.instructions } };
    }
    let text = match field {
        RecipeField::Title => editor.title,
        RecipeField::Description => editor.description.unwrap_or_default(),
        RecipeField::Commentary => editor.commentary.unwrap_or_default(),
        RecipeField::PrepTime => editor
            .prep_time_minutes
            .map(|value| format!("{value} minutes"))
            .unwrap_or_default(),
        RecipeField::CookTime => editor
            .cook_time_minutes
            .map(|value| format!("{value} minutes"))
            .unwrap_or_default(),
        RecipeField::Servings => editor
            .servings
            .map(|value| value.to_string())
            .unwrap_or_default(),
        RecipeField::Ingredients => editor
            .ingredients
            .iter()
            .map(|row| {
                let mut line = row.display_line(&row.amount);
                for substitute in &row.substitutes {
                    line.push_str(&format!(
                        "\n  or {}",
                        substitute.display_line(&substitute.amount)
                    ));
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n"),
        RecipeField::Equipment => editor
            .equipment
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        RecipeField::Visibility => match editor.visibility.as_str() {
            "public" => "Public",
            "unlisted" => "Unlisted",
            _ => "Private",
        }
        .into(),
        RecipeField::Tags => editor.tags.join(", "),
        RecipeField::Images | RecipeField::Instructions => unreachable!(),
    };
    rsx! { div { class: "draft-merge-value", if text.is_empty() { em { "Empty" } } else { "{text}" } } }
}

#[component]
fn PhotoList(photos: Vec<RecipeImage>) -> Element {
    rsx! { div { class: "draft-merge-photos",
        if photos.is_empty() { em { "No photos" } }
        for (index, photo) in photos.into_iter().enumerate() {
            figure {
                EditedRecipeImage { image: photo.clone(), alt: photo.file_name.clone().unwrap_or_else(|| format!("Photo {}", index + 1)), prefer_thumbnail: true, on_error: move |_| {} }
                figcaption { "{index + 1}. " {photo.file_name.unwrap_or_else(|| "Photo".into())} }
            }
        }
    } }
}

#[component]
fn StepList(steps: Vec<RecipeStep>) -> Element {
    rsx! {
        if steps.is_empty() { em { "No steps" } }
        else { ol { class: "draft-merge-steps", for step in steps { li { p { "{step.text}" } if !step.photos.is_empty() { PhotoList { photos: step.photos } } if !step.sub_steps.is_empty() { StepList { steps: step.sub_steps } } } } } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RecipeIngredient;

    fn example_review() -> Review {
        let base = RecipeEditorState {
            title: "Tomato soup".into(),
            ingredients: vec![
                RecipeIngredient {
                    name: "tomatoes".into(),
                    amount: "2".into(),
                    unit: "cup".into(),
                    ..Default::default()
                },
                RecipeIngredient {
                    name: "fresh basil".into(),
                    amount: "1".into(),
                    unit: "tbsp".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let mut yours = base.clone();
        yours.ingredients[0].amount = "3".into();
        let mut saved = base.clone();
        saved.title = "Roasted tomato soup".into();
        saved.ingredients.push(RecipeIngredient {
            name: "garlic".into(),
            amount: "1".into(),
            unit: "clove".into(),
            ..Default::default()
        });
        Review::new(
            DraftCheckpoint {
                revision: 0,
                editor: base,
            },
            yours,
            RecipeDraftView {
                id: uuid::Uuid::from_u128(1),
                revision: 1,
                editor: saved,
                import_images: vec![],
                recipe_id: None,
                base_version_number: None,
                forked_from_recipe_id: None,
                forked_from_version_number: None,
                forked_from_title: None,
                fork_message: None,
                created_at: chrono::DateTime::UNIX_EPOCH,
                updated_at: chrono::DateTime::UNIX_EPOCH,
            },
        )
    }

    #[component]
    fn Preview(dialog: bool, is_pending: bool) -> Element {
        let review = use_signal(|| Some(example_review()));
        let pending = use_signal(|| is_pending);
        let feedback = use_signal(|| None);
        let sync = DraftSync {
            checkpoint: use_signal(|| Some(example_review().base)),
            conflict: use_signal(|| true),
            reviewing: use_signal(|| false),
            in_flight: use_signal(|| 0),
        };
        rsx! {
            div { class: "app-shell", style: "max-width:1000px;margin:0 auto;padding:32px 20px",
                h1 { "Edit recipe" }
                p { style: "color:var(--text-secondary);margin:8px 0 24px", "Tomato soup · Private draft" }
                if dialog {
                    MergeReviewDialog { review, pending, feedback, changed_locally: false, ready: true, on_close: move |_| {}, on_save: move |_| {} }
                } else {
                    DraftConflictReview {
                        account: None, draft_id: uuid::Uuid::from_u128(1).to_string(), sync,
                        read_local: move |_| example_review().yours,
                        ready: move |_| true, blocked: false, pending_photos: 0,
                        on_upload_pending: move |_| {}, on_merged: move |_| {},
                    }
                }
                div { class: "neumo-inset", style: "padding:24px;margin-top:24px;border-radius:16px",
                    label { "Recipe title" }
                    input { class: "input", value: "Tomato soup", aria_label: "Recipe title", readonly: true, style: "display:block;width:100%;margin:8px 0 24px" }
                    h3 { "Ingredients" }
                    p { style: "margin-top:12px;white-space:pre-line", "3 cups tomatoes\n1 tbsp fresh basil" }
                }
            }
        }
    }

    #[test]
    fn dialog_shows_competing_lists_and_requires_a_choice() {
        let mut dom = VirtualDom::new_with_props(
            Preview,
            PreviewProps {
                dialog: true,
                is_pending: false,
            },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Review draft changes"));
        assert!(html.contains("complete list, including its order"));
        assert!(html.contains("3 cup tomatoes"));
        assert!(html.contains("1 clove garlic"));
        assert!(html.contains("Choose a version for every conflicting field"));
        assert!(html.contains("aria-labelledby=\"draft-merge-title\""));
        assert!(example_review()
            .plan()
            .resolve(&example_review().choices)
            .is_none());
        if let Some(directory) = std::env::var_os("NOMS_MERGE_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            for dialog in [false, true] {
                let mut dom = VirtualDom::new_with_props(
                    Preview,
                    PreviewProps {
                        dialog,
                        is_pending: false,
                    },
                );
                dom.rebuild_in_place();
                let styles = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
                let document = format!("<!doctype html><html><head><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><link rel=\"stylesheet\" href=\"file://{}/main.css\"><link rel=\"stylesheet\" href=\"file://{}/mobile.css\"><title>Draft merge preview</title></head><body>{}</body></html>", styles.display(), styles.display(), dioxus_ssr::render(&dom));
                std::fs::write(
                    directory.join(if dialog { "review.html" } else { "notice.html" }),
                    document,
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn refreshing_a_review_invalidates_old_choices_and_preserves_local_entries() {
        let mut review = example_review();
        review.choices[RecipeField::Ingredients as usize] = Some(MergeChoice::Yours);
        assert!(review.plan().resolve(&review.choices).is_some());
        let original_local = review.yours.clone();
        let mut saved = review.saved.clone();
        saved.revision += 1;
        saved.editor.ingredients[0].amount = "4".into();
        let refreshed = Review::new(review.base, review.yours, saved);
        assert!(refreshed.plan().resolve(&refreshed.choices).is_none());
        assert_eq!(refreshed.yours, original_local);
        assert_eq!(refreshed.saved.revision, 2);
    }

    #[test]
    fn selecting_a_conflicting_list_enables_confirmation_and_pending_blocks_changes() {
        use dioxus::html::{SerializedFormData, SerializedHtmlEventConverter};
        use std::rc::Rc;
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        for is_pending in [false, true] {
            let mut dom = VirtualDom::new_with_props(
                Preview,
                PreviewProps {
                    dialog: true,
                    is_pending,
                },
            );
            let mutations = dom.rebuild_to_vec();
            let inputs: Vec<_> = mutations
                .edits
                .iter()
                .filter_map(|edit| match edit {
                    dioxus_core::Mutation::NewEventListener { name, id } if *name == "change" => {
                        Some(*id)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(inputs.len(), 4);
            dom.runtime().handle_event(
                "change",
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                        "Yours".into(),
                        vec![],
                    )))) as Rc<dyn std::any::Any>,
                    true,
                ),
                inputs[2],
            );
            dom.render_immediate_to_vec();
            let html = dioxus_ssr::render(&dom);
            let save_button = html
                .split("Save merged draft")
                .next()
                .unwrap()
                .rsplit("<button")
                .next()
                .unwrap();
            assert_eq!(save_button.contains("disabled"), is_pending, "{html}");
            assert_eq!(
                html.contains("Choose a version for every conflicting field"),
                is_pending
            );
        }
    }

    #[component]
    fn CheckpointExample() -> Element {
        let sync = super::super::draft_save::use_draft_sync();
        use_hook(move || {
            let mut saved = example_review().saved;
            saved.revision = 2;
            saved.editor.title = "Latest acknowledged content".into();
            sync.acknowledge(&saved);
            saved.revision = 1;
            saved.editor.title = "Delayed older response".into();
            sync.acknowledge(&saved);
        });
        let checkpoint = sync.checkpoint.read().clone().unwrap();
        rsx! { p { "{checkpoint.revision}: {checkpoint.editor.title}" } }
    }

    #[test]
    fn acknowledged_snapshot_and_revision_stay_together_after_out_of_order_responses() {
        let mut dom = VirtualDom::new(CheckpointExample);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("2: Latest acknowledged content"));
        assert!(!html.contains("Delayed older response"));
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn a_save_during_review_requires_refetch_and_confirmation_before_merge_commits() {
        use crate::{db, test_utils};
        let (_database, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let payload = |editor: &RecipeEditorState| db::diff::RecipeDraftPayload {
            content: serde_json::from_value(serde_json::to_value(editor).unwrap()).unwrap(),
            initial_visibility: Some(editor.visibility.clone()),
        };
        let mut review = example_review();
        let draft = db::create_new_recipe_draft(&pool, owner.id, &payload(&review.base.editor))
            .await
            .unwrap();
        db::update_recipe_draft(&pool, draft.id, owner.id, 0, &payload(&review.saved.editor))
            .await
            .unwrap();
        review.saved.id = draft.id;
        review.choices[RecipeField::Ingredients as usize] = Some(MergeChoice::Yours);
        let (expected, merged) = review.submission().unwrap();

        // A third write arrives after the reviewed read and before confirmation.
        let mut latest = review.saved.clone();
        latest.revision = 2;
        latest.editor.title = "An even newer title".into();
        latest.editor.commentary = Some("Notes saved during the review".into());
        latest.editor.ingredients[0].amount = "4".into();
        db::update_recipe_draft(&pool, draft.id, owner.id, 1, &payload(&latest.editor))
            .await
            .unwrap();
        assert!(matches!(
            db::update_recipe_draft(&pool, draft.id, owner.id, expected, &payload(&merged)).await,
            Err(db::DbError::DraftConflict {
                expected: 1,
                actual: 2,
                ..
            })
        ));
        let retained = db::get_recipe_draft(&pool, draft.id, owner.id)
            .await
            .unwrap();
        assert_eq!(
            retained.decoded_snapshot().unwrap(),
            payload(&latest.editor)
        );

        let original_entries = review.yours.clone();
        let mut refreshed = Review::new(review.base, review.yours, latest);
        assert_eq!(refreshed.yours, original_entries);
        assert!(refreshed.submission().is_none());
        refreshed.choices[RecipeField::Ingredients as usize] = Some(MergeChoice::Yours);
        let (expected, merged) = refreshed.submission().unwrap();
        assert_eq!(expected, 2);
        assert_eq!(merged.ingredients, original_entries.ingredients);
        assert_eq!(merged.title, "An even newer title");
        assert_eq!(
            merged.commentary.as_deref(),
            Some("Notes saved during the review")
        );
        let saved = db::update_recipe_draft(&pool, draft.id, owner.id, expected, &payload(&merged))
            .await
            .unwrap();
        assert_eq!(saved.revision, 3);
        assert_eq!(saved.decoded_snapshot().unwrap(), payload(&merged));
    }
}
