use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct DraftCheckpoint {
    pub revision: i64,
    pub editor: crate::types::RecipeEditorState,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct DraftSync {
    pub checkpoint: Signal<Option<DraftCheckpoint>>,
    pub conflict: Signal<bool>,
    pub reviewing: Signal<bool>,
    pub in_flight: Signal<usize>,
}

pub(super) fn use_draft_sync() -> DraftSync {
    DraftSync {
        checkpoint: use_signal(|| None),
        conflict: use_signal(|| false),
        reviewing: use_signal(|| false),
        in_flight: use_signal(|| 0),
    }
}

impl DraftSync {
    pub fn revision(self) -> i64 {
        self.checkpoint
            .read()
            .as_ref()
            .map_or(0, |base| base.revision)
    }

    pub fn conflicted(self) -> bool {
        *self.conflict.read()
    }

    pub fn reviewing(self) -> bool {
        *self.reviewing.read()
    }

    pub fn acknowledge(mut self, saved: &crate::types::RecipeDraftView) {
        let current = self.checkpoint.peek().as_ref().map(|base| base.revision);
        if current.is_none_or(|revision| saved.revision >= revision) {
            self.checkpoint.set(Some(DraftCheckpoint {
                revision: saved.revision,
                editor: saved.editor.clone(),
            }));
        }
    }
}

pub(super) fn draft_revision_conflict(error: &ServerFnError) -> bool {
    matches!(error, ServerFnError::ServerError { message, .. }
        if message == crate::api::recipe::DRAFT_CHANGED)
}

/// Only a successful write from this editor may advance its revision. Reading
/// the server's latest revision cannot distinguish our save from another tab's.
pub(super) async fn save_editor_draft(
    account: Option<uuid::Uuid>,
    draft_id: String,
    mut sync: DraftSync,
    editor: crate::types::RecipeEditorState,
    may_retry: impl Fn() -> bool,
) -> Result<crate::types::RecipeDraftView, ServerFnError> {
    if *sync.conflict.peek() {
        return Err(ServerFnError::new(crate::api::recipe::DRAFT_CHANGED));
    }
    let pending = *sync.in_flight.peek() + 1;
    sync.in_flight.set(pending);
    let result = save_with_acknowledged_revision(
        || {
            sync.checkpoint
                .peek()
                .as_ref()
                .map_or(0, |base| base.revision)
        },
        &may_retry,
        |expected| {
            crate::api::recipe::save_recipe_draft(
                account,
                draft_id.clone(),
                expected,
                editor.clone(),
            )
        },
        |saved| {
            // Keep the corresponding snapshot even for older generations.
            sync.acknowledge(saved);
        },
    )
    .await;
    let pending = *sync.in_flight.peek() - 1;
    sync.in_flight.set(pending);
    if may_retry() && result.as_ref().err().is_some_and(draft_revision_conflict) {
        sync.conflict.set(true);
    }
    result
}

async fn save_with_acknowledged_revision<T, E, F, Fut>(
    revision: impl Fn() -> i64,
    may_retry: impl Fn() -> bool,
    mut save: F,
    mut acknowledge: impl FnMut(&T),
) -> Result<T, E>
where
    F: FnMut(i64) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    // Read at dispatch, after any debounce or uploads have completed.
    let expected = revision();
    let mut result = save(expected).await;
    let acknowledged = revision();
    if result.is_err() && may_retry() && acknowledged > expected {
        // A different request from this editor was confirmed while we waited.
        // CAS still protects against any intervening change from another tab.
        result = save(acknowledged).await;
    }
    if let Ok(saved) = &result {
        acknowledge(saved);
    }
    result
}

#[derive(Clone, Copy)]
pub(super) enum RecipeWriteAction {
    #[cfg(any(target_arch = "wasm32", test))]
    Autosave,
    CreateDraft,
    SaveDraft,
    Publish,
    Discard,
}

pub(super) const UNCONFIRMED_DRAFT_CREATION_MESSAGE: &str = "Could not confirm draft creation. A draft may already exist. Your entries are still here; check your dashboard in another tab and reopen the draft before saving again.";
pub(super) const UNCONFIRMED_PUBLICATION_MESSAGE: &str = "Could not confirm publication. Your draft may have been published; check your dashboard in another tab before trying again. Your entries remain on this page.";

