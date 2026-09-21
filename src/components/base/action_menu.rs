//! P2 anchored action shell. Children supply real links/buttons, not nested forms.
use dioxus::prelude::*;

/// Preserve graph-owned actions while escaping canvas clipping in a nonmodal top layer.
#[component]
pub fn ActionMenuBoundary(children: Element, on_dismiss: EventHandler<()>) -> Element {
    let id = use_hook(|| format!("menu-boundary-{}", uuid::Uuid::new_v4()));
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
    }
    let mount_id = id.clone();
    rsx! {
        div {
            id, style: "display: contents;",
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                {
                    use wasm_bindgen::JsCast;
                    use super::overlay_dom as dom;
                    let Some(root) = dom::element(&mount_id) else { return; };
                    let mut registrations = listeners.borrow_mut();
                    registrations.clear();
                    if let Some(panel) = dom::query(&root, "[role=menu]") {
                        let rect = dom::rect(&panel);
                        // Capture canvas-relative placement before promoting to the top layer.
                        dom::css(&panel, "position", "fixed");
                        dom::css(&panel, "inset", "auto");
                        dom::css(&panel, "margin", "0");
                        dom::css(&panel, "width", "min(256px, calc(100vw - 16px))");
                        dom::show_popover(&panel);
                        // Promotion can change wrapping on a narrow canvas. Clamp using
                        // the final surface dimensions, not its former canvas width.
                        let (width, height) = dom::viewport();
                        let position = super::overlay_logic::context_menu_position(
                            rect.left, rect.top, f64::from(panel.offset_width()),
                            super::overlay_logic::natural_height(panel.scroll_height(), panel.offset_height(), panel.client_height()),
                            width, height,
                        );
                        dom::css(&panel, "left", &format!("{}px", position.left));
                        dom::css(&panel, "top", &format!("{}px", position.top));
                        dom::css(&panel, "max-height", &format!("{}px", position.max_height));
                    }
                    for name in ["pointerdown", "focusin"] {
                        let root = root.clone();
                        registrations.push(dom::Listener::new(&dom::document(), name, false, move |event| {
                            let target = event.target().and_then(|target| target.dyn_into::<web_sys::Node>().ok());
                            if !root.contains(target.as_ref()) { on_dismiss.call(()); }
                        }));
                    }
                    // Outside wheel input dismisses; internal scrolling remains usable.
                    // Do not dismiss on scroll itself: mounted focus may scroll the page
                    // while the freshly opened menu is being promoted to the top layer.
                    let scroll_root = root.clone();
                    registrations.push(dom::Listener::new(&dom::document(), "wheel", true, move |event| {
                        let target = event.target().and_then(|target| target.dyn_into::<web_sys::Node>().ok());
                        if !scroll_root.contains(target.as_ref()) { on_dismiss.call(()); }
                    }));
                    registrations.push(dom::Listener::new(&web_sys::window().expect("browser window"), "resize", false, move |_| on_dismiss.call(())));
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&mount_id, on_dismiss);
            },
            {children}
        }
    }
}

#[component]
pub fn ActionMenu(children: Element) -> Element {
    let id = use_hook(|| format!("action-menu-{}", uuid::Uuid::new_v4()));
    let panel_id = format!("{id}-panel");
    let trigger_id = format!("{id}-trigger");
    #[cfg(target_arch = "wasm32")]
    {
        let cleanup_id = id.clone();
        use_drop(move || super::account_menu::dispose_account_menu(&cleanup_id));
    }
    let mount_id = id.clone();
    rsx! {
        div {
            id,
            onmounted: move |_| {
                #[cfg(target_arch = "wasm32")]
                super::account_menu::mount_menu(&mount_id, "[data-action-trigger]", "[data-action-panel]");
                #[cfg(not(target_arch = "wasm32"))]
                let _ = &mount_id;
            },
            button {
                id: trigger_id.clone(), r#type: "button",
                class: "btn btn-secondary touch-target",
                "data-action-trigger": "",
                aria_haspopup: "menu", aria_expanded: "false", aria_controls: panel_id.clone(),
                "Recipe actions"
                span { aria_hidden: "true", " ⋯" }
            }
            div {
                id: panel_id, hidden: true, role: "menu",
                aria_labelledby: trigger_id,
                "data-action-panel": "",
                class: "feedback-surface account-menu-panel action-menu-panel",
                {children}
            }
        }
    }
}
