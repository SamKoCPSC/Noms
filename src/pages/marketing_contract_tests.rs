use super::how_to::HOW_TO_SECTIONS;
use dioxus::prelude::*;

#[component]
fn LandingTestRoot(signed_in: bool) -> Element {
    let auth = use_signal(move || crate::auth::context::AuthContext {
        is_authenticated: signed_in,
        ..Default::default()
    });
    use_context_provider(|| auth);
    rsx! {
        dioxus::router::components::HistoryProvider {
            history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/")) as std::rc::Rc<dyn History>,
            Router::<crate::Route> {}
        }
    }
}

#[test]
fn about_renders_twelve_cards_and_readable_fallback() {
    let html = dioxus_ssr::render_element(rsx! { super::about::About {} });
    for title in [
        "The Ultimate Recipe Manager",
        "A side project that grew in to something larger.",
        "Hi, I&#39;m Sam!",
        "Just a simple and easy-to-use app.",
        "Building a community.",
    ] {
        assert!(html.contains(title), "Missing {title}");
    }
    assert_eq!(html.matches("<article").count(), 12);
    assert!(html.contains("class=\"kitchen-fallback\""));
    assert!(html.contains("loading=\"lazy\""));
    let opening = html.split("id=\"panel-1\"").next().unwrap();
    assert!(opening.contains("kitchen-fallback"));
    assert!(html.contains("data-kitchen-surface-color-url"));
    assert!(html.contains("data-kitchen-surface-normal-url"));
    assert!(!html.contains("data-kitchen-lighting-url"));
    assert!(!html.contains("data-kitchen-arrival-lighting-url"));
    let reason = html
        .split("id=\"panel-1\"")
        .nth(1)
        .unwrap()
        .split("id=\"panel-2\"")
        .next()
        .unwrap();
    assert!(reason.contains("office-fallback"));
    assert!(reason.contains("A UBC-inspired university library"));
    for (panel, description, attribute, fallback) in [
        (
            1,
            "A UBC-inspired university library",
            "data-library-url",
            "office-fallback",
        ),
        (
            3,
            "A modern supermarket",
            "data-supermarket-url",
            "office-fallback",
        ),
        (4, "Sam’s office", "data-office-url", "office-fallback"),
        (
            5,
            "A corporate office",
            "data-corporate-office-url",
            "office-fallback",
        ),
        (
            6,
            "A U-shaped brick-loft kitchen",
            "data-brick-kitchen-url",
            "kitchen-fallback",
        ),
        (
            7,
            "A modern artisan bakery",
            "data-bakery-url",
            "kitchen-fallback",
        ),
        (
            8,
            "A corner farmers’ market",
            "data-farmers-market-url",
            "garden-fallback",
        ),
        (
            9,
            "A contemporary L-shaped kitchen",
            "data-social-kitchen-url",
            "kitchen-fallback",
        ),
        (
            10,
            "A secluded riverside campsite",
            "data-riverside-campsite-url",
            "garden-fallback",
        ),
    ] {
        let card = html
            .split(&format!("id=\"panel-{panel}\""))
            .nth(1)
            .unwrap()
            .split(&format!("id=\"panel-{}\"", panel + 1))
            .next()
            .unwrap();
        assert!(card.contains(description), "Panel {panel}: {description}");
        assert!(card.contains(fallback), "Panel {panel}: {fallback}");
        assert!(
            card.contains("loading=\"lazy\""),
            "Panel {panel}: lazy poster"
        );
        assert!(html.contains(attribute), "Missing {attribute}");
    }
    for attribute in [
        "data-office-url",
        "data-office-left-url",
        "data-office-right-url",
        "data-office-gpu-url",
        "data-office-cpu-url",
    ] {
        assert!(html.contains(attribute));
    }
    let creator = html
        .split("id=\"panel-2\"")
        .nth(1)
        .unwrap()
        .split("id=\"panel-3\"")
        .next()
        .unwrap();
    assert!(!creator.contains("kitchen-fallback"));
    assert!(!html.contains("potluck-fallback"));
    for attribute in [
        "data-potluck-url",
        "data-potluck-window-url",
        "data-potluck-tv-url",
        "data-potluck-pastry-url",
        "data-potluck-salt-url",
        "data-potluck-bbq-url",
    ] {
        assert!(
            !html.contains(attribute),
            "Retired potluck asset: {attribute}"
        );
    }
    let garden = html.split("id=\"panel-11\"").nth(1).unwrap();
    assert!(garden.contains("garden-fallback"));
    assert!(garden.contains("lawn chairs"));
    assert!(garden.contains("Take the tour again"));
    assert!(html.contains("data-garden-url"));
    assert!(html.contains("data-garden-pastry-url"));
    assert!(html.contains("id=\"tour-chapter-status\""));
    assert!(html.contains("Nowadays I continue to support the project"));
    assert!(!html.contains("How did Noms start"));
}