/// A missing mutation response is not proof that the server did nothing.
/// Creation cannot be blindly retried without its returned draft identity.
pub(super) fn recipe_write_feedback(
    action: RecipeWriteAction,
    error: &ServerFnError,
) -> &'static str {
    if let Some(feedback) = crate::auth::account_binding::account_error_feedback(error) {
        return feedback;
    }
    if recipe_name_conflict(error) {
        return crate::api::recipe::RECIPE_NAME_TAKEN;
    }
    if draft_revision_conflict(error) {
        return crate::api::recipe::DRAFT_CHANGED;
    }
    if matches!(action, RecipeWriteAction::CreateDraft) {
        return UNCONFIRMED_DRAFT_CREATION_MESSAGE;
    }
    if matches!(error, ServerFnError::ServerError { message, .. } if message == "Not authenticated")
    {
        return "Sign in again in another tab, then try again. Keep this page open to retain your entries.";
    }
    match action {
        #[cfg(any(target_arch = "wasm32", test))]
        RecipeWriteAction::Autosave => "Autosave could not complete. Your entries are still here; try Save draft or edit again to retry.",
        RecipeWriteAction::SaveDraft => "Could not save the draft. Your entries are still here; try Save draft again.",
        RecipeWriteAction::Publish => UNCONFIRMED_PUBLICATION_MESSAGE,
        RecipeWriteAction::Discard => "Could not confirm draft removal. Check your dashboard before trying again; your entries remain on this page.",
        RecipeWriteAction::CreateDraft => UNCONFIRMED_DRAFT_CREATION_MESSAGE,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftSaveButtonState {
    Save,
    Saving,
    Saved,
}

impl DraftSaveButtonState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Save => "Save Draft",
            Self::Saving => "Saving...",
            Self::Saved => "Saved",
        }
    }

    pub fn is_actionable(self) -> bool {
        matches!(self, Self::Save)
    }
}

pub fn draft_save_button_state(is_saving: bool, has_unsaved_changes: bool) -> DraftSaveButtonState {
    if is_saving {
        DraftSaveButtonState::Saving
    } else if has_unsaved_changes {
        DraftSaveButtonState::Save
    } else {
        DraftSaveButtonState::Saved
    }
}

#[cfg(test)]
mod tests {
    use super::{draft_save_button_state, DraftSaveButtonState};

    #[test]
    fn draft_save_button_reports_save_saving_and_saved_states() {
        assert_eq!(
            draft_save_button_state(false, true),
            DraftSaveButtonState::Save
        );
        assert_eq!(
            draft_save_button_state(true, true),
            DraftSaveButtonState::Saving
        );
        assert_eq!(
            draft_save_button_state(false, false),
            DraftSaveButtonState::Saved
        );
    }

    #[test]
    fn only_the_save_state_is_actionable() {
        assert!(DraftSaveButtonState::Save.is_actionable());
        assert!(!DraftSaveButtonState::Saving.is_actionable());
        assert!(!DraftSaveButtonState::Saved.is_actionable());
    }

    #[test]
    fn uncertain_draft_creation_feedback_does_not_encourage_a_duplicate_retry() {
        let message = super::recipe_write_feedback(
            super::RecipeWriteAction::CreateDraft,
            &dioxus::prelude::ServerFnError::new("private database / transport details"),
        );
        assert!(message.contains("may already exist"));
        assert!(message.contains("dashboard"));
        assert!(message.contains("another tab"));
        assert!(!message.contains("private database"));
        assert!(!message.contains("try Save draft again"));
    }

    #[test]
    fn new_editor_stops_writes_after_unconfirmed_creation_and_retains_publication_identity() {
        let source = include_str!("recipe_new.rs");
        assert!(source.contains("let mut draft_creation_unconfirmed = use_signal(|| false)"));
        assert!(source.matches("draft_creation_unconfirmed()").count() >= 5);
        assert_eq!(
            source
                .matches("draft_creation_unconfirmed.set(true)")
                .count(),
            3
        );
        assert!(source.contains("Check Dashboard"));
        assert!(source.contains("new_tab: true"));
        assert!(source.contains(
            "if draft_creation_unconfirmed() || publication_unconfirmed() || error().is_some_and("
        ));
        let deferred_save = source
            .split("TimeoutFuture::new(2_000).await;")
            .nth(1)
            .unwrap()
            .split("let result =")
            .next()
            .unwrap();
        assert!(deferred_save.contains("draft_creation_unconfirmed()"));
        assert!(!source.contains("Autosave failed: {save_error}"));
        let publish = source.split("let publish_result = async {").nth(1).unwrap();
        let retained_id = publish
            .find("draft_id.set(Some(saved.id.to_string()))")
            .unwrap();
        let submit = publish
            .find("publish_recipe_draft(form_account(), saved.id.to_string(), None)")
            .unwrap();
        assert!(retained_id < submit);
    }

