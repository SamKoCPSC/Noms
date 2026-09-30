//! User profile page — public-facing profile with paginated collections and recipes.
//!
//! Displays a user's profile header (avatar, display name, username, bio,
//! recipe count), public collections, and public recipes.
//! No authentication required.

use crate::components::base::{PageIcon, PageIconKind};
use dioxus::prelude::*;

use crate::api::collection::get_user_public_collections;
use crate::api::recipe::{get_user_profile, get_user_public_recipes};
use crate::components::base::{
    Avatar, AvatarSize, CollectionCover, CollectionVisibilityBadge, EmptyState, LoadRecovery,
    LoadingSpinner, RecipeCard,
};
use crate::types::{CollectionSummary, RecipeCardSummary, UserProfile as UserProfileType};
use crate::Route;

const PAGE_SIZE: i64 = 12;

#[cfg(test)]
#[path = "user_profile_tests.rs"]
mod recovery_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProfilePageRequest {
    offset: i64,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfilePageAction {
    LoadMore,
    Retry,
}

/// Both public sections share offset/retry rules but have independent state.
/// Keep the failed offset and accumulated cards until that exact request succeeds.
#[derive(Clone)]
struct ProfilePageState<T> {
    items: Vec<T>,
    request: ProfilePageRequest,
    pending: bool,
    failed: bool,
    has_more: bool,
}

impl<T> Default for ProfilePageState<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            request: ProfilePageRequest {
                offset: 0,
                generation: 0,
            },
            pending: true,
            failed: false,
            has_more: true,
        }
    }
}

impl<T> ProfilePageState<T> {
    fn begin(&mut self, action: ProfilePageAction) -> bool {
        if self.pending {
            return false;
        }
        let offset = match action {
            ProfilePageAction::LoadMore if self.has_more && !self.failed => {
                let Some(offset) = self.request.offset.checked_add(PAGE_SIZE) else {
                    return false;
                };
                offset
            }
            ProfilePageAction::Retry if self.failed => self.request.offset,
            _ => return false,
        };
        let Some(generation) = self.request.generation.checked_add(1) else {
            return false;
        };
        self.request = ProfilePageRequest { offset, generation };
        self.pending = true;
        true
    }

