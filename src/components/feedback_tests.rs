//! Contracts for nonmodal T2 feedback and its first real consumer.
#[test]
fn feedback_is_named_nonmodal_and_dismissible() {
    let source = include_str!("base/feedback.rs");
    assert!(source.contains("role: \"status\""));
    assert!(source.contains("aria_live: \"polite\""));
    assert!(source.contains("aria_atomic: \"true\""));
    assert!(source.contains("key: \"{generation}\""));
    assert!(source.contains("aria_label: \"Dismiss notification\""));
    assert!(source.contains("role: \"tooltip\""));
    assert!(source.contains("aria_describedby:"));
    assert!(!source.contains("aria_modal"));
    assert!(!source.contains("showModal"));
}

#[test]
fn collection_feedback_guards_duplicate_requests_and_retains_capabilities() {
    let source = include_str!("../pages/collection_detail.rs")
        .split("#[cfg(test)]")
        .next()
        .unwrap();
    assert!(source.contains("FeedbackToast {"));
    assert_eq!(source.matches("TooltipAction {").count(), 2);
    assert!(source.contains("if link_pending() { return; }"));
    assert!(source.contains("disabled: link_pending()"));
    assert_eq!(source.matches("link_generation += 1").count(), 2);
    assert_eq!(
        source.matches("link_status.set(None)").count(),
        1,
        "only explicit dismissal clears feedback; a pending request retains it"
    );
    assert!(source
        .contains("collection_contents.collection.visibility != CollectionVisibility::Private"));
}

#[test]
fn feedback_has_glass_fallbacks_and_wrapping_without_page_backdrop() {
    let css = include_str!("../../assets/main.css");
    assert!(css.contains(".feedback-surface"));
    assert!(css.contains(".feedback-toast__message"));
    assert!(css.contains(".feedback-tooltip[hidden]"));
    assert!(css.contains("prefers-reduced-transparency"));
    assert!(css.contains("forced-colors"));
}