    #[test]
    fn unknown_publication_response_remains_visible_and_blocks_later_writes() {
        let source = include_str!("recipe_new.rs");
        assert!(source.contains("let mut publication_unconfirmed = use_signal(|| false)"));
        assert!(source.matches("publication_unconfirmed()").count() >= 6);
        assert!(source.contains("publication_attempted = true"));
        assert!(source.contains("publication_unconfirmed.set(true)"));
        assert!(source.contains(
            "if draft_creation_unconfirmed() || publication_unconfirmed() || error().is_some_and("
        ));
    }

    #[test]
    fn confirmed_publication_keeps_navigation_inside_the_app_in_both_editors() {
        for source in [
            include_str!("recipe_new.rs"),
            include_str!("recipe_edit.rs"),
        ] {
            let completion = source
                .split("match publish_result {")
                .nth(1)
                .unwrap()
                .split("Err(e) =>")
                .next()
                .unwrap();
            assert!(completion.contains("navigator.push(crate::Route::RecipeDetail"));
            assert!(completion.contains("id: recipe.id.to_string()"));
            assert!(!completion.contains("set_href"));
            assert!(!completion.contains("window.location"));
        }
    }

    #[test]
    fn saved_editor_return_uses_a_typed_destination_only_after_save_succeeds() {
        let source = include_str!("recipe_edit.rs");
        assert!(source.contains("Option::<crate::Route>::None"));
        let completion = source
            .split("match save_result {")
            .last()
            .unwrap()
            .split("Err(save_error)")
            .next()
            .unwrap();
        assert!(completion.contains("Ok(_)"));
        assert!(completion.contains("navigator.push(destination)"));
        assert!(!completion.contains("set_href"));
        let cancel = source
            .split("let on_cancel =")
            .nth(1)
            .unwrap()
            .split("let id_for_discard")
            .next()
            .unwrap();
        assert!(cancel.contains("crate::Route::Dashboard {}"));
        assert!(cancel.contains("crate::Route::RecipeDetail"));
        assert!(!cancel.contains("navigator.push"), "Cancel must save first");
    }
}

pub(super) fn recipe_name_conflict(error: &ServerFnError) -> bool {
    matches!(error, ServerFnError::ServerError { message, .. }
        if message == crate::api::recipe::RECIPE_NAME_TAKEN)
}

#[cfg(test)]
mod name_conflict_tests {
    #[test]
    fn confirmed_name_conflicts_allow_renaming_without_unknown_write_feedback() {
        let error = dioxus::prelude::ServerFnError::new(crate::api::recipe::RECIPE_NAME_TAKEN);
        assert!(super::recipe_name_conflict(&error));
        assert_eq!(
            super::recipe_write_feedback(super::RecipeWriteAction::Publish, &error),
            crate::api::recipe::RECIPE_NAME_TAKEN
        );
        assert!(!super::recipe_name_conflict(
            &dioxus::prelude::ServerFnError::new("Database operation timed out")
        ));
    }

