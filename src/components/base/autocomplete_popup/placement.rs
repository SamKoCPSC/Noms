//! Geometry uses layout-viewport coordinates, including visual-viewport offsets.
#[derive(Clone, Copy)]
pub(crate) struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug)]
pub(crate) struct Placement {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub max_height: f64,
    pub above: bool,
}

pub(crate) fn place(
    anchor: Rect,
    viewport: Rect,
    width: f64,
    height: f64,
    align_end: bool,
) -> Placement {
    const EDGE: f64 = 8.;
    const GAP: f64 = 4.;
    let width = width.min((viewport.right - viewport.left - 2. * EDGE).max(0.));
    let below = (viewport.bottom - EDGE - anchor.bottom - GAP).max(0.);
    let above = (anchor.top - GAP - viewport.top - EDGE).max(0.);
    let opens_above = height > below && above > below;
    let max_height = if opens_above { above } else { below };
    let height = height.min(max_height);
    Placement {
        left: (if align_end {
            anchor.right - width
        } else {
            anchor.left
        })
        .min(viewport.right - EDGE - width)
        .max(viewport.left + EDGE),
        top: if opens_above {
            anchor.top - GAP - height
        } else {
            anchor.bottom + GAP
        }
        .max(viewport.top + EDGE)
        .min((viewport.bottom - EDGE - height).max(viewport.top + EDGE)),
        width,
        max_height,
        above: opens_above,
    }
}

/// Smallest scroll adjustment that brings the input inside the visible viewport.
pub(crate) fn reveal_delta(anchor: Rect, viewport: Rect) -> f64 {
    if anchor.bottom > viewport.bottom - 8. {
        anchor.bottom - viewport.bottom + 8.
    } else if anchor.top < viewport.top + 8. {
        anchor.top - viewport.top - 8.
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport(bottom: f64) -> Rect {
        Rect {
            left: 0.,
            top: 0.,
            right: 390.,
            bottom,
        }
    }
    fn anchor(top: f64) -> Rect {
        Rect {
            left: 20.,
            top,
            right: 300.,
            bottom: top + 48.,
        }
    }

    #[test]
    fn opens_below_when_it_fits_and_flips_above_the_keyboard() {
        let normal = place(anchor(260.), viewport(844.), 280., 192., false);
        assert!(!normal.above);
        assert_eq!(normal.top, 312.);
        let keyboard = place(anchor(260.), viewport(400.), 280., 192., false);
        assert!(keyboard.above);
        assert_eq!(keyboard.top, 64.);
        assert!(keyboard.top + 192. < 400.);
        let reopened = place(anchor(260.), viewport(844.), 280., 192., false);
        assert!(!reopened.above);
    }

    #[test]
    fn limits_height_to_the_larger_side_without_a_forced_minimum() {
        let at = place(anchor(130.), viewport(300.), 280., 320., false);
        assert!(at.above);
        assert_eq!(at.max_height, 118.);
        assert_eq!(at.top, 8.);
    }

    #[test]
    fn panned_and_zoomed_viewports_keep_both_axes_inside_the_visible_area() {
        let view = Rect {
            left: 50.,
            top: 100.,
            right: 350.,
            bottom: 450.,
        };
        let at = place(anchor(300.), view, 500., 192., false);
        assert_eq!(at.width, 284.);
        assert_eq!(at.left, 58.);
        assert!(at.top >= 108.);
        assert!(at.top + 192_f64.min(at.max_height) <= 442.);
    }

    #[test]
    fn offscreen_inputs_can_be_revealed_with_a_minimal_scroll() {
        assert_eq!(reveal_delta(anchor(500.), viewport(400.)), 156.);
        assert_eq!(reveal_delta(anchor(-10.), viewport(400.)), -18.);
        assert_eq!(reveal_delta(anchor(180.), viewport(400.)), 0.);
    }

    #[test]
    fn wider_unit_menus_keep_the_requested_end_alignment() {
        let at = place(anchor(260.), viewport(844.), 208., 160., true);
        assert_eq!(at.left, 92.);
        assert_eq!(at.left + at.width, 300.);
    }
}
