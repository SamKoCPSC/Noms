use super::*;

#[test]
fn mutation_failures_use_safe_copy_without_promising_a_write_did_not_commit() {
    for action in [
        RecipeMutation::Fork,
        RecipeMutation::Visibility,
        RecipeMutation::Restore,
        RecipeMutation::Delete,
    ] {
        for error in [
            ServerFnError::new("internal database details"),
            ServerFnError::StreamError("response body interrupted".into()),
        ] {
            let message = recipe_mutation_feedback(action, &error);
            assert!(message.starts_with("Could not confirm"), "{message}");
            assert!(!message.contains("internal database"));
            assert!(!message.contains("response body"));
            assert!(message.contains("before trying again"));
        }
    }
}

#[test]
fn mutation_session_failures_offer_sign_in_without_discarding_the_page() {
    let error = ServerFnError::new("Not authenticated");
    for action in [
        RecipeMutation::Fork,
        RecipeMutation::Visibility,
        RecipeMutation::Restore,
        RecipeMutation::Delete,
    ] {
        let message = recipe_mutation_feedback(action, &error);
        assert!(message.contains("Sign in again in another tab"));
        assert!(message.contains("Keep this recipe open"));
    }
}

#[component]
fn DeletionClaimTestRoot() -> Element {
    let mut deleting = use_signal(|| false);
    // Invoke the actual claim logic without a browser confirmation or API call.
    assert!(!claim_recipe_deletion(deleting, false));
    assert!(!*deleting.peek());
    assert!(claim_recipe_deletion(deleting, true));
    assert!(*deleting.peek());
    assert!(!claim_recipe_deletion(deleting, true));
    assert!(!claim_recipe_deletion(deleting, false));
    assert!(*deleting.peek());
    // A failed request releases pending; cancellation still does not retry.
    deleting.set(false);
    assert!(!claim_recipe_deletion(deleting, false));
    assert!(!*deleting.peek());
    assert!(claim_recipe_deletion(deleting, true));
    rsx! { p { "One confirmed request claimed" } }
}

#[test]
fn deletion_cancellation_does_not_claim_a_request_and_pending_claims_are_exclusive() {
    let mut dom = VirtualDom::new(DeletionClaimTestRoot);
    dom.rebuild_in_place();
    assert!(dioxus_ssr::render(&dom).contains("One confirmed request claimed"));
}

#[test]
fn deletion_warning_names_the_recipe_and_failure_does_not_assume_it_still_exists() {
    let warning = recipe_delete_confirmation("CP8 Test recipe");
    assert!(warning.contains("CP8 Test recipe"));
    assert!(warning.to_lowercase().contains("permanently"));
    assert!(warning.contains("cannot be undone"));
    let message = recipe_mutation_feedback(
        RecipeMutation::Delete,
        &ServerFnError::StreamError("internal response details".into()),
    );
    assert!(message.contains("may already have been deleted"));
    assert!(message.contains("Dashboard"));
    assert!(!message.contains("internal response details"));
}

#[test]
fn deletion_guards_before_confirmation_and_uses_safe_accessible_feedback() {
    let source = include_str!("recipe_detail.rs");
    let handler = source
        .split("let confirm_delete =")
        .nth(1)
        .unwrap()
        .split("// ── Render: loading")
        .next()
        .unwrap();
    assert!(handler.contains("delete_confirmation_open.peek()"));
    assert!(handler.contains("claim_recipe_deletion("));
    assert!(handler.contains("!is_owner"));
    assert!(
        handler.find("claim_recipe_deletion(").unwrap()
            < handler.find("delete_error.set(None)").unwrap()
    );
    assert!(handler.contains("is_deleting.set(false)"));
    assert!(handler.contains("RecipeMutation::Delete"));
    assert!(!handler.contains("e.to_string()"));
    let menu = source
        .split("action-menu-item--danger")
        .nth(1)
        .unwrap()
        .split("onclick: on_delete")
        .next()
        .unwrap();
    assert!(menu.contains("aria_disabled: is_deleting()"));
    assert!(menu.contains("aria_busy: is_deleting()"));
    assert!(!menu.contains(" disabled: is_deleting()"));
    let feedback = source
        .split("fn RecipeDeletionFeedback")
        .nth(1)
        .unwrap()
        .split("/// A stable")
        .next()
        .unwrap();
    assert!(feedback.contains("role: \"alert\""));
    assert!(feedback.contains("Check your Dashboard"));
    assert!(feedback.contains("role: \"status\""));
    assert!(source.contains("pending: is_deleting(), error: delete_error()"));
    assert!(!source.contains("confirm_with_message"));
    assert!(!source.contains("window.confirm()"));
    let dialog = source
        .split("fn RecipeDeletionDialog")
        .nth(1)
        .unwrap()
        .split("/// A stable")
        .next()
        .unwrap();
    assert!(dialog.contains("SharedModal"));
    assert!(dialog.contains("focus_selector: \"#cancel-recipe-deletion\""));
}

