use dioxus::prelude::*;

use crate::components::base::BrandMark;
use crate::Route;

/// Public footer with marketing and product navigation.
#[component]
pub fn Footer() -> Element {
    rsx! {
        footer {
            class: "footer",
            div { class: "footer-inner container",
                div { class: "footer-brand",
                    Link { to: Route::Home {}, class: "footer-logo", aria_label: "Noms home", BrandMark {} span { "Noms" } }
                    p { "Recipes worth keeping, built to keep changing." }
                }
                nav { class: "footer-links", aria_label: "Footer navigation",
                    div {
                        strong { "Discover" }
                        Link { to: Route::Explore {}, "Explore recipes" }
                        Link { to: Route::About {}, "About Noms" }
                        Link { to: Route::HowTo {}, "How to use Noms" }
                    }
                    div {
                        strong { "Your kitchen" }
                        Link { to: Route::Dashboard {}, "Dashboard" }
                        Link { to: Route::RecipeNew {}, "New recipe" }
                        Link { to: Route::CollectionList {}, "Collections" }
                    }
                }
            }
            div { class: "footer-bottom container",
                p { "© 2026 Noms" }
                p { "Built with care and Dioxus." }
            }
        }
    }
}
