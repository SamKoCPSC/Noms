use dioxus::prelude::*;

use std::collections::HashSet;

#[cfg(target_arch = "wasm32")]
use crate::api::collection::reorder_collection_recipes;
use crate::api::collection::{
    delete_collection, get_collection, get_collection_recipes, remove_recipe_from_collection,
    reorder_child_collections,
};
use crate::components::base::{
    Button, ButtonVariant, CollectionCover, CollectionVisibilityBadge, EmptyState, Input,
    LoadingSpinner, PageHeader, RecipeCard, ReorderHandle,
};
use crate::types::{CollectionSummary, CollectionVisibility, Recipe};
use crate::Route;

const MAX_RECIPE_PAGE_SIZE: i64 = 100;
#[cfg(any(test, target_arch = "wasm32"))]
const RECIPE_REORDER_DEBOUNCE_MS: u32 = 500;
const RECURSIVE_DELETE_WARNING: &str = "Deleting this collection also deletes all nested collections and their memberships. Your recipes will remain.";

#[cfg(target_arch = "wasm32")]
const OPEN_DELETE_DIALOG_KEYBOARD_LIFECYCLE: &str = r#"
    window.__nomsCollectionDeleteKeyboard?.abort();
    const controller = new AbortController();
    window.__nomsCollectionDeleteKeyboard = controller;
    const dialog = document.getElementById('delete-collection-dialog');
    if (!dialog) {
        controller.abort();
        return;
    }

    dialog.querySelector('input')?.focus();
    document.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
            if (dialog.dataset.deleting !== 'true') {
                event.preventDefault();
                document.getElementById('delete-collection-cancel')
                    ?.querySelector('button')?.click();
            }
            return;
        }

        if (event.key === 'Tab') {
            const controls = [...dialog.querySelectorAll(
                'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
            )].filter((element) => element.getClientRects().length > 0);
            if (controls.length === 0) {
                event.preventDefault();
                dialog.focus();
                return;
            }

            const first = controls[0];
            const last = controls[controls.length - 1];
            if (event.shiftKey && (document.activeElement === first || !dialog.contains(document.activeElement))) {
                event.preventDefault();
                last.focus();
            } else if (!event.shiftKey && (document.activeElement === last || !dialog.contains(document.activeElement))) {
                event.preventDefault();
                first.focus();
            }
        }
    }, { signal: controller.signal });
"#;

#[cfg(target_arch = "wasm32")]
const CLOSE_DELETE_DIALOG_KEYBOARD_LIFECYCLE: &str = r#"
    window.__nomsCollectionDeleteKeyboard?.abort();
    window.__nomsCollectionDeleteKeyboard = null;
    document.getElementById('delete-collection-trigger')
        ?.querySelector('button')?.focus();
"#;

#[cfg(target_arch = "wasm32")]
const ABORT_DELETE_DIALOG_KEYBOARD_LIFECYCLE: &str = r#"
    window.__nomsCollectionDeleteKeyboard?.abort();
    window.__nomsCollectionDeleteKeyboard = null;
"#;

fn collection_delete_confirmation_phrase(collection_name: &str) -> String {
    format!("delete {collection_name}")
}

fn collection_delete_confirmation_matches(input: &str, collection_name: &str) -> bool {
    input == collection_delete_confirmation_phrase(collection_name)
}

fn collection_load_error_copy(_error: &ServerFnError) -> (&'static str, &'static str) {
    (
        "Collection not found",
        "This collection could not be found.",
    )
}

fn next_recipe_page_request(loaded_count: usize, total_count: i64) -> Option<(i64, i64)> {
    let loaded_count = i64::try_from(loaded_count).ok()?;
    let remaining = total_count.saturating_sub(loaded_count);
    (remaining > 0).then_some((loaded_count, remaining.min(MAX_RECIPE_PAGE_SIZE)))
}

fn membership_mutation_controls_disabled(
    loading_more: bool,
    removing_count: usize,
    refreshing_contents: bool,
) -> bool {
    loading_more || removing_count > 0 || refreshing_contents
}

fn can_reorder_complete_recipe_list(loaded_count: usize, total_count: i64) -> bool {
    i64::try_from(loaded_count).is_ok_and(|loaded_count| loaded_count == total_count)
}

fn collection_recipe_page_is_stale(
    expected_total_count: i64,
    loaded_count_before: usize,
    response_total_count: i64,
    response_recipe_count: usize,
    unique_new_recipe_count: usize,
    response_has_more: bool,
) -> bool {
    let Some(loaded_count_after) = loaded_count_before.checked_add(unique_new_recipe_count) else {
        return true;
    };
    let Ok(loaded_count_after) = i64::try_from(loaded_count_after) else {
        return true;
    };

    expected_total_count < 0
        || response_total_count != expected_total_count
        || unique_new_recipe_count == 0
        || unique_new_recipe_count != response_recipe_count
        || loaded_count_after > expected_total_count
        || response_has_more != (loaded_count_after < expected_total_count)
}

