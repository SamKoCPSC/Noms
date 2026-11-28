use dioxus::prelude::*;

use crate::auth::context::use_auth;
use crate::components::base::SharedModal;
use crate::components::informational::PageMeta;
use crate::Route;

use super::home_scene::{HomeScene, Scene};

const HOME_CSS: Asset = asset!("/assets/home.css");
const RECIPE_DRAFT: Asset = asset!("/assets/landing/create.webp");
const RECIPE_EXPLORE: Asset = asset!("/assets/landing/discover.webp");
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
    title: &'static str,
    body: &'static str,
    caption: &'static str,
    image: Asset,
    scene: Scene,
    size: (u32, u32),
}
const COMMUNITY_STORIES: &[Story] = &[
    Story {
        title: "Discover new recipes.",
        body: "Find dishes you've never tried and get inspiration for your next weekend meal.",
        caption: "Croissants, chocolate cake, and Korean fried chicken in Explore.",
        image: RECIPE_EXPLORE,
        scene: Scene::Banquet,
        size: (1120, 780),
    },
    Story {
        title: "Share your best creations.",
        body:
            "Publish recipes you're proud of and browse what other cooks share on their profiles.",
        caption: "Robert McRobertson’s public recipes and collections.",
        image: asset!("/assets/landing/share.webp"),
        scene: Scene::MarketStand,
        size: (1120, 900),
    },
];

const RECIPE_DRAFT_STORY: Story = Story {
    title: "Create your signature recipe.",
    body: "Keep ingredients and instructions together in a private draft that saves as you work.",
    caption: "Refine a caramelized onion and mushroom burger in a private draft.",
    image: RECIPE_DRAFT,
    scene: Scene::Plating,
    size: (1120, 780),
};
const VERSION_HISTORY_STORY: Story = Story {
    title: "Study how your recipes change.",
    body: "Compare published versions and restore an earlier one when you need it.",
    caption: "Compare two batches of chocolate chip cookies, ingredient by ingredient.",
    image: asset!("/assets/landing/history.webp"),
    scene: Scene::CookieStudy,
    size: (1120, 1200),
};

#[derive(PartialEq)]
struct Feature {
    title: &'static str,
    body: &'static str,
    image: Asset,
    caption: &'static str,
    size: (u32, u32),
}

const PANTRY_FEATURES: &[Feature] = &[
    Feature {
        title: "See what you have",
        body: "Track the ingredients and quantities you have on hand.",
        image: asset!("/assets/landing/features/pantry.webp"),
        caption: "A Kitchen Pantry with ingredient quantities.",
        size: (955, 600),
    },
    Feature {
        title: "Build a grocery list",
        body: "Start with the ingredients a recipe needs, then list only what’s missing from your pantry.",
        image: asset!("/assets/landing/features/grocery-list.webp"),
        caption: "A grocery checklist with quantities and an option to put purchased ingredients away.",
        size: (944, 590),
    },
];

const RECIPE_TOOLS: &[Feature] = &[
    Feature {
        title: "Built-in recipe image editor",
        body: "Crop, rotate, and adjust recipe photos right in Noms.",
        image: asset!("/assets/landing/features/image-editor.webp"),
        caption: "Fish tacos with pickled onions in a preview of the image editor’s crop, rotation, and color controls.",
        size: (1084, 752),
    },
    Feature {
        title: "Scale your servings",
        body: "Change the serving count and let Noms recalculate the ingredient amounts.",
        image: asset!("/assets/landing/features/servings.webp"),
        caption: "The recipe calculator scales chili garlic oil noodles from two to four servings.",
        size: (440, 572),
    },
    Feature {
        title: "Organize your recipes",
        body: "Group favorites in collections so they’re easy to find again.",
        image: asset!("/assets/landing/features/collections.webp"),
        caption: "Weeknight dinners, Weekend baking, and Dinner with friends, each with a two-photo recipe cover.",
        size: (1108, 716),
    },
    Feature {
        title: "Import and export",
        body: "Back up and re-import recipes and collections with Noms JSON, or export HTML copies to read, share, or print.",
        image: asset!("/assets/landing/features/recipe-export.webp"),
        caption: "The Export recipe dialog showing JSON and HTML formats, with an option to download saved images.",
        size: (640, 546),
    },
];

