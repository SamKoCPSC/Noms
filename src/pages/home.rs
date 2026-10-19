use dioxus::prelude::*;

use crate::auth::context::use_auth;
use crate::components::base::{NavigationArtwork, PageIconKind, SharedModal};
use crate::components::informational::PageMeta;
use crate::Route;

const HOME_CSS: Asset = asset!("/assets/home.css");
const TABLE: Asset = asset!("/assets/landing/table.webp");
const COOKING: Asset = asset!("/assets/landing/cooking.webp");
const MOBILE_BROWSE: Asset = asset!("/assets/landing/mobile-browse.webp");
const MOBILE_EDIT: Asset = asset!("/assets/landing/mobile-edit.webp");
const COOK_ONE: Asset = asset!("/assets/landing/cook-1.webp");
const COOK_TWO: Asset = asset!("/assets/landing/cook-2.webp");
const COOK_THREE: Asset = asset!("/assets/landing/cook-3.webp");

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
            label: "Start your recipe book",
            destination: HomeCtaDestination::LoginThenCreateRecipe,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Story {
    number: &'static str,
    title: &'static str,
    body: &'static str,
    caption: &'static str,
    image: Asset,
    size: (u32, u32),
}
const STORIES: &[Story] = &[
    Story { number: "01", title: "Discover new recipes.", body: "Find dishes you've never tried and get inspiration for your next weekend meal.", caption: "Croissants, chocolate cake, and Korean fried chicken in Explore.", image: asset!("/assets/landing/discover.webp"), size: (1120, 780) },
    Story { number: "02", title: "Create your signature recipe.", body: "Save all your favorite recipes, and easily make changes as you refine them.", caption: "Refine a caramelized onion and mushroom burger in a private draft.", image: asset!("/assets/landing/create.webp"), size: (1120, 780) },
    Story { number: "03", title: "Make cooking your favorites convenient", body: "Easily adjust proportions with the recipe calculator, make use of the built-in ingredients checklist, and cook with cooking mode.", caption: "Scale chili garlic oil noodles, then check off the ingredients.", image: asset!("/assets/landing/calculator-checklist.webp"), size: (900, 958) },
    Story { number: "04", title: "Study how your recipes change.", body: "View a precise breakdown of how your recipes change over time to see what works and what doesn't.", caption: "Compare two batches of chocolate chip cookies, ingredient by ingredient.", image: asset!("/assets/landing/history.webp"), size: (1120, 1200) },
    Story { number: "05", title: "Share your best creations.", body: "Publish and share recipes you're proud of, and let others know what you're working on.", caption: "Robert McRobertson’s public recipes and collections.", image: asset!("/assets/landing/share.webp"), size: (1120, 900) },
];

