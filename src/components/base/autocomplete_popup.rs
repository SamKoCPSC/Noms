//! Shared nonmodal suggestion surface. Focus and selection remain with the input.
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(any(target_arch = "wasm32", test))]
mod placement;

#[component]
pub(crate) fn AutocompleteList(
    id: String,
    accessible_name: String,
    query: String,
    active: Option<usize>,
    children: Element,
) -> Element {
    #[cfg(target_arch = "wasm32")]
    let runtime = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(None::<browser::Popup>)));
    #[cfg(target_arch = "wasm32")]
    {
        let cleanup = runtime.clone();
        use_drop(move || {
            cleanup.borrow_mut().take();
        });
        let runtime = runtime.clone();
        let panel = id.clone();
        use_effect(use_reactive((&query, &active), move |(query, active)| {
            let mut current = runtime.borrow_mut();
            if let Some(popup) = current.as_mut() {
                popup.update(&query, active);
            } else if let Some(popup) = browser::Popup::mount(&panel, &query, active) {
                *current = Some(popup);
            }
        }));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&query, active);
    rsx! {
        ul {
            id,
            class: "ingredient-autocomplete__list neumo-raised",
            role: "listbox",
            aria_label: accessible_name,
            {children}
        }
    }
}
