//! Ordinary-dialog interaction boundary: native inertness, explicit Tab wrapping,
//! pending-state focus ownership, and mount/unmount scroll and focus restoration.
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/components/base/shared_modal.js")]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = syncModal)]
    fn sync_modal(id: &str, focus_selector: &str, pending: bool);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = disposeModal)]
    fn dispose_modal(id: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = wrapModalTab)]
    fn wrap_modal_tab(id: &str, backwards: bool) -> bool;
}

/// Mount while open. Native showModal makes the background inert; explicit Tab
/// wrapping keeps keyboard navigation inside, including when pending disables
/// every control. The caller retains authority over dismissal; the lifecycle
/// bridge restores prior scroll styles and focus on unmount.
#[component]
pub fn SharedModal(
    aria_labelledby: String,
    focus_selector: String,
    #[props(default)] pending: bool,
    #[props(default = true)] dismiss_on_backdrop: bool,
    on_dismiss: EventHandler<()>,
    // Specialized editors may suppress a drag-ending backdrop click without
    // suppressing keyboard dismissal. Ordinary dialogs use on_dismiss for both.
    #[props(default)] on_backdrop_dismiss: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    let id = use_hook(|| format!("shared-modal-{}", uuid::Uuid::new_v4()));
    #[cfg(target_arch = "wasm32")]
    {
        let effect_id = id.clone();
        use_effect(use_reactive(
            (&focus_selector, &pending),
            move |(selector, pending)| {
                sync_modal(&effect_id, &selector, pending);
            },
        ));
        let cleanup_id = id.clone();
        use_drop(move || dispose_modal(&cleanup_id));
    }
    let mounted_id = id.clone();
    let keyboard_id = id.clone();
    rsx! {
        dialog {
            id,
            class: "modal-overlay shared-modal",
            aria_labelledby,
            aria_modal: "true",
            aria_busy: pending,
            onkeydown: move |event| {
                #[cfg(target_arch = "wasm32")]
                if event.key() == Key::Tab
                    && wrap_modal_tab(&keyboard_id, event.modifiers().shift())
                {
                    event.prevent_default();
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&keyboard_id, event);
            },
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                sync_modal(&mounted_id, &focus_selector, pending);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&mounted_id, &focus_selector);
            },
            oncancel: move |event| {
                // Prevent native close even when pending; state owns unmount.
                event.prevent_default();
                if !pending { on_dismiss.call(()); }
            },
            onclick: move |_| {
                if dismiss_on_backdrop && !pending {
                    on_backdrop_dismiss.unwrap_or(on_dismiss).call(());
                }
            },
            {children}
        }
    }
}
