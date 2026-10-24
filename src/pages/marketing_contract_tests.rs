use super::home::{home_cta, HomeCtaDestination};
use super::how_to::HOW_TO_SECTIONS;
use dioxus::prelude::*;

#[component]
fn LandingTestRoot(signed_in: bool) -> Element {
    let auth = use_signal(move || crate::auth::context::AuthContext {
        is_authenticated: signed_in,
        ..Default::default()
    });
    use_context_provider(|| auth);
    rsx! {
        dioxus::router::components::HistoryProvider {
            history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
            Router::<crate::Route> {}
        }
    }
}

#[test]
fn signed_out_home_cta_starts_recipe_creation_through_login() {
    let cta = home_cta(false);
    let home_source = include_str!("home.rs");

    assert_eq!(cta.label, "Start your recipe book");
    assert_eq!(cta.destination, HomeCtaDestination::LoginThenCreateRecipe);
    assert!(home_source.contains("/login?redirect_uri=/recipes/new"));
}

#[test]
fn signed_in_home_cta_opens_dashboard() {
    let cta = home_cta(true);

    assert_eq!(cta.label, "Open dashboard");
    assert_eq!(cta.destination, HomeCtaDestination::Dashboard);
}

#[test]
fn about_renders_the_four_approved_chapters_and_readable_fallback() {
    let html = dioxus_ssr::render_element(rsx! { super::about::About {} });
    for title in [
        "The Ultimate Recipe Manager",
        "A side project that grew in to something larger.",
        "Hi, I&#39;m Sam!",
        "Just a simple and easy-to-use app.",
    ] {
        assert!(html.contains(title), "Missing {title}");
    }
    assert_eq!(html.matches("<article").count(), 4);
    assert!(html.contains("class=\"kitchen-fallback\""));
    assert!(html.contains("loading=\"lazy\""));
    let opening = html.split("id=\"panel-1\"").next().unwrap();
    assert!(opening.contains("kitchen-fallback"));
    let creator = html
        .split("id=\"panel-2\"")
        .nth(1)
        .unwrap()
        .split("id=\"panel-3\"")
        .next()
        .unwrap();
    assert!(!creator.contains("kitchen-fallback"));
    assert!(html.contains("Nowadays I continue to support the project"));
    assert!(!html.contains("How did Noms start"));
}

#[test]
fn about_provides_motion_and_reading_controls_before_enhancement() {
    let html = dioxus_ssr::render_element(rsx! { super::about::About {} });
    assert!(html.contains("id=\"mode-toggle\""));
    assert!(html.contains("id=\"motion-toggle\""));
    assert!(html.contains("Story chapters"));
    for id in ["idea", "why", "creator", "community"] {
        assert!(html.contains(&format!("href=\"#{id}\"")));
    }
}

#[test]
fn about_loading_screen_keeps_a_reading_escape_and_accessible_status() {
    let html = dioxus_ssr::render_element(rsx! { super::about::About {} });
    assert!(html.contains("id=\"tour-loader\""));
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("Read the story instead"));
    assert!(html.contains("class=\"about-tour is-reader\""));
    assert!(!html.contains("data-entry-state=\"loading\""));
}

#[test]
fn how_to_uses_stable_anchor_ids_for_every_documented_workflow() {
    let ids = HOW_TO_SECTIONS
        .iter()
        .map(|section| section.id)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "sign-in",
            "create",
            "images",
            "visibility",
            "edit",
            "versions",
            "scale",
            "collections",
            "explore",
            "fork",
        ]
    );
}

#[test]
fn public_page_copy_does_not_claim_unshipped_full_search() {
    let marketing_copy = include_str!("../../docs/design/NOMS-013/page-copy.md");
    let production_copy = [
        include_str!("home.rs"),
        include_str!("about.rs"),
        include_str!("how_to.rs"),
    ]
    .join("\n");

    assert!(marketing_copy.contains("Full recipe, collection, and creator search is not part"));
    assert!(!production_copy.contains("Search your entire library"));
    assert!(!production_copy.contains("Search recipes, collections, and people"));
}

#[test]
fn landing_preserves_public_exploration_and_authenticated_creation() {
    for signed_in in [false, true] {
        let mut dom =
            VirtualDom::new_with_props(LandingTestRoot, LandingTestRootProps { signed_in });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("The Ultimate Recipe Manager"));
        let stories = html
            .split("class=\"landing-chapters landing-wrap\"")
            .nth(1)
            .unwrap()
            .split("class=\"landing-features landing-wrap\"")
            .next()
            .unwrap();
        assert_eq!(stories.matches("class=\"landing-chapter-copy\"").count(), 6);
        assert!(stories.contains("Use anywhere"));
        assert!(stories.contains("features anywhere with a mobile-optimized experience"));
        assert!(!html.contains("landing-chapter-number"));
        assert!(!html.contains("landing-mobile landing-wrap"));
        assert_eq!(html.matches("class=\"landing-recipe-search\"").count(), 1);
        assert!(html.contains("action=\"/explore\""));
        assert!(html.contains("method=\"get\""));
        assert!(html.contains("name=\"q\""));
        assert!(html.contains("href=\"/explore\""));
        assert_eq!(
            html.contains("href=\"/login?redirect_uri=/recipes/new\""),
            !signed_in
        );
        assert_eq!(html.contains("Open dashboard"), signed_in);
        assert_eq!(html.contains("Create a recipe →"), signed_in);
        assert!(html.contains("Free to use."));
        assert!(html.contains("Your data is never shared or sold."));
        assert!(html.contains("loading=\"lazy\""));
        assert!(!html.contains("<iframe"));
        assert!(!html.contains("Maya Chen"));
    }
}
