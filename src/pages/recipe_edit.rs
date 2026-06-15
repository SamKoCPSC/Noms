//! Recipe edit page.
//!
//! Fetches an existing recipe and tags, pre-populates a form identical to the
//! create form, and calls `update_recipe()` on submit.

use dioxus::prelude::*;

use crate::api::recipe::{get_recipe, get_recipe_tags, update_recipe};
use crate::components::base::{Button, ButtonVariant, Input, LoadingSpinner, PageHeader};
use crate::components::AuthRequired;

// ── Draft types ──────────────────────────────────────────────────────────────

/// A single ingredient row in the form.
#[derive(Clone, Debug)]
struct IngredientDraft {
    amount: String,
    unit: String,
    name: String,
}

/// A single step row in the form.
#[derive(Clone, Debug)]
struct StepDraft {
    text: String,
}

// ── Parsed instruction types ─────────────────────────────────────────────────

/// A single parsed ingredient (from the instructions text).
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
}

// ── Serialization helper ─────────────────────────────────────────────────────

/// Combine ingredients and steps into a single markdown-style text block
/// suitable for the `instructions` TEXT column.
fn serialize_instructions(ingredients: &[IngredientDraft], steps: &[StepDraft]) -> String {
    let mut text = String::new();

    if !ingredients.is_empty() {
        text.push_str("INGREDIENTS:\n");
        for ing in ingredients {
            let mut parts = Vec::new();
            if !ing.amount.is_empty() {
                parts.push(ing.amount.clone());
            }
            if !ing.unit.is_empty() {
                parts.push(ing.unit.clone());
            }
            parts.push(ing.name.clone());
            text.push_str(&format!("- {}\n", parts.join(" ")));
        }
        text.push('\n');
    }

    if !steps.is_empty() {
        text.push_str("STEPS:\n");
        for (i, step) in steps.iter().enumerate() {
            text.push_str(&format!("{}. {}\n", i + 1, step.text));
        }
    }

    text.trim_end().to_string()
}

// ── Parsing helper ────────────────────────────────────────────────────────────

