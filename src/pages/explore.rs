//! Explore page — public recipe discovery.
//!
//! Displays a paginated grid of public recipes with client-side search
//! filtering and tag chips. No authentication required.

use dioxus::prelude::*;

use crate::api::recipe::{get_public_recipe_tags, get_public_recipes};
use crate::components::base::{
    Button, ButtonVariant, EmptyState, Input, LoadingSpinner, PageHeader, RecipeCard,
};
use crate::types::Recipe;

const PAGE_SIZE: i64 = 12;

/// Public recipe discovery page with search and tag filtering.
#[component]
pub fn Explore() -> Element {
    // ── Signal state ─────────────────────────────────────────────────────
    let mut search_query = use_signal(String::new);
    let mut selected_tag = use_signal(String::new);
    let mut offset = use_signal(|| 0i64);
    let has_more = use_signal(|| true);
    let loaded_recipes = use_signal(Vec::<Recipe>::new);
    let tags = use_signal(Vec::<String>::new);
    let is_loading = use_signal(|| true);
    let error = use_signal(|| Option::<String>::None);

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

    // ── Fetch recipes on mount and when offset changes ────────────────────
    // use_resource tracks the offset dependency — re-fetches when offset changes.
    // We use a separate effect to accumulate results into loaded_recipes.
    let current_offset = offset();
    let recipes_resource = use_resource(move || {
        let off = current_offset;
        async move { get_public_recipes(off, PAGE_SIZE).await }
    });

    // Accumulate recipes from resource into loaded_recipes signal.
    // We spawn the accumulation to avoid a circular dependency: reading
    // loaded_recipes inside this effect would register it as a tracked
    // dependency, and writing to it would re-trigger the effect infinitely.
    use_effect(move || {
        let res = recipes_resource.read().clone();
        let mut more = has_more;
        let mut loading = is_loading;
        let mut err = error;

        match res {
            Some(Ok(resp)) => {
                let new_count = resp.recipes.len();
                // Accumulate in a spawn block to break the circular dependency
                {
                    let mut loaded = loaded_recipes;
                    spawn(async move {
                        loaded.with_mut(|current| {
                            current.extend(resp.recipes);
                        });
                    });
                }
                more.set(resp.has_more);
                if new_count == 0 && current_offset > 0 {
                    // No more recipes returned
                    more.set(false);
                }
            }
            Some(Err(e)) => {
                err.set(Some(e.to_string()));
            }
            None => {
                // Still loading
                return;
            }
        }
        loading.set(false);
    });

    // ── Client-side filtered recipes ─────────────────────────────────────
    let filtered_recipes = {
        let recipes = loaded_recipes.read().clone();
        let query = search_query.read().to_lowercase();

        recipes
            .into_iter()
            .filter(|r| {
                query.is_empty()
                    || r.title.to_lowercase().contains(&query)
                    || r.description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .collect::<Vec<_>>()
    };

    // ── Handlers ─────────────────────────────────────────────────────────
    let on_search = move |value: String| {
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
        offset.set(offset() + PAGE_SIZE);
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
