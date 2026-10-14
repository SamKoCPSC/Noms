use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
fn position(panel_id: &str, trigger_id: &str) {
    use super::{
        overlay_dom as dom,
        overlay_logic::{menu_position, natural_height},
    };
    let (Some(panel), Some(trigger)) = (dom::element(panel_id), dom::element(trigger_id)) else {
        return;
    };
    dom::show_popover(&panel);
    let (width, height) = dom::viewport();
    let at = menu_position(
        dom::rect(&trigger),
        f64::from(panel.offset_width()),
        natural_height(
            panel.scroll_height(),
            panel.offset_height(),
            panel.client_height(),
        ),
        width,
        height,
    );
    dom::css(&panel, "left", &format!("{}px", at.left));
    dom::css(&panel, "top", &format!("{}px", at.top));
    dom::css(&panel, "max-height", &format!("{}px", at.max_height));
}

#[component]
pub(super) fn CollectionPickerPopover(
    trigger_id: String,
    pending: Signal<bool>,
    on_dismiss: EventHandler<bool>,
    children: Element,
) -> Element {
    let panel_id = format!("{trigger_id}-panel");
    #[cfg(target_arch = "wasm32")]
    let listeners = use_hook(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            Vec::<super::overlay_dom::Listener>::new(),
        ))
    });
    #[cfg(target_arch = "wasm32")]
    {
        let cleanup = listeners.clone();
        use_drop(move || cleanup.borrow_mut().clear());
        let focus_id = panel_id.clone();
        use_effect(move || {
            if pending() {
                if let Some(panel) = super::overlay_dom::element(&focus_id) {
                    super::overlay_dom::focus(&panel, true);
                }
            }
        });
    }
    let mount_panel = panel_id.clone();
    let resize_panel = panel_id.clone();
    let resize_trigger = trigger_id.clone();
    rsx! {
        div { id: panel_id, class: "feedback-surface collection-picker", role: "dialog", aria_label: "Add to collection", aria_modal: "false", aria_busy: pending(), tabindex: "-1",
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                {
                    use super::overlay_dom as dom;
                    use wasm_bindgen::JsCast;
                    position(&mount_panel, &trigger_id);
                    let Some(panel) = dom::element(&mount_panel) else { return; };
                    if let Some(search) = dom::query(&panel, "input[type=search]") { dom::focus(&search, true); } else { dom::focus(&panel, true); }
                    let mut registrations = listeners.borrow_mut();
                    registrations.clear();
                    for name in ["pointerdown", "focusin"] {
                        let panel = panel.clone();
                        let trigger_id = trigger_id.clone();
                        registrations.push(dom::Listener::new(&dom::document(), name, false, move |event| {
                            let target = event.target().and_then(|target| target.dyn_into::<web_sys::Node>().ok());
                            let in_trigger = dom::element(&trigger_id).is_some_and(|trigger| trigger.contains(target.as_ref()));
                            if !pending() && !panel.contains(target.as_ref()) && !in_trigger { on_dismiss.call(false); }
                        }));
                    }
                    for name in ["scroll", "resize"] {
                        let panel_id = mount_panel.clone();
                        let trigger_id = trigger_id.clone();
                        let target: web_sys::EventTarget = if name == "scroll" { dom::document().into() } else { web_sys::window().expect("browser window").into() };
                        registrations.push(dom::Listener::new(&target, name, true, move |_| position(&panel_id, &trigger_id)));
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&mount_panel, &trigger_id);
            },
            onresize: move |_| {
                #[cfg(target_arch = "wasm32")]
                position(&resize_panel, &resize_trigger);
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&resize_panel, &resize_trigger);
            },
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    event.prevent_default(); event.stop_propagation();
                    if !pending() { on_dismiss.call(true); }
                }
            },
            {children}
        }
    }
}