#[test]
fn about_provides_motion_and_reading_controls_before_enhancement() {
    let html = dioxus_ssr::render_element(rsx! { super::about::About {} });
    assert!(html.contains("id=\"mode-toggle\""));
    assert!(html.contains("id=\"motion-toggle\""));
    assert!(!html.contains("id=\"tour-navigation\""));
    assert_eq!(html.matches("class=\"next-link\"").count(), 12);
    for id in ["idea", "why", "creator", "community", "gather"] {
        assert!(html.contains(&format!("href=\"#{id}\"")));
    }
}

#[test]
fn about_loading_screen_keeps_a_reading_escape_and_accessible_status() {
    let html = dioxus_ssr::render_element(rsx! { super::about::About {} });
    assert!(html.contains("id=\"tour-loader\""));
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("Click here if the page takes too long to load"));
    assert!(html.contains("class=\"about-tour is-reader\""));
    assert!(!html.contains("data-entry-state=\"loading\""));
}

#[test]
fn how_to_uses_stable_anchor_ids_for_every_documented_workflow() {
    let ids = HOW_TO_SECTIONS
        .iter()
        .map(|section| section.id)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "sign-in",
            "create",
            "images",
            "visibility",
            "edit",
            "versions",
            "scale",
            "collections",
            "explore",
            "fork",
        ]
    );
}

#[test]
fn public_page_copy_does_not_claim_unshipped_full_search() {
    let marketing_copy = include_str!("../../docs/design/NOMS-013/page-copy.md");
    let production_copy = [
        include_str!("home.rs"),
        include_str!("about.rs"),
        include_str!("how_to.rs"),
    ]
    .join("\n");

    assert!(marketing_copy.contains("Full recipe, collection, and creator search is not part"));
    assert!(!production_copy.contains("Search your entire library"));
    assert!(!production_copy.contains("Search recipes, collections, and people"));
}

