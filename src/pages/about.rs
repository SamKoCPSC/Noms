use dioxus::prelude::*;

use crate::auth::context::use_auth;
use crate::components::informational::{CallToActionPanel, PageMeta};
use crate::Route;

pub(super) struct AboutSection {
    pub id: &'static str,
    number: &'static str,
    eyebrow: &'static str,
    title: &'static str,
    paragraphs: &'static [&'static str],
}

pub(super) const ABOUT_SECTIONS: &[AboutSection] = &[
    AboutSection {
        id: "capture",
        number: "01",
        eyebrow: "Capture",
        title: "Keep the details that make it work.",
        paragraphs: &["The useful version of a recipe includes more than a title and ingredient list. Noms keeps instructions, timing, servings, equipment, tags, commentary, and photos together so the knowledge behind the dish stays with it."],
    },
    AboutSection {
        id: "organize",
        number: "02",
        eyebrow: "Organize",
        title: "Build a library you can understand at a glance.",
        paragraphs: &["A flat pile works until it doesn’t. Collections and subcollections give recipes a place without forcing every cook into the same filing system.", "Organize by season, cuisine, person, project, or whatever helps you find your way back."],
    },
    AboutSection {
        id: "evolve",
        number: "03",
        eyebrow: "Evolve",
        title: "Let recipes change without erasing where they came from.",
        paragraphs: &["Every published update becomes part of the recipe’s history. Inspect an earlier snapshot, compare versions, and restore an older version as a new update.", "The trusted version remains easy to find, while experiments still have somewhere to go."],
    },
    AboutSection {
        id: "cook",
        number: "04",
        eyebrow: "Cook",
        title: "Adapt the recipe without rewriting it.",
        paragraphs: &["Scale a recipe for the people at the table or around the ingredient you have on hand. Noms adjusts quantities, servings, and timing for the cooking session while leaving the saved recipe intact."],
    },
    AboutSection {
        id: "share",
        number: "05",
        eyebrow: "Share",
        title: "Decide what leaves your kitchen.",
        paragraphs: &["Some recipes are private working notes. Some are meant for one link. Others deserve a place in the public collection.", "Noms gives recipes and collections clear visibility choices, so sharing is deliberate rather than automatic."],
    },
    AboutSection {
        id: "build",
        number: "06",
        eyebrow: "Build on good ideas",
        title: "Give credit, then take the recipe somewhere new.",
        paragraphs: &["Public recipes can be forked into private drafts. The new recipe is yours to change, while its connection to the source remains visible.", "That is the kind of community Noms is designed for: useful ideas moving between cooks without losing their history."],
    },
];

#[component]
pub fn About() -> Element {
    let auth = use_auth();
    let (primary_label, primary_route, secondary_label, secondary_route) = if auth.is_authenticated
    {
        (
            "Open your recipe library",
            Route::Dashboard {},
            "Create a recipe",
            Route::RecipeNew {},
        )
    } else {
        (
            "Explore recipes",
            Route::Explore {},
            "Learn how Noms works",
            Route::HowTo {},
        )
    };

    rsx! {
        PageMeta {
            title: "About Noms — A better home for working recipes",
            description: "Learn why Noms treats recipes as living knowledge: detailed, organized, versioned, and shared with intention.",
            path: "/about",
        }

        div { class: "marketing-page about-page",
            header { class: "about-hero container",
                div { class: "about-hero__copy",
                    p { class: "marketing-eyebrow", "Why Noms exists" }
                    h1 { "A better home for the recipes you actually cook." }
                    p { class: "marketing-lede", "A good recipe is more than a list copied once and left alone. It gathers notes, substitutions, better timings, clearer instructions, and the small decisions that turn a promising dish into a dependable one." }
                    p { class: "about-hero__closing", "Noms is built around that process." }
                }
                div { class: "about-hero__manifesto", aria_hidden: "true",
                    span { "CAPTURE" }
                    span { "ORGANIZE" }
                    strong { "COOK" }
                    span { "EVOLVE" }
                    span { "SHARE" }
                }
            }

            nav { class: "about-contents container", aria_label: "About page sections",
                for section in ABOUT_SECTIONS {
                    a { href: "#{section.id}", "{section.number} {section.eyebrow}" }
                }
            }

            div { class: "about-story container",
                for (index, section) in ABOUT_SECTIONS.iter().enumerate() {
                    section {
                        id: section.id,
                        class: if index % 2 == 0 { "about-story__section" } else { "about-story__section about-story__section--reverse" },
                        div { class: "about-story__copy",
                            p { class: "marketing-eyebrow", "{section.number} · {section.eyebrow}" }
                            h2 { "{section.title}" }
                            for paragraph in section.paragraphs {
                                p { "{paragraph}" }
                            }
                        }
                        AboutVisual { kind: section.id, number: section.number }
                    }
                }
            }

            CallToActionPanel {
                title: "Keep a recipe. Improve it. Pass it on.",
                body: "Make a place for the recipes you return to and the knowledge that makes them work.",
                primary_label,
                primary_route,
                secondary_label,
                secondary_route,
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AboutVisualProps {
    kind: &'static str,
    number: &'static str,
}

#[component]
fn AboutVisual(props: AboutVisualProps) -> Element {
    rsx! {
        div { class: "about-story__visual about-story__visual--{props.kind}", aria_hidden: "true",
            span { class: "about-story__visual-number", "{props.number}" }
            div { class: "about-story__plate" }
            div { class: "about-story__card",
                span { "NOMS" }
                strong { "{props.kind}" }
            }
            div { class: "about-story__accent" }
        }
    }
}
