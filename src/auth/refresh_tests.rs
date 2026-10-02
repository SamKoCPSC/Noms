use super::*;

fn signed_in() -> AuthContext {
    let id = uuid::Uuid::from_u128(1);
    AuthContext {
        current_user_id: Some(id),
        current_user: Some(crate::auth::context::UserProfile {
            id,
            username: "test-owner".into(),
            display_name: "Test owner".into(),
            email: "synthetic@example.com".into(),
            avatar_url: None,
            bio: None,
        }),
        is_authenticated: true,
    }
}

#[test]
fn refresh_failure_retains_identity_and_requires_an_explicit_non_overlapping_retry() {
    let identity = signed_in();
    let mut state = AuthRefreshState::default();
    assert!(state.begin());
    assert!(!state.begin());
    assert!(state.pending);
    assert!(!state.visible());
    assert!(state.finish(&identity, &identity, Err(())).is_none());
    assert!(!state.pending);
    assert!(state.failed && state.visible());
    assert!(state.begin());
    assert!(state.failed && state.pending && state.visible());
    assert!(!state.begin());
    assert!(state.finish(&identity, &identity, Err(())).is_none());
    assert!(state.failed);
}

#[test]
fn successful_retry_accepts_verified_guest_and_keeps_a_focusable_completion_control() {
    let identity = signed_in();
    let guest = AuthContext::default();
    let mut state = AuthRefreshState::default();
    state.begin();
    state.finish(&identity, &identity, Err(()));
    state.dismiss();
    assert!(
        state.visible(),
        "Failed session checks must remain retryable"
    );
    state.begin();
    assert_eq!(
        state.finish(&identity, &identity, Ok(guest.clone())),
        Some(guest)
    );
    assert!(state.visible() && !state.failed && !state.pending);
    state.dismiss();
    assert!(!state.visible());
    // Initial success does not insert a banner into every page.
    state.begin();
    state.finish(&identity, &identity, Ok(identity.clone()));
    assert!(!state.visible());
}

#[test]
fn delayed_refresh_cannot_overwrite_a_new_identity_or_recent_profile_edit() {
    let initial = signed_in();
    for current in [
        AuthContext::default(),
        AuthContext {
            current_user: Some(crate::auth::context::UserProfile {
                display_name: "Recently saved name".into(),
                ..initial.current_user.clone().unwrap()
            }),
            ..initial.clone()
        },
    ] {
        let mut state = AuthRefreshState::default();
        state.begin();
        assert!(state
            .finish(&initial, &current, Ok(initial.clone()))
            .is_none());
        assert!(state.failed && !state.pending);
        assert!(state.begin());
        assert_eq!(
            state.finish(&current, &current, Ok(current.clone())),
            Some(current)
        );
    }
    let mut idle = AuthRefreshState::default();
    assert!(idle
        .finish(&initial, &initial, Ok(initial.clone()))
        .is_none());
}

#[test]
fn profile_refresh_rejects_malformed_or_inconsistent_identity_responses() {
    let owner = signed_in();
    assert_eq!(
        parse_auth_refresh(&serde_json::to_string(&owner).unwrap()),
        Ok(owner.clone())
    );
    assert_eq!(
        parse_auth_refresh(&serde_json::to_string(&AuthContext::default()).unwrap()),
        Ok(AuthContext::default())
    );
    for raw in ["internal response details", "{}", "null"] {
        assert!(parse_auth_refresh(raw).is_err());
    }
    for invalid in [
        AuthContext {
            is_authenticated: false,
            ..owner.clone()
        },
        AuthContext {
            current_user_id: None,
            ..owner.clone()
        },
        AuthContext {
            current_user: None,
            ..owner.clone()
        },
        AuthContext {
            current_user_id: Some(uuid::Uuid::from_u128(2)),
            ..owner
        },
    ] {
        assert!(parse_auth_refresh(&serde_json::to_string(&invalid).unwrap()).is_err());
    }
}

#[component]
fn FeedbackRoot(pending: bool, failed: bool) -> Element {
    rsx! { AuthRefreshFeedback { pending, failed, on_action: |_| {} } }
}

