use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(module = "/src/components/base/shared_modal.js")]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = syncModal)]
    fn sync_modal(id: &str, focus_selector: &str);
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = disposeModal)]
    fn dispose_modal(id: &str);
}

/// Ordinary modal shell. Mount while open; native showModal provides the focus
/// boundary/inert background, and the caller retains authority over dismissal.
#[component]
pub fn SharedModal(
    aria_labelledby: String,
    focus_selector: String,
    #[props(default)] pending: bool,
    #[props(default = true)] dismiss_on_backdrop: bool,
    on_dismiss: EventHandler<()>,
    children: Element,
) -> Element {
    let id = use_hook(|| format!("shared-modal-{}", uuid::Uuid::new_v4()));
    #[cfg(target_arch = "wasm32")]
    {
        let effect_id = id.clone();
        use_effect(use_reactive((&focus_selector,), move |(selector,)| {
            sync_modal(&effect_id, &selector);
        }));
        let cleanup_id = id.clone();
        use_drop(move || dispose_modal(&cleanup_id));
    }
    let mounted_id = id.clone();
    rsx! {
        dialog {
            id,
            class: "modal-overlay shared-modal",
            aria_labelledby,
            aria_modal: "true",
            aria_busy: pending,
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                sync_modal(&mounted_id, &focus_selector);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&mounted_id, &focus_selector);
            },
            oncancel: move |event| {
                // Prevent native close even when pending; state owns unmount.
                event.prevent_default();
                if !pending { on_dismiss.call(()); }
            },
            onclick: move |_| {
                if dismiss_on_backdrop && !pending { on_dismiss.call(()); }
            },
            {children}
        }
    }
}
