use super::*;

#[component]
fn NarrowComparisonTestRoot() -> Element {
    rsx! {
        VersionComparisonPanel {
            title: "Changes in v30",
            description: "Published differences from v29 to v30.",
            changes: vec![RecipeVersionChange {
                path: "/description".into(),
                op: "replace".into(),
                old_value: Some("Synthetic history version 29".into()),
                value: Some("Synthetic history version 30".into()),
            }],
        }
    }
}

#[test]
fn comparison_columns_shrink_to_available_width_and_field_headers_wrap() {
    let mut dom = VirtualDom::new(NarrowComparisonTestRoot);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("minmax(min(100%, 220px), 1fr)"), "{html}");
    assert!(html.contains("flex-wrap:wrap"), "{html}");
    assert!(html.contains("Synthetic history version 29"));
    assert!(html.contains("Synthetic history version 30"));
}

#[component]
fn ReadFeedbackTestRoot(kind: RecipeReadKind, pending: bool, failed: bool) -> Element {
    rsx! { RecipeReadFeedback { kind, pending, failed, onretry: |_| {} } }
}

#[test]
fn history_and_lineage_feedback_distinguish_loading_failure_retry_and_success() {
    for kind in [RecipeReadKind::History, RecipeReadKind::Lineage] {
        for (pending, failed) in [(true, false), (false, true), (true, true), (false, false)] {
            let mut dom = VirtualDom::new_with_props(
                ReadFeedbackTestRoot,
                ReadFeedbackTestRootProps {
                    kind,
                    pending,
                    failed,
                },
            );
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert_eq!(
                html.contains("role=\"alert\""),
                failed && !pending,
                "{html}"
            );
            assert_eq!(html.contains("role=\"status\""), pending, "{html}");
            assert_eq!(
                html.contains(kind.failure_message()),
                failed && !pending,
                "{html}"
            );
            assert_eq!(
                html.contains(kind.loading_message()),
                pending && !failed,
                "{html}"
            );
            assert_eq!(
                html.matches("<button").count(),
                usize::from(failed),
                "{html}"
            );
            assert!(!html.contains(" disabled"), "{html}");
            assert!(!html.contains("ServerFnError"), "{html}");
        }
    }
}

#[test]
fn recipe_reads_subscribe_to_independent_pending_states_and_guard_retries() {
    let source = include_str!("recipe_detail.rs")
        .split("pub fn RecipeDetail")
        .nth(1)
        .unwrap()
        .split("/// Full-page, read-only")
        .next()
        .unwrap();
    assert!(source.contains("versions_resource.state().read()"));
    assert!(source.contains("lineage_resource.state().read()"));
    assert!(source.contains("if !versions_resource.pending()"));
    assert!(source.contains("if !lineage_resource.pending()"));
    assert!(source.contains("kind: RecipeReadKind::History"));
    assert!(source.contains("kind: RecipeReadKind::Lineage"));
    assert!(!source.contains("if is_owner && !versions.is_empty()"));
}

#[derive(Clone, Default)]
struct RecipeScopeControl {
    id: std::rc::Rc<std::cell::RefCell<Option<Signal<String>>>>,
    state: std::rc::Rc<std::cell::RefCell<Option<Signal<String>>>>,
}

#[component]
fn RecipeStateProbe(id: String) -> Element {
    let state = use_signal(move || id);
    *use_context::<RecipeScopeControl>().state.borrow_mut() = Some(state);
    rsx! { p { "{state}" } }
}