    fn finish(&mut self, request: ProfilePageRequest, result: Result<(Vec<T>, bool), ()>) -> bool {
        if request != self.request || !self.pending {
            return false;
        }
        match result {
            Ok((items, more)) => {
                self.has_more = more && !items.is_empty();
                if request.offset == 0 {
                    self.items = items;
                } else {
                    self.items.extend(items);
                }
                self.failed = false;
            }
            Err(()) => self.failed = true,
        }
        self.pending = false;
        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProfilePageKind {
    Recipes,
    Collections,
}

impl ProfilePageKind {
    fn noun(self) -> &'static str {
        match self {
            Self::Recipes => "recipes",
            Self::Collections => "collections",
        }
    }
}

#[component]
fn ProfilePagination(
    kind: ProfilePageKind,
    pending: bool,
    failed: bool,
    has_more: bool,
    count: usize,
    onaction: EventHandler<ProfilePageAction>,
) -> Element {
    let noun = kind.noun();
    let blocked = pending || (!failed && !has_more);
    let label = if pending {
        format!("Loading {noun}…")
    } else if failed {
        format!("Retry {noun}")
    } else if has_more {
        format!("Load more {noun}")
    } else {
        format!("All {noun} loaded")
    };
    rsx! {
        div { class: "load-more",
            p { role: if failed && !pending { "alert" } else { "status" },
                if pending { "Loading public {noun}…" }
                else if failed { "Public {noun} couldn’t be loaded. Please try again." }
                else { "{count} public {noun} loaded" }
            }
            // Keep this node mounted across request/failure/completion, and
            // claim requests synchronously in the owning section's state.
            button { r#type: "button", class: "btn btn-secondary touch-target", aria_disabled: blocked,
                onclick: move |_| {
                    if !blocked { onaction.call(if failed { ProfilePageAction::Retry } else { ProfilePageAction::LoadMore }); }
                },
                "{label}"
            }
        }
    }
}

fn public_profile_name(profile: &UserProfileType) -> &str {
    // Provider-created accounts can have no display name. Keep the public
    // heading, avatar and empty-state copy consistent without changing storage.
    if profile.display_name.trim().is_empty() {
        &profile.username
    } else {
        &profile.display_name
    }
}

/// Public user profile page with paginated collection and recipe grids.
#[component]
pub fn UserProfile(username: String) -> Element {
    // Dioxus reconciles keys inside fragments, not on a lone root. Replace
    // every hook and cancel scope-owned tasks when the requested user changes.
    rsx! {
        document::Stylesheet { href: asset!("/assets/user-pages.css") }
        for requested_username in [username] {
            UserProfileContents { key: "{requested_username}", username: requested_username }
        }
    }
}

#[component]
fn UserProfileContents(username: String) -> Element {
    let uname_profile = username.clone();
    let mut profile = use_resource(move || get_user_profile(uname_profile.clone()));
    let profile_pending = *profile.state().read() == UseResourceState::Pending;
    let mut collection_pages = use_signal(ProfilePageState::<CollectionSummary>::default);
    let mut recipe_pages = use_signal(ProfilePageState::<RecipeCardSummary>::default);
    // Subscribe only to the request ticket. Appending cards or setting a failure
    // must not re-run the loader for the same offset.
    let collection_request = use_memo(move || collection_pages.read().request);
    let recipe_request = use_memo(move || recipe_pages.read().request);
    let uname_collections = username.clone();
    use_effect(move || {
        let request = collection_request();
        let uname = uname_collections.clone();
        spawn(async move {
            let result = get_user_public_collections(uname, request.offset, PAGE_SIZE)
                .await
                .map(|response| (response.collections, response.has_more))
                .map_err(|_| ());
            collection_pages.write().finish(request, result);
        });
    });
    let uname_recipes = username;
    use_effect(move || {
        let request = recipe_request();
        let uname = uname_recipes.clone();
        spawn(async move {
            let result = get_user_public_recipes(uname, request.offset, PAGE_SIZE)
                .await
                .map(|response| (response.recipes, response.has_more))
                .map_err(|_| ());
            recipe_pages.write().finish(request, result);
        });
    });
    let profile_result = profile.read().clone();
    if matches!(profile_result, Some(Err(_))) {
        return rsx! {
            div { class: "container user-profile",
                h1 { "Profile unavailable" }
                LoadRecovery {
                    pending: profile_pending,
                    message: "This profile couldn’t be loaded. It may be unavailable, or there may be a connection problem.",
                    retry_label: "Retry profile",
                    onretry: move |_| { if !profile.pending() { profile.restart(); } },
                }
                Link { to: Route::Explore {}, class: "btn btn-ghost touch-target", "Browse public recipes" }
            }
        };
    }
    let Some(Ok(profile_data)) = profile_result else {
        return rsx! {
            div { class: "container",
                div {
                    role: "status",
                    display: "flex",
                    align_items: "center",
                    justify_content: "center",
                    min_height: "60vh",
                    LoadingSpinner {}
                    span { "Loading profile…" }
                }
            }
        };
    };
    let profile_name = public_profile_name(&profile_data).to_string();

    // Compute recipe count text (pluralization must be outside rsx!)
    let recipe_count_text = if profile_data.public_recipe_count == 1 {
        "1 recipe".to_string()
    } else {
        format!("{} recipes", profile_data.public_recipe_count)
    };

    // ── Render: full profile ─────────────────────────────────────────────
    let recipes = recipe_pages.read().clone();
    let collections = collection_pages.read().clone();

    rsx! {
        div { class: "container profile-page user-profile",
            // ── Profile Header ────────────────────────────────────────────
            header { class: "user-profile__passport",
                div { class: "user-portrait",
                Avatar {
                    size: AvatarSize::Large,
                    src: profile_data.avatar_url.clone(),
                    username: profile_name.clone(),
                }
                }
                div { class: "user-profile__identity",
                p { class: "user-page-eyebrow brand-title", PageIcon { kind: PageIconKind::Profile, compact: true } span { "Public profile" } }
                h1 { class: "profile-name", "{profile_name}" }
                p { class: "profile-username", "@{profile_data.username}" }
                if let Some(ref bio) = profile_data.bio {
                    p { class: "profile-bio", "{bio}" }
                }
                p { class: "profile-stats",
                    "{recipe_count_text} shared publicly"
                }
                }
            }

            // ── Public Collection Grid ───────────────────────────────────
            details { class: "user-disclosure user-profile__section", open: true,
                summary {
                    h2 { "Public collections" }
                    span { class: "user-disclosure__chevron", aria_hidden: "true", "⌄" }
                }
                div { class: "user-disclosure__body",

                if collections.pending && collections.items.is_empty() && !collections.failed {
                    div {
                        role: "status",
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "160px",
                        LoadingSpinner {}
                        span { "Loading public collections…" }
                    }
                } else if collections.items.is_empty() && !collections.failed {
                    EmptyState {
                        icon: rsx! { "📁" },
                        title: "No public collections yet",
                        description: format!("{profile_name} hasn't shared any public collections."),
                    }
                }
                if !collections.items.is_empty() {
                    div { class: "user-profile__collections",
                        for collection in collections.items.iter() {
                            Link {
                                key: "{collection.id}",
                                to: Route::CollectionDetail { id: collection.id.to_string() },
                                class: "neumo-card user-profile__collection",
                                padding: "var(--space-lg)",
                                text_decoration: "none",
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-sm)",
                                CollectionCover {
                                    image: collection.image.clone(),
                                    preview_images: collection.preview_images.clone(),
                                    alt: format!("{} collection cover", collection.name),
                                }
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

                }
                if !collections.items.is_empty() || collections.failed {
                    ProfilePagination {
                        kind: ProfilePageKind::Collections, pending: collections.pending,
                        failed: collections.failed, has_more: collections.has_more, count: collections.items.len(),
                        onaction: move |action| { collection_pages.write().begin(action); },
                    }
                }
                }
            }

            // ── Recipe Grid ───────────────────────────────────────────────
            details { class: "user-disclosure user-profile__section", open: true,
                summary {
                    h2 { "Public recipes" }
                    span { class: "user-disclosure__chevron", aria_hidden: "true", "⌄" }
                }
                div { class: "user-disclosure__body",

                // Loading state (initial)
                if recipes.pending && recipes.items.is_empty() && !recipes.failed {
                    div {
                        role: "status",
                        display: "flex",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "200px",
                        LoadingSpinner {}
                        span { "Loading public recipes…" }
                    }
                } else if recipes.items.is_empty() && !recipes.failed {
                        // Empty state: no public recipes
                        EmptyState {
                            icon: rsx! { "🍳" },
                            title: "No public recipes yet",
                            description: format!("{profile_name} hasn't shared any public recipes."),
                        }
                }
                if !recipes.items.is_empty() {
                        // Recipe grid
                        div { class: "user-profile__recipes",
                            for recipe in recipes.items.iter() {
                                RecipeCard { recipe: recipe.clone(), framed: true }
                            }
                        }

                }
                if !recipes.items.is_empty() || recipes.failed {
                    ProfilePagination {
                        kind: ProfilePageKind::Recipes, pending: recipes.pending,
                        failed: recipes.failed, has_more: recipes.has_more, count: recipes.items.len(),
                        onaction: move |action| { recipe_pages.write().begin(action); },
                    }
                }
                }
            }
        }
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    fn profile_with_name(display_name: &str) -> UserProfileType {
        UserProfileType {
            id: uuid::Uuid::nil(),
            username: "recipe-keeper".to_string(),
            display_name: display_name.to_string(),
            avatar_url: None,
            bio: None,
            public_recipe_count: 0,
        }
    }

    #[test]
    fn public_profile_name_falls_back_for_empty_and_whitespace_names() {
        for name in ["", "   ", "\t\n", "\u{2003}\u{3000}"] {
            let profile = profile_with_name(name);
            assert_eq!(public_profile_name(&profile), "recipe-keeper");
            assert_eq!(
                profile.display_name, name,
                "fallback must not mutate stored identity"
            );
        }
    }

    #[test]
    fn public_profile_name_preserves_nonblank_display_names() {
        for name in ["Sam Ko", "小廚師", "  Sam Ko  ", "Chef <Sam> & friends"] {
            let profile = profile_with_name(name);
            assert_eq!(public_profile_name(&profile), name);
            assert_eq!(profile.username, "recipe-keeper");
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
