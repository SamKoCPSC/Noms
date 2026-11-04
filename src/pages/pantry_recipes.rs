//! Presence-based recipe views from the account's owned and saved library.
use crate::api::pantry::find_pantry_recipes;
use crate::auth::account_binding::use_form_account;
use crate::auth::context::AuthContext;
use crate::components::base::{LoadRecovery, RecipeCard};
use crate::pantry::{PantryRecipeMode, MAX_ALMOST_COOKABLE_MISSING};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub(super) fn PantryRecipes(
    pantry_id: Uuid,
    revision: i64,
    mut mode: Signal<PantryRecipeMode>,
    on_retry: EventHandler<()>,
) -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let selected_mode = mode();
    rsx! {
        section { class: "pantry-cookable neumo-raised", aria_labelledby: "pantry-cookable-title",
            h2 { id: "pantry-cookable-title", "What can I cook?" }
            p { class: "grocery-muted pantry-cookable-intro",
                "Recipes you own or have saved in collections, using the ingredients available in this pantry."
            }
            div { class: "pantry-recipe-views", role: "group", aria_label: "Recipe readiness",
                for (value, label) in [
                    (PantryRecipeMode::Cookable, "Can cook now"),
                    (PantryRecipeMode::AlmostCookable, "Almost cookable"),
                ] {
                    button { class: "btn btn-secondary", r#type: "button",
                        aria_pressed: selected_mode == value,
                        onclick: move |_| mode.set(value),
                        "{label}"
                    }
                }
            }
            p { class: "grocery-muted pantry-cookable-hint",
                if selected_mode == PantryRecipeMode::AlmostCookable {
                    "Missing up to {MAX_ALMOST_COOKABLE_MISSING} required ingredients, with the fewest missing first. "
                }
                "Optional ingredients don’t block matches. Ingredient names ignore capitalization. Quantities and equipment aren’t checked."
            }
            if auth.read().current_user_id != account() {
                p { role: "status", "Your account changed. Reload this page to check your recipes." }
            } else {
                PantryRecipeResults {
                    key: "{selected_mode:?}", pantry_id, revision, mode: selected_mode,
                    account: account(), on_retry,
                }
            }
        }
    }
}

#[component]
fn PantryRecipeResults(
    pantry_id: Uuid,
    revision: i64,
    mode: PantryRecipeMode,
    account: Option<Uuid>,
    on_retry: EventHandler<()>,
) -> Element {
    let auth = use_context::<Signal<AuthContext>>();
    let mut cursors = use_signal(|| vec![None::<String>]);
    let results = use_resource(move || {
        let cursor = cursors.read().last().cloned().flatten();
        async move { find_pantry_recipes(account, pantry_id, revision, cursor, Some(mode)).await }
    });
    let pending = *results.state().read() == UseResourceState::Pending;
    let response = results.read();
    rsx! {
        div { class: "pantry-recipe-results", aria_live: "polite",
            if auth.read().current_user_id != account {
                p { role: "status", "Your account changed. Reload this page to check your recipes." }
            } else if pending {
                p { role: "status", "Finding recipes…" }
            } else {
                match response.as_ref() {
                    Some(Ok(page)) if page.pantry_id == pantry_id && page.pantry_revision == revision && page.mode == mode => rsx! {
                        if page.recipes.is_empty() {
                            p { class: "pantry-cookable-empty", role: "status",
                                if mode == PantryRecipeMode::AlmostCookable {
                                    "No almost-cookable recipes yet. This view shows library recipes missing 1 or 2 required ingredients."
                                } else {
                                    "No matching recipes yet. Add ingredients or mark them available to find recipes from your library."
                                }
                            }
                        } else {
                            div { class: "pantry-cookable-grid",
                                for recipe in &page.recipes {
                                    div { key: "{recipe.id}", class: "pantry-cookable-match",
                                        RecipeMatchStatus {
                                            required: page.required_ingredients_missing.get(&recipe.id).cloned().unwrap_or_default(),
                                            optional: page.optional_ingredients_missing.get(&recipe.id).copied().unwrap_or(0),
                                        }
                                        RecipeCard { recipe: recipe.clone(), framed: true }
                                    }
                                }
                            }
                        }
                        if cursors.read().len() > 1 || page.next_cursor.is_some() {
                            nav { class: "pantry-cookable-pages", aria_label: "Cookable recipe pages",
                                button {
                                    class: "btn btn-secondary", disabled: cursors.read().len() <= 1,
                                    onclick: move |_| { if cursors.read().len() > 1 { cursors.write().pop(); } },
                                    "Previous recipes"
                                }
                                if let Some(next) = page.next_cursor.clone() {
                                    button { class: "btn btn-secondary",
                                        onclick: move |_| cursors.write().push(Some(next.clone())),
                                        "Next recipes"
                                    }
                                }
                            }
                        }
                    },
                    _ => rsx! {
                        LoadRecovery {
                            pending: false,
                            message: "Recipes couldn’t be checked. Retry to load the latest pantry and recipes.",
                            retry_label: "Retry recipe check",
                            onretry: move |_| on_retry.call(()),
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn RecipeMatchStatus(required: Vec<String>, optional: usize) -> Element {
    rsx! {
        if required.is_empty() && optional == 0 {
            p { class: "pantry-cookable-match__status", "All ingredients available" }
        }
        if !required.is_empty() {
            p { class: "pantry-cookable-match__status",
                if required.len() == 1 { "1 required ingredient missing" }
                else { "{required.len()} required ingredients missing" }
            }
            ul { class: "pantry-cookable-missing", aria_label: "Missing required ingredients",
                for name in required {
                    li { "{name}" }
                }
            }
        }
        if optional > 0 {
            p { class: "pantry-cookable-match__status pantry-cookable-match__status--optional",
                if optional == 1 { "1 optional ingredient missing" }
                else { "{optional} optional ingredients missing" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn almost_cookable_card_explains_required_and_optional_shortages() {
        let html = dioxus_ssr::render_element(rsx! {
            RecipeMatchStatus { required: vec!["Milk".into(), "<Basil>".into()], optional: 1 }
        });
        assert!(html.contains("2 required ingredients missing"));
        assert!(html.contains("Milk"));
        assert!(html.contains("&lt;Basil&gt;") || html.contains("&#60;Basil&#62;"));
        assert!(html.contains("1 optional ingredient missing"));
        assert!(!html.contains("All ingredients available"));
    }

    #[test]
    fn optional_only_shortage_is_not_presented_as_a_required_shortage() {
        let html = dioxus_ssr::render_element(rsx! {
            RecipeMatchStatus { required: vec![], optional: 2 }
        });
        assert!(html.contains("2 optional ingredients missing"));
        assert!(!html.contains("required ingredient"));
        assert!(!html.contains("All ingredients available"));
    }
}
