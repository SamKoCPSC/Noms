//! User profile page — public-facing profile with paginated recipes.
//!
//! Displays a user's profile header (avatar, display name, username, bio,
//! recipe count) and their public recipes in a paginated grid.
//! No authentication required.

use dioxus::prelude::*;

use crate::api::recipe::{get_user_profile, get_user_public_recipes};
use crate::components::base::{
    Avatar, AvatarSize, Button, ButtonVariant, EmptyState, LoadingSpinner, RecipeCard,
};
use crate::types::{Recipe, UserProfile as UserProfileType};

const PAGE_SIZE: i64 = 12;

/// Public user profile page with header and paginated recipe grid.
#[component]
pub fn UserProfile(username: String) -> Element {
    // ── Signal state ─────────────────────────────────────────────────────
    let profile = use_signal(|| Option::<UserProfileType>::None);
    let profile_error = use_signal(|| Option::<String>::None);
    let mut offset = use_signal(|| 0i64);
    let has_more = use_signal(|| true);
    let loaded_recipes = use_signal(Vec::<Recipe>::new);
    let is_loading = use_signal(|| true);
    let recipes_error = use_signal(|| Option::<String>::None);

    // ── Fetch profile (one-time) ─────────────────────────────────────────
    let uname_profile = username.clone();
    use_effect(move || {
        let uname = uname_profile.clone();
        let mut prof = profile;
        let mut err = profile_error;
        spawn(async move {
            match get_user_profile(uname).await {
                Ok(p) => {
                    prof.set(Some(p));
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                }
            }
        });
    });

    // ── Fetch paginated recipes on mount and when offset changes ──────────
    let uname_recipes = username.clone();
    use_effect(move || {
        let uname = uname_recipes.clone();
        let off = offset();
        let mut recipes = loaded_recipes;
        let mut more = has_more;
        let mut loading = is_loading;
        let mut err = recipes_error;

        spawn(async move {
            match get_user_public_recipes(uname, off, PAGE_SIZE).await {
                Ok(resp) => {
                    let new_count = resp.recipes.len();
                    let mut current = recipes.read().clone();
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

    // ── Load more handler ────────────────────────────────────────────────
    let on_load_more = move |_| {
        offset.set(offset() + PAGE_SIZE);
    };

    // ── Render: profile error ────────────────────────────────────────────
    if let Some(err_msg) = profile_error() {
        return rsx! {
            div { class: "container",
                EmptyState {
                    icon: rsx! { "⚠️" },
                    title: "Profile not found",
                    description: format!("Could not load profile: {err_msg}"),
                }
            }
        };
    }

    // ── Render: loading (profile not yet fetched) ────────────────────────
    if profile().is_none() {
        return rsx! {
            div { class: "container",
                div {
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "60vh",
                    LoadingSpinner {}
                }
            }
        };
    }

    let profile_data = profile().clone().unwrap();

    // Compute recipe count text (pluralization must be outside rsx!)
    let recipe_count_text = if profile_data.public_recipe_count == 1 {
        "1 recipe".to_string()
    } else {
        format!("{} recipes", profile_data.public_recipe_count)
    };

    // ── Render: full profile ─────────────────────────────────────────────
    let is_loading_initial = is_loading() && loaded_recipes.read().is_empty();
    let has_error = recipes_error().is_some();

    rsx! {
        div { class: "container profile-page",
            // ── Profile Header ────────────────────────────────────────────
            div { class: "profile-header",
                Avatar {
                    size: AvatarSize::Large,
                    src: profile_data.avatar_url.clone(),
                    username: profile_data.display_name.clone(),
                }
                h1 { class: "profile-name", "{profile_data.display_name}" }
                p { class: "profile-username", "@{profile_data.username}" }
                if let Some(ref bio) = profile_data.bio {
                    p { class: "profile-bio", "{bio}" }
                }
                p { class: "profile-stats",
                    "{recipe_count_text}"
                }
            }

            // ── Recipe Grid ───────────────────────────────────────────────
            div { class: "profile-recipes",
                h2 { class: "section-title", "Recipes" }

                // Loading state (initial)
                if is_loading_initial {
                    div {
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "200px",
                        LoadingSpinner {}
                    }
                }

                // Error state
                if let Some(ref err) = recipes_error() {
                    EmptyState {
                        icon: rsx! { "⚠️" },
                        title: "Error loading recipes",
                        description: err.clone(),
                    }
                } else if !is_loading_initial && !has_error {
                    if loaded_recipes.read().is_empty() {
                        // Empty state: no public recipes
                        EmptyState {
                            icon: rsx! { "🍳" },
                            title: "No public recipes yet",
                            description: format!("{0} hasn't shared any public recipes.", profile_data.display_name),
                        }
                    } else {
                        // Recipe grid
                        div { class: "recipe-grid",
                            for recipe in loaded_recipes.read().iter() {
                                RecipeCard { recipe: recipe.clone() }
                            }
                        }

                        // Load more button
                        if has_more() {
                            div { class: "load-more",
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
}
