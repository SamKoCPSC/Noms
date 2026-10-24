use super::{flight, scene::Renderer};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{
    Document, Element, Event, EventTarget, HtmlCanvasElement, HtmlElement, MediaQueryList,
    MouseEvent, Window,
};
const IDS: [&str; 4] = ["idea", "why", "creator", "community"];
const NAMES: [&str; 4] = ["The idea", "The reason", "The creator", "The philosophy"];
type StateRef = Rc<RefCell<State>>;
thread_local! {static RUNTIME:RefCell<Option<StateRef>>=const{RefCell::new(None)};}
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
    renderer: Option<Renderer>,
    media: MediaQueryList,
    panels: Vec<HtmlElement>,
    anchors: Vec<Element>,
    nav: Vec<Element>,
    motion_button: Element,
    mode_button: Element,
    raf: Option<i32>,
    callback: Option<Closure<dyn FnMut(f64)>>,
    listeners: Vec<Listener>,
    target: f32,
    progress: f32,
    pointer: [f32; 2],
    pointer_target: [f32; 2],
    width: f32,
    height: f32,
    paused: bool,
    reduced: bool,
    manual_read: bool,
    flying: bool,
    dirty: bool,
    active: usize,
    last: f64,
    time: f32,
    frames: u64,
    initial_alignment: u8,
    kitchen_requested: bool,
    generation: u32,
}
impl Drop for State {
    fn drop(&mut self) {
        if let Some(id) = self.raf {
            let _ = self.window.cancel_animation_frame(id);
        }
    }
}
fn el(doc: &Document, id: &str) -> Result<Element, JsValue> {
    doc.get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(id))
}
pub fn start() -> Result<(), JsValue> {
    stop();
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let doc = window
        .document()
        .ok_or_else(|| JsValue::from_str("No document"))?;
    let canvas: HtmlCanvasElement = el(&doc, "story-canvas")?.dyn_into()?;
    let media = window
        .match_media("(prefers-reduced-motion: reduce)")?
        .ok_or_else(|| JsValue::from_str("No media query"))?;
    let renderer = if media.matches() {
        None
    } else {
        Renderer::new(canvas.clone(), &doc).ok()
    };
    let state = Rc::new(RefCell::new(State {
        window: window.clone(),
        doc: doc.clone(),
        root: el(&doc, "about-tour")?.dyn_into()?,
        canvas: canvas.clone(),
        renderer,
        reduced: media.matches(),
        media: media.clone(),
        panels: (0..4)
            .map(|i| el(&doc, &format!("panel-{i}"))?.dyn_into::<HtmlElement>())
            .collect::<Result<_, _>>()?,
        anchors: (0..4)
            .map(|i| el(&doc, &format!("stop-{i}")))
            .collect::<Result<_, _>>()?,
        nav: (0..4)
            .map(|i| el(&doc, &format!("nav-{i}")))
            .collect::<Result<_, _>>()?,
        motion_button: el(&doc, "motion-toggle")?,
        mode_button: el(&doc, "mode-toggle")?,
        raf: None,
        callback: None,
        listeners: vec![],
        target: 0.,
        progress: 0.,
        pointer: [0.; 2],
        pointer_target: [0.; 2],
        width: 1.,
        height: 1.,
        paused: window
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1000.)
            <= 700.,
        manual_read: false,
        flying: false,
        dirty: true,
        active: usize::MAX,
        last: 0.,
        time: 0.,
        frames: 0,
        initial_alignment: 3,
        kitchen_requested: false,
        generation: 0,
    }));
    let weak = Rc::downgrade(&state);
    let callback = Closure::wrap(Box::new(move |time: f64| {
        if let Some(s) = weak.upgrade() {
            s.borrow_mut().raf = None;
            let again = s.borrow_mut().tick(time);
            maybe_load_kitchen(&s);
            if again {
                schedule(&s)
            }
        }
    }) as Box<dyn FnMut(f64)>);
    state.borrow_mut().callback = Some(callback);
    listen(&state, window.clone().into(), "scroll", |s, _| {
        s.measure();
    });
    listen(&state, window.clone().into(), "resize", |s, _| {
        s.resize();
        s.measure();
    });
    listen(&state, window.clone().into(), "popstate", |s, _| {
        s.goto_hash(false);
    });
    listen(&state, window.clone().into(), "hashchange", |s, _| {
        s.goto_hash(false);
    });
    listen(&state, doc.clone().into(), "visibilitychange", |s, _| {
        s.last = 0.;
        if s.doc.hidden() {
            if let Some(id) = s.raf.take() {
                let _ = s.window.cancel_animation_frame(id);
            }
        } else {
            s.dirty = true;
        }
    });
    listen(&state, media.into(), "change", |s, _| {
        s.reduced = s.media.matches();
        if s.reduced {
            s.renderer = None;
            s.generation += 1;
            s.kitchen_requested = false;
        } else if s.renderer.is_none() {
            s.renderer = Renderer::new(s.canvas.clone(), &s.doc).ok();
        }
        let i = s.active.min(3);
        s.apply_mode();
        s.navigate(i, false, false);
    });
    listen(&state, el(&doc, "mode-toggle")?.into(), "click", |s, _| {
        let i = s.active.min(3);
        s.manual_read = !s.manual_read;
        s.apply_mode();
        s.navigate(i, false, false);
    });
    listen(
        &state,
        el(&doc, "motion-toggle")?.into(),
        "click",
        |s, _| {
            s.paused = !s.paused;
            s.pointer_target = [0.; 2];
            s.dirty = true;
            s.buttons();
        },
    );
    listen(&state, doc.clone().into(), "click", |s, event| {
        if let Some(mouse) = event.dyn_ref::<MouseEvent>() {
            if mouse.button() != 0
                || mouse.ctrl_key()
                || mouse.meta_key()
                || mouse.shift_key()
                || mouse.alt_key()
            {
                return;
            }
        }
        let Some(target) = event.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let Some(link) = target.closest("a[href^='#']").ok().flatten() else {
            return;
        };
        let Some(href) = link.get_attribute("href") else {
            return;
        };
        let Some(i) = IDS
            .iter()
            .position(|id| *id == href.trim_start_matches('#'))
        else {
            return;
        };
        event.prevent_default();
        s.navigate(i, !s.reduced, true);
        if s.flying && link.class_name().contains("next-link") {
            if let Some(nav) = s.nav[i].dyn_ref::<HtmlElement>() {
                let _ = nav.focus();
            }
        }
    });
    listen(&state, doc.clone().into(), "mousemove", |s, e| {
        if !s.flying || s.paused || s.reduced {
            return;
        }
        if let Some(e) = e.dyn_ref::<MouseEvent>() {
            s.pointer_target = [
                e.client_x() as f32 / s.width * 2. - 1.,
                (e.client_y() as f32 - s.canvas.get_bounding_client_rect().top() as f32) / s.height
                    * 2.
                    - 1.,
            ];
            s.dirty = true;
        }
    });
    listen(&state, doc.clone().into(), "mouseleave", |s, _| {
        s.pointer_target = [0.; 2];
        s.dirty = true;
    });
    listen(&state, canvas.clone().into(), "webglcontextlost", |s, e| {
        e.prevent_default();
        let i = s.active.min(3);
        s.renderer = None;
        s.generation += 1;
        s.kitchen_requested = false;
        s.apply_mode();
        s.navigate(i, false, false);
    });
    listen(&state, canvas.into(), "webglcontextrestored", |s, _| {
        let i = s.active.min(3);
        s.renderer = Renderer::new(s.canvas.clone(), &s.doc).ok();
        s.apply_mode();
        s.navigate(i, false, false);
    });
    {
        let mut s = state.borrow_mut();
        s.apply_mode();
        s.goto_hash(false);
        s.measure();
        s.progress = s.target;
    }
    RUNTIME.with(|slot| *slot.borrow_mut() = Some(state.clone()));
    schedule(&state);
    Ok(())
}
pub fn stop() {
    RUNTIME.with(|slot| {
        if let Some(s) = slot.borrow_mut().take() {
            let mut s = s.borrow_mut();
            s.manual_read = true;
            s.apply_mode();
        }
    });
}
fn maybe_load_kitchen(state: &StateRef) {
    let (url, generation) = {
        let mut s = state.borrow_mut();
        if !s.flying || s.reduced || s.kitchen_requested || !(0.95..=2.65).contains(&s.target) {
            return;
        }
        let Some(url) = s.root.get_attribute("data-kitchen-url") else {
            return;
        };
        s.kitchen_requested = true;
        let _ = s.root.set_attribute("data-kitchen-state", "loading");
        (url, s.generation)
    };
    let weak = Rc::downgrade(state);
    wasm_bindgen_futures::spawn_local(async move {
        let result = async {
            let response = gloo_net::http::Request::get(&url)
                .send()
                .await
                .map_err(|_| JsValue::from_str("Kitchen request failed"))?;
            if !response.ok() {
                return Err(JsValue::from_str("Kitchen unavailable"));
            }
            let bytes = response
                .binary()
                .await
                .map_err(|_| JsValue::from_str("Kitchen download failed"))?;
            let data = super::gltf::decode(&bytes).map_err(|e| JsValue::from_str(&e))?;
            Ok::<_, JsValue>(data)
        }
        .await;
        if let Some(state) = weak.upgrade() {
            let mut s = state.borrow_mut();
            if s.generation != generation {
                return;
            }
            let loaded = match result {
                Ok(data) => s
                    .renderer
                    .as_mut()
                    .is_some_and(|r| r.upload_kitchen(&data).is_ok()),
                Err(_) => false,
            };
            let _ = s.root.set_attribute(
                "data-kitchen-state",
                if loaded { "ready" } else { "failed" },
            );
            s.dirty = true;
            drop(s);
            schedule(&state);
        }
    });
}
fn listen(state: &StateRef, target: EventTarget, name: &'static str, f: fn(&mut State, Event)) {
    let weak: Weak<RefCell<State>> = Rc::downgrade(state);
    let callback = Closure::wrap(Box::new(move |event: Event| {
        if let Some(s) = weak.upgrade() {
            f(&mut s.borrow_mut(), event);
            schedule(&s);
        }
    }) as Box<dyn FnMut(Event)>);
    if target
        .add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())
        .is_ok()
    {
        state.borrow_mut().listeners.push(Listener {
            target,
            name,
            callback,
        });
    }
}
fn schedule(s: &StateRef) {
    let mut s = s.borrow_mut();
    if s.raf.is_some() || s.doc.hidden() {
        return;
    }
    if let Some(cb) = &s.callback {
        if let Ok(id) = s
            .window
            .request_animation_frame(cb.as_ref().unchecked_ref())
        {
            s.raf = Some(id)
        }
    }
}
impl State {
    fn buttons(&self) {
        let _ = self
            .motion_button
            .set_attribute("aria-pressed", if self.paused { "true" } else { "false" });
        let _ = self.motion_button.set_attribute(
            "aria-label",
            if self.paused {
                "Resume ambient motion"
            } else {
                "Pause ambient motion"
            },
        );
        if let Ok(label) = el(&self.doc, "motion-label") {
            label.set_text_content(Some(if self.paused {
                "Resume drift"
            } else {
                "Pause drift"
            }));
        }
        self.mode_button.set_text_content(Some(if self.flying {
            "Read as page"
        } else if self.reduced {
            "Reduced motion"
        } else if self.renderer.is_none() {
            "Reading view"
        } else {
            "Enter 3D tour"
        }));
        let _ = self
            .mode_button
            .set_attribute("aria-pressed", if self.flying { "false" } else { "true" });
        if self.renderer.is_none() || self.reduced {
            let _ = self.mode_button.set_attribute("disabled", "");
        } else {
            let _ = self.mode_button.remove_attribute("disabled");
        }
        let _ = self.motion_button.remove_attribute("disabled");
    }
    fn apply_mode(&mut self) {
        self.flying = self.renderer.is_some() && !self.reduced && !self.manual_read;
        {
            let body = &self.root;
            body.set_class_name(if self.flying {
                "about-tour is-flight"
            } else {
                "about-tour is-reader"
            });
            let _ = body.set_attribute(
                "data-renderer",
                if self.renderer.is_some() {
                    "webgl"
                } else {
                    "fallback"
                },
            );
        }
        // The app router also resolves fragment links. In flight mode the real anchor
        // must be the document waypoint, not a span inside a fixed floating panel.
        for (i, panel) in self.panels.iter().enumerate() {
            if let Ok(Some(anchor)) = panel.query_selector(".reader-anchor") {
                let _ = anchor.set_attribute(
                    "id",
                    &if self.flying {
                        format!("reader-{}", IDS[i])
                    } else {
                        IDS[i].to_string()
                    },
                );
            }
            let _ = self.anchors[i].set_attribute(
                "id",
                &if self.flying {
                    IDS[i].to_string()
                } else {
                    format!("stop-{i}")
                },
            );
        }
        for panel in &self.panels {
            let _ = panel.remove_attribute("style");
            let _ = panel.remove_attribute("inert");
            let _ = panel.remove_attribute("aria-hidden");
        }
        self.resize();
        self.buttons();
        self.dirty = true;
    }
    fn resize(&mut self) {
        let rect = self.canvas.get_bounding_client_rect();
        self.width = rect.width().max(1.) as f32;
        self.height = rect.height().max(1.) as f32;
        if self.flying {
            if let Some(r) = &self.renderer {
                r.resize(self.window.device_pixel_ratio());
            }
        }
        self.dirty = true;
    }
    fn navigate(&mut self, i: usize, smooth: bool, history: bool) {
        if history
            && !self
                .doc
                .url()
                .unwrap_or_default()
                .starts_with("about:srcdoc")
        {
            if let Ok(h) = self.window.history() {
                let hash = format!("#{}", IDS[i]);
                if self.window.location().hash().unwrap_or_default() != hash {
                    let _ = h.push_state_with_url(&JsValue::NULL, "", Some(&hash));
                }
            }
        }
        let behavior = if smooth {
            web_sys::ScrollBehavior::Smooth
        } else {
            web_sys::ScrollBehavior::Instant
        };
        if self.flying {
            let y = self.anchors[i].get_bounding_client_rect().top()
                + self.window.scroll_y().unwrap_or(0.);
            let opt = web_sys::ScrollToOptions::new();
            opt.set_top(y);
            opt.set_behavior(behavior);
            self.window.scroll_to_with_scroll_to_options(&opt);
        } else {
            let opt = web_sys::ScrollIntoViewOptions::new();
            opt.set_block(web_sys::ScrollLogicalPosition::Start);
            opt.set_behavior(behavior);
            self.panels[i].scroll_into_view_with_scroll_into_view_options(&opt);
            self.active = i;
        }
        self.measure();
        if !smooth {
            self.progress = self.target;
        }
        self.dirty = true;
    }
    fn goto_hash(&mut self, smooth: bool) {
        let hash = self.window.location().hash().unwrap_or_default();
        if let Some(i) = IDS
            .iter()
            .position(|id| *id == hash.trim_start_matches('#'))
        {
            self.navigate(i, smooth, false);
        }
    }
    fn measure(&mut self) {
        if self.flying {
            let y = self.window.scroll_y().unwrap_or(0.);
            let a = self.anchors[0].get_bounding_client_rect().top() + y;
            let step = self.anchors[1].get_bounding_client_rect().top() + y - a;
            self.target = ((y - a) / step.max(1.)).clamp(0., 3.) as f32;
        } else {
            let mut closest = 0;
            let mut distance = f64::MAX;
            for (i, p) in self.panels.iter().enumerate() {
                let d = (p.get_bounding_client_rect().top() - 100.).abs();
                if d < distance {
                    distance = d;
                    closest = i
                }
            }
            self.active = closest;
        }
        self.dirty = true;
    }
    fn tick(&mut self, now: f64) -> bool {
        // Hydration/router scroll restoration and the first CSS layout can follow onmounted.
        // Re-align the fragment after those initial animation frames, then leave native scrolling alone.
        if self.initial_alignment > 0 {
            self.initial_alignment -= 1;
            self.goto_hash(false);
            self.measure();
            self.progress = self.target;
        }
        if self.doc.hidden() {
            return false;
        }
        if !self.flying {
            self.dirty = false;
            return self.initial_alignment > 0;
        }
        if !self.dirty && self.last != 0. && now - self.last < 30. {
            return true;
        }
        let dt = if self.last == 0. {
            0.016
        } else {
            ((now - self.last) * 0.001).clamp(0., 0.08) as f32
        };
        self.last = now;
        let motion = !self.paused;
        let blend = 1. - (-dt * 11.).exp();
        self.progress += (self.target - self.progress) * blend;
        if (self.target - self.progress).abs() < 0.0004 {
            self.progress = self.target;
        }
        for i in 0..2 {
            self.pointer[i] +=
                (if motion { self.pointer_target[i] } else { 0. } - self.pointer[i]) * blend;
        }
        if motion {
            self.time += dt;
        }
        let camera = flight::camera(
            self.progress,
            self.width / self.height.max(1.),
            self.pointer,
        );
        let active = (camera.progress + 0.5).floor().clamp(0., 3.) as usize;
        if self.active != active {
            self.active = active;
            for i in 0..4 {
                let _ = self.nav[i]
                    .set_attribute("aria-current", if i == active { "step" } else { "false" });
            }
            if let Ok(e) = el(&self.doc, "chapter-count") {
                e.set_text_content(Some(&format!("{:02} / 04", active + 1)));
            }
            if let Ok(e) = el(&self.doc, "stop-name") {
                e.set_text_content(Some(NAMES[active]));
            }
        }
        if let Ok(e) = el(&self.doc, "route-progress") {
            if let Some(e) = e.dyn_ref::<HtmlElement>() {
                let _ = e
                    .style()
                    .set_property("width", &format!("{}%", camera.progress * (100. / 3.)));
            }
        }
        for (i, panel) in self.panels.iter().enumerate() {
            let model = flight::panel_model(
                i,
                self.width,
                self.height,
                panel.offset_width() as f32,
                panel.offset_height() as f32,
            );
            let (m, depth) = flight::css_projection(camera.vp, model, self.width, self.height);
            let delta = camera.progress - i as f32;
            let opacity = if !(2.5..=48.).contains(&depth) {
                0.
            } else if delta > 0. {
                1. - super::math::smooth(0.12, 0.55, delta)
            } else {
                1. - super::math::smooth(0.55, 1.65, -delta)
            };
            let style = panel.style();
            let matrix = m
                .iter()
                .map(|v| format!("{:.8}", v / 10.))
                .collect::<Vec<_>>()
                .join(",");
            let _ = style.set_property("transform", &format!("matrix3d({matrix})"));
            let _ = style.set_property("opacity", &format!("{opacity:.4}"));
            let _ = style.set_property(
                "visibility",
                if opacity < 0.015 { "hidden" } else { "visible" },
            );
            let _ = style.set_property("z-index", &(100 - (depth.max(0.) * 2.) as i32).to_string());
            let interactive = i == active && opacity > 0.8;
            let _ = panel.set_attribute("data-active", if interactive { "true" } else { "false" });
            let _ = panel.set_attribute("aria-hidden", if interactive { "false" } else { "true" });
            if interactive {
                let _ = panel.remove_attribute("inert");
            } else {
                let _ = panel.set_attribute("inert", "");
            }
        }
        if let Some(r) = &self.renderer {
            r.render(&camera, self.time, motion, self.width <= 700.);
            self.frames += 1;
            let _ = self
                .canvas
                .set_attribute("data-travel-progress", &format!("{:.5}", self.progress));
            let _ = self
                .canvas
                .set_attribute("data-render-count", &self.frames.to_string());
            let _ = self
                .canvas
                .set_attribute("data-progress", &format!("{:.4}", camera.progress));
            let _ = self.canvas.set_attribute(
                "data-camera",
                &format!(
                    "{:.4},{:.4},{:.4}",
                    camera.eye[0], camera.eye[1], camera.eye[2]
                ),
            );
            let _ = self
                .canvas
                .set_attribute("data-active-chapter", &active.to_string());
            if self.frames == 1 {
                let _ = self
                    .canvas
                    .set_attribute("data-gl-error", &r.error().to_string());
            }
        }
        self.dirty = false;
        self.initial_alignment > 0 || motion || (self.target - self.progress).abs() > 0.0004
    }
}