#[component]
fn LandingChapter(story: Story, #[props(default)] community: bool) -> Element {
    rsx! {
        article { class: "landing-chapter",
            div { class: "landing-chapter-copy",
                if community {
                    h3 { "{story.title}" }
                } else {
                    h2 { "{story.title}" }
                }
                p { "{story.body}" }
                HomeScene { scene: story.scene }
            }
            LandingScreenshot { source: story.image, title: story.title, caption: story.caption, size: story.size }
        }
    }
}

#[component]
fn FeatureDescriptions(features: &'static [Feature]) -> Element {
    rsx! {
        div { class: "landing-feature-descriptions",
            for feature in features {
                div { key: "{feature.title}",
                    h3 { "{feature.title}" }
                    p { "{feature.body}" }
                }
            }
        }
    }
}

#[component]
fn FeaturePreviews(features: &'static [Feature]) -> Element {
    rsx! {
        div { class: "landing-feature-previews",
            for feature in features {
                LandingScreenshot {
                    key: "{feature.title}",
                    source: feature.image, title: feature.title, caption: feature.caption, size: feature.size,
                }
            }
        }
    }
}

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
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
                    LandingActions { signed_in: auth.is_authenticated, search: true }
                }
                div { class: "landing-hero-visual",
                    HomeScene { scene: Scene::Kitchen }
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
                HomeScene { scene: Scene::RecipeDesk, caption: "Good food brings people together." }
            }
            div { class: "landing-chapters landing-wrap",
                section { class: "landing-community", aria_labelledby: "landing-community-title",
                    div { class: "landing-section-head",
                        h2 { id: "landing-community-title", "Community" }
                        p { "Discover recipes, share your own, and see what other cooks are making." }
                    }
                    for story in COMMUNITY_STORIES.iter().copied() {
                        LandingChapter { key: "{story.title}", story, community: true }
                    }
                    aside { class: "landing-chapter landing-discussion", aria_labelledby: "landing-discussion-title",
                        div { class: "landing-chapter-copy",
                            h3 { id: "landing-discussion-title", "Community discussions" }
                            p { "In progress. Share cooking notes, ask questions, and talk about recipes with other cooks." }
                        }
                        LandingScreenshot {
                            source: asset!("/assets/landing/features/discussions.webp"),
                            title: "Community discussions",
                            caption: "Concept preview showing a question and reply about proofing croissants.",
                            size: (960, 600),
                        }
                    }
                }
                LandingChapter { story: RECIPE_DRAFT_STORY }
                article { class: "landing-chapter",
                    div { class: "landing-chapter-copy",
                        h2 { "Make cooking your favorites convenient" }
                        p { "Check off ingredients as you prep and follow one step at a time in cooking mode on your phone." }
                        HomeScene { scene: Scene::SoupIsland }
                    }
                    div { class: "landing-cooking-previews",
                        LandingScreenshot {
                            source: asset!("/assets/landing/features/ingredient-checklist.webp"),
                            title: "Ingredient checklist",
                            caption: "Check off ingredients as you prepare chili garlic oil noodles.",
                            size: (440, 732),
                        }
                        div { class: "landing-cooking-phone",
                            CookingPreview {}
                        }
                    }
                }
                LandingChapter { story: VERSION_HISTORY_STORY }
                section { class: "landing-chapter landing-anywhere", aria_labelledby: "landing-anywhere-title",
                    div { class: "landing-chapter-copy",
                        h2 { id: "landing-anywhere-title", "Use anywhere" }
                        p { "Take your recipe book along. Look up ingredients and follow the steps on your phone, even at a picnic." }
                        HomeScene { scene: Scene::Picnic }
                    }
                    div { class: "landing-mobile-previews",
                        LandingScreenshot {
                            source: asset!("/assets/landing/mobile-browse.webp"), title: "Browse recipes on mobile",
                            caption: "The mobile Explore page with recipe search, tags, and community recipes.",
                            size: (390, 780), phone: true,
                        }
                        LandingScreenshot {
                            source: asset!("/assets/landing/mobile-edit.webp"), title: "Edit recipes on mobile",
                            caption: "Editing the name and description of a caramelized onion and mushroom burger draft.",
                            size: (390, 780), phone: true,
                        }
                        CookingPreview { hint: "Select a screen to enlarge it." }
                    }
                }
                section { class: "landing-chapter landing-feature-section landing-pantry", aria_labelledby: "landing-pantry-title",
                    div { class: "landing-chapter-copy",
                        h2 { id: "landing-pantry-title", "Kitchen Pantry" }
                        p { "Check what you have before you cook, then turn the ingredients you’re missing into a shopping list." }
                        FeatureDescriptions { features: PANTRY_FEATURES }
                        HomeScene { scene: Scene::Pantry }
                    }
                    FeaturePreviews { features: PANTRY_FEATURES }
                }
                section { class: "landing-chapter landing-feature-section landing-recipe-tools", id: "landing-features", aria_labelledby: "landing-features-title",
                    div { class: "landing-chapter-copy",
                        h2 { id: "landing-features-title", "Recipe tools" }
                        FeatureDescriptions { features: RECIPE_TOOLS }
                    }
                    FeaturePreviews { features: RECIPE_TOOLS }
                }
            }
            section { class: "landing-trust landing-wrap", aria_labelledby: "landing-trust-title",
                div { class: "landing-section-head",
                    p { class: "landing-eyebrow", "Our approach" }
                    h2 { id: "landing-trust-title", "Prioritizing a great user experience." }
                    p { "Noms aims to provide a simple and frictionless experience that focuses on being the ultimate platform for cooking great food and sharing your favorite recipes." }
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
                p { class: "landing-tiny",
                    "Photo credits · "
                    a { href: "https://commons.wikimedia.org/wiki/File:Yangnyeom-chikin_1.jpg", "Korean fried chicken by anokarina" }
                    " ("
                    a { href: "https://creativecommons.org/licenses/by-sa/2.0/", "CC BY-SA 2.0" }
                    ", cropped) · "
                    a { href: "https://commons.wikimedia.org/wiki/File:You_Po_Mian%EF%BC%88_Oil_poured_noodles%EF%BC%89.webp", "Chili oil noodles by syctown" }
                    " ("
                    a { href: "https://creativecommons.org/licenses/by-sa/4.0/", "CC BY-SA 4.0" }
                    ", cropped)"
                }
            }
        }
    }
}

