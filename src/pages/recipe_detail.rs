use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::base::{
    Card, ForkAttribution, LoadingSpinner, PageHeader, ReconstructedVersion, VersionDiff,
    VersionSummary, VersionTimeline,
};
use crate::components::AuthRequired;

/// Active tab for the recipe detail view.
#[derive(Debug, Clone, PartialEq)]
enum RecipeTab {
    Details,
    History,
}

/// Recipe data for the Details tab.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
struct RecipeData {
    title: String,
    description: Option<String>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<serde_json::Value>,
    steps: Option<serde_json::Value>,
}

/// Single recipe view with details and version history tabs.
#[component]
pub fn RecipeDetail(id: Uuid) -> Element {
    let mut active_tab = use_signal(|| RecipeTab::Details);
    let versions = use_signal(Vec::<VersionSummary>::new);
    let loading_versions = use_signal(|| false);
    let selected_version = use_signal(|| Option::<i32>::None);
    let reconstructed = use_signal(|| Option::<ReconstructedVersion>::None);
    let loading_reconstruct = use_signal(|| false);
    let reconstruct_error = use_signal(|| Option::<String>::None);
    let mut restoring_version = use_signal(|| Option::<i32>::None);

    // Recipe data for Details tab
    let recipe = use_signal(|| Option::<RecipeData>::None);
    let loading_recipe = use_signal(|| true);

    // Fork attribution
    let fork_info = use_signal(|| Option::<(Uuid, String, Option<String>)>::None);
    let is_forking = use_signal(|| false);

    // Load fork attribution on mount.
    use_effect(move || {
        let mut info = fork_info;
        let id = id;
        spawn(async move {
            let res =
                gloo_net::http::Request::get(&format!("/api/recipes/{}/fork_info", id)).send().await;
            if let Ok(response) = res {
                if response.ok() {
                    let body = response.text().await.unwrap_or_default();
                    if let Ok(data) =
                        serde_json::from_str::<(Uuid, String, Option<String>)>(&body)
                    {
                        info.set(Some(data));
                    }
                }
            }
        });
    });

    // Load recipe data for Details tab on mount.
    use_effect(move || {
        let mut r = recipe;
        let mut l = loading_recipe;
        let recipe_id = id;
        spawn(async move {
            l.set(true);
            let res =
                gloo_net::http::Request::get(&format!("/api/recipes/{recipe_id}")).send().await;
            if let Ok(response) = res {
                if response.ok() {
                    let body = response.text().await.unwrap_or_default();
                    if let Ok(data) = serde_json::from_str::<RecipeData>(&body) {
                        r.set(Some(data));
                    }
                }
            }
            l.set(false);
        });
    });

    // Memoized handler for forking a recipe.
    let on_fork = use_callback(
        move |_| {
            let mut is_forking = is_forking;
            let id = id;
            spawn(async move {
                is_forking.set(true);
                let url = format!("/api/recipes/{}/fork", id);
                let request = match gloo_net::http::Request::post(&url)
                    .json(&serde_json::json!({}))
                {
                    Ok(req) => req,
                    Err(e) => {
                        let _ = web_sys::window()
                            .unwrap()
                            .alert_with_message(&format!("Fork failed: {}", e));
                        is_forking.set(false);
                        return;
                    }
                };
                let response = match request.send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        let _ = web_sys::window()
                            .unwrap()
                            .alert_with_message(&format!("Fork failed: {}", e));
                        is_forking.set(false);
                        return;
                    }
                };
                is_forking.set(false);

                if !response.ok() {
                    let _ = web_sys::window()
                        .unwrap()
                        .alert_with_message(&format!(
                            "Fork failed: server error {}",
                            response.status()
                        ));
                    return;
                }

                let body = response.text().await.unwrap_or_default();
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(new_id) = data.get("new_recipe_id").and_then(|v| v.as_str()) {
                        // Redirect to the new recipe's edit page
                        let _ = web_sys::window()
                            .unwrap()
                            .location()
                            .set_href(&format!("/recipes/{}/edit", new_id));
                    }
                }
            });
        },
    );

    // Load versions when History tab is activated.
    // The effect auto-subscribes to active_tab.read() and re-runs only when the tab changes.
    // We do NOT read loading_versions inside the spawned task to avoid creating a subscription
    // that would cause the effect to re-run when loading_versions changes (infinite polling loop).
    #[allow(unused_assignments)]
    use_effect(move || {
        let tab = active_tab.read().clone();
        if tab == RecipeTab::History {
            let mut v = versions;
            let mut l = loading_versions;
            spawn(async move {
                l.set(true);
                let res = gloo_net::http::Request::get(&format!("/api/recipes/{id}/versions"))
                    .send()
                    .await;
                if let Ok(response) = res {
                    if response.ok() {
                        let body = response.text().await.unwrap_or_default();
                        if let Ok(ver) = serde_json::from_str::<Vec<VersionSummary>>(&body) {
                            v.set(ver);
                        }
                    }
                }
                l.set(false);
            });
        }
    });

    // Memoized handler for selecting a version from the timeline.
    let on_version_select = use_callback({
        let mut sel_ver = selected_version;
        let rec = reconstructed;
        let load_rec = loading_reconstruct;
        let rec_err = reconstruct_error;
        move |version_number: i32| {
            sel_ver.set(Some(version_number));
            let mut rec = rec;
            let mut load = load_rec;
            let mut err = rec_err;
            let id = id;
            spawn(async move {
                load.set(true);
                err.set(None);
                let res = gloo_net::http::Request::get(&format!(
                    "/api/recipes/{id}/versions/{version_number}/reconstruct"
                ))
                .send()
                .await;
                match res {
                    Ok(response) if response.ok() => {
                        let body = response.text().await.unwrap_or_default();
                        if let Ok(v) = serde_json::from_str::<ReconstructedVersion>(&body) {
                            rec.set(Some(v));
                        } else {
                            err.set(Some("Failed to parse version data.".to_string()));
                        }
                    }
                    Ok(response) => {
                        err.set(Some(format!("Server error: {}", response.status())));
                    }
                    Err(e) => {
                        err.set(Some(format!("Network error: {}", e)));
                    }
                }
                load.set(false);
            });
        }
    });

    // Memoized handler for restoring a version from the timeline.
    let on_restore = use_callback(
        move |version_number: i32| {
            let confirmed = web_sys::window()
                .unwrap()
                .confirm_with_message(&format!(
                    "Restore version {}? This will create a new version with the data from version {}.",
                    version_number, version_number
                ))
                .unwrap_or(false);
            if !confirmed {
                return;
            }

            // Set loading state to prevent duplicate restores
            restoring_version.set(Some(version_number));

            let id = id;
            let mut v = versions;
            let mut l = loading_versions;
            let mut r = restoring_version;
            spawn(async move {
                let url = format!("/api/recipes/{}/versions/{}/restore", id, version_number);
                let response = match gloo_net::http::Request::post(&url).send().await {
                    Ok(resp) => resp,
                    Err(e) => {
                        let _ = web_sys::window()
                            .unwrap()
                            .alert_with_message(&format!("Restore failed: {}", e));
                        r.set(None);
                        return;
                    }
                };
                if !response.ok() {
                    let _ = web_sys::window()
                        .unwrap()
                        .alert_with_message(&format!("Restore failed: server error {}", response.status()));
                    r.set(None);
                    return;
                }

                // Reload versions list
                v.set(Vec::new());
                l.set(true);
                let versions_url = format!("/api/recipes/{}/versions", id);
                if let Ok(resp) = gloo_net::http::Request::get(&versions_url).send().await {
                    if resp.ok() {
                        let body = resp.text().await.unwrap_or_default();
                        if let Ok(new_versions) =
                            serde_json::from_str::<Vec<VersionSummary>>(&body)
                        {
                            v.set(new_versions);
                        }
                    }
                }
                l.set(false);
                r.set(None);
            });
        }
    );

    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "Recipe Details",
                }

                // Fork attribution bar
                if let Some((orig_id, owner_name, message)) = fork_info.read().clone() {
                    ForkAttribution {
                        original_recipe_id: orig_id,
                        original_owner_name: owner_name,
                        message: message,
                        is_variant: false,
                    }
                }

                // Tab navigation
                div {
                    class: "flex gap-2 mb-6",
                    button {
                        class: if *active_tab.read() == RecipeTab::Details {
                            "btn btn-primary touch-target"
                        } else {
                            "btn btn-ghost touch-target"
                        },
                        r#"type"#: "button",
                        onclick: move |_| active_tab.set(RecipeTab::Details),
                        "Details"
                    }
                    button {
                        class: if *active_tab.read() == RecipeTab::History {
                            "btn btn-primary touch-target"
                        } else {
                            "btn btn-ghost touch-target"
                        },
                        r#"type"#: "button",
                        onclick: move |_| active_tab.set(RecipeTab::History),
                        "History"
                    }

                    // Fork button (only for authenticated users, not shown on own recipes)
                    button {
                        class: "btn btn-ghost touch-target ml-auto",
                        r#"type"#: "button",
                        disabled: *is_forking.read(),
                        onclick: on_fork,
                        if *is_forking.read() {
                            LoadingSpinner {}
                        } else {
                            "Fork"
                        }
                    }
                }

                // Details tab
                if *active_tab.read() == RecipeTab::Details {
                    div {
                        if *loading_recipe.read() {
                            div {
                                display: "flex",
                                flex_direction: "column",
                                align_items: "center",
                                justify_content: "center",
                                min_height: "300px",
                                Card {
                                    LoadingSpinner {}
                                    p {
                                        margin_top: "var(--space-md)",
                                        color: "var(--text-secondary)",
                                        "Loading recipe..."
                                    }
                                }
                            }
                        } else if let Some(ref r) = *recipe.read() {
                            div {
                                class: "space-y-6",
                                // Title and description
                                Card {
                                    div {
                                        display: "flex",
                                        flex_direction: "column",
                                        gap: "var(--space-sm)",
                                        h2 {
                                            font_size: "24px",
                                            font_weight: "700",
                                            color: "var(--text-primary)",
                                            margin: "0",
                                            {r.title.clone()}
                                        }
                                        if let Some(ref desc) = r.description {
                                            if !desc.is_empty() {
                                                p {
                                                    font_size: "16px",
                                                    color: "var(--text-secondary)",
                                                    margin: "0",
                                                    line_height: "1.6",
                                                    {desc.clone()}
                                                }
                                            }
                                        }
                                    }
                                }

                                // Time and servings grid
                                div {
                                    class: "grid grid-cols-2 sm:grid-cols-4 gap-4",
                                    if let Some(prep) = r.prep_time_min {
                                        Card {
                                            div {
                                                display: "flex",
                                                flex_direction: "column",
                                                gap: "4px",
                                                p {
                                                    font_size: "12px",
                                                    font_weight: "600",
                                                    color: "var(--text-secondary)",
                                                    text_transform: "uppercase",
                                                    letter_spacing: "0.05em",
                                                    margin: "0",
                                                    "Prep Time"
                                                }
                                                p {
                                                    font_size: "18px",
                                                    font_weight: "600",
                                                    color: "var(--text-primary)",
                                                    margin: "0",
                                                    "{prep} min"
                                                }
                                            }
                                        }
                                    }
                                    if let Some(cook) = r.cook_time_min {
                                        Card {
                                            div {
                                                display: "flex",
                                                flex_direction: "column",
                                                gap: "4px",
                                                p {
                                                    font_size: "12px",
                                                    font_weight: "600",
                                                    color: "var(--text-secondary)",
                                                    text_transform: "uppercase",
                                                    letter_spacing: "0.05em",
                                                    margin: "0",
                                                    "Cook Time"
                                                }
                                                p {
                                                    font_size: "18px",
                                                    font_weight: "600",
                                                    color: "var(--text-primary)",
                                                    margin: "0",
                                                    "{cook} min"
                                                }
                                            }
                                        }
                                    }
                                    if let Some(total) = r.total_time_min {
                                        Card {
                                            div {
                                                display: "flex",
                                                flex_direction: "column",
                                                gap: "4px",
                                                p {
                                                    font_size: "12px",
                                                    font_weight: "600",
                                                    color: "var(--text-secondary)",
                                                    text_transform: "uppercase",
                                                    letter_spacing: "0.05em",
                                                    margin: "0",
                                                    "Total Time"
                                                }
                                                p {
                                                    font_size: "18px",
                                                    font_weight: "600",
                                                    color: "var(--text-primary)",
                                                    margin: "0",
                                                    "{total} min"
                                                }
                                            }
                                        }
                                    }
                                    if let Some(servings) = r.servings {
                                        Card {
                                            div {
                                                display: "flex",
                                                flex_direction: "column",
                                                gap: "4px",
                                                p {
                                                    font_size: "12px",
                                                    font_weight: "600",
                                                    color: "var(--text-secondary)",
                                                    text_transform: "uppercase",
                                                    letter_spacing: "0.05em",
                                                    margin: "0",
                                                    "Servings"
                                                }
                                                p {
                                                    font_size: "18px",
                                                    font_weight: "600",
                                                    color: "var(--text-primary)",
                                                    margin: "0",
                                                    "{servings}"
                                                }
                                            }
                                        }
                                    }
                                }

                                // Ingredients
                                if let Some(ref ingredients) = r.ingredients {
                                    if let Some(arr) = ingredients.as_array() {
                                        if !arr.is_empty() {
                                            Card {
                                                div {
                                                    display: "flex",
                                                    flex_direction: "column",
                                                    gap: "var(--space-sm)",
                                                    h3 {
                                                        font_size: "18px",
                                                        font_weight: "600",
                                                        color: "var(--text-primary)",
                                                        margin: "0",
                                                        "Ingredients"
                                                    }
                                                    ul {
                                                        class: "list-disc list-inside space-y-1",
                                                        padding_left: "var(--space-sm)",
                                                        for ingredient in arr.iter() {
                                                            li {
                                                                font_size: "15px",
                                                                color: "var(--text-primary)",
                                                                line_height: "1.5",
                                                                "{ingredient_to_string(ingredient)}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Steps
                                if let Some(ref steps) = r.steps {
                                    if let Some(arr) = steps.as_array() {
                                        if !arr.is_empty() {
                                            Card {
                                                div {
                                                    display: "flex",
                                                    flex_direction: "column",
                                                    gap: "var(--space-sm)",
                                                    h3 {
                                                        font_size: "18px",
                                                        font_weight: "600",
                                                        color: "var(--text-primary)",
                                                        margin: "0",
                                                        "Steps"
                                                    }
                                                    ol {
                                                        class: "list-decimal list-inside space-y-3",
                                                        padding_left: "var(--space-sm)",
                                                        for (i, step) in arr.iter().enumerate() {
                                                            li {
                                                                font_size: "15px",
                                                                color: "var(--text-primary)",
                                                                line_height: "1.6",
                                                                p {
                                                                    font_weight: "600",
                                                                    "Step {i + 1}:"
                                                                }
                                                                p {
                                                                    "{step_to_string(step)}"
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
                        } else {
                            div {
                                display: "flex",
                                flex_direction: "column",
                                align_items: "center",
                                justify_content: "center",
                                min_height: "300px",
                                Card {
                                    p {
                                        color: "var(--text-secondary)",
                                        "Recipe not found."
                                    }
                                }
                            }
                        }
                    }
                }

                // History tab
                if *active_tab.read() == RecipeTab::History {
                    div {
                        class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                        // Timeline panel (left side)
                        div {
                            class: "lg:col-span-1",
                            Card {
                                div {
                                    class: "p-4",
                                    h3 {
                                        class: "text-lg font-semibold mb-4",
                                        "Version History"
                                    }
                                    VersionTimeline {
                                        versions: versions.read().clone(),
                                        loading: *loading_versions.read(),
                                        on_version_select: on_version_select,
                                        selected_version: *selected_version.read(),
                                        on_restore: on_restore,
                                        restoring_version: *restoring_version.read(),
                                    }
                                }
                            }
                        }

                        // Diff panel (right side)
                        div {
                            class: "lg:col-span-2",
                            Card {
                                div {
                                    class: "p-4",
                                    VersionDiff {
                                        version: reconstructed.read().clone(),
                                        loading: *loading_reconstruct.read(),
                                        error: reconstruct_error.read().clone(),
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

/// Convert an ingredient JSON value to a display string.
fn ingredient_to_string(val: &serde_json::Value) -> String {
    if let s @ serde_json::Value::String(_) = val {
        s.as_str().unwrap_or("").to_string()
    } else if let Some(obj) = val.as_object() {
        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
            if let Some(amount) = obj.get("amount").and_then(|v| v.as_str()) {
                format!("{} - {}", amount, name)
            } else {
                name.to_string()
            }
        } else {
            serde_json::to_string(val).unwrap_or_default()
        }
    } else {
        serde_json::to_string(val).unwrap_or_default()
    }
}

/// Convert a step JSON value to a display string.
fn step_to_string(val: &serde_json::Value) -> String {
    if let s @ serde_json::Value::String(_) = val {
        s.as_str().unwrap_or("").to_string()
    } else if let Some(obj) = val.as_object() {
        if let Some(instruction) = obj.get("instruction").and_then(|v| v.as_str()) {
            instruction.to_string()
        } else {
            serde_json::to_string(val).unwrap_or_default()
        }
    } else {
        serde_json::to_string(val).unwrap_or_default()
    }
}