fn substitute_recipe_loader(node: VNode, probe: dioxus_core::DynamicNode) -> VNode {
    use dioxus_core::DynamicNode;
    let dynamic_nodes = node
        .dynamic_nodes
        .iter()
        .map(|dynamic| match dynamic {
            DynamicNode::Component(component)
                if component.name.ends_with("RecipeDetailContents") =>
            {
                probe.clone()
            }
            DynamicNode::Fragment(children) => DynamicNode::Fragment(
                children
                    .iter()
                    .cloned()
                    .map(|child| substitute_recipe_loader(child, probe.clone()))
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
fn RecipeScopeTestRoot() -> Element {
    let id = use_signal(|| "recipe-a".to_string());
    *use_context::<RecipeScopeControl>().id.borrow_mut() = Some(id);
    let route = RecipeDetail(RecipeDetailProps { id: id() })?;
    let probe = rsx! { RecipeStateProbe { id: id() } }?;
    Ok(substitute_recipe_loader(
        route,
        probe.dynamic_nodes[0].clone(),
    ))
}

#[test]
fn recipe_route_replacement_remounts_reads_and_state_including_return_to_same_id() {
    let control = RecipeScopeControl::default();
    let mut dom = VirtualDom::new(RecipeScopeTestRoot).with_root_context(control.clone());
    dom.rebuild_in_place();
    let mut id = control.id.borrow().unwrap();
    let mut state = control
        .state
        .borrow()
        .expect("recipe contents must own a keyed scope");
    dom.in_runtime(|| state.set("loaded older versions and errors".to_string()));
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    dom.in_runtime(|| id.set("recipe-a".to_string()));
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert!(dioxus_ssr::render(&dom).contains("loaded older versions and errors"));
    for next in ["recipe-b", "recipe-a"] {
        dom.in_runtime(|| id.set(next.to_string()));
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(next), "{html}");
        assert!(!html.contains("loaded older versions and errors"), "{html}");
    }
}

#[test]
fn history_continuations_keep_failed_cursor_items_and_guard_pending_and_terminal_actions() {
    let mut page = HistoryContinuation::default();
    assert_eq!(page.begin(Some(6)), Some(6));
    assert_eq!(page.begin(Some(6)), None);
    page.finish(Err(()));
    assert!(page.failed);
    assert!(!page.pending);
    assert_eq!(page.next_before_version_number, Some(6));
    assert_eq!(
        page.begin(Some(99)),
        Some(6),
        "retry must not skip its failed cursor"
    );
    page.finish(Ok(RecipeVersionSummaryPage {
        items: vec![summary(5), summary(4)],
        next_before_version_number: Some(4),
    }));
    assert_eq!(
        page.items
            .iter()
            .map(|v| v.version_number)
            .collect::<Vec<_>>(),
        vec![5, 4]
    );
    assert!(!page.failed);
    assert_eq!(page.begin(Some(99)), Some(4));
    page.finish(Err(()));
    assert_eq!(
        page.items.len(),
        2,
        "loaded versions survive a later failure"
    );
    assert_eq!(page.begin(None), Some(4));
    page.finish(Ok(RecipeVersionSummaryPage {
        items: vec![summary(3), summary(2), summary(1)],
        next_before_version_number: None,
    }));
    assert_eq!(page.items.len(), 5);
    assert_eq!(
        page.begin(Some(99)),
        None,
        "terminal state must ignore a stale initial cursor"
    );
    assert!(page.requested);
}

#[test]
fn history_continuations_do_not_append_without_an_active_request_or_loop_on_empty_pages() {
    let mut page = HistoryContinuation::default();
    assert_eq!(page.begin(None), None);
    page.finish(Ok(RecipeVersionSummaryPage {
        items: vec![summary(99)],
        next_before_version_number: Some(99),
    }));
    assert!(page.items.is_empty());
    assert_eq!(page.begin(Some(6)), Some(6));
    page.finish(Ok(RecipeVersionSummaryPage {
        items: vec![],
        next_before_version_number: Some(6),
    }));
    assert_eq!(page.next_before_version_number, None);
    assert_eq!(page.begin(Some(6)), None);
}

fn summary(version_number: i32) -> RecipeVersionSummary {
    RecipeVersionSummary {
        version_number,
        title: "Synthetic".into(),
        image_url: None,
        notes: None,
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        is_current: false,
        change_summary: "Synthetic".into(),
    }
}

#[test]
fn difference_activation_retries_in_place_collapses_completed_panels_and_blocks_pending() {
    assert_eq!(
        difference_activation(true, Some(3), false, 3),
        DifferenceActivation::Ignore
    );
    assert_eq!(
        difference_activation(true, Some(3), false, 2),
        DifferenceActivation::Ignore
    );
    assert_eq!(
        difference_activation(false, Some(3), false, 3),
        DifferenceActivation::Collapse
    );
    assert_eq!(
        difference_activation(false, Some(3), true, 3),
        DifferenceActivation::Load
    );
    assert_eq!(
        difference_activation(false, Some(3), true, 2),
        DifferenceActivation::Load
    );
    assert_eq!(
        difference_activation(false, None, false, 3),
        DifferenceActivation::Load
    );
}

#[test]
fn current_history_paging_and_difference_controls_use_safe_feedback_and_live_guards() {
    let source = include_str!("recipe_detail.rs")
        .split("fn RecipeDetailContents")
        .nth(1)
        .unwrap()
        .split("/// Full-page, read-only")
        .next()
        .unwrap();
    assert!(source.contains("history_continuation.write().begin("));
    assert!(source.contains("difference_activation("));
    assert!(source.contains("is_loading_version_difference.peek()"));
    assert!(source.contains("version_difference_error.peek()"));
    assert!(source.contains("Retry differences"));
    assert!(source.contains("All versions loaded"));
    assert!(source.contains("RecipeActionControl"));
    assert!(!source.contains("Some(error.to_string())"));
}
