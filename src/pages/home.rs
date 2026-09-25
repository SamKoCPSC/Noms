use dioxus::prelude::*;

const HOME_CSS: Asset = asset!("/assets/home.css");

use crate::auth::context::use_auth;
use crate::components::informational::{CallToActionPanel, PageMeta};
use crate::pages::search::landing_search_route;
use crate::Route;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HomeCtaDestination {
    LoginThenCreateRecipe,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HomeCta {
    pub label: &'static str,
    pub destination: HomeCtaDestination,
}

pub(super) fn home_cta(is_authenticated: bool) -> HomeCta {
    if is_authenticated {
        HomeCta {
            label: "Open dashboard",
            destination: HomeCtaDestination::Dashboard,
        }
    } else {
        HomeCta {
            label: "Start creating",
            destination: HomeCtaDestination::LoginThenCreateRecipe,
        }
    }
}

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    let mut search_text = use_signal(String::new);
    let primary_cta = home_cta(auth.is_authenticated);
    let final_primary_label = if auth.is_authenticated {
        "Create a recipe"
    } else {
        "Start your first recipe"
    };
    let final_primary_route = if auth.is_authenticated {
        Route::RecipeNew {}
    } else {
        Route::Login {}
    };
    let final_primary_href = (!auth.is_authenticated).then_some("/login?redirect_uri=/recipes/new");

    rsx! {
        document::Link { rel: "stylesheet", href: HOME_CSS }
        PageMeta {
            title: "Noms — Recipes worth keeping",
            description: "Capture recipes in detail, organize your library, preserve every published version, and share on your terms.",
            path: "/",
        }

        div { class: "marketing-page home-page",
            section { class: "marketing-hero home-hero container", aria_labelledby: "home-title",
                div { class: "marketing-hero__copy",
                    p { class: "marketing-eyebrow", "Your recipe library, made to evolve" }
                    h1 { id: "home-title", "A cozy home for your recipes." }
                    p { class: "marketing-lede",
                        "Save the meals you love, keep your notes, and make each version your own."
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
                    div { class: "marketing-actions",
                        match primary_cta.destination {
                            HomeCtaDestination::Dashboard => rsx! {
                                Link { to: Route::Dashboard {}, class: "btn btn-primary touch-target", "{primary_cta.label}" }
                            },
                            HomeCtaDestination::LoginThenCreateRecipe => rsx! {
                                a { href: "/login?redirect_uri=/recipes/new", class: "btn btn-primary touch-target", "{primary_cta.label}" }
                            },
                        }
                        Link { to: Route::Explore {}, class: "btn btn-secondary touch-target", "Explore recipes" }
                    }
                    p { class: "marketing-note", "Private drafts give you room to work before anything is published." }
                }

                div { class: "home-hero-illustration", aria_hidden: "true",
                    div { class: "home-hero-illustration__glow" }
                    div { class: "home-hero-illustration__sprig home-hero-illustration__sprig--left" }
                    div { class: "home-hero-illustration__sprig home-hero-illustration__sprig--right" }
                    div { class: "home-hero-illustration__book",
                        span { class: "home-hero-illustration__pages" }
                        span { class: "home-hero-illustration__spine" }
                        span { class: "home-hero-illustration__cover" }
                        span { class: "home-hero-illustration__title", "Noms recipes" }
                        span { class: "home-hero-illustration__strap" }
                        span { class: "home-hero-illustration__bookmark" }
                    }
                    div { class: "home-hero-illustration__bowl",
                        span { class: "home-hero-illustration__soup" }
                        span { class: "home-hero-illustration__bowl-rim" }
                    }
                    div { class: "home-hero-illustration__spoon" }
                }
            }

            section { class: "home-benefits container", aria_label: "What Noms helps you do",
                HomeBenefitCard { kind: "capture", title: "Capture", body: "Keep ingredients, instructions, photos, and the little notes that make a recipe yours." }
                HomeBenefitCard { kind: "organize", title: "Organize", body: "Give every recipe a place with collections and subcollections that fit your kitchen." }
                HomeBenefitCard { kind: "personalize", title: "Make it yours", body: "Refine a recipe over time without losing the versions and ideas that came before." }
            }

            section { class: "home-version-story container", aria_labelledby: "home-version-title",
                div { class: "home-version-story__copy",
                    p { class: "marketing-eyebrow", "The recipe journal" }
                    h2 { id: "home-version-title", "A little better, every time." }
                    p { "Recipes aren’t set in stone. Keep notes, try new things, and watch your favorites get even better." }
                    Link { to: Route::HowTo {}, class: "marketing-text-link", "See how versions work →" }
                }
                ol { class: "home-version-story__notes", aria_label: "Illustrative recipe version journey",
                    li { class: "home-version-note",
                        span { class: "home-version-note__number", "1" }
                        strong { "First keeper" }
                        p { "The recipe that started it all." }
                    }
                    li { class: "home-version-note",
                        span { class: "home-version-note__number", "2" }
                        strong { "A small tweak" }
                        p { "A little more garlic next time." }
                    }
                    li { class: "home-version-note",
                        span { class: "home-version-note__number", "3" }
                        strong { "Just right" }
                        p { "A favorite worth keeping." }
                    }
                }
            }

            section { class: "marketing-feature-list container", aria_label: "Noms features",
                FeatureStory {
                    eyebrow: "A calm place to work",
                    title: "Start rough. Publish when it’s ready.",
                    body: "Recipes rarely arrive fully formed. Noms keeps new recipes and edits as private, autosaved drafts while you measure, rewrite, and add the details that matter.",
                    route: Route::RecipeNew {},
                    link_label: "Create a recipe",
                    visual_kind: "draft",
                }
                FeatureStory {
                    eyebrow: "A library with structure",
                    title: "Give every recipe a place.",
                    body: "Group recipes into collections, add subcollections when one level is not enough, and arrange everything in an order that makes sense to you.",
                    route: Route::CollectionList {},
                    link_label: "Open collections",
                    visual_kind: "collections",
                    reverse: true,
                }
                FeatureStory {
                    eyebrow: "History that stays useful",
                    title: "See how a recipe became the one you trust.",
                    body: "Each published update joins the recipe’s history. Open an earlier snapshot, compare versions, or restore a previous version as a new update.",
                    route: Route::HowTo {},
                    link_label: "Learn about versions",
                    visual_kind: "versions",
                }
            }

            section { class: "marketing-community container", aria_labelledby: "community-title",
                div { class: "marketing-community__copy",
                    p { class: "marketing-eyebrow", "Cook, adapt, contribute" }
                    h2 { id: "community-title", "Find something worth cooking—and make it your own." }
                    p {
                        "Explore public recipes and visit creator profiles to see what people have chosen to share. When a recipe sparks an idea, fork it into a private draft with its source attached."
                    }
                    div { class: "marketing-actions",
                        Link { to: Route::Explore {}, class: "btn btn-primary touch-target", "Explore recipes" }
                        Link { to: Route::HowTo {}, class: "btn btn-secondary touch-target", "How forking works" }
                    }
                }
                div { class: "marketing-community__scale-card",
                    span { class: "marketing-community__scale-label", "Cooking for more?" }
                    strong { "Scale 4 servings → 6" }
                    div { class: "marketing-community__scale-line",
                        span { "2 cups" }
                        span { "3 cups" }
                    }
                    p { "Adjust the cooking view without changing the saved recipe." }
                }
            }

            CallToActionPanel {
                title: "Make room for the recipes you return to.",
                body: "Start with one recipe. Add the details you care about, organize it your way, and let it improve each time you cook.",
                primary_label: final_primary_label,
                primary_route: final_primary_route,
                primary_href: final_primary_href,
                secondary_label: "See how Noms works",
                secondary_route: Route::HowTo {},
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct HomeBenefitCardProps {
    kind: &'static str,
    title: &'static str,
    body: &'static str,
}

#[component]
fn HomeBenefitCard(props: HomeBenefitCardProps) -> Element {
    rsx! {
        article { class: "home-benefit-card neumo-card",
            div { class: "home-benefit-card__art home-benefit-card__art--{props.kind}", aria_hidden: "true",
                span {}
                span {}
                span {}
            }
            div { class: "home-benefit-card__copy",
                h2 { "{props.title}" }
                p { "{props.body}" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct FeatureStoryProps {
    eyebrow: &'static str,
    title: &'static str,
    body: &'static str,
    route: Route,
    link_label: &'static str,
    visual_kind: &'static str,
    #[props(default)]
    reverse: bool,
}

#[component]
fn FeatureStory(props: FeatureStoryProps) -> Element {
    let class = if props.reverse {
        "marketing-feature marketing-feature--reverse"
    } else {
        "marketing-feature"
    };

    rsx! {
        article { class,
            div { class: "marketing-feature__copy",
                p { class: "marketing-eyebrow", "{props.eyebrow}" }
                h2 { "{props.title}" }
                p { "{props.body}" }
                Link { to: props.route, class: "marketing-text-link", "{props.link_label} →" }
            }
            FeatureVisual { kind: props.visual_kind }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct FeatureVisualProps {
    kind: &'static str,
}

#[component]
fn FeatureVisual(props: FeatureVisualProps) -> Element {
    match props.kind {
        "collections" => rsx! {
            div { class: "marketing-feature__visual marketing-collection-visual", aria_hidden: "true",
                div { class: "marketing-folder marketing-folder--main", span { "Weeknight dinners" } }
                div { class: "marketing-folder marketing-folder--second", span { "Bread notebook" } }
                div { class: "marketing-folder marketing-folder--third", span { "Family recipes" } }
            }
        },
        "versions" => rsx! {
            div { class: "marketing-feature__visual marketing-version-visual", aria_hidden: "true",
                div { class: "marketing-version-line" }
                div { class: "marketing-version-node", strong { "v1" } span { "The first keeper" } }
                div { class: "marketing-version-node marketing-version-node--current", strong { "v4" } span { "Current · better timing" } }
                div { class: "marketing-version-node", strong { "v2" } span { "More garlic" } }
            }
        },
        _ => rsx! {
            div { class: "marketing-feature__visual marketing-draft-visual", aria_hidden: "true",
                div { class: "marketing-draft-visual__status", span { class: "marketing-status-dot" } "Autosaved privately" }
                div { class: "marketing-draft-visual__line marketing-draft-visual__line--title" }
                div { class: "marketing-draft-visual__line" }
                div { class: "marketing-draft-visual__line marketing-draft-visual__line--short" }
                div { class: "marketing-draft-visual__ingredients",
                    span { "tomatoes" }
                    span { "basil" }
                    span { "+ one good note" }
                }
            }
        },
    }
}

#[cfg(test)]
mod design_tests {
    const HOME: &str = include_str!("home.rs");
    const HOME_CSS: &str = include_str!("../../assets/home.css");

    #[test]
    fn cozy_home_has_three_benefits_and_a_version_story() {
        assert!(HOME.contains("class: \"home-hero-illustration\""));
        assert!(HOME.contains("class: \"home-benefits container\""));
        assert_eq!(
            HOME.lines()
                .filter(|line| line.trim_start().starts_with("HomeBenefitCard { kind:"))
                .count(),
            3
        );
        assert!(HOME.contains("class: \"home-version-story container\""));
        assert_eq!(
            HOME.lines()
                .filter(|line| line
                    .trim_start()
                    .starts_with("li { class: \"home-version-note\""))
                .count(),
            3
        );
    }

    #[test]
    fn home_keeps_search_and_authenticated_creation_routes() {
        assert!(HOME.contains("landing_search_route(&search_text())"));
        assert!(HOME.contains("/login?redirect_uri=/recipes/new"));
        assert!(HOME.contains("Route::Dashboard {}"));
        assert!(HOME.contains("Route::Explore {}"));
    }

    #[test]
    fn home_illustration_is_decorative_and_layout_is_route_scoped() {
        assert!(HOME.contains("class: \"home-hero-illustration\", aria_hidden: \"true\""));
        assert!(HOME.contains("aria_label: \"Illustrative recipe version journey\""));
        assert!(HOME.contains("document::Link { rel: \"stylesheet\", href: HOME_CSS }"));
        assert!(HOME_CSS.contains(".home-page .home-hero"));
        assert!(HOME_CSS.contains("@media (max-width: 900px)"));
        assert!(HOME_CSS.contains("@media (max-width: 719px)"));
        assert!(HOME_CSS.contains("@media (forced-colors: active)"));
    }
}
