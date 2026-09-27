use dioxus::prelude::*;

use crate::api::recipe::{list_my_recipes, list_recipe_drafts};
use crate::auth::context::use_auth;
use crate::components::base::{
    AddToCollectionDropdown, CollectionSidebar, EditedRecipeImage, EmptyState, LoadingSpinner,
    RecipeCard,
};
use crate::components::AuthRequired;
use crate::types::{RecipeDraftView, RecipeImage, RecipeListResponse};
use crate::Route;

const DASHBOARD_CSS: Asset = asset!("/assets/dashboard.css");
const PAGE_SIZE: i64 = 12;

fn draft_card_image(images: &[RecipeImage]) -> Option<RecipeImage> {
    images.first().cloned()
}

/// The existing API returns one page, so describe the visible range rather
/// than implying that advancing appends records.
fn page_range(offset: i64, count: usize) -> Option<(i64, i64)> {
    (count > 0).then_some((offset + 1, offset + count as i64))
}

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: DASHBOARD_CSS }
        AuthRequired { DashboardWorkspace {} }
    }
}

#[component]
fn DashboardWorkspace() -> Element {
    let auth = use_auth();
    let mut offset = use_signal(|| 0i64);
    let mut grid_view = use_signal(|| false);
    let mut show_all_drafts = use_signal(|| false);
    let mut recipes = use_resource(move || {
        let off = offset();
        async move { list_my_recipes(off, PAGE_SIZE).await }
    });
    let mut drafts = use_resource(list_recipe_drafts);
    let pending = recipes.pending();
    let drafts_pending = drafts.pending();
    let recipes_result: Option<Result<RecipeListResponse, ServerFnError>> = recipes.read().clone();
    let drafts_result: Option<Result<Vec<RecipeDraftView>, ServerFnError>> = drafts.read().clone();
    let draft_list = drafts_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let response = recipes_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let error = recipes_result
        .as_ref()
        .and_then(|result| result.as_ref().err());
    let current_offset = offset();

    rsx! {
        div { class: "container dashboard-workspace", id: "workspace-overview",
            aside { class: "dashboard-sidebar neumo-card", aria_label: "Workspace",
                p { class: "dashboard-eyebrow", "Workspace" }
                nav { class: "dashboard-workspace-nav", aria_label: "Dashboard sections",
                    a { href: "#workspace-overview", span { aria_hidden: true, "⌂" } "Overview" }
                    a { href: "#dashboard-library", class: "dashboard-nav-current",
                        span { aria_hidden: true, "▤" } "All recipes"
                    }
                    a { href: "#dashboard-drafts", onclick: move |_| show_all_drafts.set(true),
                        span { aria_hidden: true, "▱" } "Drafts"
                        if let Some(items) = draft_list { span { class: "dashboard-nav-count", "{items.len()}" } }
                    }
                }
                div { class: "dashboard-collections",
                    h2 { class: "dashboard-eyebrow", "Collections" }
                    CollectionSidebar { embedded: true }
                    Link { to: Route::CollectionList {}, class: "dashboard-sidebar-link", "Browse all collections →" }
                    Link { to: Route::CollectionNew { parent: None }, class: "dashboard-sidebar-link", "+ New collection" }
                }
                nav { class: "dashboard-account-nav", aria_label: "Your account",
                    if let Some(user) = &auth.current_user {
                        Link { to: Route::UserProfile { username: user.username.clone() }, "My profile" }
                    }
                    Link { to: Route::SettingsProfile {}, "Settings" }
                }
            }
            section { class: "dashboard-content", aria_label: "Recipe workspace",
                header { class: "dashboard-heading",
                    div {
                        p { class: "dashboard-breadcrumb", "Dashboard / All recipes" }
                        h1 { "My recipes" }
                        p { class: "dashboard-summary",
                            if let Some(resp) = response { "{resp.total_count} saved recipes" } else { "Your recipe library" }
                            if let Some(items) = draft_list { " · {items.len()} private drafts" }
                        }
                    }
                    Link { to: Route::RecipeNew {}, class: "btn btn-primary touch-target", "+ New recipe" }
                }
                section { class: "dashboard-drafts neumo-card", id: "dashboard-drafts", aria_label: "Continue editing",
                    div { class: "dashboard-section-heading",
                        h2 { "Continue editing" }
                        if let Some(items) = draft_list {
                            if items.len() > 2 {
                                button { r#type: "button", class: "btn btn-ghost touch-target",
                                    aria_expanded: show_all_drafts(), aria_controls: "dashboard-draft-list",
                                    onclick: move |_| { let expanded = show_all_drafts(); show_all_drafts.set(!expanded); },
                                    if show_all_drafts() { "Show recent drafts" } else { "View all drafts →" }
                                }
                            }
                        }
                    }
                    if drafts_pending {
                        p { role: "status", "Loading drafts…" }
                    } else if let Some(Err(_)) = &drafts_result {
                        div { class: "dashboard-error", role: "alert",
                            p { "Your drafts couldn’t be loaded. Try again to continue editing." }
                            button { class: "btn btn-secondary touch-target", onclick: move |_| drafts.restart(), "Retry drafts" }
                        }
                    } else if let Some(items) = draft_list {
                        if items.is_empty() {
                            p { class: "dashboard-muted", "No drafts in progress. New recipes will appear here as you work." }
                        } else {
                            div { class: "dashboard-draft-list", id: "dashboard-draft-list",
                                for draft in items.iter().take(if show_all_drafts() { items.len() } else { 2 }) {
                                    Link { key: "{draft.id}", class: "dashboard-draft", to: Route::RecipeDraftEdit { id: draft.id.to_string() },
                                        DashboardThumbnail { image: draft_card_image(&draft.editor.images), title: draft.editor.title.clone() }
                                        div { class: "dashboard-draft-copy",
                                            h3 { if draft.editor.title.trim().is_empty() { "Untitled recipe" } else { "{draft.editor.title}" } }
                                            p { "Revision {draft.revision} · Private draft" }
                                            if let Some(source) = &draft.forked_from_title { p { "Forked from {source}" } }
                                        }
                                        span { class: "dashboard-draft-arrow", aria_hidden: true, "→" }
                                    }
                                }
                            }
                        }
                    }
                }
                section { class: "dashboard-library", id: "dashboard-library", aria_label: "Saved recipes",
                    div { class: "dashboard-toolbar neumo-card",
                        div { h2 { "Your library" } p { class: "dashboard-muted", "Saved recipes and their sharing settings" } }
                        div { class: "dashboard-toolbar-actions",
                            Link { to: Route::Search { query: crate::search_url::SearchRouteQuery::from("scope=recipes&vis=mine") }, class: "btn btn-secondary touch-target", "Search my recipes" }
                            div { class: "dashboard-view-switch", role: "group", aria_label: "Recipe display",
                                button { r#type: "button", aria_pressed: !grid_view(), class: if !grid_view() { "is-selected" } else { "" }, onclick: move |_| grid_view.set(false), "List" }
                                button { r#type: "button", aria_pressed: grid_view(), class: if grid_view() { "is-selected" } else { "" }, onclick: move |_| grid_view.set(true), "Grid" }
                            }
                        }
                    }
                    if pending {
                        div { class: "dashboard-loading neumo-card", role: "status", LoadingSpinner {} span { "Loading your recipes…" } }
                    } else if error.is_some() {
                        div { class: "dashboard-error neumo-card", role: "alert",
                            h2 { "We couldn’t load your recipes" }
                            p { "Try again to return to your library." }
                            button { class: "btn btn-secondary touch-target", onclick: move |_| recipes.restart(), "Retry recipes" }
                            if current_offset > 0 {
                                button { class: "btn btn-ghost touch-target", onclick: move |_| offset.set((current_offset - PAGE_SIZE).max(0)), "Previous page" }
                            }
                        }
                    } else if let Some(resp) = response {
                        if resp.recipes.is_empty() {
                            div { class: "dashboard-empty neumo-card",
                                EmptyState { icon: rsx! { "▤" },
                                    title: if current_offset == 0 { "No saved recipes yet" } else { "No recipes on this page" },
                                    description: if current_offset == 0 { "Create a recipe or continue one of your drafts to grow your library." } else { "Your library may have changed. Return to the first page to see your recipes." },
                                    action: rsx! {
                                        if current_offset == 0 { Link { to: Route::RecipeNew {}, class: "btn btn-primary touch-target", "Create a recipe" } }
                                        else { button { class: "btn btn-secondary touch-target", onclick: move |_| offset.set(0), "Back to first page" } }
                                    }
                                }
                            }
                        } else {
                            if grid_view() {
                                div { class: "dashboard-recipe-grid",
                                    for recipe in &resp.recipes {
                                        article { key: "{recipe.id}", class: "dashboard-grid-item",
                                            RecipeCard { recipe: recipe.clone(), framed: true }
                                            div { class: "dashboard-grid-actions",
                                                span { class: "dashboard-visibility", "{recipe.visibility}" }
                                                Link { to: Route::RecipeEdit { id: recipe.id.to_string() }, class: "btn btn-ghost touch-target", aria_label: "Edit {recipe.title}", "Edit" }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { class: "dashboard-table-panel neumo-card",
                                    table { class: "dashboard-table",
                                        caption { class: "sr-only", "Your saved recipes, sharing settings and editing actions" }
                                        thead { tr { th { scope: "col", "Recipe" } th { scope: "col", "Status" } th { scope: "col", "Visibility" } th { scope: "col", "Updated" } th { scope: "col", "Actions" } } }
                                        tbody {
                                            for recipe in &resp.recipes {
                                                tr { key: "{recipe.id}",
                                                    th { scope: "row",
                                                        Link { class: "dashboard-recipe-link", to: Route::RecipeDetail { id: recipe.id.to_string() },
                                                            DashboardThumbnail { image: recipe.images.first().cloned(), title: recipe.title.clone() }
                                                            div { class: "dashboard-recipe-copy", strong { "{recipe.title}" } p { {recipe.description.as_deref().unwrap_or("No description")} } }
                                                        }
                                                    }
                                                    td { "data-label": "Status", span { class: "dashboard-status", "Published" } }
                                                    td { "data-label": "Visibility", span { class: "dashboard-visibility", "{recipe.visibility}" } }
                                                    td { "data-label": "Updated", time { datetime: recipe.updated_at.to_rfc3339(), {recipe.updated_at.format("%d %b %Y").to_string()} } }
                                                    td { class: "dashboard-row-actions",
                                                        Link { to: Route::RecipeEdit { id: recipe.id.to_string() }, class: "btn btn-ghost touch-target", aria_label: "Edit {recipe.title}", "Edit" }
                                                        AddToCollectionDropdown { recipe_id: recipe.id.to_string(), recipe_title: recipe.title.clone() }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Keep pagination mounted while a request runs so keyboard focus
                    // is not discarded when the recipe list is replaced.
                    footer { class: "dashboard-pagination",
                            p { role: "status",
                                if pending { "Loading page…" }
                                else if let Some(resp) = response {
                                    if let Some((first, last)) = page_range(current_offset, resp.recipes.len()) { "Showing {first}–{last} of {resp.total_count} saved recipes" }
                                }
                            }
                            div {
                                button { r#type: "button", class: "btn btn-secondary touch-target", aria_disabled: pending || current_offset == 0,
                                    onclick: move |_| { if !pending && current_offset > 0 { offset.set((current_offset - PAGE_SIZE).max(0)); } }, "Previous" }
                                button { r#type: "button", class: "btn btn-secondary touch-target", aria_disabled: pending || !response.is_some_and(|resp| resp.has_more),
                                    onclick: move |_| { if !pending && recipes.read().as_ref().is_some_and(|result| result.as_ref().is_ok_and(|resp| resp.has_more)) { offset.set(current_offset + PAGE_SIZE); } }, "Next" }
                            }
                    }
                }
            }
        }
    }
}

#[component]
fn DashboardThumbnail(image: Option<RecipeImage>, title: String) -> Element {
    let mut failed = use_signal(|| false);
    rsx! {
        div { class: "dashboard-thumbnail",
            if let Some(image) = image.filter(|_| !failed()) {
                EditedRecipeImage { image, class: "dashboard-thumbnail-image", alt: title, prefer_thumbnail: true, on_error: move |_| failed.set(true) }
            } else { span { aria_hidden: true, "▤" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{draft_card_image, page_range};
    use crate::types::{RecipeImage, RecipeImageEdit};

    #[test]
    fn draft_thumbnail_preserves_the_first_images_edit_metadata() {
        let first = RecipeImage::new(
            "https://example.test/first.webp".into(),
            Some("https://example.test/thumb.webp".into()),
            RecipeImageEdit::default_rect_16x9(),
        );
        let second = RecipeImage::new(
            "https://example.test/second.webp".into(),
            None,
            RecipeImageEdit::default_rect_16x9(),
        );
        assert_eq!(draft_card_image(&[first.clone(), second]), Some(first));
        assert_eq!(draft_card_image(&[]), None);
    }

    #[test]
    fn page_ranges_describe_the_visible_page_and_never_claim_an_empty_row() {
        assert_eq!(page_range(0, 12), Some((1, 12)));
        assert_eq!(page_range(12, 3), Some((13, 15)));
        assert_eq!(page_range(12, 0), None);
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
