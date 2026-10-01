use super::*;

#[test]
fn historical_comparison_keeps_pair_on_failure_and_claims_requests_exclusively() {
    let mut state = HistoricalComparisonState::new(5);
    assert_eq!((state.from, state.to), (1, 5));
    assert!(state.select(4, 2, 5));
    assert_eq!(state.begin(5), Some((4, 2)));
    assert!(state.begin(5).is_none());
    assert!(!state.select(1, 3, 5));
    state.finish(Err(()));
    assert!(state.failed && !state.pending);
    assert_eq!((state.from, state.to), (4, 2));
    assert_eq!(state.begin(5), Some((4, 2)));
    state.finish(Ok(RecipeVersionComparison {
        from_version_number: 4,
        to_version_number: 2,
        changes: vec![],
    }));
    assert!(!state.pending && !state.failed);
    assert!(state.comparison.is_some());
    assert!(state.select(2, 2, 5));
    assert!(state.comparison.is_none() && !state.failed);
    assert_eq!(
        state.begin(5),
        Some((2, 2)),
        "same snapshot is a valid no-difference comparison"
    );
}

#[test]
fn historical_comparison_rejects_out_of_bounds_and_inconsistent_completions() {
    let mut state = HistoricalComparisonState::new(5);
    assert!(!state.select(0, 2, 5));
    assert!(!state.select(2, 6, 5));
    assert!(state.begin(0).is_none());
    state.finish(Ok(RecipeVersionComparison {
        from_version_number: 1,
        to_version_number: 5,
        changes: vec![],
    }));
    assert!(state.comparison.is_none(), "no idle completion authority");
    state.begin(5);
    state.finish(Ok(RecipeVersionComparison {
        from_version_number: 2,
        to_version_number: 5,
        changes: vec![],
    }));
    assert!(state.failed && state.comparison.is_none() && !state.pending);
    assert!(state.select(3, 5, 5));
    assert!(!state.failed);
}

#[component]
fn HistoricalComparisonFeedbackRoot(pending: bool, failed: bool) -> Element {
    rsx! { HistoricalComparisonFeedback { pending, failed } }
}

#[test]
fn historical_comparison_feedback_has_named_pending_and_safe_retry_guidance() {
    for (pending, failed) in [(false, false), (true, false), (true, true), (false, true)] {
        let mut dom = VirtualDom::new_with_props(
            HistoricalComparisonFeedbackRoot,
            HistoricalComparisonFeedbackRootProps { pending, failed },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert_eq!(html.contains("role=\"status\""), pending);
        assert_eq!(html.contains("role=\"alert\""), failed && !pending);
        assert_eq!(html.contains("Retry comparison"), failed && !pending);
        assert!(!html.contains("ServerFnError"));
    }
}

#[test]
fn historical_snapshot_wiring_subscribes_before_returns_and_guards_live_pair() {
    let source = include_str!("recipe_detail.rs")
        .split("fn RecipeVersionDetailContents")
        .nth(1)
        .expect("keyed snapshot contents");
    assert!(source.contains("version_resource.state().read()"));
    assert!(source.contains("historical_comparison.write().begin("));
    assert!(source.contains("historical_comparison.write().select("));
    assert!(source.contains("if !version_resource.pending()"));
    assert!(source.contains("Retry comparison"));
    assert!(source.contains("historical-snapshot"));
    assert!(source.contains("set_focus(true)"));
    assert!(!source.contains("Some(error.to_string())"));
    assert!(source.contains("minmax(min(100%, 180px), 1fr)"));
}

fn substitute_snapshot_loader(node: VNode, probe: dioxus_core::DynamicNode) -> VNode {
    use dioxus_core::DynamicNode;
    let dynamic_nodes = node
        .dynamic_nodes
        .iter()
        .map(|dynamic| match dynamic {
            DynamicNode::Component(component)
                if component.name.ends_with("RecipeVersionDetailContents") =>
            {
                probe.clone()
            }
            DynamicNode::Fragment(children) => DynamicNode::Fragment(
                children
                    .iter()
                    .cloned()
                    .map(|child| substitute_snapshot_loader(child, probe.clone()))
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
fn SnapshotScopeTestRoot() -> Element {
    let route = use_signal(|| "recipe-a/5".to_string());
    *use_context::<read_recovery_tests::RecipeScopeControl>()
        .id
        .borrow_mut() = Some(route);
    let current = route();
    let (id, number) = current.split_once('/').unwrap();
    let node = RecipeVersionDetail(RecipeVersionDetailProps {
        id: id.into(),
        version_number: number.parse().unwrap(),
    })?;
    let probe = rsx! { read_recovery_tests::RecipeStateProbe { id: current } }?;
    Ok(substitute_snapshot_loader(
        node,
        probe.dynamic_nodes[0].clone(),
    ))
}

#[test]
fn snapshot_route_visit_replaces_state_for_recipe_and_version_including_return() {
    let control = read_recovery_tests::RecipeScopeControl::default();
    let mut dom = VirtualDom::new(SnapshotScopeTestRoot).with_root_context(control.clone());
    dom.rebuild_in_place();
    let mut route = control.id.borrow().unwrap();
    let mut state = control.state.borrow().unwrap();
    dom.in_runtime(|| state.set("old failed comparison".into()));
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    dom.in_runtime(|| route.set("recipe-a/5".into()));
    dom.render_immediate(&mut dioxus_core::NoOpMutations);
    assert!(dioxus_ssr::render(&dom).contains("old failed comparison"));
    for next in ["recipe-a/4", "recipe-b/5", "recipe-a/5"] {
        dom.in_runtime(|| route.set(next.into()));
        dom.render_immediate(&mut dioxus_core::NoOpMutations);
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains(next), "{html}");
        assert!(!html.contains("old failed comparison"), "{html}");
    }
}
