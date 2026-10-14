use super::*;

#[test]
fn profile_retry_focus_requires_owned_focus_and_an_active_document() {
    for retry_requested in [false, true] {
        for document_has_focus in [false, true] {
            for body_has_focus in [false, true] {
                assert_eq!(
                    profile_retry_should_focus_heading(
                        retry_requested,
                        document_has_focus,
                        body_has_focus,
                    ),
                    retry_requested && document_has_focus && body_has_focus,
                    "retry={retry_requested}, document={document_has_focus}, body={body_has_focus}"
                );
            }
        }
    }
}

#[test]
fn profile_recovery_cancels_focus_transfer_when_the_user_leaves_retry() {
    let source = include_str!("user_profile.rs");
    assert!(source.contains("onfocusout: move |_| retry_focus_requested.set(false)"));
    assert!(source.contains("id: \"public-profile-title\", tabindex: \"-1\""));
    assert!(source.contains("let restore_retry_focus = *retry_focus_requested.peek();"));
    assert!(source.contains("document.has_focus().unwrap_or(false)"));
}

#[test]
fn profile_disclosure_content_wraps_long_display_names() {
    let css = include_str!("../../assets/user-pages.css");
    let rule = css.split(".user-profile .user-disclosure__body {").nth(1);
    assert!(
        rule.is_some(),
        "Profile empty-state copy needs scoped wrapping"
    );
    assert!(rule
        .unwrap()
        .split('}')
        .next()
        .unwrap()
        .contains("overflow-wrap: anywhere"));
}

#[test]
fn profile_pages_retry_failed_offsets_and_claim_requests_synchronously() {
    let mut page = ProfilePageState::<u32>::default();
    assert!(!page.begin(ProfilePageAction::LoadMore));
    assert!(!page.begin(ProfilePageAction::Retry));
    assert!(page.finish(page.request, Ok(((1..=12).collect(), true))));
    assert!(page.begin(ProfilePageAction::LoadMore));
    assert_eq!(page.request.offset, 12);
    assert!(!page.begin(ProfilePageAction::LoadMore));
    assert!(page.finish(page.request, Err(())));
    assert_eq!(page.items, (1..=12).collect::<Vec<_>>());
    assert!(!page.begin(ProfilePageAction::LoadMore));
    let failed_request = page.request;
    assert!(page.begin(ProfilePageAction::Retry));
    assert_eq!(page.request.offset, 12);
    assert!(!page.begin(ProfilePageAction::Retry));
    assert!(!page.finish(failed_request, Ok((vec![99], false))));
    assert!(page.finish(page.request, Ok(((13..=24).collect(), true))));
    assert_eq!(page.items, (1..=24).collect::<Vec<_>>());
    assert!(!page.failed);
    assert!(page.begin(ProfilePageAction::LoadMore));
    assert_eq!(page.request.offset, 24);
    assert!(page.finish(page.request, Ok((vec![25], false))));
    assert_eq!(page.items, (1..=25).collect::<Vec<_>>());
    assert!(!page.begin(ProfilePageAction::LoadMore));
    assert!(!page.begin(ProfilePageAction::Retry));
    assert!(!page.finish(page.request, Ok((vec![26], false))));
}

#[test]
fn profile_initial_failures_retry_without_appending_and_empty_pages_stop() {
    let mut page = ProfilePageState::<u32>::default();
    assert!(page.finish(page.request, Err(())));
    assert!(page.begin(ProfilePageAction::Retry));
    assert_eq!(page.request.offset, 0);
    assert!(page.finish(page.request, Ok((vec![1], true))));
    assert!(page.begin(ProfilePageAction::LoadMore));
    assert!(page.finish(page.request, Ok((vec![], true))));
    assert!(!page.has_more);
    assert_eq!(page.items, vec![1]);
}

#[test]
fn profile_pagination_overflow_never_claims_a_request() {
    let mut page = ProfilePageState::<u32> {
        pending: false,
        ..Default::default()
    };
    page.request.offset = i64::MAX;
    assert!(!page.begin(ProfilePageAction::LoadMore));
    page.request.offset = 0;
    page.request.generation = u64::MAX;
    assert!(!page.begin(ProfilePageAction::LoadMore));
    assert!(!page.pending);
}

