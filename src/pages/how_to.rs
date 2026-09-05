use dioxus::prelude::*;

use crate::components::informational::{CallToActionPanel, PageMeta};
use crate::Route;

pub(super) struct HowToSection {
    pub id: &'static str,
    pub title: &'static str,
}

pub(super) const HOW_TO_SECTIONS: &[HowToSection] = &[
    HowToSection {
        id: "sign-in",
        title: "Sign in",
    },
    HowToSection {
        id: "create",
        title: "Create a recipe",
    },
    HowToSection {
        id: "images",
        title: "Work with images",
    },
    HowToSection {
        id: "visibility",
        title: "Choose visibility",
    },
    HowToSection {
        id: "edit",
        title: "Edit a recipe",
    },
    HowToSection {
        id: "versions",
        title: "Compare and restore versions",
    },
    HowToSection {
        id: "scale",
        title: "Scale a recipe",
    },
    HowToSection {
        id: "collections",
        title: "Organize with collections",
    },
    HowToSection {
        id: "explore",
        title: "Explore public recipes",
    },
    HowToSection {
        id: "fork",
        title: "Fork a recipe",
    },
];

#[component]
pub fn HowTo() -> Element {
    rsx! {
        PageMeta {
            title: "How to use Noms",
            description: "Learn how to create, organize, revise, scale, discover, and share recipes with Noms.",
            path: "/how-to",
        }

        div { class: "marketing-page how-to-page",
            header { class: "how-to-hero container",
                p { class: "marketing-eyebrow", "A practical guide" }
                h1 { "From first draft to trusted recipe." }
                p { class: "marketing-lede", "Create your first recipe, keep work private until it is ready, organize your library, and share only what you choose." }
            }

            div { class: "how-to-layout container",
                aside { class: "how-to-contents",
                    nav { aria_label: "How-to topics",
                        p { class: "how-to-contents__title", "On this page" }
                        ol {
                            for section in HOW_TO_SECTIONS {
                                li { a { href: "#{section.id}", "{section.title}" } }
                            }
                        }
                    }
                }

                main { class: "how-to-guide",
                    GuideSection { id: "sign-in", eyebrow: "Getting started", title: "Sign in to open your recipe library",
                        ol {
                            li { "Select " strong { "Sign In" } " from the navigation." }
                            li { "Continue with Google or GitHub." }
                            li { "After authorization, Noms returns you to the page you requested or opens your dashboard." }
                        }
                        ExpectedResult { "Your dashboard shows published recipes, private drafts, and collection navigation." }
                    }

                    GuideSection { id: "create", eyebrow: "Create", title: "Capture the recipe while the details are fresh",
                        ol {
                            li { "Open " strong { "New Recipe" } "." }
                            li { "Add a title, description, commentary, timing, and servings." }
                            li { "Add ingredients with amounts, units, names, and optional notes." }
                            li { "Add instructions in cooking order, then equipment and tags where useful." }
                            li { "Save a private draft or continue to publication." }
                        }
                        ExpectedResult { "Drafts appear in Dashboard under Drafts; published recipes appear in My Recipes." }
                    }

                    GuideSection { id: "images", eyebrow: "Images", title: "Add photos without leaving the editor",
                        p { "Add recipe images from the editor. Images are processed and uploaded when you save. Individual steps can also carry photos, and image controls let you adjust their presentation before publication." }
                        GuideCallout { tone: "note", title: "Before saving", "Wait for active image processing to finish. Retry or remove an image if its upload fails." }
                    }

                    GuideSection { id: "visibility", eyebrow: "Publish", title: "Choose who can see the recipe",
                        div { class: "visibility-grid",
                            VisibilityCard { label: "Private", body: "Only you can view it." }
                            VisibilityCard { label: "Unlisted", body: "Anyone with the link can view it." }
                            VisibilityCard { label: "Public", body: "It can appear in Explore and on your public profile." }
                        }
                        p { "Choose the intended visibility, then select " strong { "Publish Recipe" } ". Draft edits remain private until you select " strong { "Publish Changes" } "." }
                    }

                    GuideSection { id: "edit", eyebrow: "Revise", title: "Improve a published recipe safely",
                        ol {
                            li { "Open a recipe you own and select " strong { "Edit" } "." }
                            li { "Make your changes. Work is autosaved privately." }
                            li { "Select " strong { "Publish Changes" } " when the update is ready." }
                            li { "Use " strong { "Discard Draft" } " to remove the unpublished revision." }
                        }
                        ExpectedResult { "Publishing creates a new entry in the recipe’s version history." }
                    }

                    GuideSection { id: "versions", eyebrow: "History", title: "Follow the recipe’s published history",
                        p { "On a recipe you own, Version history lets you:" }
                        ul {
                            li { "view differences introduced by a published version;" }
                            li { "open a read-only snapshot;" }
                            li { "compare selected versions;" }
                            li { "restore an earlier version as the new current version; or" }
                            li { "open the version graph for a wider view of its lineage." }
                        }
                        GuideCallout { tone: "success", title: "History stays intact", "Restoring does not delete versions published after the selected snapshot." }
                    }

                    GuideSection { id: "scale", eyebrow: "At the stove", title: "Adjust quantities for this cooking session",
                        p { "Open the recipe scaler from a recipe page. Choose a multiplier, target serving count, or target ingredient amount. Noms recalculates ingredient quantities and applicable timing for the current view without changing the saved recipe." }
                    }

                    GuideSection { id: "collections", eyebrow: "Organize", title: "Give related recipes a shared place",
                        ol {
                            li { "Open " strong { "Collections" } " and select " strong { "New Collection" } "." }
                            li { "Add a name, optional description, visibility, and optional cover image." }
                            li { "Create subcollections when another level helps." }
                            li { "From a recipe page, choose " strong { "Add to Collection" } " and select one or more destinations." }
                            li { "Reorder recipes or child collections from the collection page." }
                        }
                        GuideCallout { tone: "warning", title: "Deleting a collection", "Deleting removes its nested collections and memberships, but it does not delete the recipes themselves." }
                    }

                    GuideSection { id: "explore", eyebrow: "Discover", title: "Browse what other cooks have shared",
                        p { "Open Explore to browse public recipes. Use its text field to filter recipes currently loaded by title or description." }
                        p { "Creator links open public profiles containing the recipes and collections that person has chosen to share." }
                    }

                    GuideSection { id: "fork", eyebrow: "Build on an idea", title: "Start from a recipe without overwriting it",
                        p { "Select " strong { "Fork" } " on a recipe page. If you are signed out, Noms asks you to sign in first. The fork opens as your private draft, ready to edit and publish independently." }
                        ExpectedResult { "The new recipe keeps attribution and lineage back to its source." }
                    }

                    section { class: "how-to-troubleshooting", aria_labelledby: "troubleshooting-title",
                        p { class: "marketing-eyebrow", "Troubleshooting" }
                        h2 { id: "troubleshooting-title", "When something doesn’t look right" }
                        details { summary { "My images are still processing" } p { "Wait for processing to finish. If an image fails, retry it or remove it before publishing." } }
                        details { summary { "I cannot find an unpublished edit" } p { "Open Dashboard and look in Drafts. Edits remain private until you publish them." } }
                        details { summary { "Someone cannot open my recipe or collection" } p { "Check its visibility. Private items are owner-only; use Unlisted for link access or Public for discovery." } }
                        details { summary { "Explore did not inspect my whole library" } p { "Explore filters public recipes already loaded on the page. Full recipe, collection, and creator search is not part of the current experience." } }
                    }
                }
            }

            CallToActionPanel {
                title: "Ready to keep your first recipe?",
                body: "Start with the version you cook today. There will be room for what you learn next time.",
                primary_label: "Create a recipe",
                primary_route: Route::RecipeNew {},
                secondary_label: "Explore recipes",
                secondary_route: Route::Explore {},
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct GuideSectionProps {
    id: &'static str,
    eyebrow: &'static str,
    title: &'static str,
    children: Element,
}

#[component]
fn GuideSection(props: GuideSectionProps) -> Element {
    rsx! {
        section { id: props.id, class: "how-to-section",
            p { class: "marketing-eyebrow", "{props.eyebrow}" }
            h2 { "{props.title}" }
            {props.children}
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct VisibilityCardProps {
    label: &'static str,
    body: &'static str,
}

#[component]
fn VisibilityCard(props: VisibilityCardProps) -> Element {
    rsx! {
        div { class: "visibility-card",
            strong { "{props.label}" }
            p { "{props.body}" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct GuideCalloutProps {
    tone: &'static str,
    title: &'static str,
    children: Element,
}

#[component]
fn GuideCallout(props: GuideCalloutProps) -> Element {
    rsx! {
        aside { class: "guide-callout guide-callout--{props.tone}",
            strong { "{props.title}" }
            p { {props.children} }
        }
    }
}

#[component]
fn ExpectedResult(children: Element) -> Element {
    rsx! {
        GuideCallout { tone: "success", title: "Expected result", {children} }
    }
}
