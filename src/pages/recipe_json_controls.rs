use dioxus::html::FileData;
use dioxus::prelude::*;

#[component]
pub(super) fn RecipeJsonExport(recipe_id: String) -> Element {
    let mut ready = use_signal(|| false);
    let mut open = use_signal(|| false);
    let mut with_images = use_signal(|| false);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        button {
            r#type: "button", class: "btn btn-secondary touch-target", disabled: !ready(),
            onmounted: move |_| ready.set(true),
            onclick: move |_| { error.set(None); open.set(true); },
            "Export recipe"
        }
        if open() {
            crate::components::base::SharedModal {
                aria_labelledby: "recipe-export-title", focus_selector: "[data-export-download]",
                pending: pending(), on_dismiss: move |_| { if !pending() { open.set(false); } },
                div { class: "modal-card", display: "flex", flex_direction: "column", gap: "var(--space-md)",
                    onclick: move |event| event.stop_propagation(),
                    h2 { id: "recipe-export-title", "Export recipe" }
                    p { "Recipe JSON excludes recipe and step images. You can add images manually after re-importing." }
                    label {
                        input { r#type: "checkbox", checked: with_images(), disabled: pending(),
                            onchange: move |event| with_images.set(event.checked()) }
                        " Download saved images too"
                    }
                    p { "With images selected, download one ZIP containing recipe.json and saved image files. Externally linked images are not downloaded. Image crops and adjustments are not baked into these files." }
                    if let Some(problem) = error() { p { role: "alert", "{problem}" } }
                    div { display: "flex", flex_wrap: "wrap", justify_content: "flex-end", gap: "var(--space-sm)",
                        button { r#type: "button", class: "btn btn-secondary", disabled: pending(),
                            onclick: move |_| open.set(false), "Cancel" }
                        button {
                            r#type: "button", class: "btn btn-primary", "data-export-download": "",
                            disabled: pending(), aria_busy: pending(),
                            onclick: move |_| {
                                if pending() { return; }
                                pending.set(true); error.set(None);
                                let id = recipe_id.clone();
                                let images = with_images();
                                spawn(async move {
                                    let response = if images { Ok(String::new()) }
                                        else { crate::api::recipe::export_recipe_json(id.clone()).await };
                                    match response {
                                        Ok(json) => {
                                            let extension = if images { "zip" } else { "json" };
                                            let args = serde_json::json!([json, format!("noms-recipe-{id}.{extension}"), images, id]);
                                            let script = format!(r#"
                                                const [content, filename, images, id] = {args};
                                                let blob;
                                                if (images) {{
                                                    const response = await fetch('/api/recipes/' + encodeURIComponent(id) + '/export-images', {{credentials:'same-origin'}});
                                                    if (!response.ok) throw new Error('Image download failed');
                                                    blob = await response.blob();
                                                }} else {{ blob = new Blob([content], {{type:'application/json'}}); }}
                                                const url = URL.createObjectURL(blob);
                                                const link = document.createElement('a');
                                                link.href = url; link.download = filename;
                                                document.body.append(link); link.click(); link.remove();
                                                setTimeout(() => URL.revokeObjectURL(url), 1000);
                                                return true;
                                            "#);
                                            if document::eval(&script).await.is_err() {
                                                error.set(Some(if images { "Could not download saved images. An image may be unavailable or the ZIP may exceed 32 MiB. Try again, or download JSON only." } else { "Download failed. Please try again." }.into()));
                                            } else { open.set(false); }
                                        }
                                        Err(_) => error.set(Some("Could not export this recipe. Please try again.".into())),
                                    }
                                    pending.set(false);
                                });
                            },
                            if pending() { "Preparing download…" } else if with_images() { "Download ZIP" } else { "Download JSON" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn JsonRecipeImport() -> Element {
    let account = crate::auth::account_binding::use_form_account();
    let mut selected = use_signal(|| None::<FileData>);
    let mut ready = use_signal(|| false);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let navigator = use_navigator();
    rsx! {
        section { class: "recipe-import__field", aria_labelledby: "recipe-json-title",
            h3 { id: "recipe-json-title", "Or import a Noms JSON file" }
            p { id: "recipe-json-help", class: "recipe-import__hint",
                "Choose a recipe exported from Noms (up to 2 MiB). Import creates a new private draft; existing recipes stay as they are. Images are not imported. Add them manually in the editor."
            }
            label { r#for: "recipe-json-file", "Noms recipe JSON" }
            input {
                id: "recipe-json-file", r#type: "file", accept: ".json,application/json",
                aria_describedby: "recipe-json-help", disabled: !ready() || pending(),
                onmounted: move |_| ready.set(true),
                onchange: move |event| {
                    if !ready() || pending() { return; }
                    error.set(None); selected.set(None);
                    if let Some(file) = event.files().into_iter().next() {
                        if file.size() > crate::recipe_json::MAX_JSON_BYTES as u64 || file.size() == 0 {
                            error.set(Some("Choose a nonempty Noms JSON file up to 2 MiB.".into()));
                        } else { selected.set(Some(file)); }
                    }
                }
            }
            button {
                r#type: "button", class: "btn btn-secondary", disabled: !ready() || pending() || selected().is_none(),
                aria_busy: pending(),
                onclick: move |_| {
                    if pending() { return; }
                    let Some(file) = selected() else { return; };
                    pending.set(true); error.set(None);
                    spawn(async move {
                        match file.read_string().await {
                            Ok(json) if json.len() <= crate::recipe_json::MAX_JSON_BYTES => {
                                match crate::api::recipe::import_recipe_json(account(), json).await {
                                    Ok(draft) => { navigator.push(crate::Route::RecipeDraftEdit { id: draft.id.to_string() }); }
                                    Err(problem) => error.set(Some(crate::auth::account_binding::account_error_feedback(&problem)
                                        .map(str::to_owned).unwrap_or_else(|| format!("Could not import JSON: {problem}")))),
                                }
                            }
                            _ => error.set(Some("Could not read this JSON file, or it exceeds 2 MiB.".into())),
                        }
                        pending.set(false);
                    });
                },
                if pending() { "Importing JSON…" } else { "Import JSON as private draft" }
            }
            if let Some(problem) = error() { p { role: "alert", "{problem}" } }
        }
    }
}