    #[test]
    fn draft_conflicts_explain_recovery_without_suggesting_a_blind_retry() {
        let error = dioxus::prelude::ServerFnError::new(crate::api::recipe::DRAFT_CHANGED);
        for action in [
            super::RecipeWriteAction::Autosave,
            super::RecipeWriteAction::SaveDraft,
            super::RecipeWriteAction::Publish,
        ] {
            let message = super::recipe_write_feedback(action, &error);
            assert_eq!(message, crate::api::recipe::DRAFT_CHANGED);
            assert!(message.contains("Your entries are still here"));
            assert!(message.contains("compare"));
            assert!(!message.contains("try Save draft"));
            assert!(!message.contains("may have been published"));
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod revision_tests {
    use super::save_with_acknowledged_revision;
    use std::cell::{Cell, RefCell};

    #[tokio::test]
    async fn confirmed_local_save_can_retry_the_newest_snapshot() {
        let revision = Cell::new(0);
        let attempts = RefCell::new(Vec::new());
        let result = save_with_acknowledged_revision(
            || revision.get(),
            || true,
            |expected| {
                attempts.borrow_mut().push(expected);
                std::future::ready(if expected == 0 {
                    // Our older in-flight save's acknowledgment arrives while
                    // this request is waiting on its conflict response.
                    revision.set(1);
                    Err("conflict")
                } else {
                    Ok(expected + 1)
                })
            },
            |saved| revision.set(*saved),
        )
        .await;
        assert_eq!(result, Ok(2));
        assert_eq!(*attempts.borrow(), vec![0, 1]);
        assert_eq!(revision.get(), 2);
    }

    #[tokio::test]
    async fn superseded_snapshot_does_not_retry_but_success_still_acknowledges() {
        let revision = Cell::new(0);
        let attempts = Cell::new(0);
        let result = save_with_acknowledged_revision(
            || revision.get(),
            || false,
            |_| {
                attempts.set(attempts.get() + 1);
                revision.set(1);
                std::future::ready(Err::<i64, _>("conflict"))
            },
            |_| panic!("a failed write must not acknowledge"),
        )
        .await;
        assert_eq!(result, Err("conflict"));
        assert_eq!(attempts.get(), 1);

        let result = save_with_acknowledged_revision(
            || revision.get(),
            || false,
            |expected| std::future::ready(Ok::<_, &str>(expected + 1)),
            |saved| revision.set(*saved),
        )
        .await;
        assert_eq!(result, Ok(2));
        assert_eq!(revision.get(), 2);
    }

    #[tokio::test]
    async fn missing_response_without_an_acknowledgment_is_not_retried() {
        let attempts = Cell::new(0);
        let result = save_with_acknowledged_revision(
            || 0,
            || true,
            |_| {
                attempts.set(attempts.get() + 1);
                std::future::ready(Err::<i64, _>("response lost after commit"))
            },
            |_| panic!("an unconfirmed write must not acknowledge"),
        )
        .await;
        assert_eq!(result, Err("response lost after commit"));
        assert_eq!(attempts.get(), 1);
    }

    #[tokio::test]
    async fn another_tabs_changes_survive_stale_saves_and_local_retries() {
        use crate::{db, test_utils};
        let (_database, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let payload = |title| db::diff::RecipeDraftPayload {
            content: test_utils::test_recipe_snapshot(title),
            initial_visibility: Some("private".to_string()),
        };
        let draft = db::create_new_recipe_draft(&pool, owner.id, &payload("Initial"))
            .await
            .unwrap();
        let remote = payload("Saved in another tab");
        let local = payload("Unsaved local entries");
        db::update_recipe_draft(&pool, draft.id, owner.id, 0, &remote)
            .await
            .unwrap();
        let revision = Cell::new(0);
        let attempts = RefCell::new(Vec::new());
        let result = save_with_acknowledged_revision(
            || revision.get(),
            || true,
            |expected| {
                attempts.borrow_mut().push(expected);
                db::update_recipe_draft(&pool, draft.id, owner.id, expected, &local)
            },
            |saved| revision.set(saved.revision),
        )
        .await;
        assert!(matches!(
            result,
            Err(db::DbError::DraftConflict {
                expected: 0,
                actual: 1,
                ..
            })
        ));
        assert_eq!(*attempts.borrow(), vec![0]);
        assert_eq!(revision.get(), 0);
        let retained = db::get_recipe_draft(&pool, draft.id, owner.id)
            .await
            .unwrap();
        assert_eq!(retained.decoded_snapshot().unwrap(), remote);

        // Even a safe retry after our own acknowledgment must still reject a
        // subsequent write from another tab instead of adopting its revision.
        revision.set(1);
        let own_save = db::update_recipe_draft(&pool, draft.id, owner.id, 1, &local)
            .await
            .unwrap();
        db::update_recipe_draft(&pool, draft.id, owner.id, own_save.revision, &remote)
            .await
            .unwrap();
        attempts.borrow_mut().clear();
        let result = save_with_acknowledged_revision(
            || revision.get(),
            || true,
            |expected| {
                attempts.borrow_mut().push(expected);
                revision.set(own_save.revision);
                db::update_recipe_draft(&pool, draft.id, owner.id, expected, &local)
            },
            |saved| revision.set(saved.revision),
        )
        .await;
        assert!(matches!(
            result,
            Err(db::DbError::DraftConflict {
                expected: 2,
                actual: 3,
                ..
            })
        ));
        assert_eq!(*attempts.borrow(), vec![1, 2]);
        let retained = db::get_recipe_draft(&pool, draft.id, owner.id)
            .await
            .unwrap();
        assert_eq!(retained.decoded_snapshot().unwrap(), remote);
        assert_eq!(local.content.title, "Unsaved local entries");
    }
}
