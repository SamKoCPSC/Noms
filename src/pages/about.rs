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
        title: "Keep the little details.",
        paragraphs: &["The useful version of a recipe includes more than a title and ingredient list. Noms keeps instructions, timing, servings, equipment, tags, commentary, and photos together so the knowledge behind the dish stays with it."],
    },
    AboutSection {
        id: "organize",
        number: "02",
        eyebrow: "Organize",
        title: "Everything in its place.",
        paragraphs: &["A flat pile works until it doesn’t. Collections and subcollections give recipes a place without forcing every cook into the same filing system.", "Organize by season, cuisine, person, project, or whatever helps you find your way back."],
    },
    AboutSection {
        id: "evolve",
        number: "03",
        eyebrow: "Evolve",
        title: "Room to grow.",
        paragraphs: &["Every published update becomes part of the recipe’s history. Inspect an earlier snapshot, compare versions, and restore an older version as a new update.", "The trusted version remains easy to find, while experiments still have somewhere to go."],
    },
    AboutSection {
        id: "cook",
        number: "04",
        eyebrow: "Cook",
        title: "Make it fit your table.",
        paragraphs: &["Scale a recipe for the people at the table or around the ingredient you have on hand. Noms adjusts quantities, servings, and timing for the cooking session while leaving the saved recipe intact."],
    },
    AboutSection {
        id: "share",
        number: "05",
        eyebrow: "Share",
        title: "Share on your terms.",
        paragraphs: &["Some recipes are private working notes. Some are meant for one link. Others deserve a place in the public collection.", "Noms gives recipes and collections clear visibility choices, so sharing is deliberate rather than automatic."],
    },
    AboutSection {
        id: "build",
        number: "06",
        eyebrow: "Build on good ideas",
        title: "A new take, not a blank page.",
        paragraphs: &["Public recipes can be forked into private drafts. The new recipe is yours to change, while its connection to the source remains visible.", "That is the kind of community Noms is designed for: useful ideas moving between cooks without losing their history."],
    },
];

#[component]
pub fn About() -> Element {
    let auth = use_auth();
    let (primary_label, primary_route, primary_href, secondary_label, secondary_route) =
        if auth.is_authenticated {
            (
                "Start a new recipe",
                Route::RecipeNew {},
                None,
                "Open your recipe library",
                Route::Dashboard {},
            )
        } else {
            (
                "Start your recipe book",
                Route::Login {},
                Some("/login?redirect_uri=/recipes/new"),
                "Explore recipes",
                Route::Explore {},
            )
        };

    rsx! {
        PageMeta {
            title: "About Noms — Good recipes deserve a little home",
            description: "Learn why Noms treats recipes as living knowledge: detailed, organized, versioned, and shared with intention.",
            path: "/about",
        }

        div { class: "marketing-page about-page",
            header { class: "about-hero container", aria_labelledby: "about-title",
                div { class: "about-hero__copy",
                    p { class: "marketing-eyebrow", "Why Noms exists" }
                    h1 { id: "about-title", "Good recipes deserve a little home." }
                    p { class: "marketing-lede", "Noms keeps your recipes, notes, and kitchen discoveries together, with room for every version to grow." }
                    div { class: "marketing-actions",
                        if auth.is_authenticated {
                            Link { to: Route::RecipeNew {}, class: "btn btn-primary touch-target", "Start a new recipe" }
                        } else {
                            a { href: "/login?redirect_uri=/recipes/new", class: "btn btn-primary touch-target", "Start your recipe book" }
                        }
                        Link { to: Route::Explore {}, class: "btn btn-secondary touch-target", "Explore recipes" }
                    }
                }
                div { class: "about-hero__illustration", aria_hidden: "true",
                    div { class: "about-hero__cloth" }
                    div { class: "about-hero__book",
                        div { class: "about-hero__book-spine" }
                        div { class: "about-hero__book-label", "Recipe notes" }
                        div { class: "about-hero__book-band" }
                        div { class: "about-hero__book-pages" }
                        div { class: "about-hero__bookmark" }
                    }
                    div { class: "about-hero__mug" }
                    div { class: "about-hero__note", span { "Kitchen notes" } span { "A little more lemon." } }
                    div { class: "about-hero__sprig",
                        span {}
                        span {}
                        span {}
                        span {}
                    }
                }
            }

            nav { class: "about-contents container", aria_label: "About page sections",
                for section in ABOUT_SECTIONS {
                    a { href: "#{section.id}", "{section.number} {section.eyebrow}" }
                }
            }

            div { class: "about-story container",
                for section in ABOUT_SECTIONS {
                    section {
                        id: section.id,
                        class: "about-story__section",
                        AboutVisual { kind: section.id }
                        div { class: "about-story__copy",
                            p { class: "marketing-eyebrow", "{section.number} · {section.eyebrow}" }
                            h2 { "{section.title}" }
                            for paragraph in section.paragraphs {
                                p { "{paragraph}" }
                            }
                        }
                    }
                }
            }

            CallToActionPanel {
                title: "Something worth keeping?",
                body: "Make a place for the recipes you return to and the knowledge that makes them work.",
                primary_label,
                primary_route,
                primary_href,
                secondary_label,
                secondary_route,
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AboutVisualProps {
    kind: &'static str,
}

#[component]
fn AboutVisual(props: AboutVisualProps) -> Element {
    rsx! {
        div { class: "about-story__visual about-story__visual--{props.kind}", aria_hidden: "true",
            if props.kind == "evolve" {
                div { class: "about-story__versions",
                    span { small { "VERSION 1" } strong { "The original" } i {} i {} }
                    span { small { "VERSION 2" } strong { "Less sugar" } i {} i {} }
                    span { small { "VERSION 3" } strong { "Just right" } i {} i {} }
                }
            } else if props.kind == "share" {
                div { class: "about-story__visibility",
                    span { strong { "Private" } small { "Only you" } }
                    span { strong { "Unlisted" } small { "Anyone with the link" } }
                    span { strong { "Public" } small { "Everyone" } }
                }
            } else if props.kind == "organize" {
                div { class: "about-story__folders",
                    span { "Weeknight favorites" }
                    span { "Something sweet" }
                    span { "Family recipes" }
                }
            } else if props.kind == "cook" {
                div { class: "about-story__scaling",
                    div { small { "SERVINGS" } strong { "2" } }
                    span { "→" }
                    div { small { "SERVINGS" } strong { "6" } }
                    p { "Same recipe. More seats at the table." }
                }
            } else if props.kind == "build" {
                div { class: "about-story__lineage",
                    div { small { "ORIGINAL" } strong { "Lemon cake" } }
                    span { "↳" }
                    div { small { "YOUR TAKE" } strong { "Lemon & poppy seed" } }
                }
            } else {
                div { class: "about-story__card",
                    span { "FROM THE RECIPE BOOK" }
                    strong { "Lemon herb pasta" }
                    p { "Extra zest. A little more garlic." }
                    i {} i {}
                }
                div { class: "about-story__accent" }
            }
        }
    }
}
