//! Recipe card for the dashboard grid.
//!
//! Displays a vertical card with author avatar, 16:9 image placeholder,
//! title, description, and action buttons.

use crate::components::base::avatar::Avatar;
use crate::components::base::avatar::AvatarSize;
use crate::components::base::EditedRecipeImage;
use crate::types::Recipe;
use dioxus::prelude::*;

/// Props for the [`RecipeCard`] component.
#[derive(Props, Clone, PartialEq)]
pub struct RecipeCardProps {
    /// The recipe to display.
    pub recipe: Recipe,
}

/// A clickable card showing a recipe preview.
///
/// Clicking the card navigates to the recipe detail page.
#[component]
pub fn RecipeCard(props: RecipeCardProps) -> Element {
    let RecipeCardProps { recipe } = props;
    let id = recipe.id.to_string();

    // Track whether the image failed to load (for graceful fallback)
    let mut image_failed = use_signal(|| false);

    let time_str = match (&recipe.prep_time_minutes, &recipe.cook_time_minutes) {
        (Some(prep), Some(cook)) => format!("{}m", prep + cook),
        (Some(prep), None) => format!("{}m", prep),
        (None, Some(cook)) => format!("{}m", cook),
        (None, None) => String::new(),
    };

    let servings_str = recipe
        .servings
        .map(|s| format!("{} servings", s))
        .unwrap_or_default();

    let desc = recipe.description.as_deref().unwrap_or("No description");

    rsx! {
        div {
            class: "recipe-card",
            div {
                class: "recipe-card__author",
                Avatar {
                    src: recipe.author_avatar_url.clone(),
                    size: AvatarSize::Small,
                    username: recipe.author_username.clone(),
                }
                span {
                    class: "recipe-card__author-name",
                    "{recipe.author_username}"
                }
            }

            Link {
                to: crate::Route::RecipeDetail { id },
                class: "recipe-card__link",
                div {
                    class: "recipe-card__image",
                    if let Some(first_image) = recipe.images.first() {
                        if !image_failed() {
                            EditedRecipeImage {
                                image: first_image.clone(),
                                class: "recipe-card__image-img",
                                alt: recipe.title.clone(),
                                prefer_thumbnail: true,
                                on_error: move |_| {
                                    image_failed.set(true);
                                },
                            }
                        } else {
                            div {
                                class: "recipe-card__image-placeholder",
                                aria_hidden: true,
                            }
                        }
                    } else {
                        div {
                            class: "recipe-card__image-placeholder",
                            aria_hidden: true,
                        }
                    }
                }

                div {
                    class: "recipe-card__content",
                    h3 {
                        class: "recipe-card__title",
                        "{recipe.title}"
                    }
                    p {
                        class: "recipe-card__description",
                        "{desc}"
                    }
                    div {
                        class: "recipe-card__meta",
                        if !time_str.is_empty() {
                            span {
                                class: "recipe-card__meta-item",
                                "{time_str}"
                            }
                        }
                        if !servings_str.is_empty() {
                            span {
                                class: "recipe-card__meta-item",
                                "{servings_str}"
                            }
                        }
                    }
                }
            }

            div {
                class: "recipe-card__actions",
                button {
                    class: "recipe-card__action-btn",
                    disabled: true,
                    span {
                        class: "recipe-card__action-icon",
                        "★"
                    }
                }
                button {
                    class: "recipe-card__action-btn",
                    disabled: true,
                    span {
                        class: "recipe-card__action-icon",
                        "🔖"
                    }
                }
            }
        }
    }
}
