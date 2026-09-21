use super::overlay_logic::*;

#[test]
fn tooltip_geometry_preserves_centering_clamping_and_flip() {
    assert_eq!(
        tooltip_position(
            Rect {
                left: 100.,
                right: 140.,
                top: 40.,
                bottom: 70.
            },
            100.,
            30.,
            500.,
            400.
        ),
        (70., 78.)
    );
    assert_eq!(
        tooltip_position(
            Rect {
                left: 470.,
                right: 490.,
                top: 350.,
                bottom: 380.
            },
            120.,
            40.,
            500.,
            400.
        ),
        (372., 302.)
    );
    assert_eq!(
        tooltip_position(
            Rect {
                left: 0.,
                right: 10.,
                top: 40.,
                bottom: 70.
            },
            120.,
            40.,
            500.,
            400.
        )
        .0,
        8.
    );
}

#[test]
fn tooltip_escape_hover_gap_and_disabled_behavior() {
    let mut state = TooltipState::default();
    state.update(TooltipEvent::Focus, false);
    assert!(state.visible);
    state.update(TooltipEvent::Escape, false);
    assert!(!state.visible);
    state.update(TooltipEvent::Enter, false);
    assert!(!state.visible);
    state.update(TooltipEvent::Leave, false);
    state.update(TooltipEvent::Blur, false);
    state.update(TooltipEvent::GraceElapsed, false);
    state.update(TooltipEvent::Enter, false);
    assert!(state.visible);
    state.update(TooltipEvent::Leave, false);
    state.update(TooltipEvent::Enter, false);
    state.update(TooltipEvent::GraceElapsed, false);
    assert!(state.visible);
    state.update(TooltipEvent::Leave, false);
    state.update(TooltipEvent::GraceElapsed, false);
    state.update(TooltipEvent::Focus, true);
    assert!(!state.visible);
}

#[test]
fn menu_geometry_uses_natural_height_without_expanding_scroll_container() {
    assert_eq!(
        menu_position(
            Rect {
                left: 900.,
                right: 990.,
                top: 20.,
                bottom: 60.
            },
            280.,
            320.,
            1000.,
            800.
        ),
        MenuPosition {
            left: 710.,
            top: 68.,
            max_height: 724.
        }
    );
    assert_eq!(
        menu_position(
            Rect {
                left: 0.,
                right: 90.,
                top: 600.,
                bottom: 640.
            },
            280.,
            320.,
            400.,
            700.
        ),
        MenuPosition {
            left: 8.,
            top: 272.,
            max_height: 584.
        }
    );
    assert_eq!(natural_height(294, 156, 153), 297.);
    assert_eq!(
        menu_position(
            Rect {
                left: 900.,
                right: 990.,
                top: 20.,
                bottom: 60.
            },
            280.,
            297.,
            1000.,
            225.
        )
        .max_height,
        149.
    );
}

#[test]
fn menu_navigation_wraps_and_supports_home_end_and_typeahead() {
    let labels = vec![
        "Dashboard".into(),
        "My profile".into(),
        "Settings".into(),
        "Sign Out".into(),
    ];
    for (key, active, want) in [
        ("ArrowDown", Some(3), Some(0)),
        ("ArrowUp", Some(0), Some(3)),
        ("Home", Some(3), Some(0)),
        ("End", Some(0), Some(3)),
        ("s", Some(0), Some(2)),
        ("s", Some(2), Some(3)),
        ("x", Some(0), None),
    ] {
        assert_eq!(menu_target(key, active, &labels), want);
    }
    assert_eq!(menu_target("ArrowDown", None, &[]), None);
}

#[test]
fn modal_tab_boundaries_and_unavailable_controls() {
    assert_eq!(tab_target(2, Some(0), false), TabTarget::Native);
    assert_eq!(tab_target(2, Some(0), true), TabTarget::Control(1));
    assert_eq!(tab_target(2, Some(1), false), TabTarget::Control(0));
    assert_eq!(tab_target(2, Some(1), true), TabTarget::Native);
    assert_eq!(tab_target(0, None, false), TabTarget::Dialog);
    assert_eq!(tab_target(1, None, false), TabTarget::Control(0));
    assert!(tab_candidate(
        false, false, 0, true, false, false, "visible"
    ));
    for (disabled, hidden, index, rects, inert, css_disabled, visibility) in [
        (true, false, 0, true, false, false, "visible"),
        (false, true, 0, true, false, false, "visible"),
        (false, false, -1, true, false, false, "visible"),
        (false, false, 0, false, false, false, "visible"),
        (false, false, 0, true, true, false, "visible"),
        (false, false, 0, true, false, true, "visible"),
        (false, false, 0, true, false, false, "hidden"),
        (false, false, 0, true, false, false, "collapse"),
    ] {
        assert!(!tab_candidate(
            disabled,
            hidden,
            index,
            rects,
            inert,
            css_disabled,
            visibility
        ));
    }
    assert_eq!(tab_priority(0), i32::MAX);
    assert_eq!(tab_priority(3), 3);
}

#[test]
fn overlay_consumers_no_longer_import_javascript_lifecycle_modules() {
    for source in [
        include_str!("shared_modal.rs"),
        include_str!("feedback.rs"),
        include_str!("../navbar.rs"),
    ] {
        assert!(!source.contains("wasm_bindgen(module"));
    }
    for file in ["shared_modal.js", "feedback.js", "account-menu.js"] {
        assert!(!std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/components/base")
            .join(file)
            .exists());
    }
}
