use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Event, EventTarget, HtmlElement, Window};

/// Owns both listeners and queued animation frames; neither outlives the scope.
pub struct MobileViewport {
    listeners: Vec<DomListener>,
    update: Rc<AnimationFrame>,
    select_section: Rc<AnimationFrame>,
    root: HtmlElement,
}

pub fn mount() -> Option<MobileViewport> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let root = document
        .document_element()?
        .dyn_into::<HtmlElement>()
        .ok()?;
    let viewport = window.visual_viewport();
    // Retain the unfocused layout height while editing. Some Android browsers
    // resize both viewports, so innerHeight alone loses the keyboard's height.
    let reference_height = Cell::new(0.0_f64);
    let reference_width = Cell::new(width(&window));
    let update = Rc::new(AnimationFrame::new(window.clone(), {
        let (window, document, root, viewport) = (
            window.clone(),
            document.clone(),
            root.clone(),
            viewport.clone(),
        );
        move || {
            let editing = document.active_element().is_some_and(|element| {
                element.matches("input:not([type=checkbox]):not([type=radio]):not([type=range]), textarea, select")
                    .unwrap_or(false)
            });
            let height = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let current_width = width(&window);
            if current_width != reference_width.get() || !editing {
                reference_width.set(current_width);
                reference_height.set(height);
            } else {
                reference_height.set(reference_height.get().max(height));
            }
            let visual = viewport.as_ref().map(|v| (v.height(), v.offset_top()));
            let keyboard =
                super::keyboard_offset(current_width, editing, reference_height.get(), visual);
            let offset = super::keyboard_offset(current_width, editing, height, visual);
            let _ = root.set_attribute(
                "data-mobile-keyboard",
                if keyboard > 0.0 { "open" } else { "closed" },
            );
            let _ = root
                .style()
                .set_property("--mobile-keyboard-offset", &format!("{offset}px"));
            let (visible_height, visible_top) = visual.unwrap_or((height, 0.));
            let _ = root
                .style()
                .set_property("--mobile-visual-height", &format!("{visible_height}px"));
            let _ = root
                .style()
                .set_property("--mobile-visual-top", &format!("{visible_top}px"));
        }
    }));
    let heading_id = Rc::new(RefCell::new(None::<String>));
    let select_section = Rc::new(AnimationFrame::new(window.clone(), {
        let (document, heading_id) = (document.clone(), heading_id.clone());
        move || {
            let Some(id) = heading_id.borrow_mut().take() else {
                return;
            };
            let Some(heading) = document
                .get_element_by_id(&id)
                .and_then(|e| e.dyn_into::<HtmlElement>().ok())
            else {
                return;
            };
            let focus = web_sys::FocusOptions::new();
            focus.set_prevent_scroll(true);
            let _ = heading.focus_with_options(&focus);
            let scroll = web_sys::ScrollIntoViewOptions::new();
            scroll.set_block(web_sys::ScrollLogicalPosition::Start);
            scroll.set_behavior(web_sys::ScrollBehavior::Instant);
            heading.scroll_into_view_with_scroll_into_view_options(&scroll);
        }
    }));
    let mut runtime = MobileViewport {
        listeners: Vec::new(),
        update,
        select_section,
        root,
    };
    let mut targets: Vec<(EventTarget, &str)> = vec![
        (window.clone().into(), "resize"),
        (document.clone().into(), "focusin"),
        (document.clone().into(), "focusout"),
    ];
    if let Some(viewport) = viewport {
        targets.push((viewport.clone().into(), "resize"));
        targets.push((viewport.into(), "scroll"));
    }
    for (target, name) in targets {
        let update = runtime.update.clone();
        runtime
            .listeners
            .push(DomListener::new(target, name, move |_| update.request()).ok()?);
    }
    let select_section = runtime.select_section.clone();
    runtime.listeners.push(
        DomListener::new(document.into(), "click", move |event| {
            if width(&window) > 760.0 {
                return;
            }
            let Some(target) = event
                .target()
                .and_then(|e| e.dyn_into::<web_sys::Element>().ok())
            else {
                return;
            };
            let Some(anchor) = target
                .closest(".recipe-editor__contents a[href^=\"#recipe-\"]")
                .ok()
                .flatten()
            else {
                return;
            };
            let Some(href) = anchor.get_attribute("href") else {
                return;
            };
            // We scroll locally after the section has rendered. Native fragment
            // navigation would notify the router of an autosaved draft URL and
            // remount the editor, losing the selected section and field focus.
            event.prevent_default();
            *heading_id.borrow_mut() = href.strip_prefix('#').map(str::to_owned);
            select_section.request();
        })
        .ok()?,
    );
    runtime.update.request();
    Some(runtime)
}

impl Drop for MobileViewport {
    fn drop(&mut self) {
        self.update.cancel();
        self.select_section.cancel();
        let _ = self.root.remove_attribute("data-mobile-keyboard");
        let _ = self
            .root
            .style()
            .remove_property("--mobile-keyboard-offset");
        let _ = self.root.style().remove_property("--mobile-visual-height");
        let _ = self.root.style().remove_property("--mobile-visual-top");
    }
}

fn width(window: &Window) -> f64 {
    window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(f64::INFINITY)
}

/// Coalesce events until the next frame without leaking a Closure or Rc cycle.
struct AnimationFrame {
    window: Window,
    pending: Rc<Cell<Option<i32>>>,
    callback: Closure<dyn FnMut(f64)>,
}

impl AnimationFrame {
    fn new(window: Window, mut callback: impl FnMut() + 'static) -> Self {
        let pending = Rc::new(Cell::new(None));
        let frame_pending = pending.clone();
        let callback = Closure::wrap(Box::new(move |_: f64| {
            frame_pending.set(None);
            callback();
        }) as Box<dyn FnMut(f64)>);
        Self {
            window,
            pending,
            callback,
        }
    }

    fn request(&self) {
        self.cancel();
        self.pending.set(
            self.window
                .request_animation_frame(self.callback.as_ref().unchecked_ref())
                .ok(),
        );
    }

    fn cancel(&self) {
        if let Some(frame) = self.pending.take() {
            let _ = self.window.cancel_animation_frame(frame);
        }
    }
}

impl Drop for AnimationFrame {
    fn drop(&mut self) {
        self.cancel();
    }
}

struct DomListener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}

impl DomListener {
    fn new(
        target: EventTarget,
        name: &'static str,
        callback: impl FnMut(Event) + 'static,
    ) -> Result<Self, JsValue> {
        let callback = Closure::wrap(Box::new(callback) as Box<dyn FnMut(Event)>);
        target.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
        Ok(Self {
            target,
            name,
            callback,
        })
    }
}

impl Drop for DomListener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.name, self.callback.as_ref().unchecked_ref());
    }
}
