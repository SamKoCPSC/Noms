//! First-person controls for the fenced village.
use super::{capture_walk_mouse, el, listen, schedule, subscribe, State, StateRef};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlElement, KeyboardEvent, MouseEvent, PointerEvent};

const WALK_SPEED: f32 = 2.5;
const SPRINT_SPEED: f32 = 5.;
const ACCELERATION: f32 = 7.5;

#[derive(Default)]
pub(super) struct Walk {
    keys: [bool; 4],
    sprinting: bool,
    stick: [f32; 2],
    stick_id: Option<i32>,
    velocity: [f32; 2],
    blend: f32,
    last: Option<f64>,
}

impl Walk {
    pub fn key(&mut self, key: &KeyboardEvent, down: bool) -> bool {
        let slot = match key.code().as_str() {
            "ShiftLeft" | "ShiftRight" => {
                self.sprinting = key.shift_key();
                return true;
            }
            "KeyW" => 0,
            "KeyS" => 1,
            "KeyA" => 2,
            "KeyD" => 3,
            _ => return false,
        };
        if !key.repeat() {
            self.keys[slot] = down;
            self.sprinting = key.shift_key();
        }
        key.prevent_default();
        true
    }

    fn pause(&mut self) {
        self.keys = [false; 4];
        self.sprinting = false;
        self.stick = [0.; 2];
        self.stick_id = None;
        self.velocity = [0.; 2];
        self.last = None;
    }

    pub fn moving(&self) -> bool {
        self.blend < 1.
            || self.keys.contains(&true)
            || self.stick != [0.; 2]
            || self.velocity != [0.; 2]
    }

    pub fn frame(&mut self, now: f64, returning: bool, yaw: f32, pitch: f32) -> [f32; 5] {
        let dt = ((now - self.last.replace(now).unwrap_or(now)).max(0.) / 1000.) as f32;
        self.blend = (self.blend + if returning { -dt } else { dt } / 1.2).clamp(0., 1.);
        let weight = self.blend * self.blend * (3. - 2. * self.blend);
        let axis = |positive: bool, negative: bool| positive as i8 as f32 - negative as i8 as f32;
        let side = self.stick[0] + axis(self.keys[3], self.keys[2]);
        let forward = self.stick[1] + axis(self.keys[0], self.keys[1]);
        let speed = if self.sprinting {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };
        let scale = speed / side.hypot(forward).max(1.);
        // Bound the physics step after a suspended tab or a slow draw.
        let dt = dt.min(0.08);
        for (speed, axis) in self.velocity.iter_mut().zip([side, forward]) {
            let target = if returning || self.blend < 1. {
                0.
            } else {
                axis * scale
            };
            *speed += (target - *speed).clamp(-ACCELERATION * dt, ACCELERATION * dt);
        }
        if !returning && !self.moving() {
            self.last = None;
        }
        [
            weight,
            yaw,
            pitch,
            self.velocity[0] * dt,
            self.velocity[1] * dt,
        ]
    }
}

impl State {
    pub(super) fn walk(&mut self) -> Option<&mut Walk> {
        self.look.as_mut().map(|look| &mut look.walk)
    }

    pub(super) fn pause_walk(&mut self) {
        if let Some(walk) = self.walk() {
            walk.pause();
            if let Ok(stick) = el(&self.doc, "village-walk-stick") {
                stick.remove_attribute("style").ok();
            }
        }
    }

    pub(super) fn release_walk_mouse(&self) {
        if self
            .doc
            .pointer_lock_element()
            .is_some_and(|e| e.id() == "village-look-surface")
        {
            self.doc.exit_pointer_lock();
        }
    }
}

pub(super) fn bind(state: &StateRef) -> Result<(), JsValue> {
    let doc = state.borrow().doc.clone();
    let window = state.borrow().window.clone();
    subscribe(state, window.as_ref(), "keyup", |state, event| {
        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        if state
            .borrow_mut()
            .walk()
            .is_some_and(|walk| walk.key(key, false))
        {
            schedule(state);
        }
    })?;
    listen(state, window.as_ref(), "blur", |state, _| {
        state.borrow_mut().pause_walk()
    })?;
    listen(state, doc.as_ref(), "visibilitychange", |state, _| {
        if state.borrow().doc.hidden() {
            state.borrow_mut().pause_walk();
        }
    })?;
    listen(state, doc.as_ref(), "pointerlockchange", |state, _| {
        let mut s = state.borrow_mut();
        if s.doc
            .pointer_lock_element()
            .is_some_and(|e| e.id() == "village-look-surface")
        {
            s.root.set_attribute("data-mouse-locked", "").ok();
        } else {
            s.root.remove_attribute("data-mouse-locked").ok();
            s.pause_walk();
        }
    })?;
    subscribe(state, doc.as_ref(), "mousemove", |state, event| {
        let Some(mouse) = event.dyn_ref::<MouseEvent>() else {
            return;
        };
        let mut s = state.borrow_mut();
        if !s.root.has_attribute("data-mouse-locked") {
            return;
        }
        if let Some(look) = s.look.as_mut() {
            look.walk_turn(mouse.movement_x(), mouse.movement_y());
        }
        drop(s);
        schedule(state);
    })?;
    let stick: HtmlElement = el(&doc, "village-walk-stick")?.dyn_into()?;
    for name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "lostpointercapture",
    ] {
        subscribe(state, stick.as_ref(), name, move |state, event| {
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            let Some(stick) = p
                .current_target()
                .and_then(|t| t.dyn_into::<HtmlElement>().ok())
            else {
                return;
            };
            let mut s = state.borrow_mut();
            let Some(walk) = s.walk() else { return };
            let id = p.pointer_id();
            if name == "pointerdown"
                && walk.stick_id.is_none()
                && stick.set_pointer_capture(id).is_ok()
            {
                walk.stick_id = Some(id);
            }
            if walk.stick_id != Some(id) {
                return;
            }
            if name == "pointerdown" || name == "pointermove" {
                let r = stick.get_bounding_client_rect();
                let dx = (f64::from(p.client_x()) - r.left() - r.width() / 2.) as f32;
                let dy = (f64::from(p.client_y()) - r.top() - r.height() / 2.) as f32;
                let range = (r.width() as f32 * 0.32).max(1.);
                let divisor = range.max(dx.hypot(dy));
                walk.stick = [dx / divisor, -dy / divisor];
                stick
                    .style()
                    .set_property("--stick-x", &format!("{}px", walk.stick[0] * range))
                    .ok();
                stick
                    .style()
                    .set_property("--stick-y", &format!("{}px", -walk.stick[1] * range))
                    .ok();
            } else {
                walk.stick_id = None;
                walk.stick = [0.; 2];
                stick.remove_attribute("style").ok();
            }
            p.prevent_default();
            drop(s);
            schedule(state);
        })?;
    }
    Ok(())
}

pub(super) fn capture(surface: &web_sys::Element, pointer: &PointerEvent) {
    if pointer.pointer_type() == "mouse" {
        capture_walk_mouse(surface);
    }
}