#[cfg(any(test, target_arch = "wasm32"))]
fn should_persist_reorder_generation(scheduled_generation: u64, current_generation: u64) -> bool {
    scheduled_generation == current_generation
}

#[cfg(target_arch = "wasm32")]
fn browser_api_error(action: &str, error: wasm_bindgen::JsValue) -> String {
    use wasm_bindgen::JsCast;

    let detail = error
        .dyn_ref::<js_sys::Error>()
        .map(|error| error.message().into())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| format!("{error:?}"));
    format!("{action}: {detail}")
}

#[cfg(target_arch = "wasm32")]
fn absolute_collection_url(relative_url: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "Browser window is unavailable.".to_string())?;
    let origin = window
        .location()
        .origin()
        .map_err(|error| browser_api_error("Could not read the page origin", error))?;
    web_sys::Url::new_with_base(relative_url, &origin)
        .map(|url| url.href())
        .map_err(|error| browser_api_error("Could not build the collection URL", error))
}

#[cfg(target_arch = "wasm32")]
async fn copy_to_clipboard(url: &str) -> Result<(), String> {
    let navigator = web_sys::window()
        .ok_or_else(|| "Browser window is unavailable.".to_string())?
        .navigator();
    wasm_bindgen_futures::JsFuture::from(navigator.clipboard().write_text(url))
        .await
        .map(|_| ())
        .map_err(|error| browser_api_error("Could not copy the collection link", error))
}

async fn share_collection_link(title: String, url: String) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window =
            web_sys::window().ok_or_else(|| "Browser window is unavailable.".to_string())?;
        let navigator = window.navigator();
        let absolute_url = absolute_collection_url(&url)?;
        let share_api = js_sys::Reflect::get(
            navigator.as_ref(),
            &wasm_bindgen::JsValue::from_str("share"),
        )
        .map_err(|error| browser_api_error("Could not inspect Web Share support", error))?;

        if share_api.is_function() {
            let share_data = web_sys::ShareData::new();
            share_data.set_title(&title);
            share_data.set_url(&absolute_url);
            wasm_bindgen_futures::JsFuture::from(navigator.share_with_data(&share_data))
                .await
                .map_err(|error| browser_api_error("Could not share the collection link", error))?;
        } else {
            copy_to_clipboard(&absolute_url).await?;
        }
        Ok("Collection link shared or copied successfully.".to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (title, url);
        Err("Sharing is only available in the browser.".to_string())
    }
}

async fn copy_collection_link(url: String) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let absolute_url = absolute_collection_url(&url)?;
        copy_to_clipboard(&absolute_url).await?;
        Ok("Collection link copied to clipboard.".to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = url;
        Err("Copying links is only available in the browser.".to_string())
    }
}

