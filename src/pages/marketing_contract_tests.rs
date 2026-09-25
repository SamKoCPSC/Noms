use super::about::ABOUT_SECTIONS;
use super::home::{home_cta, HomeCtaDestination};
use super::how_to::HOW_TO_SECTIONS;

#[test]
fn signed_out_home_cta_starts_recipe_creation_through_login() {
    let cta = home_cta(false);
    let home_source = include_str!("home.rs");

    assert_eq!(cta.label, "Start creating");
    assert_eq!(cta.destination, HomeCtaDestination::LoginThenCreateRecipe);
    assert!(
        home_source
            .matches("/login?redirect_uri=/recipes/new")
            .count()
            >= 2
    );
}

#[test]
fn signed_in_home_cta_opens_dashboard() {
    let cta = home_cta(true);

    assert_eq!(cta.label, "Open dashboard");
    assert_eq!(cta.destination, HomeCtaDestination::Dashboard);
}

#[test]
fn about_story_covers_the_six_approved_beats() {
    let ids = ABOUT_SECTIONS
        .iter()
        .map(|section| section.id)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        ["capture", "organize", "evolve", "cook", "share", "build"]
    );
}

#[test]
fn about_cozy_layout_preserves_navigation_and_recipe_creation_flow() {
    let page = include_str!("about.rs");
    let css = include_str!("../../assets/main.css");

    assert!(page.contains("class: \"about-hero__illustration\""));
    assert!(page.contains("class: \"about-story__section\""));
    assert!(page.contains("href: \"/login?redirect_uri=/recipes/new\""));
    assert!(page.contains("to: Route::RecipeNew {}"));
    assert!(page.contains("for section in ABOUT_SECTIONS"));
    assert!(page.contains("class: \"about-story__versions\""));
    assert!(page.contains("class: \"about-story__visibility\""));
    assert!(css.contains(".about-story {\n    display: grid;"));
    assert!(css.contains(".about-story__section {\n    position: relative;"));
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
