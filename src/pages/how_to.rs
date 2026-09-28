use dioxus::prelude::*;

use crate::components::base::{PageIcon, PageIconKind};
use crate::components::informational::{CallToActionPanel, PageMeta};
use crate::Route;

const HOW_TO_CSS: Asset = asset!("/assets/how-to.css");

pub(super) struct HowToSection {
    pub id: &'static str,
    pub title: &'static str,
}

// Keep existing deep links while bringing the three core workflows together.
pub(super) const HOW_TO_SECTIONS: &[HowToSection] = &[
    HowToSection {
        id: "sign-in",
        title: "Getting started",
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
        title: "Version history",
    },
    HowToSection {
        id: "scale",
        title: "Scale a recipe",
    },
    HowToSection {
        id: "collections",
        title: "Collections",
    },
    HowToSection {
        id: "explore",
        title: "Explore and search",
    },
    HowToSection {
        id: "fork",
        title: "Fork a recipe",
    },
];

#[component]
pub fn HowTo() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: HOW_TO_CSS }
        PageMeta {
            title: "How to use Noms",
            description: "Learn how to create, organize, revise, scale, discover, and share recipes with Noms.",
            path: "/how-to",
        }
        div { class: "marketing-page how-to-page",
            HowToGuide {}
            CallToActionPanel {
                title: "Ready to keep your first recipe?",
                body: "Add a title and a few ingredients. You can fill in the details as you go.",
                primary_label: "Create a recipe",
                primary_route: Route::RecipeNew {},
                secondary_label: "Explore recipes",
                secondary_route: Route::Explore {},
            }
        }
    }
}

