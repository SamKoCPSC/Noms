//! T2 nonmodal feedback. Toast lifetime belongs to the consumer: messages persist
//! until dismissed/replaced. Tooltips supplement an already named action button.
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/components/base/feedback.js")]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = mountTooltip)]
    fn mount_tooltip(id: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = disposeTooltip)]
    fn dispose_tooltip(id: &str);
}

/// One persistent, politely announced result. Errors are not automatically urgent.
/// Render even without a message so the live region exists before updates arrive.
#[component]
pub fn FeedbackToast(
    message: Option<Result<String, String>>,
    #[props(default)] generation: u64,
    on_dismiss: EventHandler<()>,
) -> Element {
    let text = message
        .as_ref()
        .map(|m| match m {
            Ok(s) | Err(s) => s.clone(),
        })
        .unwrap_or_default();
    let error = matches!(message, Some(Err(_)));
    let surface = if message.is_some() {
        "feedback-surface feedback-toast"
    } else {
        "feedback-toast--empty"
    };
    rsx! {
        div { class: "feedback-host",
            div { class: surface,
                if message.is_some() {
                    span { class: if error { "feedback-toast__icon feedback-toast__icon--error" } else { "feedback-toast__icon" },
                        aria_hidden: "true", if error { "!" } else { "✓" }
                    }
                }
                div { class: "feedback-toast__message", role: "status", aria_live: "polite", aria_atomic: "true",
                    // Replace this node on completion, even for identical text,
                    // while keeping the live region itself mounted.
                    span { key: "{generation}", "{text}" }
                }
                if message.is_some() {
                    button { r#type: "button", class: "btn btn-ghost feedback-toast__close",
                        aria_label: "Dismiss notification", onclick: move |_| on_dismiss.call(()), "×" }
                }
            }
        }
    }
}

/// A normal button with supplementary, noninteractive hover/focus help.
#[component]
pub fn TooltipAction(
    label: String,
    description: String,
    #[props(default)] disabled: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    let id = use_hook(|| format!("feedback-{}", uuid::Uuid::new_v4()));
    let tooltip_id = format!("{id}-tip");
    #[cfg(target_arch = "wasm32")]
    {
        let cleanup_id = id.clone();
        use_drop(move || dispose_tooltip(&cleanup_id));
    }
    let mount_id = id.clone();
    rsx! {
        span { id, class: "feedback-trigger",
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                mount_tooltip(&mount_id);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = &mount_id;
            },
            button { r#type: "button", class: "btn btn-secondary touch-target", disabled,
                aria_describedby: tooltip_id.clone(), onclick, "{label}" }
            span { id: tooltip_id, class: "feedback-surface feedback-tooltip", role: "tooltip", hidden: true, "{description}" }
        }
    }
}