#[component]
fn DeletionDialogTestRoot(pending: bool) -> Element {
    rsx! { RecipeDeletionDialog {
        title: "CP8 Test recipe", pending, error: None,
        on_cancel: |_| {}, on_confirm: |_| {},
    } }
}

#[test]
fn recipe_deletion_dialog_has_explicit_safe_cancel_and_confirmation_controls() {
    for pending in [false, true] {
        let mut dom = VirtualDom::new_with_props(
            DeletionDialogTestRoot,
            DeletionDialogTestRootProps { pending },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("<dialog"), "{html}");
        assert!(html.contains("aria-labelledby=\"delete-recipe-title\""));
        assert!(html.contains("CP8 Test recipe"));
        assert!(html.contains("cannot be undone"));
        assert!(html.contains("id=\"cancel-recipe-deletion\""));
        assert_eq!(html.matches("<button").count(), 2);
        assert_eq!(html.contains("role=\"status\""), pending, "{html}");
        assert!(!html.contains(" disabled"));
        assert_eq!(html.matches(&format!("aria-disabled={pending}")).count(), 2);
    }
}

#[component]
fn DeletionDialogEventsTestRoot(
    pending: bool,
    canceled: std::rc::Rc<std::cell::Cell<usize>>,
    confirmed: std::rc::Rc<std::cell::Cell<usize>>,
) -> Element {
    rsx! { RecipeDeletionDialog {
        title: "Synthetic", pending, error: None,
        on_cancel: move |_| canceled.set(canceled.get() + 1),
        on_confirm: move |_| confirmed.set(confirmed.get() + 1),
    } }
}

#[test]
fn deletion_dialog_cancel_never_confirms_and_pending_controls_do_not_dispatch_actions() {
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::rc::Rc;
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for pending in [false, true] {
        let canceled = Rc::new(std::cell::Cell::new(0));
        let confirmed = Rc::new(std::cell::Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            DeletionDialogEventsTestRoot,
            DeletionDialogEventsTestRootProps {
                pending,
                canceled: canceled.clone(),
                confirmed: confirmed.clone(),
            },
        );
        let mutations = dom.rebuild_to_vec();
        let click_handlers = mutations
            .edits
            .iter()
            .filter_map(|edit| match edit {
                dioxus_core::Mutation::NewEventListener { name, id } if *name == "click" => {
                    Some(*id)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        // The dialog/backdrop handlers precede its two actual button handlers.
        let buttons = &click_handlers[click_handlers.len() - 2..];
        for (index, button) in buttons.iter().enumerate() {
            dom.runtime().handle_event(
                "click",
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(
                        SerializedMouseData::default(),
                    ))) as Rc<dyn std::any::Any>,
                    true,
                ),
                *button,
            );
            assert_eq!(canceled.get(), usize::from(!pending));
            assert_eq!(
                confirmed.get(),
                usize::from(!pending && index == 1),
                "Cancel must never dispatch the separately confirmed action"
            );
        }
    }
}

#[component]
fn DeletionFeedbackTestRoot(pending: bool, failed: bool) -> Element {
    let error = failed.then(|| {
        recipe_mutation_feedback(
            RecipeMutation::Delete,
            &ServerFnError::new("private database internals"),
        )
        .to_string()
    });
    rsx! { RecipeDeletionFeedback { pending, error } }
}

