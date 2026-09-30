use dioxus::prelude::*;

/// Recovery for a failed resource that retains its last error while retrying.
/// Callers also guard the live resource state before restarting: the rendered
/// pending prop alone cannot prevent two clicks before a new render.
#[component]
pub fn LoadRecovery(
    pending: bool,
    message: String,
    retry_label: String,
    onretry: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            p { role: if pending { "status" } else { "alert" },
                if pending { "Trying again…" } else { "{message}" }
            }
            // aria-disabled keeps the retry button focused while its label
            // and announcement change, without allowing a competing request.
            button { r#type: "button", class: "btn btn-secondary touch-target",
                aria_disabled: pending,
                onclick: move |_| { if !pending { onretry.call(()); } },
                if pending { "Retrying…" } else { "{retry_label}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn RecoveryTestRoot(pending: bool) -> Element {
        rsx! { LoadRecovery {
            pending,
            message: "Your collections couldn’t be loaded. Please try again.",
            retry_label: "Retry collections",
            onretry: |_| {},
        } }
    }

    #[test]
    fn load_recovery_keeps_one_focusable_touch_target_during_retry() {
        for pending in [false, true] {
            let mut dom =
                VirtualDom::new_with_props(RecoveryTestRoot, RecoveryTestRootProps { pending });
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert_eq!(html.matches("<button").count(), 1, "{html}");
            assert!(html.contains("touch-target"), "{html}");
            assert!(html.contains(&format!("aria-disabled={pending}")), "{html}");
            assert!(!html.contains(" disabled"), "{html}");
            assert!(
                html.contains(if pending {
                    "Retrying…"
                } else {
                    "Retry collections"
                }),
                "{html}"
            );
        }
    }

    #[test]
    fn load_recovery_announces_a_failure_or_pending_retry_not_both() {
        for pending in [false, true] {
            let mut dom =
                VirtualDom::new_with_props(RecoveryTestRoot, RecoveryTestRootProps { pending });
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert!(
                html.contains(if pending {
                    "role=\"status\""
                } else {
                    "role=\"alert\""
                }),
                "{html}"
            );
            assert!(
                !html.contains(if pending {
                    "role=\"alert\""
                } else {
                    "role=\"status\""
                }),
                "{html}"
            );
            assert_eq!(
                html.contains("Your collections couldn’t be loaded."),
                !pending,
                "{html}"
            );
        }
    }
}
