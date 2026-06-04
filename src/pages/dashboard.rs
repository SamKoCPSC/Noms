use dioxus::prelude::*;

use crate::components::base::{EmptyState, PageHeader};
use crate::components::AuthRequired;
use crate::Route;

/// User's recipe library — empty state placeholder.
#[component]
pub fn Dashboard() -> Element {
    rsx! {
        AuthRequired {
            div { class: "container",
                PageHeader {
                    title: "My Recipes",
                    action: rsx! {
                        Link {
                            to: Route::RecipeNew {},
                            class: "btn btn-primary touch-target",
                            "+ New Recipe"
                        }
                    },
                }
                EmptyState {
                    icon: rsx! { "📖" },
                    title: "No recipes yet",
                    description: "Start building your recipe library by adding your first recipe.",
                    action: rsx! {
                        Link {
                            to: Route::RecipeNew {},
                            class: "btn btn-primary touch-target",
                            "Create your first recipe"
                        }
                    },
                }
            }
        }
    }
}
