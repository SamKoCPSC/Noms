use crate::components::base::{PageIcon, PageIconKind};
use dioxus::prelude::*;

use crate::api::collection::list_root_collections;
use crate::components::base::{CollectionAlbum, EmptyState, LoadRecovery, LoadingSpinner};
use crate::components::AuthRequired;
use crate::Route;

fn matches_collection_query(name: &str, description: Option<&str>, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    name.to_lowercase().contains(&query)
        || description
            .unwrap_or_default()
            .to_lowercase()
            .contains(&query)
}

#[component]
pub fn CollectionList() -> Element {
    let mut collections = use_resource(list_root_collections);
    let mut query = use_signal(String::new);
    // Resource::pending() peeks without subscribing; read state so a retry
    // immediately changes its announcement and button state.
    let pending = *collections.state().read() == UseResourceState::Pending;
    rsx! {
        AuthRequired {
            div { class: "container collections-page",
                header { class: "collections-heading",
                    h1 { class: "brand-title", PageIcon { kind: PageIconKind::Collections } span { "Your collections" } }
                    p { "Group your recipes into albums, with subcollections to keep things organized." }
                }
                nav { class: "mobile-library-tabs", aria_label: "Library sections",
                    Link { to: Route::Dashboard {}, "Recipes" }
                    Link { to: Route::CollectionList {}, aria_current: "page", "Collections" }
                    Link { to: Route::Dashboard {}, "Drafts" }
                }
                div { class: "collections-toolbar",
                    label { class: "collections-filter",
                        span { class: "sr-only", "Filter top-level collections" }
                        svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "currentColor", stroke_width: "1.8", "aria-hidden": "true",
                            circle { cx: "10", cy: "10", r: "6" }
                            path { d: "m15 15 5 5" }
                        }
                        input {
                            r#type: "search", class: "input input--shared",
                            placeholder: "Search your collections…", value: query(),
                            oninput: move |event| query.set(event.value()),
                            aria_describedby: "collections-filter-help",
                        }
                    }
                    Link { to: Route::CollectionNew { parent: None }, class: "btn btn-primary touch-target", "＋ New collection" }
                }
                p { id: "collections-filter-help", class: "collections-help", "Showing top-level collections. Open an album to see its subcollections." }
                match collections.read().as_ref() {
                    None => rsx! { div { class: "collections-state", role: "status", LoadingSpinner {} span { "Loading collections…" } } },
                    Some(Err(_)) => rsx! {
                        div { class: "collections-state",
                            LoadRecovery {
                                pending,
                                message: "Your collections couldn’t be loaded. Please try again.",
                                retry_label: "Try again",
                                onretry: move |_| { if !collections.pending() { collections.restart(); } },
                            }
                        }
                    },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        div { class: "collections-state", EmptyState { icon: rsx! { "📁" }, title: "Your first album starts here",
                            description: "Create a collection, then add recipes from your dashboard or a recipe page.",
                            action: rsx! { Link { to: Route::CollectionNew { parent: None }, class: "btn btn-primary touch-target", "Create a collection" } }
                        } }
                    },
                    Some(Ok(items)) => {
                        let filtered = items.iter().filter(|item| matches_collection_query(&item.name, item.description.as_deref(), &query())).collect::<Vec<_>>();
                        rsx! {
                            p { class: "collections-result-count", role: "status", "{filtered.len()} of {items.len()} collections" }
                            if filtered.is_empty() {
                                div { class: "collections-state",
                                    h2 { "No matching collections" }
                                    p { "Try another name or description." }
                                    button { class: "btn btn-secondary", onclick: move |_| query.set(String::new()), "Clear search" }
                                }
                            } else {
                                div { class: "collection-album-grid",
                                    for collection in filtered {
                                        CollectionAlbum { key: "{collection.id}", collection: collection.clone() }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::matches_collection_query;

    #[test]
    fn collection_load_recovery_uses_safe_copy_and_live_pending_guard() {
        let source = include_str!("collection_list.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        assert!(!source.contains("Could not load collections: {error}"));
        assert!(source.contains("LoadRecovery"));
        assert!(source.contains("collections.state().read()"));
        assert!(source.contains("if !collections.pending()"));
    }

    #[test]
    fn collection_filter_matches_names_and_descriptions() {
        assert!(matches_collection_query("Weeknight meals", None, " NIGHT "));
        assert!(matches_collection_query(
            "Family",
            Some("Grandma’s baking"),
            "BAKING"
        ));
        assert!(matches_collection_query("Été", None, "été"));
        assert!(matches_collection_query("Family", None, "   "));
        assert!(!matches_collection_query("Family", None, "weeknight"));
    }
}
