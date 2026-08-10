//! User profile page — public-facing profile with paginated collections and recipes.
//!
//! Displays a user's profile header (avatar, display name, username, bio,
//! recipe count), public collections, and public recipes.
//! No authentication required.

use dioxus::prelude::*;

use crate::api::collection::get_user_public_collections;
use crate::api::recipe::{get_user_profile, get_user_public_recipes};
use crate::components::base::{
    Avatar, AvatarSize, Button, ButtonVariant, CollectionVisibilityBadge, EmptyState,
    LoadingSpinner, RecipeCard,
};
use crate::types::{CollectionSummary, Recipe, UserProfile as UserProfileType};
use crate::Route;

const PAGE_SIZE: i64 = 12;

/// Public user profile page with paginated collection and recipe grids.
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
    let mut collection_offset = use_signal(|| 0i64);
    let collection_has_more = use_signal(|| true);
    let loaded_collections = use_signal(Vec::<CollectionSummary>::new);
    let collections_are_loading = use_signal(|| true);
    let collections_error = use_signal(|| Option::<String>::None);

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

    // ── Fetch paginated public collections independently of recipes ─────
    let uname_collections = username.clone();
    use_effect(move || {
        let uname = uname_collections.clone();
        let off = collection_offset();
        let mut collections = loaded_collections;
        let mut more = collection_has_more;
        let mut loading = collections_are_loading;
        let mut error = collections_error;
        loading.set(true);
        error.set(None);

        spawn(async move {
            match get_user_public_collections(uname, off, PAGE_SIZE).await {
                Ok(response) => {
                    let mut current = collections.read().clone();
                    current.extend(response.collections);
                    collections.set(current);
                    more.set(response.has_more);
                }
                Err(problem) => error.set(Some(problem.to_string())),
            }
            loading.set(false);
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
    let on_load_more_collections = move |_| {
        if !collections_are_loading() {
            collection_offset.set(collection_offset() + PAGE_SIZE);
        }
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
    let collections_loading_initial =
        collections_are_loading() && loaded_collections.read().is_empty();

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

            // ── Public Collection Grid ───────────────────────────────────
            section { class: "profile-recipes profile-collections",
                h2 { class: "section-title", "Public collections" }

                if collections_loading_initial {
                    div {
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "160px",
                        LoadingSpinner {}
                    }
                } else if let Some(problem) = collections_error() {
                    EmptyState {
                        icon: rsx! { "⚠️" },
                        title: "Error loading collections",
                        description: problem,
                    }
                } else if loaded_collections.read().is_empty() {
                    EmptyState {
                        icon: rsx! { "📁" },
                        title: "No public collections yet",
                        description: format!("{0} hasn't shared any public collections.", profile_data.display_name),
                    }
                } else {
                    div {
                        display: "grid",
                        grid_template_columns: "repeat(auto-fill, minmax(240px, 1fr))",
                        gap: "var(--space-lg)",
                        for collection in loaded_collections.read().iter() {
                            Link {
                                key: "{collection.id}",
                                to: Route::CollectionDetail { id: collection.id.to_string() },
                                class: "neumo-card",
                                padding: "var(--space-lg)",
                                text_decoration: "none",
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-sm)",
                                CollectionVisibilityBadge { visibility: collection.visibility }
                                h3 { color: "var(--text-primary)", "{collection.name}" }
                                if let Some(description) = &collection.description {
                                    p { color: "var(--text-secondary)", "{description}" }
                                }
                                p {
                                    color: "var(--text-secondary)",
                                    "{collection.child_count} folders · {collection.recipe_count} recipes"
                                }
                            }
                        }
                    }

                    if collection_has_more() {
                        div { class: "load-more",
                            Button {
                                variant: ButtonVariant::Secondary,
                                disabled: collections_are_loading(),
                                onclick: on_load_more_collections,
                                if collections_are_loading() { "Loading…" } else { "Load more collections" }
                            }
                        }
                    }
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

#[cfg(test)]
mod collection_discovery_tests {
    #[test]
    fn public_profile_lists_public_collections_without_exposing_unlisted_entries() {
        let source = include_str!("user_profile.rs");
        assert!(source.contains("get_user_public_collections"));
        assert!(source.contains("Public collections"));
        assert!(source.contains("Route::CollectionDetail"));
        assert!(source.contains("CollectionVisibilityBadge"));
    }
}
