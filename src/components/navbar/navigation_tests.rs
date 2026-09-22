use super::*;

#[test]
fn primary_destinations_match_authentication_without_inventing_routes() {
    assert_eq!(
        primary_navigation(false),
        vec![
            (Route::Home {}, "Home"),
            (Route::Explore {}, "Explore"),
            (Route::About {}, "About"),
            (Route::HowTo {}, "How to")
        ]
    );
    assert_eq!(
        primary_navigation(true),
        vec![
            (Route::Dashboard {}, "Dashboard"),
            (Route::CollectionList {}, "Collections"),
            (Route::Explore {}, "Explore")
        ]
    );
}

#[test]
fn collection_descendants_keep_their_navigation_section_selected() {
    for route in [
        Route::CollectionList {},
        Route::CollectionNew {},
        Route::CollectionDetail {
            id: "sample".into(),
        },
        Route::CollectionEdit {
            id: "sample".into(),
        },
    ] {
        assert!(navigation_is_current(&Route::CollectionList {}, &route));
        assert!(!navigation_is_current(&Route::Explore {}, &route));
    }
    assert!(navigation_is_current(
        &Route::Search {
            query: Default::default()
        },
        &Route::Search {
            query: "q=soup".into()
        }
    ));
    assert!(!navigation_is_current(
        &Route::Home {},
        &Route::Dashboard {}
    ));
}

#[test]
fn navigation_uses_one_accessible_drawer_and_native_search() {
    let source = include_str!("../navbar.rs");
    assert!(source.contains("SharedModal {"));
    assert!(!source.contains("sidebar_open"));
    assert!(source.contains("aria_haspopup: \"dialog\""));
    assert!(source.contains("method: \"get\""));
    assert!(source.contains("action: \"/search\""));
    assert!(source.contains("name: \"q\""));
    assert!(source.contains("landing_search_route(&text())"));
    assert!(source.contains("on_navigate: move |_| menu_open.set(false)"));
    assert!(source.contains("on_navigate.call(())"));
    assert!(source.contains("maxlength: \"256\""));
    assert!(source.contains("\"My profile\""));
    assert!(source.contains("\"Collections\""));
}

#[test]
fn navigation_is_neumorphic_with_bounded_responsive_controls() {
    let css = include_str!("../../../assets/main.css");
    let navigation = css
        .split("/* N1/N4 shared navigation */")
        .nth(1)
        .unwrap()
        .split("/* End shared navigation */")
        .next()
        .unwrap();
    assert!(navigation.contains("background: var(--surface)"));
    assert!(navigation.contains("box-shadow: var(--elevation-raised)"));
    assert!(!navigation.contains("backdrop-filter"));
    assert!(navigation.contains("@media (max-width: 1099px)"));
    assert!(navigation.contains("overflow-wrap: anywhere"));
    assert!(navigation.contains(":focus-visible"));
}
