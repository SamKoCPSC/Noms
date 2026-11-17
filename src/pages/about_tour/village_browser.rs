//! Rust owns the village's navigation, loading gate, accessibility and lifecycle.
//! The JS boundary only loads assets and renders the approved walking camera.
use crate::pages::about::{IDS, NAMES};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{
    Document, Element, Event, EventTarget, HtmlCanvasElement, HtmlElement, KeyboardEvent,
    MediaQueryList, MouseEvent, ResizeObserver, Window,
};

#[wasm_bindgen(module = "/assets/about-village-bootstrap.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = loadVillage)]
    async fn load_village(
        canvas: &HtmlCanvasElement,
        catalog: &str,
        bridge: &str,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = readyVillage)]
    async fn ready_village(view: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = drawVillage)]
    fn draw_village(
        view: &JsValue,
        progress: f32,
        width: f32,
        height: f32,
        slot: &[f32],
        opening_distance: f32,
    ) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = disposeVillage)]
    fn dispose_village(view: &JsValue) -> Result<(), JsValue>;
}
const MODELS: [&str; 12] = [
    "kitchen",
    "library",
    "bedroom",
    "supermarket",
    "office",
    "corporate-office",
    "brick-kitchen",
    "bakery",
    "farmers-market",
    "social-kitchen",
    "riverside-campsite",
    "garden",
];
const TRANSITION_MS: f64 = 3200.;
const OPENING_MS: f64 = 2400.;

#[derive(Clone, Copy)]
enum OpeningShot {
    Wide,
    Approach(Option<f64>),
    Finished,
}

impl OpeningShot {
    fn distance(&mut self, now: f64) -> f32 {
        match self {
            // Reveal one wide frame before starting the clock. Compiling the
            // first WebGL frame can otherwise consume the entire approach.
            Self::Wide => {
                *self = Self::Approach(None);
                1.
            }
            Self::Approach(start) => {
                let t = ((now - *start.get_or_insert(now)) / OPENING_MS).clamp(0., 1.);
                if t >= 1. {
                    *self = Self::Finished;
                }
                (1. - t * t * (3. - 2. * t)) as f32
            }
            Self::Finished => 0.,
        }
    }
}

