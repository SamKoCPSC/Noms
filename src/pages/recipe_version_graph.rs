//! Dedicated shell for the recipe version graph.
//!
//! This initial page loads only the selected recipe. Graph data and layout are
//! intentionally kept separate from the shell.

use dioxus::prelude::*;

use crate::api::recipe::{get_public_recipe, get_recipe};
use crate::components::base::{Card, EmptyState, LoadingSpinner, PageHeader};
use crate::types::Recipe;
use crate::Route;

fn server_error_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message.clone(),
        _ => error.to_string(),
    }
}

fn is_recipe_unavailable_error(message: &str) -> bool {
    matches!(message, "Recipe not found" | "Not authenticated")
}

#[component]
pub fn RecipeVersionGraph(id: String) -> Element {
    let recipe_id_is_valid = uuid::Uuid::parse_str(&id).is_ok();
    let resource_recipe_id_is_valid = recipe_id_is_valid;
    let id_for_recipe = id.clone();
    let recipe_resource = use_resource(move || {
        let recipe_id = id_for_recipe.clone();
        async move {
            if !resource_recipe_id_is_valid {
                return Err(ServerFnError::new("Invalid recipe ID"));
            }

            if let Ok(recipe) = get_public_recipe(recipe_id.clone()).await {
                return Ok(recipe);
            }

            get_recipe(recipe_id).await
        }
    });

    let back_to_recipe = rsx! {
        Link {
            to: Route::RecipeDetail { id: id.clone() },
            class: "btn btn-secondary touch-target",
            "Back to recipe"
        }
    };
    // Subscribe to the completed value before an early loading return. Reading
    // only `pending()` can leave a client-side route in its initial state.
    let recipe_result: Option<Result<Recipe, ServerFnError>> = recipe_resource.read().clone();

    if !recipe_id_is_valid {
        return rsx! {
            div { class: "container",
                Card {
                    EmptyState {
                        icon: rsx! { span { aria_hidden: "true", "↩" } },
                        title: "Invalid recipe link",
                        description: "This version graph link does not contain a valid recipe ID.",
                        action: back_to_recipe,
                    }
                }
            }
        };
    }

    if recipe_resource.pending() {
        return rsx! {
            div { class: "container",
                PageHeader { title: "Recipe version graph" }
                Card {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "240px",
                        LoadingSpinner {}
                        p {
                            margin_top: "var(--space-md)",
                            color: "var(--text-secondary)",
                            "Loading version graph..."
                        }
                    }
                }
            }
        };
    }

    let Some(recipe_result) = recipe_result else {
        return rsx! {
            div { class: "container",
                Card {
                    EmptyState {
                        icon: rsx! { span { aria_hidden: "true", "○" } },
                        title: "No versions to display yet",
                        description: "Version graph data is not available for this recipe yet.",
                        action: back_to_recipe,
                    }
                }
            }
        };
    };

    let recipe = match recipe_result {
        Ok(recipe) => recipe,
        Err(error) => {
            let error_message = server_error_message(&error);
            let recipe_is_unavailable = is_recipe_unavailable_error(&error_message);
            let state_icon = if recipe_is_unavailable { "🔒" } else { "!" };
            let state_title = if recipe_is_unavailable {
                "Version graph unavailable"
            } else {
                "Unable to load version graph"
            };
            let state_description = if recipe_is_unavailable {
                "This recipe doesn't exist or you don't have access to it."
            } else {
                "We couldn't load the version graph. Please try again."
            };
            return rsx! {
                div { class: "container",
                    PageHeader { title: "Recipe version graph" }
                    Card {
                        EmptyState {
                            icon: rsx! { span { aria_hidden: "true", "{state_icon}" } },
                            title: state_title,
                            description: state_description,
                            action: back_to_recipe,
                        }
                    }
                }
            };
        }
    };

    rsx! {
        div { class: "container",
            PageHeader {
                title: "Version graph for {recipe.title}",
                action: Some(back_to_recipe),
            }
            Card {
                EmptyState {
                    icon: rsx! { span { aria_hidden: "true", "⑂" } },
                    title: "No versions to display yet",
                    description: "A visual history will appear here when graph data is available.",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_recipe_unavailable_error;

    #[test]
    fn graph_shell_renders_navigation_and_all_resource_states() {
        let source = include_str!("recipe_version_graph.rs");

        assert!(source.contains("Recipe version graph"));
        assert!(source.contains("Back to recipe"));
        assert!(source.contains("Loading version graph"));
        assert!(source.contains("No versions to display yet"));
        assert!(source.contains("doesn't exist or you don't have access"));
        assert!(source.contains("We couldn't load the version graph"));
        assert!(source.contains("Invalid recipe link"));
        assert!(source.contains("Route::RecipeDetail"));
        assert!(!source.contains(&["VersionGraph", "Node"].concat()));
        assert!(!source.contains(&["VersionGraph", "Edge"].concat()));
    }

    #[test]
    fn only_privacy_safe_lookup_failures_use_the_unavailable_state() {
        assert!(is_recipe_unavailable_error("Recipe not found"));
        assert!(is_recipe_unavailable_error("Not authenticated"));
        assert!(!is_recipe_unavailable_error("database connection failed"));
    }

    #[test]
    fn graph_shell_subscribes_to_the_recipe_result_before_returning_loading_ui() {
        let source = include_str!("recipe_version_graph.rs");
        let result_read = source.find("recipe_resource.read().clone()").unwrap();
        let loading_return = source.find("if recipe_resource.pending()").unwrap();

        assert!(result_read < loading_return);
    }
}
