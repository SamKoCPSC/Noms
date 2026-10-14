//! Platform-independent overlay decisions, shared by the WASM adapters and tests.
#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

pub(super) fn tooltip_position(
    trigger: Rect,
    width: f64,
    height: f64,
    viewport_width: f64,
    viewport_height: f64,
) -> (f64, f64) {
    let left = ((trigger.left + trigger.right - width) / 2.)
        .min(viewport_width - width - 8.)
        .max(8.);
    let below = trigger.bottom + 8.;
    (
        left,
        if below + height <= viewport_height - 8. {
            below
        } else {
            (trigger.top - height - 8.).max(8.)
        },
    )
}

#[derive(Default)]
pub(super) struct TooltipState {
    pub visible: bool,
    hovered: bool,
    focused: bool,
    dismissed: bool,
}
#[derive(Clone, Copy)]
pub(super) enum TooltipEvent {
    Enter,
    Leave,
    Focus,
    Blur,
    Escape,
    GraceElapsed,
}
impl TooltipState {
    pub fn update(&mut self, event: TooltipEvent, disabled: bool) {
        match event {
            TooltipEvent::Enter => self.hovered = true,
            TooltipEvent::Leave => self.hovered = false,
            TooltipEvent::Focus => self.focused = true,
            TooltipEvent::Blur => self.focused = false,
            TooltipEvent::Escape => {
                self.dismissed = true;
                self.visible = false;
            }
            TooltipEvent::GraceElapsed => {
                if !self.hovered && !self.focused {
                    self.visible = false;
                    self.dismissed = false;
                }
            }
        }
        if matches!(event, TooltipEvent::Enter | TooltipEvent::Focus)
            && !disabled
            && !self.dismissed
        {
            self.visible = true;
        }
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct MenuPosition {
    pub left: f64,
    pub top: f64,
    pub max_height: f64,
}
pub(super) fn natural_height(scroll_height: i32, offset_height: i32, client_height: i32) -> f64 {
    f64::from(scroll_height + offset_height - client_height)
}
pub(super) fn context_menu_position(
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    viewport_width: f64,
    viewport_height: f64,
) -> MenuPosition {
    let max_height = (viewport_height - 16.).max(0.);
    MenuPosition {
        left: left.min(viewport_width - width - 8.).max(8.),
        top: top
            .min(viewport_height - height.min(max_height) - 8.)
            .max(8.),
        max_height,
    }
}
pub(super) fn menu_position(
    trigger: Rect,
    width: f64,
    height: f64,
    viewport_width: f64,
    viewport_height: f64,
) -> MenuPosition {
    let below = (viewport_height - trigger.bottom - 16.).max(0.);
    let above = (trigger.top - 16.).max(0.);
    let opens_above = height > below && above > below;
    let max_height = if opens_above { above } else { below };
    MenuPosition {
        left: (trigger.right - width)
            .min(viewport_width - width - 8.)
            .max(8.),
        top: if opens_above {
            (trigger.top - height.min(max_height) - 8.).max(8.)
        } else {
            trigger.bottom + 8.
        },
        max_height,
    }
}
pub(super) fn menu_target(key: &str, active: Option<usize>, labels: &[String]) -> Option<usize> {
    let count = labels.len();
    if count == 0 {
        return None;
    }
    match key {
        "ArrowDown" => Some(active.map_or(0, |index| (index + 1) % count)),
        "ArrowUp" => Some(active.map_or(count - 1, |index| (index + count - 1) % count)),
        "Home" => Some(0),
        "End" => Some(count - 1),
        key if key.chars().count() == 1 && key != " " => {
            let start = active.map_or(0, |index| (index + 1) % count);
            (0..count)
                .map(|offset| (start + offset) % count)
                .find(|&index| {
                    labels[index]
                        .trim()
                        .to_lowercase()
                        .starts_with(&key.to_lowercase())
                })
        }
        _ => None,
    }
}

#[derive(Debug, PartialEq)]
pub(super) enum TabTarget {
    Native,
    Dialog,
    Control(usize),
}
pub(super) fn tab_target(count: usize, active: Option<usize>, backwards: bool) -> TabTarget {
    if count == 0 {
        TabTarget::Dialog
    } else if active.is_none() || active == Some(if backwards { 0 } else { count - 1 }) {
        TabTarget::Control(if backwards { count - 1 } else { 0 })
    } else {
        TabTarget::Native
    }
}
pub(super) fn tab_candidate(
    disabled: bool,
    hidden: bool,
    index: i32,
    has_rects: bool,
    inert: bool,
    css_disabled: bool,
    visibility: &str,
) -> bool {
    !disabled
        && !hidden
        && index >= 0
        && has_rects
        && !inert
        && !css_disabled
        && visibility != "hidden"
        && visibility != "collapse"
}
pub(super) fn tab_priority(index: i32) -> i32 {
    if index > 0 {
        index
    } else {
        i32::MAX
    }
}
