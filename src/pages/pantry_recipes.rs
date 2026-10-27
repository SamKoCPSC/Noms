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
    let mut cursors = use_signal(|| vec![None::<String>]);
    let results = use_resource(move || {
        let cursor = cursors.read().last().cloned().flatten();
        let owner = account();
        async move { find_pantry_recipes(owner, pantry_id, revision, cursor).await }
    });
    let pending = *results.state().read() == UseResourceState::Pending;
    let response = results.read();
    rsx! {
        section { class: "pantry-cookable neumo-raised", aria_labelledby: "pantry-cookable-title",
            h2 { id: "pantry-cookable-title", "What can I cook?" }
            p { class: "grocery-muted pantry-cookable-intro",
                "Recipes you own or have saved in collections, using the ingredients available in this pantry."
            }
            p { class: "grocery-muted pantry-cookable-hint",
                "Ingredient names ignore capitalization. Quantities and equipment aren’t checked."
            }
            if auth.read().current_user_id != account() {
                p { role: "status", "Your account changed. Reload this page to check your recipes." }
            } else if pending {
                p { role: "status", "Finding recipes…" }
            } else {
                match response.as_ref() {
                    Some(Ok(page)) if page.pantry_id == pantry_id && page.pantry_revision == revision => rsx! {
                        if page.recipes.is_empty() {
                            p { class: "pantry-cookable-empty", role: "status",
                                "No matching recipes yet. Add ingredients or mark them available to find recipes from your library."
                            }
                        } else {
                            div { class: "pantry-cookable-grid",
                                for recipe in &page.recipes {
                                    RecipeCard { key: "{recipe.id}", recipe: recipe.clone(), framed: true }
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
