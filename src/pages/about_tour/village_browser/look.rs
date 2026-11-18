//! Look around holds the current tour pose and adds a temporary view rotation.
use super::walk::{self, Walk};
use super::{el, schedule, subscribe, State, StateRef};
use std::f32::consts::{PI, TAU};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{KeyboardEvent, PointerEvent};

#[derive(Default)]
pub(super) struct Look {
    pub(super) walk: Option<Walk>,
    yaw: f32,
    tilt: f32,
    drag: Option<(i32, i32, i32)>,
    // A return starts on its first animation frame, using the RAF clock.
    returning: bool,
    started: Option<f64>,
}

impl Look {
    pub(super) fn walk_turn(&mut self, dx: i32, dy: i32) {
        self.turn(-(dx as f32) * 0.003, -(dy as f32) * 0.002);
    }

    fn turn(&mut self, yaw: f32, tilt: f32) {
        if !self.returning {
            self.yaw = (self.yaw + yaw + PI).rem_euclid(TAU) - PI;
            self.tilt = (self.tilt + tilt).clamp(-1., 1.);
        }
    }
}

impl State {
    pub(super) fn explore_label(&self) -> &'static str {
        if self.progress <= 1. {
            "Explore village"
        } else {
            "Look around"
        }
    }

    pub(super) fn toggle_look(&mut self) {
        if let Some(look) = self.look.as_mut() {
            look.returning = true;
            look.drag = None;
            self.release_walk_mouse();
            self.pause_walk();
        } else {
            let walk = (self.progress <= 1.).then(Walk::default);
            if walk.is_some() {
                let _ = self.root.set_attribute("data-walking", "");
            }
            self.look = Some(Look {
                walk,
                ..Look::default()
            });
            self.transition = None;
            self.focus = false;
            self.target = self.progress;
            self.align_scroll();
            let _ = self.root.set_attribute("data-looking", "");
            self.look_button.set_text_content(Some("Resume tour"));
            let _ = self.look_button.focus();
            self.gate(false);
        }
    }

    pub(super) fn clear_look(&mut self) {
        self.release_walk_mouse();
        self.pause_walk();
        if self.look.take().is_some() {
            let _ = self.root.remove_attribute("data-walking");
            let _ = self.root.remove_attribute("data-mouse-locked");
            let _ = self.root.remove_attribute("data-looking");
            self.look_button
                .set_text_content(Some(self.explore_label()));
            self.gate(false);
        }
    }

    pub(super) fn look_frame(&mut self, now: f64) -> ([f32; 3], Option<[f32; 5]>) {
        let Some(look) = &mut self.look else {
            return ([0.; 3], None);
        };
        if let Some(walk) = &mut look.walk {
            let frame = walk.frame(
                now,
                look.returning,
                look.yaw,
                look.tilt * 80_f32.to_radians(),
            );
            if look.returning && frame[0] <= 0. {
                self.clear_look();
                self.align_scroll();
                return ([0.; 3], None);
            }
            return ([0.; 3], Some(frame));
        }
        let weight = if look.returning {
            let t = ((now - *look.started.get_or_insert(now)) / 1200.).clamp(0., 1.) as f32;
            if t >= 1. {
                self.clear_look();
                self.align_scroll();
                return ([0.; 3], None);
            }
            1. - t * t * (3. - 2. * t)
        } else {
            1.
        };
        ([look.yaw * weight, look.tilt * weight, weight], None)
    }

    pub(super) fn look_moving(&self) -> bool {
        self.look
            .as_ref()
            .is_some_and(|look| look.returning || look.walk.as_ref().is_some_and(Walk::moving))
    }

    pub(super) fn look_key(&mut self, key: &KeyboardEvent) {
        let Some(look) = &mut self.look else { return };
        if !look.returning && look.walk.as_mut().is_some_and(|walk| walk.key(key, true)) {
            return;
        }
        if key.key() == "Escape" && look.walk.is_some() {
            self.release_walk_mouse();
            self.pause_walk();
            key.prevent_default();
            return;
        }
        match key.key().as_str() {
            "ArrowLeft" => look.turn(0.07, 0.),
            "ArrowRight" => look.turn(-0.07, 0.),
            "ArrowUp" => look.turn(0., 0.05),
            "ArrowDown" => look.turn(0., -0.05),
            "Escape" => self.toggle_look(),
            _ => return,
        }
        key.prevent_default();
    }
}

pub(super) fn bind_drag(state: &StateRef) -> Result<(), JsValue> {
    let surface = el(&state.borrow().doc, "village-look-surface")?;
    for name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "pointercancel",
        "lostpointercapture",
    ] {
        subscribe(state, surface.as_ref(), name, move |state, event| {
            let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            let mut s = state.borrow_mut();
            let locked = s.root.has_attribute("data-mouse-locked");
            let Some(look) = &mut s.look else { return };
            let id = pointer.pointer_id();
            let (x, y) = (pointer.client_x(), pointer.client_y());
            match (name, look.drag) {
                ("pointerdown", None)
                    if (look.walk.is_some() || pointer.is_primary())
                        && pointer.button() == 0
                        && !look.returning =>
                {
                    if let Some(surface) = pointer
                        .current_target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    {
                        if look.walk.is_some() {
                            walk::capture(&surface, pointer);
                        }
                        if surface.set_pointer_capture(id).is_ok() {
                            look.drag = Some((id, x, y));
                            pointer.prevent_default();
                        }
                    }
                }
                ("pointermove", Some((active, prev_x, prev_y))) if id == active => {
                    if look.walk.is_some() {
                        if !locked {
                            look.walk_turn(x - prev_x, y - prev_y);
                        }
                    } else {
                        look.turn((x - prev_x) as f32 * 0.003, (y - prev_y) as f32 * 0.002);
                    }
                    look.drag = Some((id, x, y));
                    drop(s);
                    schedule(state);
                }
                ("pointerup" | "pointercancel" | "lostpointercapture", Some((active, _, _)))
                    if id == active =>
                {
                    look.drag = None
                }
                _ => {}
            }
        })?;
    }
    Ok(())
}
