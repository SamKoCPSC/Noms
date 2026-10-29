use super::placement::{place, reveal_delta, Rect};
use crate::components::base::overlay_dom as dom;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{EventTarget, HtmlElement};

#[derive(Clone, Copy)]
struct Limits {
    min_width: f64,
    max_height: f64,
}

pub(super) struct Popup {
    // Dropping listeners removes every registration on close or navigation.
    _listeners: Vec<dom::Listener>,
    panel: HtmlElement,
    active: Rc<Cell<Option<usize>>>,
    update: Rc<dom::AnimationFrame>,
    query: String,
}

impl Popup {
    pub fn mount(id: &str, query: &str, active: Option<usize>) -> Option<Self> {
        let panel = dom::element(id)?;
        let anchor = panel
            .parent_element()?
            .query_selector("input[role=combobox]")
            .ok()??
            .dyn_into::<HtmlElement>()
            .ok()?;
        let window = web_sys::window()?;
        let style = window.get_computed_style(&panel).ok()??;
        let pixels = |name| {
            style
                .get_property_value(name)
                .ok()
                .and_then(|value| value.trim_end_matches("px").parse::<f64>().ok())
        };
        let limits = Limits {
            min_width: pixels("min-width").unwrap_or(0.),
            max_height: pixels("max-height").unwrap_or(192.),
        };
        // Native top-layer positioning avoids transformed/scroll-clipped parents,
        // including pantry/grocery dialogs. It does not move focus or change DOM ownership.
        dom::css(&panel, "position", "fixed");
        dom::css(&panel, "inset", "auto");
        dom::css(&panel, "margin", "0");
        dom::css(&panel, "min-width", "0");
        dom::css(&panel, "visibility", "hidden");
        dom::show_popover(&panel);
        let active = Rc::new(Cell::new(active));
        let reveal_input = Rc::new(Cell::new(true));
        let update = Rc::new(dom::AnimationFrame::new({
            let (panel, anchor, active, reveal) = (
                panel.clone(),
                anchor.clone(),
                active.clone(),
                reveal_input.clone(),
            );
            move || position(&panel, &anchor, limits, active.get(), reveal.replace(false))
        }));
        update.request();
        let mut listeners = Vec::new();
        let mut targets: Vec<(EventTarget, &str, bool)> = vec![
            (window.clone().into(), "resize", true),
            (dom::document().into(), "scroll", false),
        ];
        if let Some(viewport) = window.visual_viewport() {
            targets.push((viewport.clone().into(), "resize", true));
            targets.push((viewport.into(), "scroll", false));
        }
        for (target, name, reveal) in targets {
            let (panel, update, reveal_input) =
                (panel.clone(), update.clone(), reveal_input.clone());
            listeners.push(dom::Listener::new(&target, name, true, move |event| {
                // User scrolling inside the list must not snap back to the
                // previously highlighted option or trigger another layout pass.
                if name == "scroll"
                    && event
                        .target()
                        .is_some_and(|target| target == panel.clone().into())
                {
                    return;
                }
                reveal_input.set(reveal_input.get() || reveal);
                update.request();
            }));
        }
        Some(Self {
            _listeners: listeners,
            panel,
            active,
            update,
            query: query.into(),
        })
    }

    pub fn update(&mut self, query: &str, active: Option<usize>) {
        if query != self.query {
            self.panel.set_scroll_top(0);
            self.query = query.into();
        }
        self.active.set(active);
        self.update.request();
    }
}

impl Drop for Popup {
    fn drop(&mut self) {
        self.update.cancel();
        dom::hide_popover(&self.panel);
    }
}

fn rect(element: &HtmlElement) -> Rect {
    let r = element.get_bounding_client_rect();
    Rect {
        left: r.left(),
        top: r.top(),
        right: r.right(),
        bottom: r.bottom(),
    }
}

fn viewport() -> Rect {
    let window = web_sys::window().expect("browser window");
    if let Some(viewport) = window.visual_viewport() {
        Rect {
            left: viewport.offset_left(),
            top: viewport.offset_top(),
            right: viewport.offset_left() + viewport.width(),
            bottom: viewport.offset_top() + viewport.height(),
        }
    } else {
        let (width, height) = dom::viewport();
        Rect {
            left: 0.,
            top: 0.,
            right: width,
            bottom: height,
        }
    }
}

fn reveal(anchor: &HtmlElement, view: Rect) {
    if dom::document().active_element().as_ref() != Some(anchor.as_ref()) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let mut parent = anchor.parent_element();
    while let Some(element) = parent {
        let delta = reveal_delta(rect(anchor), view);
        if delta.abs() < 1. {
            return;
        }
        let scrollable = window
            .get_computed_style(&element)
            .ok()
            .flatten()
            .and_then(|s| s.get_property_value("overflow-y").ok())
            .is_some_and(|value| matches!(value.as_str(), "auto" | "scroll"));
        if scrollable && element.scroll_height() > element.client_height() {
            element.set_scroll_top(element.scroll_top() + delta.round() as i32);
        }
        parent = element.parent_element();
    }
    let delta = reveal_delta(rect(anchor), view);
    if delta.abs() >= 1. {
        window.scroll_by_with_x_and_y(0., delta);
    }
}

fn position(
    panel: &HtmlElement,
    anchor: &HtmlElement,
    limits: Limits,
    active: Option<usize>,
    reveal_input: bool,
) {
    if !panel.is_connected() {
        return;
    }
    let view = viewport();
    if reveal_input {
        reveal(anchor, view);
    }
    let anchor_rect = rect(anchor);
    let width = (anchor_rect.right - anchor_rect.left).max(limits.min_width);
    let width = width.min((view.right - view.left - 16.).max(0.));
    dom::css(panel, "width", &format!("{width}px"));
    let height = f64::from(panel.scroll_height() + panel.offset_height() - panel.client_height())
        .min(limits.max_height);
    let align_end = panel.parent_element().is_some_and(|parent| {
        parent
            .class_name()
            .split_whitespace()
            .any(|class| class == "unit-autocomplete--end")
    });
    let at = place(anchor_rect, view, width, height, align_end);
    dom::css(panel, "left", &format!("{}px", at.left));
    dom::css(panel, "top", &format!("{}px", at.top));
    dom::css(panel, "width", &format!("{}px", at.width));
    dom::css(
        panel,
        "max-height",
        &format!("{}px", limits.max_height.min(at.max_height)),
    );
    let _ = panel.set_attribute("data-placement", if at.above { "above" } else { "below" });
    dom::css(
        panel,
        "visibility",
        if anchor_rect.bottom > view.top && anchor_rect.top < view.bottom {
            "visible"
        } else {
            "hidden"
        },
    );
    // Scroll only the list, never the page or modal containing the input.
    if let Some(option) = active.and_then(|index| dom::element(&format!("{}-{index}", panel.id())))
    {
        let list = rect(panel);
        let selected = rect(&option);
        let delta = if selected.top < list.top + 2. {
            selected.top - list.top - 2.
        } else if selected.bottom > list.bottom - 2. {
            selected.bottom - list.bottom + 2.
        } else {
            0.
        };
        if delta.abs() >= 1. {
            panel.set_scroll_top(panel.scroll_top() + delta.round() as i32);
        }
    }
}
