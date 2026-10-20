use crate::api::collection_portability::{
    export_collection_json, import_collection_json, inspect_collection_json,
};
use crate::collection_json::{CollectionImportPreview, CollectionImportResult};
use crate::utils::download::download_file;
use dioxus::{html::FileData, prelude::*};
use std::rc::Rc;

#[component]
pub(super) fn CollectionExport(collection_id: String) -> Element {
    let mut ready = use_signal(|| false);
    let mut open = use_signal(|| false);
    let mut format = use_signal(|| "json");
    let mut layout = use_signal(|| "unified");
    let mut images = use_signal(|| true);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    use_effect(use_reactive((&collection_id,), move |_| {
        open.set(false);
        error.set(None);
    }));
    rsx! {
        document::Stylesheet { href: asset!("/assets/recipe-export-dialog.css") }
        button { r#type: "button", class: "btn btn-secondary touch-target", disabled: !ready(),
            onmounted: move |_| ready.set(true), onclick: move |_| { error.set(None); open.set(true); }, "Export collection" }
        if open() {
            crate::components::base::SharedModal {
                aria_labelledby: "collection-export-title", focus_selector: "input[name=collection-export-format]:checked", pending: pending(),
                on_dismiss: move |_| { if !pending() { open.set(false); } },
                div { class: "modal-card", max_width: "580px", display: "flex", flex_direction: "column", gap: "var(--space-md)",
                    onclick: move |event| event.stop_propagation(),
                    h2 { id: "collection-export-title", "Export collection" }
                    p { "Export this collection and its subcollections with their ordered saved recipes. Recipes used more than once are included once." }
                    fieldset { class: "recipe-export__fieldset", disabled: pending(),
                        legend { "Format" }
                        div { class: "recipe-export__formats",
                            label { class: "recipe-export__format",
                                input { r#type: "radio", name: "collection-export-format", value: "json", checked: format() == "json", onchange: move |_| { format.set("json"); error.set(None); } }
                                span { strong { "JSON" } small { "Back up and re-import into Noms." } }
                            }
                            label { class: "recipe-export__format",
                                input { r#type: "radio", name: "collection-export-format", value: "html", checked: format() == "html", onchange: move |_| { format.set("html"); error.set(None); } }
                                span { strong { "HTML" } small { "Read, share and print offline." } }
                            }
                        }
                    }
                    if format() == "json" {
                        p { class: "recipe-export__help", "JSON keeps image links and edits, but does not contain image files, drafts or history. Linked images may become unavailable. Collection covers can be reused only by their uploader." }
                        p { class: "recipe-export__help", "Up to 100 collections, 100 recipes, 1,000 memberships, 128 image placements and 8 MiB per file. Export a smaller subcollection if needed." }
                    } else {
                        label { r#for: "collection-html-layout", "HTML layout" }
                        select { id: "collection-html-layout", class: "neumo-inset input collection-export__layout", disabled: pending(), value: layout(),
                            onchange: move |event| { if event.value() == "individual" { layout.set("individual"); } else { layout.set("unified"); } error.set(None); },
                            option { value: "unified", "One unified document (.html)" }
                            option { value: "individual", "Individual documents (.zip)" }
                        }
                        p { class: "recipe-export__help",
                            if layout() == "unified" { "One cookbook with a contents list and links to each recipe. Convenient for sharing or printing the whole collection." }
                            else { "A ZIP with one standalone HTML file per recipe and a collection index. Extract it, then open index.html or share individual recipe files." }
                        }
                        label { class: "recipe-export__images",
                            input { r#type: "checkbox", checked: images(), disabled: pending(), onchange: move |event| images.set(event.checked()) }
                            "Include saved recipe images"
                        }
                        p { class: "recipe-export__help", "HTML is a reading copy; use JSON for re-import. Covers, history and drafts are excluded. Included images are embedded originals without crops or adjustments. External or unavailable images are omitted with notices in the recipe documents." }
                        p { class: "recipe-export__help", "Up to 100 collections, 100 recipes, 1,000 memberships, 128 image placements and 32 MiB per download. Export a smaller subcollection if needed." }
                    }
                    if let Some(problem) = error() { p { role: "alert", "{problem}" } }
                    div { display: "flex", gap: "var(--space-sm)", justify_content: "flex-end", flex_wrap: "wrap",
                        button { r#type: "button", class: "btn btn-secondary touch-target", disabled: pending(), onclick: move |_| open.set(false), "Cancel" }
                        button { r#type: "button", class: "btn btn-primary touch-target", "data-collection-export": "", disabled: pending(), aria_busy: pending(),
                            onclick: move |_| {
                                if pending() { return; }
                                pending.set(true); error.set(None);
                                let id = collection_id.clone();
                                spawn(async move {
                                    let download = if format() == "html" {
                                        let extension = if layout() == "individual" { "zip" } else { "html" };
                                        download_file(String::new(), format!("noms-collection-{id}.{extension}"),
                                            if extension == "zip" { "application/zip" } else { "text/html" },
                                            Some(format!("/api/collections/{id}/export-html?layout={}&images={}", layout(), images()))).await
                                    } else {
                                        match export_collection_json(id.clone()).await {
                                            Ok(json) => download_file(json, format!("noms-collection-{id}.json"), "application/json", None).await,
                                            Err(problem) => Err(format!("Could not export this collection: {problem}")),
                                        }
                                    };
                                    match download {
                                        Ok(()) => open.set(false),
                                        Err(problem) => error.set(Some(format!("{problem} Try a smaller collection or text-only HTML."))),
                                    }
                                    pending.set(false);
                                });
                            },
                            if pending() { "Preparing download…" } else if format() == "json" { "Download JSON" } else if layout() == "individual" { "Download HTML ZIP" } else { "Download HTML" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn CollectionJsonImport(on_imported: EventHandler<()>) -> Element {
    let account = crate::auth::account_binding::use_form_account();
    let auth = use_context::<Signal<crate::auth::context::AuthContext>>();
    let mut ready = use_signal(|| false);
    let mut open = use_signal(|| false);
    let mut selected = use_signal(|| None::<FileData>);
    let mut checked_json = use_signal(|| None::<Rc<String>>);
    let mut preview = use_signal(|| None::<CollectionImportPreview>);
    let mut result = use_signal(|| None::<CollectionImportResult>);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        button { r#type: "button", class: "btn btn-secondary touch-target", disabled: !ready(),
            onmounted: move |_| ready.set(true),
            onclick: move |_| { selected.set(None); checked_json.set(None); preview.set(None); result.set(None); error.set(None); open.set(true); },
            "Import collection JSON"
        }
        if open() {
            crate::components::base::SharedModal {
                aria_labelledby: "collection-import-title", focus_selector: "#collection-json-file", pending: pending(),
                on_dismiss: move |_| { if !pending() { open.set(false); } },
                div { class: "modal-card", max_width: "620px", display: "flex", flex_direction: "column", gap: "var(--space-md)",
                    onclick: move |event| event.stop_propagation(),
                    h2 { id: "collection-import-title", "Import collection JSON" }
                    if let Some(imported) = result() {
                        p { role: "status", "Imported {imported.collection_count} ",
                            if imported.collection_count == 1 { "collection" } else { "collections" }
                            " and {imported.recipe_count} ", if imported.recipe_count == 1 { "recipe" } else { "recipes" }
                            " as new private copies."
                        }
                        if !imported.warnings.is_empty() {
                            ul { for warning in imported.warnings { li { "{warning}" } } }
                        }
                        if !imported.image_reviews.is_empty() {
                            h3 { "Some recipe images need attention" }
                            p { "Recipe text is saved in the collection. Open these edit drafts to retry images, upload replacements or leave them out, then save your changes." }
                            ul { for review in imported.image_reviews {
                                li { Link { to: crate::Route::RecipeDraftEdit { id: review.draft_id.to_string() }, "{review.title}: review {review.image_count} ", if review.image_count == 1 { "image" } else { "images" } } }
                            } }
                        }
                        Link { class: "btn btn-primary touch-target", to: crate::Route::CollectionDetail { id: imported.collection_id.to_string() }, "Open imported collection" }
                        button { r#type: "button", class: "btn btn-secondary touch-target", onclick: move |_| open.set(false), "Close" }
                    } else {
                        p { "Choose a Noms collection JSON file (up to 8 MiB). Import creates a new private collection tree and new private saved recipes. Existing recipes and collections are kept. Nothing is shared publicly." }
                        p { "Recipe images are reused when accessible. Missing images remain reviewable in edit drafts. Collection covers require access as their uploader; unavailable covers can be uploaded again in Edit collection." }
                        label { r#for: "collection-json-file", "Collection JSON file" }
                        input { id: "collection-json-file", r#type: "file", accept: ".json,application/json", disabled: pending(),
                            onchange: move |event| {
                                selected.set(None); checked_json.set(None); preview.set(None); error.set(None);
                                if let Some(file) = event.files().into_iter().next() {
                                    if file.size() == 0 || file.size() > crate::collection_json::MAX_JSON_BYTES as u64 {
                                        error.set(Some("Choose a nonempty collection JSON file up to 8 MiB.".into()));
                                    } else { selected.set(Some(file)); }
                                }
                            }
                        }
                        if let Some(info) = preview() {
                            div { class: "neumo-inset", padding: "var(--space-md)", role: "status",
                                strong { "{info.name}" }
                                p { "{info.collection_count} ", if info.collection_count == 1 { "collection" } else { "collections" }
                                    " · {info.recipe_count} unique ", if info.recipe_count == 1 { "recipe" } else { "recipes" }
                                }
                                p { "Ready to import as private copies. A matching collection name gets a new name; nothing is overwritten." }
                            }
                        }
                        if let Some(problem) = error() { p { role: "alert", "{problem}" } }
                        div { display: "flex", gap: "var(--space-sm)", justify_content: "flex-end", flex_wrap: "wrap",
                            button { r#type: "button", class: "btn btn-secondary touch-target", disabled: pending(), onclick: move |_| open.set(false), "Cancel" }
                            button { r#type: "button", class: "btn btn-primary touch-target", disabled: pending() || selected().is_none(), aria_busy: pending(),
                                onclick: move |_| {
                                    if pending() { return; }
                                    let Some(file) = selected() else { return; };
                                    let cached = checked_json(); let info = preview(); let initiating = account();
                                    pending.set(true); error.set(None);
                                    spawn(async move {
                                        if let (Some(json), Some(info)) = (cached, info) {
                                            let available = super::recipe_json_controls::check_linked_images(info.external_image_urls).await;
                                            match import_collection_json(initiating, (*json).clone(), available).await {
                                                Ok(imported) => {
                                                    let current = auth.read().is_authenticated.then_some(auth.read().current_user_id).flatten();
                                                    if crate::auth::account_binding::account_completion_matches(initiating, current, imported.owner_id) {
                                                        result.set(Some(imported)); checked_json.set(None); selected.set(None); preview.set(None); on_imported.call(());
                                                    } else { error.set(Some(crate::auth::account_binding::ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into())); }
                                                }
                                                Err(problem) => error.set(Some(crate::auth::account_binding::account_error_feedback(&problem).map(str::to_owned).unwrap_or_else(|| format!("Could not import this collection: {problem}")))),
                                            }
                                        } else {
                                            match file.read_string().await {
                                                Ok(json) if !json.is_empty() && json.len() <= crate::collection_json::MAX_JSON_BYTES => {
                                                    match inspect_collection_json(initiating, json.clone()).await {
                                                        Ok(info) => { checked_json.set(Some(Rc::new(json))); preview.set(Some(info)); }
                                                        Err(problem) => error.set(Some(format!("Could not check this file: {problem}"))),
                                                    }
                                                }
                                                _ => error.set(Some("Could not read this collection JSON file. Choose a nonempty UTF-8 file up to 8 MiB.".into())),
                                            }
                                        }
                                        pending.set(false);
                                    });
                                },
                                if pending() { "Working…" } else if preview().is_some() { "Import private collection" } else { "Check file" }
                            }
                        }
                    }
                }
            }
        }
    }
}
