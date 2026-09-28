use dioxus::prelude::*;

use crate::api::{
    collection::list_root_collections, recipe::list_recipe_drafts, search::search_recipes,
};
use crate::auth::context::use_auth;
use crate::components::base::{BrandMark, CollectionAlbum, PageIcon, PageIconKind, RecipeCard};
use crate::components::AuthRequired;
use crate::types::{
    CollectionSummary, RecipeDraftView, RecipeSearchRequest, RecipeSearchResponse,
    RecipeSearchSort, RecipeSearchVisibility,
};
use crate::Route;

const PERSONAL_HOME_CSS: Asset = asset!("/assets/personal-home.css");

/// Keep the Home preview owner-only and sort the entire library before limiting it.
fn recent_recipe_request() -> RecipeSearchRequest {
    RecipeSearchRequest {
        visibility: RecipeSearchVisibility::MineOnly,
        sort: RecipeSearchSort::UpdatedNewest,
        page_size: 6,
        ..Default::default()
    }
}

#[derive(Clone, PartialEq)]
enum HomeLoadState<T> {
    Pending,
    Ready(T),
    Error,
}

impl<T> HomeLoadState<T> {
    fn from_result(result: Option<Result<T, ServerFnError>>, pending: bool) -> Self {
        if pending {
            return Self::Pending;
        }
        match result {
            Some(Ok(items)) => Self::Ready(items),
            Some(Err(_)) => Self::Error,
            None => Self::Pending,
        }
    }
}

/// The marketing Home at `/` remains separate from this authenticated entry point.
#[component]
pub fn PersonalHome() -> Element {
    rsx! {
        document::Stylesheet { href: PERSONAL_HOME_CSS }
        document::Title { "Home — Noms" }
        AuthRequired { PersonalHomeWorkspace {} }
    }
}

#[component]
fn PersonalHomeWorkspace() -> Element {
    let auth = use_auth();
    let name = auth
        .current_user
        .as_ref()
        .map(|user| {
            if user.display_name.trim().is_empty() {
                user.username.clone()
            } else {
                user.display_name.trim().to_string()
            }
        })
        .unwrap_or_else(|| "cook".to_string());
    // Resources live inside AuthRequired, so guests never initiate private reads.
    let mut recipes = use_resource(|| search_recipes(recent_recipe_request()));
    let mut drafts = use_resource(list_recipe_drafts);
    let mut collections = use_resource(list_root_collections);
    let recipe_state = HomeLoadState::from_result(recipes.read().clone(), recipes.pending());
    let draft_state = HomeLoadState::from_result(drafts.read().clone(), drafts.pending());
    let collection_state =
        HomeLoadState::from_result(collections.read().clone(), collections.pending());

    rsx! {
        PersonalHomeContent {
            name, recipes: recipe_state, drafts: draft_state, collections: collection_state,
            on_retry_recipes: move |_| recipes.restart(),
            on_retry_drafts: move |_| drafts.restart(),
            on_retry_collections: move |_| collections.restart(),
        }
    }
}

