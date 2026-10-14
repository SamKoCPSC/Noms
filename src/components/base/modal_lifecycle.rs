//! Testable modal lifecycle. The browser adapter below uses typed web-sys APIs.
use super::overlay_logic::{tab_target, TabTarget};

pub(super) trait ModalPlatform {
    type Element: Clone + PartialEq;
    type ScrollSnapshot;
    fn dialog(&self, id: &str) -> Option<Self::Element>;
    fn active(&self) -> Option<Self::Element>;
    fn target(&self, dialog: &Self::Element, selector: &str) -> Option<Self::Element>;
    fn open(&self, dialog: &Self::Element, target: Option<&Self::Element>) -> bool;
    fn close(&self, dialog: &Self::Element);
    fn lock_scroll(&self) -> Self::ScrollSnapshot;
    fn restore_scroll(&self, snapshot: Self::ScrollSnapshot);
    fn connected(&self, element: &Self::Element) -> bool;
    fn contains(&self, parent: &Self::Element, child: &Self::Element) -> bool;
    fn focus(&self, element: &Self::Element, prevent_scroll: bool);
    fn focus_dialog(&self, element: &Self::Element, prevent_scroll: bool);
    fn controls(&self, dialog: &Self::Element) -> Vec<Self::Element>;
    fn is_open(&self, dialog: &Self::Element) -> bool;
    fn suspended(&self, _dialog: &Self::Element) -> bool {
        false
    }
}
struct ModalEntry<E> {
    id: String,
    dialog: E,
    previous_focus: Option<E>,
    selector: Option<String>,
    pending: bool,
    suspended: bool,
}
pub(super) struct ModalLifecycle<P: ModalPlatform> {
    platform: P,
    entries: Vec<ModalEntry<P::Element>>,
    scroll: Option<P::ScrollSnapshot>,
}
impl<P: ModalPlatform> ModalLifecycle<P> {
    pub fn new(platform: P) -> Self {
        Self {
            platform,
            entries: vec![],
            scroll: None,
        }
    }
    pub fn sync(&mut self, id: &str, selector: &str, pending: bool) {
        let Some(dialog) = self.platform.dialog(id) else {
            return;
        };
        let index = if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
            index
        } else {
            let previous_focus = self.platform.active();
            let target = self.platform.target(&dialog, selector);
            let suspended = self.platform.suspended(&dialog);
            if !suspended && !self.platform.open(&dialog, target.as_ref()) {
                return;
            }
            if !suspended && self.scroll.is_none() {
                self.scroll = Some(self.platform.lock_scroll());
            }
            self.entries.push(ModalEntry {
                id: id.into(),
                dialog,
                previous_focus,
                selector: None,
                pending: false,
                suspended,
            });
            self.entries.len() - 1
        };
        let entry = &mut self.entries[index];
        if entry.suspended {
            entry.selector = Some(selector.into());
            entry.pending = pending;
            return;
        }
        if entry.selector.as_deref() != Some(selector) || entry.pending != pending {
            if pending {
                self.platform.focus_dialog(&entry.dialog, true);
            } else if let Some(target) = self.platform.target(&entry.dialog, selector) {
                self.platform.focus(&target, false);
            }
            entry.selector = Some(selector.into());
            entry.pending = pending;
        }
    }
    /// Release native background inertness without disposing editor state or
    /// its original focus target. Hidden pending updates may continue safely.
    pub fn suspend(&mut self, id: &str) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id && !entry.suspended)
        {
            entry.suspended = true;
            self.platform.close(&entry.dialog);
        }
        if self.entries.iter().all(|entry| entry.suspended) {
            if let Some(snapshot) = self.scroll.take() {
                self.platform.restore_scroll(snapshot);
            }
        }
    }
    pub fn resume(&mut self, id: &str) {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id && entry.suspended)
        else {
            return;
        };
        if !self.platform.connected(&entry.dialog) || self.platform.suspended(&entry.dialog) {
            return;
        }
        let target = entry
            .selector
            .as_deref()
            .and_then(|selector| self.platform.target(&entry.dialog, selector));
        if !self.platform.open(&entry.dialog, target.as_ref()) {
            return;
        }
        entry.suspended = false;
        if self.scroll.is_none() {
            self.scroll = Some(self.platform.lock_scroll());
        }
        if entry.pending {
            self.platform.focus_dialog(&entry.dialog, true);
        } else if let Some(target) = target {
            self.platform.focus(&target, false);
        }
    }
    pub fn dispose(&mut self, id: &str) {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return;
        };
        let entry = self.entries.remove(index);
        self.platform.close(&entry.dialog);
        if self.entries.iter().all(|entry| entry.suspended) {
            if let Some(snapshot) = self.scroll.take() {
                self.platform.restore_scroll(snapshot);
            }
        }
        if let Some(previous) = entry.previous_focus.filter(|_| !entry.suspended) {
            if self.platform.connected(&previous)
                && self
                    .entries
                    .iter()
                    .rev()
                    .find(|entry| !entry.suspended)
                    .is_none_or(|top| self.platform.contains(&top.dialog, &previous))
            {
                self.platform.focus(&previous, true);
            }
        }
    }
    pub fn wrap_tab(&self, id: &str, backwards: bool) -> bool {
        let Some(entry) = self
            .entries
            .iter()
            .rev()
            .find(|entry| !entry.suspended)
            .filter(|entry| entry.id == id)
        else {
            return false;
        };
        if !self.platform.is_open(&entry.dialog) || !self.platform.connected(&entry.dialog) {
            return false;
        }
        let controls = self.platform.controls(&entry.dialog);
        let active = self
            .platform
            .active()
            .and_then(|active| controls.iter().position(|control| *control == active));
        match tab_target(controls.len(), active, backwards) {
            TabTarget::Native => false,
            TabTarget::Dialog => {
                self.platform.focus_dialog(&entry.dialog, false);
                true
            }
            TabTarget::Control(index) => {
                self.platform.focus(&controls[index], false);
                true
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) mod browser {
    use super::*;
    use crate::components::base::{
        overlay_dom as dom,
        overlay_logic::{tab_candidate, tab_priority},
    };
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlDialogElement, HtmlElement};
    struct Browser;
    impl ModalPlatform for Browser {
        type Element = HtmlElement;
        type ScrollSnapshot = Vec<(HtmlElement, String, String)>;
        fn dialog(&self, id: &str) -> Option<HtmlElement> {
            dom::element(id).filter(|element| {
                element.is_connected() && element.is_instance_of::<HtmlDialogElement>()
            })
        }
        fn active(&self) -> Option<HtmlElement> {
            dom::document().active_element()?.dyn_into().ok()
        }
        fn target(&self, dialog: &HtmlElement, selector: &str) -> Option<HtmlElement> {
            dom::query(dialog, selector)
        }
        fn open(&self, dialog: &HtmlElement, target: Option<&HtmlElement>) -> bool {
            if let Some(target) = target {
                let _ = target.set_attribute("autofocus", "");
            }
            let opened = dialog
                .unchecked_ref::<HtmlDialogElement>()
                .show_modal()
                .is_ok();
            if let Some(target) = target {
                let _ = target.remove_attribute("autofocus");
            }
            opened
        }
        fn close(&self, dialog: &HtmlElement) {
            dialog.unchecked_ref::<HtmlDialogElement>().close();
        }
        fn lock_scroll(&self) -> Self::ScrollSnapshot {
            let document = dom::document();
            [
                document
                    .document_element()
                    .and_then(|element| element.dyn_into::<HtmlElement>().ok()),
                document.body(),
            ]
            .into_iter()
            .flatten()
            .map(|element| {
                let style = element.style();
                let value = style.get_property_value("overflow").unwrap_or_default();
                let priority = style.get_property_priority("overflow");
                let _ = style.set_property_with_priority("overflow", "hidden", "important");
                (element, value, priority)
            })
            .collect()
        }
        fn restore_scroll(&self, snapshot: Self::ScrollSnapshot) {
            for (element, value, priority) in snapshot {
                if value.is_empty() {
                    let _ = element.style().remove_property("overflow");
                } else {
                    let _ = element
                        .style()
                        .set_property_with_priority("overflow", &value, &priority);
                }
            }
        }
        fn connected(&self, element: &HtmlElement) -> bool {
            element.is_connected()
        }
        fn contains(&self, parent: &HtmlElement, child: &HtmlElement) -> bool {
            parent.contains(Some(child))
        }
        fn focus(&self, element: &HtmlElement, prevent_scroll: bool) {
            dom::focus(element, prevent_scroll);
        }
        fn focus_dialog(&self, element: &HtmlElement, prevent_scroll: bool) {
            element.set_tab_index(-1);
            dom::focus(element, prevent_scroll);
        }
        fn is_open(&self, dialog: &HtmlElement) -> bool {
            dialog.unchecked_ref::<HtmlDialogElement>().open()
        }
        fn suspended(&self, dialog: &HtmlElement) -> bool {
            dialog
                .closest("#session-retained-page[hidden]")
                .ok()
                .flatten()
                .is_some()
        }
        fn controls(&self, dialog: &HtmlElement) -> Vec<HtmlElement> {
            let Ok(nodes)=dialog.query_selector_all("button, input, select, textarea, a[href], area[href], summary, [tabindex], [contenteditable=\"true\"]") else {return vec![];};
            let mut controls: Vec<_> = (0..nodes.length())
                .filter_map(|index| nodes.item(index)?.dyn_into::<HtmlElement>().ok())
                .filter(|control| {
                    let visibility = web_sys::window()
                        .and_then(|window| window.get_computed_style(control).ok().flatten())
                        .and_then(|style| style.get_property_value("visibility").ok())
                        .unwrap_or_default();
                    tab_candidate(
                        control.has_attribute("disabled"),
                        control.hidden(),
                        control.tab_index(),
                        control.get_client_rects().length() > 0,
                        control.closest("[inert]").ok().flatten().is_some(),
                        control.matches(":disabled").unwrap_or(false),
                        &visibility,
                    )
                })
                .collect();
            controls.sort_by_key(|control| tab_priority(control.tab_index()));
            controls
        }
    }
    thread_local! {static MODALS:RefCell<ModalLifecycle<Browser>>=RefCell::new(ModalLifecycle::new(Browser));}
    pub(crate) fn sync_modal(id: &str, selector: &str, pending: bool) {
        MODALS.with_borrow_mut(|modals| modals.sync(id, selector, pending));
    }
    pub(crate) fn dispose_modal(id: &str) {
        MODALS.with_borrow_mut(|modals| modals.dispose(id));
    }
    pub(crate) fn wrap_modal_tab(id: &str, backwards: bool) -> bool {
        MODALS.with_borrow(|modals| modals.wrap_tab(id, backwards))
    }
    pub(crate) fn set_form_modals_suspended(root_id: &str, suspended: bool) {
        let Some(root) = dom::element(root_id) else {
            return;
        };
        MODALS.with_borrow_mut(|modals| {
            let ids: Vec<_> = modals
                .entries
                .iter()
                .filter(|entry| root.contains(Some(&entry.dialog)))
                .map(|entry| entry.id.clone())
                .collect();
            for id in ids {
                if suspended {
                    modals.suspend(&id);
                } else {
                    modals.resume(&id);
                }
            }
        });
    }
}
