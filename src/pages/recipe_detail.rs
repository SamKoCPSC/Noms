//! Single recipe detail page.
//!
//! Fetches recipe data and tags via server functions, renders typed vectors
//! of ingredients, steps (with recursive sub-steps), and equipment directly.
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
use crate::types::{Recipe, RecipeEquipment, RecipeIngredient, RecipeStep};
#[cfg(target_arch = "wasm32")]
use crate::utils::recipe_scaler::{
    format_amount, parse_amount, IngredientRef, ScaleCalculator, ScaleMode, ScaledIngredient,
};

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

// ── Recursive Step Rendering ─────────────────────────────────────────────────

/// Roman numeral for a given 1-based index (up to 10).
fn roman_numeral(n: usize) -> String {
    match n {
        1 => "i".to_string(),
        2 => "ii".to_string(),
        3 => "iii".to_string(),
        4 => "iv".to_string(),
        5 => "v".to_string(),
        6 => "vi".to_string(),
        7 => "vii".to_string(),
        8 => "viii".to_string(),
        9 => "ix".to_string(),
        10 => "x".to_string(),
        _ => n.to_string(),
    }
}

/// Build a hierarchical step label.
/// Level 0: "1.", "2."
/// Level 1: "1a.", "1b."
/// Level 2+: "1a-i.", "1a-ii."
fn build_step_label(path: &[usize]) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for (i, &idx) in path.iter().enumerate() {
        let zero_idx = idx; // 0-based index from the parent's sub_steps vec
        match i {
            0 => parts.push(format!("{}", zero_idx + 1)),
            1 => parts.push(format!("{}", ((b'a' as usize) + zero_idx) as u8 as char)),
            _ => parts.push(roman_numeral(zero_idx + 1)),
        }
    }
    format!("{}.", parts.join("-"))
}

/// Recursive component that renders a `RecipeStep` and its nested sub-steps
/// with hierarchical numbering.
#[component]
fn StepNode(step: RecipeStep, path: Vec<usize>, level: usize) -> Element {
    let label = build_step_label(&path);
    let indent = if level == 0 {
        0.0
    } else {
        (level - 1) as f64 * 20.0
    };

    rsx! {
        li {
            padding: "var(--space-xs) 0",
            margin_left: "{indent}px",
            font_size: "14px",
            color: "var(--text-primary)",
            line_height: "1.6",
            strong { font_weight: "600", margin_right: "var(--space-xs)", "{label}" }
            "{step.text}"

            if !step.sub_steps.is_empty() {
                ol {
                    padding_left: "0",
                    margin_top: "var(--space-xs)",
                    list_style: "none",
                    for (idx, sub) in step.sub_steps.iter().enumerate() {
                        StepNode { step: sub.clone(), path: [path.clone(), vec![idx]].concat(), level: level + 1 }
                    }
                }
            }
        }
    }
}

// ── Recipe Scaler Component (CP12) ───────────────────────────────────────────

