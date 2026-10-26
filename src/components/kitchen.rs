//! The shared home for pantry inventory and independent grocery lists.
use crate::components::AuthRequired;
use crate::Route;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum KitchenView {
    Pantry,
    Groceries,
}

impl KitchenView {
    pub(crate) fn for_route(route: &Route) -> Option<Self> {
        match route {
            Route::Pantry {} | Route::PantryNew {} | Route::PantryDetail { .. } => {
                Some(Self::Pantry)
            }
            Route::GroceryLists {} | Route::GroceryListNew {} | Route::GroceryListDetail { .. } => {
                Some(Self::Groceries)
            }
            _ => None,
        }
    }
}

/// Dioxus Link computes aria-current from exact URL equality. Section links
/// need explicit selection on descendant routes and legacy URL aliases too.
#[component]
pub(crate) fn SectionLink(
    to: Route,
    class: String,
    current: bool,
    children: Element,
    #[props(default)] on_navigate: EventHandler<()>,
) -> Element {
    let href = router().prefix().unwrap_or_default() + &to.to_string();
    rsx! {
        a {
            href,
            class,
            aria_current: current.then_some("page"),
            onclick: move |event: MouseEvent| {
                // Preserve open-in-new-tab/window, middle click, and native hrefs.
                if !event.modifiers().is_empty()
                    || event.trigger_button() != Some(dioxus::html::input_data::MouseButton::Primary)
                {
                    return;
                }
                event.prevent_default();
                navigator().push(to.clone());
                on_navigate.call(());
            },
            {children}
        }
    }
}

#[component]
pub fn KitchenLayout() -> Element {
    let route = use_route::<Route>();
    let active = KitchenView::for_route(&route);
    // The enclosing session scope resets these destinations on account changes.
    let mut pantry = use_signal(|| Route::Pantry {});
    let mut groceries = use_signal(|| Route::GroceryLists {});
    use_effect(use_reactive(&route, move |route| match route {
        Route::Pantry {} | Route::PantryDetail { .. } => pantry.set(route),
        Route::GroceryLists {} | Route::GroceryListDetail { .. } => groceries.set(route),
        _ => (),
    }));
    rsx! {
        AuthRequired {
            div { class: "kitchen-workspace",
                header { class: "container kitchen-header",
                    p { class: "kitchen-title", "Kitchen" }
                    nav { class: "kitchen-views", aria_label: "Kitchen views",
                        for (view, label, destination) in [
                            (KitchenView::Pantry, "Pantry", pantry()),
                            (KitchenView::Groceries, "Grocery lists", groceries()),
                        ] {
                            SectionLink {
                                to: destination,
                                class: "kitchen-view",
                                current: active == Some(view),
                                "{label}"
                            }
                        }
                    }
                    p { class: "kitchen-description",
                        match active {
                            Some(KitchenView::Pantry) => "Track what ingredients you have and determine what recipes are available.",
                            Some(KitchenView::Groceries) => "Create a grocery list based on selected recipes.",
                            None => "",
                        }
                    }
                }
                Outlet::<Route> {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, Routable)]
    enum ProbeRoute {
        #[route("/probe")]
        SectionProbe {},
    }

    #[component]
    fn SectionProbe() -> Element {
        rsx! {
            SectionLink { to: Route::Pantry {}, class: "selected-section", current: true, "Pantry" }
            SectionLink { to: Route::GroceryLists {}, class: "other-section", current: false, "Grocery lists" }
        }
    }

    #[component]
    fn SectionProbeRoot() -> Element {
        rsx! {
            dioxus::router::components::HistoryProvider {
                history: |_| std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/probe")) as std::rc::Rc<dyn History>,
                Router::<ProbeRoute> {}
            }
        }
    }

    #[test]
    fn section_links_keep_explicit_selection_when_the_url_is_not_an_exact_match() {
        let mut dom = VirtualDom::new(SectionProbeRoot);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        let links: Vec<_> = html.split("<a ").skip(1).collect();
        assert_eq!(links.len(), 2);
        assert!(links[0].contains("href=\"/kitchen/pantry\""));
        assert!(links[0].contains("aria-current=\"page\""));
        assert!(!links[1].contains("aria-current"));
    }

    #[test]
    fn old_bookmarks_and_kitchen_entry_resolve_to_canonical_views() {
        for (old, new, view) in [
            ("/kitchen", "/kitchen/pantry", KitchenView::Pantry),
            ("/pantry", "/kitchen/pantry", KitchenView::Pantry),
            ("/pantry/new", "/kitchen/pantry/new", KitchenView::Pantry),
            (
                "/pantry/example",
                "/kitchen/pantry/example",
                KitchenView::Pantry,
            ),
            (
                "/grocery-lists",
                "/kitchen/grocery-lists",
                KitchenView::Groceries,
            ),
            (
                "/grocery-lists/new",
                "/kitchen/grocery-lists/new",
                KitchenView::Groceries,
            ),
            (
                "/grocery-lists/example",
                "/kitchen/grocery-lists/example",
                KitchenView::Groceries,
            ),
        ] {
            let route: Route = old.parse().unwrap();
            assert_eq!(route.to_string(), new, "{old}");
            assert_eq!(route, new.parse().unwrap());
            assert_eq!(KitchenView::for_route(&route), Some(view));
        }
        assert_eq!(KitchenView::for_route(&Route::Dashboard {}), None);
    }
}
