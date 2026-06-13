use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use gloo_timers::callback::Timeout;
use uuid::Uuid;

use crate::components::base::{Button, ButtonVariant, Input, PageHeader};
use crate::components::AuthRequired;
use crate::Route;

/// Debounce interval for auto-save (milliseconds).
const AUTO_SAVE_DELAY_MS: u32 = 2_000;

/// Edit existing recipe or draft with auto-save and publish.
#[component]
pub fn RecipeEdit(id: Uuid) -> Element {
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut prep_time = use_signal(String::new);
    let mut cook_time = use_signal(String::new);
    let mut total_time = use_signal(String::new);
    let mut servings = use_signal(String::new);
    let mut ingredients_text = use_signal(String::new);
    let mut steps_text = use_signal(String::new);
    let mut version_notes = use_signal(String::new);

    // State
    let is_draft = use_signal(|| true);
    let is_loading = use_signal(|| true);
    let is_saving = use_signal(|| false);
    let save_status = use_signal(String::new);
    let auto_save_timer = use_signal(|| Option::<Rc<RefCell<Option<Timeout>>>>::None);
    let load_error = use_signal(|| Option::<String>::None);

    // Load recipe on mount
    use_effect(move || {
        let mut title_sig = title;
        let mut desc_sig = description;
        let mut prep_sig = prep_time;
        let mut cook_sig = cook_time;
        let mut total_sig = total_time;
        let mut servings_sig = servings;
        let mut ing_sig = ingredients_text;
        let mut steps_sig = steps_text;
        let mut is_draft_sig = is_draft;
        let mut is_loading_sig = is_loading;
        let mut error_sig = load_error;
        let id_val = id;

        spawn(async move {
            let res =
                gloo_net::http::Request::get(&format!("/api/recipes/{id_val}/versions"))
                    .send()
                    .await;

            match res {
                Ok(resp) if resp.ok() => {
                    let body = resp.text().await.unwrap_or_default();
                    // Get the latest version from the versions list
                    if let Ok(versions) =
                        serde_json::from_str::<Vec<VersionSummaryApiResponse>>(&body)
                    {
                        if let Some(latest) = versions.iter().find(|v| v.is_latest) {
                            title_sig.set(latest.title.clone().unwrap_or_default());
                            // We need the full recipe for all fields — fetch from reconstruct endpoint
                            let rec_res = gloo_net::http::Request::get(&format!(
                                "/api/recipes/{}/versions/{}/reconstruct",
                                id_val, latest.version_number
                            ))
                            .send()
                            .await;

                            if let Ok(rec_resp) = rec_res {
                                if rec_resp.ok() {
                                    let rec_body = rec_resp.text().await.unwrap_or_default();
                                    if let Ok(rec) =
                                        serde_json::from_str::<ReconstructedApiResponse>(&rec_body)
                                    {
                                        title_sig.set(rec.title);
                                        desc_sig.set(rec.description.unwrap_or_default());
                                        prep_sig.set(
                                            rec.prep_time_min
                                                .map(|v| v.to_string())
                                                .unwrap_or_default(),
                                        );
                                        cook_sig.set(
                                            rec.cook_time_min
                                                .map(|v| v.to_string())
                                                .unwrap_or_default(),
                                        );
                                        total_sig.set(
                                            rec.total_time_min
                                                .map(|v| v.to_string())
                                                .unwrap_or_default(),
                                        );
                                        servings_sig.set(
                                            rec.servings
                                                .map(|v| v.to_string())
                                                .unwrap_or_default(),
                                        );
                                        ing_sig.set(json_array_to_text(&rec.ingredients));
                                        steps_sig.set(json_array_to_text(&rec.steps));
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(resp) => {
                    error_sig.set(Some(format!("Failed to load recipe: {}", resp.status())));
                }
                Err(e) => {
                    error_sig.set(Some(format!("Failed to load recipe: {}", e)));
                }
            }

            // Also check if this is a draft by fetching the recipe list
            let list_res =
                gloo_net::http::Request::get("/api/recipes?include_drafts=true").send().await;
            if let Ok(list_resp) = list_res {
                if list_resp.ok() {
                    let list_body = list_resp.text().await.unwrap_or_default();
                    if let Ok(list) = serde_json::from_str::<ListRecipesApiResponse>(&list_body) {
                        if let Some(r) = list.recipes.iter().find(|r| r.id == id_val) {
                            is_draft_sig.set(r.is_draft);
                        }
                    }
                }
            }

            is_loading_sig.set(false);
        });
    });

    // Debounced auto-save effect
    use_effect(move || {
        let is_saving = is_saving;
        let save_status = save_status;
        let mut timer = auto_save_timer;
        let t = title;
        let d = description;
        let pt = prep_time;
        let ct = cook_time;
        let tt = total_time;
        let sv = servings;
        let ing = ingredients_text;
        let st = steps_text;
        let id_val = id;

        // Clear any existing timer on re-render
        if let Some(old_timer_rc) = timer.write().take() {
            if let Some(t) = old_timer_rc.borrow_mut().take() {
                t.cancel();
            }
        }

        let new_timer = Timeout::new(AUTO_SAVE_DELAY_MS, move || {
            let title_val = t.read().clone();
            let desc_val = d.read().clone();
            let prep_val: Option<i32> = pt.read().parse().ok();
            let cook_val: Option<i32> = ct.read().parse().ok();
            let total_val: Option<i32> = tt.read().parse().ok();
            let servings_val: Option<i32> = sv.read().parse().ok();
            let ingredients_json = parse_json_array(&ing.read());
            let steps_json = parse_json_array(&st.read());

            if title_val.trim().is_empty() {
                return;
            }

            if *is_saving.read() {
                return;
            }

            let body = serde_json::json!({
                "recipe_id": id_val,
                "title": title_val,
                "description": if desc_val.is_empty() { serde_json::Value::Null } else { serde_json::json!(desc_val) },
                "prep_time_min": prep_val,
                "cook_time_min": cook_val,
                "total_time_min": total_val,
                "servings": servings_val,
                "ingredients": ingredients_json,
                "steps": steps_json,
            });

            let mut saving = is_saving;
            let mut status = save_status;
            let mut timer_sig = timer;

            spawn(async move {
                saving.set(true);
                status.set("Saving draft...".to_string());

                let res = match gloo_net::http::Request::post("/api/recipes/drafts")
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                {
                    Ok(req) => req.send().await,
                    Err(e) => {
                        status.set(format!("Save failed: {}", e));
                        saving.set(false);
                        timer_sig.write().take();
                        return;
                    }
                };

                match res {
                    Ok(resp) if resp.ok() => {
                        status.set("Draft saved".to_string());
                    }
                    Ok(resp) => {
                        status.set(format!("Save failed: {}", resp.status()));
                    }
                    Err(e) => {
                        status.set(format!("Save failed: {}", e));
                    }
                }
                saving.set(false);
                timer_sig.write().take();
            });
        });

        let rc_timer = Rc::new(RefCell::new(Some(new_timer)));
        timer.set(Some(rc_timer.clone()));
    });

    // Manual save handler
    let on_save_draft = use_callback({
        let mut timer = auto_save_timer;
        move |_| {
            let mut status = save_status;
            let mut is_saving_sig = is_saving;
            let t = title.read().clone();
            let d = description.read().clone();
            let pt: Option<i32> = prep_time.read().parse().ok();
            let ct: Option<i32> = cook_time.read().parse().ok();
            let tt: Option<i32> = total_time.read().parse().ok();
            let sv: Option<i32> = servings.read().parse().ok();
            let ing = parse_json_array(&ingredients_text.read());
            let st = parse_json_array(&steps_text.read());
            let id_val = id;

            if t.trim().is_empty() {
                status.set("Title is required".to_string());
                return;
            }

            is_saving_sig.set(true);
            status.set("Saving draft...".to_string());

            let body = serde_json::json!({
                "recipe_id": id_val,
                "title": t,
                "description": if d.is_empty() { serde_json::Value::Null } else { serde_json::json!(d) },
                "prep_time_min": pt,
                "cook_time_min": ct,
                "total_time_min": tt,
                "servings": sv,
                "ingredients": ing,
                "steps": st,
            });

            spawn(async move {
                let res = match gloo_net::http::Request::post("/api/recipes/drafts")
                    .header("Content-Type", "application/json")
                    .body(body.to_string())
                {
                    Ok(req) => req.send().await,
                    Err(e) => {
                        status.set(format!("Save failed: {}", e));
                        is_saving_sig.set(false);
                        return;
                    }
                };

                match res {
                    Ok(resp) if resp.ok() => {
                        status.set("Draft saved".to_string());
                    }
                    Ok(resp) => {
                        status.set(format!("Save failed: {}", resp.status()));
                    }
                    Err(e) => {
                        status.set(format!("Save failed: {}", e));
                    }
                }
                is_saving_sig.set(false);
                // Reset auto-save timer after manual save
                timer.write().take();
            });
        }
    });

    // Publish handler
    let on_publish = use_callback(move |_| {
        let mut status = save_status;
        let mut is_saving_sig = is_saving;
        let id_val = id;

        is_saving_sig.set(true);
        status.set("Publishing...".to_string());

        spawn(async move {
            let res =
                gloo_net::http::Request::post(&format!("/api/recipes/{id_val}/publish"))
                    .send()
                    .await;

            match res {
                Ok(resp) if resp.ok() => {
                    let _ = web_sys::window()
                        .unwrap()
                        .alert_with_message("Recipe published!");
                    let nav = dioxus::prelude::use_navigator();
                    nav.push(Route::RecipeDetail { id: id_val });
                }
                Ok(resp) => {
                    status.set(format!("Publish failed: {}", resp.status()));
                    is_saving_sig.set(false);
                }
                Err(e) => {
                    status.set(format!("Publish failed: {}", e));
                    is_saving_sig.set(false);
                }
            }
        });
    });

    // Loading state
    if *is_loading.read() {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    PageHeader { title: "Edit Recipe" }
                    div {
                        display: "flex",
                        justify_content: "center",
                        padding: "var(--space-xl)",
                        "Loading..."
                    }
                }
            }
        };
    }

    // Error state
    if let Some(err) = load_error.read().clone() {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    PageHeader { title: "Edit Recipe" }
                    div {
                        color: "var(--error-color)",
                        padding: "var(--space-lg)",
                        {err}
                    }
                }
            }
        };
    }

    let is_disabled = *is_saving.read();
    let current_is_draft = *is_draft.read();

    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "Edit Recipe",
                    action: if current_is_draft {
                        Some(rsx! {
                            span {
                                class: "badge badge-warning",
                                "DRAFT"
                            }
                        })
                    } else { None },
                }

                // Save status
                if !save_status.read().is_empty() {
                    div {
                        class: "save-status",
                        margin_bottom: "var(--space-md)",
                        font_size: "14px",
                        color: "var(--text-secondary)",
                        {save_status.read().clone()}
                    }
                }

                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-md)",
                    max_width: "600px",

                    // Title
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Recipe Name *"
                        }
                        Input {
                            value: title.read().clone(),
                            placeholder: "Recipe name",
                            oninput: move |v| title.set(v),
                        }
                    }

                    // Description
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Description"
                        }
                        textarea {
                            class: "neumo-inset input",
                            placeholder: "Description...",
                            rows: "4",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            background_color: "var(--surface)",
                            outline: "none",
                            resize: "vertical",
                            value: description.read().clone(),
                            oninput: move |evt| description.set(evt.value()),
                        }
                    }

                    // Time fields
                    div {
                        display: "grid",
                        grid_template_columns: "1fr 1fr 1fr",
                        gap: "var(--space-md)",
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Prep Time (min)"
                            }
                            Input {
                                value: prep_time.read().clone(),
                                placeholder: "15",
                                input_type: "number".to_string(),
                                oninput: move |v| prep_time.set(v),
                            }
                        }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Cook Time (min)"
                            }
                            Input {
                                value: cook_time.read().clone(),
                                placeholder: "30",
                                input_type: "number".to_string(),
                                oninput: move |v| cook_time.set(v),
                            }
                        }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Total Time (min)"
                            }
                            Input {
                                value: total_time.read().clone(),
                                placeholder: "45",
                                input_type: "number".to_string(),
                                oninput: move |v| total_time.set(v),
                            }
                        }
                    }

                    // Servings
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Servings"
                        }
                        Input {
                            value: servings.read().clone(),
                            placeholder: "4",
                            input_type: "number".to_string(),
                            oninput: move |v| servings.set(v),
                        }
                    }

                    // Ingredients
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Ingredients (one per line)"
                        }
                        textarea {
                            class: "neumo-inset input",
                            placeholder: "One ingredient per line",
                            rows: "6",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            background_color: "var(--surface)",
                            outline: "none",
                            resize: "vertical",
                            value: ingredients_text.read().clone(),
                            oninput: move |evt| ingredients_text.set(evt.value()),
                        }
                    }

                    // Steps
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Steps (one per line)"
                        }
                        textarea {
                            class: "neumo-inset input",
                            placeholder: "One step per line",
                            rows: "6",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            background_color: "var(--surface)",
                            outline: "none",
                            resize: "vertical",
                            value: steps_text.read().clone(),
                            oninput: move |evt| steps_text.set(evt.value()),
                        }
                    }

                    // Version notes (only shown for published recipes)
                    if !current_is_draft {
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Version Notes"
                            }
                            textarea {
                                class: "neumo-inset input",
                                placeholder: "What changed in this version?",
                                rows: "2",
                                padding: "var(--space-sm) var(--space-md)",
                                font_family: "var(--font-body)",
                                font_size: "14px",
                                color: "var(--text-primary)",
                                background_color: "var(--surface)",
                                outline: "none",
                                resize: "vertical",
                                value: version_notes.read().clone(),
                                oninput: move |evt| version_notes.set(evt.value()),
                            }
                        }
                    }

                    // Action buttons
                    div {
                        display: "flex",
                        gap: "var(--space-md)",
                        margin_top: "var(--space-md)",

                        if current_is_draft {
                            Button {
                                variant: ButtonVariant::Primary,
                                disabled: is_disabled,
                                onclick: on_publish,
                                "Publish Recipe"
                            }

                            Button {
                                variant: ButtonVariant::Primary,
                                disabled: is_disabled,
                                onclick: on_save_draft,
                                "Save Draft"
                            }
                        }

                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| {
                                let nav = dioxus::prelude::use_navigator();
                                nav.push(Route::RecipeDetail { id });
                            },
                            "Back"
                        }
                    }
                }
            }
        }
    }
}

/// Parse multiline text into a JSON array of strings.
fn parse_json_array(text: &str) -> serde_json::Value {
    let items: Vec<String> = text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    serde_json::json!(items)
}

/// Convert a JSON array value to multiline text.
fn json_array_to_text(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

// ── API response types ──────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct VersionSummaryApiResponse {
    version_number: i32,
    title: Option<String>,
    is_latest: bool,
}

#[derive(serde::Deserialize)]
struct ReconstructedApiResponse {
    title: String,
    description: Option<String>,
    prep_time_min: Option<i32>,
    cook_time_min: Option<i32>,
    total_time_min: Option<i32>,
    servings: Option<i32>,
    ingredients: Option<serde_json::Value>,
    steps: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct ListRecipesApiResponse {
    recipes: Vec<RecipeListItem>,
}

#[derive(serde::Deserialize)]
struct RecipeListItem {
    id: Uuid,
    is_draft: bool,
}
