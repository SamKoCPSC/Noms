//! On-demand, session-owned pantry check for the live recipe details page.
use crate::api::pantry::{get_kitchen, list_kitchens};
use crate::auth::account_binding::use_form_account;
use crate::auth::context::AuthContext;
use crate::components::base::LoadRecovery;
use crate::pantry::feasibility::{check_pantry, IngredientStatus, PantryCheck};
use crate::types::RecipeIngredient;
use crate::Route;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub(super) fn RecipePantryCheck(ingredients: Vec<RecipeIngredient>) -> Element {
    let auth = crate::auth::context::use_auth();
    rsx! {
        if auth.is_authenticated {
            SignedInPantryCheck { ingredients }
        } else {
            p { class: "recipe-pantry__hint",
                Link { to: Route::Login {}, "Sign in" }
                " to check this recipe against your pantry."
            }
        }
    }
}

#[component]
fn SignedInPantryCheck(ingredients: Vec<RecipeIngredient>) -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let mut expanded = use_signal(|| false);
    let mut selected = use_signal(|| None::<Uuid>);
    let mut retry = use_signal(|| 0_u64);
    let pantries = use_resource(move || {
        let open = expanded();
        let owner = account();
        let _ = retry();
        async move {
            if !open {
                return Ok(None);
            }
            list_kitchens(owner).await.map(Some)
        }
    });
    let pantry = use_resource(move || {
        let open = expanded();
        let id = selected();
        let owner = account();
        let _ = retry();
        async move {
            match (open, id) {
                (true, Some(id)) => get_kitchen(owner, id).await.map(Some),
                _ => Ok(None),
            }
        }
    });
    use_effect(move || {
        if !expanded() || *pantries.state().read() == UseResourceState::Pending {
            return;
        }
        if let Some(Ok(Some(rows))) = pantries.read().as_ref() {
            if !rows.iter().any(|p| Some(p.id) == selected()) {
                let remembered = account().and_then(super::pantry::saved_selection);
                let choice = remembered
                    .filter(|id| rows.iter().any(|p| p.id == *id))
                    .or_else(|| rows.first().map(|p| p.id));
                if choice != selected() {
                    selected.set(choice);
                }
            }
        }
    });
    // Never show a previous selection's result during a retry or switch.
    let pending = *pantries.state().read() == UseResourceState::Pending
        || *pantry.state().read() == UseResourceState::Pending;
    let rows = pantries
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .flatten();
    let current = pantry
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .flatten()
        .filter(|p| {
            Some(p.id) == selected()
                && Some(p.owner_id) == account()
                && rows
                    .as_ref()
                    .is_some_and(|rows| rows.iter().any(|r| r.id == p.id))
        });
    let failed = matches!(pantries.read().as_ref(), Some(Err(_)))
        || matches!(pantry.read().as_ref(), Some(Err(_)));
    let account_changed = auth.read().current_user_id != account();
    rsx! {
        div { class: "recipe-pantry",
            button {
                r#type: "button",
                class: "recipe-pantry__toggle touch-target",
                aria_expanded: expanded(),
                aria_controls: "recipe-pantry-content",
                onclick: move |_| expanded.toggle(),
                span { "Check pantry" }
                span { aria_hidden: "true",
                    if expanded() {
                        "−"
                    } else {
                        "+"
                    }
                }
            }
            if expanded() {
                div {
                    id: "recipe-pantry-content",
                    class: "recipe-pantry__content",
                    if account_changed {
                        p { role: "status",
                            "Your account changed. Reload this page to check your pantry."
                        }
                    } else {
                        if let Some(options) = rows.as_ref() {
                            if options.is_empty() && !pending && !failed {
                                p {
                                    "Create a pantry with the ingredients you have to check this recipe."
                                }
                                Link {
                                    to: Route::PantryNew {},
                                    class: "btn btn-secondary touch-target",
                                    "Create pantry"
                                }
                            } else if !options.is_empty() {
                                div { class: "recipe-pantry__controls",
                                    label { r#for: "recipe-pantry-select", "Pantry" }
                                    select {
                                        id: "recipe-pantry-select",
                                        value: selected().map(|id| id.to_string()).unwrap_or_default(),
                                        onchange: move |event| {
                                            let choice = Uuid::parse_str(&event.value()).ok();
                                            selected.set(choice);
                                            if let Some(owner) = account() {
                                                super::pantry::remember_selection(owner, choice);
                                            }
                                        },
                                        option { value: "", disabled: true, selected: selected().is_none(), "Choose a pantry" }
                                        for option in options {
                                            option { value: "{option.id}", selected: selected() == Some(option.id), "{option.name}" }
                                        }
                                    }
                                }
                            }
                        }
                        if pending {
                            p { role: "status", "Checking pantry…" }
                        } else if failed {
                            LoadRecovery {
                                pending: false,
                                message: "Your pantry couldn’t be loaded. Please try again.",
                                retry_label: "Retry pantry check",
                                onretry: move |_| retry += 1,
                            }
                        } else if let Some(current) = current {
                            PantryCheckResult { check: check_pantry(&ingredients, &current.entries) }
                            p { class: "recipe-pantry__hint",
                                "Quantities aren’t checked. Ingredient names ignore capitalization. Equipment isn’t checked."
                            }
                            Link {
                                to: Route::PantryDetail {
                                    id: current.id.to_string(),
                                },
                                "Edit this pantry"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PantryCheckResult(check: PantryCheck) -> Element {
    let label = check.label();
    let missing: Vec<_> = check
        .ingredients
        .iter()
        .filter(|row| row.status == IngredientStatus::Missing)
        .collect();
    rsx! {
        div { class: "recipe-pantry__result",
            p { role: "status", class: "recipe-pantry__verdict", "{label}" }
            if !missing.is_empty() {
                ul { class: "recipe-pantry__ingredients",
                    for (index, row) in missing.iter().enumerate() {
                        li {
                            key: "{index}",
                            class: "recipe-pantry__ingredient",
                            strong { "{row.name}" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn result_lists_missing_ingredients_and_announces_the_verdict() {
        let mut dom = VirtualDom::new(|| {
            use crate::pantry::feasibility::IngredientCheck;
            let check = PantryCheck {
                ingredients: vec![
                    IngredientCheck {
                        name: "Rice".into(),
                        status: IngredientStatus::Missing,
                    },
                    IngredientCheck {
                        name: "Salt".into(),
                        status: IngredientStatus::Available,
                    },
                ],
            };
            rsx! { PantryCheckResult { check } }
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("role=\"status\""));
        assert!(html.contains("Missing ingredients"));
        assert!(html.contains("Rice"));
        assert!(!html.contains("Salt"));
        assert!(!html.contains("Missing or out of stock"));
        assert!(!html.contains("Needed:"));
        assert!(!html.contains("Pantry:"));
    }
}
