//! Recipe edit page.
//!
//! Fetches an existing recipe and tags, pre-populates a form identical to the
//! create form, and calls `update_recipe()` on submit.

use dioxus::prelude::*;

use crate::api::recipe::{get_recipe, get_recipe_tags, update_recipe};
use crate::components::base::{Button, ButtonVariant, Input, LoadingSpinner, PageHeader};
use crate::components::AuthRequired;
use crate::types::{RecipeEquipment, RecipeIngredient, RecipeStep};

// ── Local form types ─────────────────────────────────────────────────────────

/// A single step row in the form (flat representation with depth for nesting).
#[derive(Clone, Debug)]
struct StepForm {
    text: String,
    depth: usize,
}

/// Flatten a tree of `RecipeStep` into a flat list with depth information.
fn flatten_steps(steps: &[RecipeStep], depth: usize) -> Vec<StepForm> {
    let mut result = Vec::new();
    for step in steps {
        result.push(StepForm {
            text: step.text.clone(),
            depth,
        });
        result.extend(flatten_steps(&step.sub_steps, depth + 1));
    }
    result
}

/// Build a tree of `RecipeStep` from a flat list with depth information.
fn build_step_tree(flat: &[StepForm]) -> Vec<RecipeStep> {
    if flat.is_empty() {
        return Vec::new();
    }

    // Build tree using a stack of (depth, index_in_parent_list)
    // We construct the tree bottom-up using indices into the result vector.
    let mut root = Vec::new();
    // Stack holds (depth, parent_index) where parent_index is the index in
    // the vector that currently holds sub_steps we're appending to.
    // We use a flat representation: a list of RecipeSteps where we track
    // parent-child relationships via indices.
    let mut nodes: Vec<RecipeStep> = Vec::new();
    // Stack of (depth, node_index) — the node at that index is the current parent
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for step in flat {
        let node = RecipeStep {
            text: step.text.clone(),
            sub_steps: Vec::new(),
        };
        let node_idx = nodes.len();
        nodes.push(node);

        // Pop stack until we find the right parent depth
        while stack.len() > 1 && stack.last().unwrap().0 >= step.depth {
            let _ = stack.pop();
        }

        if step.depth == 0 {
            // Root-level step — add to root
            root.push(nodes[node_idx].clone());
            stack.push((0, node_idx));
        } else {
            // Find parent and add this node as sub_step
            let parent_idx = stack.last().unwrap().1;
            let node_clone = nodes[node_idx].clone();
            nodes[parent_idx].sub_steps.push(node_clone);
            stack.push((step.depth, node_idx));
        }
    }

    root
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
    let mut ingredients = use_signal(Vec::<RecipeIngredient>::new);
    let mut steps = use_signal(Vec::<StepForm>::new);
    let mut tags_input = use_signal(String::new);
    let mut equipment = use_signal(Vec::<RecipeEquipment>::new);
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

                    // Use typed ingredients directly
                    ingredients.set(recipe.ingredients);

                    // Flatten tree steps into flat list with depth
                    steps.set(flatten_steps(&recipe.instructions, 0));

                    // Fetch tags
                    if let Ok(tags) = get_recipe_tags(id).await {
                        tags_input.set(tags.join(", "))
                    }

                    // Use typed equipment directly
                    equipment.set(recipe.equipment);

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

        // ── Build typed vectors from form data ──────────────────────────
        let typed_ingredients = ingredients();

        // Build step tree from flat form
        let instructions = build_step_tree(&steps());

        // Equipment is already typed
        let equip_vec = equipment();

        // ── Commit values and spawn async call ──────────────────────────
        is_saving.set(true);

        let desc = if description().trim().is_empty() {
            None
        } else {
            Some(description().trim().to_string())
        };

        let recipe_id = id_for_submit.clone();
        let vis = visibility().clone();

        spawn(async move {
            match update_recipe(
                recipe_id.clone(),
                Some(trimmed_title),
                desc,
                prep,
                cook,
                serv,
                Some(typed_ingredients),
                Some(instructions),
                Some(equip_vec),
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
        ingredients.write().push(RecipeIngredient {
            amount: String::new(),
            unit: String::new(),
            name: String::new(),
        });
    };

    // ── Step helpers ────────────────────────────────────────────────────
    let on_add_step = move |_| {
        steps.write().push(StepForm {
            text: String::new(),
            depth: 0,
        });
    };

    // ── Equipment helpers ───────────────────────────────────────────────
    let on_add_equipment = move |_| {
        equipment.write().push(RecipeEquipment {
            name: String::new(),
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
                                // Indentation padding
                                style: format!("padding-left: {}px", steps()[idx].depth * 24),
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
                                // Reorder + indent + remove controls
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
                                    // Indent (increase depth)
                                    if steps()[idx].depth < 3 {
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                steps.write()[idx].depth += 1;
                                            },
                                            "→"
                                        }
                                    }
                                    // Unindent (decrease depth)
                                    if steps()[idx].depth > 0 {
                                        Button {
                                            variant: ButtonVariant::Ghost,
                                            onclick: move |_| {
                                                steps.write()[idx].depth -= 1;
                                            },
                                            "←"
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

                    // ── Equipment (dynamic list) ────────────────────────
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

                        for (idx, _eq) in equipment().iter().enumerate() {
                            div {
                                key: "{idx}",
                                display: "flex",
                                gap: "var(--space-sm)",
                                align_items: "center",
                                Input {
                                    value: equipment()[idx].name.clone(),
                                    placeholder: "e.g. mixing bowl",
                                    oninput: move |v| {
                                        equipment.write()[idx].name = v;
                                    },
                                }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| {
                                        equipment.write().remove(idx);
                                    },
                                    "✕"
                                }
                            }
                        }

                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: on_add_equipment,
                            "+ Add Equipment"
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
