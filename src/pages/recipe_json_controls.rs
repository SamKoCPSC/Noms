use crate::utils::download::download_file;
use dioxus::html::FileData;
use dioxus::prelude::*;

#[component]
pub(super) fn RecipeJsonExport(recipe_id: String) -> Element {
    let mut ready = use_signal(|| false);
    let mut open = use_signal(|| false);
    let mut format = use_signal(|| "json");
    let mut with_images = use_signal(|| false);
    let mut html_images = use_signal(|| true);
    let mut prepared = use_signal(|| None::<std::rc::Rc<crate::recipe_html::HtmlExport>>);
    let mut pending = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    use_effect(use_reactive((&recipe_id,), move |_| {
        open.set(false);
        prepared.set(None);
        error.set(None);
    }));
    rsx! {
        document::Stylesheet { href: asset!("/assets/recipe-export-dialog.css") }
        button {
            r#type: "button", class: "btn btn-secondary touch-target", disabled: !ready(),
            onmounted: move |_| ready.set(true),
            onclick: move |_| { error.set(None); prepared.set(None); open.set(true); },
            "Export recipe"
        }
        if open() {
            crate::components::base::SharedModal {
                aria_labelledby: "recipe-export-title", focus_selector: "[data-export-download]",
                pending: pending(), on_dismiss: move |_| { if !pending() { open.set(false); } },
                div { class: "modal-card", max_width: "620px", display: "flex", flex_direction: "column", gap: "var(--space-md)",
                    onclick: move |event| event.stop_propagation(),
                    h2 { id: "recipe-export-title", "Export recipe" }
                    p { class: "recipe-export__help", "Take a copy of the current saved recipe. Choose the format that fits what you want to do next." }
                    fieldset { class: "recipe-export__fieldset", disabled: pending(),
                        legend { "Format" }
                        div { class: "recipe-export__formats",
                            label { class: "recipe-export__format",
                                input { r#type: "radio", name: "recipe-export-format", value: "json", checked: format() == "json",
                                    onchange: move |_| { format.set("json"); prepared.set(None); error.set(None); } }
                                span { strong { "JSON" } small { "Back up & re-import into Noms" } }
                            }
                            label { class: "recipe-export__format",
                                input { r#type: "radio", name: "recipe-export-format", value: "html", checked: format() == "html",
                                    onchange: move |_| { format.set("html"); prepared.set(None); error.set(None); } }
                                span { strong { "HTML" } small { "Read, share & print offline" } }
                            }
                        }
                    }
                    if format() == "json" {
                        p { class: "recipe-export__help",
                            "JSON keeps structured content, image links and edits for Noms re-import. Linked images may become unavailable. It is not a formatted reading copy."
                        }
                        label { class: "recipe-export__images",
                            input { r#type: "checkbox", checked: with_images(), disabled: pending(),
                                onchange: move |event| with_images.set(event.checked()) }
                            "Download saved images too"
                        }
                        p { class: "recipe-export__help",
                            "With images selected, download a ZIP with recipe.json and saved image files for manual recovery. External image files are not downloaded. Crops and adjustments are not baked into the files."
                        }
                    } else {
                        p { class: "recipe-export__help",
                            "HTML is a styled reading copy that opens in a browser without Noms or an internet connection. Share the file itself, or print it to paper or PDF. It does not include editing or history and cannot be re-imported into Noms."
                        }
                        label { class: "recipe-export__images",
                            input { r#type: "checkbox", checked: html_images(), disabled: pending(),
                                onchange: move |event| { html_images.set(event.checked()); prepared.set(None); } }
                            "Include saved images"
                        }
                        p { class: "recipe-export__help",
                            if html_images() {
                                "Saved images are embedded in this one file, making it larger (up to 32 MiB). External or unavailable images may be left out; you can review omissions before downloading. Image crops and adjustments are not applied."
                            } else { "Download a smaller text-only HTML file. Recipe and step images will be left out." }
                        }
                    }
                    if let Some(copy) = prepared() {
                        div { class: "recipe-export__warnings", role: "alert",
                            strong { "Some images could not be included" }
                            p { "The recipe text is ready. Download this copy without the images listed below, or cancel to choose another option." }
                            ul { for warning in copy.warnings.clone() { li { "{warning}" } } }
                        }
                    }
                    if let Some(problem) = error() { p { role: "alert", "{problem}" } }
                    div { class: "recipe-export__actions",
                        button { r#type: "button", class: "btn btn-secondary touch-target", disabled: pending(),
                            onclick: move |_| open.set(false), "Cancel" }
                        if prepared().is_some() {
                            button { r#type: "button", class: "btn btn-secondary touch-target", disabled: pending(),
                                onclick: move |_| { prepared.set(None); error.set(None); }, "Try again" }
                        }
                        button {
                            r#type: "button", class: "btn btn-primary touch-target", "data-export-download": "",
                            disabled: pending(), aria_busy: pending(),
                            onclick: move |_| {
                                if pending() { return; }
                                pending.set(true); error.set(None);
                                let id = recipe_id.clone();
                                let html = format() == "html";
                                let images = if html { html_images() } else { with_images() };
                                let existing = prepared();
                                spawn(async move {
                                    let result = if html {
                                        match existing {
                                            Some(copy) => Ok((copy.html.clone(), "html", "text/html;charset=utf-8", None)),
                                            None => match crate::api::recipe::export_recipe_html(id.clone(), images).await {
                                                Ok(copy) if !copy.warnings.is_empty() => {
                                                    prepared.set(Some(std::rc::Rc::new(copy))); pending.set(false); return;
                                                }
                                                Ok(copy) => Ok((copy.html, "html", "text/html;charset=utf-8", None)),
                                                Err(problem) => Err(problem.to_string()),
                                            }
                                        }
                                    } else if images {
                                        Ok((String::new(), "zip", "application/zip", Some(format!("/api/recipes/{id}/export-images"))))
                                    } else {
                                        crate::api::recipe::export_recipe_json(id.clone()).await
                                            .map(|content| (content, "json", "application/json", None))
                                            .map_err(|problem| problem.to_string())
                                    };
                                    match result {
                                        Ok((content, extension, mime, url)) => {
                                            if download_file(content, format!("noms-recipe-{id}.{extension}"), mime, url).await.is_ok() {
                                                open.set(false); prepared.set(None);
                                            } else {
                                                error.set(Some("Could not download this file. Try again, or choose a text-only export.".into()));
                                            }
                                        }
                                        Err(problem) => error.set(Some(format!("Could not export this recipe: {problem}"))),
                                    }
                                    pending.set(false);
                                });
                            },
                            if pending() { "Preparing download…" }
                            else if format() == "html" && prepared().is_some() { "Download HTML anyway" }
                            else if format() == "html" { "Download HTML" }
                            else if with_images() { "Download ZIP" }
                            else { "Download JSON" }
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
                "Choose a recipe exported from Noms (up to 2 MiB). Import creates a new private draft; existing recipes stay as they are. Accessible images are reused. Review unresolved images in the editor to retry, re-upload or leave them out."
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
                                let checked = match crate::api::recipe::recipe_json_image_urls(account(), json.clone()).await {
                                    Ok(urls) => check_linked_images(urls).await,
                                    Err(problem) => { error.set(Some(format!("Could not import JSON: {problem}"))); pending.set(false); return; }
                                };
                                match crate::api::recipe::import_recipe_json(account(), json, checked).await {
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

/// Browser-only image decoding checks; the server never fetches remote URLs.
pub(super) async fn check_linked_images(urls: Vec<String>) -> Vec<String> {
    if urls.is_empty() {
        return vec![];
    }
    let args = serde_json::to_string(&urls).unwrap_or_else(|_| "[]".into());
    let script = format!(
        r#"
        const urls = {args}; const available = []; let next = 0;
        const deadline = Date.now() + 12000;
        async function worker() {{
            while (next < urls.length && Date.now() < deadline) {{
                const url = urls[next++];
                const ok = await new Promise(resolve => {{
                    const image = new Image(); image.referrerPolicy = 'no-referrer'; let timer;
                    function finish(ok) {{ clearTimeout(timer); image.onload = null; image.onerror = null; image.src = ''; resolve(ok); }}
                    image.onload = () => finish(image.naturalWidth > 0);
                    image.onerror = () => finish(false);
                    timer = setTimeout(() => finish(false), Math.min(3000, Math.max(0, deadline - Date.now()))); image.src = url;
                }});
                if (ok) available.push(url);
            }}
        }}
        await Promise.all(Array.from({{length: Math.min(4, urls.length)}}, worker)); return available;
    "#
    );
    document::eval(&script)
        .await
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}
#[component]
pub(super) fn ImportedImageReview(
    images: Vec<crate::types::ImportedRecipeImage>,
    enabled: bool,
    pending: bool,
    on_resolve: EventHandler<(uuid::Uuid, bool)>,
) -> Element {
    if images.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "neumo-raised", padding: "var(--space-lg)", margin_bottom: "var(--space-lg)", aria_labelledby: "imported-images-title",
            h2 { id: "imported-images-title", "Imported images need attention" }
            p { "Your recipe text is saved. These images are not included yet. Retry, upload replacements using Photos or the indicated step's photo controls, or leave them out." }
            if !enabled && !pending { p { "Save your draft before reviewing imported images." } }
            ul {
                for image in images {
                    li { key: "{image.id}", margin_top: "var(--space-md)",
                        strong { "{image.placement()}" }
                        p { "{image.status.message()}" }
                        p { overflow_wrap: "anywhere", "{image.image.url}" }
                        if let Some(text) = image.step_text.as_ref() { p { "Original step: {text}" } }
                        div { display: "flex", gap: "var(--space-sm)",
                            button { r#type: "button", class: "btn btn-secondary touch-target", disabled: !enabled || pending,
                                onclick: move |_| on_resolve.call((image.id, false)), "Retry image" }
                            button { r#type: "button", class: "btn btn-secondary touch-target", disabled: !enabled || pending,
                                onclick: move |_| on_resolve.call((image.id, true)), "Leave image out" }
                        }
                    }
                }
            }
            if pending { p { role: "status", "Checking imported image…" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_review_identifies_nested_placement_and_escapes_untrusted_urls() {
        fn review() -> Element {
            let image = crate::types::ImportedRecipeImage {
                id: uuid::Uuid::new_v4(),
                image: crate::types::RecipeImage::new(
                    "https://images.example.test/<script>".into(),
                    None,
                    crate::types::RecipeImageEdit::default_rect_16x9(),
                ),
                step_path: vec![0, 1],
                step_text: Some("Stir".into()),
                position: 0,
                status: crate::types::ImportImageStatus::Unavailable,
            };
            rsx! { ImportedImageReview { images: vec![image], enabled: false, pending: false, on_resolve: |_| {} } }
        }
        let mut dom = VirtualDom::new(review);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Step 1.2 photo 1"));
        assert!(html.contains("Retry image") && html.contains("Leave image out"));
        assert!(html.contains("Save your draft") && html.contains("disabled"));
        assert!(!html.contains("<script>"));
    }
    #[test]
    fn review_pending_inertness_does_not_lock_the_idle_editor() {
        let html = dioxus_ssr::render_element(rsx! { div { inert: false, "Editable" } });
        assert!(!html.contains("inert"), "{html}");
        let html = dioxus_ssr::render_element(rsx! { div { inert: true, "Checking" } });
        assert!(html.contains("inert"));
    }
}
