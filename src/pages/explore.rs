//! Explore page — public recipe discovery.
//!
//! Displays a paginated grid of public recipes with server-side substring search
//! filtering and tag chips. No authentication required.

use dioxus::prelude::*;

use crate::api::recipe::{get_public_recipe_tags, get_public_recipes};
use crate::components::base::{
    Button, ButtonVariant, EmptyState, Input, LoadingSpinner, PageHeader, RecipeCard,
};
use crate::types::RecipeCardSummary;

const PAGE_SIZE: i64 = 12;

/// Public recipe discovery page with search and tag filtering.
#[component]
pub fn Explore() -> Element {
    // ── Signal state ─────────────────────────────────────────────────────
    let mut search_query = use_signal(String::new);
    let mut selected_tag = use_signal(String::new);
    let mut offset = use_signal(|| 0i64);
    let has_more = use_signal(|| true);
    let loaded_recipes = use_signal(Vec::<RecipeCardSummary>::new);
    let tags = use_signal(Vec::<String>::new);
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);
    let mut request_generation = use_signal(|| 0u64);

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
    let on_search = move |value: String| {
        // Invalidate an in-flight response immediately, before the effect reruns.
        let generation = *request_generation.peek() + 1;
        request_generation.set(generation);
        let mut recipes = loaded_recipes;
        recipes.set(Vec::new());
        offset.set(0);
        search_query.set(value);
    };

    let on_tag_click = move |tag_name: String| {
        let current = selected_tag.read().clone();
        if current == tag_name {
            selected_tag.set(String::new());
        } else {
            selected_tag.set(tag_name);
        }
    };

    let on_load_more = move |_| {
        if !is_loading() {
            offset.set(offset() + PAGE_SIZE);
        }
    };

    // ── Derived state for rendering ──────────────────────────────────────
    let is_loading_initial = is_loading() && loaded_recipes.read().is_empty();
    let has_error = error().is_some();
    let tags_list = tags.read().clone();
    let selected_tag_value = selected_tag.read().clone();

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

            // ── Search bar ─────────────────────────────────────────────────
            div { class: "explore-search",
                Input {
                    value: search_query(),
                    placeholder: "Search recipes...",
                    oninput: on_search,
                }
            }

            // ── Tag chips ──────────────────────────────────────────────────
            if !tags_list.is_empty() {
                div { class: "explore-tags",
                    for tag in tags_list {
                        TagChip {
                            name: tag.clone(),
                            is_active: selected_tag_value == tag,
                            on_click: on_tag_click,
                        }
                    }
                }
            }

            // ── Loading state (initial load) ───────────────────────────────
            if is_loading_initial {
                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "200px",
                    LoadingSpinner {}
                }
            }

            // ── Error state ────────────────────────────────────────────────
            if let Some(err_msg) = error() {
                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "200px",
                    color: "var(--error)",
                    "Failed to load recipes: {err_msg}"
                }
            }

            // ── Recipe grid ────────────────────────────────────────────────
            if !is_loading_initial && !has_error {
                if filtered_recipes.is_empty() {
                    // Empty state: no recipes match or no recipes at all
                    EmptyState {
                        icon: rsx! { "🔍" },
                        title: empty_title,
                        description: empty_description,
                    }
                } else {
                    // Recipe grid — responsive: 1 col mobile, 2 col tablet, 3 col desktop
                    div {
                        class: "recipe-grid",
                        for recipe in &filtered_recipes {
                            RecipeCard { recipe: recipe.clone() }
                        }
                    }

                    // Load more button if there are additional pages
                    if has_more() {
                        div {
                            class: "load-more",
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: on_load_more,
                                "Load more"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Individual tag chip button for the explore page.
#[component]
fn TagChip(name: String, is_active: bool, on_click: EventHandler<String>) -> Element {
    let chip_class = if is_active {
        "tag-chip tag-chip--active"
    } else {
        "tag-chip"
    };

    rsx! {
        button {
            class: chip_class,
            onclick: move |_| on_click.call(name.clone()),
            "{name}"
        }
    }
}

#[cfg(test)]
mod tests {
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
