//! Hover/focus help implemented entirely in Rust; timers/listeners own cleanup.
use super::{
    overlay_dom as dom,
    overlay_logic::{tooltip_position, TooltipEvent, TooltipState},
};
use gloo_timers::callback::Timeout;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{Event, EventTarget, HtmlElement, KeyboardEvent};

struct Tooltip {
    root: HtmlElement,
    button: HtmlElement,
    tip: HtmlElement,
    state: RefCell<TooltipState>,
    timer: RefCell<Option<Timeout>>,
}
impl Tooltip {
    fn update(self: &Rc<Self>, event: TooltipEvent) {
        if matches!(
            event,
            TooltipEvent::Enter | TooltipEvent::Focus | TooltipEvent::Leave | TooltipEvent::Blur
        ) {
            self.timer.borrow_mut().take();
        }
        self.state
            .borrow_mut()
            .update(event, self.button.has_attribute("disabled"));
        if self.state.borrow().visible {
            dom::show_popover(&self.tip);
            self.position();
        } else {
            dom::hide_popover(&self.tip);
        }
        if matches!(event, TooltipEvent::Leave | TooltipEvent::Blur) {
            let weak = Rc::downgrade(self);
            *self.timer.borrow_mut() = Some(Timeout::new(150, move || {
                if let Some(tooltip) = weak.upgrade() {
                    tooltip.update(TooltipEvent::GraceElapsed);
                }
            }));
        }
    }
    fn position(&self) {
        if self.tip.hidden() {
            return;
        }
        let (width, height) = dom::viewport();
        let (left, top) = tooltip_position(
            dom::rect(&self.button),
            f64::from(self.tip.offset_width()),
            f64::from(self.tip.offset_height()),
            width,
            height,
        );
        dom::css(&self.tip, "left", &format!("{left}px"));
        dom::css(&self.tip, "top", &format!("{top}px"));
    }
}
struct Registration {
    tooltip: Rc<Tooltip>,
    _listeners: Vec<dom::Listener>,
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.tooltip.timer.borrow_mut().take();
        dom::hide_popover(&self.tooltip.tip);
    }
}
thread_local! {static TOOLTIPS:RefCell<HashMap<String,Registration>>=RefCell::new(HashMap::new());}

fn listen(
    tooltip: &Rc<Tooltip>,
    target: &EventTarget,
    name: &'static str,
    capture: bool,
    callback: impl Fn(&Rc<Tooltip>, Event) + 'static,
) -> dom::Listener {
    let weak = Rc::downgrade(tooltip);
    dom::Listener::new(target, name, capture, move |event| {
        if let Some(tooltip) = weak.upgrade() {
            callback(&tooltip, event);
        }
    })
}
pub(super) fn mount_tooltip(id: &str) {
    let Some(root) = dom::element(id) else {
        return;
    };
    TOOLTIPS.with_borrow_mut(|registry| {
        if registry
            .get(id)
            .is_some_and(|registration| registration.tooltip.root == root)
        {
            return;
        }
        registry.remove(id);
        let Some(button) = dom::query(&root, "button") else {
            return;
        };
        let Some(tip) = dom::query(&root, "[role=\"tooltip\"]") else {
            return;
        };
        let tooltip = Rc::new(Tooltip {
            root,
            button,
            tip,
            state: RefCell::new(TooltipState::default()),
            timer: RefCell::new(None),
        });
        let mut listeners = vec![];
        for (target, name, event) in [
            (&tooltip.root, "mouseenter", TooltipEvent::Enter),
            (&tooltip.root, "mouseleave", TooltipEvent::Leave),
            (&tooltip.tip, "mouseenter", TooltipEvent::Enter),
            (&tooltip.tip, "mouseleave", TooltipEvent::Leave),
            (&tooltip.button, "focus", TooltipEvent::Focus),
            (&tooltip.button, "blur", TooltipEvent::Blur),
        ] {
            listeners.push(listen(&tooltip, target, name, false, move |tooltip, _| {
                tooltip.update(event)
            }));
        }
        listeners.push(listen(
            &tooltip,
            &dom::document(),
            "keydown",
            false,
            |tooltip, event| {
                if event
                    .dyn_ref::<KeyboardEvent>()
                    .is_some_and(|event| event.key() == "Escape")
                    && !tooltip.tip.hidden()
                {
                    event.prevent_default();
                    tooltip.update(TooltipEvent::Escape);
                }
            },
        ));
        listeners.push(listen(
            &tooltip,
            &web_sys::window().expect("browser window"),
            "resize",
            false,
            |tooltip, _| tooltip.position(),
        ));
        listeners.push(listen(
            &tooltip,
            &dom::document(),
            "scroll",
            true,
            |tooltip, _| tooltip.position(),
        ));
        registry.insert(
            id.into(),
            Registration {
                tooltip,
                _listeners: listeners,
            },
        );
    });
}
pub(super) fn dispose_tooltip(id: &str) {
    TOOLTIPS.with_borrow_mut(|registry| {
        registry.remove(id);
    });
}
