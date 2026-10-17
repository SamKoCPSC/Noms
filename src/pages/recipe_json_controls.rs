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
                    p { "Recipe JSON keeps image links and edits. Import reuses accessible images and lets you review any that could not be restored." }
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
        async function worker() {{
            while (next < urls.length) {{
                const url = urls[next++];
                const ok = await new Promise(resolve => {{
                    const image = new Image(); image.referrerPolicy = 'no-referrer'; let timer;
                    function finish(ok) {{ clearTimeout(timer); image.onload = null; image.onerror = null; image.src = ''; resolve(ok); }}
                    image.onload = () => finish(image.naturalWidth > 0);
                    image.onerror = () => finish(false);
                    timer = setTimeout(() => finish(false), 3000); image.src = url;
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
