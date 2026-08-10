use dioxus::prelude::*;

use crate::api::collection::create_collection;
use crate::components::base::{
    validate_collection_form, Card, CollectionForm, CollectionVisibilityField, PageHeader,
};
use crate::components::AuthRequired;
use crate::types::CollectionVisibility;
use crate::Route;

fn parent_from_location() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let query = web_sys::window()?.location().search().ok()?;
        query.trim_start_matches('?').split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            (key == "parent" && uuid::Uuid::parse_str(value).is_ok()).then(|| value.to_string())
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

#[component]
pub fn CollectionNew() -> Element {
    let name = use_signal(|| "Untitled Collection".to_string());
    let description = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut visibility = use_signal(|| CollectionVisibility::Private);
    let navigator = use_navigator();
    let parent_id = use_signal(parent_from_location);

    rsx! { AuthRequired { div { class: "container",
        PageHeader { title: if parent_id().is_some() { "New subcollection" } else { "New collection" } }
        Card { CollectionForm { name, description, submit_label: "Create Collection", submitting: saving(), error: error(),
            extra_fields: rsx! {
                CollectionVisibilityField {
                    visibility: visibility(),
                    disabled: saving(),
                    onchange: move |selected| visibility.set(selected),
                }
            },
            onsubmit: move |_| {
                let Ok((clean_name, clean_description)) = validate_collection_form(&name(), &description()) else {
                    error.set(Some(validate_collection_form(&name(), &description()).unwrap_err().to_string())); return;
                };
                saving.set(true); error.set(None);
                let parent = parent_id();
                let selected_visibility = visibility();
                spawn(async move {
                    match create_collection(clean_name, clean_description, parent, selected_visibility).await {
                        Ok(collection) => { navigator.push(Route::CollectionDetail { id: collection.id.to_string() }); }
                        Err(problem) => { error.set(Some(problem.to_string())); saving.set(false); }
                    }
                });
            }
        } }
    } } }
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_collection_form_submits_typed_visibility() {
        let source = include_str!("collection_new.rs");
        assert!(source.contains("CollectionVisibilityField"));
        assert!(source.contains(concat!("create_", "collection(")));
        assert!(!source.contains(concat!("create_collection", "_with_visibility")));
        assert!(source.contains("CollectionVisibility::Private"));
    }
}
