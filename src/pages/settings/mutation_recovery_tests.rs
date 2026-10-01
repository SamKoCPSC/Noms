use super::*;

#[test]
fn mutation_feedback_never_exposes_unknown_errors_or_promises_rollback() {
    for action in [
        SettingsMutation::Unlink,
        SettingsMutation::RemoveAvatar,
        SettingsMutation::DeleteAccount,
    ] {
        for error in [
            ServerFnError::new("internal database details"),
            ServerFnError::StreamError("transport body".into()),
        ] {
            let message = settings_mutation_feedback(action, &error);
            assert!(message.contains("Could not confirm"));
            assert!(message.contains("another tab"));
            assert!(!message.contains("internal database"));
            assert!(!message.contains("transport body"));
        }
        let session = settings_mutation_feedback(action, &ServerFnError::new("Not authenticated"));
        assert!(session.contains("Sign in again in another tab"));
        assert!(session.contains("Keep this page open"));
    }
    assert!(settings_mutation_feedback(
        SettingsMutation::Unlink,
        &ServerFnError::new("You must have at least one linked account")
    )
    .contains("Keep at least one"));
}

#[component]
fn ControlRoot(
    pending: bool,
    unavailable: bool,
    calls: std::rc::Rc<std::cell::Cell<usize>>,
) -> Element {
    rsx! { SettingsMutationControl {
        id: "test-mutation", variant: ButtonVariant::Danger,
        pending, unavailable, label: "Perform action", pending_label: "Working…",
        on_action: move |_| calls.set(calls.get() + 1),
    } }
}

#[test]
fn mutation_controls_remain_focusable_and_actual_handlers_guard_unavailable_actions() {
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::rc::Rc;
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for (pending, unavailable) in [(false, false), (true, false), (false, true)] {
        let calls = Rc::new(std::cell::Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            ControlRoot,
            ControlRootProps {
                pending,
                unavailable,
                calls: calls.clone(),
            },
        );
        let edits = dom.rebuild_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.matches("<button").count(), 1);
        assert!(!html.contains(" disabled"));
        assert!(html.contains("touch-target"));
        assert!(html.contains(&format!("aria-disabled={}", pending || unavailable)));
        assert!(html.contains(&format!("aria-busy={pending}")));
        let id = edits
            .edits
            .iter()
            .find_map(|edit| match edit {
                dioxus_core::Mutation::NewEventListener { name, id } if *name == "click" => {
                    Some(*id)
                }
                _ => None,
            })
            .unwrap();
        dom.runtime().handle_event(
            "click",
            Event::new(
                Rc::new(PlatformEventData::new(Box::new(
                    SerializedMouseData::default(),
                ))) as Rc<dyn std::any::Any>,
                true,
            ),
            id,
        );
        assert_eq!(calls.get(), usize::from(!pending && !unavailable));
    }
}

#[test]
fn settings_handlers_use_safe_feedback_and_keep_pending_controls_mounted() {
    let profile = include_str!("settings_profile.rs");
    for (start, end, action) in [
        (
            "let on_delete_final =",
            "// Selecting an avatar",
            "SettingsMutation::DeleteAccount",
        ),
        (
            "let on_remove_avatar =",
            "document::Stylesheet",
            "SettingsMutation::RemoveAvatar",
        ),
    ] {
        let handler = profile
            .split(start)
            .nth(1)
            .unwrap()
            .split(end)
            .next()
            .unwrap();
        assert!(handler.contains(action));
        assert!(!handler.contains("e.to_string()"));
        assert!(!handler.contains("message.clone()"));
    }
    assert!(profile.contains("id: \"avatar-remove-photo\""));
    assert!(profile.contains("pending: avatar_upload_state.read().removing"));
    assert!(profile.contains("id: \"account-delete-final\""));
    assert!(profile.contains("AccountDeletionFeedback {"));
    let accounts = include_str!("settings_accounts.rs")
        .split("pub fn SettingsAccounts()")
        .nth(1)
        .unwrap();
    assert!(accounts.contains("AccountUnlinkDialog {"));
    assert!(accounts.contains("unlink_state.write().begin()"));
    assert!(accounts.contains("can_unlink_account"));
    assert!(!accounts.contains("e.to_string()"));
    assert!(!accounts.contains("Duration::from_secs(10)"));
    assert!(!accounts.contains("location().set_href(\"/settings/accounts\")"));
}
