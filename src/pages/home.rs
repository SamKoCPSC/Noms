use dioxus::prelude::*;

use crate::Route;

/// Landing page with app branding.
#[component]
pub fn Home() -> Element {
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
