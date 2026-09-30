use super::*;

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
