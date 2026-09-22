//! Explore page — public recipe discovery.
//!
//! Displays a paginated grid of public recipes with server-side substring search
//! filtering and community tag labels. No authentication required.

use dioxus::prelude::*;

use crate::api::recipe::{get_public_recipe_tags, get_public_recipes};
use crate::components::base::{
    Button, ButtonVariant, EmptyState, LoadingSpinner, PageHeader, RecipeCard,
};
use crate::types::RecipeCardSummary;

const PAGE_SIZE: i64 = 12;

fn next_page_offset(offset: i64, loading: bool, has_more: bool, has_error: bool) -> Option<i64> {
    if loading || !has_more || has_error {
        None
    } else {
        offset.checked_add(PAGE_SIZE)
    }
}

/// Public recipe discovery page with server-side search.
#[component]
pub fn Explore() -> Element {
    // ── Signal state ─────────────────────────────────────────────────────
    let mut search_query = use_signal(String::new);
    let mut offset = use_signal(|| 0i64);
    let has_more = use_signal(|| true);
    let loaded_recipes = use_signal(Vec::<RecipeCardSummary>::new);
    let tags = use_signal(Vec::<String>::new);
    let mut is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);
    let mut request_generation = use_signal(|| 0u64);
    let mut retry_generation = use_signal(|| 0u64);

    // ── Fetch tags on mount ──────────────────────────────────────────────
    use_hook(move || {
        let mut tag_signal = tags;
        spawn(async move {
            match get_public_recipe_tags().await {
                Ok(tag_list) => tag_signal.set(tag_list),
                Err(e) => {
                    tracing::warn!(error = ?e, "Failed to fetch public tags");
                }
            }
        });
    });

    // Search the complete description on the server, not the bounded card excerpt.
    use_effect(move || {
        let off = offset();
        let query = search_query();
        let _retry = retry_generation();
        let generation = *request_generation.peek() + 1;
        request_generation.set(generation);
        let mut recipes = loaded_recipes;
        let mut more = has_more;
        let mut loading = is_loading;
        let mut err = error;
        loading.set(true);
        err.set(None);

        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            if !query.is_empty() {
                gloo_timers::future::TimeoutFuture::new(180).await;
            }
            if *request_generation.peek() != generation {
                return;
            }
            let response = get_public_recipes(off, PAGE_SIZE, query).await;
            // A previous search or page must never overwrite the newest result.
            if *request_generation.peek() != generation {
                return;
            }
            match response {
                Ok(resp) => {
                    let new_count = resp.recipes.len();
                    let mut current = if off == 0 {
                        Vec::new()
                    } else {
                        recipes.peek().clone()
                    };
                    current.extend(resp.recipes);
                    recipes.set(current);
                    more.set(resp.has_more);
                    if new_count == 0 && off > 0 {
                        more.set(false);
                    }
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                }
            }
            loading.set(false);
        });
    });

    let filtered_recipes = loaded_recipes.read().clone();

    // ── Handlers ─────────────────────────────────────────────────────────
    let mut on_search = move |value: String| {
        if value == *search_query.peek() {
            return;
        }
        // Invalidate an in-flight response immediately, before the effect reruns.
        let generation = *request_generation.peek() + 1;
        request_generation.set(generation);
        let mut recipes = loaded_recipes;
        recipes.set(Vec::new());
        is_loading.set(true);
        offset.set(0);
        search_query.set(value);
    };

    let on_load_more = move |_| {
        if let Some(next) = next_page_offset(offset(), is_loading(), has_more(), error().is_some())
        {
            // Claim the request synchronously so rapid clicks cannot skip a page.
            is_loading.set(true);
            offset.set(next);
        }
    };
    let on_retry = move |_| {
        if !is_loading() {
            is_loading.set(true);
            // Retry the failed offset, rather than moving past its results.
            let next = *retry_generation.peek() + 1;
            retry_generation.set(next);
        }
    };

    // ── Derived state for rendering ──────────────────────────────────────
    let is_loading_initial = is_loading() && loaded_recipes.read().is_empty();
    let has_error = error().is_some();
    let tags_list = tags.read().clone();

    // Empty state messages (computed outside rsx! for compatibility)
    let empty_title = if search_query.read().is_empty() {
        "No recipes yet".to_string()
    } else {
        "No recipes found".to_string()
    };
    let empty_description = if search_query.read().is_empty() {
        "Public recipes will appear here as users share them with the community.".to_string()
    } else {
        "Try a different search term.".to_string()
    };

    rsx! {
        div { class: "container explore-page",
            PageHeader {
                title: "Explore",
            }
            p { class: "explore-intro", "Public recipes from the community." }

            // ── Search bar ─────────────────────────────────────────────────
            div { class: "explore-search",
                label { r#for: "explore-query", "Search public recipes" }
                input {
                    id: "explore-query",
                    class: "neumo-inset input input--shared",
                    r#type: "search",
                    aria_label: "Search public recipes",
                    value: search_query(),
                    placeholder: "Search titles and descriptions…",
                    oninput: move |event| on_search(event.value()),
                }
            }

            // ── Tag chips ──────────────────────────────────────────────────
            if !tags_list.is_empty() {
                div { class: "explore-tags",
                    span { class: "explore-tags__label", "Community tags" }
                    for tag in tags_list {
                        span { key: "{tag}", class: "explore-tag", "{tag}" }
                    }
                }
            }

            // ── Loading state (initial load) ───────────────────────────────
            if is_loading_initial {
                div {
                    class: "explore-status",
                    role: "status",
                    LoadingSpinner {}
                    "Loading recipes…"
                }
            }

            // ── Error state ────────────────────────────────────────────────
            if has_error && !is_loading() {
                div {
                    class: "explore-status explore-status--error",
                    role: "alert",
                    p { "We couldn’t load recipes. Please try again." }
                    Button { variant: ButtonVariant::Secondary, onclick: on_retry, "Retry" }
                }
            }

            // ── Recipe grid ────────────────────────────────────────────────
            if !is_loading_initial {
                if filtered_recipes.is_empty() && !has_error {
                    // Empty state: no recipes match or no recipes at all
                    EmptyState {
                        icon: rsx! { "🔍" },
                        title: empty_title,
                        description: empty_description,
                    }
                } else if !filtered_recipes.is_empty() {
                    p { class: "explore-results", role: "status", "{filtered_recipes.len()} recipes loaded" }
                    div {
                        class: "explore-grid",
                        for recipe in &filtered_recipes {
                            RecipeCard { key: "{recipe.id}", recipe: recipe.clone(), framed: true }
                        }
                    }

                    // Load more button if there are additional pages
                    if has_more() && !has_error {
                        div {
                            class: "load-more",
                            Button {
                                variant: ButtonVariant::Secondary,
                                disabled: is_loading(),
                                onclick: on_load_more,
                                if is_loading() { "Loading recipes…" } else { "Load more" }
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

    #[test]
    fn pagination_advances_only_when_ready_and_never_skips_a_failed_page() {
        assert_eq!(next_page_offset(0, false, true, false), Some(12));
        assert_eq!(next_page_offset(12, false, true, false), Some(24));
        assert_eq!(next_page_offset(12, true, true, false), None);
        assert_eq!(next_page_offset(12, false, true, true), None);
        assert_eq!(next_page_offset(12, false, false, false), None);
        assert_eq!(next_page_offset(i64::MAX, false, true, false), None);
    }

    #[test]
    fn explore_uses_framed_cards_and_honest_accessible_states() {
        let source = include_str!("explore.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(source.contains("framed: true"));
        assert!(source.contains("key: \"{recipe.id}\""));
        assert!(source.contains("aria_label: \"Search public recipes\""));
        assert!(source.contains("retry_generation()"));
        assert!(source.contains("\"Retry\""));
        assert!(!source.contains("selected_tag"));
        assert!(!source.contains("TagChip"));
        assert!(source.contains("role: \"alert\""));
    }

    #[test]
    fn framed_cards_are_opt_in_with_container_driven_columns() {
        let card = include_str!("../components/base/recipe_card.rs");
        assert!(card.contains("pub framed: bool"));
        assert!(card.contains("if framed"));
        assert!(card.contains("Route::UserProfile"));
        let css = include_str!("../../assets/main.css");
        assert!(css.contains("repeat(auto-fill, minmax(min(100%, 260px), 1fr))"));
        assert!(css.contains(".recipe-card--framed"));
    }

    #[test]
    fn pagination_fetch_reads_the_current_offset_reactively() {
        let source = include_str!("explore.rs");
        let pagination_fetch = source
            .split("use_effect(move || {")
            .nth(1)
            .expect("pagination fetch section")
            .split("let filtered_recipes =")
            .next()
            .expect("pagination fetch body");

        assert!(pagination_fetch.contains("let off = offset();"));
        assert!(pagination_fetch.contains("let query = search_query();"));
        assert!(!pagination_fetch.contains("let current_offset = offset();"));
    }
}
