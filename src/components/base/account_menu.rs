//! P1 browser behavior in Rust. Closing hides, never unmounts the native POST form.
use super::{
    overlay_dom as dom,
    overlay_logic::{menu_position, menu_target, natural_height},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{Element, Event, EventTarget, HtmlElement, KeyboardEvent, Node};

struct AccountMenu {
    root: HtmlElement,
    trigger: HtmlElement,
    panel: HtmlElement,
}
impl AccountMenu {
    fn items(&self) -> Vec<HtmlElement> {
        let Ok(nodes) = self.panel.query_selector_all("[role=\"menuitem\"]") else {
            return vec![];
        };
        (0..nodes.length())
            .filter_map(|index| nodes.item(index)?.dyn_into::<HtmlElement>().ok())
            .filter(|item| {
                !item.has_attribute("disabled")
                    && item.get_attribute("aria-disabled").as_deref() != Some("true")
            })
            .collect()
    }
    fn close(&self, restore: bool) {
        dom::hide_popover(&self.panel);
        let _ = self.trigger.set_attribute("aria-expanded", "false");
        if restore && self.trigger.is_connected() {
            dom::focus(&self.trigger, false);
        }
    }
    fn position(&self) {
        if self.panel.hidden() {
            return;
        }
        if self.trigger.get_client_rects().length() == 0 {
            self.close(false);
            return;
        }
        // Never expand the scroll container during focus scrolling (P1 regression).
        let height = natural_height(
            self.panel.scroll_height(),
            self.panel.offset_height(),
            self.panel.client_height(),
        );
        let (viewport_width, viewport_height) = dom::viewport();
        let at = menu_position(
            dom::rect(&self.trigger),
            f64::from(self.panel.offset_width()),
            height,
            viewport_width,
            viewport_height,
        );
        dom::css(&self.panel, "left", &format!("{}px", at.left));
        dom::css(&self.panel, "top", &format!("{}px", at.top));
        dom::css(&self.panel, "max-height", &format!("{}px", at.max_height));
    }
    fn open(&self, last: bool) {
        dom::show_popover(&self.panel);
        let _ = self.trigger.set_attribute("aria-expanded", "true");
        self.position();
        if !self.panel.hidden() {
            let items = self.items();
            if let Some(item) = if last { items.last() } else { items.first() } {
                dom::focus(item, false);
            }
        }
    }
    fn keydown(&self, event: &KeyboardEvent) {
        match event.key().as_str() {
            "Escape" => {
                event.prevent_default();
                event.stop_propagation();
                self.close(true);
            }
            // Let the browser perform sequential traversal from the trigger.
            "Tab" => self.close(true),
            key => {
                let items = self.items();
                let active = dom::document().active_element();
                let index = items.iter().position(|item| {
                    active
                        .as_ref()
                        .is_some_and(|active| active == item.as_ref())
                });
                let labels: Vec<_> = items
                    .iter()
                    .map(|item| item.text_content().unwrap_or_default())
                    .collect();
                let modified = event.ctrl_key() || event.meta_key() || event.alt_key();
                if (!modified || key.chars().count() != 1)
                    && menu_target(key, index, &labels).is_some()
                {
                    let next = menu_target(key, index, &labels).expect("matched target");
                    event.prevent_default();
                    dom::focus(&items[next], false);
                } else if key == " " {
                    if let Some(item) = index
                        .and_then(|index| items.get(index))
                        .filter(|item| item.tag_name() == "A")
                    {
                        event.prevent_default();
                        item.click();
                    }
                }
            }
        }
    }
}
struct Registration {
    menu: Rc<AccountMenu>,
    _listeners: Vec<dom::Listener>,
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.menu.close(false);
    }
}
thread_local! {static MENUS:RefCell<HashMap<String,Registration>>=RefCell::new(HashMap::new());}
fn listen(
    menu: &Rc<AccountMenu>,
    target: &EventTarget,
    name: &'static str,
    capture: bool,
    callback: impl Fn(&AccountMenu, Event) + 'static,
) -> dom::Listener {
    let weak = Rc::downgrade(menu);
    dom::Listener::new(target, name, capture, move |event| {
        if let Some(menu) = weak.upgrade() {
            callback(&menu, event);
        }
    })
}
pub(crate) fn mount_account_menu(id: &str) {
    mount_menu(id, "[data-account-trigger]", "[data-account-panel]");
}

/// Shared nonmodal menu lifecycle; consumers retain their own actions and permissions.
pub(crate) fn mount_menu(id: &str, trigger_selector: &str, panel_selector: &str) {
    let Some(root) = dom::element(id) else {
        return;
    };
    MENUS.with_borrow_mut(|registry| {
        if registry
            .get(id)
            .is_some_and(|registration| registration.menu.root == root)
        {
            return;
        }
        registry.remove(id);
        let Some(trigger) = dom::query(&root, trigger_selector) else {
            return;
        };
        let Some(panel) = dom::query(&root, panel_selector) else {
            return;
        };
        let menu = Rc::new(AccountMenu {
            root,
            trigger,
            panel,
        });
        let mut listeners = vec![
            listen(&menu, &menu.trigger, "click", false, |menu, _| {
                if menu.panel.hidden() {
                    menu.open(false);
                } else {
                    menu.close(true);
                }
            }),
            listen(&menu, &menu.trigger, "keydown", false, |menu, event| {
                if let Some(event) = event.dyn_ref::<KeyboardEvent>() {
                    if matches!(event.key().as_str(), "ArrowDown" | "ArrowUp") {
                        event.prevent_default();
                        menu.open(event.key() == "ArrowUp");
                    }
                }
            }),
            listen(&menu, &menu.panel, "keydown", false, |menu, event| {
                if let Some(event) = event.dyn_ref::<KeyboardEvent>() {
                    menu.keydown(event);
                }
            }),
            listen(&menu, &menu.panel, "click", false, |menu, event| {
                if event
                    .target()
                    .and_then(|target| target.dyn_into::<Element>().ok())
                    .and_then(|target| target.closest("[role=\"menuitem\"]").ok().flatten())
                    .is_some()
                {
                    menu.close(true);
                }
                // Never prevent default: links and the logout form keep native activation.
            }),
        ];
        for name in ["pointerdown", "focusin"] {
            listeners.push(listen(
                &menu,
                &dom::document(),
                name,
                false,
                |menu, event| {
                    let target = event
                        .target()
                        .and_then(|target| target.dyn_into::<Node>().ok());
                    if !menu.panel.hidden() && !menu.root.contains(target.as_ref()) {
                        menu.close(false);
                    }
                },
            ));
        }
        listeners.push(listen(
            &menu,
            &web_sys::window().expect("browser window"),
            "resize",
            false,
            |menu, _| menu.position(),
        ));
        listeners.push(listen(
            &menu,
            &dom::document(),
            "scroll",
            true,
            |menu, _| menu.position(),
        ));
        registry.insert(
            id.into(),
            Registration {
                menu,
                _listeners: listeners,
            },
        );
    });
}
pub(crate) fn dispose_account_menu(id: &str) {
    MENUS.with_borrow_mut(|registry| {
        registry.remove(id);
    });
}
