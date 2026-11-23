//! Recipes from the account's library that match the selected pantry.
use crate::api::pantry::find_pantry_recipes;
use crate::auth::account_binding::use_form_account;
use crate::auth::context::AuthContext;
use crate::components::base::{LoadRecovery, RecipeCard};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub(super) fn PantryRecipes(pantry_id: Uuid, revision: i64, on_retry: EventHandler<()>) -> Element {
    let account = use_form_account();
    let auth = use_context::<Signal<AuthContext>>();
    let mut almost = use_signal(|| false);
    let mut cursors = use_signal(|| vec![None::<String>]);
    let results = use_resource(move || {
        let cursor = cursors.read().last().cloned().flatten();
        let owner = account();
        let near = almost();
        async move { find_pantry_recipes(owner, pantry_id, revision, cursor, near).await }
    });
    let pending = *results.state().read() == UseResourceState::Pending;
    let response = results.read();
    let account_changed = auth.read().current_user_id != account();
    let page = response
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .filter(|page| {
            !account_changed && page.pantry_id == pantry_id && page.pantry_revision == revision
        });
    rsx! {
        section { class: "pantry-cookable neumo-raised", aria_labelledby: "pantry-cookable-title",
            h2 { id: "pantry-cookable-title", "What can I cook?" }
            p { class: "grocery-muted pantry-cookable-intro",
                "Recipes you own or have saved in collections, using the ingredients available in this pantry."
            }
            p { class: "grocery-muted pantry-cookable-hint",
                "Optional ingredients don’t block matches. Ingredient names ignore capitalization. Quantities and equipment aren’t checked."
            }
            div { class: "pantry-cookable-views", role: "group", aria_label: "Recipe availability",
                for (near, label) in [(false, "Cookable"), (true, "Almost cookable")] {
                    button {
                        class: if almost() == near { "btn btn-primary" } else { "btn btn-secondary" },
                        r#type: "button",
                        aria_pressed: almost() == near,
                        onclick: move |_| {
                            if almost() != near {
                                cursors.set(vec![None]);
                                almost.set(near);
                            }
                        },
                        "{label}"
                        if let Some(page) = page {
                            span { class: "pantry-cookable-count",
                                if near { "{page.almost_cookable_count}" } else { "{page.cookable_count}" }
                            }
                        }
                    }
                }
            }
            if account_changed {
                p { role: "status", "Your account changed. Reload this page to check your recipes." }
            } else if pending {
                p { role: "status", "Finding recipes…" }
            } else {
                match page {
                    Some(page) => rsx! {
                        if page.recipes.is_empty() {
                            p { class: "pantry-cookable-empty", role: "status",
                                "No matching recipes yet. Add ingredients or mark them available to find recipes from your library."
                            }
                        } else {
                            div { class: "pantry-cookable-grid",
                                for recipe in &page.recipes {
                                    div { key: "{recipe.id}", class: "pantry-cookable-match",
                                        if let Some(shortage) = page.shortages.get(&recipe.id) {
                                            div { class: "pantry-cookable-shortage",
                                                p { class: "pantry-cookable-match__status",
                                                    if shortage.missing.len() == 1 { "Missing ingredient" } else { "Missing ingredients" }
                                                }
                                                ul { class: "pantry-cookable-shortage__ingredients", role: "list",
                                                    for name in &shortage.missing {
                                                        li { key: "{name}", "{name}" }
                                                    }
                                                }
                                                if shortage.substitutions_available {
                                                    p { class: "pantry-cookable-shortage__substitutions", "Substitutions available" }
                                                }
                                                Link {
                                                    class: "pantry-cookable-shortage__link",
                                                    to: crate::Route::GroceryListFromPantry { pantry_id: pantry_id.to_string(), recipe_id: recipe.id.to_string() },
                                                    "Review missing ingredients"
                                                }
                                            }
                                        } else if let Some(count) = page.optional_ingredients_missing.get(&recipe.id) {
                                            p { class: "pantry-cookable-match__status pantry-cookable-match__status--optional",
                                                if *count == 1 { "1 optional ingredient missing" } else { "{count} optional ingredients missing" }
                                            }
                                        } else {
                                            p { class: "pantry-cookable-match__status", "All ingredients available" }
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