/// Parse the serialized instructions text back into ingredients and steps.
///
/// Reverses the format produced by `serialize_instructions()`.
fn parse_instructions(text: &str) -> ParsedInstructions {
    let mut result = ParsedInstructions::default();
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
                if let Some(dot_pos) = trimmed.find('.') {
                    let num_part = &trimmed[..dot_pos];
                    let rest = trimmed[dot_pos + 1..].trim();
                    if num_part.parse::<u32>().is_ok() && !rest.is_empty() {
                        result.steps.push(rest.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    result
}

// ── Component ────────────────────────────────────────────────────────────────

/// Edit recipe page — fetches existing data, pre-populates the form, saves via update_recipe.
#[component]
pub fn RecipeEdit(id: String) -> Element {
    // ── Form signals ─────────────────────────────────────────────────────
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut prep_time = use_signal(String::new);
    let mut cook_time = use_signal(String::new);
    let mut servings = use_signal(String::new);
    let mut ingredients = use_signal(Vec::<IngredientDraft>::new);
    let mut steps = use_signal(Vec::<StepDraft>::new);
    let mut tags_input = use_signal(String::new);
    let mut equipment = use_signal(String::new);
    let mut visibility = use_signal(|| "private".to_string());

    // UI state
    let mut is_loading = use_signal(|| true);
    let mut is_saving = use_signal(|| false);
    let mut load_error = use_signal(|| Option::<String>::None);
    let mut form_error = use_signal(|| Option::<String>::None);

    // ── Fetch recipe data on mount ───────────────────────────────────────
    let id_for_fetch = id.clone();
    use_effect(move || {
        let id = id_for_fetch.clone();
        spawn(async move {
            match get_recipe(id.clone()).await {
                Ok(recipe) => {
                    title.set(recipe.title.clone());
                    description.set(recipe.description.clone().unwrap_or_default());
                    prep_time.set(
                        recipe
                            .prep_time_minutes
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    );
                    cook_time.set(
                        recipe
                            .cook_time_minutes
                            .map(|v| v.to_string())
                            .unwrap_or_default(),
                    );
                    servings.set(recipe.servings.map(|v| v.to_string()).unwrap_or_default());

                    // Parse instructions into ingredients/steps
                    if let Some(ref instructions) = recipe.instructions {
                        let parsed = parse_instructions(instructions);
                        ingredients.set(
                            parsed
                                .ingredients
                                .iter()
                                .map(|p| IngredientDraft {
                                    amount: p.amount.clone(),
                                    unit: p.unit.clone(),
                                    name: p.name.clone(),
                                })
                                .collect(),
                        );
                        steps.set(
                            parsed
                                .steps
                                .iter()
                                .map(|s| StepDraft { text: s.clone() })
                                .collect(),
                        );
                    }

                    // Fetch tags
                    if let Ok(tags) = get_recipe_tags(id).await {
                        tags_input.set(tags.join(", "))
                    }
                    equipment.set(recipe.equipment.clone().unwrap_or_default());
                    visibility.set(recipe.visibility);
                }
                Err(e) => {
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };
                    if !msg.contains("Not authenticated") {
                        load_error.set(Some(msg));
                    }
                }
            }
            is_loading.set(false);
        });
    });

    // ── Clone id for closures that need it ──────────────────────────────
    let id_for_submit = id.clone();
    let id_for_cancel = id.clone();

    // ── Submit handler ───────────────────────────────────────────────────
    let on_submit = move |_| {
        // Clear previous errors
        form_error.set(None);

        // ── Client-side validation ──────────────────────────────────────
        let trimmed_title = title().trim().to_string();
        if trimmed_title.is_empty() {
            form_error.set(Some("Title is required".to_string()));
            return;
        }

        let ings = ingredients();
        if ings.is_empty() {
            form_error.set(Some("Add at least one ingredient".to_string()));
            return;
        }
        if ings.iter().all(|i| i.name.trim().is_empty()) {
            form_error.set(Some("At least one ingredient must have a name".to_string()));
            return;
        }

        let sts = steps();
        if sts.is_empty() {
            form_error.set(Some("Add at least one step".to_string()));
            return;
        }
        if sts.iter().all(|s| s.text.trim().is_empty()) {
            form_error.set(Some("At least one step must have text".to_string()));
            return;
        }

        // ── Parse optional numeric fields ───────────────────────────────
        let prep = prep_time().trim().parse::<i32>().ok();
        let cook = cook_time().trim().parse::<i32>().ok();
        let serv = servings().trim().parse::<i32>().ok();

        // ── Parse tags ──────────────────────────────────────────────────
        let tags: Vec<String> = tags_input()
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();

        // ── Serialize instructions ──────────────────────────────────────
        let instructions = serialize_instructions(&ings, &sts);

        // ── Commit values and spawn async call ──────────────────────────
        is_saving.set(true);

        let desc = if description().trim().is_empty() {
            None
        } else {
            Some(description().trim().to_string())
        };

        let recipe_id = id_for_submit.clone();
        let vis = visibility().clone();
        let equip = if equipment().trim().is_empty() {
            None
        } else {
            Some(equipment().trim().to_string())
        };

        spawn(async move {
            match update_recipe(
                recipe_id.clone(),
                Some(trimmed_title),
                desc,
                prep,
                cook,
                serv,
                Some(instructions),
                equip,
                Some(tags),
                Some(vis),
            )
            .await
            {
                Ok(_) => {
                    // Redirect to recipe detail page
                    if let Some(window) = web_sys::window() {
                        let _ = window
                            .location()
                            .set_href(&format!("/recipes/{}", recipe_id));
                    }
                }
                Err(e) => {
                    let msg = match &e {
                        ServerFnError::ServerError { message, .. } => message.clone(),
                        _ => e.to_string(),
                    };
                    form_error.set(Some(msg));
                    is_saving.set(false);
                }
            }
        });
    };

    // ── Cancel handler ──────────────────────────────────────────────────
    let on_cancel = move |_| {
        if let Some(window) = web_sys::window() {
            let _ = window
                .location()
                .set_href(&format!("/recipes/{}", id_for_cancel));
        }
    };

    // ── Ingredient helpers ──────────────────────────────────────────────
    let on_add_ingredient = move |_| {
        ingredients.write().push(IngredientDraft {
            amount: String::new(),
            unit: String::new(),
            name: String::new(),
        });
    };

    // ── Step helpers ────────────────────────────────────────────────────
    let on_add_step = move |_| {
        steps.write().push(StepDraft {
            text: String::new(),
        });
    };

    // ── Render: loading ─────────────────────────────────────────────────
    if is_loading() {
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

    // ── Render: error ───────────────────────────────────────────────────
    if let Some(err_msg) = load_error() {
        return rsx! {
            AuthRequired {
                div { class: "container",
                    div {
                        display: "flex",
                        flex_direction: "column",
                        align_items: "center",
                        text_align: "center",
                        gap: "var(--space-md)",
                        padding: "var(--space-xl)",
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

    // ── Render: form ────────────────────────────────────────────────────
    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "Edit Recipe",
                }
                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "var(--space-md)",
                    max_width: "600px",

                    // ── Basic Info ──────────────────────────────────────
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
                            value: title().clone(),
                            placeholder: "e.g. Grandma's Chocolate Chip Cookies",
                            oninput: move |v| {
                                title.set(v);
                                form_error.set(None);
                            },
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
                            placeholder: "Brief description of the recipe...",
                            rows: "4",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            background_color: "var(--surface)",
                            outline: "none",
                            resize: "vertical",
                            width: "100%",
                            value: description().clone(),
                            oninput: move |evt| {
                                description.set(evt.value());
                            },
                        }
                    }

                    // Time & servings row
                    div {
                        display: "grid",
                        grid_template_columns: "1fr 1fr 1fr",
                        gap: "var(--space-sm)",
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-xs)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Prep Time (min)"
                            }
                            Input {
                                value: prep_time().clone(),
                                placeholder: "0",
                                input_type: "number",
                                oninput: move |v| prep_time.set(v),
                            }
                        }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-xs)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Cook Time (min)"
                            }
                            Input {
                                value: cook_time().clone(),
                                placeholder: "0",
                                input_type: "number",
                                oninput: move |v| cook_time.set(v),
                            }
                        }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "var(--space-xs)",
                            label {
                                font_size: "14px",
                                font_weight: "600",
                                color: "var(--text-secondary)",
                                "Servings"
                            }
                            Input {
                                value: servings().clone(),
                                placeholder: "4",
                                input_type: "number",
                                oninput: move |v| servings.set(v),
                            }
                        }
                    }

                    // ── Ingredients ─────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Ingredients *"
                        }

                        for (idx, _ing) in ingredients().iter().enumerate() {
                            div {
                                key: "{idx}",
                                display: "flex",
                                gap: "var(--space-sm)",
                                align_items: "center",
                                // Amount
                                div {
                                    style: "width: 70px; flex-shrink: 0;",
                                    Input {
                                        value: ingredients()[idx].amount.clone(),
                                        placeholder: "Amt",
                                        oninput: move |v| {
                                            ingredients.write()[idx].amount = v;
                                        },
                                    }
                                }
                                // Unit
                                div {
                                    style: "width: 80px; flex-shrink: 0;",
                                    Input {
                                        value: ingredients()[idx].unit.clone(),
                                        placeholder: "Unit",
                                        oninput: move |v| {
                                            ingredients.write()[idx].unit = v;
                                        },
                                    }
                                }
                                // Name
                                Input {
                                    value: ingredients()[idx].name.clone(),
                                    placeholder: "Ingredient name",
                                    oninput: move |v| {
                                        ingredients.write()[idx].name = v;
                                    },
                                }
                                // Remove
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| {
                                        ingredients.write().remove(idx);
                                    },
                                    "✕"
                                }
                            }
                        }

                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: on_add_ingredient,
                            "+ Add Ingredient"
                        }
                    }

                    // ── Steps ───────────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Steps *"
                        }

                        for (idx, _step) in steps().iter().enumerate() {
                            div {
                                key: "{idx}",
                                display: "flex",
                                gap: "var(--space-sm)",
                                align_items: "flex-start",
                                // Step number
                                span {
                                    font_weight: "600",
                                    color: "var(--text-secondary)",
                                    padding: "var(--space-sm) 0",
                                    min_width: "24px",
                                    "{idx + 1}."
                                }
                                // Step text
                                textarea {
                                    class: "neumo-inset input",
                                    placeholder: "Describe this step...",
                                    rows: "2",
                                    style: "flex: 1;",
                                    padding: "var(--space-sm) var(--space-md)",
                                    font_family: "var(--font-body)",
                                    font_size: "14px",
                                    color: "var(--text-primary)",
                                    background_color: "var(--surface)",
                                    outline: "none",
                                    resize: "vertical",
                                    value: steps()[idx].text.clone(),
                                    oninput: move |evt| {
                                        steps.write()[idx].text = evt.value();
                                    },
                                }
                                // Reorder + remove controls
                                div {
                                    display: "flex",
                                    flex_direction: "column",
                                    gap: "var(--space-xs)",
                                    align_items: "center",
                                    // Move up
                                    if idx > 0 {
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                let mut s = steps.write();
                                                s.swap(idx, idx - 1);
                                            },
                                            "↑"
                                        }
                                    }
                                    // Move down
                                    if idx < steps().len() - 1 {
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                let mut s = steps.write();
                                                s.swap(idx, idx + 1);
                                            },
                                            "↓"
                                        }
                                    }
                                    // Remove
                                    Button {
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| {
                                            steps.write().remove(idx);
                                        },
                                        "✕"
                                    }
                                }
                            }
                        }

                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: on_add_step,
                            "+ Add Step"
                        }
                    }

                    // ── Tags ────────────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Tags"
                        }
                        Input {
                            value: tags_input().clone(),
                            placeholder: "dinner, chicken, quick",
                            oninput: move |v| tags_input.set(v),
                        }
                        span {
                            font_size: "12px",
                            color: "var(--text-tertiary)",
                            "Comma-separated"
                        }
                    }

                    // ── Equipment ───────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Equipment"
                        }
                        textarea {
                            class: "neumo-inset input",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            background_color: "var(--surface)",
                            outline: "none",
                            border_radius: "var(--radius-md)",
                            width: "100%",
                            min_height: "80px",
                            resize: "vertical",
                            placeholder: "mixing bowl, whisk, large pan",
                            value: equipment().clone(),
                            oninput: move |v| equipment.set(v.value()),
                        }
                        span {
                            font_size: "12px",
                            color: "var(--text-tertiary)",
                            "One item per line or comma-separated"
                        }
                    }

                    // ── Visibility ──────────────────────────────────────
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "var(--space-sm)",
                        label {
                            font_size: "14px",
                            font_weight: "600",
                            color: "var(--text-secondary)",
                            "Visibility"
                        }
                        select {
                            class: "neumo-inset input",
                            padding: "var(--space-sm) var(--space-md)",
                            font_family: "var(--font-body)",
                            font_size: "14px",
                            color: "var(--text-primary)",
                            background_color: "var(--surface)",
                            outline: "none",
                            border_radius: "var(--radius-md)",
                            width: "100%",
                            value: visibility().clone(),
                            onchange: move |evt| {
                                visibility.set(evt.value());
                            },
                            option { value: "private", "Private — only you can see this recipe" }
                            option { value: "unlisted", "Unlisted — anyone with the link can view" }
                            option { value: "public", "Public — appears in Explore and search" }
                        }
                    }

                    // ── Error message ───────────────────────────────────
                    if let Some(err) = form_error() {
                        div {
                            padding: "var(--space-sm) var(--space-md)",
                            background_color: "var(--error-bg)",
                            border_radius: "var(--radius-md)",
                            color: "var(--error)",
                            font_size: "14px",
                            "{err}"
                        }
                    }

                    // ── Action buttons ──────────────────────────────────
                    div {
                        display: "flex",
                        gap: "var(--space-md)",
                        margin_top: "var(--space-md)",
                        Button {
                            variant: ButtonVariant::Primary,
                            disabled: is_saving(),
                            onclick: on_submit,
                            if is_saving() {
                                "Saving..."
                            } else {
                                "Save Changes"
                            }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            disabled: is_saving(),
                            onclick: on_cancel,
                            "Cancel"
                        }
                    }
                }
            }
        }
    }
}
