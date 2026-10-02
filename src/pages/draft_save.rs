use dioxus::prelude::ServerFnError;

#[derive(Clone, Copy)]
pub(super) enum RecipeWriteAction {
    #[cfg(any(target_arch = "wasm32", test))]
    Autosave,
    CreateDraft,
    SaveDraft,
    Publish,
    Discard,
}

pub(super) const UNCONFIRMED_DRAFT_CREATION_MESSAGE: &str = "Could not confirm draft creation. A draft may already exist. Your entries are still here; check your dashboard and reopen the draft before saving again.";

/// A missing mutation response is not proof that the server did nothing.
/// Creation cannot be blindly retried without its returned draft identity.
pub(super) fn recipe_write_feedback(
    action: RecipeWriteAction,
    error: &ServerFnError,
) -> &'static str {
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
        RecipeWriteAction::Publish => "Could not confirm publication. Your draft may have been published; check your dashboard before trying again.",
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
        assert!(source.contains("if draft_creation_unconfirmed() || error().is_some()"));
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
            .find("publish_recipe_draft(saved.id.to_string(), None)")
            .unwrap();
        assert!(retained_id < submit);
    }
}
