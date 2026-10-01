use super::*;

#[test]
fn unlink_confirmation_retains_provider_across_failure_and_blocks_pending_changes() {
    let id = Uuid::from_u128(1);
    let mut state = AccountUnlinkState::default();
    assert!(state.begin().is_none());
    assert!(state.open(id, "Google".into()));
    assert_eq!(state.begin(), Some((id, "Google".into())));
    assert!(state.begin().is_none());
    assert!(!state.open(Uuid::from_u128(2), "GitHub".into()));
    state.cancel();
    assert!(state.pending && state.confirmation.is_some());
    state.finish(Err("Safe failure".into()));
    assert!(!state.pending);
    assert_eq!(state.confirmation, Some((id, "Google".into())));
    assert_eq!(state.error.as_deref(), Some("Safe failure"));
    state.cancel();
    assert!(state.confirmation.is_none());
    assert_eq!(state.error.as_deref(), Some("Safe failure"));
    state.finish(Ok(()));
    assert!(state.success.is_none(), "Idle/stale completion is ignored");
    state.open(id, "Google".into());
    state.begin();
    state.finish(Ok(()));
    assert!(state.confirmation.is_none() && !state.pending);
    assert!(state.success.as_deref().unwrap().contains("Google"));
}

#[test]
fn unlink_requires_a_loaded_current_candidate_and_more_than_one_sign_in_method() {
    let a = LinkedAccount {
        id: Uuid::from_u128(1),
        provider: "google".into(),
        email: None,
        last_used_at: chrono::Utc::now(),
    };
    let b = LinkedAccount {
        id: Uuid::from_u128(2),
        provider: "github".into(),
        ..a.clone()
    };
    assert!(!can_unlink_account(&[], a.id));
    assert!(!can_unlink_account(&[a.clone()], a.id));
    assert!(!can_unlink_account(&[a.clone(), b.clone()], Uuid::nil()));
    assert!(can_unlink_account(&[a.clone(), b], a.id));
}

#[test]
fn oauth_conflict_feedback_is_persistent_safe_and_does_not_recommend_account_deletion() {
    let message = oauth_link_feedback(Some("account_already_linked"), Some("github")).unwrap();
    assert!(message.contains("GitHub"));
    assert!(!message.contains("deleted"));
    assert!(oauth_link_feedback(Some("unknown"), Some("github")).is_none());
    assert!(!oauth_link_feedback(
        Some("account_already_linked"),
        Some("untrusted query details")
    )
    .unwrap()
    .contains("untrusted query"));
}

#[component]
fn DialogRoot(pending: bool, failed: bool) -> Element {
    rsx! { AccountUnlinkDialog { provider: "Google",pending,
        error: failed.then(||"Safe recovery guidance".to_string()),
        on_cancel: |_|{}, on_confirm: |_|{},
    } }
}

#[test]
fn unlink_dialog_announces_pending_or_failure_and_keeps_safe_cancel_target() {
    for (pending, failed) in [(false, false), (true, false), (false, true)] {
        let mut dom = VirtualDom::new_with_props(DialogRoot, DialogRootProps { pending, failed });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("aria-labelledby=\"unlink-account-title\""));
        assert!(html.contains("Unlink Google?"));
        assert_eq!(html.matches("<button").count(), 2);
        assert!(!html.contains(" disabled"));
        assert_eq!(html.contains("role=\"status\""), pending);
        assert_eq!(html.contains("role=\"alert\""), failed && !pending);
    }
}

#[component]
fn DialogEventsRoot(
    pending: bool,
    canceled: std::rc::Rc<std::cell::Cell<usize>>,
    confirmed: std::rc::Rc<std::cell::Cell<usize>>,
) -> Element {
    rsx! { AccountUnlinkDialog {
        provider:"Google",pending,error:None,
        on_cancel:move |_|canceled.set(canceled.get()+1),
        on_confirm:move |_|confirmed.set(confirmed.get()+1),
    } }
}

#[test]
fn unlink_cancel_never_confirms_and_pending_dialog_buttons_dispatch_neither_action() {
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::rc::Rc;
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for pending in [false, true] {
        let canceled = Rc::new(std::cell::Cell::new(0));
        let confirmed = Rc::new(std::cell::Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            DialogEventsRoot,
            DialogEventsRootProps {
                pending,
                canceled: canceled.clone(),
                confirmed: confirmed.clone(),
            },
        );
        let edits = dom.rebuild_to_vec();
        // Child components can be mounted in a different mutation order. Resolve
        // the real controls by their IDs rather than assuming handler order.
        for (index, target) in ["unlink-account-cancel-button", "unlink-account-confirm"]
            .iter()
            .enumerate()
        {
            let id = edits
                .edits
                .iter()
                .find_map(|edit| match edit {
                    dioxus_core::Mutation::SetAttribute {
                        name,
                        value: dioxus_core::AttributeValue::Text(value),
                        id,
                        ..
                    } if *name == "id" && value == target => Some(*id),
                    _ => None,
                })
                .expect("actual named dialog button");
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
            assert_eq!(canceled.get(), usize::from(!pending));
            assert_eq!(confirmed.get(), usize::from(!pending && index == 1));
        }
    }
}
