//! Mobile viewport behavior; browser registrations live for the application scope.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_arch = "wasm32")]
pub use browser::mount;

fn keyboard_offset(
    width: f64,
    editing: bool,
    layout_height: f64,
    visual_viewport: Option<(f64, f64)>,
) -> f64 {
    let Some((height, offset_top)) = visual_viewport else {
        return 0.0;
    };
    let offset = (layout_height - height - offset_top).max(0.0);
    if width <= 760.0 && editing && offset > 120.0 {
        offset
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::keyboard_offset;

    #[test]
    fn keyboard_offset_accounts_for_viewport_panning() {
        assert_eq!(
            keyboard_offset(390.0, true, 844.0, Some((524.0, 20.0))),
            300.0
        );
    }

    #[test]
    fn browser_chrome_and_unfocused_fields_do_not_open_keyboard_layout() {
        for height in [844.0, 800.0, 724.0, 900.0] {
            assert_eq!(
                keyboard_offset(390.0, true, 844.0, Some((height, 0.0))),
                0.0
            );
        }
        assert_eq!(
            keyboard_offset(390.0, false, 844.0, Some((500.0, 0.0))),
            0.0
        );
    }

    #[test]
    fn keyboard_layout_is_mobile_only_and_viewport_support_is_optional() {
        assert_eq!(
            keyboard_offset(760.0, true, 844.0, Some((500.0, 0.0))),
            344.0
        );
        assert_eq!(keyboard_offset(761.0, true, 844.0, Some((500.0, 0.0))), 0.0);
        assert_eq!(keyboard_offset(390.0, true, 844.0, None), 0.0);
    }
}
