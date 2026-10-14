//! Recipe card for the dashboard grid.
//!
//! Displays a vertical card with author avatar, 16:9 image placeholder,
//! title, description, and action buttons.

use crate::components::base::avatar::Avatar;
use crate::components::base::avatar::AvatarSize;
use crate::components::base::EditedRecipeImage;
use crate::types::RecipeCardSummary;
use dioxus::prelude::*;

/// Props for the [`RecipeCard`] component.
#[derive(Props, Clone, PartialEq)]
pub struct RecipeCardProps {
    /// The recipe to display.
    #[props(into)]
    pub recipe: RecipeCardSummary,
    /// Framed discovery layout; legacy consumers retain their existing presentation.
    #[props(default)]
    pub framed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::router::components::HistoryProvider;
    use dioxus_core::NoOpMutations;

    #[derive(Clone, PartialEq, Routable)]
    enum CardTestRoute {
        #[route("/")]
        CardTestPage {},
    }

    #[component]
    fn CardTestPage() -> Element {
        let props = use_context::<RecipeCardProps>();
        rsx! { RecipeCard { recipe: props.recipe, framed: props.framed } }
    }

    #[allow(non_snake_case)]
    fn CardTestRoot(props: RecipeCardProps) -> Element {
        use_context_provider(|| props);
        rsx! {
            HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
                Router::<CardTestRoute> {}
            }
        }
    }

    fn render_card(framed: bool) -> String {
        let recipe = RecipeCardSummary {
            id: uuid::Uuid::nil(),
            title: "Soup & herbs".into(),
            description: Some("A cozy supper.".into()),
            prep_time_minutes: Some(10),
            cook_time_minutes: Some(20),
            servings: Some(4),
            image: None,
            visibility: "public".into(),
            author_username: "test-cook".into(),
            author_avatar_url: None,
        };
        let mut dom = VirtualDom::new_with_props(CardTestRoot, RecipeCardProps { recipe, framed });
        dom.rebuild(&mut NoOpMutations);
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn framed_card_renders_real_metadata_and_separate_links_without_placeholder_actions() {
        let html = render_card(true);
        assert!(html.contains("recipe-card--framed"), "{html}");
        assert!(html.contains("No recipe photo"), "{html}");
        assert!(
            html.contains("30m") && html.contains("4 servings"),
            "{html}"
        );
        assert!(html.contains("@test-cook"), "{html}");
        assert_eq!(html.matches("<a ").count(), 2, "{html}");
        let recipe_link_end = html.find("</a>").unwrap();
        let author_link = html.find("class=\"recipe-card__author\"").unwrap();
        assert!(author_link > recipe_link_end, "{html}");
        assert!(!html.contains("<button"), "{html}");
    }

    #[test]
    fn legacy_card_keeps_its_original_structure() {
        let html = render_card(false);
        assert!(!html.contains("recipe-card--framed"), "{html}");
        assert!(html.contains("recipe-card__actions"), "{html}");
        assert_eq!(html.matches("<a ").count(), 1, "{html}");
    }
}

/// A clickable card showing a recipe preview.
///
/// Clicking the card navigates to the recipe detail page.
#[component]
pub fn RecipeCard(props: RecipeCardProps) -> Element {
    let RecipeCardProps { recipe, framed } = props;
    let id = recipe.id.to_string();

    // Track whether the image failed to load (for graceful fallback)
    let mut image_failed = use_signal(|| false);
    let mut avatar_failed = use_signal(|| false);

    let time_str = match (&recipe.prep_time_minutes, &recipe.cook_time_minutes) {
        (Some(prep), Some(cook)) => format!("{}m", i64::from(*prep) + i64::from(*cook)),
        (Some(prep), None) => format!("{}m", prep),
        (None, Some(cook)) => format!("{}m", cook),
        (None, None) => String::new(),
    };

    let servings_str = recipe
        .servings
        .map(|s| format!("{} servings", s))
        .unwrap_or_default();

    let desc = recipe.description.as_deref().unwrap_or("No description");

    if framed {
        return rsx! {
            article { class: "recipe-card recipe-card--framed",
                Link {
                    to: crate::Route::RecipeDetail { id },
                    class: "recipe-card__link",
                    div { class: "recipe-card__image",
                        if let Some(image) = recipe.image.as_ref().filter(|_| !image_failed()) {
                            EditedRecipeImage {
                                image: image.clone().into_recipe_image(),
                                class: "recipe-card__image-img",
                                alt: recipe.title.clone(),
                                prefer_thumbnail: true,
                                on_error: move |_| image_failed.set(true),
                            }
                        } else {
                            div { class: "recipe-card__image-placeholder", "No recipe photo" }
                        }
                    }
                    div { class: "recipe-card__content",
                        h3 { class: "recipe-card__title", "{recipe.title}" }
                        if !desc.trim().is_empty() {
                            p { class: "recipe-card__description", "{desc}" }
                        }
                        div { class: "recipe-card__meta",
                            if !time_str.is_empty() { span { "{time_str}" } }
                            if !servings_str.is_empty() { span { "{servings_str}" } }
                        }
                    }
                }
                Link {
                    class: "recipe-card__author",
                    to: crate::Route::UserProfile { username: recipe.author_username.clone() },
                    if let Some(url) = recipe.author_avatar_url.as_ref().filter(|_| !avatar_failed()) {
                        img {
                            class: "recipe-card__avatar", src: "{url}", alt: "",
                            onerror: move |_| avatar_failed.set(true),
                        }
                    } else {
                        Avatar { size: AvatarSize::Small, username: recipe.author_username.clone() }
                    }
                    span { class: "recipe-card__author-name", "@{recipe.author_username}" }
                }
            }
        };
    }

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
                    if let Some(first_image) = recipe.image.as_ref() {
                        if !image_failed() {
                            EditedRecipeImage {
                                image: first_image.clone().into_recipe_image(),
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