#[component]
fn HowToGuide() -> Element {
    rsx! {
        header { class: "how-to-hero container",
            div {
                h1 { class: "brand-title",
                    PageIcon { kind: PageIconKind::HowTo }
                    span { "Let’s get cooking." }
                }
                p { class: "marketing-lede", "A friendly guide to your recipe book." }
            }
            GuideIllustration {}
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
                    a { class: "how-to-contents__faq", href: "#questions", "Common questions", span { aria_hidden: "true", " ↗" } }
                }
            }
            div { class: "how-to-guide",
                div { class: "how-to-overview",
                    GuideSection { id: "create", number: "1", title: "Create your first recipe",
                        p { "Open New Recipe. Add a title, ingredients and instructions. You can fill in the details as you go." }
                        figure { class: "how-to-example", aria_label: "Example recipe draft",
                            figcaption { class: "sr-only", "Example recipe draft" }
                            span { class: "how-to-example__label", "Title" }
                            div { class: "how-to-example__well", "My favorite pasta" }
                            span { class: "how-to-example__label", "Ingredients" }
                            ul { class: "how-to-example__well",
                                li { "200 g pasta" }
                                li { "2 tbsp olive oil" }
                                li { "1 clove garlic" }
                            }
                            div { class: "how-to-draft-status", span { aria_hidden: "true", "✓" } " Private draft · autosaved" }
                        }
                        p { class: "how-to-card-note", "Private drafts autosave after your first edit. Add timing, servings, photos, equipment and tags whenever you’re ready." }
                    }
                    GuideSection { id: "visibility", number: "2", title: "Choose who can see it",
                        p { "When you publish, choose the audience for your recipe. Unpublished draft edits stay private." }
                        div { class: "how-to-example how-to-visibility",
                            p { class: "how-to-example__label", "Recipe visibility" }
                            dl {
                                div { dt { span { aria_hidden: "true", "◉" } " Private" } dd { "Only you can view it." } }
                                div { dt { span { aria_hidden: "true", "○" } " Unlisted" } dd { "Anyone with the link can view it." } }
                                div { dt { span { aria_hidden: "true", "○" } " Public" } dd { "It can appear in Explore and on your public profile." } }
                            }
                        }
                        GuideCallout { title: "Before you share", "Choose the intended visibility, then select Publish Recipe. A private published recipe is separate from an unpublished draft." }
                    }
                    GuideSection { id: "versions", number: "3", title: "Keep track of changes",
                        p { "Each published update becomes a new version. Open Version history on a recipe you own to revisit it." }
                        figure { class: "how-to-example", aria_label: "Example published version history",
                            figcaption { "Version history · example" }
                            ol { class: "how-to-version-list",
                                li { class: "how-to-version-list__current",
                                    strong { "v2" }
                                    div { span { class: "how-to-version-status", "Current" } p { "Added notes and timing." } }
                                }
                                li { strong { "v1" } div { span { class: "how-to-version-status", "Published" } p { "The first recipe." } } }
                            }
                        }
                        GuideCallout { title: "History stays intact", "Restoring creates a new current version. Later versions remain in the history." }
                    }
                }
                section { class: "how-to-troubleshooting", id: "questions", tabindex: "-1", aria_labelledby: "troubleshooting-title",
                    h2 { id: "troubleshooting-title", "A few common questions" }
                    details { summary { "Can I edit a recipe after publishing?" } p { "Yes. Open a recipe you own and select Edit. Changes autosave to a private draft; select Publish Changes when you’re ready. Use Discard Draft to remove the unpublished revision." } }
                    details { summary { "Why can’t someone open my recipe or collection?" } p { "Check its visibility. Private items are owner-only. Use Unlisted for link access or Public for discovery. Sharing a collection does not make its private recipes public." } }
                    details { summary { "Where is my unfinished recipe?" } p { "Open Dashboard and look in Drafts. Drafts and unpublished edits remain private until you publish them." } }
                    details { summary { "How do Explore and Search differ?" } p { "Explore is public browsing, with title and description search and Load more. Sign in to Search for recipes, collections and cooks, with result scopes and filters. Results respect content visibility." } }
                    details { summary { "What if an image is still processing?" } p { "Wait for processing to finish. If an image fails, retry it or remove it before publishing." } }
                }
                div { class: "how-to-more-heading",
                    p { class: "marketing-eyebrow", "Make yourself at home" }
                    h2 { "A little more know-how" }
                    p { "Photos, collections and the details that make a recipe yours." }
                }
                div { class: "how-to-details-grid",
                    GuideSection { id: "sign-in", title: "First, open your recipe book",
                        p { a { href: "/login", "Sign in" } " with Google or GitHub to create recipes and organize your library. After authorization, you return to your requested page or your dashboard." }
                        p { "Just browsing? " a { href: "/explore", "Explore public recipes" } " without an account." }
                    }
                    GuideSection { id: "images", title: "Add photos along the way",
                        p { "Choose or drag recipe photos into the editor. Individual steps can also carry photos. Open the image editor to crop, rotate or adjust an image before saving." }
                        GuideCallout { title: "Before saving", "Images are processed and uploaded when you save. Wait for processing to finish, and retry or remove an image if its upload fails." }
                    }
                    GuideSection { id: "edit", title: "Improve a published recipe",
                        ol {
                            li { "Open a recipe you own and select " strong { "Edit" } "." }
                            li { "Make your changes. Work is autosaved privately." }
                            li { "Select " strong { "Publish Changes" } " when the update is ready, or " strong { "Discard Draft" } " to remove the unpublished revision." }
                        }
                        p { "Publishing adds a new entry to the recipe’s version history. Open a read-only snapshot, compare versions, or follow forks in the version graph." }
                    }
                    GuideSection { id: "scale", title: "Cook a little more. Or less.",
                        p { "Open the recipe scaler on a recipe page. Choose a multiplier, target serving count or target ingredient amount." }
                        p { "Ingredient quantities and applicable timing change for the current view. The saved recipe stays the same." }
                    }
                    GuideSection { id: "collections", title: "Give your recipes a shared place",
                        ol {
                            li { "Open " strong { "Collections" } " and select " strong { "New Collection" } "." }
                            li { "Add a name, description, visibility and optional cover image. Create subcollections if another level helps." }
                            li { "On a recipe, choose " strong { "Add to Collection" } " and select one or more destinations." }
                            li { "Reorder recipes or child collections from the collection page." }
                        }
                        GuideCallout { title: "Deleting a collection", "Deleting removes its nested collections and memberships, but does not delete the recipes themselves." }
                    }
                    GuideSection { id: "explore", title: "Find your next recipe",
                        p { a { href: "/explore", "Explore" } " searches public recipe titles and descriptions, including recipes beyond those already loaded. Select Load more to see additional results. Community tags are labels, not filter controls." }
                        p { a { href: "/search", "Search" } " requires sign-in and lets you find recipes, collections and cooks. Choose a result scope to see its filters and sorting options. Only content you can access appears." }
                        p { "Creator links open public profiles with the recipes and collections that person has chosen to share." }
                    }
                    GuideSection { id: "fork", title: "Start with someone else’s idea",
                        p { "Select " strong { "Fork" } " on a recipe page. Sign in if prompted. Your fork opens as a private draft, ready to edit and publish independently." }
                        GuideCallout { title: "Keep the connection", "The original recipe stays unchanged. Your new recipe keeps attribution and lineage back to its source." }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct GuideSectionProps {
    id: &'static str,
    title: &'static str,
    #[props(default)]
    number: Option<&'static str>,
    children: Element,
}

#[component]
fn GuideSection(props: GuideSectionProps) -> Element {
    rsx! {
        section { id: props.id, class: "how-to-section", tabindex: "-1", aria_labelledby: "how-to-{props.id}-title",
            div { class: "how-to-section__heading",
                if let Some(number) = props.number {
                    span { class: "how-to-step-number", aria_hidden: "true", "{number}" }
                }
                h2 { id: "how-to-{props.id}-title", "{props.title}" }
            }
            {props.children}
        }
    }
}

#[component]
fn GuideCallout(title: &'static str, children: Element) -> Element {
    rsx! {
        aside { class: "how-to-note",
            strong { "{title}" }
            p { {children} }
        }
    }
}

/// A recipe-card vignette, distinct from the production Noms mark.
#[component]
fn GuideIllustration() -> Element {
    rsx! {
        svg { class: "how-to-illustration", view_box: "0 0 300 200", fill: "none", "aria-hidden": "true", "focusable": "false",
            ellipse { cx: "146", cy: "174", rx: "114", ry: "15", class: "how-to-art-shadow" }
            g { transform: "rotate(-8 124 98)",
                rect { x: "42", y: "24", width: "146", height: "147", rx: "16", class: "how-to-art-back" }
                rect { x: "48", y: "32", width: "134", height: "139", rx: "12", class: "how-to-art-paper" }
                path { d: "M154 24h14v48l-7-5-7 5z", class: "how-to-art-coral" }
                path { d: "M70 106h83M70 122h68M70 138h76", class: "how-to-art-lines", stroke_width: "4", stroke_linecap: "round" }
                path { d: "M78 66c0-13 11-16 17-7 6-9 17-6 17 7 0 8-17 20-17 20S78 74 78 66Z", class: "how-to-art-coral" }
            }
            g { transform: "rotate(24 226 102)",
                rect { x: "219", y: "76", width: "14", height: "103", rx: "7", class: "how-to-art-wood" }
                ellipse { cx: "226", cy: "58", rx: "23", ry: "35", class: "how-to-art-wood" }
                ellipse { cx: "226", cy: "54", rx: "14", ry: "23", class: "how-to-art-spoon-well" }
            }
            path { d: "M242 176q4-27 27-46", class: "how-to-art-stem", stroke_width: "3", stroke_linecap: "round" }
            path { d: "M248 162q-18-23-13-29 20 3 13 29ZM256 151q-1-28 11-30 8 19-11 30ZM248 164q29-17 34-5-9 14-34 5Z", class: "how-to-art-leaves" }
            path { d: "m28 57-10-5m9 25-13 2m177-62 4-9", class: "how-to-art-spark", stroke_width: "4", stroke_linecap: "round" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered_guide() -> String {
        let mut dom = VirtualDom::new(HowToGuide);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    #[test]
    fn guide_heading_uses_the_approved_decorative_identity() {
        let html = rendered_guide();
        assert!(html.contains("how-to-v1"), "{html}");
        assert!(html.contains("brand-title"), "{html}");
        assert!(html.contains("alt=\"\""), "{html}");
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(html.contains("Let’s get cooking."), "{html}");
    }

    #[test]
    fn every_contents_link_has_one_focusable_destination() {
        let html = rendered_guide();
        for id in HOW_TO_SECTIONS
            .iter()
            .map(|section| section.id)
            .chain(["questions"])
        {
            assert!(
                html.contains(&format!("href=\"#{id}\"")),
                "missing link: {id}"
            );
            assert_eq!(
                html.matches(&format!("id=\"{id}\"")).count(),
                1,
                "duplicate/missing destination: {id}"
            );
        }
        assert_eq!(
            html.matches("tabindex=\"-1\"").count(),
            HOW_TO_SECTIONS.len() + 1
        );
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(
            !html.contains("<main"),
            "AppLayout already supplies the main landmark"
        );
    }

    #[test]
    fn tutorial_examples_are_readable_without_fake_form_controls() {
        let html = rendered_guide();
        assert!(html.contains("Example recipe draft"));
        assert!(html.contains("Example published version history"));
        assert!(html.contains("<dl"));
        assert!(!html.contains("<input"));
        assert!(!html.contains("<button"));
        assert_eq!(html.matches("<details").count(), 5);
        assert_eq!(html.matches("<summary").count(), 5);
    }
}