#[component]
pub fn CollectionDetail(id: String) -> Element {
    let requested_id = id.clone();
    let mut contents = use_resource(move || {
        let id = requested_id.clone();
        async move { get_collection(id).await }
    });
    let navigator = use_navigator();
    let mut deleting = use_signal(|| false);
    let mut delete_modal_open = use_signal(|| false);
    let mut delete_confirmation_input = use_signal(String::new);
    let mut delete_error = use_signal(|| None::<String>);
    let mut additional_recipes = use_signal(Vec::new);
    let mut loading_more = use_signal(|| false);
    let mut recipe_page_error = use_signal(|| None::<String>);
    let mut removing_recipes = use_signal(HashSet::<uuid::Uuid>::new);
    let mut removal_error = use_signal(|| None::<String>);
    let mut child_order_override = use_signal(|| None::<Vec<CollectionSummary>>);
    let mut recipe_order_override = use_signal(|| None::<Vec<Recipe>>);
    let mut folder_save_in_flight = use_signal(|| false);
    let mut recipe_reorder_pending = use_signal(|| false);
    let mut recipe_save_in_flight = use_signal(|| false);
    let mut recipe_reorder_generation = use_signal(|| 0_u64);
    let mut reorder_error = use_signal(|| None::<String>);
    let mut link_status = use_signal(|| None::<Result<String, String>>);
    let refreshing_contents = contents.pending();
    let loaded = contents.read().clone();

    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if delete_modal_open() {
            spawn(async move {
                let _ = document::eval(OPEN_DELETE_DIALOG_KEYBOARD_LIFECYCLE).await;
            });
        }
    });

    rsx! { div { class: "container",
        match loaded {
            None => rsx! { LoadingSpinner {} },
            Some(Err(error)) => {
                let (title, description) = collection_load_error_copy(&error);
                rsx! { EmptyState { icon: rsx! { "🔒" }, title: title, description: description } }
            },
            Some(Ok(collection_contents)) => {
                let collection_id = collection_contents.collection.id.to_string();
                let collection_name = collection_contents.collection.name.clone();
                let delete_confirmation_phrase =
                    collection_delete_confirmation_phrase(&collection_name);
                let delete_confirmation_matches = collection_delete_confirmation_matches(
                    &delete_confirmation_input(),
                    &collection_name,
                );
                let delete_collection_id = collection_id.clone();
                let new_child_url = format!("/collections/new?parent={collection_id}");
                let displayed_children = child_order_override()
                    .unwrap_or_else(|| collection_contents.child_collections.clone());
                let loaded_recipes = collection_contents
                    .recipes
                    .recipes
                    .iter()
                    .cloned()
                    .chain(additional_recipes())
                    .collect::<Vec<_>>();
                let displayed_recipes =
                    recipe_order_override().unwrap_or_else(|| loaded_recipes.clone());
                let can_edit = collection_contents.can_edit;
                let complete_recipe_list = can_reorder_complete_recipe_list(
                    displayed_recipes.len(),
                    collection_contents.recipes.total_count,
                );
                let membership_controls_disabled = membership_mutation_controls_disabled(
                    loading_more(),
                    removing_recipes.read().len(),
                    refreshing_contents,
                ) || folder_save_in_flight()
                    || recipe_reorder_pending()
                    || recipe_save_in_flight();
                let folder_controls_disabled = folder_save_in_flight()
                    || recipe_reorder_pending()
                    || recipe_save_in_flight()
                    || membership_mutation_controls_disabled(
                        loading_more(),
                        removing_recipes.read().len(),
                        refreshing_contents,
                    );
                let recipe_controls_disabled = !complete_recipe_list
                    || folder_save_in_flight()
                    || recipe_save_in_flight()
                    || membership_mutation_controls_disabled(
                        loading_more(),
                        removing_recipes.read().len(),
                        refreshing_contents,
                );
                let pagination_disabled = loading_more()
                    || refreshing_contents
                    || (can_edit
                        && (folder_save_in_flight()
                            || recipe_reorder_pending()
                            || recipe_save_in_flight()
                            || !removing_recipes.read().is_empty()));
                rsx! {
                    nav {
                        aria_label: "Breadcrumbs",
                        display: "flex",
                        flex_wrap: "wrap",
                        align_items: "center",
                        gap: "var(--space-xs)",
                        min_width: "0",
                        max_width: "100%",
                        margin_bottom: "var(--space-md)",
                        if can_edit {
                            Link {
                                to: Route::CollectionList {},
                                max_width: "100%",
                                overflow_wrap: "anywhere",
                                "Collections"
                            }
                        } else {
                            Link {
                                to: Route::UserProfile { username: collection_contents.owner_username.clone() },
                                max_width: "100%",
                                overflow_wrap: "anywhere",
                                "@{collection_contents.owner_username}"
                            }
                        }
                        for crumb in &collection_contents.breadcrumbs {
                            span { aria_hidden: "true", flex_shrink: "0", "/" }
                            Link {
                                to: Route::CollectionDetail { id: crumb.id.to_string() },
                                min_width: "0",
                                max_width: "100%",
                                overflow_wrap: "anywhere",
                                aria_current: if crumb.id == collection_contents.collection.id { "page" } else { "false" },
                                "{crumb.name}"
                            }
                        }
                    }
                    PageHeader { title: collection_contents.collection.name.clone(), action: rsx! {
                        div { display: "flex", flex_wrap: "wrap", gap: "var(--space-sm)",
                            if can_edit {
                                a { href: "{new_child_url}", class: "btn btn-secondary touch-target", "New subcollection" }
                                Link { to: Route::CollectionEdit { id: collection_id.clone() }, class: "btn btn-secondary touch-target", "Edit" }
                                span {
                                    id: "delete-collection-trigger",
                                    Button { variant: ButtonVariant::Danger, disabled: deleting(), onclick: move |_| {
                                        delete_confirmation_input.set(String::new());
                                        delete_error.set(None);
                                        delete_modal_open.set(true);
                                    }, "Delete" }
                                }
                            }
                            if can_edit && collection_contents.collection.visibility != CollectionVisibility::Private {
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    onclick: {
                                        let url = format!("/collections/{collection_id}");
                                        move |_| {
                                            link_status.set(None);
                                            let url = url.clone();
                                            spawn(async move {
                                                link_status.set(Some(copy_collection_link(url).await));
                                            });
                                        }
                                    },
                                    "Copy link"
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    onclick: {
                                        let title = collection_name.clone();
                                        let url = format!("/collections/{collection_id}");
                                        move |_| {
                                            link_status.set(None);
                                            let title = title.clone();
                                            let url = url.clone();
                                            spawn(async move {
                                                link_status.set(Some(share_collection_link(title, url).await));
                                            });
                                        }
                                    },
                                    "Share"
                                }
                            }
                        }
                    } }
                    CollectionCover {
                        image: collection_contents.collection.image.clone(),
                        preview_images: collection_contents.collection.preview_images.clone(),
                        alt: format!("{} collection cover", collection_contents.collection.name),
                        class: "collection-cover--hero".to_string(),
                    }
                    div {
                        display: "flex",
                        flex_wrap: "wrap",
                        align_items: "center",
                        gap: "var(--space-sm)",
                        margin_bottom: "var(--space-md)",
                        CollectionVisibilityBadge { visibility: collection_contents.collection.visibility }
                        span { color: "var(--text-secondary)", "By @{collection_contents.owner_username}" }
                    }
                    if let Some(description) = &collection_contents.collection.description { p { color: "var(--text-secondary)", margin_bottom: "var(--space-lg)", "{description}" } }
                    if can_edit {
                        if let Some(error) = delete_error() { p { role: "alert", color: "var(--error)", "{error}" } }
                    }
                    if can_edit {
                        if let Some(status) = link_status() {
                            match status {
                                Ok(message) => rsx! { p { role: "status", aria_live: "polite", color: "var(--success)", "{message}" } },
                                Err(error) => rsx! { p { role: "alert", aria_live: "assertive", color: "var(--error)", "Collection link action failed: {error}" } },
                            }
                        }
                    }
                    if can_edit {
                        if let Some(error) = reorder_error() {
                            p {
                                role: "alert",
                                color: "var(--error)",
                                "Could not save the new order. The collection was reloaded: {error}"
                            }
                        }
                    }
                    if !displayed_children.is_empty() {
                        h2 { "Folders" }
                        div { display: "grid", grid_template_columns: "repeat(auto-fill, minmax(220px, 1fr))", gap: "var(--space-md)", margin_bottom: "var(--space-xl)",
                            for (child_index, child) in displayed_children.iter().cloned().enumerate() {
                                div {
                                    key: "{child.id}",
                                    class: "neumo-card",
                                    padding: "var(--space-md)",
                                    display: "flex",
                                    flex_direction: "column",
                                    gap: "var(--space-sm)",
                                    CollectionCover {
                                        image: child.image.clone(),
                                        preview_images: child.preview_images.clone(),
                                        alt: format!("{} collection cover", child.name),
                                    }
                                    Link {
                                        to: Route::CollectionDetail { id: child.id.to_string() },
                                        text_decoration: "none",
                                        "📁  {child.name}"
                                    }
                                    if can_edit {
                                        ReorderHandle {
                                            current_index: child_index,
                                            item_count: displayed_children.len(),
                                            item_label: format!("folder {}", child.name),
                                            disabled: folder_controls_disabled,
                                            on_reorder: {
                                                let collection_id = collection_id.clone();
                                                let displayed_children = displayed_children.clone();
                                                move |target_index| {
                                                    if folder_controls_disabled {
                                                        return;
                                                    }
                                                    let mut reordered_children = displayed_children.clone();
                                                    reordered_children.swap(child_index, target_index);
                                                    let ordered_collection_ids = reordered_children
                                                        .iter()
                                                        .map(|child| child.id.to_string())
                                                        .collect::<Vec<_>>();
                                                    child_order_override.set(Some(reordered_children));
                                                    folder_save_in_flight.set(true);
                                                    reorder_error.set(None);
                                                    let parent_collection_id = Some(collection_id.clone());
                                                    spawn(async move {
                                                        match reorder_child_collections(
                                                            parent_collection_id,
                                                            ordered_collection_ids,
                                                        )
                                                        .await
                                                        {
                                                            Ok(()) => folder_save_in_flight.set(false),
                                                            Err(error) => {
                                                                folder_save_in_flight.set(false);
                                                                child_order_override.set(None);
                                                                recipe_order_override.set(None);
                                                                additional_recipes.set(Vec::new());
                                                                reorder_error.set(Some(error.to_string()));
                                                                contents.restart();
                                                            }
                                                        }
                                                    });
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                        if can_edit && folder_save_in_flight() {
                            p { role: "status", color: "var(--text-secondary)", "Saving folder order…" }
                        }
                    }
                    h2 { "Recipes ({collection_contents.recipes.total_count})" }
                    if collection_contents.recipes.recipes.is_empty() && collection_contents.child_collections.is_empty() {
                        if can_edit {
                            EmptyState { icon: rsx! { "📂" }, title: "This collection is empty", description: "Add recipes or create a subcollection to start organizing.",
                                action: rsx! { Link { to: Route::Dashboard {}, class: "btn btn-primary touch-target", "Add recipes" } }
                            }
                        } else {
                            EmptyState { icon: rsx! { "📂" }, title: "Nothing shared here yet", description: "This collection does not currently contain any recipes or subcollections you can view." }
                        }
                    } else if collection_contents.recipes.recipes.is_empty() { p { color: "var(--text-secondary)", "No recipes directly in this collection." } }
                    else { div { class: "recipe-grid",
                        for (recipe_index, recipe) in displayed_recipes.iter().cloned().enumerate() {
                            {
                                let recipe_id = recipe.id;
                                let recipe_id_string = recipe_id.to_string();
                                let reorder_collection_id = collection_id.clone();
                                let removal_collection_id = collection_id.clone();
                                let displayed_recipes = displayed_recipes.clone();
                                rsx! {
                                    div {
                                        key: "{recipe_id}",
                                        display: "flex",
                                        flex_direction: "column",
                                        gap: "var(--space-sm)",
                                        RecipeCard { recipe }
                                        if collection_contents.can_edit {
                                            ReorderHandle {
                                                current_index: recipe_index,
                                                item_count: displayed_recipes.len(),
                                                item_label: format!("recipe {}", displayed_recipes[recipe_index].title),
                                                disabled: recipe_controls_disabled,
                                                on_reorder: move |target_index| {
                                                    if recipe_controls_disabled {
                                                        return;
                                                    }
                                                    let mut reordered_recipes = displayed_recipes.clone();
                                                    reordered_recipes.swap(recipe_index, target_index);
                                                    let ordered_recipe_ids = reordered_recipes
                                                        .iter()
                                                        .map(|recipe| recipe.id.to_string())
                                                        .collect::<Vec<_>>();
                                                    recipe_order_override.set(Some(reordered_recipes));
                                                    recipe_reorder_generation += 1;
                                                    let scheduled_generation = recipe_reorder_generation();
                                                    recipe_reorder_pending.set(true);
                                                    reorder_error.set(None);
                                                    let collection_id = reorder_collection_id.clone();
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    let _ = (
                                                        scheduled_generation,
                                                        ordered_recipe_ids,
                                                        collection_id,
                                                    );
                                                    #[cfg(target_arch = "wasm32")]
                                                    spawn(async move {
                                                        gloo_timers::future::TimeoutFuture::new(
                                                            RECIPE_REORDER_DEBOUNCE_MS,
                                                        )
                                                        .await;
                                                        if !should_persist_reorder_generation(
                                                            scheduled_generation,
                                                            recipe_reorder_generation(),
                                                        ) {
                                                            return;
                                                        }
                                                        recipe_save_in_flight.set(true);
                                                        match reorder_collection_recipes(
                                                            collection_id,
                                                            ordered_recipe_ids,
                                                        )
                                                        .await
                                                        {
                                                            Ok(()) => {
                                                                if should_persist_reorder_generation(
                                                                    scheduled_generation,
                                                                    recipe_reorder_generation(),
                                                                ) {
                                                                    recipe_reorder_pending.set(false);
                                                                    recipe_save_in_flight.set(false);
                                                                }
                                                            }
                                                            Err(error) => {
                                                                if should_persist_reorder_generation(
                                                                    scheduled_generation,
                                                                    recipe_reorder_generation(),
                                                                ) {
                                                                    recipe_reorder_pending.set(false);
                                                                    recipe_save_in_flight.set(false);
                                                                    recipe_order_override.set(None);
                                                                    child_order_override.set(None);
                                                                    additional_recipes.set(Vec::new());
                                                                    reorder_error.set(Some(error.to_string()));
                                                                    contents.restart();
                                                                }
                                                            }
                                                        }
                                                    });
                                                },
                                            }
                                            Button {
                                                variant: ButtonVariant::Danger,
                                                disabled: membership_controls_disabled,
                                                onclick: move |_| {
                                                    if membership_mutation_controls_disabled(loading_more(), removing_recipes.read().len(), contents.pending())
                                                        || folder_save_in_flight()
                                                        || recipe_reorder_pending()
                                                        || recipe_save_in_flight()
                                                    {
                                                        return;
                                                    }
                                                    removing_recipes.write().insert(recipe_id);
                                                    removal_error.set(None);
                                                    let collection_id = removal_collection_id.clone();
                                                    let recipe_id_string = recipe_id_string.clone();
                                                    spawn(async move {
                                                        match remove_recipe_from_collection(collection_id, recipe_id_string).await {
                                                            Ok(()) => {
                                                                recipe_reorder_generation += 1;
                                                                recipe_reorder_pending.set(false);
                                                                recipe_save_in_flight.set(false);
                                                                recipe_order_override.set(None);
                                                                child_order_override.set(None);
                                                                additional_recipes.set(Vec::new());
                                                                contents.restart();
                                                            }
                                                            Err(error) => removal_error.set(Some(error.to_string())),
                                                        }
                                                        removing_recipes.write().remove(&recipe_id);
                                                    });
                                                },
                                                if removing_recipes.read().contains(&recipe_id) {
                                                    "Removing…"
                                                } else {
                                                    "Remove"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } }
                    if collection_contents.can_edit {
                        if !complete_recipe_list && collection_contents.recipes.total_count > 1 {
                            p {
                                color: "var(--text-secondary)",
                                "Load all recipes to enable reordering."
                            }
                        } else if recipe_reorder_pending() {
                            p { role: "status", color: "var(--text-secondary)", "Saving recipe order…" }
                        }
                        if let Some(error) = removal_error() {
                            p {
                                role: "alert",
                                color: "var(--error)",
                                "Could not remove recipe from this collection: {error}"
                            }
                        }
                    }
                    if let Some(error) = recipe_page_error() { p { role: "alert", color: "var(--error)", "Could not load more recipes: {error}" } }
                    if let Some((offset, limit)) = next_recipe_page_request(collection_contents.recipes.recipes.len() + additional_recipes().len(), collection_contents.recipes.total_count) {
                        Button { variant: ButtonVariant::Secondary, disabled: pagination_disabled, onclick: move |_| {
                            if loading_more()
                                || contents.pending()
                                || (can_edit
                                    && (folder_save_in_flight()
                                        || recipe_reorder_pending()
                                        || recipe_save_in_flight()
                                        || !removing_recipes.read().is_empty())) {
                                return;
                            }
                            loading_more.set(true); recipe_page_error.set(None); let id = collection_id.clone();
                            let initial_recipe_ids = collection_contents.recipes.recipes.iter().map(|recipe| recipe.id).collect::<std::collections::HashSet<_>>();
                            let expected_total_count = collection_contents.recipes.total_count;
                            let loaded_count_before = collection_contents.recipes.recipes.len() + additional_recipes().len();
                            spawn(async move {
                                match get_collection_recipes(id, offset, limit).await {
                                    Ok(page) => {
                                        let response_total_count = page.total_count;
                                        let response_recipe_count = page.recipes.len();
                                        let response_has_more = page.has_more;
                                        let mut loaded_recipe_ids = initial_recipe_ids;
                                        loaded_recipe_ids.extend(additional_recipes().iter().map(|recipe| recipe.id));
                                        let unique_new_recipes = page
                                            .recipes
                                            .into_iter()
                                            .filter(|candidate| loaded_recipe_ids.insert(candidate.id))
                                            .collect::<Vec<_>>();

                                        if collection_recipe_page_is_stale(
                                            expected_total_count,
                                            loaded_count_before,
                                            response_total_count,
                                            response_recipe_count,
                                            unique_new_recipes.len(),
                                            response_has_more,
                                        ) {
                                            recipe_reorder_generation += 1;
                                            recipe_reorder_pending.set(false);
                                            recipe_save_in_flight.set(false);
                                            recipe_order_override.set(None);
                                            child_order_override.set(None);
                                            additional_recipes.set(Vec::new());
                                            recipe_page_error.set(Some(
                                                "Collection contents changed while loading more recipes. The collection was refreshed."
                                                    .to_string(),
                                            ));
                                            contents.restart();
                                        } else {
                                            additional_recipes.write().extend(unique_new_recipes);
                                        }
                                    }
                                    Err(error) => recipe_page_error.set(Some(error.to_string())),
                                }
                                loading_more.set(false);
                            });
                        }, if loading_more() { "Loading…" } else { "Load more" } }
                    }
                    if collection_contents.can_edit && delete_modal_open() {
                        div {
                            class: "modal-overlay",
                            onclick: move |_| {
                                if !deleting() {
                                    delete_modal_open.set(false);
                                    delete_confirmation_input.set(String::new());
                                    #[cfg(target_arch = "wasm32")]
                                    spawn(async move {
                                        let _ = document::eval(CLOSE_DELETE_DIALOG_KEYBOARD_LIFECYCLE).await;
                                    });
                                }
                            },
                            div {
                                id: "delete-collection-dialog",
                                class: "modal-card",
                                role: "dialog",
                                aria_modal: "true",
                                aria_labelledby: "delete-collection-title",
                                tabindex: "-1",
                                "data-deleting": if deleting() { "true" } else { "false" },
                                onclick: move |event| event.stop_propagation(),
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-md)",
                                h2 { id: "delete-collection-title", "Delete {collection_name}?" }
                                p {
                                    font_size: "14px",
                                    color: "var(--text-secondary)",
                                    "{RECURSIVE_DELETE_WARNING}"
                                }
                                p {
                                    font_size: "14px",
                                    color: "var(--text-secondary)",
                                    "Type "
                                    code { "{delete_confirmation_phrase}" }
                                    " to confirm."
                                }
                                fieldset {
                                    disabled: deleting(),
                                    border: "0",
                                    padding: "0",
                                    margin: "0",
                                    label {
                                        display: "flex",
                                        flex_direction: "column",
                                        gap: "var(--space-xs)",
                                        span {
                                            font_size: "14px",
                                            font_weight: "600",
                                            color: "var(--text-secondary)",
                                            "Confirmation phrase"
                                        }
                                        Input {
                                            value: delete_confirmation_input(),
                                            placeholder: delete_confirmation_phrase.clone(),
                                            oninput: move |value| {
                                                delete_confirmation_input.set(value);
                                                delete_error.set(None);
                                            },
                                        }
                                    }
                                }
                                if let Some(error) = delete_error() {
                                    p { role: "alert", color: "var(--error)", "{error}" }
                                }
                                div {
                                    display: "flex",
                                    justify_content: "flex-end",
                                    gap: "var(--space-sm)",
                                    span {
                                        id: "delete-collection-cancel",
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            disabled: deleting(),
                                            onclick: move |_| {
                                                delete_modal_open.set(false);
                                                delete_confirmation_input.set(String::new());
                                                #[cfg(target_arch = "wasm32")]
                                                spawn(async move {
                                                    let _ = document::eval(CLOSE_DELETE_DIALOG_KEYBOARD_LIFECYCLE).await;
                                                });
                                            },
                                            "Cancel"
                                        }
                                    }
                                    Button {
                                        variant: ButtonVariant::Danger,
                                        disabled: deleting() || !delete_confirmation_matches,
                                        onclick: move |_| {
                                            if deleting()
                                                || !collection_delete_confirmation_matches(
                                                    &delete_confirmation_input(),
                                                    &collection_name,
                                                )
                                            {
                                                return;
                                            }

                                            deleting.set(true);
                                            delete_error.set(None);
                                            let id = delete_collection_id.clone();
                                            spawn(async move {
                                                match delete_collection(id).await {
                                                    Ok(()) => {
                                                        #[cfg(target_arch = "wasm32")]
                                                        let _ = document::eval(
                                                            ABORT_DELETE_DIALOG_KEYBOARD_LIFECYCLE,
                                                        )
                                                        .await;
                                                        navigator.push(Route::Dashboard {});
                                                    }
                                                    Err(error) => {
                                                        delete_error.set(Some(error.to_string()));
                                                        deleting.set(false);
                                                    }
                                                }
                                            });
                                        },
                                        if deleting() { "Deleting…" } else { "Delete collection" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } }
}

#[cfg(test)]
mod tests {
    use super::{
        can_reorder_complete_recipe_list, collection_delete_confirmation_matches,
        collection_delete_confirmation_phrase, collection_load_error_copy,
        collection_recipe_page_is_stale, membership_mutation_controls_disabled,
        next_recipe_page_request, should_persist_reorder_generation, RECIPE_REORDER_DEBOUNCE_MS,
        RECURSIVE_DELETE_WARNING,
    };
    use dioxus::prelude::ServerFnError;

    #[test]
    fn collection_access_failures_do_not_reveal_authentication_or_existence() {
        let unauthenticated = ServerFnError::new("Not authenticated");
        let missing = ServerFnError::new(
            "error running server function: Collection not found (details: None)",
        );

        let unauthenticated_copy = collection_load_error_copy(&unauthenticated);
        let missing_copy = collection_load_error_copy(&missing);

        assert_eq!(unauthenticated_copy, missing_copy);
        assert_eq!(unauthenticated_copy.0, "Collection not found");
        assert_eq!(
            unauthenticated_copy.1,
            "This collection could not be found."
        );
        assert!(!unauthenticated_copy.1.contains("server function"));
    }

    #[test]
    fn recipe_pagination_requests_every_remaining_page() {
        assert_eq!(next_recipe_page_request(12, 30), Some((12, 18)));
        assert_eq!(next_recipe_page_request(30, 30), None);
        assert_eq!(next_recipe_page_request(0, 0), None);
    }

    #[test]
    fn collection_recipe_cards_offer_owner_gated_removal() {
        let source = include_str!("collection_detail.rs");
        assert!(source.contains("remove_recipe_from_collection"));
        assert!(source.contains("\"Remove\""));
    }

    #[test]
    fn pagination_and_removal_are_mutually_exclusive() {
        assert!(!membership_mutation_controls_disabled(false, 0, false));
        assert!(membership_mutation_controls_disabled(true, 0, false));
        assert!(membership_mutation_controls_disabled(false, 1, false));
        assert!(membership_mutation_controls_disabled(false, 0, true));
    }

    #[test]
    fn recipe_reorder_requires_the_complete_membership_list() {
        assert!(!can_reorder_complete_recipe_list(12, 30));
        assert!(can_reorder_complete_recipe_list(30, 30));
        assert!(can_reorder_complete_recipe_list(0, 0));
    }

    #[test]
    fn only_the_latest_debounced_recipe_order_is_persisted() {
        let scheduled_generations = [1, 2, 3];
        let persisted_generations = scheduled_generations
            .into_iter()
            .filter(|scheduled| should_persist_reorder_generation(*scheduled, 3))
            .collect::<Vec<_>>();

        assert_eq!(RECIPE_REORDER_DEBOUNCE_MS, 500);
        assert_eq!(persisted_generations, vec![3]);
    }

    #[test]
    fn stale_or_non_progressing_recipe_pages_trigger_a_reload() {
        assert!(!collection_recipe_page_is_stale(30, 12, 30, 18, 18, false,));
        assert!(!collection_recipe_page_is_stale(
            200, 12, 200, 100, 100, true,
        ));
        assert!(collection_recipe_page_is_stale(30, 12, 31, 18, 18, true,));
        assert!(collection_recipe_page_is_stale(30, 12, 30, 18, 17, false,));
        assert!(collection_recipe_page_is_stale(30, 12, 30, 0, 0, true,));
        assert!(collection_recipe_page_is_stale(30, 12, 30, 18, 18, true,));
    }

    #[test]
    fn collection_detail_uses_owner_scoped_reorder_endpoints() {
        let source = include_str!("collection_detail.rs");
        assert!(source.contains("reorder_child_collections"));
        assert!(source.contains("reorder_collection_recipes"));
        assert!(source.contains("500"));
    }

    #[test]
    fn recursive_delete_confirmation_explains_scope_and_recipe_safety() {
        assert!(RECURSIVE_DELETE_WARNING.contains("nested collections"));
        assert!(RECURSIVE_DELETE_WARNING.contains("memberships"));
        assert!(RECURSIVE_DELETE_WARNING.contains("recipes will remain"));
    }

    #[test]
    fn recursive_delete_confirmation_includes_the_exact_collection_name() {
        assert_eq!(
            collection_delete_confirmation_phrase("Weekend Favorites"),
            "delete Weekend Favorites"
        );
    }

    #[test]
    fn recursive_delete_requires_an_exact_case_sensitive_phrase() {
        let collection_name = "Weekend Favorites";

        assert!(collection_delete_confirmation_matches(
            "delete Weekend Favorites",
            collection_name,
        ));
        assert!(!collection_delete_confirmation_matches(
            "Delete Weekend Favorites",
            collection_name,
        ));
        assert!(!collection_delete_confirmation_matches(
            "delete weekend favorites",
            collection_name,
        ));
        assert!(!collection_delete_confirmation_matches(
            "delete Weekend Favorites ",
            collection_name,
        ));
        assert!(!collection_delete_confirmation_matches(
            "delete Weekend",
            collection_name,
        ));
    }

    #[test]
    fn recursive_delete_modal_manages_keyboard_focus_and_has_a_persistent_label() {
        let source = include_str!("collection_detail.rs");

        assert!(source.contains("delete-collection-dialog"));
        assert!(source.contains("delete-collection-trigger"));
        assert!(source.contains("delete-collection-cancel"));
        assert!(source.contains("AbortController"));
        assert!(source.contains("event.key === 'Tab'"));
        assert!(source.contains("event.key === 'Escape'"));
        assert!(source.contains("querySelector('input')?.focus()"));
        assert!(source.contains("label {"));
    }

    #[test]
    fn deep_breadcrumbs_wrap_without_overflowing_the_collection_page() {
        let source = include_str!("collection_detail.rs");
        assert!(source.contains("aria_label: \"Breadcrumbs\""));
        assert!(source.contains("flex_wrap: \"wrap\""));
        assert!(source.contains("overflow_wrap: \"anywhere\""));
    }

    #[test]
    fn shared_collection_detail_is_public_and_owner_controls_are_capability_gated() {
        let source = include_str!("collection_detail.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let link_api_source = source
            .split("fn browser_api_error")
            .nth(1)
            .unwrap()
            .split("#[component]")
            .next()
            .unwrap();
        assert!(!source.contains("AuthRequired"));
        assert!(!source.contains("if auth.is_authenticated"));
        assert!(source.contains("collection_contents.can_edit"));
        assert!(source.contains("CollectionVisibilityBadge"));
        assert!(source.contains("copy_collection_link"));
        assert!(source.contains("\"Copy link\""));
        assert!(source.contains("share_collection_link"));
        assert!(link_api_source.contains("web_sys::ShareData"));
        assert!(link_api_source.contains("navigator.share_with_data"));
        assert!(link_api_source.contains("navigator.clipboard().write_text"));
        assert!(!link_api_source.contains("document::eval"));
        assert!(!link_api_source.contains("navigator.share({"));
        assert!(!link_api_source.contains("navigator.clipboard.writeText"));
    }
}
