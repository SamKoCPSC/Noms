//! Typed browser primitives. Listener ownership is RAII; no handwritten JS bridge.
use super::overlay_logic::Rect;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Element, Event, EventTarget, HtmlElement};

pub(super) struct Listener {
    target: EventTarget,
    name: &'static str,
    capture: bool,
    callback: Closure<dyn FnMut(Event)>,
}
impl Listener {
    pub fn new(
        target: &EventTarget,
        name: &'static str,
        capture: bool,
        callback: impl FnMut(Event) + 'static,
    ) -> Self {
        let callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event)>);
        target
            .add_event_listener_with_callback_and_bool(
                name,
                callback.as_ref().unchecked_ref(),
                capture,
            )
            .expect("overlay event listener");
        Self {
            target: target.clone(),
            name,
            capture,
            callback,
        }
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.name,
            self.callback.as_ref().unchecked_ref(),
            self.capture,
        );
    }
}
pub(super) fn document() -> web_sys::Document {
    web_sys::window()
        .expect("browser window")
        .document()
        .expect("browser document")
}
pub(super) fn element(id: &str) -> Option<HtmlElement> {
    document().get_element_by_id(id)?.dyn_into().ok()
}
pub(super) fn query(root: &Element, selector: &str) -> Option<HtmlElement> {
    root.query_selector(selector)
        .ok()
        .flatten()?
        .dyn_into()
        .ok()
}
pub(super) fn rect(element: &Element) -> Rect {
    let r = element.get_bounding_client_rect();
    Rect {
        left: r.left(),
        right: r.right(),
        top: r.top(),
        bottom: r.bottom(),
    }
}
pub(super) fn viewport() -> (f64, f64) {
    let window = web_sys::window().expect("browser window");
    (
        window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.),
        window
            .inner_height()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.),
    )
}
pub(super) fn focus(element: &HtmlElement, prevent_scroll: bool) {
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(prevent_scroll);
    let _ = element.focus_with_options(&options);
}
pub(super) fn css(element: &HtmlElement, name: &str, value: &str) {
    let _ = element.style().set_property(name, value);
}
pub(super) fn show_popover(element: &HtmlElement) {
    element.set_hidden(false);
    let _ = element.set_attribute("popover", "manual");
    if !element.matches(":popover-open").unwrap_or(false) {
        let _ = element.show_popover();
    }
}
pub(super) fn hide_popover(element: &HtmlElement) {
    if element.matches(":popover-open").unwrap_or(false) {
        let _ = element.hide_popover();
    }
    element.set_hidden(true);
}
