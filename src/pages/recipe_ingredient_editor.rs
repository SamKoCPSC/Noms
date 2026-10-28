//! Shared ingredient authoring for new recipes, edit drafts and imported drafts.
use crate::components::ingredient_autocomplete::IngredientAutocomplete;
use crate::types::RecipeIngredient;
use dioxus::prelude::*;

#[component]
pub(super) fn RecipeIngredientEditor(
    mut ingredients: Signal<Vec<RecipeIngredient>>,
    index: usize,
) -> Element {
    let Some(value) = ingredients.read().get(index).cloned() else {
        return rsx! {};
    };
    let substitutes = value.substitutes.clone();
    rsx! {
        div { class: "recipe-ingredient-editor",
            IngredientFields {
                id: "recipe-ingredient-{index}",
                label: "Ingredient {index + 1}",
                value,
                onchange: move |value| {
                    if let Some(row) = ingredients.write().get_mut(index) { *row = value; }
                },
                onremove: move |_| {
                    if index < ingredients.read().len() { ingredients.write().remove(index); }
                },
                details { class: "recipe-ingredient-details recipe-ingredient-substitutions",
                    summary {
                        "Substitutions"
                        if !substitutes.is_empty() { span { class: "recipe-ingredient-badge", "{substitutes.len()}" } }
                    }
                    div { class: "recipe-ingredient-substitutes",
                        p { class: "recipe-editor__hint", "add alternative ingredients" }
                        for (sub_index, substitute) in substitutes.iter().enumerate() {
                            div { key: "{sub_index}", class: "recipe-ingredient-substitute",
                                h4 { "Substitution {sub_index + 1}" }
                                IngredientFields {
                                    id: "recipe-ingredient-{index}-substitute-{sub_index}",
                                    label: "Ingredient {index + 1} substitution {sub_index + 1}",
                                    value: substitute.clone(),
                                    onchange: move |value| {
                                        if let Some(row) = ingredients.write().get_mut(index).and_then(|row| row.substitutes.get_mut(sub_index)) { *row = value; }
                                    },
                                    onremove: move |_| {
                                        if let Some(row) = ingredients.write().get_mut(index) {
                                            if sub_index < row.substitutes.len() { row.substitutes.remove(sub_index); }
                                        }
                                    },
                                }
                            }
                        }
                        button {
                            r#type: "button", class: "btn btn-secondary touch-target",
                            disabled: substitutes.len() >= crate::types::MAX_INGREDIENT_SUBSTITUTES,
                            onclick: move |_| {
                                if let Some(row) = ingredients.write().get_mut(index) {
                                    if row.substitutes.len() < crate::types::MAX_INGREDIENT_SUBSTITUTES { row.substitutes.push(RecipeIngredient::default()); }
                                }
                            },
                            "Add substitution"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn IngredientFields(
    id: String,
    label: String,
    value: RecipeIngredient,
    onchange: EventHandler<RecipeIngredient>,
    onremove: EventHandler<()>,
    #[props(default = rsx! {})] children: Element,
) -> Element {
    let preview = value.display_line(&value.amount);
    let substitution_count = value.substitutes.len();
    let mut modifiers_open = use_signal(|| false);
    // Comparing the full value also prevents index-keyed row removal from
    // applying a preceding row's confirmation to a different ingredient.
    let mut confirmed = use_signal(|| confirmation_issue(&value).is_none().then(|| value.clone()));
    let compact = confirmed.read().as_ref() == Some(&value);
    let issue = confirmation_issue(&value);
    let mut focus_target = use_signal(|| None::<String>);
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        use wasm_bindgen::JsCast;
        if let Some(id) = focus_target() {
            focus_target.set(None);
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(&id))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            {
                let _ = element.focus();
            }
        }
    });
    rsx! {
        if compact {
            div { class: "recipe-ingredient-compact",
                div { class: "recipe-ingredient-compact-text",
                    p { "{preview}" }
                    if substitution_count > 0 {
                        small {
                            if substitution_count == 1 { "1 alternative substitution" }
                            else { "{substitution_count} alternative substitutions" }
                        }
                    }
                }
                div { class: "recipe-ingredient-compact-actions",
                    button {
                        id: "{id}-edit", r#type: "button", class: "btn btn-ghost recipe-ingredient-icon-action",
                        aria_label: "Edit {label}", title: "Edit {label}",
                        onclick: {
                            let id = id.clone();
                            move |_| { confirmed.set(None); focus_target.set(Some(format!("{id}-name"))); }
                        },
                        IngredientActionIcon {}
                    }
                    button {
                        r#type: "button", class: "btn btn-danger recipe-ingredient-icon-action recipe-ingredient-remove",
                        aria_label: "Remove {label}", title: "Delete {label}", onclick: move |_| onremove.call(()),
                        IngredientActionIcon { delete: true }
                    }
                }
            }
        } else {
            div { class: "recipe-ingredient-basic",
                div { class: "recipe-ingredient-name",
                    label { r#for: "{id}-name", "Ingredient" }
                    IngredientAutocomplete {
                        id: "{id}-name", accessible_name: "{label} name",
                        value: value.name.clone(), maxlength: crate::types::MAX_INGREDIENT_NAME_CHARS as i64,
                        oninput: {
                            let mut value = value.clone();
                            move |(name, vocabulary_id)| { value.name = name; value.vocabulary_id = vocabulary_id; onchange.call(value.clone()); }
                        },
                    }
                }
                label { class: "recipe-ingredient-amount", "Amount"
                    input { class: "neumo-inset input input--shared", aria_label: "{label} amount", value: value.amount.clone(), placeholder: "e.g. 2",
                        maxlength: crate::types::MAX_INGREDIENT_AMOUNT_CHARS as i64,
                        oninput: { let mut value = value.clone(); move |event| { value.amount = event.value(); onchange.call(value.clone()); } },
                    }
                }
                div { class: "recipe-ingredient-unit",
                    label { r#for: "{id}-unit", "Unit" }
                    crate::components::unit_autocomplete::UnitAutocomplete {
                        id: "{id}-unit", accessible_name: "{label} unit", align_end: false,
                        value: value.unit.clone(), placeholder: "Unit",
                        maxlength: crate::types::MAX_INGREDIENT_UNIT_CHARS as i64,
                        oninput: { let mut value = value.clone(); move |unit| { value.unit = unit; onchange.call(value.clone()); } },
                    }
                }
                label { class: "recipe-ingredient-optional",
                    input { r#type: "checkbox", aria_label: "{label} optional", checked: value.optional,
                        onchange: { let mut value = value.clone(); move |event| { value.optional = event.checked(); onchange.call(value.clone()); } },
                    }
                    "Optional"
                }
                button {
                    r#type: "button", class: "recipe-ingredient-modifier-toggle touch-target",
                    aria_label: "{label} modifiers", aria_expanded: modifiers_open(), aria_controls: "{id}-modifiers",
                    onclick: move |_| { let open = modifiers_open(); modifiers_open.set(!open); },
                    span { aria_hidden: "true", if modifiers_open() { "▾" } else { "▸" } }
                    "Modifiers"
                }
                button { r#type: "button", class: "btn btn-danger recipe-ingredient-icon-action recipe-ingredient-remove", aria_label: "Remove {label}", title: "Delete {label}", onclick: move |_| onremove.call(()), IngredientActionIcon { delete: true } }
            }
            div { id: "{id}-modifiers", class: "recipe-ingredient-modifier-section", hidden: !modifiers_open(),
                div { class: "recipe-ingredient-options",
                    p { class: "recipe-editor__hint", "add extra details about the ingredient" }
                    div { class: "recipe-ingredient-modifiers",
                        label { "Text before ingredient"
                            input { class: "neumo-inset input input--shared", aria_label: "{label} text before", value: value.text_before.clone(), placeholder: "e.g. Acme",
                                maxlength: crate::types::MAX_INGREDIENT_MODIFIER_CHARS as i64,
                                oninput: { let mut value = value.clone(); move |event| { value.text_before = event.value(); onchange.call(value.clone()); } },
                            }
                        }
                        label { "Text after ingredient"
                            input { class: "neumo-inset input input--shared", aria_label: "{label} text after", value: value.text_after.clone(), placeholder: "e.g. drained and rinsed",
                                maxlength: crate::types::MAX_INGREDIENT_MODIFIER_CHARS as i64,
                                oninput: { let mut value = value.clone(); move |event| { value.text_after = event.value(); onchange.call(value.clone()); } },
                            }
                        }
                    }
                }
            }
            if !value.name.trim().is_empty() {
                p { class: "recipe-ingredient-preview", "{preview}" }
            }
            {children}
            div { class: "recipe-ingredient-confirm-actions",
                if let Some(issue) = issue {
                    p { id: "{id}-confirmation-hint", class: "recipe-editor__hint", "{issue}" }
                }
                button {
                    r#type: "button", class: "btn btn-primary touch-target",
                    aria_label: "Confirm {label}",
                    aria_describedby: issue.map(|_| format!("{id}-confirmation-hint")),
                    disabled: issue.is_some(),
                    onclick: move |_| {
                        if confirmation_issue(&value).is_none() {
                            confirmed.set(Some(value.clone()));
                            focus_target.set(Some(format!("{id}-edit")));
                        }
                    },
                    "Done"
                }
            }
        }
    }
}

fn confirmation_issue(value: &RecipeIngredient) -> Option<&'static str> {
    if value.name.trim().is_empty() {
        Some("Add an ingredient name before finishing.")
    } else if noms::units::canonicalize(&value.unit).is_err() {
        Some("Choose a supported unit before finishing.")
    } else if value.substitutes.iter().any(|substitute| {
        substitute.name.trim().is_empty() || noms::units::canonicalize(&substitute.unit).is_err()
    }) {
        Some("Complete each substitution's name and unit before finishing.")
    } else {
        None
    }
}

#[component]
fn IngredientActionIcon(#[props(default)] delete: bool) -> Element {
    rsx! {
        svg {
            width: "16", height: "16", view_box: "0 0 24 24",
            fill: "none", stroke: "currentColor", stroke_width: "1.8",
            stroke_linecap: "round", stroke_linejoin: "round",
            "aria-hidden": "true", "focusable": "false",
            path { d: if delete {
                "M3 6h18M19 6l-1 14H6L5 6M9 6V3h6v3M10 10v7M14 10v7"
            } else {
                "m15 4 5 5M4 20l5-1L21 7l-5-5L4 14v6Z"
            } }
        }
    }
}

/// The same authored wording is visible in current, draft and historical views.
#[component]
pub(super) fn RecipeIngredientLine(
    ingredient: RecipeIngredient,
    #[props(default)] amount: Option<String>,
    #[props(default)] multiplier: Option<f64>,
    #[props(default = true)] checkable: bool,
    #[props(default)] control_id: Option<String>,
) -> Element {
    let amount = amount.unwrap_or_else(|| ingredient.amount.clone());
    let line = ingredient.display_line(&amount);
    rsx! {
        if checkable {
            label { class: "mobile-ingredient", r#for: control_id.clone(),
                input { id: control_id, r#type: "checkbox", aria_label: "Mark {ingredient.name} prepared" }
                span { "{line}" }
            }
        } else {
            span { "{line}" }
        }
        if !ingredient.substitutes.is_empty() {
            ul { class: "recipe-ingredient-alternatives", aria_label: "Substitutions for {ingredient.name}",
                for substitute in &ingredient.substitutes {
                    li {
                        "Or "
                        {
                            let amount = multiplier.and_then(|factor| crate::utils::recipe_scaler::parse_amount(&substitute.amount).map(|amount| crate::utils::recipe_scaler::format_amount(amount * factor))).unwrap_or_else(|| substitute.amount.clone());
                            substitute.display_line(&amount)
                        }
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    #[test]
    fn ingredient_options_render_escaped_wording_and_scale_each_substitute() {
        let mut dom = VirtualDom::new(|| {
            rsx! {
                RecipeIngredientLine {
                    amount: "4", multiplier: 2.0,
                    ingredient: RecipeIngredient {
                        amount: "2".into(), unit: "cans".into(), name: "chickpeas".into(),
                        text_before: "<Acme>".into(), text_after: ", drained".into(), optional: true,
                        substitutes: vec![RecipeIngredient { name: "white beans".into(), amount: "400".into(), unit: "g".into(), text_before: "cooked".into(), text_after: "and cooled".into(), optional: true, ..Default::default() }],
                        ..Default::default()
                    },
                }
            }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(
            html.contains("4 cans &#60;Acme&#62; chickpeas, drained (optional)"),
            "{html}"
        );
        assert!(
            html.contains("800 g cooked white beans and cooled (optional)"),
            "{html}"
        );
        assert!(!html.contains("<Acme>"));
    }
}
