//! Explore the village from the current tour position, then return to the saved pose.
use super::walk::{self, Walk};
use super::{el, schedule, subscribe, State, StateRef};
use std::f32::consts::{PI, TAU};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{KeyboardEvent, PointerEvent};

#[derive(Default)]
pub(super) struct Look {
    pub(super) walk: Walk,
    yaw: f32,
    tilt: f32,
    drag: Option<(i32, i32, i32)>,
    returning: bool,
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
    pub(super) fn toggle_look(&mut self) {
        if let Some(look) = self.look.as_mut() {
            look.returning = true;
            look.drag = None;
            self.release_walk_mouse();
            self.pause_walk();
        } else {
            let _ = self.root.set_attribute("data-walking", "");
            self.look = Some(Look::default());
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
            self.look_button.set_text_content(Some("Explore village"));
            self.gate(false);
        }
    }

    pub(super) fn look_frame(&mut self, now: f64) -> Option<[f32; 5]> {
        let look = self.look.as_mut()?;
        let frame = look.walk.frame(
            now,
            look.returning,
            look.yaw,
            look.tilt * 80_f32.to_radians(),
        );
        if look.returning && frame[0] <= 0. {
            self.clear_look();
            self.align_scroll();
            return None;
        }
        Some(frame)
    }

    pub(super) fn look_moving(&self) -> bool {
        self.look
            .as_ref()
            .is_some_and(|look| look.returning || look.walk.moving())
    }

    pub(super) fn look_key(&mut self, key: &KeyboardEvent) {
        let Some(look) = &mut self.look else { return };
        if !look.returning && look.walk.key(key, true) {
            return;
        }
        if key.key() == "Escape" {
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
                ("pointerdown", None) if pointer.button() == 0 && !look.returning => {
                    if let Some(surface) = pointer
                        .current_target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                    {
                        walk::capture(&surface, pointer);
                        if surface.set_pointer_capture(id).is_ok() {
                            look.drag = Some((id, x, y));
                            pointer.prevent_default();
                        }
                    }
                }
                ("pointermove", Some((active, prev_x, prev_y))) if id == active => {
                    if !locked {
                        look.walk_turn(x - prev_x, y - prev_y);
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