#[component]
fn LandingActions(
    signed_in: bool,
    #[props(default)] create: bool,
    #[props(default)] search: bool,
) -> Element {
    let cta = home_cta(signed_in);
    let mut search_text = use_signal(String::new);
    let mut submit = move |text: String| {
        search_text.set(text.clone());
        navigator().push(crate::pages::search::public_recipe_search_url(&text));
    };
    rsx! {
        if search {
            form { class: "landing-recipe-search", role: "search", aria_label: "Find a recipe", action: "/search", method: "get",
                "data-autocomplete-anchor": "true",
                onsubmit: move |event| { event.prevent_default(); submit(search_text()); },
                label { class: "sr-only", r#for: "landing-recipe-query", "Search public recipes" }
                svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "currentColor", stroke_width: "1.8", "aria-hidden": "true",
                    circle { cx: "10.5", cy: "10.5", r: "6.5" }
                    path { d: "m16 16 4 4" }
                }
                crate::components::recipe_title_autocomplete::RecipeTitleAutocomplete {
                    id: "landing-recipe-query", accessible_name: "Search public recipes", public_only: true,
                    placeholder: "Find your next recipe…", input_class: "", value: search_text(),
                    oninput: move |text| search_text.set(text), onsubmit: submit,
                }
                button { class: "btn btn-primary", r#type: "submit", "Search" }
            }
        }
        div { class: "landing-actions",
            if cta.destination == HomeCtaDestination::Dashboard {
                if create { Link { to: Route::RecipeNew {}, class: "btn btn-primary touch-target", "Create a recipe →" } }
                else { Link { to: Route::Dashboard {}, class: "btn btn-primary touch-target", "{cta.label} →" } }
            } else {
                a { href: "/login?redirect_uri=/recipes/new", class: "btn btn-primary touch-target", "{cta.label} →" }
            }
            if !search {
                Link { to: Route::Explore {}, class: "btn btn-secondary touch-target", "Explore recipes" }
            }
        }
        p { class: "landing-tiny", "Free to use · Browse public recipes without an account" }
    }
}

#[component]
fn CookingPreview(hint: Option<&'static str>) -> Element {
    let mut paused = use_signal(|| false);
    rsx! {
        LandingScreenshot {
            source: COOK_ONE, title: "Cook on mobile", caption: "Miso mushroom ramen in cooking mode.",
            size: (390, 780), phone: true, animate: true, paused: paused(),
        }
        div { class: "landing-preview-controls",
            if let Some(hint) = hint { span { "{hint}" } }
            button { class: "btn btn-secondary", r#type: "button", aria_pressed: paused(), onclick: move |_| paused.toggle(),
                if paused() { "Play cooking preview" } else { "Pause cooking preview" }
            }
        }
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
