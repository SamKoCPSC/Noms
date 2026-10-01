use super::*;

#[test]
fn graph_feedback_hides_transport_and_database_details() {
    for error in [
        ServerFnError::new("database connection details"),
        ServerFnError::StreamError("private response body".into()),
    ] {
        let message = server_error_message(&error);
        assert!(message.contains("try again"));
        assert!(!message.contains("database connection"));
        assert!(!message.contains("private response"));
    }
    assert!(is_recipe_unavailable_error(&server_error_message(
        &ServerFnError::new("Recipe not found")
    )));
}

#[component]
fn GraphClaimRoot() -> Element {
    let mut pending = use_signal(|| false);
    let a = VersionGraphNodeId {
        recipe_id: uuid::Uuid::from_u128(1),
        version_number: 1,
    };
    let b = VersionGraphNodeId {
        version_number: 2,
        ..a
    };
    assert!(claim_version_graph_comparison(pending, None, Some(b)).is_none());
    assert!(claim_version_graph_comparison(pending, Some(a), Some(a)).is_none());
    assert_eq!(
        claim_version_graph_comparison(pending, Some(a), Some(b)),
        Some((a, b))
    );
    assert!(claim_version_graph_comparison(pending, Some(b), Some(a)).is_none());
    pending.set(false);
    assert_eq!(
        claim_version_graph_comparison(pending, Some(b), Some(a)),
        Some((b, a))
    );
    rsx! { p { "Exclusive directional comparison" } }
}

#[test]
fn graph_comparison_claims_one_request_and_rejects_missing_or_equal_pairs() {
    let mut dom = VirtualDom::new(GraphClaimRoot);
    dom.rebuild_in_place();
    assert!(dioxus_ssr::render(&dom).contains("Exclusive directional comparison"));
}

#[test]
fn graph_initial_retry_and_comparison_controls_have_live_guards_and_stable_focus() {
    let source = include_str!("recipe_version_graph.rs")
        .split("pub fn RecipeVersionGraph")
        .nth(1)
        .unwrap()
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    assert!(source.contains("graph_resource.state().read()"));
    assert!(source.contains("if !graph_resource.pending()"));
    assert!(source.contains("Retry version graph"));
    assert!(source.contains("version-graph-content"));
    assert!(source.contains("set_focus(true)"));
    assert_eq!(source.matches("claim_version_graph_comparison(").count(), 2);
    assert!(source.contains("aria_busy: comparison_is_loading()"));
    assert!(source.matches("if *comparison_is_loading.peek()").count() >= 4);
}
