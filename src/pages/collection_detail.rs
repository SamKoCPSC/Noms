use dioxus::prelude::*;

use crate::api::collection::{delete_collection, get_collection, get_collection_recipes};
use crate::components::base::{
    Button, ButtonVariant, EmptyState, LoadingSpinner, PageHeader, RecipeCard,
};
use crate::components::AuthRequired;
use crate::Route;

const MAX_RECIPE_PAGE_SIZE: i64 = 100;

fn next_recipe_page_request(loaded_count: usize, total_count: i64) -> Option<(i64, i64)> {
    let loaded_count = i64::try_from(loaded_count).ok()?;
    let remaining = total_count.saturating_sub(loaded_count);
    (remaining > 0).then_some((loaded_count, remaining.min(MAX_RECIPE_PAGE_SIZE)))
}

fn confirm_recursive_delete() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().and_then(|window| window.confirm_with_message("Delete this collection and all nested collections? Folder memberships will be removed, but your recipes will remain.").ok()).unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

#[component]
pub fn CollectionDetail(id: String) -> Element {
    let requested_id = id.clone();
    let contents = use_resource(move || {
        let id = requested_id.clone();
        async move { get_collection(id).await }
    });
    let navigator = use_navigator();
    let mut deleting = use_signal(|| false);
    let mut delete_error = use_signal(|| None::<String>);
    let mut additional_recipes = use_signal(Vec::new);
    let mut loading_more = use_signal(|| false);
    let mut recipe_page_error = use_signal(|| None::<String>);
    let loaded = contents.read().clone();
    rsx! { AuthRequired { div { class: "container",
        match loaded {
            None => rsx! { LoadingSpinner {} },
            Some(Err(error)) => rsx! { EmptyState { icon: rsx! { "🔒" }, title: "Collection unavailable", description: format!("This collection could not be found or accessed: {error}") } },
            Some(Ok(contents)) => {
                let collection_id = contents.collection.id.to_string();
                let delete_collection_id = collection_id.clone();
                let new_child_url = format!("/collections/new?parent={collection_id}");
                rsx! {
                    nav { aria_label: "Breadcrumbs", margin_bottom: "var(--space-md)",
                        Link { to: Route::CollectionList {}, "Collections" }
                        for crumb in &contents.breadcrumbs { " / " Link { to: Route::CollectionDetail { id: crumb.id.to_string() }, "{crumb.name}" } }
                    }
                    PageHeader { title: contents.collection.name.clone(), action: rsx! {
                        div { display: "flex", gap: "var(--space-sm)",
                            a { href: "{new_child_url}", class: "btn btn-secondary touch-target", "New subcollection" }
                            Link { to: Route::CollectionEdit { id: collection_id.clone() }, class: "btn btn-secondary touch-target", "Edit" }
                            Button { variant: ButtonVariant::Danger, disabled: deleting(), onclick: move |_| {
                                if !confirm_recursive_delete() { return; }
                                deleting.set(true); delete_error.set(None); let id = delete_collection_id.clone();
                                spawn(async move { match delete_collection(id).await { Ok(()) => { navigator.push(Route::Dashboard {}); }, Err(error) => { delete_error.set(Some(error.to_string())); deleting.set(false); } } });
                            }, if deleting() { "Deleting…" } else { "Delete" } }
                        }
                    } }
                    if let Some(description) = &contents.collection.description { p { color: "var(--text-secondary)", margin_bottom: "var(--space-lg)", "{description}" } }
                    if let Some(error) = delete_error() { p { role: "alert", color: "var(--error)", "{error}" } }
                    if !contents.child_collections.is_empty() {
                        h2 { "Folders" }
                        div { display: "grid", grid_template_columns: "repeat(auto-fill, minmax(220px, 1fr))", gap: "var(--space-md)", margin_bottom: "var(--space-xl)",
                            for child in &contents.child_collections { Link { to: Route::CollectionDetail { id: child.id.to_string() }, class: "neumo-card", padding: "var(--space-md)", text_decoration: "none", "📁  {child.name}" } }
                        }
                    }
                    h2 { "Recipes ({contents.recipes.total_count})" }
                    if contents.recipes.recipes.is_empty() && contents.child_collections.is_empty() {
                        EmptyState { icon: rsx! { "📂" }, title: "This collection is empty", description: "Add recipes or create a subcollection to start organizing.",
                            action: rsx! { Link { to: Route::Dashboard {}, class: "btn btn-primary touch-target", "Add recipes" } }
                        }
                    } else if contents.recipes.recipes.is_empty() { p { color: "var(--text-secondary)", "No recipes directly in this collection." } }
                    else { div { class: "recipe-grid",
                        for recipe in &contents.recipes.recipes { RecipeCard { recipe: recipe.clone() } }
                        for recipe in additional_recipes() { RecipeCard { recipe } }
                    } }
                    if let Some(error) = recipe_page_error() { p { role: "alert", color: "var(--error)", "Could not load more recipes: {error}" } }
                    if let Some((offset, limit)) = next_recipe_page_request(contents.recipes.recipes.len() + additional_recipes().len(), contents.recipes.total_count) {
                        Button { variant: ButtonVariant::Secondary, disabled: loading_more(), onclick: move |_| {
                            loading_more.set(true); recipe_page_error.set(None); let id = collection_id.clone();
                            let initial_recipe_ids = contents.recipes.recipes.iter().map(|recipe| recipe.id).collect::<std::collections::HashSet<_>>();
                            spawn(async move {
                                match get_collection_recipes(id, offset, limit).await {
                                    Ok(page) => {
                                        let loaded_recipe_ids = additional_recipes().iter().map(|recipe| recipe.id).collect::<std::collections::HashSet<_>>();
                                        additional_recipes.write().extend(page.recipes.into_iter().filter(|candidate| !initial_recipe_ids.contains(&candidate.id) && !loaded_recipe_ids.contains(&candidate.id)));
                                    }
                                    Err(error) => recipe_page_error.set(Some(error.to_string())),
                                }
                                loading_more.set(false);
                            });
                        }, if loading_more() { "Loading…" } else { "Load more" } }
                    }
                }
            }
        }
    } } }
}

#[cfg(test)]
mod tests {
    use super::next_recipe_page_request;

    #[test]
    fn recipe_pagination_requests_every_remaining_page() {
        assert_eq!(next_recipe_page_request(12, 30), Some((12, 18)));
        assert_eq!(next_recipe_page_request(30, 30), None);
        assert_eq!(next_recipe_page_request(0, 0), None);
    }
}