#[component]
fn PersonalHomeContent(
    name: String,
    recipes: HomeLoadState<RecipeSearchResponse>,
    drafts: HomeLoadState<Vec<RecipeDraftView>>,
    collections: HomeLoadState<Vec<CollectionSummary>>,
    #[props(default)] on_retry_recipes: EventHandler<()>,
    #[props(default)] on_retry_drafts: EventHandler<()>,
    #[props(default)] on_retry_collections: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "container personal-home",
            header { class: "personal-home-heading",
                div {
                    p { class: "personal-home-welcome", "Welcome back, {name}" }
                    h1 { class: "brand-title", BrandMark {} span { "Your kitchen" } }
                    p { class: "personal-home-muted", "Pick up where you left off, or find something to cook." }
                }
                Link { to: Route::Dashboard {}, class: "btn btn-secondary touch-target", "Open dashboard →" }
            }
            section { class: "personal-home-start neumo-card", aria_label: "Find or create a recipe",
                form { class: "personal-home-search", action: "/search", method: "get",
                    label { r#for: "home-recipe-search", "Find a recipe in your library" }
                    div { class: "personal-home-search-controls",
                        input { id: "home-recipe-search", class: "input input--shared", r#type: "search", name: "q", placeholder: "Search your recipes…", maxlength: "256" }
                        input { r#type: "hidden", name: "scope", value: "recipes" }
                        input { r#type: "hidden", name: "vis", value: "mine" }
                        button { r#type: "submit", class: "btn btn-primary touch-target", "Search" }
                    }
                }
                nav { class: "personal-home-quick-actions", aria_label: "Quick actions",
                    Link { to: Route::RecipeNew {}, class: "btn btn-primary touch-target", "+ New recipe" }
                    Link { to: Route::CollectionNew { parent: None }, class: "btn btn-secondary touch-target", "+ New collection" }
                }
            }
            div { class: "personal-home-layout",
                section { class: "personal-home-section neumo-card", aria_labelledby: "home-recent-title",
                    div { class: "personal-home-section-heading",
                        h2 { id: "home-recent-title", "Recently updated" }
                        Link { to: Route::Search { query: "scope=recipes&vis=mine&sort=updated_newest".into() }, class: "personal-home-text-link", "View all recipes →" }
                    }
                    p { class: "personal-home-muted", "The latest changes in your saved recipe library." }
                    match &recipes {
                        HomeLoadState::Pending => rsx! { p { class: "personal-home-state", role: "status", "Loading recipes…" } },
                        HomeLoadState::Error => rsx! { HomeReadError { label: "recipes", on_retry: on_retry_recipes } },
                        HomeLoadState::Ready(response) if response.results.is_empty() => rsx! {
                            div { class: "personal-home-state",
                                h3 { "No saved recipes yet" }
                                p { "Create your first recipe, or explore public recipes to find a starting point." }
                                Link { to: Route::RecipeNew {}, class: "btn btn-secondary touch-target", "Create a recipe" }
                            }
                        },
                        HomeLoadState::Ready(response) => rsx! {
                            div { class: "personal-home-recipe-grid",
                                for result in response.results.iter().take(6) {
                                    RecipeCard { key: "{result.recipe.id}", recipe: result.recipe.clone(), framed: true }
                                }
                            }
                        },
                    }
                }
                aside { class: "personal-home-side", aria_label: "Continue and discover",
                    section { class: "personal-home-section neumo-card", aria_labelledby: "home-drafts-title",
                        h2 { id: "home-drafts-title", class: "brand-title", PageIcon { kind: PageIconKind::Editor, compact: true } span { "Continue editing" } }
                        match &drafts {
                            HomeLoadState::Pending => rsx! { p { class: "personal-home-state", role: "status", "Loading drafts…" } },
                            HomeLoadState::Error => rsx! { HomeReadError { label: "drafts", on_retry: on_retry_drafts } },
                            HomeLoadState::Ready(items) if items.is_empty() => rsx! {
                                div { class: "personal-home-state", h3 { "No drafts in progress" } p { "Your private drafts will be ready here when you return." } }
                            },
                            HomeLoadState::Ready(items) => rsx! {
                                div { class: "personal-home-drafts",
                                    for draft in items.iter().take(3) {
                                        Link { key: "{draft.id}", to: Route::RecipeDraftEdit { id: draft.id.to_string() }, class: "personal-home-draft neumo-inset",
                                            strong { if draft.editor.title.trim().is_empty() { "Untitled recipe" } else { "{draft.editor.title}" } }
                                            span { "Private draft · Revision {draft.revision}" }
                                            time { datetime: draft.updated_at.to_rfc3339(), {draft.updated_at.format("Updated %d %b").to_string()} }
                                        }
                                    }
                                }
                            },
                        }
                        Link { to: "/dashboard#dashboard-drafts", class: "personal-home-text-link", "All drafts →" }
                    }
                    section { class: "personal-home-discover neumo-card", aria_labelledby: "home-explore-title",
                        PageIcon { kind: PageIconKind::Explore, compact: true }
                        h2 { id: "home-explore-title", "Find something new" }
                        p { class: "personal-home-muted", "Browse recipes shared by the community." }
                        Link { to: Route::Explore {}, class: "btn btn-secondary touch-target", "Explore recipes →" }
                    }
                }
                section { class: "personal-home-section personal-home-collections neumo-card", aria_labelledby: "home-collections-title",
                    div { class: "personal-home-section-heading",
                        h2 { id: "home-collections-title", class: "brand-title", PageIcon { kind: PageIconKind::Collections, compact: true } span { "Your collections" } }
                        Link { to: Route::CollectionList {}, class: "personal-home-text-link", "View all collections →" }
                    }
                    match &collections {
                        HomeLoadState::Pending => rsx! { p { class: "personal-home-state", role: "status", "Loading collections…" } },
                        HomeLoadState::Error => rsx! { HomeReadError { label: "collections", on_retry: on_retry_collections } },
                        HomeLoadState::Ready(items) if items.is_empty() => rsx! {
                            div { class: "personal-home-state", h3 { "No collections yet" } p { "Give your recipes a place with your first collection." }
                                Link { to: Route::CollectionNew { parent: None }, class: "btn btn-secondary touch-target", "Create a collection" }
                            }
                        },
                        HomeLoadState::Ready(items) => rsx! {
                            p { class: "personal-home-muted", "A few of your top-level collections, in your saved order." }
                            div { class: "personal-home-collection-grid",
                                for collection in items.iter().take(3) {
                                    CollectionAlbum { key: "{collection.id}", collection: collection.clone(), compact: true }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn HomeReadError(label: &'static str, on_retry: EventHandler<()>) -> Element {
    rsx! {
        div { class: "personal-home-state", role: "alert",
            p { "Your {label} couldn’t be loaded. Please try again." }
            button { class: "btn btn-secondary touch-target", r#type: "button", onclick: move |_| on_retry.call(()), "Retry {label}" }
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
        let state = use_context::<HomeLoadState<Vec<RecipeDraftView>>>();
        rsx! {
            PersonalHomeContent {
                name: "Cook <test>".to_string(),
                recipes: HomeLoadState::Ready(RecipeSearchResponse {
                    results: vec![], total_count: 0,
                    facets: crate::types::RecipeSearchFacets { tags: vec![], visibility: vec![], total_time: vec![], servings: vec![] },
                }),
                drafts: state,
                collections: HomeLoadState::Ready(vec![]),
            }
        }
    }

    #[component]
    fn TestRoot(state: HomeLoadState<Vec<RecipeDraftView>>) -> Element {
        use_context_provider(|| state);
        rsx! {
            HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<HomeTestRoute> {}
            }
        }
    }

    fn render(state: HomeLoadState<Vec<RecipeDraftView>>) -> String {
        let mut dom = VirtualDom::new_with_props(TestRoot, TestRootProps { state });
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn recently_updated_request_is_owner_scoped_sorted_and_bounded() {
        let request = recent_recipe_request();
        assert_eq!(request.visibility, RecipeSearchVisibility::MineOnly);
        assert_eq!(request.sort, RecipeSearchSort::UpdatedNewest);
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 6);
    }

    #[test]
    fn empty_home_has_real_navigation_search_and_no_marketing_or_management_controls() {
        let html = render(HomeLoadState::Ready(vec![]));
        for content in [
            "Recently updated",
            "Continue editing",
            "Your collections",
            "No saved recipes yet",
            "No drafts in progress",
            "No collections yet",
            "action=\"/search\"",
            "name=\"vis\" value=\"mine\"",
            "href=\"/dashboard\"",
            "href=\"/recipes/new\"",
            "href=\"/explore\"",
        ] {
            assert!(html.contains(content), "missing {content}: {html}");
        }
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(!html.contains("<table"));
        assert!(!html.contains("A cozy home for your recipes."));
        assert!(!html.contains("<test>"), "names must be escaped");
    }

    #[test]
    fn drafts_have_independent_loading_error_and_untitled_states() {
        assert!(render(HomeLoadState::Pending).contains("Loading drafts"));
        let failed = render(HomeLoadState::Error);
        assert!(failed.contains("Retry drafts"));
        assert!(failed.contains("role=\"alert\""));
        let draft = RecipeDraftView {
            id: uuid::Uuid::nil(),
            recipe_id: None,
            base_version_number: None,
            revision: 1,
            editor: crate::types::RecipeEditorState::default(),
            forked_from_recipe_id: None,
            forked_from_version_number: None,
            forked_from_title: None,
            fork_message: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let populated = render(HomeLoadState::Ready(vec![draft; 5]));
        assert_eq!(populated.matches("Untitled recipe").count(), 3);
        assert!(populated.contains("/recipes/drafts/00000000-0000-0000-0000-000000000000/edit"));
        assert!(populated.contains("Private draft"));
    }
}