#[test]
fn deletion_feedback_announces_pending_or_safe_failure_and_preserves_a_separate_tab_exit() {
    for (pending, failed) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut dom = VirtualDom::new_with_props(
            DeletionFeedbackTestRoot,
            DeletionFeedbackTestRootProps { pending, failed },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.contains("role=\"status\""), pending, "{html}");
        assert_eq!(
            html.contains("role=\"alert\""),
            failed && !pending,
            "{html}"
        );
        assert_eq!(
            html.contains("Check your Dashboard"),
            failed && !pending,
            "{html}"
        );
        if failed && !pending {
            assert!(html.contains("href=\"/dashboard\""));
            assert!(html.contains("target=\"_blank\""));
            assert!(html.contains("rel=\"noopener\""));
        }
        assert!(!html.contains("private database internals"), "{html}");
    }
}

#[component]
fn MutationControlTestRoot(pending: bool, unavailable: bool) -> Element {
    rsx! { RecipeActionControl {
        pending, unavailable, label: "Save visibility", pending_label: "Saving visibility…",
        on_action: |_| {}
    } }
}

#[test]
fn mutation_controls_keep_a_focusable_touch_target_and_announce_pending() {
    for (pending, unavailable) in [(false, false), (true, false), (false, true)] {
        let mut dom = VirtualDom::new_with_props(
            MutationControlTestRoot,
            MutationControlTestRootProps {
                pending,
                unavailable,
            },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.matches("<button").count(), 1, "{html}");
        assert!(!html.contains(" disabled"), "{html}");
        assert!(html.contains("touch-target"), "{html}");
        assert!(
            html.contains(&format!("aria-disabled={}", pending || unavailable)),
            "{html}"
        );
        assert!(html.contains(&format!("aria-busy={pending}")), "{html}");
        assert_eq!(html.contains("role=\"status\""), pending, "{html}");
    }
}

#[component]
fn MutationGuardTestRoot(
    pending: bool,
    unavailable: bool,
    calls: std::rc::Rc<std::cell::Cell<usize>>,
) -> Element {
    rsx! { RecipeActionControl {
        pending, unavailable, label: "Restore", pending_label: "Restoring…",
        on_action: move |_| calls.set(calls.get() + 1)
    } }
}

#[test]
fn mutation_control_blocks_pending_and_unavailable_activation() {
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::rc::Rc;
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for (pending, unavailable, expected) in [(true, false, 0), (false, true, 0), (false, false, 1)]
    {
        let calls = Rc::new(std::cell::Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            MutationGuardTestRoot,
            MutationGuardTestRootProps {
                pending,
                unavailable,
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
            .expect("mutation control handler");
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

#[test]
fn visibility_sync_does_not_subscribe_to_save_completion_or_overwrite_failed_selection() {
    let source = include_str!("recipe_detail.rs");
    let effect = source
        .rsplit("// Tags (signal-based, loaded conditionally based on ownership)")
        .next()
        .unwrap()
        .split("// Load tags once recipe is available")
        .next()
        .unwrap();
    assert!(effect.contains("is_saving_visibility.peek()"));
    assert!(!effect.contains("is_saving_visibility()"));
    assert!(effect.contains("visibility_selection_differs_from_recipe"));
}

#[test]
fn recipe_mutation_handlers_guard_live_pending_state_and_scope_restore_feedback() {
    let source = include_str!("recipe_detail.rs");
    let visibility = source
        .split("let save_visibility =")
        .nth(1)
        .unwrap()
        .split("let id_for_fork")
        .next()
        .unwrap();
    assert!(visibility.contains("is_saving_visibility.peek()"));
    assert!(visibility.contains("recipe_mutation_feedback("));
    assert!(visibility.contains("RecipeMutation::Visibility"));
    assert!(!visibility.contains("error.to_string()"));
    let fork = source
        .split("let on_fork =")
        .nth(1)
        .unwrap()
        .split("// ── Delete handler")
        .next()
        .unwrap();
    assert!(fork.contains("is_forking.peek()"));
    assert!(fork.contains("recipe_mutation_feedback(RecipeMutation::Fork"));
    assert!(!fork.contains("error.to_string()"));
    let history = source
        .split("h2 { \"Version history\" }")
        .nth(1)
        .unwrap()
        .split("/// Full-page, read-only")
        .next()
        .unwrap();
    assert!(history.contains("restoring_version.peek()"));
    assert!(history.contains("RecipeActionControl"));
    assert!(history.contains("RecipeMutation::Restore"));
    assert!(history.contains("failed_version == version_number"));
    assert!(history.contains("role: \"alert\""));
}