type StateRef = Rc<RefCell<State>>;
thread_local! { static RUNTIME: RefCell<Option<StateRef>> = const { RefCell::new(None) }; }
struct Listener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.name, self.callback.as_ref().unchecked_ref());
    }
}
struct State {
    window: Window,
    doc: Document,
    root: HtmlElement,
    canvas: HtmlCanvasElement,
    media: MediaQueryList,
    panels: Vec<HtmlElement>,
    anchors: Vec<Element>,
    slot: Element,
    view: Option<JsValue>,
    requested: bool,
    ready: bool,
    failed: bool,
    generation: u32,
    manual_read: bool,
    flying: bool,
    progress: f64,
    target: f64,
    active: usize,
    // Timestamp is initialized by the first RAF, sharing its time origin.
    transition: Option<(f64, f64, Option<f64>)>,
    opening: OpeningShot,
    expected_scroll: Option<f64>,
    focus: bool,
    raf: Option<i32>,
    callback: Option<Closure<dyn FnMut(f64)>>,
    listeners: Vec<Listener>,
    observer: Option<ResizeObserver>,
    observer_callback: Option<Closure<dyn FnMut()>>,
}
impl Drop for State {
    fn drop(&mut self) {
        if let Some(id) = self.raf {
            let _ = self.window.cancel_animation_frame(id);
        }
        if let Some(observer) = &self.observer {
            observer.disconnect();
        }
        if let Some(view) = self.view.take() {
            let _ = dispose_village(&view);
        }
    }
}
fn el(doc: &Document, id: &str) -> Result<Element, JsValue> {
    doc.get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(id))
}
fn fragment(window: &Window) -> Option<usize> {
    let hash = window.location().hash().ok()?;
    IDS.iter()
        .position(|id| *id == hash.trim_start_matches('#'))
}
fn listen(
    state: &StateRef,
    target: &EventTarget,
    name: &'static str,
    handler: impl Fn(&StateRef, Event) + 'static,
) -> Result<(), JsValue> {
    let weak = Rc::downgrade(state);
    let callback = Closure::wrap(Box::new(move |event| {
        if let Some(state) = weak.upgrade() {
            handler(&state, event);
            schedule(&state);
        }
    }) as Box<dyn FnMut(Event)>);
    target.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
    state.borrow_mut().listeners.push(Listener {
        target: target.clone(),
        name,
        callback,
    });
    Ok(())
}
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let doc = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;
    let root: HtmlElement = el(&doc, "about-tour")?.dyn_into()?;
    let canvas: HtmlCanvasElement = el(&doc, "story-three-canvas")?.dyn_into()?;
    let media = window
        .match_media("(prefers-reduced-motion: reduce)")?
        .ok_or_else(|| JsValue::from_str("No media query"))?;
    let initial = fragment(&window).unwrap_or(0);
    let state = Rc::new(RefCell::new(State {
        window: window.clone(),
        doc: doc.clone(),
        root: root.clone(),
        canvas: canvas.clone(),
        media: media.clone(),
        panels: (0..IDS.len())
            .map(|i| el(&doc, &format!("panel-{i}"))?.dyn_into())
            .collect::<Result<_, _>>()?,
        anchors: (0..IDS.len())
            .map(|i| el(&doc, &format!("stop-{i}")))
            .collect::<Result<_, _>>()?,
        slot: el(&doc, "tour-scene-framing")?,
        view: None,
        requested: false,
        ready: false,
        failed: false,
        generation: 0,
        manual_read: false,
        flying: false,
        progress: initial as f64,
        target: initial as f64,
        active: initial,
        transition: None,
        opening: if initial == 0 {
            OpeningShot::Wide
        } else {
            OpeningShot::Finished
        },
        expected_scroll: None,
        focus: false,
        raf: None,
        callback: None,
        listeners: vec![],
        observer: None,
        observer_callback: None,
    }));
    let weak = Rc::downgrade(&state);
    state.borrow_mut().callback = Some(Closure::wrap(Box::new(move |time| {
        if let Some(state) = weak.upgrade() {
            let moving = state.borrow_mut().tick(time);
            if moving {
                schedule(&state);
            }
        }
    }) as Box<dyn FnMut(f64)>));
    listen(&state, window.as_ref(), "scroll", |state, _| {
        let mut s = state.borrow_mut();
        if !s.flying || !s.ready {
            return;
        }
        let y = s.window.scroll_y().unwrap_or(0.);
        if s.expected_scroll
            .is_some_and(|expected| (expected - y).abs() < 2.)
        {
            return;
        }
        s.opening = OpeningShot::Finished;
        s.expected_scroll = None;
        s.transition = None;
        s.focus = false;
        s.target = s.scroll_progress();
    })?;
    for name in ["hashchange", "popstate"] {
        listen(&state, window.as_ref(), name, |state, _| {
            let mut s = state.borrow_mut();
            if let Some(i) = fragment(&s.window) {
                s.navigate(i, false);
            }
        })?;
    }
    listen(&state, root.as_ref(), "click", |state, event| {
        let Some(mouse) = event.dyn_ref::<MouseEvent>() else {
            return;
        };
        if mouse.button() != 0
            || mouse.ctrl_key()
            || mouse.meta_key()
            || mouse.shift_key()
            || mouse.alt_key()
        {
            return;
        }
        let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let mut s = state.borrow_mut();
        if target
            .closest("#tour-loader-read, #village-view-toggle")
            .ok()
            .flatten()
            .is_some()
        {
            event.prevent_default();
            event.stop_propagation();
            s.manual_read = s.flying;
            s.apply_mode();
            s.focus_heading();
            drop(s);
            maybe_load(state);
            return;
        }
        if !s.flying || !s.ready {
            return;
        }
        if let Ok(Some(link)) = target.closest("a[href^='#']") {
            let hash = link.get_attribute("href").unwrap_or_default();
            if let Some(i) = IDS
                .iter()
                .position(|id| *id == hash.trim_start_matches('#'))
            {
                event.prevent_default();
                event.stop_propagation();
                s.navigate(i, true);
            }
        }
    })?;
    listen(&state, window.as_ref(), "keydown", |state, event| {
        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        if key.alt_key() || key.ctrl_key() || key.meta_key() || key.shift_key() {
            return;
        }
        if event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|e| {
                e.closest("input, textarea, select, [contenteditable='true'], [role='dialog'], nav")
                    .ok()
                    .flatten()
            })
            .is_some()
        {
            return;
        }
        let mut s = state.borrow_mut();
        if !s.flying || !s.ready {
            return;
        }
        let next = match key.key().as_str() {
            "ArrowRight" | "PageDown" => Some((s.active + 1).min(IDS.len() - 1)),
            "ArrowLeft" | "PageUp" => Some(s.active.saturating_sub(1)),
            _ => None,
        };
        if let Some(i) = next {
            event.prevent_default();
            s.navigate(i, true);
        }
    })?;
    listen(
        &state,
        canvas.as_ref(),
        "webglcontextlost",
        |state, event| {
            event.prevent_default();
            let mut s = state.borrow_mut();
            s.failed = true;
            s.apply_mode();
            s.focus_heading();
        },
    )?;
    listen(&state, canvas.as_ref(), "villageassetready", |state, _| {
        let s = state.borrow();
        let loaded = s
            .canvas
            .get_attribute("data-loaded-models")
            .unwrap_or_default();
        if let Ok(status) = el(&s.doc, "tour-loading-status") {
            status.set_text_content(Some(&format!(
                "Preparing the village · {loaded} of 13 places"
            )));
        }
    })?;
    for (target, name) in [(window.as_ref(), "resize"), (media.as_ref(), "change")] {
        listen(&state, target, name, |state, _| {
            state.borrow_mut().apply_mode();
            maybe_load(state);
        })?;
    }
    let weak = Rc::downgrade(&state);
    let callback = Closure::wrap(Box::new(move || {
        if let Some(state) = weak.upgrade() {
            schedule(&state);
        }
    }) as Box<dyn FnMut()>);
    let observer = ResizeObserver::new(callback.as_ref().unchecked_ref())?;
    observer.observe(canvas.as_ref());
    observer.observe(&state.borrow().slot);
    observer.observe(&el(&doc, "story")?);
    for panel in &state.borrow().panels {
        observer.observe(panel.as_ref());
    }
    state.borrow_mut().observer = Some(observer);
    state.borrow_mut().observer_callback = Some(callback);
    state.borrow_mut().apply_mode();
    RUNTIME.with(|slot| *slot.borrow_mut() = Some(state.clone()));
    maybe_load(&state);
    schedule(&state);
    Ok(())
}
pub fn stop() {
    RUNTIME.with(|slot| {
        if let Some(state) = slot.borrow_mut().take() {
            let mut s = state.borrow_mut();
            // Route cleanup must not scroll the destination page. Dropping the
            // runtime cancels timers/listeners and releases the Three scene.
            s.generation += 1;
            s.flying = false;
            s.root.set_class_name("about-tour is-reader");
            s.gate(false);
        }
    });
}
fn schedule(state: &StateRef) {
    let mut s = state.borrow_mut();
    if s.raf.is_some() || !s.flying {
        return;
    }
    if let Some(callback) = &s.callback {
        s.raf = s
            .window
            .request_animation_frame(callback.as_ref().unchecked_ref())
            .ok();
    }
}
fn maybe_load(state: &StateRef) {
    let request = {
        let mut s = state.borrow_mut();
        if !s.flying || s.requested {
            return;
        }
        s.requested = true;
        let url = |key: &str| s.root.get_attribute(key).unwrap_or_default();
        let models: Vec<_> = MODELS
            .iter()
            .map(|name| url(&format!("data-village-{name}-url")))
            .collect();
        (s.canvas.clone(), serde_json::json!({"layout":url("data-village-layout-url"),"world":url("data-village-world-url"),"models":models}).to_string(), url("data-village-bridge-url"), s.generation)
    };
    let weak = Rc::downgrade(state);
    wasm_bindgen_futures::spawn_local(async move {
        let result = load_village(&request.0, &request.1, &request.2).await;
        let view = match result {
            Ok(view) => view,
            Err(error) => {
                if let Some(state) = weak.upgrade() {
                    let mut s = state.borrow_mut();
                    if s.generation == request.3 {
                        tracing::warn!(?error, "Village uses reading fallback");
                        s.failed = true;
                        s.apply_mode();
                        s.focus_heading();
                    }
                }
                return;
            }
        };
        if let Some(state) = weak.upgrade() {
            let mut s = state.borrow_mut();
            if s.generation != request.3 {
                let _ = dispose_village(&view);
                return;
            }
            s.view = Some(view.clone());
        } else {
            let _ = dispose_village(&view);
            return;
        }
        let result = ready_village(&view).await;
        if let Some(state) = weak.upgrade() {
            {
                let mut s = state.borrow_mut();
                if s.generation != request.3 {
                    return;
                }
                match result {
                    Ok(()) => s.ready = true,
                    Err(error) => {
                        tracing::warn!(?error, "Village uses reading fallback");
                        s.failed = true;
                        s.apply_mode();
                        s.focus_heading();
                    }
                }
            }
            schedule(&state);
        }
    });
}
impl State {
    fn focus_heading(&self) {
        if let Ok(heading) = el(&self.doc, &format!("title-{}", self.active))
            .and_then(|e| Ok(e.dyn_into::<HtmlElement>()?))
        {
            let _ = heading.set_attribute("tabindex", "-1");
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = heading.focus_with_options(&options);
        }
    }
    fn apply_mode(&mut self) {
        let was = self.flying;
        self.flying = !self.manual_read
            && !self.failed
            && !self.media.matches()
            && self
                .window
                .inner_height()
                .ok()
                .and_then(|n| n.as_f64())
                .unwrap_or(0.)
                >= 600.;
        self.root.set_class_name(if self.flying {
            "about-tour is-flight"
        } else {
            "about-tour is-reader"
        });
        let _ = self.root.set_attribute(
            "data-renderer",
            if self.flying {
                "three-village"
            } else {
                "fallback"
            },
        );
        let _ = self
            .canvas
            .style()
            .set_property("display", if self.flying { "block" } else { "none" });
        for (i, panel) in self.panels.iter().enumerate() {
            if let Ok(Some(anchor)) = panel.query_selector(".reader-anchor") {
                anchor.set_id(&if self.flying {
                    format!("reader-{}", IDS[i])
                } else {
                    IDS[i].into()
                });
            }
            self.anchors[i].set_id(&if self.flying {
                IDS[i].into()
            } else {
                format!("stop-{i}")
            });
            if !self.flying {
                let _ = panel.remove_attribute("style");
                let _ = panel.remove_attribute("inert");
                let _ = panel.remove_attribute("aria-hidden");
            }
        }
        if let Ok(button) = el(&self.doc, "village-view-toggle") {
            if self.failed || self.media.matches() {
                let _ = button.set_attribute("hidden", "");
            } else {
                let _ = button.remove_attribute("hidden");
            }
            button.set_text_content(Some(if self.flying {
                "Read as page"
            } else {
                "Back to tour"
            }));
        }
        if !self.flying {
            self.opening = OpeningShot::Finished;
            self.generation += 1;
            if !self.ready || self.failed {
                self.ready = false;
                if let Some(view) = self.view.take() {
                    let _ = dispose_village(&view);
                }
                self.requested = false;
            }
            self.transition = None;
            self.gate(false);
            if was {
                if let Some(id) = self.raf.take() {
                    let _ = self.window.cancel_animation_frame(id);
                }
                self.panels[self.active].scroll_into_view();
            }
        } else {
            self.gate(!self.ready);
            if !was {
                self.progress = self.active as f64;
                self.target = self.progress;
            }
            self.align_scroll();
        }
    }
    fn gate(&self, loading: bool) {
        let _ = self.root.set_attribute(
            "data-entry-state",
            if loading { "loading" } else { "ready" },
        );
        let _ = self
            .root
            .set_attribute("aria-busy", if loading { "true" } else { "false" });
        for id in ["story", "scroll-track"] {
            if let Ok(node) = el(&self.doc, id) {
                if loading {
                    let _ = node.set_attribute("inert", "");
                } else {
                    let _ = node.remove_attribute("inert");
                }
            }
        }
    }
    fn align_scroll(&mut self) {
        let index = self.target.floor() as usize;
        let anchor = self.anchors[index].get_bounding_client_rect();
        let y = self.window.scroll_y().unwrap_or(0.)
            + anchor.top()
            + anchor.height() * self.target.fract();
        self.expected_scroll = Some(y);
        let options = web_sys::ScrollToOptions::new();
        options.set_top(y);
        options.set_behavior(web_sys::ScrollBehavior::Instant);
        self.window.scroll_to_with_scroll_to_options(&options);
    }
    fn scroll_progress(&self) -> f64 {
        let tops: Vec<_> = self
            .anchors
            .iter()
            .map(|a| a.get_bounding_client_rect().top())
            .collect();
        for i in 0..tops.len() - 1 {
            if tops[i + 1] > 0. {
                return (i as f64 + (-tops[i] / (tops[i + 1] - tops[i]).max(1.)).clamp(0., 1.))
                    .clamp(0., (IDS.len() - 1) as f64);
            }
        }
        (IDS.len() - 1) as f64
    }
    fn navigate(&mut self, index: usize, history: bool) {
        self.opening = OpeningShot::Finished;
        if !self.flying {
            self.active = index;
            self.panels[index].scroll_into_view();
            return;
        }
        if history {
            if let Ok(h) = self.window.history() {
                let _ =
                    h.push_state_with_url(&JsValue::NULL, "", Some(&format!("#{}", IDS[index])));
            }
        }
        self.active = index;
        self.target = index as f64;
        self.transition = if (self.progress - self.target).abs() <= 1.1 {
            Some((self.progress, self.target, None))
        } else {
            self.progress = self.target;
            None
        };
        self.focus = true;
        self.align_scroll();
    }
    fn tick(&mut self, now: f64) -> bool {
        self.raf = None;
        if !self.flying || !self.ready {
            return false;
        }
        if let Some((from, to, start)) = self.transition {
            let start = start.unwrap_or(now);
            let t = ((now - start) / TRANSITION_MS).clamp(0., 1.);
            // Quintic ease: start and finish with zero velocity and acceleration.
            let ease = t * t * t * (t * (t * 6. - 15.) + 10.);
            self.progress = from + (to - from) * ease;
            self.transition = if t >= 1. {
                None
            } else {
                Some((from, to, Some(start)))
            };
        } else {
            self.progress = self.target;
        }
        let rect = self.canvas.get_bounding_client_rect();
        // On short phones, give the longest card just enough room rather than
        // clipping its copy or introducing a second scrollable surface.
        if rect.width() <= 700. {
            let top = self
                .window
                .get_computed_style(&self.root)
                .ok()
                .flatten()
                .and_then(|style| style.get_property_value("--tour-top").ok())
                .and_then(|value| value.trim().trim_end_matches("px").parse::<f64>().ok())
                .unwrap_or(82.);
            let card_height = self
                .panels
                .iter()
                .map(|p| p.get_bounding_client_rect().height())
                .fold(0., f64::max);
            let text_height = ((rect.height() - top - 48.) * 0.48).max(card_height + 8.);
            if text_height > rect.height() - top - 140. {
                self.manual_read = true;
                self.apply_mode();
                self.focus_heading();
                return false;
            }
            let _ = self
                .root
                .style()
                .set_property("--village-text-height", &format!("{text_height}px"));
        }
        let slot = self.slot.get_bounding_client_rect();
        let opening_distance = self.opening.distance(now);
        let drawn = self.view.as_ref().map(|view| {
            draw_village(
                view,
                self.progress as f32,
                rect.width() as f32,
                rect.height() as f32,
                &[
                    (slot.left() - rect.left()) as f32,
                    (slot.top() - rect.top()) as f32,
                    slot.width() as f32,
                    slot.height() as f32,
                ],
                opening_distance,
            )
        });
        match drawn {
            Some(Ok(true)) => {
                if self.root.get_attribute("data-entry-state").as_deref() == Some("loading") {
                    if self
                        .doc
                        .active_element()
                        .and_then(|e| e.closest("#tour-loader").ok().flatten())
                        .is_some()
                    {
                        self.focus = true;
                    }
                    self.gate(false);
                    // The loading curtain locks page overflow. Align the document
                    // after unlocking so the first swipe starts at the visible stop.
                    self.align_scroll();
                }
            }
            Some(Err(error)) => {
                tracing::warn!(?error, "Village render failed");
                self.failed = true;
                self.apply_mode();
                self.focus_heading();
                return false;
            }
            _ => return false,
        }
        self.active = (self.progress.round() as usize).min(IDS.len() - 1);
        for (i, panel) in self.panels.iter().enumerate() {
            let opacity = (1. - (self.progress - i as f64).abs() * 2.7).clamp(0., 1.);
            let _ = panel.style().set_property("opacity", &opacity.to_string());
            let _ = panel.style().set_property(
                "visibility",
                if opacity > 0. { "visible" } else { "hidden" },
            );
            let active = i == self.active && opacity > 0.35;
            let _ = panel
                .style()
                .set_property("pointer-events", if active { "auto" } else { "none" });
            let _ = panel.set_attribute("aria-hidden", if active { "false" } else { "true" });
            if active {
                let _ = panel.remove_attribute("inert");
            } else {
                let _ = panel.set_attribute("inert", "");
            }
        }
        let moving = !matches!(self.opening, OpeningShot::Finished)
            || self.transition.is_some()
            || (self.progress - self.target).abs() > 0.0001;
        let _ = self
            .root
            .set_attribute("data-active-stop", &self.active.to_string());
        let _ = self
            .root
            .set_attribute("data-camera-moving", if moving { "true" } else { "false" });
        if !moving {
            if let Ok(status) = el(&self.doc, "tour-chapter-status") {
                status.set_text_content(Some(&format!(
                    "{} of {} · {}",
                    self.active + 1,
                    IDS.len(),
                    NAMES[self.active]
                )));
            }
            if self.focus {
                self.focus_heading();
                self.focus = false;
            }
        }
        moving
    }
}