/// Collapsible recipe scaling widget.
///
/// Supports two modes:
/// - Multiplier: user enters a direct multiplier (e.g. 2x, 0.5x)
/// - Target Ingredient: user picks an ingredient and desired amount
#[cfg(target_arch = "wasm32")]
#[component]
fn RecipeScaler(
    ingredients: Vec<RecipeIngredient>,
    prep_time_minutes: Option<i32>,
    cook_time_minutes: Option<i32>,
    servings: Option<i32>,
) -> Element {
    let mut is_expanded = use_signal(|| false);

    // Convert RecipeIngredient -> IngredientRef for ScaleCalculator
    let refs: Vec<IngredientRef> = ingredients
        .iter()
        .map(|ing| IngredientRef {
            amount: ing.amount.clone(),
            unit: ing.unit.clone(),
            name: ing.name.clone(),
        })
        .collect();

    let mut calculator = use_signal(|| {
        ScaleCalculator::new(refs.clone(), servings, prep_time_minutes, cook_time_minutes)
    });
    let mut multiplier_input = use_signal(String::new);
    let mut target_amount_input = use_signal(String::new);
    let mut target_ingredient_idx = use_signal(|| 0_usize);

    // Read calculator state early (clone to own the data)
    let current_mode = calculator.read().mode().clone();
    let current_multiplier = calculator.read().multiplier();

    // Preset multipliers — pre-compute all data for rsx! (no `let` allowed inside rsx!)
    let preset_data: Vec<(f64, &'static str, String)> = [
        (0.5, "½x"),
        (1.0, "1x"),
        (2.0, "2x"),
        (3.0, "3x"),
        (4.0, "4x"),
    ]
    .iter()
    .map(|(mult, label)| {
        let is_active = matches!(current_mode, ScaleMode::Multiplier(_))
            && (current_multiplier - mult).abs() < 0.001;
        let class = if is_active {
            "recipe-scaler__preset recipe-scaler__preset--active".to_string()
        } else {
            "recipe-scaler__preset".to_string()
        };
        (*mult, *label, class)
    })
    .collect();

    // Mode toggle handler
    let on_set_multiplier = move |_| {
        let val = multiplier_input.read().clone();
        if let Some(m) = parse_amount(&val) {
            calculator.with_mut(|calc| calc.set_multiplier(m));
        }
    };

    let on_set_target = move |_| {
        let val = target_amount_input.read().clone();
        if let Some(target) = parse_amount(&val) {
            let idx = target_ingredient_idx();
            calculator.with_mut(|calc| calc.set_target_ingredient(idx, target));
        }
    };

    let on_select_ingredient = move |e: Event<dioxus::events::FormData>| {
        if let Ok(idx) = e.value().parse::<usize>() {
            target_ingredient_idx.set(idx);
        }
    };

    let on_reset = move |_| {
        multiplier_input.set(String::new());
        target_amount_input.set(String::new());
        target_ingredient_idx.set(0);
        calculator.with_mut(|calc| calc.reset());
    };

    // Scaled ingredients — use ScaleCalculator's method (handles non-numeric correctly)
    let scaled_ingredients: Vec<ScaledIngredient> = calculator.read().scaled_ingredients();
    let scaled_servings = calculator.read().scaled_servings();
    let scaled_prep = calculator.read().scaled_prep_time();
    let scaled_cook = calculator.read().scaled_cook_time();
    let show_scaled = (current_multiplier - 1.0).abs() > 0.001;

    // Pre-compute display strings for scaled ingredients (rsx! can't handle multi-arg format strings)
    let scaled_ingredient_display: Vec<String> = scaled_ingredients
        .iter()
        .map(|ing| {
            if !ing.unit.is_empty() {
                format!("- {} {} {}", ing.amount, ing.unit, ing.name)
            } else if !ing.amount.is_empty() {
                format!("- {} {}", ing.amount, ing.name)
            } else {
                format!("- {}", ing.name)
            }
        })
        .collect();

    // Pre-computed option labels for target ingredient select
    let option_labels: Vec<String> = refs
        .iter()
        .map(|r| {
            let amt = parse_amount(&r.amount).unwrap_or(0.0);
            format!("- {} ({})", r.name, format_amount(amt))
        })
        .collect();

    rsx! {
        div { class: "recipe-scaler",
            // Toggle button
            button {
                class: "recipe-scaler__toggle",
                onclick: move |_| is_expanded.set(!is_expanded()),
                if is_expanded() {
                    "▼"
                } else {
                    "▶"
                }
                " Scale Recipe"
                span {
                    class: "recipe-scaler__badge",
                    "×{current_multiplier:.1}"
                }
            }

            // Collapsible body
            if is_expanded() {
                div { class: "recipe-scaler__body",
                    // Mode: Multiplier
                    div {
                        label {
                            display: "block",
                            font_size: "13px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-xs)",
                            "Multiplier"
                        }
                        div {
                            display: "flex",
                            gap: "var(--space-sm)",
                            align_items: "center",
                            input {
                                class: "recipe-scaler__input",
                                r#type: "text",
                                placeholder: "e.g. 2",
                                value: multiplier_input.read().clone(),
                                oninput: move |e| multiplier_input.set(e.value()),
                            }
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: on_set_multiplier,
                                "Apply"
                            }
                        }
                        // Presets
                        div { class: "recipe-scaler__presets",
                            for (mult, label, class) in &preset_data {
                                button {
                                    class: class.clone(),
                                    onclick: {
                                        let mult_val = *mult;
                                        move |_| {
                                            multiplier_input.set(format!("{mult_val}"));
                                            calculator.with_mut(|calc| calc.set_multiplier(mult_val));
                                        }
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }

                    // Mode: Target Ingredient
                    div {
                        margin_top: "var(--space-md)",
                        label {
                            display: "block",
                            font_size: "13px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            margin_bottom: "var(--space-xs)",
                            "Or scale by target ingredient"
                        }
                        select {
                            class: "recipe-scaler__select",
                            onchange: on_select_ingredient,
                            for (idx, label) in option_labels.iter().enumerate() {
                                option { value: idx.to_string(), "{label}" }
                            }
                        }
                        div {
                            display: "flex",
                            gap: "var(--space-sm)",
                            align_items: "center",
                            margin_top: "var(--space-xs)",
                            input {
                                class: "recipe-scaler__input",
                                r#type: "text",
                                placeholder: "desired amount",
                                value: target_amount_input.read().clone(),
                                oninput: move |e| target_amount_input.set(e.value()),
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: on_set_target,
                                "Apply"
                            }
                        }
                    }

                    // Scaled meta info
                    if show_scaled {
                        div {
                            margin_top: "var(--space-md)",
                            padding_top: "var(--space-sm)",
                            border_top: "1px solid var(--surface)",
                            display: "flex",
                            flex_wrap: "wrap",
                            gap: "var(--space-sm)",
                            font_size: "13px",
                            color: "var(--text-secondary)",

                            if let Some(s) = scaled_servings {
                                span { "🍽 {s} servings" }
                            }
                            if let Some(p) = scaled_prep {
                                span { "⏱ Prep: {p} min" }
                            }
                            if let Some(c) = scaled_cook {
                                span { "🔥 Cook: {c} min" }
                            }
                        }
                    }

                    // Scaled ingredients
                    if show_scaled {
                        div {
                            margin_top: "var(--space-md)",
                            h3 {
                                font_size: "16px",
                                color: "var(--text-primary)",
                                margin_bottom: "var(--space-xs)",
                                "Scaled Ingredients"
                            }
                            ul {
                                list_style: "none",
                                padding: "0",
                                margin: "0",
                                display: "flex",
                                flex_direction: "column",
                                gap: "var(--space-xs)",
                                for text in &scaled_ingredient_display {
                                    li {
                                        padding: "var(--space-xs) var(--space-sm)",
                                        font_size: "14px",
                                        color: "var(--text-primary)",
                                        "{text}"
                                    }
                                }
                            }
                        }
                    }

                    // Reset button
                    button {
                        class: "recipe-scaler__reset",
                        onclick: on_reset,
                        "Reset to original"
                    }
                }
            }
        }
    }
}

// ── Helper: conditional RecipeScaler rendering ───────────────────────────────

/// Render the RecipeScaler widget on WASM; empty element on server.
fn render_recipe_scaler(
    ingredients: &[RecipeIngredient],
    #[allow(unused_variables)] prep_time_minutes: Option<i32>,
    #[allow(unused_variables)] cook_time_minutes: Option<i32>,
    #[allow(unused_variables)] servings: Option<i32>,
) -> Element {
    if ingredients.is_empty() {
        return rsx! {};
    }
    #[cfg(target_arch = "wasm32")]
    {
        rsx! {
            RecipeScaler {
                ingredients: ingredients.to_vec(),
                prep_time_minutes,
                cook_time_minutes,
                servings,
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rsx! {}
    }
}

// ── Helper: conditional Equipment rendering ────────────────────────────

/// Render the equipment section if present.
fn render_equipment(equipment: &[RecipeEquipment]) -> Element {
    if equipment.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            margin_bottom: "var(--space-lg)",
            h2 {
                font_size: "20px",
                color: "var(--text-primary)",
                margin_bottom: "var(--space-sm)",
                padding_bottom: "var(--space-xs)",
                border_bottom: "2px solid var(--surface)",
                "Equipment"
            }
            ul {
                list_style: "none",
                padding: "0",
                margin: "0",
                display: "flex",
                flex_direction: "column",
                gap: "var(--space-xs)",
                for item in equipment {
                    li {
                        padding: "var(--space-xs) var(--space-sm)",
                        font_size: "14px",
                        color: "var(--text-primary)",
                        "• {item.name}"
                    }
                }
            }
        }
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

            // ── Recipe Scaler (CP12) ────────────────────────────────────────
            {render_recipe_scaler(&recipe.ingredients, recipe.prep_time_minutes, recipe.cook_time_minutes, recipe.servings)}

            // ── Equipment ───────────────────────────────────────────────────
            {render_equipment(&recipe.equipment)}

            // ── Ingredients ─────────────────────────────────────────────────
            if !recipe.ingredients.is_empty() {
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
                        for ing in &recipe.ingredients {
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
            if !recipe.instructions.is_empty() {
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
                        list_style: "none",
                        for (idx, step) in recipe.instructions.iter().enumerate() {
                            StepNode { step: step.clone(), path: vec![idx], level: 0 }
                        }
                    }
                }
            }
        }
    }
}
