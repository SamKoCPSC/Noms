//! Single recipe detail page.
//!
//! Fetches recipe data and tags via server functions, parses the serialized
//! instructions text back into ingredients and steps for display.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;

use crate::api::recipe::{get_recipe, get_recipe_tags};
#[cfg(target_arch = "wasm32")]
use crate::api::recipe::delete_recipe;
use crate::auth::context::use_auth;
use crate::components::base::{Button, ButtonVariant, Card, LoadingSpinner, PageHeader};
use crate::components::AuthRequired;
use crate::types::Recipe;

// ── Parsed instruction types ─────────────────────────────────────────────────

/// A single parsed ingredient.
#[derive(Clone, Debug)]
struct ParsedIngredient {
    amount: String,
    unit: String,
    name: String,
}

/// Result of parsing the serialized instructions text.
#[derive(Clone, Debug, Default)]
struct ParsedInstructions {
    ingredients: Vec<ParsedIngredient>,
    steps: Vec<String>,
    /// Fallback: original text if no structured sections were found.
    raw: Option<String>,
}

// ── Parsing helper ────────────────────────────────────────────────────────────

/// Parse the serialized instructions text back into ingredients and steps.
///
/// Reverses the format produced by `serialize_instructions()` in `recipe_new.rs`:
/// ```text
/// INGREDIENTS:
/// - 2 cups flour
/// - 1 tsp salt
///
/// STEPS:
/// 1. Mix dry ingredients
/// 2. Add wet ingredients
/// ```
fn parse_instructions(text: &str) -> ParsedInstructions {
    let mut result = ParsedInstructions {
        raw: Some(text.to_string()),
        ..Default::default()
    };

    let mut section = ""; // "ingredients" or "steps"

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed == "INGREDIENTS:" {
            section = "ingredients";
            continue;
        }
        if trimmed == "STEPS:" {
            section = "steps";
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        match section {
            "ingredients" => {
                // Parse: "- 2 cups flour" → { amount: "2", unit: "cups", name: "flour" }
                if let Some(rest) = trimmed.strip_prefix('-').map(|s| s.trim()) {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    // Heuristic: first token = amount, last token = name,
                    // everything in between = unit.
                    let amount = parts[0].to_string();
                    let name = parts[parts.len() - 1].to_string();
                    let unit = if parts.len() > 2 {
                        parts[1..parts.len() - 1].join(" ")
                    } else {
                        String::new()
                    };
                    result.ingredients.push(ParsedIngredient { amount, unit, name });
                }
            }
            "steps" => {
                // Parse: "1. Mix dry ingredients" → "Mix dry ingredients"
                // The format is "{number}. {text}\n"
                if let Some(dot_pos) = trimmed.find('.') {
                    let num_part = &trimmed[..dot_pos];
                    let rest = trimmed[dot_pos + 1..].trim();
                    // Verify the prefix is a number
                    if num_part.parse::<u32>().is_ok() && !rest.is_empty() {
                        result.steps.push(rest.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    // If we found nothing structured, keep raw as fallback
    if result.ingredients.is_empty() && result.steps.is_empty() {
        result.raw = Some(text.to_string());
    }

    result
}

// ── Date formatting helper ───────────────────────────────────────────────────

/// Format a UTC datetime as a relative time string (e.g. "3 days ago").
fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);
    if diff.num_days() > 365 {
        format!("{} years ago", diff.num_days() / 365)
    } else if diff.num_days() > 30 {
        format!("{} months ago", diff.num_days() / 30)
    } else if diff.num_days() > 0 {
        format!("{} days ago", diff.num_days())
    } else if diff.num_hours() > 0 {
        format!("{} hours ago", diff.num_hours())
    } else if diff.num_minutes() > 0 {
        format!("{} minutes ago", diff.num_minutes())
    } else {
        "Just now".to_string()
    }
}

// ── Component ────────────────────────────────────────────────────────────────

/// Single recipe detail page.
#[component]
pub fn RecipeDetail(id: String) -> Element {
    // ── Resources ────────────────────────────────────────────────────────
    // Clone id before each use_resource to avoid move issues.
    // The closure is FnMut (called potentially multiple times), so we clone
    // the captured value inside the closure body before the async block.
    let id_for_recipe = id.clone();
    let recipe_resource = use_resource(move || {
        let rid = id_for_recipe.clone();
        async move { get_recipe(rid).await }
    });

    let id_for_tags = id.clone();
    let tags_resource = use_resource(move || {
        let tid = id_for_tags.clone();
        async move { get_recipe_tags(tid).await }
    });

    // ── Delete state ─────────────────────────────────────────────────────
    #[allow(unused_mut)]
    let mut is_deleting = use_signal(|| false);
    #[allow(unused_mut)]
    let mut delete_error = use_signal(|| Option::<String>::None);

    // ── Extract resource states ──────────────────────────────────────────
    let recipe_pending = recipe_resource.pending();
    let tags_pending = tags_resource.pending();
    let any_pending = recipe_pending || tags_pending;

    // Read resource results to avoid borrow issues in rsx!
    let recipe_result: Option<Result<Recipe, ServerFnError>> =
        recipe_resource.read().clone();
    let tags_result: Option<Result<Vec<String>, ServerFnError>> =
        tags_resource.read().clone();

    // ── Derived state ────────────────────────────────────────────────────
    let recipe = recipe_result.as_ref().and_then(|r| r.as_ref().ok());
    let tags = tags_result.as_ref().and_then(|r| r.as_ref().ok());

    // Determine error message (if any)
    let error_message = if let Some(Err(e)) = &recipe_result {
        let msg = match e {
            ServerFnError::ServerError { message, .. } => message.clone(),
            _ => e.to_string(),
        };
        // Check for specific error types
        if msg.contains("Not authenticated") {
            None // AuthRequired handles this
        } else {
            Some(msg)
        }
    } else {
        None
    };

    // ── Delete handler ───────────────────────────────────────────────────
    // Clone id before the move closure (id was already moved into use_resource closures)
    #[allow(unused_variables)]
    let id_for_delete = id.clone();
    let on_delete = move |_| {
        // Clone inside the FnMut closure so it doesn't move out
        #[allow(unused_variables)]
        let id_clone = id_for_delete.clone();
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                // Note: web_sys `confirm(msg)` requires the "confirm" feature.
                // Using no-arg version which shows a generic confirmation dialog.
                let confirmed = window.confirm().unwrap_or(false);
                if confirmed {
                    is_deleting.set(true);
                    delete_error.set(None);
                    spawn(async move {
                        match delete_recipe(id_clone).await {
                            Ok(()) => {
                                if let Some(w) = web_sys::window() {
                                    let _ = w.location().set_href("/dashboard");
                                }
                            }
                            Err(e) => {
                                let msg = match &e {
                                    ServerFnError::ServerError { message, .. } => {
                                        message.clone()
                                    }
                                    _ => e.to_string(),
                                };
                                delete_error.set(Some(msg));
                                is_deleting.set(false);
                            }
                        }
                    });
                }
            }
        }
    };

    // ── Render: loading ──────────────────────────────────────────────────
    if any_pending {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        justify_content: "center",
                        min_height: "300px",
                        LoadingSpinner {}
                        p {
                            margin_top: "var(--space-md)",
                            color: "var(--text-secondary)",
                            "Loading recipe..."
                        }
                    }
                }
            }
        };
    }

    // ── Render: error ────────────────────────────────────────────────────
    if let Some(err_msg) = &error_message {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    Card {
                        div {
                            display: "flex",
                            flex_direction: "column",
                            align_items: "center",
                            text_align: "center",
                            gap: "var(--space-md)",
                            p {
                                color: "var(--error)",
                                font_weight: "600",
                                font_size: "18px",
                                "{err_msg}"
                            }
                            Link {
                                to: crate::Route::Dashboard {},
                                class: "btn btn-secondary touch-target",
                                "Back to Recipes"
                            }
                        }
                    }
                }
            }
        };
    }

    // ── Render: content ──────────────────────────────────────────────────
    let Some(recipe) = recipe else {
        // Should not reach here if logic is correct, but guard anyway
        return rsx! {
            AuthRequired {
                div { class: "container",
                    Card {
                        div {
                            text_align: "center",
                            color: "var(--error)",
                            "Unable to load recipe."
                        }
                    }
                }
            }
        };
    };

    // Parse instructions
    let parsed = recipe
        .instructions
        .as_deref()
        .map(parse_instructions)
        .unwrap_or_default();

    // Auth context for username
    let auth = use_auth();
    let username = auth
        .current_user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "you".to_string());
    let relative_time = format_relative_time(recipe.created_at);

    rsx! {
        AuthRequired {
            div { class: "container",
                // ── Header ──────────────────────────────────────────────────────
                PageHeader {
                    title: "{recipe.title}",
                    action: rsx! {
                        div {
                            display: "flex",
                            gap: "var(--space-sm)",
                            // Edit button
                            Link {
                                to: crate::Route::RecipeEdit { id: recipe.id.to_string() },
                                class: "btn btn-secondary touch-target",
                                "Edit"
                            }
                            // Delete button
                            Button {
                                variant: ButtonVariant::Danger,
                                disabled: is_deleting(),
                                onclick: on_delete,
                                if is_deleting() {
                                    "Deleting..."
                                } else {
                                    "Delete"
                                }
                            }
                        }
                    }
                }

                // ── Back link ───────────────────────────────────────────────────
                div { margin_bottom: "var(--space-md)",
                    Link {
                        to: crate::Route::Dashboard {},
                        style: "color: var(--accent); text-decoration: none; font-size: 14px; font-weight: 500;",
                        "← Back to Recipes"
                    }
                }

                // ── Delete error ────────────────────────────────────────────────
                if let Some(del_err) = delete_error() {
                    div {
                        padding: "var(--space-sm) var(--space-md)",
                        background_color: "var(--error-bg)",
                        border_radius: "var(--radius-md)",
                        color: "var(--error)",
                        font_size: "14px",
                        margin_bottom: "var(--space-md)",
                        "{del_err}"
                    }
                }

                // ── Tags ────────────────────────────────────────────────────────
                if let Some(tag_list) = tags {
                    if !tag_list.is_empty() {
                        div {
                            display: "flex",
                            flex_wrap: "wrap",
                            gap: "var(--space-xs)",
                            margin_bottom: "var(--space-md)",
                            for tag in tag_list {
                                span {
                                    display: "inline-block",
                                    padding: "4px 12px",
                                    border_radius: "var(--radius-full)",
                                    background_color: "rgba(217, 115, 90, 0.10)",
                                    color: "var(--accent)",
                                    font_size: "13px",
                                    font_weight: "500",
                                    "{tag}"
                                }
                            }
                        }
                    }
                }

                // ── Meta info row ───────────────────────────────────────────────
                div {
                    display: "flex",
                    flex_wrap: "wrap",
                    gap: "var(--space-md)",
                    margin_bottom: "var(--space-md)",
                    padding: "var(--space-sm) 0",
                    border_bottom: "1px solid var(--surface)",

                    if let Some(prepare) = recipe.prep_time_minutes {
                        span {
                            font_size: "14px",
                            color: "var(--text-secondary)",
                            "⏱ Prep: {prepare} min"
                        }
                    }
                    if let Some(cook) = recipe.cook_time_minutes {
                        span {
                            font_size: "14px",
                            color: "var(--text-secondary)",
                            "🔥 Cook: {cook} min"
                        }
                    }
                    if let Some(serv) = recipe.servings {
                        span {
                            font_size: "14px",
                            color: "var(--text-secondary)",
                            "🍽 Servings: {serv}"
                        }
                    }
                }

                // ── Description ─────────────────────────────────────────────────
                if let Some(desc) = &recipe.description {
                    if !desc.is_empty() {
                        div {
                            margin_bottom: "var(--space-md)",
                            p {
                                font_size: "15px",
                                color: "var(--text-secondary)",
                                line_height: "1.6",
                                "{desc}"
                            }
                        }
                    }
                }

                // ── Author line ─────────────────────────────────────────────────
                div {
                    margin_bottom: "var(--space-lg)",
                    font_size: "13px",
                    color: "var(--text-tertiary)",
                    "by @{username} • created {relative_time}"
                }

                // ── Ingredients ─────────────────────────────────────────────────
                if !parsed.ingredients.is_empty() {
                    div {
                        margin_bottom: "var(--space-lg)",
                        h2 {
                            font_size: "20px",
                            color: "var(--text-primary)",
                            margin_bottom: "var(--space-sm)",
                            padding_bottom: "var(--space-xs)",
                            border_bottom: "2px solid var(--surface)",
                            "Ingredients"
                        }
                        ul {
                            list_style: "none",
                            padding: "0",
                            margin: "0",
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-xs)",
                            for ing in &parsed.ingredients {
                                li {
                                    padding: "var(--space-xs) var(--space-sm)",
                                    font_size: "14px",
                                    color: "var(--text-primary)",
                                    if !ing.amount.is_empty() && !ing.unit.is_empty() {
                                        "- {ing.amount} {ing.unit} {ing.name}"
                                    } else if !ing.amount.is_empty() {
                                        "- {ing.amount} {ing.name}"
                                    } else {
                                        "- {ing.name}"
                                    }
                                }
                            }
                        }
                    }
                }

                // ── Steps ───────────────────────────────────────────────────────
                if !parsed.steps.is_empty() {
                    div {
                        margin_bottom: "var(--space-lg)",
                        h2 {
                            font_size: "20px",
                            color: "var(--text-primary)",
                            margin_bottom: "var(--space-sm)",
                            padding_bottom: "var(--space-xs)",
                            border_bottom: "2px solid var(--surface)",
                            "Steps"
                        }
                        ol {
                            padding_left: "var(--space-lg)",
                            margin: "0",
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-sm)",
                            for step in &parsed.steps {
                                li {
                                    padding: "var(--space-xs) 0",
                                    font_size: "14px",
                                    color: "var(--text-primary)",
                                    line_height: "1.6",
                                    "{step}"
                                }
                            }
                        }
                    }
                }

                // ── Raw instructions fallback ───────────────────────────────────
                if parsed.ingredients.is_empty() && parsed.steps.is_empty() {
                    if let Some(raw) = &parsed.raw {
                        if !raw.is_empty() {
                            div {
                                margin_bottom: "var(--space-lg)",
                                h2 {
                                    font_size: "20px",
                                    color: "var(--text-primary)",
                                    margin_bottom: "var(--space-sm)",
                                    padding_bottom: "var(--space-xs)",
                                    border_bottom: "2px solid var(--surface)",
                                    "Instructions"
                                }
                                pre {
                                    background_color: "var(--surface)",
                                    padding: "var(--space-md)",
                                    border_radius: "var(--radius-md)",
                                    font_size: "14px",
                                    color: "var(--text-secondary)",
                                    line_height: "1.6",
                                    white_space: "pre-wrap",
                                    word_wrap: "break-word",
                                    margin: "0",
                                    font_family: "var(--font-body)",
                                    "{raw}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
