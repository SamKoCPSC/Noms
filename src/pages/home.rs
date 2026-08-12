use dioxus::prelude::*;

use crate::pages::search::landing_search_route;
use crate::Route;

/// Landing page with app branding.
#[component]
pub fn Home() -> Element {
    let mut search_text = use_signal(String::new);

    rsx! {
        div { class: "container",
            div {
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                justify_content: "center",
                text_align: "center",
                padding_top: "var(--space-2xl)",
                padding_bottom: "var(--space-2xl)",
                h1 {
                    font_size: "48px",
                    color: "var(--accent)",
                    margin_bottom: "var(--space-md)",
                    "🍴 Noms"
                }
                p {
                    font_size: "20px",
                    color: "var(--text-secondary)",
                    margin_bottom: "var(--space-xl)",
                    max_width: "500px",
                    "Your personal recipe library. Save, organize, and discover delicious meals."
                }
                form {
                    class: "landing-search",
                    role: "search",
                    onsubmit: move |event| {
                        event.prevent_default();
                        if let Some(route) = landing_search_route(&search_text()) {
                            navigator().push(route);
                        }
                    },
                    label { r#for: "landing-search-query", class: "sr-only", "Search Noms" }
                    input {
                        id: "landing-search-query",
                        r#type: "search",
                        value: search_text(),
                        placeholder: "Search recipes, collections, and cooks",
                        autocomplete: "off",
                        oninput: move |event| search_text.set(event.value()),
                    }
                    button { class: "btn btn-primary", r#type: "submit", "Search" }
                }
                div {
                    display: "flex",
                    gap: "var(--space-md)",
                    flex_wrap: "wrap",
                    justify_content: "center",
                    Link {
                        to: Route::Dashboard {},
                        class: "btn btn-primary touch-target",
                        "Go to Dashboard"
                    }
                    Link {
                        to: Route::Explore {},
                        class: "btn btn-secondary touch-target",
                        "Explore Recipes"
                    }
                }
            }
        }
    }
}
