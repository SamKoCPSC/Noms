use dioxus::prelude::*;

use crate::api::recipe::get_public_recipes;
use crate::components::base::{BrandMark, PageIcon, PageIconKind, RecipeCard};
use crate::components::AuthRequired;
use crate::types::{PublicRecipeListResponse, RecipeCardSummary};
use crate::Route;

const COMMUNITY_HOME_CSS: Asset = asset!("/assets/community-home.css");
const COMMUNITY_RECIPE_LIMIT: i64 = 6;

#[derive(Clone, PartialEq)]
enum CommunityRecipeState {
    Pending,
    Ready(Vec<RecipeCardSummary>),
    Error,
}

impl CommunityRecipeState {
    fn from_result(
        result: Option<Result<PublicRecipeListResponse, ServerFnError>>,
        pending: bool,
    ) -> Self {
        if pending {
            return Self::Pending;
        }
        match result {
            Some(Ok(response)) => {
                // Public discovery enforces visibility on the server. Keep this
                // presentation boundary fail-closed if its data source changes.
                Self::Ready(
                    response
                        .recipes
                        .into_iter()
                        .filter(|recipe| recipe.visibility == "public")
                        .take(COMMUNITY_RECIPE_LIMIT as usize)
                        .collect(),
                )
            }
            Some(Err(_)) => Self::Error,
            None => Self::Pending,
        }
    }
}

/// Community discovery and editorial notices, not the personal Dashboard.
#[component]
pub fn CommunityHome() -> Element {
    rsx! {
        document::Stylesheet { href: COMMUNITY_HOME_CSS }
        AuthRequired { CommunityHomeFeed {} }
    }
}

#[component]
fn CommunityHomeFeed() -> Element {
    // Reuse Explore's bounded, public-only source. No drafts, owned collections,
    // personal search or invented popularity/engagement data belongs on Home.
    let mut recipes = use_resource(|| get_public_recipes(0, COMMUNITY_RECIPE_LIMIT, String::new()));
    let state = CommunityRecipeState::from_result(recipes.read().clone(), recipes.pending());
    rsx! {
        CommunityHomeContent { recipes: state, on_retry: move |_| recipes.restart() }
    }
}

