use dioxus::prelude::*;

use crate::api::collection::create_collection;
use crate::components::base::{validate_collection_form, Card, CollectionForm, PageHeader};
use crate::components::AuthRequired;
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
    let navigator = use_navigator();
    let parent_id = use_signal(parent_from_location);

    rsx! { AuthRequired { div { class: "container",
        PageHeader { title: if parent_id().is_some() { "New subcollection" } else { "New collection" } }
        Card { CollectionForm { name, description, submit_label: "Create Collection", submitting: saving(), error: error(),
            onsubmit: move |_| {
                let Ok((clean_name, clean_description)) = validate_collection_form(&name(), &description()) else {
                    error.set(Some(validate_collection_form(&name(), &description()).unwrap_err().to_string())); return;
                };
                saving.set(true); error.set(None);
                let parent = parent_id();
                spawn(async move {
                    match create_collection(clean_name, clean_description, parent).await {
                        Ok(collection) => { navigator.push(Route::CollectionDetail { id: collection.id.to_string() }); }
                        Err(problem) => { error.set(Some(problem.to_string())); saving.set(false); }
                    }
                });
            }
        } }
    } } }
}