#[test]
fn landing_preserves_public_exploration_and_authenticated_creation() {
    for signed_in in [false, true] {
        let mut dom =
            VirtualDom::new_with_props(LandingTestRoot, LandingTestRootProps { signed_in });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let (welcome, tagline) = html
            .split_once("<h1 id=\"home-title\">")
            .unwrap()
            .1
            .split_once("</span>")
            .unwrap();
        assert!(welcome.contains("Welcome to Noms"));
        assert!(tagline
            .trim_start()
            .starts_with("The Ultimate Recipe Manager</h1>"));
        let hero_actions = html
            .split_once("class=\"landing-recipe-search\"")
            .unwrap()
            .1
            .split_once("</form>")
            .unwrap()
            .1
            .split_once("class=\"landing-tiny\"")
            .unwrap()
            .0;
        assert!(hero_actions.contains("href=\"#why-noms-exists\""));
        assert!(hero_actions.contains(">Learn more</a>"));
        assert!(hero_actions.contains("href=\"/explore\""));
        assert!(hero_actions.contains(">Explore</a>"));
        assert!(html.contains("id=\"why-noms-exists\""));
        let (account_url, account_label) = if signed_in {
            ("href=\"/dashboard\"", "Open dashboard")
        } else {
            ("href=\"/login\"", "Create your account")
        };
        assert!(hero_actions.contains(account_url));
        assert!(hero_actions.contains(account_label));
        assert_eq!(
            hero_actions
                .matches("class=\"btn btn-primary touch-target\"")
                .count(),
            3
        );
        assert!(!hero_actions.contains("Start your recipe book"));
        let stories = html
            .split("class=\"landing-chapters landing-wrap\"")
            .nth(1)
            .unwrap()
            .split("class=\"landing-trust landing-wrap\"")
            .next()
            .unwrap();
        let chapter = |title: &str| {
            stories
                .split("<article class=\"landing-chapter\"")
                .skip(1)
                .filter_map(|part| part.split_once("</article>").map(|(article, _)| article))
                .find(|article| article.contains(title))
                .unwrap_or_else(|| panic!("Missing landing chapter {title}"))
        };
        let discover = chapter("Discover new recipes.");
        assert!(discover.contains("Discover new recipes."));
        assert!(discover.contains("data-scene=\"pandacotta-banquet\""));
        assert!(discover.contains("bao, tacos, and strawberry cream cake"));
        assert!(discover.contains("loading=\"lazy\""));
        let (intro, screenshot) = discover.split_once("class=\"landing-app-window\"").unwrap();
        assert!(intro.contains("landing-pandacotta-banquet"));
        assert!(screenshot.contains("landing-shot-button"));
        assert!(
            screenshot.contains("Croissants, chocolate cake, and Korean fried chicken in Explore.")
        );
        let cooking = chapter("Make cooking your favorites convenient");
        let (intro, screenshot) = cooking.split_once("class=\"landing-app-window\"").unwrap();
        assert!(intro.contains("data-scene=\"owlfredo-kitchen-island\""));
        assert!(intro.contains("Owlfredo stirs, seasons, and tastes tomato soup"));
        assert!(intro.contains("loading=\"lazy\""));
        assert!(
            screenshot.contains("Check off ingredients as you prepare chili garlic oil noodles")
        );
        assert!(screenshot.contains("Pause cooking preview"));
        assert!(!cooking.contains("Scale your servings"));
        let create = chapter("Create your signature recipe.");
        let (intro, screenshot) = create.split_once("class=\"landing-app-window\"").unwrap();
        assert!(intro.contains("data-scene=\"signature-recipe\""));
        assert!(intro.contains("grated truffle"));
        assert!(screenshot.contains("landing-shot-button"));
        let history = chapter("Study how your recipes change.");
        let (intro, screenshot) = history.split_once("class=\"landing-app-window\"").unwrap();
        assert!(intro.contains("data-scene=\"terrimisu-cookie-study\""));
        assert!(intro.contains("Terrimisu tastes cookies from two batches."));
        assert!(intro.contains("loading=\"lazy\""));
        assert!(screenshot.contains("Compare two batches of chocolate chip cookies"));
        let anywhere = stories
            .split_once("id=\"landing-anywhere-title\"")
            .unwrap()
            .1
            .split_once("</section>")
            .unwrap()
            .0;
        assert!(anywhere.contains("Use anywhere"));
        assert!(anywhere.contains("data-scene=\"cattatouille-picnic\""));
        assert!(anywhere.contains("Cattatouille checks a recipe on his phone"));
        assert!(anywhere.contains("loading=\"lazy\""));
        let (_, gallery) = anywhere
            .split_once("class=\"landing-mobile-previews\"")
            .unwrap();
        for title in [
            "Browse recipes on mobile",
            "Edit recipes on mobile",
            "Cook on mobile",
        ] {
            assert!(gallery.contains(&format!("Enlarge screenshot: {title}")));
        }
        assert!(gallery.contains("Pause cooking preview"));
        let share = chapter("Share your best creations.");
        let (intro, screenshot) = share.split_once("class=\"landing-app-window\"").unwrap();
        assert!(intro.contains("data-scene=\"pandacotta-market-stand\""));
        assert!(intro.contains("Pandacotta hands him the recipe card"));
        assert!(intro.contains("loading=\"lazy\""));
        assert!(screenshot.contains("Robert McRobertson’s public recipes and collections."));
        let community = stories
            .split("id=\"landing-community-title\"")
            .nth(1)
            .unwrap()
            .split("</section>")
            .next()
            .unwrap();
        assert!(community.contains("Discover new recipes."));
        assert!(community.contains("Share your best creations."));
        let discussion = community
            .split_once("class=\"landing-chapter landing-discussion\"")
            .unwrap()
            .1;
        let (copy, preview) = discussion
            .split_once("class=\"landing-app-window\"")
            .unwrap();
        assert!(copy.contains("class=\"landing-chapter-copy\""));
        assert!(copy.contains("In progress."));
        assert!(preview.contains("Concept preview"));
        let pantry = stories
            .split("id=\"landing-pantry-title\"")
            .nth(1)
            .unwrap()
            .split("</section>")
            .next()
            .unwrap();
        let (copy, previews) = pantry
            .split_once("class=\"landing-feature-previews\"")
            .unwrap();
        assert!(copy.contains("See what you have"));
        assert!(copy.contains("Build a grocery list"));
        assert_eq!(previews.matches("class=\"landing-shot-button\"").count(), 2);
        let tools = stories
            .split("id=\"landing-features-title\"")
            .nth(1)
            .unwrap()
            .split("</section>")
            .next()
            .unwrap();
        let (copy, previews) = tools
            .split_once("class=\"landing-feature-previews\"")
            .unwrap();
        for feature in [
            "Built-in recipe image editor",
            "Scale your servings",
            "Organize your recipes",
            "Import and export",
        ] {
            assert!(copy.contains(feature));
        }
        assert_eq!(previews.matches("class=\"landing-shot-button\"").count(), 4);
        assert!(previews.contains("The Export recipe dialog showing JSON and HTML formats"));
        assert_eq!(stories.matches(">Scale your servings</h3>").count(), 1);
        assert!(!html.contains("landing-chapter-number"));
        assert!(!html.contains("landing-mobile landing-wrap"));
        assert_eq!(html.matches("class=\"landing-recipe-search\"").count(), 1);
        assert!(html.contains("action=\"/search\""));
        assert!(html.contains("method=\"get\""));
        assert!(html.contains("name=\"q\""));
        assert!(html.contains("href=\"/explore\""));
        assert_eq!(
            html.contains("href=\"/login?redirect_uri=/recipes/new\""),
            !signed_in
        );
        assert_eq!(html.contains("Open dashboard"), signed_in);
        assert_eq!(html.contains("Create a recipe →"), signed_in);
        assert!(html.contains("Free to use."));
        assert!(html.contains("Your data is never shared or sold."));
        assert!(html.contains("loading=\"lazy\""));
        assert!(!html.contains("<iframe"));
        assert!(!html.contains("Maya Chen"));
    }
}
