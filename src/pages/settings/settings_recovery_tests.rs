use super::*;

#[test]
fn avatar_removal_claims_shared_busy_state_and_failed_removal_retains_selected_edits() {
    let mut state = AvatarUploadState::<Vec<u8>>::default();
    let edit = RecipeImageEdit::default_ellipse_1x1();
    state.select(vec![1, 2, 3], edit.clone());
    assert!(state.begin_removal());
    assert!(state.uploading && state.removing);
    assert!(!state.begin_removal());
    assert!(state.begin_upload().is_none());
    assert!(!state.select(vec![4], edit.clone()));
    state.finish(false);
    assert!(!state.uploading && !state.removing);
    assert_eq!(state.pending, Some((vec![1, 2, 3], edit)));
    assert!(state.begin_removal());
    state.finish(true);
    assert!(state.pending.is_none() && !state.removing && !state.uploading);
}

#[component]
fn DeletionFeedbackRoot(pending: bool, failed: bool) -> Element {
    rsx! { AccountDeletionFeedback { pending,
        error: failed.then(||settings_mutation_feedback(SettingsMutation::DeleteAccount,&ServerFnError::new("internal details")).to_string()),
    } }
}

#[test]
fn account_deletion_feedback_announces_pending_or_safe_failure_and_keeps_check_in_another_tab() {
    for (pending, failed) in [(false, false), (true, true), (false, true)] {
        let mut dom = VirtualDom::new_with_props(
            DeletionFeedbackRoot,
            DeletionFeedbackRootProps { pending, failed },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.contains("role=\"status\""), pending);
        assert_eq!(html.contains("role=\"alert\""), failed && !pending);
        assert!(!html.contains("internal details"));
        if failed && !pending {
            assert!(html.contains("may already have been deleted"));
            assert!(html.contains("target=\"_blank\""));
        }
    }
}

#[test]
fn profile_save_feedback_keeps_known_validation_and_session_guidance() {
    for message in [
        "Username must be 3-30 characters",
        "Username can only contain letters, numbers, hyphens, and underscores",
        "Username cannot start or end with a hyphen or underscore",
        "Display name must be 2-30 characters",
        "Bio must be 160 characters or less",
        "That username is already taken. Please choose another.",
    ] {
        assert_eq!(profile_save_feedback(&ServerFnError::new(message)), message);
    }
    let error = ServerFnError::new("Not authenticated");
    let session = profile_save_feedback(&error);
    assert!(session.contains("Sign in again in another tab"));
    assert!(session.contains("Keep this page open"));
}

#[test]
fn profile_save_feedback_never_displays_unknown_server_or_transport_details() {
    for error in [
        ServerFnError::new("internal database details"),
        ServerFnError::StreamError("Failed to read response body".into()),
    ] {
        assert_eq!(
            profile_save_feedback(&error),
            "Could not save your profile. Your entries are kept here; try Save changes again."
        );
    }
}

#[component]
fn SaveControlTestRoot(pending: bool, valid: bool) -> Element {
    rsx! { ProfileSaveControl { pending, valid, on_save: |_| {} } }
}

#[test]
fn profile_save_control_retains_focusable_button_and_announces_pending() {
    for (pending, valid) in [(false, true), (true, true), (false, false)] {
        let mut dom = VirtualDom::new_with_props(
            SaveControlTestRoot,
            SaveControlTestRootProps { pending, valid },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.matches("<button").count(), 1, "{html}");
        assert!(!html.contains(" disabled"), "{html}");
        assert!(html.contains("touch-target"), "{html}");
        assert!(
            html.contains(&format!("aria-disabled={}", pending || !valid)),
            "{html}"
        );
        assert!(
            html.contains(if pending { "Saving…" } else { "Save changes" }),
            "{html}"
        );
        assert_eq!(html.contains("role=\"status\""), pending, "{html}");
    }
}

#[component]
fn SaveGuardTestRoot(
    pending: bool,
    valid: bool,
    calls: std::rc::Rc<std::cell::Cell<usize>>,
) -> Element {
    rsx! { ProfileSaveControl { pending, valid, on_save: move |_| calls.set(calls.get() + 1) } }
}

#[test]
fn profile_save_control_blocks_pending_and_invalid_activation() {
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::rc::Rc;
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for (pending, valid, expected) in [(true, true, 0), (false, false, 0), (false, true, 1)] {
        let calls = Rc::new(std::cell::Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            SaveGuardTestRoot,
            SaveGuardTestRootProps {
                pending,
                valid,
                calls: calls.clone(),
            },
        );
        let mutations = dom.rebuild_to_vec();
        let button_id = mutations
            .edits
            .iter()
            .find_map(|edit| match edit {
                dioxus_core::Mutation::NewEventListener { name, id } if *name == "click" => {
                    Some(*id)
                }
                _ => None,
            })
            .expect("save control click handler");
        dom.runtime().handle_event(
            "click",
            Event::new(
                Rc::new(PlatformEventData::new(Box::new(
                    SerializedMouseData::default(),
                ))) as Rc<dyn std::any::Any>,
                true,
            ),
            button_id,
        );
        assert_eq!(calls.get(), expected);
    }
}
