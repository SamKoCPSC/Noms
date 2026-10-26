use super::*;

#[component]
fn NavbarDestinationTestRoot(signed_in: bool) -> Element {
    let auth = use_signal(move || crate::auth::context::AuthContext {
        is_authenticated: signed_in,
        ..Default::default()
    });
    use_context_provider(|| auth);
    rsx! {
        dioxus::router::components::HistoryProvider {
            history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
            Router::<Route> {}
        }
    }
}

#[test]
fn brand_logo_always_links_to_landing_regardless_of_authentication() {
    for signed_in in [true, false] {
        let mut dom = VirtualDom::new_with_props(
            NavbarDestinationTestRoot,
            NavbarDestinationTestRootProps { signed_in },
        );
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let class_position = html.find("class=\"navbar-logo\"").unwrap();
        let link_start = html[..class_position].rfind("<a ").unwrap();
        let link_end = html[class_position..].find('>').unwrap() + class_position;
        let logo = &html[link_start..link_end];
        assert!(logo.contains("href=\"/\""), "signed_in={signed_in}: {logo}");
        assert!(logo.contains("aria-label=\"Noms introduction\""), "{logo}");
    }
}

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
            (Route::CommunityHome {}, "Home"),
            (Route::Dashboard {}, "Dashboard"),
            (Route::CollectionList {}, "Collections"),
            (Route::Pantry {}, "Kitchen"),
            (Route::Explore {}, "Explore")
        ]
    );
}

#[test]
fn collection_descendants_keep_their_navigation_section_selected() {
    for route in [
        Route::CollectionList {},
        Route::CollectionNew { parent: None },
        Route::CollectionNew {
            parent: Some(uuid::Uuid::nil()),
        },
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
    assert!(!navigation_is_current(
        &Route::Home {},
        &Route::CommunityHome {}
    ));
    assert_eq!(Route::Home {}.to_string(), "/");
    assert_eq!(Route::CommunityHome {}.to_string(), "/home");
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

#[test]
fn grocery_routes_keep_their_navigation_section_selected() {
    for route in [
        Route::GroceryLists {},
        Route::GroceryListNew {},
        Route::GroceryListDetail { id: "list".into() },
    ] {
        assert!(navigation_is_current(&Route::Pantry {}, &route));
        assert!(!navigation_is_current(&Route::CollectionList {}, &route));
    }
}

#[test]
fn pantry_descendants_stay_in_kitchen_navigation() {
    for route in [
        Route::Pantry {},
        Route::PantryNew {},
        Route::PantryDetail {
            id: "sample".into(),
        },
    ] {
        assert!(navigation_is_current(&Route::Pantry {}, &route));
        assert!(!navigation_is_current(&Route::CollectionList {}, &route));
    }
}