#[test]
fn refresh_feedback_preserves_one_touch_control_and_safe_status_across_retry_completion() {
    for (pending, failed) in [(false, true), (true, true), (false, false)] {
        let mut dom =
            VirtualDom::new_with_props(FeedbackRoot, FeedbackRootProps { pending, failed });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.matches("<button").count(), 1, "{html}");
        assert!(html.contains("touch-target"));
        assert!(html.contains(&format!("aria-disabled={pending}")));
        assert!(!html.contains(" disabled"));
        assert_eq!(html.contains("role=\"alert\""), failed && !pending);
        assert_eq!(html.contains("role=\"status\""), !failed || pending);
        assert!(html.contains(if pending {
            "Checking…"
        } else if failed {
            "Retry sign-in check"
        } else {
            "Dismiss"
        }));
        assert!(!html.contains("Sign in to continue"));
        assert!(!html.contains("internal response"));
    }
}

#[component]
fn GuardRoot(pending: bool, calls: std::rc::Rc<std::cell::Cell<usize>>) -> Element {
    rsx! { AuthRefreshFeedback { pending, failed: true, on_action: move |_| calls.set(calls.get() + 1) } }
}

#[test]
fn refresh_feedback_pending_handler_cannot_dispatch_another_request() {
    use dioxus::html::{SerializedHtmlEventConverter, SerializedMouseData};
    use std::rc::Rc;
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    for pending in [false, true] {
        let calls = Rc::new(std::cell::Cell::new(0));
        let mut dom = VirtualDom::new_with_props(
            GuardRoot,
            GuardRootProps {
                pending,
                calls: calls.clone(),
            },
        );
        let mutations = dom.rebuild_to_vec();
        let button = mutations
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
            button,
        );
        assert_eq!(calls.get(), usize::from(!pending));
    }
}

#[test]
fn shared_shell_owns_one_refresh_without_route_or_page_reset() {
    let shell = include_str!("../components/app_layout.rs");
    let app = include_str!("../main.rs");
    assert_eq!(shell.matches("AuthRefresh {}").count(), 1);
    assert!(!app.contains("Request::get(\"/api/user_profile\")"));
    let refresh = include_str!("refresh.rs");
    assert!(refresh.contains("state.write().begin()"));
    assert!(refresh.contains("auth.peek().clone()"));
    assert!(refresh.contains("state.write().finish("));
    assert!(!refresh.contains("location().reload"));
}

#[test]
fn session_activity_during_a_check_is_coalesced_into_one_follow_up_check() {
    let owner = signed_in();
    let mut state = AuthRefreshState::default();
    assert!(state.begin());
    assert!(!state.begin());
    assert!(!state.begin());
    state.finish(&owner, &owner, Ok(owner.clone()));
    assert!(
        state.take_queued(),
        "A focus/route change may replace the cookie mid-request"
    );
    assert!(!state.take_queued(), "Repeated events must coalesce");
    assert!(state.begin());
    assert_eq!(
        state.finish(&owner, &owner, Ok(AuthContext::default())),
        Some(AuthContext::default())
    );
    assert!(!state.take_queued());
}

#[test]
fn shared_session_checks_follow_activation_and_routes_with_owned_listener_cleanup() {
    let source = include_str!("refresh.rs");
    for event in ["\"focus\"", "\"pageshow\"", "\"visibilitychange\""] {
        assert!(
            source.contains(event),
            "Missing session activation event {event}"
        );
    }
    assert!(source.contains("document.hidden()"));
    assert!(source.contains("full_route_string()"));
    assert!(source.contains("remove_event_listener_with_callback"));
    assert!(source.contains("use_drop("));
    assert!(
        !source.contains("local_storage"),
        "Never publish identities or tokens through storage"
    );
}

#[test]
fn session_routes_reset_read_views_but_preserve_and_hide_original_forms() {
    let shell = include_str!("../components/app_layout.rs");
    assert!(shell.contains("SessionPage {}"));
    let gate = include_str!("../components/auth_required.rs");
    assert!(gate.contains("previously_authenticated"));
    assert!(gate.contains("hidden: !auth.is_authenticated"));
}
