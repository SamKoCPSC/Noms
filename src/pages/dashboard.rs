use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::base::{Button, ButtonVariant, Card, EmptyState, PageHeader};
use crate::components::AuthRequired;
use crate::Route;

/// User's recipe library with draft filter and badges.
#[component]
pub fn Dashboard() -> Element {
    let recipes = use_signal(Vec::<RecipeSummary>::new);
    let draft_count = use_signal(|| 0i32);
    let mut show_drafts = use_signal(|| false);
    let is_loading = use_signal(|| true);

    // Fetch recipes on mount and when toggle changes.
    // Only subscribes to show_drafts — other signals are captured as mutable handles.
    // We do NOT read is_loading inside the spawned task to avoid creating a subscription
    // that would cause the effect to re-run when is_loading changes (infinite polling loop).
    use_effect(move || {
        let include_drafts = *show_drafts.read();
        let mut recs = recipes;
        let mut dc = draft_count;
        let mut loading = is_loading;

        spawn(async move {
            loading.set(true);
            let url = if include_drafts {
                "/api/recipes?include_drafts=true"
            } else {
                "/api/recipes"
            };

            let res = gloo_net::http::Request::get(url).send().await;
            match res {
                Ok(resp) if resp.ok() => {
                    let body = resp.text().await.unwrap_or_default();
                    if let Ok(list) = serde_json::from_str::<ListRecipesResponse>(&body) {
                        recs.set(list.recipes);
                        dc.set(list.draft_count);
                    }
                }
                Err(_) => {}
                _ => {}
            }
            loading.set(false);
        });
    });

    let current_recipes = recipes.read().clone();
    let current_draft_count = *draft_count.read();
    let current_show_drafts = *show_drafts.read();
    let current_loading = *is_loading.read();

    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
          title: format!("My Recipes{}", if current_show_drafts && current_draft_count > 0 {
                format!(" ({} drafts)", current_draft_count)
            } else {
                String::new()
            }),
                    action: rsx! {
                        Link {
                            to: Route::RecipeNew {},
                            class: "btn btn-primary touch-target",
                            "+ New Recipe"
                        }
                    },
                }

                // Draft toggle
                div {
                    display: "flex",
                    align_items: "center",
                    gap: "var(--space-sm)",
                    margin_bottom: "var(--space-md)",
                    font_size: "14px",
                    color: "var(--text-secondary)",
                    label {
                        cursor: "pointer",
                        onclick: move |_| {
                            let current = *show_drafts.read();
                            show_drafts.set(!current);
                        },
                        if current_show_drafts {
                            "☑"
                        } else {
                            "☐"
                        }
                        " Show drafts"
                    }
                }

                // Loading state
                if current_loading {
                    div {
                        display: "flex",
                        justify_content: "center",
                        padding: "var(--space-xl)",
                        "Loading recipes..."
                    }
                } else if current_recipes.is_empty() {
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
                    // Recipe grid
                    div {
                        class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                        for recipe in current_recipes {
                            RecipeCard { recipe: recipe.clone() }
                        }
                    }
                }
            }
        }
    }
}

/// Individual recipe card with optional DRAFT badge.
#[component]
fn RecipeCard(recipe: RecipeSummary) -> Element {
    rsx! {
        Link {
            to: Route::RecipeDetail { id: recipe.id },
            class: "recipe-card-link",
            Card {
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-sm)",
                    div {
                        display: "flex",
                        justify_content: "space-between",
                        align_items: "flex-start",
                        h3 {
                            font_size: "16px",
                            font_weight: "600",
                            color: "var(--text-primary)",
                            margin: "0",
                            {recipe.title}
                        }
                        if recipe.is_draft {
                            span {
                                class: "badge badge-warning",
                                "DRAFT"
                            }
                        }
                    }
                    if let Some(desc) = recipe.description {
                        p {
                            font_size: "14px",
                            color: "var(--text-secondary)",
                            margin: "0",
                            line_height: "1.4",
                            {desc}
                        }
                    }
                    div {
                        display: "flex",
                        gap: "var(--space-sm)",
                        margin_top: "auto",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |evt: MouseEvent| {
                                evt.stop_propagation();
                                let nav = dioxus::prelude::use_navigator();
                                nav.push(Route::RecipeEdit { id: recipe.id });
                            },
                            "Edit"
                        }
                    }
                }
            }
        }
    }
}

// ── API response types ──────────────────────────────────────────────────────

#[derive(Clone, PartialEq, serde::Deserialize)]
struct RecipeSummary {
    id: Uuid,
    title: String,
    is_draft: bool,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct ListRecipesResponse {
    recipes: Vec<RecipeSummary>,
    draft_count: i32,
}