#[component]
fn PaginationTestRoot(
    kind: ProfilePageKind,
    pending: bool,
    failed: bool,
    has_more: bool,
) -> Element {
    rsx! { ProfilePagination { kind, pending, failed, has_more, count: 12, onaction: |_| {} } }
}

#[test]
fn profile_pagination_has_safe_feedback_and_a_stable_focusable_control() {
    for kind in [ProfilePageKind::Recipes, ProfilePageKind::Collections] {
        for (pending, failed, has_more) in [
            (false, false, true),
            (true, false, true),
            (false, true, true),
            (true, true, true),
            (false, false, false),
        ] {
            let mut dom = VirtualDom::new_with_props(
                PaginationTestRoot,
                PaginationTestRootProps {
                    kind,
                    pending,
                    failed,
                    has_more,
                },
            );
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert_eq!(html.matches("<button").count(), 1, "{html}");
            assert!(html.contains("touch-target"), "{html}");
            assert!(html.contains("profile-pagination"), "{html}");
            assert!(!html.contains(" disabled"), "{html}");
            assert!(
                html.contains(&format!(
                    "aria-disabled={}",
                    pending || (!failed && !has_more)
                )),
                "{html}"
            );
            assert_eq!(
                html.contains("role=\"alert\""),
                failed && !pending,
                "{html}"
            );
            assert!(!html.contains("ServerFnError"), "{html}");
            assert!(html.contains(kind.noun()), "{html}");
        }
    }
}

#[derive(Clone, Default)]
struct ProfileScopeControl {
    username: std::rc::Rc<std::cell::RefCell<Option<Signal<String>>>>,
    local_state: std::rc::Rc<std::cell::RefCell<Option<Signal<String>>>>,
}

#[component]
fn ProfileStateProbe(username: String) -> Element {
    let state = use_signal(move || username);
    *use_context::<ProfileScopeControl>()
        .local_state
        .borrow_mut() = Some(state);
    rsx! { p { "{state}" } }
}

fn substitute_profile_loader(node: VNode, probe: dioxus_core::DynamicNode) -> VNode {
    use dioxus_core::DynamicNode;
    let dynamic_nodes = node
        .dynamic_nodes
        .iter()
        .map(|dynamic| match dynamic {
            DynamicNode::Component(component)
                if component.name.ends_with("UserProfileContents") =>
            {
                probe.clone()
            }
            DynamicNode::Fragment(children) => DynamicNode::Fragment(
                children
                    .iter()
                    .cloned()
                    .map(|child| substitute_profile_loader(child, probe.clone()))
                    .collect(),
            ),
            _ => dynamic.clone(),
        })
        .collect::<Vec<_>>();
    VNode::new(
        node.key.clone(),
        node.template,
        dynamic_nodes.into_boxed_slice(),
        node.dynamic_attrs.clone(),
    )
}

#[component]
fn ProfileScopeTestRoot() -> Element {
    let username = use_signal(|| "profile-a".to_string());
    *use_context::<ProfileScopeControl>().username.borrow_mut() = Some(username);
    let route = UserProfile(UserProfileProps {
        username: username(),
    })?;
    let probe = rsx! { ProfileStateProbe { username: username() } }?;
    Ok(substitute_profile_loader(
        route,
        probe.dynamic_nodes[0].clone(),
    ))
}

#[test]
fn profile_username_change_remounts_state_but_same_username_preserves_it() {
    let control = ProfileScopeControl::default();
    let mut dom = VirtualDom::new(ProfileScopeTestRoot).with_root_context(control.clone());
    dom.rebuild_in_place();
    let mut username = control.username.borrow().unwrap();
    let mut state = control
        .local_state
        .borrow()
        .expect("keyed profile contents must own state");
    dom.in_runtime(|| state.set("loaded page three".to_string()));
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    dom.in_runtime(|| username.set("profile-a".to_string()));
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert!(dioxus_ssr::render(&dom).contains("loaded page three"));
    for next in ["profile-b", "profile-a"] {
        dom.in_runtime(|| username.set(next.to_string()));
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(next), "{html}");
        assert!(!html.contains("loaded page three"), "{html}");
    }
}
