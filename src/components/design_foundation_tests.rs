//! Source contracts for shared foundations; browser validation covers the cascade.
//! Kept std-only so these checks can also run with `rustc --test` before a full build.

const CSS: &str = include_str!("../../assets/main.css");
const INPUT: &str = include_str!("base/input.rs");

fn rule(selector: &str) -> &str {
    CSS.split_once(&format!("{selector} {{"))
        .unwrap_or_else(|| panic!("missing rule: {selector}"))
        .1
        .split_once('}')
        .unwrap()
        .0
}

fn navbar_rule(selector: &str) -> &str {
    CSS.split_once("/* N1/N4 shared navigation */")
        .unwrap()
        .1
        .split_once(&format!("{selector} {{"))
        .unwrap_or_else(|| panic!("missing navbar rule: {selector}"))
        .1
        .split_once('}')
        .unwrap()
        .0
}

fn token<'a>(block: &'a str, name: &str) -> &'a str {
    block
        .split_once(&format!("{name}:"))
        .unwrap_or_else(|| panic!("missing token: {name}"))
        .1
        .split_once(';')
        .unwrap()
        .0
        .trim()
}

fn luminance(hex: &str) -> f64 {
    let hex = hex.trim_start_matches('#');
    let channel = |start| {
        let value = u8::from_str_radix(&hex[start..start + 2], 16).unwrap() as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    channel(0) * 0.2126 + channel(2) * 0.7152 + channel(4) * 0.0722
}

fn contrast(first: &str, second: &str) -> f64 {
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

#[test]
fn existing_brand_palette_and_fonts_are_preserved() {
    let light = rule(":root");
    for (name, value) in [
        ("--bg-base", "#FAF7F2"),
        ("--surface", "#F5F0E8"),
        ("--accent", "#D9735A"),
        ("--accent-hover", "#C4613F"),
        ("--text-primary", "#2D2A26"),
    ] {
        assert_eq!(token(light, name), value);
    }
    assert!(token(light, "--font-display").contains("Fredoka"));
    assert!(token(light, "--font-body").contains("Nunito"));
    assert_eq!(token(rule(".dark"), "--accent"), "#E8896E");
}

#[test]
fn shared_elevation_has_one_token_contract() {
    for selector in [".neumo-raised", ".neumo-card"] {
        assert!(rule(selector).contains("box-shadow: var(--elevation-raised)"));
    }
    assert!(rule(".neumo-inset").contains("box-shadow: var(--elevation-inset)"));
    let shared = rule(":root, .dark");
    for name in [
        "--neumo-surface",
        "--elevation-raised",
        "--elevation-raised-hover",
        "--elevation-raised-compact",
        "--elevation-inset",
        "--elevation-inset-compact",
    ] {
        assert!(
            shared.contains(&format!("{name}:")),
            "missing shared material token: {name}"
        );
    }
    for selector in [".recipe-card", ".recipe-card--framed"] {
        assert!(
            rule(selector).contains("background: var(--neumo-surface)"),
            "{selector}"
        );
        assert!(
            rule(selector).contains("box-shadow: var(--elevation-raised)"),
            "{selector}"
        );
    }
    assert!(rule(".explore-search .input").contains("box-shadow: var(--elevation-inset)"));
    assert!(rule(".input--shared").contains("background-color: var(--neumo-surface)"));
    for selector in [".recipe-scaler", ".search-input-row", ".entity-result-card"] {
        assert!(
            rule(selector).contains("background: var(--neumo-surface)"),
            "{selector}"
        );
        assert!(
            rule(selector).contains("box-shadow: var(--elevation-raised)"),
            "{selector}"
        );
    }
}

#[test]
fn shared_button_text_roles_meet_aa_in_both_themes() {
    for theme in [":root", ".dark"] {
        let block = rule(theme);
        for background in ["--accent", "--accent-hover"] {
            assert!(
                contrast(token(block, "--on-accent"), token(block, background)) >= 4.5,
                "{theme} {background}"
            );
        }
        assert!(contrast(token(block, "--on-danger"), token(block, "--error")) >= 4.5);
        assert!(contrast(token(block, "--accent-ink"), token(block, "--surface")) >= 4.5);
        assert!(
            contrast(
                token(block, "--field-placeholder"),
                token(block, "--surface")
            ) >= 4.5
        );
        assert!(contrast(token(block, "--focus-ring"), token(block, "--surface")) >= 3.0);
    }
    for selector in [
        ".btn-primary,\n.navbar .navbar-contained-action",
        ".btn-primary:hover:not(:disabled),\n.navbar .navbar-contained-action:hover:not(:disabled)",
        ".btn-secondary:hover:not(:disabled)",
    ] {
        assert!(
            rule(selector).contains("color: var(--on-accent)"),
            "{selector}"
        );
    }
    assert!(rule(".btn-danger").contains("color: var(--on-danger)"));
    assert!(rule(".btn-secondary").contains("color: var(--accent-ink)"));
}

#[test]
fn outlined_buttons_use_shared_neumorphic_material_and_depth() {
    let resting = rule(".btn-secondary");
    assert!(resting.contains("background: var(--neumo-surface)"));
    assert!(resting.contains("box-shadow: var(--elevation-raised-compact)"));
    assert!(resting.contains("position: relative"));
    assert!(resting.contains("--outline-radius: var(--radius-md)"));
    assert!(resting.contains("isolation: isolate"));
    assert!(resting.contains("border: 1px solid transparent"));
    assert!(!resting.contains("var(--btn-edge)"));

    let outline = rule(".btn-secondary::after");
    assert!(outline.contains("inset: 5px"));
    assert!(outline.contains("border: 1px solid var(--accent)"));
    assert!(outline.contains("pointer-events: none"));

    let hovered = rule(".btn-secondary:hover:not(:disabled)");
    assert!(hovered.contains("box-shadow: var(--elevation-raised)"));
    assert!(!hovered.contains("background: var(--accent)"));
    let hover_fill = rule(".btn-secondary:hover:not(:disabled)::before");
    assert!(hover_fill.contains("inset: 6px"));
    assert!(hover_fill.contains("background: var(--accent)"));
    assert!(hover_fill.contains("pointer-events: none"));
    assert!(rule(".btn-secondary:hover:not(:disabled)::after")
        .contains("border-color: var(--on-accent)"));
    assert!(rule(".btn-secondary:active:not(:disabled)")
        .contains("box-shadow: var(--elevation-inset-compact)"));
    assert!(CSS.contains(".btn-secondary::before, .btn-secondary::after { display: none; }"));
}

#[test]
fn contained_primary_actions_share_the_homepage_button_material() {
    let navbar = include_str!("navbar.rs");
    let home = include_str!("../pages/home.rs");
    let image_editor = include_str!("base/image_editor_modal.rs");
    assert!(home.contains("button { class: \"btn btn-primary\", r#type: \"submit\", \"Search\" }"));
    assert!(home.contains("class: \"btn btn-primary touch-target\""));
    for class in [
        "navbar-link navbar-sign-in btn btn-primary navbar-contained-action",
        "navbar-theme-toggle btn btn-primary navbar-contained-action",
        "navbar-menu-toggle btn btn-primary navbar-contained-action",
        "navbar-user-dropdown btn btn-primary navbar-contained-action",
    ] {
        assert!(
            navbar.contains(class),
            "missing contained navbar action: {class}"
        );
    }
    assert!(image_editor
        .contains("class: \"btn btn-primary touch-target image-editor__primary-button\""));
    let resting = rule(".btn-primary,\n.navbar .navbar-contained-action");
    assert!(resting.contains("background: var(--accent)"));
    assert!(resting.contains("color: var(--on-accent)"));
    assert!(resting.contains("inset 0 0 0 5px var(--btn-edge)"));
    let hover = rule(
        ".btn-primary:hover:not(:disabled),\n.navbar .navbar-contained-action:hover:not(:disabled)",
    );
    assert!(hover.contains("background: var(--accent-hover)"));
    assert!(hover.contains("inset 0 0 0 5px var(--btn-edge)"));
    let pressed = rule(".btn-primary:active:not(:disabled),\n.navbar .navbar-contained-action:active:not(:disabled)");
    assert!(pressed.contains("box-shadow: var(--elevation-inset)"));

    let navbar_sizing = navbar_rule(".navbar .navbar-contained-action");
    assert!(navbar_sizing.contains("border: 0"));
    assert!(!navbar_sizing.contains("background:"));
    assert!(!navbar_sizing.contains("box-shadow:"));
    assert!(!CSS.contains(".navbar .navbar-contained-action:hover:not(:disabled) { box-shadow:"));
    assert!(!CSS.contains(".navbar .navbar-contained-action:active:not(:disabled) { box-shadow:"));
    assert!(!CSS.contains(".image-editor__primary-button {"));
    assert!(!CSS.contains(".image-editor__primary-button:hover"));
    assert!(!CSS.contains(".image-editor__primary-button:active"));
    assert!(CSS.contains(".navbar :is(a, button, input):focus-visible"));
    assert!(CSS.contains(
        ".navbar .navbar-contained-action { border: 1px solid CanvasText; box-shadow: none; }"
    ));
}

#[test]
fn navbar_contained_actions_keep_comfortable_padding() {
    let action = navbar_rule(".navbar .navbar-contained-action");
    assert!(action.contains("min-height: 48px"));
    assert!(action.contains("padding: 10px 18px"));

    let icon = rule(".navbar .navbar-theme-toggle.navbar-contained-action");
    assert!(icon.contains("min-width: 48px"));
    assert!(icon.contains("padding: 10px"));
}

#[test]
fn navbar_search_keeps_inset_depth_without_a_visible_border() {
    let header = navbar_rule(".navigation-search--header");
    assert!(header.contains("border: 0"));
    assert!(navbar_rule(".navigation-search").contains("box-shadow: var(--elevation-inset)"));
    assert!(navbar_rule(".navigation-search input").contains("border: 0"));
    assert!(CSS.contains(".navbar :is(a, button, input):focus-visible"));
    assert!(CSS.contains(".navbar, .navigation-search, .navbar-link--current { border: 1px solid CanvasText; box-shadow: none; }"));
}

#[test]
fn shared_fields_and_focus_do_not_depend_on_shadows() {
    assert!(!INPUT.contains("outline: \"none\""));
    assert!(rule(".input").contains("border: 1px solid var(--field-border)"));
    assert!(rule(".input::placeholder").contains("var(--field-placeholder)"));
    assert!(CSS.contains(":focus-visible"));
    assert!(CSS.contains("outline: 2px solid var(--focus-ring)"));
    assert!(CSS.contains("outline-offset: 3px"));
}

#[test]
fn shared_input_presentation_does_not_resize_page_owned_controls() {
    assert!(
        !rule(".input").contains("width: 100%"),
        "recipe visibility select uses the generic input class and must retain intrinsic width"
    );
    assert!(INPUT.contains("input--shared"));
    assert!(rule(".input--shared").contains("width: 100%"));
}

#[test]
fn motion_and_forced_colors_have_shared_fallbacks() {
    let reduced = CSS
        .rsplit_once("@media (prefers-reduced-motion: reduce)")
        .unwrap()
        .1;
    assert!(reduced.contains(".bg-gradient-animated"));
    assert!(reduced.contains(".spinner"));
    assert!(reduced.contains("animation: none"));
    let forced = CSS.rsplit_once("@media (forced-colors: active)").unwrap().1;
    assert!(forced.contains(".btn"));
    assert!(forced.contains(".input"));
    assert!(forced.contains("CanvasText"));
    assert!(
        forced.contains("outline: 2px solid Highlight !important"),
        "legacy page inputs suppress outlines inline; forced colors removes their shadow fallback"
    );
}

#[test]
fn ordinary_modal_material_is_scoped_and_opaque_by_default() {
    let modal = rule(".modal-card:not(.modal-card--editor)");
    assert!(modal.contains("background: var(--overlay-opaque)"));
    assert!(modal.contains("border: 1px solid var(--overlay-border)"));
    assert!(modal.contains("box-shadow: var(--overlay-shadow)"));
    assert!(modal.contains("--text-secondary: var(--overlay-muted)"));
    assert!(modal.contains("overflow-y: auto"));
    assert!(modal.contains("max-height: calc(100dvh - 2 * var(--space-md))"));
    assert!(rule(".modal-card--editor").contains("background: var(--bg-base)"));
    assert!(rule(".glass").contains("background: var(--glass-fill)"));
    let supported = CSS
        .split_once(
            "@supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px)))",
        )
        .unwrap()
        .1;
    assert!(supported.contains("background: var(--overlay-fill)"));
    assert!(supported.contains("backdrop-filter: var(--overlay-blur)"));
}

#[test]
fn modal_text_contrast_survives_extreme_underlays_in_both_themes() {
    for theme in [":root", ".dark"] {
        let block = rule(theme);
        let fill = token(block, "--overlay-fill");
        let channels: Vec<f64> = fill
            .strip_prefix("rgba(")
            .unwrap()
            .strip_suffix(')')
            .unwrap()
            .split(',')
            .map(|part| part.trim().parse().unwrap())
            .collect();
        assert_eq!(channels.len(), 4);
        assert!(channels[3] > 0.0 && channels[3] < 1.0);
        for underlay in [0.0, 255.0] {
            let composite: Vec<u8> = channels[..3]
                .iter()
                .map(|channel| {
                    (channel * channels[3] + underlay * (1.0 - channels[3])).round() as u8
                })
                .collect();
            let background = format!(
                "#{:02X}{:02X}{:02X}",
                composite[0], composite[1], composite[2]
            );
            for foreground in ["--text-primary", "--overlay-muted"] {
                assert!(
                    contrast(token(block, foreground), &background) >= 4.5,
                    "{theme} {foreground} on {background}"
                );
            }
        }
    }
}

#[test]
fn ordinary_modal_material_has_preference_and_system_fallbacks() {
    let transparency = CSS
        .rsplit_once("@media (prefers-reduced-transparency: reduce)")
        .unwrap()
        .1;
    assert!(transparency.contains(".modal-card:not(.modal-card--editor)"));
    assert!(transparency.contains("background: var(--overlay-opaque)"));
    assert!(transparency.contains("backdrop-filter: none"));
    let motion = CSS
        .rsplit_once("@media (prefers-reduced-motion: reduce)")
        .unwrap()
        .1;
    assert!(motion.contains(".modal-overlay"));
    assert!(motion.contains(".modal-card:not(.modal-card--editor)"));
    assert!(motion.contains("animation: none"));
    let forced = CSS.rsplit_once("@media (forced-colors: active)").unwrap().1;
    assert!(forced.contains(".modal-card:not(.modal-card--editor)"));
    assert!(forced.contains("background: Canvas"));
    assert!(forced.contains("--text-secondary: CanvasText"));
    assert!(forced.contains("border: 1px solid CanvasText"));
    assert!(forced.contains("backdrop-filter: none"));
}