const FEATURES: &[(PageIconKind, &str, &str)] = &[
    (PageIconKind::Editor, "Recipe editor", "Keep ingredients, amounts, instructions, timings, servings, equipment, and notes together."),
    (PageIconKind::Editor, "Recipe and step photos", "Add photos to the whole recipe or to individual steps when a picture helps explain."),
    (PageIconKind::Editor, "Photo editing", "Crop, rotate, and adjust your photos before saving them."),
    (PageIconKind::History, "Autosaved drafts", "Work on a private draft and keep your edits as you go. Publish when you’re ready."),
    (PageIconKind::Editor, "Clipboard import", "Paste a copied recipe from a supported website into an editable draft."),
    (PageIconKind::Collections, "Nested collections", "Group recipes in collections and subcollections, then arrange them in your own order."),
    (PageIconKind::Explore, "Search and filters", "Browse public recipes in Explore. Sign in to search recipes, collections, and cooks with filters."),
    (PageIconKind::Editor, "Servings calculator", "Adjust portions for your table and recalculate the ingredient quantities."),
    (PageIconKind::Editor, "Ingredient-based scaling", "Have a fixed amount of one ingredient? Scale the rest of the recipe around it."),
    (PageIconKind::Editor, "Ingredient checklist", "Check off ingredients as you prepare them in the mobile recipe view."),
    (PageIconKind::Editor, "Step-by-step cooking", "Follow one step at a time in a focused mobile cooking view."),
    (PageIconKind::History, "Version history", "Keep a record of published updates so earlier versions remain available."),
    (PageIconKind::History, "Version comparisons", "See the before and after of each field to understand exactly what changed."),
    (PageIconKind::History, "Restore a version", "Bring an earlier published version back as a new update."),
    (PageIconKind::Explore, "Adapt with attribution", "Fork a recipe into a private draft while keeping the original source attached."),
    (PageIconKind::Collections, "Visibility controls", "Choose private, unlisted, or public access for recipes and collections."),
    (PageIconKind::Explore, "Public cook profiles", "Share your published recipes and collections on your own cook profile."),
    (PageIconKind::Collections, "JSON import and export", "Move recipes with Noms JSON. Include a saved-image archive when exporting."),
];

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    let mut paused = use_signal(|| false);
    rsx! {
        document::Link { rel: "stylesheet", href: HOME_CSS }
        PageMeta {
            title: "Noms — The Ultimate Recipe Manager",
            description: "Noms is a community-focused recipe archive designed for saving, sharing, and developing recipes",
            path: "/",
        }
        div { class: "home-page landing-page",
            section { class: "landing-hero landing-wrap", aria_labelledby: "home-title",
                div { class: "landing-hero-copy",
                    h1 { id: "home-title", "The Ultimate Recipe Manager" }
                    p { class: "landing-intro", "Noms is a community-focused recipe archive designed for saving, sharing, and developing recipes" }
                    LandingActions { signed_in: auth.is_authenticated }
                }
                div { class: "landing-hero-visual",
                    img { src: TABLE, alt: "A shared table with homemade pasta, soup, bread, and salad", width: "1536", height: "1024", fetchpriority: "high" }
                    div { class: "landing-floating-note",
                        strong { "A recipe is a starting point." }
                        span { "Keep learning. Keep cooking." }
                    }
                }
            }
            div { class: "landing-promises landing-wrap", aria_label: "Noms commitments",
                for promise in ["For every kind of cook", "Free to use", "No intrusive advertising", "Your data stays yours"] {
                    span { "{promise}" }
                }
            }
            section { class: "landing-mission landing-wrap", aria_labelledby: "landing-mission-title",
                div { class: "landing-section-head",
                    p { class: "landing-eyebrow", "Why Noms exists" }
                    h2 { id: "landing-mission-title", "Create and share your signature recipes." }
                    p { "Noms makes it easier for cooks from all backgrounds to save and discover their best dishes. Bringing together a community that's passionate about making and eating great food." }
                }
                figure {
                    img { src: COOKING, alt: "Two home cooks preparing a meal together in a sunlit kitchen", width: "1536", height: "1024", loading: "lazy", decoding: "async" }
                    figcaption { "Good food brings people together." }
                }
            }
            section { class: "landing-chapters landing-wrap", aria_label: "Five ways to use Noms",
                for story in STORIES.iter().copied() {
                    article { class: "landing-chapter", key: "{story.number}",
                        div { class: "landing-chapter-copy",
                            span { class: "landing-chapter-number", "{story.number}" }
                            h2 { "{story.title}" }
                            p { "{story.body}" }
                        }
                        LandingScreenshot { source: story.image, title: story.title, caption: story.caption, size: story.size }
                    }
                }
            }
            section { class: "landing-features landing-wrap", id: "landing-features", aria_labelledby: "landing-features-title",
                div { class: "landing-section-head",
                    p { class: "landing-eyebrow", "What you can do" }
                    h2 { id: "landing-features-title", "Recipe features, at a glance." }
                    p { "Tools for writing, organizing, cooking, and keeping track of your recipes." }
                }
                div { class: "landing-feature-grid",
                    for (index, (icon, title, body)) in FEATURES.iter().enumerate() {
                        article { class: "landing-feature-card", key: "{title}",
                            div { class: "landing-feature-top",
                                NavigationArtwork { kind: *icon }
                                span { aria_hidden: "true", "{index + 1:02}" }
                            }
                            h3 { "{title}" }
                            p { "{body}" }
                        }
                    }
                }
            }
            section { class: "landing-mobile landing-wrap", aria_labelledby: "landing-mobile-title",
                div { class: "landing-section-head",
                    p { class: "landing-eyebrow", "Noms on mobile" }
                    h2 { id: "landing-mobile-title", "Your recipes, wherever you cook." }
                    p { "Look up a recipe at the shops, refine an idea on the sofa, or keep the method beside you at the stove. Noms brings the same recipe tools to a layout made for your phone." }
                }
                div { class: "landing-mobile-grid",
                    MobilePreview { source: MOBILE_BROWSE, number: "01", title: "Find your next recipe", body: "Browse dishes and open your library with navigation that stays within reach." }
                    MobilePreview { source: MOBILE_EDIT, number: "02", title: "Keep an idea moving", body: "Edit a recipe in clear sections, with your draft saved while you work." }
                    MobilePreview { source: COOK_ONE, number: "03", title: "A hand in the kitchen", body: "Adjust portions, check ingredients, and follow the method one step at a time.", animate: true, paused: paused() }
                }
                div { class: "landing-motion-control",
                    button { class: "btn btn-secondary", r#type: "button", aria_pressed: paused(), onclick: move |_| { let next = !paused(); paused.set(next); },
                        if paused() { "Play animation" } else { "Pause animation" }
                    }
                    p { "Actual Noms screens. Tap any screen to view it larger." }
                }
            }
            section { class: "landing-trust landing-wrap", aria_labelledby: "landing-trust-title",
                div { class: "landing-section-head",
                    p { class: "landing-eyebrow", "Our approach" }
                    h2 { id: "landing-trust-title", "Here for your cooking." }
                    p { "Noms is free to use. We want a useful place to cook and share, built on respect for the people who use it." }
                }
                div { class: "landing-trust-grid",
                    for (index, title, body) in [
                        ("01", "Free to use.", "Save, manage, develop, and share your recipes without a subscription."),
                        ("02", "No intrusive advertising.", "Space to focus on recipes, without advertising getting in the way of cooking."),
                        ("03", "Your data stays yours.", "Your data is never shared or sold. It isn’t used to manipulate you into buying other products.")
                    ] {
                        article { span { aria_hidden: "true", "{index}" } h3 { "{title}" } p { "{body}" } }
                    }
                }
            }
            section { class: "landing-closing landing-wrap", aria_labelledby: "landing-closing-title",
                p { class: "landing-eyebrow", "You can start with one recipe." }
                h2 { id: "landing-closing-title", "Begin your kitchen journal." }
                LandingActions { signed_in: auth.is_authenticated, create: true }
                div { class: "landing-more-links",
                    Link { to: Route::About {}, "About Noms" }
                    Link { to: Route::HowTo {}, "How to use Noms" }
                }
            }
        }
    }
}

