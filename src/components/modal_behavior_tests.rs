//! State transitions are executable tests; native-dialog integration also needs browser proof.
#[path = "../pages/settings/account_delete_flow.rs"]
mod account_delete_flow;
use account_delete_flow::{AccountDeleteStep, AccountDeletionState};

#[test]
fn deletion_requires_all_three_stages_and_exact_current_username() {
    let mut state = AccountDeletionState::default();
    assert!(!state.begin_delete("testuser"));
    state.open();
    assert_eq!(state.step, Some(AccountDeleteStep::Confirming));
    state.set_confirmation("delete testuser".into());
    state.advance("testuser");
    assert_eq!(state.step, Some(AccountDeleteStep::Confirming));
    assert!(!state.begin_delete("testuser"));
    state.confirm();
    assert_eq!(state.step, Some(AccountDeleteStep::Typing));
    for invalid in [
        "",
        "delete Testuser",
        "delete testuser ",
        "delete someone-else",
    ] {
        state.set_confirmation(invalid.into());
        state.advance("testuser");
        assert_eq!(state.step, Some(AccountDeleteStep::Typing));
        assert!(!state.begin_delete("testuser"));
    }
    state.set_confirmation("delete testuser".into());
    state.advance("testuser");
    assert_eq!(state.step, Some(AccountDeleteStep::Final));
    assert!(!state.begin_delete("renamed-user"));
    assert!(state.begin_delete("testuser"));
}

#[test]
fn pending_deletion_blocks_dismissal_navigation_edits_and_duplicate_submit() {
    let mut state = AccountDeletionState::default();
    state.open();
    state.confirm();
    state.set_confirmation("delete testuser".into());
    state.advance("testuser");
    assert!(state.begin_delete("testuser"));
    let pending = state.clone();
    state.cancel();
    state.open();
    state.confirm();
    state.go_back();
    state.set_confirmation("changed".into());
    state.advance("testuser");
    assert!(!state.begin_delete("testuser"));
    assert_eq!(state, pending);
    assert!(state.pending);
}

#[test]
fn failed_deletion_preserves_confirmation_and_allows_retry_or_cancel() {
    let mut state = AccountDeletionState::default();
    state.open();
    state.confirm();
    state.set_confirmation("delete testuser".into());
    state.advance("testuser");
    assert!(state.begin_delete("testuser"));
    state.fail_delete("Temporary failure".into());
    assert!(!state.pending);
    assert_eq!(state.step, Some(AccountDeleteStep::Final));
    assert_eq!(state.confirmation, "delete testuser");
    assert_eq!(state.error.as_deref(), Some("Temporary failure"));
    assert!(state.begin_delete("testuser"));
    assert_eq!(state.error, None);
    state.fail_delete("Again".into());
    state.cancel();
    assert_eq!(state, AccountDeletionState::default());
}

#[test]
fn cancel_clears_each_non_pending_stage_and_back_preserves_typed_phrase() {
    let mut state = AccountDeletionState::default();
    state.open();
    state.cancel();
    assert_eq!(state, AccountDeletionState::default());
    state.open();
    state.confirm();
    state.set_confirmation("partial".into());
    state.cancel();
    assert_eq!(state, AccountDeletionState::default());
    state.open();
    state.confirm();
    state.set_confirmation("delete testuser".into());
    state.advance("testuser");
    state.go_back();
    assert_eq!(state.step, Some(AccountDeleteStep::Typing));
    assert_eq!(state.confirmation, "delete testuser");
    state.advance("testuser");
    state.cancel();
    assert_eq!(state, AccountDeletionState::default());
    state.fail_delete("stale completion".into());
    assert_eq!(state, AccountDeletionState::default());
}

#[test]
fn account_dialog_uses_shared_semantics_and_announces_failure() {
    let page = include_str!("../pages/settings/settings_profile.rs");
    assert!(page.contains("SharedModal {"));
    assert!(page.contains("AccountDeletionState::default"));
    assert!(page.contains("begin_delete("));
    assert!(page.contains("aria_labelledby:"));
    assert!(page.contains("role: \"alert\""));
    assert!(page.contains("account-delete-title"));
    assert!(!page.contains("delete_step.set("));
}

#[test]
fn linked_account_unlink_confirmation_uses_the_shared_modal_contract() {
    let page = include_str!("../pages/settings/settings_accounts.rs");
    assert!(page.contains("SharedModal {"));
    assert!(page.contains("aria_labelledby: \"unlink-account-title\""));
    assert!(page.contains("focus_selector: \"#unlink-account-cancel button\""));
    assert!(page.contains("class: \"modal-card\""));
    assert!(!page.contains("background_color: \"rgba(0, 0, 0, 0.5)\""));
}