#[component]
fn CommunityHomeContent(
    recipes: CommunityRecipeState,
    #[props(default)] on_retry: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "container community-home",
            header { class: "community-home-heading",
                div {
                    p { class: "community-home-eyebrow", "THE NOMS COMMUNITY" }
                    h1 { class: "brand-title", BrandMark {} span { "Community Home" } }
                    p { class: "community-home-muted", "Recipe discoveries, community updates, and ways to take part." }
                }
                Link { to: Route::Explore {}, class: "btn btn-primary touch-target", "Explore recipes →" }
            }
            div { class: "community-home-layout",
                section { class: "community-home-panel community-home-spotlight neumo-card", aria_labelledby: "home-spotlight-title",
                    div { class: "community-home-section-heading",
                        h2 { id: "home-spotlight-title", class: "brand-title", PageIcon { kind: PageIconKind::Explore, compact: true } span { "Community spotlight" } }
                    }
                    p { class: "community-home-muted", "A closer look at the newest public recipe on Noms." }
                    match &recipes {
                        CommunityRecipeState::Pending => rsx! {
                            p { class: "community-home-state", role: "status", "Loading community recipes…" }
                        },
                        CommunityRecipeState::Error => rsx! {
                            div { class: "community-home-state", role: "alert",
                                p { "Community recipes couldn’t be loaded. Please try again." }
                                button { class: "btn btn-secondary touch-target", r#type: "button", onclick: move |_| on_retry.call(()), "Retry community recipes" }
                            }
                        },
                        CommunityRecipeState::Ready(items) => {
                            if let Some(recipe) = items.first() {
                                rsx! { RecipeCard { key: "{recipe.id}", recipe: recipe.clone(), framed: true } }
                            } else {
                                rsx! {
                                    div { class: "community-home-state",
                                        h3 { "No community recipes yet" }
                                        p { "Public recipes will appear here as cooks share them. Private and unlisted recipes stay out of this feed." }
                                        Link { to: Route::RecipeNew {}, class: "btn btn-secondary touch-target", "Start a recipe" }
                                    }
                                }
                            }
                        },
                    }
                }
                aside { class: "community-home-side", aria_label: "Community news and participation",
                    section { class: "community-home-panel neumo-card", aria_labelledby: "home-announcements-title",
                        h2 { id: "home-announcements-title", "Announcements" }
                        // This is an authored product notice, not a simulated
                        // activity feed or an announcement-management feature.
                        article { class: "community-home-notice",
                            div { class: "community-home-notice-meta",
                                span { class: "community-home-eyebrow", "NOMS UPDATE" }
                                time { datetime: "2026-09-28", "28 Sep 2026" }
                            }
                            h3 { "A home for the community" }
                            p { class: "community-home-muted", "Home now brings public recipes and community news together. Your recipes, drafts and collections are still in Dashboard." }
                            Link { to: Route::Dashboard {}, class: "community-home-text-link", "Open Dashboard →" }
                        }
                    }
                    section { class: "community-home-panel neumo-card", aria_labelledby: "home-participation-title",
                        h2 { id: "home-participation-title", class: "brand-title", PageIcon { kind: PageIconKind::Profile, compact: true } span { "Get involved" } }
                        nav { class: "community-home-participation", aria_label: "Community participation",
                            Link { to: Route::Search { query: "scope=users".into() }, class: "community-home-action neumo-inset",
                                strong { "Meet the cooks →" }
                                span { "Find public profiles and the recipes people have shared." }
                            }
                            Link { to: Route::RecipeNew {}, class: "community-home-action neumo-inset",
                                strong { "Share a recipe →" }
                                span { "Start a private draft, then publish it as public when you’re ready." }
                            }
                            Link { to: "/how-to#fork", class: "community-home-action neumo-inset",
                                strong { "Try your own variation →" }
                                span { "Learn how to fork a recipe while keeping its source attached." }
                            }
                        }
                    }
                }
                if let CommunityRecipeState::Ready(items) = &recipes {
                    if items.len() > 1 {
                        section { class: "community-home-panel community-home-recipes neumo-card", aria_labelledby: "home-community-recipes-title",
                            div { class: "community-home-section-heading",
                                h2 { id: "home-community-recipes-title", "More from the community" }
                                Link { to: Route::Explore {}, class: "community-home-text-link", "Browse all recipes →" }
                            }
                            p { class: "community-home-muted", "More recent public recipes, with the cooks behind them." }
                            div { class: "community-home-recipe-grid",
                                for recipe in items.iter().skip(1) {
                                    RecipeCard { key: "{recipe.id}", recipe: recipe.clone(), framed: true }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::router::components::HistoryProvider;

    #[derive(Clone, PartialEq, Routable)]
    enum HomeTestRoute {
        #[route("/")]
        TestHome {},
    }

    #[component]
    fn TestHome() -> Element {
        let state = use_context::<CommunityRecipeState>();
        rsx! { CommunityHomeContent { recipes: state } }
    }

    #[component]
    fn TestRoot(state: CommunityRecipeState) -> Element {
        use_context_provider(|| state);
        rsx! {
            HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<HomeTestRoute> {}
            }
        }
    }

    fn render(state: CommunityRecipeState) -> String {
        let mut dom = VirtualDom::new_with_props(TestRoot, TestRootProps { state });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    fn recipe(index: u128, visibility: &str) -> RecipeCardSummary {
        RecipeCardSummary {
            id: uuid::Uuid::from_u128(index),
            title: format!("Community recipe {index}"),
            description: Some("Soup <script> & herbs".into()),
            prep_time_minutes: Some(10),
            cook_time_minutes: Some(20),
            servings: Some(4),
            image: None,
            visibility: visibility.into(),
            author_username: format!("cook-{index}"),
            author_avatar_url: None,
        }
    }

    fn response(recipes: Vec<RecipeCardSummary>) -> PublicRecipeListResponse {
        PublicRecipeListResponse {
            total_count: recipes.len() as i64,
            recipes,
            has_more: false,
        }
    }

    #[test]
    fn home_is_a_community_hub_not_a_personal_workspace() {
        let html = render(CommunityRecipeState::Ready(vec![]));
        for content in [
            "Community Home",
            "Announcements",
            "Get involved",
            "Meet the cooks",
        ] {
            assert!(html.contains(content), "missing {content}: {html}");
        }
        for personal_content in [
            "Continue editing",
            "Your collections",
            "Recently updated",
            "Welcome back",
            "vis=mine",
        ] {
            assert!(
                !html.contains(personal_content),
                "personal content remains: {personal_content}"
            );
        }
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(!html.contains("<table"));
        assert!(!html.contains("<form"));
        for destination in [
            "href=\"/explore\"",
            "href=\"/search?scope=users\"",
            "href=\"/recipes/new\"",
            "href=\"/dashboard\"",
            "href=\"/how-to#fork\"",
        ] {
            assert!(html.contains(destination), "missing {destination}");
        }
        for unsupported_claim in ["Trending", "likes", "comments", "challenge", "Leaderboard"] {
            assert!(!html.contains(unsupported_claim));
        }
    }

    #[test]
    fn community_feed_only_uses_public_recipes_in_source_order_with_a_bounded_preview() {
        let mut items = vec![recipe(100, "private"), recipe(101, "unlisted")];
        items.extend((1..=8).map(|index| recipe(index, "public")));
        let CommunityRecipeState::Ready(cards) =
            CommunityRecipeState::from_result(Some(Ok(response(items))), false)
        else {
            panic!("expected public cards")
        };
        assert_eq!(
            cards
                .iter()
                .map(|card| card.id.as_u128())
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
        let source = include_str!("community_home.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(source.contains("get_public_recipes(0, COMMUNITY_RECIPE_LIMIT, String::new())"));
        for private_read in [
            "list_recipe_drafts",
            "list_root_collections",
            "search_recipes",
            "current_user",
        ] {
            assert!(!source.contains(private_read));
        }
    }

    #[test]
    fn spotlight_and_feed_link_real_recipes_and_creators_without_duplicates() {
        let html = render(CommunityRecipeState::Ready(vec![
            recipe(1, "public"),
            recipe(2, "public"),
        ]));
        assert!(html.contains("Community spotlight"));
        assert!(html.contains("newest public recipe"));
        assert!(html.contains("More from the community"));
        for index in [1, 2] {
            assert_eq!(
                html.matches(&format!("Community recipe {index}")).count(),
                1
            );
            assert!(html.contains(&format!("/recipes/{}", uuid::Uuid::from_u128(index))));
            assert!(html.contains(&format!("/u/cook-{index}")));
        }
        assert!(!html.contains("<script>"), "metadata must be escaped");
        let single = render(CommunityRecipeState::Ready(vec![recipe(1, "public")]));
        assert!(!single.contains("More from the community"));
    }

    #[test]
    fn news_and_participation_remain_available_during_loading_empty_and_error_states() {
        for state in [
            CommunityRecipeState::Pending,
            CommunityRecipeState::Error,
            CommunityRecipeState::Ready(vec![]),
        ] {
            let html = render(state);
            assert!(html.contains("A home for the community"));
            assert!(html.contains("Get involved"));
        }
        let loading = render(CommunityRecipeState::Pending);
        assert!(loading.contains("Loading community recipes"));
        assert!(loading.contains("role=\"status\""));
        let failed = render(CommunityRecipeState::Error);
        assert!(failed.contains("Retry community recipes"));
        assert!(failed.contains("role=\"alert\""));
        assert!(render(CommunityRecipeState::Ready(vec![])).contains("No community recipes yet"));
    }

    #[test]
    fn pending_retries_hide_previous_results_and_failure_details_are_not_displayed() {
        assert!(matches!(
            CommunityRecipeState::from_result(Some(Ok(response(vec![recipe(1, "public")]))), true),
            CommunityRecipeState::Pending
        ));
        assert!(matches!(
            CommunityRecipeState::from_result(None, false),
            CommunityRecipeState::Pending
        ));
        let failed = CommunityRecipeState::from_result(
            Some(Err(ServerFnError::new("private backend details"))),
            false,
        );
        assert!(matches!(failed, CommunityRecipeState::Error));
        assert!(!render(failed).contains("private backend details"));
    }
}