#[component]
fn LandingActions(signed_in: bool, #[props(default)] create: bool) -> Element {
    let cta = home_cta(signed_in);
    rsx! {
        div { class: "landing-actions",
            if cta.destination == HomeCtaDestination::Dashboard {
                if create { Link { to: Route::RecipeNew {}, class: "btn btn-primary touch-target", "Create a recipe →" } }
                else { Link { to: Route::Dashboard {}, class: "btn btn-primary touch-target", "{cta.label} →" } }
            } else {
                a { href: "/login?redirect_uri=/recipes/new", class: "btn btn-primary touch-target", "{cta.label} →" }
            }
            Link { to: Route::Explore {}, class: "btn btn-secondary touch-target", "Explore recipes" }
        }
        p { class: "landing-tiny", "Free to use · Browse public recipes without an account" }
    }
}

#[component]
fn LandingScreenshot(
    source: Asset,
    title: &'static str,
    caption: &'static str,
    #[props(default = (1120, 780))] size: (u32, u32),
    #[props(default)] phone: bool,
    #[props(default)] animate: bool,
    #[props(default)] paused: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let id = use_hook(|| format!("landing-shot-{}", uuid::Uuid::new_v4()));
    let title_id = format!("{id}-title");
    let close_id = format!("{id}-close");
    rsx! {
        figure { class: if phone { "landing-phone" } else { "landing-app-window" },
            if !phone {
                div { class: "landing-window-bar", aria_hidden: "true", i {} i {} i {} span { "Noms / {title}" } }
            }
            button { class: "landing-shot-button", r#type: "button", aria_label: "Enlarge screenshot: {title}", onclick: move |_| open.set(true),
                if animate {
                    div { class: if paused { "landing-cooking-frames is-paused" } else { "landing-cooking-frames" },
                        img { src: source, alt: "Noms mobile cooking mode: miso mushroom ramen, step one", width: "390", height: "780", loading: "lazy", decoding: "async" }
                        img { src: COOK_TWO, alt: "", aria_hidden: "true", width: "390", height: "780", loading: "lazy", decoding: "async" }
                        img { src: COOK_THREE, alt: "", aria_hidden: "true", width: "390", height: "780", loading: "lazy", decoding: "async" }
                    }
                } else {
                    img { src: source, alt: caption, width: "{size.0}", height: "{size.1}", loading: "lazy", decoding: "async" }
                }
            }
            if !phone { figcaption { "{caption}" span { "View larger →" } } }
        }
        if open() {
            SharedModal { aria_labelledby: title_id.clone(), focus_selector: format!("#{close_id}"), on_dismiss: move |_| open.set(false),
                div { class: "landing-screenshot-dialog", onclick: move |event| event.stop_propagation(),
                    header {
                        h2 { id: title_id, "{title}" }
                        button { id: close_id, class: "btn btn-secondary", r#type: "button", onclick: move |_| open.set(false), "Close screenshot" }
                    }
                    div { class: "landing-screenshot-scroll",
                        img { class: if phone { "landing-enlarged-phone" } else { "landing-enlarged-desktop" }, src: source, alt: caption }
                    }
                }
            }
        }
    }
}

#[component]
fn MobilePreview(
    source: Asset,
    number: &'static str,
    title: &'static str,
    body: &'static str,
    #[props(default)] animate: bool,
    #[props(default)] paused: bool,
) -> Element {
    rsx! {
        article { class: "landing-mobile-card",
            LandingScreenshot { source, title, caption: title, size: (390, 780), phone: true, animate, paused }
            span { class: "landing-mobile-index", "{number} / MOBILE" }
            h3 { "{title}" }
            p { "{body}" }
        }
    }
}
