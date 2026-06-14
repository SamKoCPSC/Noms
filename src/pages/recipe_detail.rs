//! Single recipe detail page.
//!
//! Fetches recipe data and tags via server functions, parses the serialized
//! instructions text back into ingredients and steps for display.
//!
//! Publicly accessible: tries `get_public_recipe` first (no auth needed),
//! falls back to authenticated `get_recipe` for private/unlisted recipes.

use chrono::{DateTime, Utc};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::api::recipe::delete_recipe;
use crate::api::recipe::{
    get_public_recipe, get_public_recipe_tags_by_id, get_recipe, get_recipe_owner_username,
    get_recipe_tags,
};
use crate::auth::context::use_auth;
use crate::components::base::{Button, ButtonVariant, Card, LoadingSpinner, PageHeader};
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
                    result
                        .ingredients
                        .push(ParsedIngredient { amount, unit, name });
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
///
/// Publicly accessible: tries public endpoint first, falls back to authenticated.
#[component]
pub fn RecipeDetail(id: String) -> Element {
    // ── Resources ────────────────────────────────────────────────────────
    // Clone id before each use_resource to avoid move issues.
    // The closure is FnMut (called potentially multiple times), so we clone
    // the captured value inside the closure body before the async block.
    let id_for_recipe = id.clone();
    let recipe_resource = use_resource(move || {
        let rid = id_for_recipe.clone();
        async move {
            // Try public first (no auth needed)
            if let Ok(r) = get_public_recipe(rid.clone()).await {
                return Ok(r);
            }
            // Fall back to authenticated (requires login + ownership)
            get_recipe(rid).await
        }
    });

    // Owner username resource
    let id_for_owner = id.clone();
    let owner_username_resource = use_resource(move || {
        let oid = id_for_owner.clone();
        async move { get_recipe_owner_username(oid).await }
    });

    // Tags (signal-based, loaded conditionally based on ownership)
    let tags = use_signal(|| Option::<Vec<String>>::None);
    let tags_error = use_signal(|| Option::<String>::None);

    // Load tags once recipe is available
    let id_for_tags_effect = id.clone();
    use_effect(move || {
        let recipe_result = recipe_resource.read().clone();
        let Some(Ok(ref recipe)) = recipe_result else {
            return;
        };

        let auth = use_auth();
        let is_owner = auth.current_user_id == Some(recipe.user_id);
        let rid = id_for_tags_effect.clone();
        let mut tag_signal = tags;
        let mut tag_err = tags_error;

        spawn(async move {
            if is_owner {
                match get_recipe_tags(rid).await {
                    Ok(t) => tag_signal.set(Some(t)),
                    Err(e) => tag_err.set(Some(e.to_string())),
                }
            } else {
                match get_public_recipe_tags_by_id(rid).await {
                    Ok(t) => tag_signal.set(Some(t)),
                    Err(e) => tag_err.set(Some(e.to_string())),
                }
            }
        });
    });

    // ── Delete state ─────────────────────────────────────────────────────
    #[allow(unused_mut)]
    let mut is_deleting = use_signal(|| false);
    #[allow(unused_mut)]
    let mut delete_error = use_signal(|| Option::<String>::None);

    // ── Extract resource states ──────────────────────────────────────────
    let recipe_pending = recipe_resource.pending();
    let any_pending = recipe_pending;

    // Read resource results to avoid borrow issues in rsx!
    let recipe_result: Option<Result<Recipe, ServerFnError>> = recipe_resource.read().clone();

    // ── Derived state ────────────────────────────────────────────────────
    let recipe = recipe_result.as_ref().and_then(|r| r.as_ref().ok());

    // Determine error message (if any)
    let error_message = if let Some(Err(e)) = &recipe_result {
        let msg = match e {
            ServerFnError::ServerError { message, .. } => message.clone(),
            _ => e.to_string(),
        };
        Some(msg)
    } else {
        None
    };

    // ── Auth context for ownership check ─────────────────────────────────
    let auth = use_auth();
    let is_owner = recipe
        .map(|r| auth.current_user_id == Some(r.user_id))
        .unwrap_or(false);

    // Owner username: use owner's own username if they're viewing,
    // otherwise use the fetched owner username from the resource.
    let owner_username = if is_owner {
        auth.current_user.as_ref().map(|u| u.username.clone())
    } else {
        owner_username_resource
            .read()
            .clone()
            .and_then(|r| r.ok())
            .flatten()
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
                                    ServerFnError::ServerError { message, .. } => message.clone(),
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
        };
    }

    // ── Render: error ────────────────────────────────────────────────────
    if let Some(err_msg) = &error_message {
        return rsx! {
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
        };
    }

    // ── Render: content ──────────────────────────────────────────────────
    let Some(recipe) = recipe else {
        // Should not reach here if logic is correct, but guard anyway
        return rsx! {
            div { class: "container",
                Card {
                    div {
                        text_align: "center",
                        color: "var(--error)",
                        "Unable to load recipe."
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

    let relative_time = format_relative_time(recipe.created_at);

    rsx! {
        div { class: "container",
            // ── Header ──────────────────────────────────────────────────────
            PageHeader {
                title: "{recipe.title}",
                action: if is_owner {
                    Some(rsx! {
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
                    })
                } else {
                    None
                },
            }

            // ── Back link ───────────────────────────────────────────────────
            div { margin_bottom: "var(--space-md)",
                if is_owner {
                    Link {
                        to: crate::Route::Dashboard {},
                        style: "color: var(--accent); text-decoration: none; font-size: 14px; font-weight: 500;",
                        "← Back to Dashboard"
                    }
                } else {
                    Link {
                        to: crate::Route::Explore {},
                        style: "color: var(--accent); text-decoration: none; font-size: 14px; font-weight: 500;",
                        "← Back to Explore"
                    }
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
            if let Some(ref tag_list) = tags() {
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
                if let Some(username) = &owner_username {
                    "by "
                    Link {
                        to: crate::Route::UserProfile { username: username.clone() },
                        style: "color: var(--accent); text-decoration: none; font-weight: 500;",
                        "@{username}"
                    }
                    " • created {relative_time}"
                } else {
                    "created {relative_time}"
                }
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
