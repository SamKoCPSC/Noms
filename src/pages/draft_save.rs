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
}
