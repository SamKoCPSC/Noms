use dioxus::prelude::*;

use crate::api::recipe::list_my_recipes;
use crate::components::base::{
    Button, ButtonVariant, EmptyState, LoadingSpinner, PageHeader, RecipeCard,
};
use crate::components::AuthRequired;
use crate::types::RecipeListResponse;
use crate::Route;

const PAGE_SIZE: i64 = 12;

/// User's recipe library — paginated grid of recipe cards.
#[component]
pub fn Dashboard() -> Element {
    let mut offset = use_signal(|| 0i64);

    // Fetch recipes for the current page.
    // The closure is FnMut — it reads the offset signal on each render,
    // so changing offset triggers a re-fetch.
    let recipes = use_resource(move || {
        let off = offset();
        async move { list_my_recipes(off, PAGE_SIZE).await }
    });

    // Extract resource state to avoid borrow issues in rsx!
    let pending = recipes.pending();
    let recipes_result: Option<Result<RecipeListResponse, ServerFnError>> = recipes.read().clone();

    // Derived state
    let response = recipes_result.as_ref().and_then(|r| r.as_ref().ok());
    let error = recipes_result.as_ref().and_then(|r| r.as_ref().err());
    let current_offset = offset();

    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "My Recipes",
                    action: rsx! {
                        Link {
                            to: Route::RecipeNew {},
                            class: "btn btn-primary touch-target",
                            "+ New Recipe"
                        }
                    },
                }

                // Loading state (first load only)
                if pending {
                    div {
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "200px",
                        LoadingSpinner {}
                    }
                }

                // Error state
                if let Some(err) = error {
                    div {
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "200px",
                        color: "var(--error)",
                        "Failed to load recipes: {err}"
                    }
                }

                // Success state
                if let Some(resp) = response {
                    if resp.recipes.is_empty() {
                        // No recipes at all — show empty state
                        EmptyState {
                            icon: rsx! { "📖" },
                            title: "No recipes yet",
                            description: "Start building your recipe library by adding your first recipe.",
                            action: rsx! {
                                Link {
                                    to: Route::RecipeNew {},
                                    class: "btn btn-primary touch-target",
                                    "Create your first recipe"
                                }
                            },
                        }
                    } else {
                        // Recipe grid — responsive: 1 col mobile, 2 col tablet, 3 col desktop
                        div {
                            class: "recipe-grid",
                            for recipe in &resp.recipes {
                                RecipeCard { recipe: recipe.clone() }
                            }
                        }

                        // Load more button if there are additional pages
                        if resp.has_more {
                            div {
                                class: "load-more",
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    onclick: move |_| {
                                        let next_offset = offset() + PAGE_SIZE;
                                        offset.set(next_offset);
                                    },
                                    "Load more"
                                }
                            }
                        }

                        // If we've loaded past the first page, show count
                        if current_offset > 0 {
                            p {
                                class: "recipe-count",
                                "Showing {resp.recipes.len()} of {resp.total_count} recipe(s)"
                            }
                        }
                    }
                }
            }
        }
    }
}
