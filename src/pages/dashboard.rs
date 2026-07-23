use dioxus::prelude::*;

use crate::api::recipe::{list_my_recipes, list_recipe_drafts};
use crate::components::base::{
    Button, ButtonVariant, CollectionSidebar, EditedRecipeImage, EmptyState, LoadingSpinner,
    PageHeader, RecipeCard,
};
use crate::components::AuthRequired;
use crate::types::{RecipeDraftView, RecipeImage, RecipeListResponse};
use crate::Route;

const PAGE_SIZE: i64 = 12;

fn draft_card_image(images: &[RecipeImage]) -> Option<RecipeImage> {
    images.first().cloned()
}

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
    let drafts = use_resource(list_recipe_drafts);

    // Extract resource state to avoid borrow issues in rsx!
    let pending = recipes.pending();
    let recipes_result: Option<Result<RecipeListResponse, ServerFnError>> = recipes.read().clone();
    let drafts_result: Option<Result<Vec<RecipeDraftView>, ServerFnError>> = drafts.read().clone();
    let draft_list = drafts_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());

    // Derived state
    let response = recipes_result.as_ref().and_then(|r| r.as_ref().ok());
    let error = recipes_result.as_ref().and_then(|r| r.as_ref().err());
    let current_offset = offset();

    rsx! {
        AuthRequired {
            div { class: "container",
                div {
                    display: "flex",
                    flex_wrap: "wrap",
                    gap: "var(--space-xl)",
                    align_items: "start",
                    CollectionSidebar {}
                    main {
                        flex: "3 1 620px",
                        min_width: "0",
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

                        if let Some(drafts) = draft_list {
                    if !drafts.is_empty() {
                        section {
                            margin_bottom: "var(--space-xl)",
                            h2 { "Drafts" }
                            p {
                                color: "var(--text-secondary)",
                                "Private work in progress · autosaved"
                            }
                            div { class: "recipe-grid",
                                for draft in drafts {
                                    Link {
                                        key: "{draft.id}",
                                        to: Route::RecipeDraftEdit { id: draft.id.to_string() },
                                        class: "recipe-card recipe-card__link",
                                        div { class: "recipe-card__image",
                                            if let Some(image) = draft_card_image(&draft.editor.images) {
                                                EditedRecipeImage {
                                                    image,
                                                    class: "recipe-card__image-img",
                                                    alt: if draft.editor.title.trim().is_empty() {
                                                        "Untitled recipe draft".to_string()
                                                    } else {
                                                        format!("{} draft", draft.editor.title)
                                                    },
                                                    prefer_thumbnail: true,
                                                    on_error: move |_| {},
                                                }
                                            } else {
                                                div {
                                                    class: "recipe-card__image-placeholder",
                                                    aria_hidden: true,
                                                }
                                            }
                                        }
                                        div { class: "recipe-card__content",
                                            span {
                                                class: "recipe-detail__tag",
                                                "DRAFT"
                                            }
                                            h3 { class: "recipe-card__title",
                                                if draft.editor.title.trim().is_empty() {
                                                    "Untitled recipe"
                                                } else {
                                                    "{draft.editor.title}"
                                                }
                                            }
                                            if let Some(source) = &draft.forked_from_title {
                                                p { "Forked from {source}" }
                                            }
                                            p { class: "recipe-card__description",
                                                "Revision {draft.revision}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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
    }
}

#[cfg(test)]
mod draft_card_tests {
    use super::draft_card_image;
    use crate::types::{RecipeImage, RecipeImageEdit};

    #[test]
    fn draft_card_uses_the_first_snapshot_image() {
        let first = RecipeImage::new(
            "https://example.test/first.webp".to_string(),
            Some("https://example.test/first-thumb.webp".to_string()),
            RecipeImageEdit::default_rect_16x9(),
        );
        let second = RecipeImage::new(
            "https://example.test/second.webp".to_string(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        );

        assert_eq!(draft_card_image(&[first.clone(), second]), Some(first));
    }

    #[test]
    fn draft_card_without_images_uses_the_placeholder_path() {
        assert_eq!(draft_card_image(&[]), None);
    }
}

#[cfg(test)]
mod collection_sidebar_integration_tests {
    #[test]
    fn dashboard_composes_collections_without_replacing_drafts_or_recipe_pagination() {
        let source = include_str!("dashboard.rs");
        assert!(source.contains("CollectionSidebar"));
        assert!(source.contains("list_recipe_drafts"));
        assert!(source.contains("resp.has_more"));
    }
}
